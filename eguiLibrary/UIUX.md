# egui 机器人上位机 UI/UX 规范

本文只记录 `D:\JazzyCWork\eguiLibrary` 中可复用的 egui 库、组件模板和机器人上位机交互要点。业务协议、驱动实现和仿真模型不放在这里。

## 1. 库清单与职责

### 1.1 页面基础与模板

| 库 | 版本/来源 | 用途 | 引入策略 |
| --- | --- | --- | --- |
| [`egui_dock`](https://crates.io/crates/egui_dock) | crates.io `0.21.1`（egui `0.36`） | 停靠、分栏、标签页和可选浮动窗口 | 基础依赖；与选定的 `egui` 版本一起锁定 |
| [`egui-shadcn`](https://crates.io/crates/egui-shadcn) | 本地 `shadcn-rs-master`，workspace `0.5.0`（egui `0.33`） | shadcn 风格的按钮、卡片、表单、Tabs、Dialog、Table、Sidebar、Toast、Tooltip 和主题令牌 | UI 模板；按需启用，不复制业务状态 |
| [`egui_material_icons`](https://crates.io/crates/egui_material_icons) | crates.io `0.8.0`（egui `0.35`） | Material Symbols 图标 | 统一图标风格时选用 |
| [`egui-phosphor`](https://crates.io/crates/egui-phosphor) | crates.io `0.13.0`（egui `0.35`） | Phosphor 图标及 Regular/Bold/Fill 等字重 | 与 Material 二选一，避免混用 |

`egui-shadcn` 的本地示例目录是 `shadcn-rs-master/crates/egui-shadcn/examples/`。优先参考 `button`、`card`、`dialog`、`tabs`、`table`、`sidebar`、`toast`、`form` 和 `tooltip`，通过 `Theme` 与 `ControlVariant`/`ControlSize` 保持同一套视觉令牌。该仓库 README 的安装示例仍写着 `0.3.1`，而本地 workspace 的 `Cargo.toml` 是 `0.5.0`，以锁定版本的 `Cargo.toml` 和 `Cargo.lock` 为准。

### 1.2 数据可视化模板

| 库 | 本地版本与 egui 基线 | 适用内容 | 引入策略 |
| --- | --- | --- | --- |
| [`egui_plot`](https://crates.io/crates/egui_plot) | `egui_plot-main` `0.37.0`，`egui/eframe` `0.36` | 关节位置、速度、力矩、误差和延迟的实时曲线 | 首选曲线库；只保留有界时间窗 |
| [`egui_graphs`](https://crates.io/crates/egui_graphs) | `egui_graphs-main` `0.31.0`，`egui` `0.35` | TF/设备关系、节点拓扑、依赖关系 | 有拓扑需求时启用；使用 `GraphView` |
| [`egui_node_graph2`](https://crates.io/crates/egui_node_graph2) | `egui_node_graph2-main` `0.7.0`，`egui` `0.30` | 规划管线、动作编排、可编辑数据流 | 仅在需要节点编辑器时启用；从 `egui_node_graph2_example/src/app.rs` 开始 |
| [`egui-charts`](https://crates.io/crates/egui-charts) | `egui-charts-main` `0.2.0`，`egui` `0.33.3` | 带缩放、十字线、绘图工具的复杂分析图表 | 可选；其模型偏金融图表，不能替代机器人实时曲线 |
| [`rerun`](https://crates.io/crates/rerun) | crates.io `0.36.3` | 模型、TF、图像、点云、规划/实际轨迹和事件的记录与查看 | 独立 Viewer/记录进程；不承担控制 |

本地可参考的入口：

- `egui_plot-main/examples/lines`、`interaction`、`linked_axes`、`performance`。
- `egui_graphs-main/crates/egui_graphs/examples/basic.rs` 和 `graph_view_response.rs`。
- `egui-charts-main/examples/basic_chart.rs`、`with_indicators.rs`、`drawing_tools.rs`。

维护提示：`egui-shadcn` README 标注 API 可能发生破坏性变化，应固定精确版本；`egui_graphs` README 标注项目目前不是活跃开发，若用于长期产品需准备替换或自行维护。

### 1.3 通信、遥测与输入

| 库 | 版本 | 用途 | 引入策略 |
| --- | --- | --- | --- |
| [`tonic`](https://crates.io/crates/tonic) | `0.14.6` | gRPC 客户端/服务端、协议边界 | 控制网关层；UI 只依赖领域接口 |
| [`crossbeam-channel`](https://crates.io/crates/crossbeam-channel) | `0.5.16` | UI 与后台任务之间的有界命令/事件通道 | 基础依赖；命令和诊断分通道 |
| [`triple_buffer`](https://crates.io/crates/triple_buffer) | `9.0.0` | 单生产者/单消费者的最新遥测快照 | 高频状态读取；不用于历史记录 |
| [`gilrs`](https://crates.io/crates/gilrs) | `0.11.2` | 手柄/摇杆事件 | 需要手柄时启用；必须经过死区、限幅和使能检查 |
| [`egui-probe`](https://crates.io/crates/egui-probe) | `0.13.0`（egui `0.36`） | derive 生成参数检查控件 | 调试/仿真参数页；不可直接写驱动 |
| [`egui_inspect`](https://crates.io/crates/egui_inspect) | `0.1.3`（egui `0.16`） | 结构体只读/可编辑检查面板 | 与 `egui-probe` 二选一；版本过旧时优先升级或隔离 |
| [`egui-modal`](https://crates.io/crates/egui-modal) | `0.6.0`（egui `0.30`） | 危险动作确认和阻塞式对话框 | Stop、清除故障、覆盖轨迹等动作 |
| [`egui-notify`](https://crates.io/crates/egui-notify) | `0.22.0`（egui `0.34`） | 成功、信息、警告和错误 Toast | 短暂反馈；严重故障仍需固定区域显示 |

## 2. 版本与依赖边界

本地可见的图表/图形库和反馈组件并不使用同一个 `egui` 小版本：`egui_plot`/`egui_dock`/`egui-probe` 为 `0.36`，`egui_graphs`/图标库为 `0.35`，`egui-notify` 为 `0.34`，`egui-charts`/`egui-shadcn` 为 `0.33`，`egui_node_graph2`/`egui-modal` 为 `0.30`，`egui_inspect` 仍为 `0.16`。同一个二进制中如果出现多个 `egui` 版本，组件的 `egui::Ui` 类型不能互换。

新工程按以下顺序锁定：

1. 先选 `egui`/`eframe` 基线，并让 `egui_dock`、图表和组件库跟随该基线；必要时选择对应历史版本或 fork 更新依赖。
2. 再加入 `egui-shadcn`、图标和反馈组件；用 `cargo tree -i egui` 检查是否只有一个 egui 小版本。
3. 最后加入 `tonic`、`crossbeam-channel`、`triple_buffer` 和 `rerun`。它们不应让 UI 组件直接依赖 ROS 2 消息类型。

建议在项目根目录执行（版本号以最终锁定为准）：

```powershell
cargo add egui_dock@0.21.1
cargo add tonic@0.14.6 --features transport
cargo add crossbeam-channel@0.5.16
cargo add triple_buffer@9.0.0
cargo add rerun@0.36.3
cargo tree -i egui
```

图表、图形和 `egui-shadcn` 不要机械地全部加入；先确认它们与选定 egui 基线兼容，再选择一个曲线库、一个图形库和一个组件模板。

## 3. UI 模板组合

### 3.1 页面骨架

使用 `eframe`/`egui` 提供窗口和基础布局，`egui_dock` 管理页面区域。推荐的可复用骨架是：

```text
应用栏（产品名、连接模式、心跳、全局停止）
└── DockArea
    ├── 控制区：连接、使能、模式、点动、轨迹请求
    ├── 观测区：关节表、曲线、TF/拓扑、Rerun 入口
    └── 诊断区：故障、事件、数据质量和日志
```

骨架只管理布局；机器人状态放在 ViewModel/状态仓库，不能塞进 `DockState` 或 Tab 对象。

### 3.2 组件对应关系

| 页面需求 | 直接复用的组件/库 | 组合规则 |
| --- | --- | --- |
| 主操作 | `egui-shadcn::button` + Material/Phosphor 图标 | 文案必须包含动作对象；危险动作使用 destructive 变体 |
| 状态摘要 | `card`、`label`、`badge`、`separator` | 状态同时显示文字、图标和颜色 |
| 参数编辑 | `form`、`select`、`slider`、`egui-probe` 或 `egui_inspect` | 标注单位、范围、来源和保存结果 |
| 页面切换 | `tabs` + `egui_dock::DockArea` | Tabs 表示业务上下文，Dock 只负责空间编排 |
| 表格 | `table` 模板或 `egui::Grid` | 数值使用等宽字体；列宽固定或有界 |
| 危险确认 | `egui-modal` | 对话框中写明影响、对象和确认后的结果 |
| 非阻塞反馈 | `egui-notify::Toasts` | 只提示结果，不替代故障区和事件记录 |
| 实时曲线 | `egui_plot::Plot` | 使用有界缓存，显示单位、时间窗和数据新鲜度 |
| 拓扑/流程 | `egui_graphs::GraphView` 或 `egui_node_graph2` | 仅在关系或流程可编辑时引入 |
| 远程观测 | `rerun` Viewer | UI 提供连接/打开入口，不从 Viewer 下发控制 |

### 3.3 `egui_dock` 最小用法

```rust
use egui::{Id, Ui, WidgetText};
use egui_dock::{DockArea, DockState, Style, TabViewer};

#[derive(Debug, Hash, PartialEq, Eq)]
enum Tab { Control, Telemetry, Faults }

struct Viewer;
impl TabViewer for Viewer {
    type Tab = Tab;
    fn id(&mut self, tab: &mut Tab) -> Id { Id::new(tab) }
    fn title(&mut self, tab: &mut Tab) -> WidgetText { format!("{tab:?}").into() }
    fn ui(&mut self, ui: &mut Ui, tab: &mut Tab) { ui.label(format!("{tab:?}")); }
}

fn show_dock(ui: &mut Ui, state: &mut DockState<Tab>) {
    DockArea::new(state)
        .style(Style::from_egui(ui.style().as_ref()))
        .show_inside(ui, &mut Viewer);
}
```

### 3.4 图表与状态快照

```rust
use egui_plot::{Line, Plot, PlotPoints};

let points = PlotPoints::from_iter(history.iter().map(|s| [s.time_s, s.position_rad]));
Plot::new("joint-position")
    .height(180.0)
    .show(ui, |plot_ui| plot_ui.line(Line::new("joint_1", points)));
```

后台快照使用 `triple_buffer`：

```rust
use triple_buffer::triple_buffer;

let (mut producer, mut consumer) = triple_buffer(&RobotTelemetry::default());
producer.write(next_frame);
let latest = consumer.read();
```

`history` 由独立的有界环形缓存维护；三缓冲只提供最新值。

拓扑图的最小入口来自本地 `egui_graphs-main/crates/egui_graphs/examples/basic.rs`：

```rust
let mut graph = egui_graphs::Graph::new();
let a = graph.add_node(());
let b = graph.add_node(());
graph.add_edge(a, b, ());
egui_graphs::DefaultGraphView::new().show(ui, &mut graph);
```

`egui_node_graph2` 的语义由应用实现，直接从 `egui_node_graph2_example/src/app.rs` 的 `GraphEditorState` 和 trait 实现开始；不要把 TF 拓扑和可编辑动作节点混用。`egui-charts` 采用 `ChartBuilder`/`Chart`，只在需要其十字线、绘图工具或复杂分析时启用。

### 3.5 通信与反馈

```rust
use crossbeam_channel::bounded;

let (command_tx, command_rx) = bounded::<RobotCommand>(64);
let (event_tx, event_rx) = bounded::<RobotEvent>(256);
while let Ok(event) = event_rx.try_recv() {
    // 更新 ViewModel；UI 线程不等待网络或磁盘
}
```

`tonic` 只出现在协议客户端/网关层，例如由生成的客户端调用 `Channel::from_shared(endpoint)?.connect().await?`。UI 调用 `ControlService::jog(command)` 之类的领域接口，不构造 `tonic::Request`，也不直接发布 ROS 2 topic/service/action。

### 3.6 参数、图标、弹窗和 Toast

```rust
// egui-probe：参数结构体自动生成检查控件
use egui_probe::{EguiProbe, Probe};
#[derive(EguiProbe)]
struct SimParams { max_speed: f32, damping: f32 }
Probe::new(&mut params).show(ui);

// egui_inspect：选择一个库即可，不与 egui-probe 同时使用
use egui_inspect::{EguiInspect, InspectNumber};
#[derive(EguiInspect)]
struct Limits { max_speed: f32 }
limits.inspect_mut("Limits", ui);

// Material Symbols
egui_material_icons::initialize(ctx);
ui.button(egui_material_icons::icons::ICON_STOP);

// Phosphor（需要先把字体加入 egui FontDefinitions）
egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
ui.label(egui::RichText::new(egui_phosphor::regular::WARNING));

// 危险动作确认与 Toast
let stop_modal = egui_modal::Modal::new(ctx, "stop-confirm");
if stop_requested { stop_modal.open(); }
stop_modal.show(|ui| {
    ui.label("确认停止当前运动？");
    if stop_modal.caution_button(ui, "停止").clicked() { /* 发送 Stop */ }
});

let mut toasts = egui_notify::Toasts::new();
toasts.warning("遥测延迟超过阈值");
toasts.show(ctx);
```

手柄事件在后台轮询，再转换为与鼠标/键盘相同的领域命令：

```rust
let mut gilrs = gilrs::Gilrs::new()?;
while let Some(event) = gilrs.next_event() {
    // 读取 event.event，经过死区和限幅后发送 RobotCommand
}
```

### 3.7 Rerun 记录入口

```rust
let rec = rerun::RecordingStreamBuilder::new("seeed_arm_console")
    .connect_grpc()?;
rec.set_time_sequence("frame", frame_id);
rec.log("robot/joints/joint_1", &joint_position)?;
```

记录路径建议保持稳定：`robot/model`、`robot/joints/*`、`robot/frames/*`、`robot/planned_trajectory`、`robot/actual_trajectory`、`events/commands`、`diagnostics/faults`。控制命令只从控制服务发出，Rerun 只接收状态、图像、点云和事件。

## 4. UX 要点

### 4.1 状态可理解

- 顶部固定显示运行模式、连接、使能、心跳/看门狗和全局停止结果。
- 状态不能只用颜色表达；至少同时使用文字和图标/形状。
- UI 状态由服务端确认（ACK、事件或遥测）驱动；点击按钮不能直接伪造“成功”。
- 断连、过期帧、丢帧和未知值明确标记，不显示成 0 或“空闲”。

### 4.2 操作安全

- 运动操作遵循“连接 → 运动许可/使能 → 点动或轨迹请求”的前置条件；不满足条件时禁用控件并说明原因。
- Stop、急停、停用和清除故障使用不同文案、图标和确认路径；软件按钮不冒充硬件急停。
- 点动显示关节、方向、步长、速度上限和单位；长按模式必须做到松开即停，并由后端 watchdog 兜底。
- 每条命令包含 `session_id`、`command_id`、时间戳、单位和有效期；UI 显示“已请求/已接受/已完成/已拒绝”的阶段。

### 4.3 信息密度与可读性

- 先显示当前状态和下一步，再显示原始数据和调试字段。
- 位置、速度、力矩、时间戳和序列号使用等宽字体；中文字体使用明确的 fallback。
- 每个表格或曲线标注单位、来源、采样频率、时间窗和最近帧年龄。
- 深色主题可降低长时间观察的眩光，但对比度、字号和焦点边框必须保持可见。

### 4.4 反馈与故障

- Toast 用于短暂成功/警告/错误反馈；故障等级、故障码、影响和恢复步骤必须在固定区域持续可见。
- 错误文案使用“发生了什么 → 影响 → 下一步”，避免只显示数字错误码。
- 高优先级故障进入单独事件序列，不能被普通遥测淹没；通道满载时保留 Stop、故障和连接事件。

### 4.5 性能与可观测性

- egui 帧循环只做非阻塞读取和渲染；网络、磁盘、模型加载和 Rerun 连接在后台任务执行。
- 曲线使用固定容量的 10–30 秒窗口；原始高频数据交给后端或 Rerun 记录。
- 通过 `triple_buffer` 读取最新遥测，通过 `crossbeam-channel` 传递命令和事件；不要让 UI 消费每一条高频样本。
- Rerun Viewer 与控制界面默认进程隔离，记录失败不能阻塞控制；记录状态应在 UI 中可见。

### 4.6 输入与可访问性

- 鼠标、键盘和手柄都经过同一命令模型；手柄轴有死区、限幅和连接丢失处理。
- 所有图标按钮提供 tooltip 和可见文字替代；焦点顺序与 Tab 顺序稳定。
- 禁用控件说明前置条件；弹窗支持取消和 Escape；危险动作默认焦点放在取消。
- 窗口缩放、125%/150% DPI 和中文文本不能导致关键按钮被裁剪。

## 5. 边界检查

- UI 层只依赖 ViewModel、领域命令和事件，不依赖 ROS 2 消息、OpenRAVE/MuJoCo 内部对象或驱动句柄。
- `egui_dock` 只保存布局；`egui_plot`/`egui_graphs` 只渲染数据；`rerun` 只记录和查看数据。
- `egui-probe`、`egui_inspect` 只编辑本地待保存值，保存动作必须经过范围校验、版本校验和后端确认。
- 新增库必须能对应一个页面能力或数据流；先在本地示例验证，再加入正式工程的 `Cargo.toml`。
