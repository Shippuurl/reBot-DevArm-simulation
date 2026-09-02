//! Optional Rerun recording sink for the UI telemetry boundary.
//!
//! The sink is deliberately kept out of the control path: logging errors are
//! returned to the UI and never sent back to the gateway.  Enable it with the
//! `rerun-recording` Cargo feature.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::telemetry::{
    MAX_IMAGE_BYTES, MAX_IMAGES_PER_FRAME, MAX_POINT_CLOUD_POINTS, MAX_POINT_CLOUDS_PER_FRAME,
    TelemetryFrame,
};

pub struct RerunRecorder {
    recording: rerun::RecordingStream,
}

impl RerunRecorder {
    /// Wrap an already-connected recording stream.
    ///
    /// The embedded Viewer owns the gRPC connection setup because it also
    /// starts the local Rerun receiver.  Reusing the stream here keeps model
    /// assets and live gateway telemetry in one recording instead of creating
    /// a second, disconnected sample recording.
    pub fn from_recording(recording: rerun::RecordingStream) -> Self {
        Self { recording }
    }

    /// Borrow the underlying stream for protocol-specific frame logging.
    pub fn recording(&self) -> &rerun::RecordingStream {
        &self.recording
    }

    /// Return ownership of the wrapped stream after one-off setup is done.
    pub fn into_recording(self) -> rerun::RecordingStream {
        self.recording
    }
}

#[derive(Debug, Deserialize)]
struct ModelManifest {
    #[serde(default)]
    urdf: Option<String>,
    #[serde(default)]
    visuals: Vec<ModelVisual>,
}

#[derive(Debug, Deserialize)]
struct ModelVisual {
    link: String,
    name: String,
    mesh: String,
    /// Optional per-visual color in RGBA byte order.  STL has no material
    /// channel, so this is the portable way to keep the model legible in
    /// Rerun without requiring a conversion to glTF.
    #[serde(default)]
    albedo_factor: Option<[u8; 4]>,
}

impl RerunRecorder {
    pub fn save(path: impl AsRef<Path>) -> Result<Self, String> {
        // When running beside the embedded Viewer, stream directly to its
        // gRPC receiver; otherwise keep the existing offline .rrd workflow.
        let builder = rerun::RecordingStreamBuilder::new("robot_workspace");
        let recording = if let Ok(url) = std::env::var("RERUN_GRPC_URL") {
            builder
                .connect_grpc_opts(url)
                .map_err(|error| format!("连接 Rerun gRPC 接收器失败: {error}"))?
        } else {
            builder
                .save(path.as_ref().to_path_buf())
                .map_err(|error| format!("创建 Rerun 记录失败: {error}"))?
        };
        Ok(Self { recording })
    }

    pub fn log_frame(&self, frame: &TelemetryFrame) -> Result<(), String> {
        self.recording
            .set_time_sequence("frame", frame.sequence as i64);

        for (index, (&position, &velocity)) in frame
            .joint_position
            .iter()
            .zip(frame.joint_velocity.iter())
            .enumerate()
        {
            let joint_path = format!("robot/joints/joint_{}/position", index + 1);
            self.recording
                .log(joint_path, &rerun::Scalars::single(position))
                .map_err(|error| format!("记录关节位置失败: {error}"))?;

            let velocity_path = format!("robot/joints/joint_{}/velocity", index + 1);
            self.recording
                .log(velocity_path, &rerun::Scalars::single(velocity))
                .map_err(|error| format!("记录关节速度失败: {error}"))?;
        }

        for transform in &frame.tf {
            let parent = canonical_frame_name(&transform.parent);
            let child = canonical_frame_name(&transform.child);
            let path = format!("robot/frames/{}", entity_name(&child));
            let value = rerun::Transform3D::from_translation_rotation(
                transform.translation_m,
                rerun::Quaternion::from_xyzw(transform.rotation_xyzw),
            )
            .with_parent_frame(frame_id(&parent))
            .with_child_frame(frame_id(&child));
            self.recording
                .log(path, &value)
                .map_err(|error| format!("记录 TF 失败: {error}"))?;
        }

        log_trajectory(&self.recording, "planned", &frame.planned_trajectory)?;
        log_trajectory(&self.recording, "actual", &frame.actual_trajectory)?;
        log_sensors(&self.recording, frame)?;
        Ok(())
    }

    /// Logs every visual mesh listed by the model manifest.  Each visual is a
    /// separate child entity below its link, so Rerun can apply the dynamic TF
    /// of that link and users can hide individual components in the tree.
    pub fn log_model_assets(&self, root: impl AsRef<Path>) -> Result<usize, String> {
        let root = root.as_ref();
        self.recording
            .log_static("robot", &rerun::ViewCoordinates::RIGHT_HAND_Z_UP())
            .map_err(|error| format!("记录 Rerun 坐标约定失败: {error}"))?;
        self.recording
            .log_static("robot/frames", &rerun::CoordinateFrame::new("world"))
            .map_err(|error| format!("记录世界坐标帧失败: {error}"))?;

        let manifest_path = model_manifest_path(root)?;
        let (manifest, urdf_path) = load_model_manifest(root)?;
        let mut count = 0;
        let mut linked_frames = std::collections::BTreeSet::new();
        for (index, visual) in manifest.iter().enumerate() {
            let path = resolve_model_path(root, &visual.mesh)?;
            let mut asset = rerun::Asset3D::from_file_path(&path)
                .map_err(|error| format!("读取模型资源 {} 失败: {error}", path.display()))?;
            if let Some(color) = visual.albedo_factor {
                asset = asset.with_albedo_factor(u32::from_be_bytes(color));
            }
            let link = entity_name(&canonical_frame_name(&visual.link));
            let name = entity_name(&visual.name);
            if linked_frames.insert(link.clone()) {
                self.recording
                    .log_static(
                        format!("robot/frames/{link}"),
                        &rerun::CoordinateFrame::new(frame_id(&link)),
                    )
                    .map_err(|error| format!("记录坐标帧 {link} 失败: {error}"))?;
                self.recording
                    .log_static(
                        format!("robot/frames/{link}"),
                        &rerun::Transform3D::from_translation([0.0, 0.0, 0.0])
                            .with_parent_frame("world")
                            .with_child_frame(frame_id(&link)),
                    )
                    .map_err(|error| format!("记录坐标帧初始变换 {link} 失败: {error}"))?;
            }
            let model_entity = format!("robot/frames/{link}/model/{index:02}_{name}");
            // Asset3D entities otherwise get an implicit `tf#<entity-path>`
            // frame of their own. Attach each mesh explicitly to its link's
            // named frame so dynamic link transforms resolve all the way to
            // world in the Rerun transform graph.
            self.recording
                .log_static(
                    model_entity.as_str(),
                    &rerun::CoordinateFrame::new(frame_id(&link)),
                )
                .map_err(|error| format!("记录模型坐标帧 {model_entity} 失败: {error}"))?;
            self.recording
                .log_static(model_entity, &asset)
                .map_err(|error| format!("记录模型资源 {} 失败: {error}", visual.mesh))?;
            count += 1;
        }

        // Keep the manifest itself in the recording.  It makes a .rrd
        // self-describing: a later viewer session can inspect the exact mesh
        // list and palette used to produce the scene.
        if manifest_path.is_file() {
            let document = rerun::TextDocument::from_file_path(&manifest_path)
                .map_err(|error| format!("读取模型清单失败: {error}"))?;
            self.recording
                .log_static("robot/model/manifest", &document)
                .map_err(|error| format!("记录模型清单失败: {error}"))?;
        }

        if let Some(urdf) = urdf_path {
            let document = rerun::TextDocument::from_file_path(&urdf)
                .map_err(|error| format!("读取 URDF 文件失败: {error}"))?;
            self.recording
                .log_static("robot/model/urdf", &document)
                .map_err(|error| format!("记录 URDF 文件失败: {error}"))?;
        }

        let scene = root.join("mujoco/scene.xml");
        if scene.is_file() {
            let document = rerun::TextDocument::from_file_path(&scene)
                .map_err(|error| format!("读取 MuJoCo 场景文件失败: {error}"))?;
            self.recording
                .log_static("robot/model/mujoco_scene", &document)
                .map_err(|error| format!("记录 MuJoCo 场景文件失败: {error}"))?;
        }
        Ok(count)
    }
}

fn load_model_manifest(root: &Path) -> Result<(Vec<ModelVisual>, Option<PathBuf>), String> {
    let configured = std::env::var("ROBOT_MODEL_MANIFEST").ok();
    let manifest_path = model_manifest_path(root)?;
    if manifest_path.is_file() {
        let text = fs::read_to_string(&manifest_path)
            .map_err(|error| format!("读取模型清单 {} 失败: {error}", manifest_path.display()))?;
        let manifest: ModelManifest = serde_json::from_str(&text)
            .map_err(|error| format!("解析模型清单 {} 失败: {error}", manifest_path.display()))?;
        if manifest.visuals.is_empty() {
            return Err(format!(
                "模型清单没有 visual mesh: {}",
                manifest_path.display()
            ));
        }
        let urdf = manifest
            .urdf
            .map(|relative| resolve_model_path(root, &relative))
            .transpose()?;
        return Ok((manifest.visuals, urdf));
    }
    if configured.is_some() {
        return Err(format!("模型清单不存在: {}", manifest_path.display()));
    }

    // Keep recordings usable for older asset folders that do not yet contain
    // a manifest.  New models should add rerun/model.json instead.
    let fallback = [
        ("base_link", "base_link", "meshes/base_link.STL"),
        ("link1", "link1", "meshes/link1.STL"),
        ("link2", "link2", "meshes/link2.STL"),
        ("link3", "link3", "meshes/link3.STL"),
        ("link4", "link4", "meshes/link4.STL"),
        ("link5", "link5", "meshes/link5.STL"),
        ("link6", "link6", "meshes/link6.STL"),
        ("gripper_end", "gripper_end", "meshes/gripper_end.STL"),
    ];
    Ok((
        fallback
            .into_iter()
            .map(|(link, name, mesh)| ModelVisual {
                link: link.to_owned(),
                name: name.to_owned(),
                mesh: mesh.to_owned(),
                albedo_factor: None,
            })
            .collect(),
        None,
    ))
}

fn model_manifest_path(root: &Path) -> Result<PathBuf, String> {
    let relative =
        std::env::var("ROBOT_MODEL_MANIFEST").unwrap_or_else(|_| "rerun/model.json".to_owned());
    let relative_path = Path::new(&relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(format!("模型清单路径必须位于资源目录内: {relative}"));
    }
    Ok(root.join(relative_path))
}

fn resolve_model_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(format!("模型路径必须位于资源目录内: {relative}"));
    }
    let path = root.join(relative_path);
    if !path.is_file() {
        return Err(format!("模型文件不存在: {}", path.display()));
    }
    Ok(path)
}

fn log_trajectory(
    recording: &rerun::RecordingStream,
    kind: &str,
    trajectory: &[crate::telemetry::TrajectoryPoint],
) -> Result<(), String> {
    let Some(point) = trajectory.last() else {
        return Ok(());
    };
    for (index, &position) in point.position_rad.iter().enumerate() {
        let path = format!("robot/trajectory/{kind}/joint_{}", index + 1);
        recording
            .log(path, &rerun::Scalars::single(position))
            .map_err(|error| format!("记录 {kind} 轨迹失败: {error}"))?;
    }
    Ok(())
}

fn log_sensors(recording: &rerun::RecordingStream, frame: &TelemetryFrame) -> Result<(), String> {
    for image in frame
        .images
        .iter()
        .filter(|image| image.width <= 4096 && image.height <= 4096)
        .take(MAX_IMAGES_PER_FRAME)
    {
        // Encoded images are intentionally skipped when they exceed the
        // transport budget. Truncating compressed bytes would create an
        // invalid payload and is less safe than dropping one frame.
        if image.data.len() > MAX_IMAGE_BYTES {
            continue;
        }
        if image.data.is_empty() {
            continue;
        }
        let path = format!("sensors/{}/image", entity_name(&image.sensor));
        let encoded = rerun::EncodedImage::from_file_contents(image.data.clone());
        recording
            .log(path, &encoded)
            .map_err(|error| format!("记录图像失败: {error}"))?;
    }
    for cloud in frame.point_clouds.iter().take(MAX_POINT_CLOUDS_PER_FRAME) {
        if cloud.positions.is_empty() {
            continue;
        }
        let path = format!("sensors/{}/points", entity_name(&cloud.sensor));
        let stride = cloud
            .positions
            .len()
            .div_ceil(MAX_POINT_CLOUD_POINTS)
            .max(1);
        let positions: Vec<[f32; 3]> = cloud
            .positions
            .iter()
            .copied()
            .enumerate()
            .filter(|(index, _)| index % stride == 0)
            .take(MAX_POINT_CLOUD_POINTS)
            .map(|(_, position)| position)
            .collect();
        let mut points = rerun::Points3D::new(positions);
        if cloud.colors_rgba.len() == cloud.positions.len() {
            let colors: Vec<u32> = cloud
                .colors_rgba
                .iter()
                .copied()
                .enumerate()
                .filter(|(index, _)| index % stride == 0)
                .take(MAX_POINT_CLOUD_POINTS)
                .map(|(_, color)| color)
                .collect();
            points = points.with_colors(colors);
        }
        recording
            .log(path, &points)
            .map_err(|error| format!("记录点云失败: {error}"))?;
    }
    Ok(())
}

fn entity_name(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn canonical_frame_name(value: &str) -> String {
    match value {
        // The local mock source predates the URDF names.  Normalize those
        // aliases at the recording boundary so the same assets work for both
        // mock and MuJoCo telemetry.
        "base" => "base_link".to_owned(),
        "tool" => "gripper_end".to_owned(),
        other => other.to_owned(),
    }
}

/// Explicit frame IDs are namespaced so a recording can contain another
/// robot or a sensor tree without accidentally joining transform graphs.
fn frame_id(value: &str) -> String {
    format!("robot::{value}")
}

#[cfg(test)]
mod tests {
    use super::RerunRecorder;
    use crate::telemetry::{TelemetryFrame, Transform3D};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temporary_recording(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{suffix}.rrd"))
    }

    #[test]
    fn writes_a_recording_file_without_a_viewer() {
        let path = temporary_recording("robot-workspace");

        let recorder = RerunRecorder::save(&path).expect("create recording");
        let mut frame = TelemetryFrame {
            sequence: 1,
            ..TelemetryFrame::default()
        };
        frame.tf.push(Transform3D::default());
        recorder.log_frame(&frame).expect("log frame");
        drop(recorder);

        let metadata = fs::metadata(&path).expect("recording file");
        assert!(metadata.len() > 0);
        fs::remove_file(path).expect("remove temporary recording");
    }

    #[test]
    fn writes_all_manifest_model_assets_to_an_existing_recording() {
        let path = temporary_recording("robot-model-assets");
        let recording = rerun::RecordingStreamBuilder::new("model_asset_test")
            .save(&path)
            .expect("create recording");
        let recorder = RerunRecorder::from_recording(recording);
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/robot/b601_rs");

        let count = recorder
            .log_model_assets(root)
            .expect("log model manifest assets");
        assert_eq!(count, 25);
        let frame = TelemetryFrame {
            sequence: 1,
            tf: vec![Transform3D::default()],
            ..TelemetryFrame::default()
        };
        recorder
            .log_frame(&frame)
            .expect("log live transform after static model");
        drop(recorder);

        let metadata = fs::metadata(&path).expect("model recording file");
        assert!(metadata.len() > 0);
        fs::remove_file(path).expect("remove temporary model recording");
    }
}
