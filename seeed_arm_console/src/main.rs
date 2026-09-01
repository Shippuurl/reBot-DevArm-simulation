use std::time::{Duration, Instant};
#[cfg(feature = "rerun-recording")]
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

mod design;
mod icons;
#[cfg(feature = "rerun-recording")]
mod rerun_bridge;
mod telemetry;

use design::*;
use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Layout, Margin, Pos2, Rect, RichText, Stroke, Ui,
    Vec2,
    containers::{CentralPanel, Panel},
};
use egui_dock::{DockArea, DockState, Style, TabViewer};
use egui_plot::{Line, Plot, PlotPoints};
use telemetry::{
    LinkState, MockTelemetrySource, TcpTelemetrySource, TelemetryFrame, TelemetrySource,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Tab {
    Scene,
    Joints,
    Plot,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Self::Scene => "场景",
            Self::Joints => "关节",
            Self::Plot => "曲线",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StreamState {
    Offline,
    Connecting,
    Connected,
    Fault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceChoice {
    Mock,
    Tcp,
}

impl SourceChoice {
    fn label(self) -> &'static str {
        match self {
            Self::Mock => "本地模拟",
            Self::Tcp => "TCP 网关",
        }
    }
}

struct RobotWorkspace {
    dock: DockState<Tab>,
    stream: StreamState,
    source_choice: SourceChoice,
    endpoint: String,
    recording: bool,
    selected_joint: usize,
    selected_entity: &'static str,
    joint_position: [f32; 6],
    joint_velocity: [f32; 6],
    position_history: Vec<[f64; 2]>,
    telemetry_source: Box<dyn TelemetrySource>,
    latest_telemetry: TelemetryFrame,
    #[cfg(feature = "rerun-recording")]
    rerun_recorder: Option<rerun_bridge::RerunRecorder>,
    started_at: Instant,
    frame: u64,
    stop_armed_until: Option<Instant>,
    last_event: String,
}

impl Default for RobotWorkspace {
    fn default() -> Self {
        Self {
            dock: DockState::new(vec![Tab::Scene, Tab::Joints, Tab::Plot]),
            stream: StreamState::Offline,
            source_choice: SourceChoice::Mock,
            endpoint: "127.0.0.1:50051".to_owned(),
            recording: false,
            selected_joint: 0,
            selected_entity: "robot",
            joint_position: [0.0; 6],
            joint_velocity: [0.0; 6],
            position_history: Vec::with_capacity(900),
            telemetry_source: Box::new(MockTelemetrySource::default()),
            latest_telemetry: TelemetryFrame::default(),
            #[cfg(feature = "rerun-recording")]
            rerun_recorder: None,
            started_at: Instant::now(),
            frame: 0,
            stop_armed_until: None,
            last_event: "等待数据".to_owned(),
        }
    }
}

impl RobotWorkspace {
    fn issue_command(&mut self, command: &str, label: &str) {
        match self.telemetry_source.send_command(command) {
            Ok(()) => self.last_event = label.to_owned(),
            Err(error) => self.last_event = error,
        }
    }

    fn elapsed(&self) -> f64 {
        self.started_at.elapsed().as_secs_f64()
    }

    fn tick(&mut self, ctx: &egui::Context) {
        if self.stream != StreamState::Offline {
            let t = self.elapsed();
            if let Some(frame) = self.telemetry_source.next(t, self.selected_joint) {
                let value = frame.joint_position[self.selected_joint];
                self.frame = frame.sequence;
                self.joint_position = frame.joint_position;
                self.joint_velocity = frame.joint_velocity;
                #[cfg(feature = "rerun-recording")]
                if self.recording
                    && let Some(recorder) = self.rerun_recorder.as_ref()
                    && let Err(error) = recorder.log_frame(&frame)
                {
                    self.recording = false;
                    self.rerun_recorder = None;
                    self.last_event = error;
                }
                self.latest_telemetry = frame;
                self.position_history.push([t, value as f64]);
                if self.position_history.len() > 900 {
                    let keep_from = self.position_history.len() - 900;
                    self.position_history.drain(..keep_from);
                }
            }
            self.stream = match self.telemetry_source.link_state() {
                LinkState::Connecting => StreamState::Connecting,
                LinkState::Connected => StreamState::Connected,
                LinkState::Fault => StreamState::Fault,
                LinkState::Offline => StreamState::Offline,
            };
        }
        ctx.request_repaint_after(Duration::from_millis(16));
    }

    fn show(&mut self, ui: &mut Ui) {
        self.tick(ui.ctx());
        self.apply_theme(ui.ctx());

        Panel::top("top_bar")
            .exact_size(42.0)
            .frame(panel_frame(PANEL))
            .show(ui, |ui| self.top_bar(ui));

        Panel::bottom("status_bar")
            .exact_size(26.0)
            .frame(panel_frame(PANEL))
            .show(ui, |ui| self.status_bar(ui));

        Panel::left("navigator")
            .resizable(true)
            .default_size(190.0)
            .size_range(150.0..=280.0)
            .frame(panel_frame_inset(PANEL))
            .show(ui, |ui| self.navigator(ui));

        Panel::right("inspector")
            .resizable(true)
            .default_size(240.0)
            .size_range(190.0..=340.0)
            .frame(panel_frame_inset(PANEL))
            .show(ui, |ui| self.inspector(ui));

        CentralPanel::default()
            .frame(panel_frame(BG))
            .show(ui, |ui| {
                DockArea::new(&mut self.dock)
                    .style(dock_style(ui))
                    .show_inside(
                        ui,
                        &mut WorkspaceTabs {
                            stream: self.stream,
                            recording: self.recording,
                            selected_joint: self.selected_joint,
                            joint_position: self.joint_position,
                            joint_velocity: self.joint_velocity,
                            position_history: &self.position_history,
                            frame: self.frame,
                        },
                    );
            });
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        ctx.set_theme(egui::Theme::Dark);
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = PANEL;
        visuals.window_fill = PANEL;
        visuals.extreme_bg_color = BG;
        visuals.faint_bg_color = PANEL_ALT;
        visuals.override_text_color = Some(TEXT);
        visuals.widgets.noninteractive.bg_fill = PANEL;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
        visuals.widgets.inactive.bg_fill = PANEL_ALT;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(31, 39, 47);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
        visuals.widgets.active.bg_fill = Color32::from_rgb(42, 67, 99);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
        visuals.selection.bg_fill = Color32::from_rgb(39, 72, 115);
        visuals.selection.stroke = Stroke::new(1.0, ACCENT);
        visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);
        visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
        visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
        visuals.widgets.active.corner_radius = CornerRadius::same(6);
        visuals.widgets.open.corner_radius = CornerRadius::same(6);
        visuals.window_shadow = egui::epaint::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(120),
        };
        let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
        style.visuals = visuals;
        style.spacing.item_spacing = Vec2::new(SPACE_MD, SPACE_SM);
        style.spacing.button_padding = Vec2::new(12.0, 7.0);
        style.spacing.menu_margin = Margin::same(8);
        style.spacing.interact_size.y = CONTROL_HEIGHT;
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(15.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, FontId::proportional(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, FontId::proportional(18.0));
        style
            .text_styles
            .insert(egui::TextStyle::Monospace, FontId::monospace(14.0));
        style.interaction.selectable_labels = false;
        ctx.set_style_of(egui::Theme::Dark, style);
    }

    fn top_bar(&mut self, ui: &mut Ui) {
        ui.horizontal_centered(|ui| {
            ui.add_space(SPACE_MD);
            ui.label(RichText::new("机器人工作区").strong().color(TEXT));
            ui.separator();
            ui.label(RichText::new("仿真").small().color(MUTED));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let stop_label = if self.stop_armed_until.is_some() {
                    "确认停止"
                } else {
                    "停止"
                };
                let stop = ui.add_sized(
                    [108.0, 30.0],
                    egui::Button::new(RichText::new(stop_label).strong().color(Color32::WHITE))
                        .fill(DANGER),
                );
                if stop.clicked() {
                    if self.stop_armed_until.is_some() {
                        self.stop_armed_until = None;
                        self.recording = false;
                        self.issue_command(r#"{"type":"stop"}"#, "已发送停止命令");
                    } else {
                        self.stop_armed_until = Some(Instant::now() + Duration::from_secs(3));
                    }
                }
                if self
                    .stop_armed_until
                    .is_some_and(|deadline| deadline < Instant::now())
                {
                    self.stop_armed_until = None;
                }
                ui.add_space(SPACE_LG);
                ui.label(RichText::new("●").color(stream_color(self.stream)));
                ui.label(
                    RichText::new(stream_label(self.stream))
                        .small()
                        .color(MUTED),
                );
                ui.add_space(SPACE_LG);
            });
        });
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal_centered(|ui| {
            ui.add_space(SPACE_MD);
            status_dot(ui, stream_color(self.stream));
            ui.label(
                RichText::new(stream_status_label(self.stream))
                    .small()
                    .color(MUTED),
            );
            ui.separator();
            ui.label(
                RichText::new(format!("帧 {:06}", self.frame))
                    .monospace()
                    .small()
                    .color(MUTED),
            );
            ui.separator();
            ui.label(
                RichText::new(if self.recording {
                    "录制中"
                } else {
                    "就绪"
                })
                .small()
                .color(if self.recording { ACCENT } else { MUTED }),
            );
            ui.separator();
            ui.label(RichText::new(&self.last_event).small().color(MUTED));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new("60 Hz").monospace().small().color(MUTED));
                ui.add_space(SPACE_MD);
            });
        });
    }

    fn navigator(&mut self, ui: &mut Ui) {
        ui.add_space(SPACE_MD);
        ui.label(RichText::new("工作区").small().strong().color(MUTED));
        ui.add_space(SPACE_SM);
        nav_item(
            ui,
            icons::Icon::Overview,
            "概览",
            self.selected_entity == "robot",
            || {
                self.selected_entity = "robot";
            },
        );
        ui.add_space(SPACE_LG);
        ui.label(RichText::new("实体").small().strong().color(MUTED));
        for (icon, id, label) in [
            (icons::Icon::Robot, "robot", "机器人"),
            (icons::Icon::Joints, "joints", "关节"),
            (icons::Icon::Frames, "frames", "坐标系"),
            (icons::Icon::Sensors, "sensors", "传感器"),
        ] {
            let selected = self.selected_entity == id;
            nav_item(ui, icon, label, selected, || self.selected_entity = id);
        }
        ui.add_space(SPACE_LG);
        ui.label(RichText::new("布局").small().strong().color(MUTED));
        if ui.selectable_label(false, "重置布局").clicked() {
            self.dock = DockState::new(vec![Tab::Scene, Tab::Joints, Tab::Plot]);
        }
    }

    fn inspector(&mut self, ui: &mut Ui) {
        ui.add_space(SPACE_MD);
        ui.label(RichText::new("检查器").small().strong().color(MUTED));
        ui.add_space(SPACE_MD);
        section(ui, entity_label(self.selected_entity));
        match self.selected_entity {
            "joints" => {
                for (idx, value) in self.joint_position.iter().enumerate() {
                    value_row(ui, &format!("J{}", idx + 1), &format!("{value:+.3} rad"));
                }
            }
            "frames" => {
                value_row(ui, "根节点", "base");
                value_row(ui, "末端", "tool");
                let count = self.latest_telemetry.tf.len().to_string();
                value_row(ui, "数量", &count);
            }
            "sensors" => {
                value_row(ui, "图像", "未连接");
                value_row(ui, "点云", "未连接");
            }
            _ => {
                value_row(ui, "模式", "仿真");
                value_row(ui, "自由度", "6");
                value_row(ui, "来源", self.latest_telemetry.source.label());
                value_row(ui, "质量", self.latest_telemetry.quality.label());
                let planned = self.latest_telemetry.planned_trajectory.len().to_string();
                let actual = self.latest_telemetry.actual_trajectory.len().to_string();
                value_row(ui, "规划点", &planned);
                value_row(ui, "实际点", &actual);
            }
        }
        ui.add_space(SPACE_XL);
        section(ui, "数据流");
        ui.horizontal(|ui| {
            ui.label(RichText::new("来源").small().color(MUTED));
            egui::ComboBox::from_id_salt("source_choice")
                .selected_text(self.source_choice.label())
                .show_ui(ui, |ui| {
                    ui.add_enabled_ui(self.stream == StreamState::Offline, |ui| {
                        ui.selectable_value(
                            &mut self.source_choice,
                            SourceChoice::Mock,
                            SourceChoice::Mock.label(),
                        );
                        ui.selectable_value(
                            &mut self.source_choice,
                            SourceChoice::Tcp,
                            SourceChoice::Tcp.label(),
                        );
                    });
                });
        });
        if self.source_choice == SourceChoice::Tcp {
            ui.add_space(SPACE_SM);
            ui.add_enabled(
                self.stream == StreamState::Offline,
                egui::TextEdit::singleline(&mut self.endpoint)
                    .hint_text("127.0.0.1:50051")
                    .desired_width(ui.available_width()),
            );
        }
        ui.add_space(SPACE_SM);
        let stream_button = if self.stream == StreamState::Offline {
            "连接"
        } else {
            "断开"
        };
        if ui.button(stream_button).clicked() {
            if self.stream != StreamState::Offline {
                self.stream = StreamState::Offline;
                self.telemetry_source = Box::new(MockTelemetrySource::default());
            } else {
                match self.source_choice {
                    SourceChoice::Mock => {
                        self.telemetry_source = Box::new(MockTelemetrySource::default());
                        self.stream = StreamState::Connected;
                    }
                    SourceChoice::Tcp => match TcpTelemetrySource::connect(&self.endpoint) {
                        Ok(source) => {
                            self.telemetry_source = Box::new(source);
                            self.stream = StreamState::Connecting;
                        }
                        Err(_) => {
                            self.stream = StreamState::Fault;
                        }
                    },
                }
            }
        }
        ui.label(
            RichText::new(match self.stream {
                StreamState::Connected => "接收实时快照",
                StreamState::Connecting => "等待网关连接",
                StreamState::Fault => "网关连接失败",
                StreamState::Offline => "数据源未启动",
            })
            .small()
            .color(MUTED),
        );
        ui.add_space(SPACE_XL);
        section(ui, "控制");
        ui.horizontal(|ui| {
            if ui.button("使能").clicked() {
                self.issue_command(r#"{"type":"enable","enabled":true}"#, "已发送使能命令");
            }
            if ui.button("停用").clicked() {
                self.issue_command(r#"{"type":"enable","enabled":false}"#, "已发送停用命令");
            }
        });
        if ui.button("停止运动").clicked() {
            self.issue_command(r#"{"type":"stop"}"#, "已发送停止命令");
        }
        ui.horizontal(|ui| {
            if ui.button("Jog −").clicked() {
                self.issue_command(
                    &format!(
                        r#"{{"type":"jog","joint_index":{},"step_rad":-0.05}}"#,
                        self.selected_joint
                    ),
                    "已发送负向 Jog",
                );
            }
            if ui.button("Jog +").clicked() {
                self.issue_command(
                    &format!(
                        r#"{{"type":"jog","joint_index":{},"step_rad":0.05}}"#,
                        self.selected_joint
                    ),
                    "已发送正向 Jog",
                );
            }
        });
        ui.add_space(SPACE_XL);
        section(ui, "Rerun");
        let button_text = if self.recording {
            "停止录制"
        } else {
            "开始录制"
        };
        if ui.button(button_text).clicked() {
            #[cfg(feature = "rerun-recording")]
            self.toggle_rerun_recording();
            #[cfg(not(feature = "rerun-recording"))]
            {
                self.last_event = "请使用 --features rerun-recording 启用 Rerun 记录".to_owned();
            }
        }
        ui.add_space(SPACE_SM);
        ui.label(
            RichText::new(if cfg!(feature = "rerun-recording") {
                "记录文件：recordings/*.rrd"
            } else {
                "记录功能未启用"
            })
            .small()
            .color(MUTED),
        );
    }

    #[cfg(feature = "rerun-recording")]
    fn toggle_rerun_recording(&mut self) {
        if self.recording {
            self.recording = false;
            self.rerun_recorder = None;
            self.last_event = "Rerun 记录已停止".to_owned();
            return;
        }

        let directory = PathBuf::from("recordings");
        if let Err(error) = fs::create_dir_all(&directory) {
            self.last_event = format!("创建记录目录失败: {error}");
            return;
        }
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        let path = directory.join(format!("robot-{timestamp}.rrd"));
        match rerun_bridge::RerunRecorder::save(&path) {
            Ok(recorder) => {
                let model_root = std::env::var_os("ROBOT_MODEL_ROOT")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("assets/robot/b601_rs"));
                let model_message = match recorder.log_model_assets(&model_root) {
                    Ok(0) => "；未找到模型资源".to_owned(),
                    Ok(count) => format!("；已记录 {count} 个模型资源"),
                    Err(error) => format!("；模型资源记录失败：{error}"),
                };
                self.rerun_recorder = Some(recorder);
                self.recording = true;
                self.last_event = format!(
                    "Rerun 记录: {}{model_message}（模型 {}）",
                    path.display(),
                    model_root.display()
                );
            }
            Err(error) => self.last_event = error,
        }
    }
}

struct WorkspaceTabs<'a> {
    stream: StreamState,
    recording: bool,
    selected_joint: usize,
    joint_position: [f32; 6],
    joint_velocity: [f32; 6],
    position_history: &'a [[f64; 2]],
    frame: u64,
}

impl TabViewer for WorkspaceTabs<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.label().into()
    }

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(*tab)
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::Scene => self.scene(ui),
            Tab::Joints => self.joints(ui),
            Tab::Plot => self.plot(ui),
        }
    }

    fn is_closeable(&self, _tab: &Self::Tab) -> bool {
        false
    }
}

impl WorkspaceTabs<'_> {
    fn scene(&mut self, ui: &mut Ui) {
        let available = ui.available_rect_before_wrap();
        let (rect, _) = ui.allocate_exact_size(available.size(), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::ZERO, BG);
        draw_grid(&painter, rect);
        draw_axes(&painter, rect);
        draw_arm(&painter, rect, self.joint_position);
        painter.text(
            rect.left_top() + Vec2::new(16.0, 14.0),
            egui::Align2::LEFT_TOP,
            "三维视图",
            FontId::proportional(14.0),
            MUTED,
        );
        painter.text(
            rect.right_bottom() - Vec2::new(16.0, 14.0),
            egui::Align2::RIGHT_BOTTOM,
            if self.recording {
                "录制"
            } else if self.stream == StreamState::Connected {
                "实时"
            } else if self.stream == StreamState::Connecting {
                "连接中"
            } else {
                "无数据流"
            },
            FontId::monospace(13.0),
            stream_color(self.stream),
        );
    }

    fn joints(&mut self, ui: &mut Ui) {
        ui.add_space(SPACE_XL);
        ui.horizontal(|ui| {
            ui.label(RichText::new("关节状态").strong().color(TEXT));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("帧 {:06}", self.frame))
                        .monospace()
                        .small()
                        .color(MUTED),
                );
            });
        });
        ui.add_space(SPACE_MD);
        egui::Grid::new("joint_grid")
            .striped(true)
            .min_col_width(90.0)
            .show(ui, |ui| {
                for heading in ["关节", "位置", "速度", "状态"] {
                    ui.label(RichText::new(heading).small().strong().color(MUTED));
                }
                ui.end_row();
                for (idx, (&position, &velocity)) in self
                    .joint_position
                    .iter()
                    .zip(self.joint_velocity.iter())
                    .enumerate()
                {
                    let selected = idx == self.selected_joint;
                    let response = ui.selectable_label(selected, format!("J{}", idx + 1));
                    if response.clicked() {
                        self.selected_joint = idx;
                    }
                    ui.label(RichText::new(format!("{position:+.4} rad")).monospace());
                    ui.label(
                        RichText::new(format!("{velocity:+.4} rad/s"))
                            .monospace()
                            .color(MUTED),
                    );
                    ui.label(RichText::new("正常").small().color(OK));
                    ui.end_row();
                }
            });
    }

    fn plot(&mut self, ui: &mut Ui) {
        ui.add_space(SPACE_XL);
        ui.horizontal(|ui| {
            ui.label(RichText::new("位置 / J1").strong().color(TEXT));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new("弧度").monospace().small().color(MUTED));
            });
        });
        ui.add_space(SPACE_SM);
        let points = PlotPoints::from_iter(self.position_history.iter().copied());
        Plot::new("position_plot")
            .height(ui.available_height().max(200.0))
            .allow_zoom(true)
            .allow_drag(true)
            .allow_scroll(false)
            .show_axes([true, true])
            .show_grid([true, true])
            .show(ui, |plot_ui| {
                plot_ui.line(Line::new("J1", points).color(ACCENT).width(2.0));
            });
    }
}

impl eframe::App for RobotWorkspace {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

fn panel_frame(fill: Color32) -> egui::Frame {
    egui::Frame {
        inner_margin: Margin::ZERO,
        outer_margin: Margin::ZERO,
        corner_radius: CornerRadius::ZERO,
        fill,
        stroke: Stroke::new(1.0, BORDER),
        shadow: egui::epaint::Shadow::NONE,
    }
}

fn panel_frame_inset(fill: Color32) -> egui::Frame {
    egui::Frame {
        inner_margin: Margin::symmetric(PANEL_INSET as i8, 0),
        ..panel_frame(fill)
    }
}

fn dock_style(ui: &Ui) -> Style {
    let mut style = Style::from_egui(ui.style().as_ref());
    style.tab_bar.bg_fill = PANEL;
    style.tab.active.bg_fill = PANEL_ALT;
    style.tab.inactive.bg_fill = PANEL;
    style.tab.active.text_color = TEXT;
    style.tab.inactive.text_color = MUTED;
    style.tab.active.outline_color = ACCENT;
    style.tab.inactive.outline_color = BORDER;
    style.separator.color_idle = BORDER;
    style.separator.color_hovered = ACCENT;
    style.separator.color_dragged = ACCENT;
    style.separator.width = 1.0;
    style
}

fn section(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .small()
            .strong()
            .color(MUTED),
    );
    ui.add_space(SPACE_XS);
}

fn entity_label(id: &str) -> &'static str {
    match id {
        "joints" => "关节",
        "frames" => "坐标系",
        "sensors" => "传感器",
        _ => "机器人",
    }
}

fn value_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).small().color(MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(value).small().monospace().color(TEXT));
        });
    });
    ui.add_space(SPACE_XS);
}

fn status_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.0, color);
}

fn stream_color(state: StreamState) -> Color32 {
    match state {
        StreamState::Connected => OK,
        StreamState::Connecting => ACCENT,
        StreamState::Fault => DANGER,
        StreamState::Offline => MUTED,
    }
}

fn stream_label(state: StreamState) -> &'static str {
    match state {
        StreamState::Connected => "数据流",
        StreamState::Connecting => "连接中",
        StreamState::Fault => "连接失败",
        StreamState::Offline => "离线",
    }
}

fn stream_status_label(state: StreamState) -> &'static str {
    match state {
        StreamState::Connected => "已连接",
        StreamState::Connecting => "连接中",
        StreamState::Fault => "连接失败",
        StreamState::Offline => "未连接",
    }
}

fn nav_item(ui: &mut Ui, icon: icons::Icon, label: &str, selected: bool, mut action: impl FnMut()) {
    let mut clicked = false;
    ui.horizontal(|ui| {
        let width = ui.available_width().max(0.0);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, NAV_ROW_HEIGHT), egui::Sense::click());
        let fill = if selected {
            Color32::from_rgb(28, 47, 73)
        } else if response.hovered() {
            PANEL_ALT
        } else {
            Color32::TRANSPARENT
        };
        if fill != Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
        }
        let icon_rect = Rect::from_min_size(
            rect.left_top() + Vec2::new(10.0, (rect.height() - icons::icon_size().y) * 0.5),
            icons::icon_size(),
        );
        icons::draw(ui, icon, icon_rect, if selected { ACCENT } else { MUTED });
        ui.painter().text(
            Pos2::new(icon_rect.right() + 10.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            FontId::proportional(14.0),
            if selected { TEXT } else { MUTED },
        );
        clicked = response.clicked();
    });
    if clicked {
        action();
    }
}

fn draw_grid(painter: &egui::Painter, rect: Rect) {
    let origin = Pos2::new(rect.center().x, rect.center().y + rect.height() * 0.18);
    let horizon = origin.y - rect.height() * 0.08;
    for i in -10..=10 {
        let offset = i as f32 * 28.0;
        let color = if i == 0 {
            Color32::from_rgb(53, 63, 73)
        } else {
            Color32::from_rgb(25, 30, 36)
        };
        painter.line_segment(
            [
                Pos2::new(origin.x + offset, horizon),
                Pos2::new(origin.x + offset * 2.2, rect.bottom()),
            ],
            Stroke::new(1.0, color),
        );
        painter.line_segment(
            [
                Pos2::new(rect.left(), horizon + offset * 0.26),
                Pos2::new(rect.right(), horizon + offset * 0.26),
            ],
            Stroke::new(1.0, color),
        );
    }
}

fn draw_axes(painter: &egui::Painter, rect: Rect) {
    let base = Pos2::new(rect.left() + 54.0, rect.bottom() - 52.0);
    painter.line_segment(
        [base, base + Vec2::new(34.0, 0.0)],
        Stroke::new(2.0, DANGER),
    );
    painter.line_segment([base, base + Vec2::new(0.0, -34.0)], Stroke::new(2.0, OK));
    painter.line_segment(
        [base, base + Vec2::new(-20.0, 14.0)],
        Stroke::new(2.0, ACCENT),
    );
    painter.text(
        base + Vec2::new(38.0, -2.0),
        egui::Align2::LEFT_CENTER,
        "X",
        FontId::monospace(12.0),
        DANGER,
    );
    painter.text(
        base + Vec2::new(4.0, -40.0),
        egui::Align2::LEFT_CENTER,
        "Z",
        FontId::monospace(12.0),
        OK,
    );
    painter.text(
        base + Vec2::new(-29.0, 18.0),
        egui::Align2::LEFT_CENTER,
        "Y",
        FontId::monospace(12.0),
        ACCENT,
    );
}

fn draw_arm(painter: &egui::Painter, rect: Rect, joints: [f32; 6]) {
    let base = Pos2::new(rect.center().x, rect.bottom() - 92.0);
    let mut points = vec![base];
    let lengths = [74.0, 88.0, 78.0, 52.0, 40.0, 34.0];
    let mut angle = -std::f32::consts::FRAC_PI_2;
    for (idx, length) in lengths.iter().enumerate() {
        angle += joints[idx] * 0.45 + 0.08;
        let previous = *points.last().unwrap_or(&base);
        points.push(previous + Vec2::new(angle.cos() * length, angle.sin() * length));
    }
    for pair in points.windows(2) {
        painter.line_segment(
            [pair[0], pair[1]],
            Stroke::new(12.0, Color32::from_rgb(57, 67, 78)),
        );
        painter.line_segment(
            [pair[0], pair[1]],
            Stroke::new(7.0, Color32::from_rgb(166, 177, 188)),
        );
    }
    for (idx, point) in points.iter().enumerate() {
        painter.circle_filled(
            *point,
            if idx == 0 { 13.0 } else { 9.0 },
            Color32::from_rgb(32, 40, 48),
        );
        painter.circle_stroke(
            *point,
            if idx == 0 { 13.0 } else { 9.0 },
            Stroke::new(2.0, ACCENT),
        );
    }
    painter.circle_filled(*points.last().unwrap_or(&base), 5.0, ACCENT);
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Robot Workspace")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([980.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Robot Workspace",
        options,
        Box::new(|cc| {
            design::configure_fonts(&cc.egui_ctx);
            Ok(Box::new(RobotWorkspace::default()))
        }),
    )
}
