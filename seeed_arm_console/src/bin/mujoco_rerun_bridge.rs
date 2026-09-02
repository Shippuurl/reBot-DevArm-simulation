#[cfg(feature = "rerun-recording")]
use rebot_sim_viewer::telemetry::{MAX_POINT_CLOUD_POINTS, MAX_POINT_CLOUDS_PER_FRAME};
#[cfg(feature = "rerun-recording")]
use std::io::{BufRead, BufReader};
#[cfg(feature = "rerun-recording")]
use std::net::TcpStream;
#[cfg(feature = "rerun-recording")]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "rerun-recording")]
fn trajectory_positions(value: &serde_json::Value, field: &str) -> Vec<Vec<f64>> {
    value
        .get(field)
        .and_then(|v| v.as_array())
        .map(|points| {
            points
                .iter()
                .filter_map(|point| {
                    point
                        .get("position_rad")
                        .and_then(|v| v.as_array())
                        .map(|values| values.iter().filter_map(|v| v.as_f64()).collect())
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(feature = "rerun-recording")]
fn frame_id(value: &str) -> String {
    if value == "world" {
        return "world".to_owned();
    }
    let canonical = match value {
        "base" => "base_link",
        "tool" => "gripper_end",
        other => other,
    };
    format!("robot::{canonical}")
}

#[cfg(feature = "rerun-recording")]
fn sensor_entity(value: &str) -> String {
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

#[cfg(feature = "rerun-recording")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The canonical gateway port is gRPC (50051). This bridge intentionally
    // targets the explicitly legacy JSON adapter on 50052.
    let gateway = std::env::var("MUJOCO_GATEWAY_ADDR").unwrap_or_else(|_| "127.0.0.1:50052".into());
    let rerun_url = std::env::var("RERUN_GRPC_URL")
        .unwrap_or_else(|_| "rerun+http://127.0.0.1:9876/proxy".into());
    let rec = rerun::RecordingStreamBuilder::new("mujoco_gateway").connect_grpc_opts(rerun_url)?;
    let stream = TcpStream::connect(gateway)?;
    for line in BufReader::new(stream).lines() {
        let value: serde_json::Value = serde_json::from_str(&line?)?;
        let sequence = value.get("sequence").and_then(|v| v.as_u64()).unwrap_or(0);
        let sim_time_ns = value
            .get("sim_time_ns")
            .or_else(|| value.get("timestamp_ns"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        rec.set_time_sequence("frame", sequence as i64);
        rec.set_time_sequence("sim_time", sim_time_ns as i64);
        let wall_time_ns = value
            .get("wall_time_ns")
            .and_then(|v| v.as_u64())
            .or_else(|| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_nanos() as u64)
            });
        if let Some(wall_time_ns) = wall_time_ns {
            rec.set_timestamp_nanos_since_epoch("wall_time", wall_time_ns as i64);
        }
        // `time_from_start_ns` is relative to a trajectory and may reset or
        // overlap while the live recording continues.  Clear any stale value
        // and keep all live entities on the monotonic frame/sim timelines.
        rec.disable_timeline("trajectory_time");
        for (kind, field) in [
            ("position", "joint_position_rad"),
            ("velocity", "joint_velocity_rad_s"),
        ] {
            if let Some(values) = value.get(field).and_then(|v| v.as_array()) {
                for (index, item) in values.iter().enumerate() {
                    if let Some(number) = item.as_f64() {
                        rec.log(
                            format!("robot/joints/joint_{}/{kind}", index + 1),
                            &rerun::Scalars::single(number as f32),
                        )?;
                    }
                }
            }
        }
        if let Some(transforms) = value.get("tf").and_then(|v| v.as_array()) {
            for transform in transforms {
                let number =
                    |name: &str| transform.get(name).and_then(|v| v.as_f64()).unwrap_or(0.0);
                let parent = transform
                    .get("parent")
                    .and_then(|v| v.as_str())
                    .unwrap_or("world");
                let child = transform
                    .get("child")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let translation = [
                    number("translation_x_m"),
                    number("translation_y_m"),
                    number("translation_z_m"),
                ];
                let rotation = rerun::Quaternion::from_xyzw([
                    number("rotation_x") as f32,
                    number("rotation_y") as f32,
                    number("rotation_z") as f32,
                    transform
                        .get("rotation_w")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.0) as f32,
                ]);
                rec.log(
                    format!("robot/frames/{child}"),
                    &rerun::Transform3D::from_translation_rotation(translation, rotation)
                        .with_parent_frame(frame_id(parent))
                        .with_child_frame(frame_id(child)),
                )?;
            }
        }
        for (trajectory_name, field) in [
            ("planned_trajectory", "planned_trajectory"),
            ("actual_trajectory", "actual_trajectory"),
        ] {
            if let Some(points) = value.get(field).and_then(|v| v.as_array()) {
                if let Some(point) = points.last() {
                    let timestamp = point
                        .get("time_from_start_ns")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(sequence);
                    if let Some(positions) = point.get("position_rad").and_then(|v| v.as_array()) {
                        for (index, item) in positions.iter().enumerate() {
                            if let Some(number) = item.as_f64() {
                                rec.log(
                                    format!(
                                        "planning/{trajectory_name}/joint_position_rad/{}",
                                        index + 1
                                    ),
                                    &rerun::Scalars::single(number as f32),
                                )?;
                            }
                        }
                    }
                    rec.log(
                        format!("planning/{trajectory_name}/time_from_start_ns"),
                        &rerun::Scalars::single(timestamp as f64),
                    )?;
                }
            }
        }
        let planned = trajectory_positions(&value, "planned_trajectory");
        let actual = trajectory_positions(&value, "actual_trajectory");
        if let (Some(planned_last), Some(actual_last)) = (planned.last(), actual.last()) {
            let errors: Vec<f32> = planned_last
                .iter()
                .zip(actual_last)
                .map(|(planned, actual)| (actual - planned) as f32)
                .collect();
            if !errors.is_empty() {
                let rms = (errors.iter().map(|error| error * error).sum::<f32>()
                    / errors.len() as f32)
                    .sqrt();
                let max_abs = errors
                    .iter()
                    .map(|error| error.abs())
                    .fold(0.0_f32, f32::max);
                rec.log(
                    "diagnostics/trajectory_error/joint_rad",
                    &rerun::Scalars::new(errors.iter().copied()),
                )?;
                rec.log(
                    "diagnostics/trajectory_error/rms_rad",
                    &rerun::Scalars::single(rms),
                )?;
                rec.log(
                    "diagnostics/trajectory_error/max_abs_rad",
                    &rerun::Scalars::single(max_abs),
                )?;
            }
        }
        for cloud in value
            .get("point_clouds")
            .and_then(|payload| payload.as_array())
            .into_iter()
            .flatten()
            .take(MAX_POINT_CLOUDS_PER_FRAME)
        {
            let Some(raw_positions) = cloud
                .get("positions")
                .and_then(|payload| payload.as_array())
            else {
                continue;
            };
            let stride = raw_positions.len().div_ceil(MAX_POINT_CLOUD_POINTS).max(1);
            let positions: Vec<[f32; 3]> = raw_positions
                .iter()
                .enumerate()
                .filter(|(index, _)| index % stride == 0)
                .take(MAX_POINT_CLOUD_POINTS)
                .filter_map(|(_, point)| {
                    let values = point.as_array()?;
                    let xyz = [
                        values.first()?.as_f64()? as f32,
                        values.get(1)?.as_f64()? as f32,
                        values.get(2)?.as_f64()? as f32,
                    ];
                    xyz.iter().all(|value| value.is_finite()).then_some(xyz)
                })
                .collect();
            if positions.is_empty() {
                continue;
            }
            let sensor = cloud
                .get("sensor")
                .and_then(|payload| payload.as_str())
                .unwrap_or("depth");
            let point_count = positions.len();
            let mut points = rerun::Points3D::new(positions);
            if let Some(raw_colors) = cloud
                .get("colors_rgba")
                .and_then(|payload| payload.as_array())
                && raw_colors.len() == raw_positions.len()
            {
                let colors: Vec<u32> = raw_colors
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| index % stride == 0)
                    .take(MAX_POINT_CLOUD_POINTS)
                    .filter_map(|(_, color)| color.as_u64().map(|value| value as u32))
                    .collect();
                if colors.len() == point_count {
                    points = points.with_colors(colors);
                }
            }
            rec.log(format!("sensors/{}/points", sensor_entity(sensor)), &points)?;
        }
        if let Some(contacts) = value.get("contacts").and_then(|v| v.as_array()) {
            let mut max_force = 0.0_f32;
            let mut min_distance = f32::INFINITY;
            for contact in contacts {
                if let Some(force) = contact.get("normal_force_n").and_then(|v| v.as_f64()) {
                    max_force = max_force.max(force as f32);
                }
                if let Some(distance) = contact.get("distance_m").and_then(|v| v.as_f64()) {
                    min_distance = min_distance.min(distance as f32);
                }
            }
            rec.log(
                "diagnostics/contact/count",
                &rerun::Scalars::single(contacts.len() as f32),
            )?;
            rec.log(
                "diagnostics/contact/max_normal_force_n",
                &rerun::Scalars::single(max_force),
            )?;
            if min_distance.is_finite() {
                rec.log(
                    "diagnostics/contact/min_distance_m",
                    &rerun::Scalars::single(min_distance),
                )?;
            }
        }
    }
    Ok(())
}

#[cfg(not(feature = "rerun-recording"))]
fn main() {
    eprintln!(
        "请使用 `cargo run --features rerun-recording --bin mujoco_rerun_bridge` 启动 JSON→Rerun 转发器"
    );
}
