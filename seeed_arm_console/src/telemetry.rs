//! Transport-neutral telemetry boundary for the desktop UI.
//!
//! A C++ gateway can later feed the same `TelemetryFrame` through gRPC or
//! WebSocket. The UI only consumes the latest frame and never depends on ROS 2
//! message types or a rendering process.

use std::{
    io::{BufRead, BufReader, Write},
    net::{SocketAddr, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use crossbeam_channel::Receiver;
use serde::Deserialize;

pub const JOINT_COUNT: usize = 6;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    Mock,
    MuJoCo,
    Ros2,
    Driver,
    Gateway,
}

impl SourceKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::MuJoCo => "mujoco",
            Self::Ros2 => "ros2",
            Self::Driver => "driver",
            Self::Gateway => "gateway",
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleQuality {
    Valid,
    Stale,
    Limited,
    Fault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkState {
    Offline,
    Connecting,
    Connected,
    Fault,
}

impl LinkState {
    fn as_u8(self) -> u8 {
        match self {
            Self::Offline => 0,
            Self::Connecting => 1,
            Self::Connected => 2,
            Self::Fault => 3,
        }
    }

    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Connecting,
            2 => Self::Connected,
            3 => Self::Fault,
            _ => Self::Offline,
        }
    }
}

impl SampleQuality {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::Stale => "stale",
            Self::Limited => "limited",
            Self::Fault => "fault",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transform3D {
    pub parent: String,
    pub child: String,
    pub translation_m: [f32; 3],
    pub rotation_xyzw: [f32; 4],
}

impl Default for Transform3D {
    fn default() -> Self {
        Self {
            parent: "world".to_owned(),
            child: "base".to_owned(),
            translation_m: [0.0; 3],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TrajectoryPoint {
    pub time_from_start_ns: u64,
    pub position_rad: Vec<f32>,
    pub velocity_rad_s: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ImageFrame {
    pub sensor: String,
    pub width: u32,
    pub height: u32,
    pub encoding: String,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct PointCloudFrame {
    pub sensor: String,
    pub positions: Vec<[f32; 3]>,
    /// Packed RGBA colors (`0xRRGGBBAA`), one per point when present.
    pub colors_rgba: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TelemetryFrame {
    pub sequence: u64,
    pub timestamp_ns: u64,
    pub source: SourceKind,
    pub quality: SampleQuality,
    pub joint_position: [f32; JOINT_COUNT],
    pub joint_velocity: [f32; JOINT_COUNT],
    pub tf: Vec<Transform3D>,
    pub planned_trajectory: Vec<TrajectoryPoint>,
    pub actual_trajectory: Vec<TrajectoryPoint>,
    pub images: Vec<ImageFrame>,
    pub point_clouds: Vec<PointCloudFrame>,
}

impl Default for TelemetryFrame {
    fn default() -> Self {
        Self {
            sequence: 0,
            timestamp_ns: 0,
            source: SourceKind::Mock,
            quality: SampleQuality::Stale,
            joint_position: [0.0; JOINT_COUNT],
            joint_velocity: [0.0; JOINT_COUNT],
            tf: Vec::new(),
            planned_trajectory: Vec::new(),
            actual_trajectory: Vec::new(),
            images: Vec::new(),
            point_clouds: Vec::new(),
        }
    }
}

/// UI-facing source contract. Implementations must be non-blocking.
pub trait TelemetrySource: Send {
    fn next(&mut self, elapsed_secs: f64, selected_joint: usize) -> Option<TelemetryFrame>;

    fn send_command(&self, _command: &str) -> Result<(), String> {
        Err("当前数据源不支持控制命令".to_owned())
    }

    fn link_state(&self) -> LinkState {
        LinkState::Connected
    }
}

/// Deterministic local source used until a C++ gateway is connected.
#[derive(Debug)]
pub struct MockTelemetrySource {
    sequence: u64,
    started_at: Instant,
}

impl Default for MockTelemetrySource {
    fn default() -> Self {
        Self {
            sequence: 0,
            started_at: Instant::now(),
        }
    }
}

impl TelemetrySource for MockTelemetrySource {
    fn next(&mut self, elapsed_secs: f64, _selected_joint: usize) -> Option<TelemetryFrame> {
        self.sequence = self.sequence.saturating_add(1);
        let elapsed = self.started_at.elapsed().as_nanos() as u64;
        let t = elapsed_secs.max(0.0);
        let mut position = [0.0; JOINT_COUNT];
        let mut velocity = [0.0; JOINT_COUNT];
        for joint in 0..JOINT_COUNT {
            let phase = t * 1.4 + joint as f64 * 0.31;
            position[joint] = (phase.sin() * 0.24) as f32;
            velocity[joint] = (phase.cos() * 0.336) as f32;
        }
        let mut planned = position;
        for joint in 0..JOINT_COUNT {
            planned[joint] += velocity[joint] * 0.2;
        }
        Some(TelemetryFrame {
            sequence: self.sequence,
            timestamp_ns: elapsed,
            source: SourceKind::Mock,
            quality: SampleQuality::Valid,
            joint_position: position,
            joint_velocity: velocity,
            tf: mock_tf(position),
            planned_trajectory: vec![TrajectoryPoint {
                time_from_start_ns: 200_000_000,
                position_rad: planned.to_vec(),
                velocity_rad_s: velocity.to_vec(),
            }],
            actual_trajectory: vec![TrajectoryPoint {
                time_from_start_ns: 0,
                position_rad: position.to_vec(),
                velocity_rad_s: velocity.to_vec(),
            }],
            images: Vec::new(),
            point_clouds: Vec::new(),
        })
    }
}

fn mock_tf(position: [f32; JOINT_COUNT]) -> Vec<Transform3D> {
    let lengths = [0.08_f32, 0.11, 0.10, 0.07, 0.05, 0.04];
    let mut transforms = vec![Transform3D {
        parent: "world".to_owned(),
        child: "base".to_owned(),
        ..Transform3D::default()
    }];
    let mut angle = 0.0_f32;
    for (index, length) in lengths.iter().enumerate() {
        angle += position[index];
        transforms.push(Transform3D {
            parent: if index == 0 {
                "base".to_owned()
            } else {
                format!("link{index}")
            },
            child: format!("link{}", index + 1),
            translation_m: [
                angle.cos() * length,
                angle.sin() * length,
                0.08 + index as f32 * 0.01,
            ],
            ..Transform3D::default()
        });
    }
    transforms.push(Transform3D {
        parent: "link6".to_owned(),
        child: "tool".to_owned(),
        translation_m: [0.0, 0.0, 0.02],
        ..Transform3D::default()
    });
    // Keep the local mock's fixed gripper geometry visible with the same
    // frame hierarchy as the URDF.  A real MuJoCo source replaces these with
    // the live prismatic-joint poses.
    transforms.push(Transform3D {
        parent: "tool".to_owned(),
        child: "gripper_left".to_owned(),
        translation_m: [-0.041939, -0.0000734, 0.0],
        rotation_xyzw: [0.5, -0.5, 0.5000018, 0.4999982],
    });
    transforms.push(Transform3D {
        parent: "tool".to_owned(),
        child: "gripper_right".to_owned(),
        translation_m: [-0.041939, 0.0000734, 0.0],
        rotation_xyzw: [-0.5, -0.5, -0.5000018, 0.4999982],
    });
    transforms
}

#[derive(Debug, Deserialize)]
struct JsonTelemetryFrame {
    sequence: u64,
    timestamp_ns: u64,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    quality: Option<String>,
    #[serde(alias = "joint_position_rad", default)]
    joint_position: Vec<f32>,
    #[serde(alias = "joint_velocity_rad_s", default)]
    joint_velocity: Vec<f32>,
    #[serde(default)]
    tf: Vec<JsonTransform3D>,
    #[serde(default)]
    planned_trajectory: Vec<JsonTrajectoryPoint>,
    #[serde(default)]
    actual_trajectory: Vec<JsonTrajectoryPoint>,
    #[serde(default)]
    images: Vec<JsonImageFrame>,
    #[serde(default)]
    point_clouds: Vec<JsonPointCloudFrame>,
}

#[derive(Debug, Deserialize)]
struct JsonImageFrame {
    #[serde(default)]
    sensor: String,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
    #[serde(default)]
    encoding: String,
    #[serde(default)]
    data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct JsonPointCloudFrame {
    #[serde(default)]
    sensor: String,
    #[serde(default)]
    positions: Vec<[f32; 3]>,
    #[serde(default)]
    colors_rgba: Vec<u32>,
}

#[derive(Debug, Deserialize)]
struct JsonTransform3D {
    #[serde(default)]
    parent: String,
    #[serde(default)]
    child: String,
    #[serde(default)]
    translation_x_m: f32,
    #[serde(default)]
    translation_y_m: f32,
    #[serde(default)]
    translation_z_m: f32,
    #[serde(default)]
    rotation_x: f32,
    #[serde(default)]
    rotation_y: f32,
    #[serde(default)]
    rotation_z: f32,
    #[serde(default = "default_rotation_w")]
    rotation_w: f32,
}

#[derive(Debug, Deserialize)]
struct JsonTrajectoryPoint {
    #[serde(default)]
    time_from_start_ns: u64,
    #[serde(alias = "position_rad", default)]
    position: Vec<f32>,
    #[serde(alias = "velocity_rad_s", default)]
    velocity: Vec<f32>,
}

fn default_rotation_w() -> f32 {
    1.0
}

impl JsonTelemetryFrame {
    fn into_frame(self) -> Option<TelemetryFrame> {
        if self.joint_position.len() != JOINT_COUNT || self.joint_velocity.len() != JOINT_COUNT {
            return None;
        }
        Some(TelemetryFrame {
            sequence: self.sequence,
            timestamp_ns: self.timestamp_ns,
            source: parse_source(self.source.as_deref()),
            quality: parse_quality(self.quality.as_deref()),
            joint_position: self.joint_position.try_into().ok()?,
            joint_velocity: self.joint_velocity.try_into().ok()?,
            tf: self
                .tf
                .into_iter()
                .map(|transform| Transform3D {
                    parent: transform.parent,
                    child: transform.child,
                    translation_m: [
                        transform.translation_x_m,
                        transform.translation_y_m,
                        transform.translation_z_m,
                    ],
                    rotation_xyzw: [
                        transform.rotation_x,
                        transform.rotation_y,
                        transform.rotation_z,
                        transform.rotation_w,
                    ],
                })
                .collect(),
            planned_trajectory: self
                .planned_trajectory
                .into_iter()
                .map(TrajectoryPoint::from_json)
                .collect(),
            actual_trajectory: self
                .actual_trajectory
                .into_iter()
                .map(TrajectoryPoint::from_json)
                .collect(),
            images: self
                .images
                .into_iter()
                .map(|image| ImageFrame {
                    sensor: image.sensor,
                    width: image.width,
                    height: image.height,
                    encoding: image.encoding,
                    data: image.data,
                })
                .collect(),
            point_clouds: self
                .point_clouds
                .into_iter()
                .map(|cloud| PointCloudFrame {
                    sensor: cloud.sensor,
                    positions: cloud.positions,
                    colors_rgba: cloud.colors_rgba,
                })
                .collect(),
        })
    }
}

impl TrajectoryPoint {
    fn from_json(point: JsonTrajectoryPoint) -> Self {
        Self {
            time_from_start_ns: point.time_from_start_ns,
            position_rad: point.position,
            velocity_rad_s: point.velocity,
        }
    }
}

fn parse_source(value: Option<&str>) -> SourceKind {
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "mujoco" => SourceKind::MuJoCo,
        "ros2" => SourceKind::Ros2,
        "driver" => SourceKind::Driver,
        "gateway" => SourceKind::Gateway,
        "mock" => SourceKind::Mock,
        _ => SourceKind::Gateway,
    }
}

fn parse_quality(value: Option<&str>) -> SampleQuality {
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "stale" => SampleQuality::Stale,
        "limited" => SampleQuality::Limited,
        "fault" => SampleQuality::Fault,
        _ => SampleQuality::Valid,
    }
}

/// Bounded channel adapter for a future gRPC/WebSocket worker.
///
/// `next` drains all pending samples and returns only the newest one. This
/// keeps a 60 Hz UI responsive when the gateway publishes at a higher rate.
#[allow(dead_code)]
pub struct ChannelTelemetrySource {
    rx: Receiver<TelemetryFrame>,
    latest: Option<TelemetryFrame>,
}

#[allow(dead_code)]
impl ChannelTelemetrySource {
    pub fn new(rx: Receiver<TelemetryFrame>) -> Self {
        Self { rx, latest: None }
    }
}

#[allow(dead_code)]
impl TelemetrySource for ChannelTelemetrySource {
    fn next(&mut self, _elapsed_secs: f64, _selected_joint: usize) -> Option<TelemetryFrame> {
        for frame in self.rx.try_iter() {
            self.latest = Some(frame);
        }
        self.latest.clone()
    }
}

/// Newline-delimited JSON source for the headless C++ gateway.
///
/// The worker owns all blocking socket I/O. The egui thread only drains the
/// bounded channel, so a stalled gateway cannot freeze rendering.
pub struct TcpTelemetrySource {
    inner: ChannelTelemetrySource,
    state: Arc<AtomicU8>,
    stop: Arc<AtomicBool>,
    command_stream: Arc<Mutex<TcpStream>>,
    _worker: Option<thread::JoinHandle<()>>,
}

impl TcpTelemetrySource {
    pub fn connect(endpoint: &str) -> Result<Self, String> {
        let (tx, rx) = crossbeam_channel::bounded(128);
        let state = Arc::new(AtomicU8::new(LinkState::Connecting.as_u8()));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_state = Arc::clone(&state);
        let worker_stop = Arc::clone(&stop);
        let address: SocketAddr = endpoint
            .parse()
            .map_err(|_| format!("无效的 TCP 地址: {endpoint}"))?;
        let stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
            .map_err(|error| format!("连接网关失败: {error}"))?;
        let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
        let command_stream = stream
            .try_clone()
            .map_err(|error| format!("无法创建 TCP 控制通道: {error}"))?;
        let _ = command_stream.set_write_timeout(Some(Duration::from_millis(500)));
        let worker = thread::Builder::new()
            .name("telemetry-tcp".to_owned())
            .spawn(move || tcp_worker(stream, tx, worker_state, worker_stop))
            .map_err(|error| format!("无法启动 TCP 数据线程: {error}"))?;

        Ok(Self {
            inner: ChannelTelemetrySource::new(rx),
            state,
            stop,
            command_stream: Arc::new(Mutex::new(command_stream)),
            _worker: Some(worker),
        })
    }
}

impl TelemetrySource for TcpTelemetrySource {
    fn next(&mut self, elapsed_secs: f64, selected_joint: usize) -> Option<TelemetryFrame> {
        self.inner.next(elapsed_secs, selected_joint)
    }

    fn send_command(&self, command: &str) -> Result<(), String> {
        let mut stream = self
            .command_stream
            .lock()
            .map_err(|_| "TCP 控制通道锁定失败".to_owned())?;
        stream
            .write_all(format!("{}\n", command.trim()).as_bytes())
            .map_err(|error| format!("发送控制命令失败: {error}"))
    }

    fn link_state(&self) -> LinkState {
        LinkState::from_u8(self.state.load(Ordering::Acquire))
    }
}

impl Drop for TcpTelemetrySource {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        // The read timeout in `tcp_worker` bounds shutdown latency.
        if let Some(worker) = self._worker.take() {
            let _ = worker.join();
        }
    }
}

fn tcp_worker(
    stream: TcpStream,
    tx: crossbeam_channel::Sender<TelemetryFrame>,
    state: Arc<AtomicU8>,
    stop: Arc<AtomicBool>,
) {
    state.store(LinkState::Connected.as_u8(), Ordering::Release);
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        if stop.load(Ordering::Acquire) {
            break;
        }
        match line {
            Ok(line) if !line.trim().is_empty() => {
                if let Ok(raw) = serde_json::from_str::<JsonTelemetryFrame>(&line)
                    && let Some(frame) = raw.into_frame()
                {
                    let _ = tx.try_send(frame);
                }
            }
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                continue;
            }
            Err(_) => break,
        }
    }
    if !stop.load(Ordering::Acquire) {
        state.store(LinkState::Offline.as_u8(), Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use crossbeam_channel::bounded;
    use std::{io::Write, net::TcpListener, thread, time::Duration};

    use super::{
        ChannelTelemetrySource, JOINT_COUNT, JsonTelemetryFrame, LinkState, MockTelemetrySource,
        SampleQuality, SourceKind, TcpTelemetrySource, TelemetryFrame, TelemetrySource,
    };

    #[test]
    fn mock_frames_are_complete_and_monotonic() {
        let mut source = MockTelemetrySource::default();
        let first = source.next(0.0, 0).expect("first frame");
        let second = source.next(0.1, 0).expect("second frame");
        assert_eq!(first.source, SourceKind::Mock);
        assert_eq!(first.quality, SampleQuality::Valid);
        assert_eq!(first.joint_position.len(), JOINT_COUNT);
        assert_eq!(first.tf.len(), 10);
        assert_eq!(first.planned_trajectory.len(), 1);
        assert!(second.sequence > first.sequence);
    }

    #[test]
    fn channel_source_keeps_only_latest_frame() {
        let (tx, rx) = bounded(4);
        tx.send(TelemetryFrame {
            sequence: 1,
            ..TelemetryFrame::default()
        })
        .unwrap();
        tx.send(TelemetryFrame {
            sequence: 2,
            ..TelemetryFrame::default()
        })
        .unwrap();
        let mut source = ChannelTelemetrySource::new(rx);
        assert_eq!(source.next(0.0, 0).unwrap().sequence, 2);
    }

    #[test]
    fn json_frame_accepts_protocol_field_names() {
        let raw = r#"{
            "sequence": 7,
            "timestamp_ns": 42,
            "source": "mujoco",
            "quality": "valid",
            "joint_position_rad": [0, 1, 2, 3, 4, 5],
            "joint_velocity_rad_s": [0, 0, 0, 0, 0, 0],
            "tf": [{"parent":"world","child":"tool","translation_x_m":0.4,"rotation_w":1}],
            "planned_trajectory": [{"time_from_start_ns":100,"position_rad":[1,2,3,4,5,6],"velocity_rad_s":[0,0,0,0,0,0]}],
            "actual_trajectory": [{"time_from_start_ns":50,"position_rad":[0,1,2,3,4,5],"velocity_rad_s":[0,0,0,0,0,0]}]
        }"#;
        let parsed: JsonTelemetryFrame = serde_json::from_str(raw).unwrap();
        let frame = parsed.into_frame().unwrap();
        assert_eq!(frame.source, SourceKind::MuJoCo);
        assert_eq!(frame.joint_position[5], 5.0);
        assert_eq!(frame.tf.len(), 1);
        assert_eq!(frame.tf[0].child, "tool");
        assert_eq!(frame.planned_trajectory[0].time_from_start_ns, 100);
    }

    #[test]
    fn json_frame_accepts_sensor_payloads() {
        let raw = r#"{
            "sequence": 8,
            "timestamp_ns": 43,
            "joint_position_rad": [0, 1, 2, 3, 4, 5],
            "joint_velocity_rad_s": [0, 0, 0, 0, 0, 0],
            "images": [{"sensor":"front","width":2,"height":1,"encoding":"png","data":[137,80,78,71]}],
            "point_clouds": [{"sensor":"depth","positions":[[0,0,1],[1,0,1]],"colors_rgba":[4278190335,16711935]}]
        }"#;
        let parsed: JsonTelemetryFrame = serde_json::from_str(raw).unwrap();
        let frame = parsed.into_frame().unwrap();
        assert_eq!(frame.images[0].sensor, "front");
        assert_eq!(frame.images[0].data.len(), 4);
        assert_eq!(frame.point_clouds[0].positions.len(), 2);
        assert_eq!(frame.point_clouds[0].colors_rgba.len(), 2);
    }

    #[test]
    fn link_state_values_are_stable() {
        assert_eq!(
            LinkState::from_u8(LinkState::Connected.as_u8()),
            LinkState::Connected
        );
    }

    #[test]
    fn tcp_source_receives_latest_json_frame() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(
                    br#"{"sequence":9,"timestamp_ns":99,"source":"mujoco","quality":"valid","joint_position_rad":[1,2,3,4,5,6],"joint_velocity_rad_s":[0,0,0,0,0,0]}
                    "#,
                )
                .unwrap();
            thread::sleep(Duration::from_millis(50));
        });

        let mut source = TcpTelemetrySource::connect(&endpoint.to_string()).unwrap();
        let mut received = None;
        for _ in 0..50 {
            if let Some(frame) = source.next(0.0, 0) {
                received = Some(frame);
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        let frame = received.expect("TCP frame");
        assert_eq!(frame.sequence, 9);
        assert!(matches!(
            source.link_state(),
            LinkState::Connected | LinkState::Offline
        ));
        drop(source);
        server.join().unwrap();
    }
}
