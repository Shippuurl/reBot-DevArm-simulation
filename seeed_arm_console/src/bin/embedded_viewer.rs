//! reBot-DevArm 仿真 Viewer（Rerun 主生命周期 + 自定义控制台）。
//!
//! 外层由本项目的 `eframe` 应用管理窗口，内层持有 `re_viewer::App`；
//! ArmGateway gRPC 状态和遥测记录在后台任务中接入，保持 Viewer 主生命周期单一。

#[cfg(not(feature = "embedded-viewer"))]
fn main() {
    eprintln!("请使用 `cargo run --features embedded-viewer` 启动 Rerun 仿真 Viewer");
}

#[cfg(feature = "grpc-client")]
use rebot_arm_sdk as grpc_client;
#[cfg(feature = "grpc-client")]
use rebot_sim_viewer::rerun_bridge::RerunRecorder;
#[cfg(feature = "grpc-client")]
use rebot_sim_viewer::telemetry::{
    MAX_IMAGE_BYTES, MAX_IMAGES_PER_FRAME, MAX_POINT_CLOUD_POINTS, MAX_POINT_CLOUDS_PER_FRAME,
};
#[cfg(feature = "embedded-viewer")]
use rerun::external::{eframe, egui, re_crash_handler, re_grpc_server, re_log, re_viewer, tokio};
#[cfg(feature = "grpc-client")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "embedded-viewer")]
#[path = "../design.rs"]
mod design;
#[cfg(feature = "embedded-viewer")]
struct RerunRuntimeConfig {
    memory_limit: re_viewer::external::re_memory::MemoryLimit,
    telemetry_rate_hz: u32,
}

#[cfg(feature = "embedded-viewer")]
fn rerun_runtime_config() -> RerunRuntimeConfig {
    let debug_mode = std::env::var("RERUN_DEBUG_MODE").ok().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "on" | "yes"
        )
    });
    let default_history_limit = if debug_mode { "64MiB" } else { "512MiB" };
    let memory_limit = match std::env::var("RERUN_HISTORY_LIMIT") {
        Ok(value) => match re_viewer::external::re_memory::MemoryLimit::parse(value.trim()) {
            Ok(limit) => limit,
            Err(error) => {
                eprintln!(
                    "RERUN_HISTORY_LIMIT={value:?} 无效: {error}；回退到 {default_history_limit}"
                );
                re_viewer::external::re_memory::MemoryLimit::parse(default_history_limit)
                    .expect("built-in Rerun history limit is valid")
            }
        },
        Err(_) => re_viewer::external::re_memory::MemoryLimit::parse(default_history_limit)
            .expect("built-in Rerun history limit is valid"),
    };
    let default_rate = if debug_mode { 30 } else { 100 };
    let telemetry_rate_hz = std::env::var("RERUN_TELEMETRY_RATE_HZ")
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(default_rate)
        .clamp(1, 200);
    RerunRuntimeConfig {
        memory_limit,
        telemetry_rate_hz,
    }
}

#[cfg(feature = "embedded-viewer")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let main_thread_token = re_viewer::MainThreadToken::i_promise_i_am_on_the_main_thread();

    re_log::setup_logging();
    re_crash_handler::install_crash_handlers(re_viewer::build_info());

    let rerun_config = rerun_runtime_config();
    let rerun_status = format!(
        "Rerun 历史预算: {}，遥测: {} Hz",
        rerun_config.memory_limit, rerun_config.telemetry_rate_hz
    );

    // 让 C++/Rust 数据源可以通过标准 Rerun gRPC 端口推送到内嵌 Viewer。
    let (log_rx, _grpc_server_handle) = re_grpc_server::spawn_with_recv(
        "127.0.0.1:9876".parse()?,
        re_grpc_server::ServerOptions {
            memory_limit: rerun_config.memory_limit,
            ..Default::default()
        },
        re_grpc_server::shutdown::never(),
    );

    let mut native_options = re_viewer::native::eframe_options(None);
    native_options.viewport = native_options
        .viewport
        .with_app_id("rebot_dev_arm_sim_viewer");

    let startup_options = re_viewer::StartupOptions::default();
    let app_environment = re_viewer::AppEnvironment::Custom("reBot-DevArm Simulation".to_owned());
    #[cfg(feature = "grpc-client")]
    let telemetry_status = Arc::new(Mutex::new(TelemetryStatus::default()));
    #[cfg(feature = "grpc-client")]
    let command_status = Arc::new(Mutex::new(String::from("未发送命令")));
    #[cfg(feature = "grpc-client")]
    let session_state = Arc::new(Mutex::new(None::<String>));
    #[cfg(feature = "grpc-client")]
    let gateway_endpoint = std::env::var("ARM_GATEWAY_GRPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:50051".to_owned());
    #[cfg(feature = "grpc-client")]
    let telemetry_recorder = rerun::RecordingStreamBuilder::new("arm_gateway_grpc")
        .connect_grpc_opts(
            std::env::var("RERUN_GRPC_URL")
                .unwrap_or_else(|_| "rerun+http://127.0.0.1:9876/proxy".to_owned()),
        )
        .ok()
        .map(RerunRecorder::from_recording);
    #[cfg(feature = "grpc-client")]
    let model_status = if let Some(recorder) = telemetry_recorder.as_ref() {
        let model_root = std::env::var_os("ROBOT_MODEL_ROOT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/robot/b601_rs")
            });
        match recorder.log_model_assets(&model_root) {
            Ok(count) => format!("模型: 已加载 {count} 个网格"),
            Err(error) => format!("模型: 加载失败: {error}"),
        }
    } else {
        "模型: Rerun 连接失败".to_owned()
    };
    #[cfg(feature = "grpc-client")]
    let (gateway_status, _initial_session_id) =
        match grpc_client::ArmGatewayClient::connect(gateway_endpoint.clone(), "rebot_sim_viewer")
            .await
        {
            Ok(mut client) => match client.handshake().await {
                Ok(reply) => {
                    if let Ok(mut session) = session_state.lock() {
                        *session = Some(reply.session_id.clone());
                    }
                    (
                        format!("已连接 {} (dof={})", reply.session_id, reply.dof),
                        Some(reply.session_id),
                    )
                }
                Err(error) => (format!("未连接: {error}"), None),
            },
            Err(error) => (format!("未连接: {error}"), None),
        };
    #[cfg(feature = "grpc-client")]
    {
        let status = Arc::clone(&telemetry_status);
        let endpoint = gateway_endpoint.clone();
        let session_state_for_task = Arc::clone(&session_state);
        let recorder = telemetry_recorder;
        let telemetry_rate_hz = rerun_config.telemetry_rate_hz;
        tokio::spawn(async move {
            let mut retry_delay_ms = 250_u64;
            loop {
                let current_session = session_state_for_task
                    .lock()
                    .ok()
                    .and_then(|session| session.clone());
                let mut client = match grpc_client::ArmGatewayClient::connect(
                    endpoint.clone(),
                    "rebot_sim_viewer",
                )
                .await
                {
                    Ok(client) => client,
                    Err(error) => {
                        if let Ok(mut latest) = status.lock() {
                            latest.link_state = format!("连接失败，重连中: {error}");
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(retry_delay_ms)).await;
                        retry_delay_ms = (retry_delay_ms.saturating_mul(2)).min(5_000);
                        continue;
                    }
                };
                match current_session {
                    Some(session_id) => {
                        client.set_session_id(session_id.clone());
                    }
                    None => match client.handshake().await {
                        Ok(reply) => {
                            let session_id = reply.session_id;
                            if let Ok(mut session) = session_state_for_task.lock() {
                                *session = Some(session_id.clone());
                            }
                            retry_delay_ms = 250;
                        }
                        Err(error) => {
                            if let Ok(mut latest) = status.lock() {
                                latest.link_state = format!("握手失败，重连中: {error}");
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(retry_delay_ms))
                                .await;
                            retry_delay_ms = (retry_delay_ms.saturating_mul(2)).min(5_000);
                            continue;
                        }
                    },
                }
                match client.subscribe_telemetry(telemetry_rate_hz).await {
                    Ok(mut stream) => {
                        retry_delay_ms = 250;
                        if let Ok(mut latest) = status.lock() {
                            latest.link_state = "已连接".to_owned();
                        }
                        loop {
                            match stream.message().await {
                                Ok(Some(frame)) => {
                                    if let Ok(mut latest) = status.lock() {
                                        latest.sequence = frame.sequence;
                                        latest.quality = frame.quality.clone();
                                        latest.joint_count = frame.joint_position_rad.len();
                                        latest.link_state = "已连接".to_owned();
                                    }
                                    if let Some(recorder) = recorder.as_ref() {
                                        record_gateway_frame(recorder.recording(), &frame);
                                    }
                                }
                                Ok(None) => break,
                                Err(error) => {
                                    if let Ok(mut latest) = status.lock() {
                                        latest.link_state = format!("流错误，重连中: {error}");
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    Err(error) => {
                        if let Ok(mut session) = session_state_for_task.lock() {
                            *session = None;
                        }
                        if let Ok(mut latest) = status.lock() {
                            latest.link_state = format!("订阅失败，重连中: {error}");
                        }
                    }
                }
                if let Ok(mut session) = session_state_for_task.lock() {
                    *session = None;
                }
                tokio::time::sleep(std::time::Duration::from_millis(retry_delay_ms)).await;
                retry_delay_ms = (retry_delay_ms.saturating_mul(2)).min(5_000);
            }
        });
    }
    #[cfg(not(feature = "grpc-client"))]
    let gateway_status = "未启用 gRPC 客户端".to_owned();
    #[cfg(not(feature = "grpc-client"))]
    let telemetry_status = ();

    eframe::run_native(
        "reBot-DevArm Simulation",
        native_options,
        Box::new(move |cc| {
            re_viewer::customize_eframe_and_setup_renderer(cc)?;
            // Rerun owns the eframe lifecycle, but custom panels still need
            // the bundled CJK-capable font fallback installed on its context.
            design::configure_fonts(&cc.egui_ctx);

            let mut rerun_app = re_viewer::App::new(
                main_thread_token,
                re_viewer::build_info(),
                app_environment,
                startup_options,
                cc,
                None,
                re_viewer::AsyncRuntimeHandle::from_current_tokio_runtime_or_wasmbindgen()?,
            );
            rerun_app.app_options_mut().memory_limit = rerun_config.memory_limit;
            rerun_app.add_log_receiver(log_rx);

            Ok(Box::new(EmbeddedViewerApp {
                rerun_app,
                panel_enabled: true,
                gateway_status,
                rerun_status,
                #[cfg(feature = "grpc-client")]
                model_status,
                #[cfg(feature = "grpc-client")]
                gateway_endpoint,
                #[cfg(feature = "grpc-client")]
                session_id: session_state,
                #[cfg(feature = "grpc-client")]
                speed_scale: 1.0,
                #[cfg(feature = "grpc-client")]
                command_status,
                telemetry_status,
            }))
        }),
    )?;

    Ok(())
}

#[cfg(feature = "embedded-viewer")]
struct EmbeddedViewerApp {
    rerun_app: re_viewer::App,
    panel_enabled: bool,
    gateway_status: String,
    rerun_status: String,
    #[cfg(feature = "grpc-client")]
    model_status: String,
    #[cfg(feature = "grpc-client")]
    gateway_endpoint: String,
    #[cfg(feature = "grpc-client")]
    session_id: Arc<Mutex<Option<String>>>,
    #[cfg(feature = "grpc-client")]
    speed_scale: f32,
    #[cfg(feature = "grpc-client")]
    command_status: Arc<Mutex<String>>,
    #[cfg(feature = "grpc-client")]
    telemetry_status: Arc<Mutex<TelemetryStatus>>,
}

#[cfg(feature = "grpc-client")]
struct TelemetryStatus {
    sequence: u64,
    quality: String,
    joint_count: usize,
    link_state: String,
}

#[cfg(feature = "grpc-client")]
impl Default for TelemetryStatus {
    fn default() -> Self {
        Self {
            sequence: 0,
            quality: "STALE".to_owned(),
            joint_count: 0,
            link_state: "未连接".to_owned(),
        }
    }
}

#[cfg(feature = "grpc-client")]
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

#[cfg(feature = "grpc-client")]
fn record_gateway_frame(recorder: &rerun::RecordingStream, frame: &grpc_client::TelemetryFrame) {
    recorder.set_time_sequence("frame", frame.sequence as i64);
    recorder.set_time_sequence(
        "sim_time",
        (if frame.sim_time_ns == 0 {
            frame.timestamp_ns
        } else {
            frame.sim_time_ns
        }) as i64,
    );
    if frame.wall_time_ns > 0 {
        recorder.set_timestamp_nanos_since_epoch("wall_time", frame.wall_time_ns as i64);
    }
    for (index, value) in frame.joint_position_rad.iter().enumerate() {
        let _ = recorder.log(
            format!("robot/joints/joint_{}/position", index + 1),
            &rerun::Scalars::single(*value as f32),
        );
    }
    for (index, value) in frame.joint_velocity_rad_s.iter().enumerate() {
        let _ = recorder.log(
            format!("robot/joints/joint_{}/velocity", index + 1),
            &rerun::Scalars::single(*value as f32),
        );
    }
    for transform in &frame.tf {
        let rotation = rerun::Quaternion::from_xyzw([
            transform.rotation_xyzw[0] as f32,
            transform.rotation_xyzw[1] as f32,
            transform.rotation_xyzw[2] as f32,
            transform.rotation_xyzw[3] as f32,
        ]);
        let translation = transform.translation_m;
        let _ = recorder.log(
            format!("robot/frames/{}", transform.child),
            &rerun::Transform3D::from_translation_rotation(translation, rotation)
                .with_parent_frame(frame_id(&transform.parent))
                .with_child_frame(frame_id(&transform.child)),
        );
    }
    for (trajectory_name, points) in [
        ("planned_trajectory", &frame.planned_trajectory),
        ("actual_trajectory", &frame.actual_trajectory),
    ] {
        if let Some(point) = points.last() {
            recorder.set_time_sequence("trajectory_time", point.time_from_start_ns as i64);
            for (index, value) in point.position_rad.iter().enumerate() {
                let _ = recorder.log(
                    format!(
                        "planning/{trajectory_name}/joint_position_rad/{}",
                        index + 1
                    ),
                    &rerun::Scalars::single(*value as f32),
                );
            }
        }
    }
    {
        let max_force = frame
            .contacts
            .iter()
            .map(|contact| contact.normal_force_n as f32)
            .fold(0.0_f32, f32::max);
        let min_distance = frame
            .contacts
            .iter()
            .map(|contact| contact.distance_m as f32)
            .fold(f32::INFINITY, f32::min);
        let _ = recorder.log(
            "diagnostics/contact/count",
            &rerun::Scalars::single(frame.contacts.len() as f32),
        );
        let _ = recorder.log(
            "diagnostics/contact/max_normal_force_n",
            &rerun::Scalars::single(max_force),
        );
        if !frame.contacts.is_empty() && min_distance.is_finite() {
            let _ = recorder.log(
                "diagnostics/contact/min_distance_m",
                &rerun::Scalars::single(min_distance),
            );
        }
    }
    for image in frame.images.iter().take(MAX_IMAGES_PER_FRAME) {
        if image.data.len() > MAX_IMAGE_BYTES
            || image.width > 4096
            || image.height > 4096
            || image.data.is_empty()
        {
            continue;
        }
        let path = format!("sensors/{}/image", sensor_entity(&image.sensor));
        let encoded = rerun::EncodedImage::from_file_contents(image.data.clone());
        let _ = recorder.log(path, &encoded);
    }
    for cloud in frame.point_clouds.iter().take(MAX_POINT_CLOUDS_PER_FRAME) {
        let raw_count = cloud.positions_xyz.len();
        if raw_count == 0 {
            continue;
        }
        let stride = raw_count.div_ceil(MAX_POINT_CLOUD_POINTS).max(1);
        let colors_aligned = cloud.colors_rgba.len() == raw_count;
        let mut positions = Vec::with_capacity(raw_count.min(MAX_POINT_CLOUD_POINTS));
        let mut colors = Vec::with_capacity(raw_count.min(MAX_POINT_CLOUD_POINTS));
        for point_index in (0..raw_count).step_by(stride).take(MAX_POINT_CLOUD_POINTS) {
            let xyz = cloud.positions_xyz[point_index];
            if !xyz.iter().all(|value| value.is_finite()) {
                continue;
            }
            positions.push(xyz);
            if colors_aligned {
                colors.push(cloud.colors_rgba[point_index]);
            }
        }
        if positions.is_empty() {
            continue;
        }
        let point_count = positions.len();
        let mut points = rerun::Points3D::new(positions);
        if colors_aligned && colors.len() == point_count {
            points = points.with_colors(colors);
        }
        let path = format!("sensors/{}/points", sensor_entity(&cloud.sensor));
        let _ = recorder.log(path, &points);
    }
}

#[cfg(feature = "grpc-client")]
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

#[cfg(feature = "grpc-client")]
fn send_gateway_command(
    endpoint: String,
    session_state: Arc<Mutex<Option<String>>>,
    command_status: Arc<Mutex<String>>,
    command_id: &'static str,
    payload: grpc_client::GatewayCommand,
) {
    let session_id = session_state
        .lock()
        .ok()
        .and_then(|session| session.clone());
    let Some(session_id) = session_id else {
        if let Ok(mut status) = command_status.lock() {
            *status = "未连接，命令未发送".to_owned();
        }
        return;
    };
    if let Ok(mut status) = command_status.lock() {
        *status = format!("发送 {command_id} …");
    }
    tokio::spawn(async move {
        let result = async {
            let mut client = grpc_client::ArmGatewayClient::connect(endpoint, "rebot_sim_viewer")
                .await
                .map_err(|error| error.to_string())?;
            client.set_session_id(session_id);
            client
                .command(payload, command_id)
                .await
                .map_err(|error| error.to_string())
        }
        .await;
        if let Ok(mut status) = command_status.lock() {
            *status = match result {
                Ok(ack) => {
                    format!("{}: {}", ack.status, ack.reason)
                }
                Err(error) => format!("命令错误: {error}"),
            };
        }
    });
}

#[cfg(feature = "embedded-viewer")]
impl eframe::App for EmbeddedViewerApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.rerun_app.save(storage);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::Panel::left("console_panel")
            .default_size(280.0)
            .resizable(true)
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.heading("控制台");
                ui.separator();
                ui.label("Rerun Viewer 已嵌入当前窗口");
                ui.label(format!("ArmGateway: {}", self.gateway_status));
                ui.label(&self.rerun_status);
                #[cfg(feature = "grpc-client")]
                ui.label(&self.model_status);
                #[cfg(feature = "grpc-client")]
                if let Ok(latest) = self.telemetry_status.lock() {
                    ui.label(format!(
                        "遥测: seq={} quality={} joints={} 状态={}",
                        latest.sequence, latest.quality, latest.joint_count, latest.link_state
                    ));
                }
                #[cfg(feature = "grpc-client")]
                {
                    ui.add_space(8.0);
                    ui.label("控制（仿真）");
                    ui.horizontal(|ui| {
                        if ui.button("使能").clicked() {
                            send_gateway_command(
                                self.gateway_endpoint.clone(),
                                self.session_id.clone(),
                                Arc::clone(&self.command_status),
                                "enable",
                                grpc_client::GatewayCommand::Enable { enabled: true },
                            );
                        }
                        if ui.button("停止").clicked() {
                            send_gateway_command(
                                self.gateway_endpoint.clone(),
                                self.session_id.clone(),
                                Arc::clone(&self.command_status),
                                "stop",
                                grpc_client::GatewayCommand::Stop { emergency: false },
                            );
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("暂停").clicked() {
                            send_gateway_command(
                                self.gateway_endpoint.clone(),
                                self.session_id.clone(),
                                Arc::clone(&self.command_status),
                                "pause",
                                grpc_client::GatewayCommand::Pause,
                            );
                        }
                        if ui.button("恢复").clicked() {
                            send_gateway_command(
                                self.gateway_endpoint.clone(),
                                self.session_id.clone(),
                                Arc::clone(&self.command_status),
                                "resume",
                                grpc_client::GatewayCommand::Resume,
                            );
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Slider::new(&mut self.speed_scale, 0.1..=2.0)
                                .text("执行倍率")
                                .suffix("×"),
                        );
                        if ui.button("应用").clicked() {
                            send_gateway_command(
                                self.gateway_endpoint.clone(),
                                self.session_id.clone(),
                                Arc::clone(&self.command_status),
                                "speed-scale",
                                grpc_client::GatewayCommand::SpeedScale {
                                    scale: self.speed_scale as f64,
                                },
                            );
                        }
                    });
                    if ui.button("J1 +0.01 rad").clicked() {
                        send_gateway_command(
                            self.gateway_endpoint.clone(),
                            self.session_id.clone(),
                            Arc::clone(&self.command_status),
                            "jog-j1",
                            grpc_client::GatewayCommand::Jog {
                                joint_index: 0,
                                step_rad: 0.01,
                                speed_limit_rad_s: 0.0,
                            },
                        );
                    }
                    if let Ok(status) = self.command_status.lock() {
                        ui.small(format!("命令状态: {status}"));
                    }
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label("数据端口");
                    ui.monospace("127.0.0.1:9876");
                });
                ui.add_space(8.0);
                ui.checkbox(&mut self.panel_enabled, "测试面板");
            });

        // 自定义面板占用左侧区域，剩余区域交给 Rerun Viewer。
        self.rerun_app.ui(ui, frame);
    }

    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.rerun_app.logic(ctx, frame);
    }
}
