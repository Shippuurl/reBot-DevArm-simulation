//! 生成一份可直接在 Win11 Rerun Viewer 中打开的最小记录。
//!
//! 用法：`cargo run --features rerun-recording --bin rerun_sample -- [输出.rrd]`

#[cfg(not(feature = "rerun-recording"))]
fn main() {
    eprintln!("请使用 --features rerun-recording 编译此工具");
}

#[cfg(feature = "rerun-recording")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{fs, path::PathBuf};

    let output = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("recordings/sample.rrd"));
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    let builder = rerun::RecordingStreamBuilder::new("robot_workspace_sample");
    let grpc_url = std::env::var("RERUN_GRPC_URL")
        .ok()
        .filter(|url| !url.trim().is_empty());
    let recording = if let Some(url) = grpc_url.as_deref() {
        builder.connect_grpc_opts(url)?
    } else {
        builder.save(&output)?
    };
    let model_root = std::env::var_os("ROBOT_MODEL_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("assets/robot/b601_rs"));
    let recorder = rebot_sim_viewer::rerun_bridge::RerunRecorder::from_recording(recording);
    recorder
        .log_model_assets(&model_root)
        .map_err(std::io::Error::other)?;
    let recording = recorder.into_recording();

    for frame in 0..120_u64 {
        let t = frame as f32 * 0.02;
        recording.set_time_sequence("frame", frame as i64);
        let positions: [f32; 6] =
            std::array::from_fn(|joint| (t * 1.4 + joint as f32 * 0.31).sin() * 0.24);
        let velocities: [f32; 6] =
            std::array::from_fn(|joint| (t * 1.4 + joint as f32 * 0.31).cos() * 0.336);
        recording.log(
            "robot/frames/base_link",
            &rerun::Transform3D::from_translation_rotation(
                [0.0, 0.0, 0.0],
                rerun::Quaternion::from_xyzw([0.0, 0.0, 0.0, 1.0]),
            )
            .with_parent_frame("world")
            .with_child_frame(frame_id("base_link")),
        )?;
        // The sample uses the actual joint origins/axes from the RS URDF so
        // that the recorded meshes line up with the real model instead of
        // drawing a visually plausible but geometrically wrong chain.
        let origins = [
            (
                [-0.00034283, -0.00098683, 0.075],
                [0.0, 0.0, 0.0],
                [0.0, 0.0, -1.0],
            ),
            (
                [0.020343, 0.027237, 0.07],
                [-1.5708, 0.0, 0.0],
                [0.0, 0.0, 1.0],
            ),
            ([-0.236, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
            (
                [0.228, -0.072746, 0.0045],
                [0.0, 0.0, 0.0],
                [0.0, 0.0, -1.0],
            ),
            (
                [0.087, -0.048, -0.03075],
                [-1.5708, 0.0, 0.0],
                [0.0, 0.0, -1.0],
            ),
            ([0.0365, 0.0, 0.048], [0.0, 1.5708, 0.0], [0.0, 0.0, -1.0]),
        ];
        for (joint, (translation, rpy, axis)) in origins.iter().enumerate() {
            let parent = if joint == 0 {
                "base_link".to_owned()
            } else {
                format!("link{joint}")
            };
            let child = format!("link{}", joint + 1);
            recording.log(
                format!("robot/frames/{child}"),
                &rerun::Transform3D::from_translation_rotation(
                    *translation,
                    rerun::Quaternion::from_xyzw(quat_multiply(
                        quat_from_rpy(rpy[0], rpy[1], rpy[2]),
                        quat_axis_angle(*axis, positions[joint]),
                    )),
                )
                .with_parent_frame(frame_id(&parent))
                .with_child_frame(frame_id(&child)),
            )?;
        }
        let end_rotation = quat_from_rpy(3.1416, -1.5708, 0.0);
        recording.log(
            "robot/frames/gripper_end",
            &rerun::Transform3D::from_translation_rotation(
                [0.0, 0.0, 0.16621],
                rerun::Quaternion::from_xyzw(end_rotation),
            )
            .with_parent_frame(frame_id("link6"))
            .with_child_frame(frame_id("gripper_end")),
        )?;
        recording.log(
            "robot/frames/gripper_left",
            &rerun::Transform3D::from_translation_rotation(
                [-0.041939, -0.0000734, 0.0],
                rerun::Quaternion::from_xyzw([0.5, -0.5, 0.5000018, 0.4999982]),
            )
            .with_parent_frame(frame_id("gripper_end"))
            .with_child_frame(frame_id("gripper_left")),
        )?;
        recording.log(
            "robot/frames/gripper_right",
            &rerun::Transform3D::from_translation_rotation(
                [-0.041939, 0.0000734, 0.0],
                rerun::Quaternion::from_xyzw([-0.5, -0.5, -0.5000018, 0.4999982]),
            )
            .with_parent_frame(frame_id("gripper_end"))
            .with_child_frame(frame_id("gripper_right")),
        )?;
        for joint in 0..6 {
            recording.log(
                format!("robot/joints/joint_{}/position", joint + 1),
                &rerun::Scalars::single(positions[joint]),
            )?;
            recording.log(
                format!("robot/joints/joint_{}/velocity", joint + 1),
                &rerun::Scalars::single(velocities[joint]),
            )?;
        }
        for (joint, position) in positions.iter().copied().enumerate() {
            recording.log(
                format!("robot/trajectory/actual/joint_{}", joint + 1),
                &rerun::Scalars::single(position),
            )?;
        }
    }
    drop(recording);
    if let Some(url) = grpc_url {
        // A gRPC recording is streamed to the Viewer and intentionally does
        // not create the local output path.  Do not probe it as if this were
        // the offline .rrd mode (which used to produce ENOENT after a
        // successful online sample run).
        println!("recording=grpc url={url} frames=120");
    } else {
        let bytes = fs::metadata(&output)?.len();
        println!("recording={} bytes={bytes}", output.display());
    }
    Ok(())
}

#[cfg(feature = "rerun-recording")]
fn frame_id(value: &str) -> String {
    format!("robot::{value}")
}

#[cfg(feature = "rerun-recording")]
fn quat_from_rpy(roll: f32, pitch: f32, yaw: f32) -> [f32; 4] {
    let (sr, cr) = (roll * 0.5).sin_cos();
    let (sp, cp) = (pitch * 0.5).sin_cos();
    let (sy, cy) = (yaw * 0.5).sin_cos();
    [
        sr * cp * cy - cr * sp * sy,
        cr * sp * cy + sr * cp * sy,
        cr * cp * sy - sr * sp * cy,
        cr * cp * cy + sr * sp * sy,
    ]
}

#[cfg(feature = "rerun-recording")]
fn quat_axis_angle(axis: [f32; 3], angle: f32) -> [f32; 4] {
    let norm = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    if norm <= f32::EPSILON {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let (sin_half, cos_half) = (angle * 0.5).sin_cos();
    [
        axis[0] / norm * sin_half,
        axis[1] / norm * sin_half,
        axis[2] / norm * sin_half,
        cos_half,
    ]
}

#[cfg(feature = "rerun-recording")]
fn quat_multiply(lhs: [f32; 4], rhs: [f32; 4]) -> [f32; 4] {
    [
        lhs[3] * rhs[0] + lhs[0] * rhs[3] + lhs[1] * rhs[2] - lhs[2] * rhs[1],
        lhs[3] * rhs[1] - lhs[0] * rhs[2] + lhs[1] * rhs[3] + lhs[2] * rhs[0],
        lhs[3] * rhs[2] + lhs[0] * rhs[1] - lhs[1] * rhs[0] + lhs[2] * rhs[3],
        lhs[3] * rhs[3] - lhs[0] * rhs[0] - lhs[1] * rhs[1] - lhs[2] * rhs[2],
    ]
}
