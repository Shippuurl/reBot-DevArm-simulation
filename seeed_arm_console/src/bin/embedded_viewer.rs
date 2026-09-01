//! Rerun Native Viewer 集成入口。
//!
//! 该入口只负责验证官方 `extend_viewer_ui` 集成方式：外层由本项目的
//! `eframe` 应用管理窗口，内层持有 `re_viewer::App`。正式控制状态和
//! 遥测桥接在此入口验证通过后再迁移，避免影响 `robot_workspace`。

#[cfg(not(feature = "embedded-viewer"))]
fn main() {
    eprintln!(
        "请使用 `cargo run --features embedded-viewer --bin embedded_viewer` 启动 Rerun 嵌入验证入口"
    );
}

#[cfg(feature = "embedded-viewer")]
use rerun::external::{eframe, egui, re_crash_handler, re_grpc_server, re_log, re_viewer, tokio};

#[cfg(feature = "embedded-viewer")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let main_thread_token = re_viewer::MainThreadToken::i_promise_i_am_on_the_main_thread();

    re_log::setup_logging();
    re_crash_handler::install_crash_handlers(re_viewer::build_info());

    // 让 C++/Rust 数据源可以通过标准 Rerun gRPC 端口推送到内嵌 Viewer。
    let (log_rx, _grpc_server_handle) = re_grpc_server::spawn_with_recv(
        "127.0.0.1:9876".parse()?,
        Default::default(),
        re_grpc_server::shutdown::never(),
    );

    let mut native_options = re_viewer::native::eframe_options(None);
    native_options.viewport = native_options
        .viewport
        .with_app_id("seeed_arm_console_embedded_viewer");

    let startup_options = re_viewer::StartupOptions::default();
    let app_environment = re_viewer::AppEnvironment::Custom("Seeed Arm Console".to_owned());

    eframe::run_native(
        "Seeed Arm Console",
        native_options,
        Box::new(move |cc| {
            re_viewer::customize_eframe_and_setup_renderer(cc)?;

            let mut rerun_app = re_viewer::App::new(
                main_thread_token,
                re_viewer::build_info(),
                app_environment,
                startup_options,
                cc,
                None,
                re_viewer::AsyncRuntimeHandle::from_current_tokio_runtime_or_wasmbindgen()?,
            );
            rerun_app.add_log_receiver(log_rx);

            Ok(Box::new(EmbeddedViewerApp {
                rerun_app,
                panel_enabled: true,
            }))
        }),
    )?;

    Ok(())
}

#[cfg(feature = "embedded-viewer")]
struct EmbeddedViewerApp {
    rerun_app: re_viewer::App,
    panel_enabled: bool,
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
