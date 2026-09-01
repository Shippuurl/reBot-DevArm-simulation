# 无 X11 调试闭环计划

## 目标

Windows 端的可视化软件直接完成连接、状态观察、轨迹检查和控制指令下发。桌面端采用自定义 `eframe` 外壳，将 Rerun Native Viewer (`re_viewer::App`) 嵌入同一窗口；ROS 2、C++ 仿真器和驱动只运行无界面的后台进程，不设置 `DISPLAY`，不依赖 VcXsrv 或其他 X11 转发。

## 目标拓扑

```text
┌────────────────────────────── Win11 ──────────────────────────────┐
│  Seeed Arm Console（自定义 eframe 外壳）                           │
│  ┌──────────────┐ ┌────────────────────────────────────────────┐  │
│  │ 控制/Jog/状态 │ │ Rerun Native Viewer（re_viewer::App）      │  │
│  │ egui 面板     │ │ 3D / TF / 曲线 / 图像 / 点云 / 时间线回放  │  │
│  └──────────────┘ └────────────────────────────────────────────┘  │
│                    同一 egui/wgpu 渲染循环                         │
│                         ▲ gRPC / WebSocket                         │
└───────────────────────────────┬──────────────────────────────────┘
                                │ TCP，仅传输数据和指令
┌───────────────────────────────▼──────────────────────────────────┐
│  Headless Gateway / ros2-bridge（Docker ROS 2 Jazzy）             │
│  会话、心跳、限位、命令确认、遥测节流、Rerun 日志桥               │
└───────────────┬──────────────────────┬───────────────────────────┘
                │                      │
       C++ MuJoCo/OpenRAVE        ROS 2 DDS / 驱动
       仿真或回放进程              （后续接入实机）
```

Rerun 只接收时间线数据，不参与运动控制；渲染崩溃不能阻塞控制 RPC。模型、TF、关节、图像、点云和规划/实际轨迹使用同一时间戳。

## Rerun Viewer 集成模式

桌面端采用 Rerun 官方 `extend_viewer_ui` 示例的集成方式：外层仍由项目自己的 `eframe::App` 控制窗口生命周期，内部持有 `re_viewer::App`，每帧先绘制控制面板，再调用 `rerun_app.ui` 绘制剩余区域，并在 `logic` 中转发 Rerun 的后台逻辑。

依赖固定在同一版本系列，嵌入入口启用：

```toml
rerun = { version = "=0.36.3", default-features = false, features = ["sdk", "native_viewer", "server"] }
```

`native_viewer` 会引入 Rerun Viewer 及其渲染依赖，编译和发布体积会增加。Viewer 扩展接口目前不是稳定 API，升级 Rerun 时必须重新验证并按版本更新适配代码。

官方参考实现：[extend_viewer_ui](https://github.com/rerun-io/rerun/tree/0.36.3/examples/rust/extend_viewer_ui)。本项目已增加同版本的独立验证入口 `embedded_viewer`，先验证 Viewer 外壳，再迁移正式控制界面。

Rerun 内置时间线、实体树和 3D 视图由 `re_viewer` 管理；项目自定义面板使用 `SidePanel`、`TopBottomPanel` 或局部 `egui_dock`，不再用 `egui_dock` 包裹整个 Rerun Viewer。

当前 `.rrd` 文件记录器继续保留作为离线回放出口。实时显示增加 Rerun `LogReceiver`/gRPC 输入，模型清单、TF、关节、轨迹、图像和点云可以同时写入实时流与 `.rrd` 文件。

## 数据边界

v1 的跨语言定义位于项目根目录的 `protocol/arm_console.proto`，C++ 网关和 Windows 客户端均以该文件生成代码；文档中的字段说明与协议保持一致。

### 控制通道

- `Connect`、`Enable`、`Stop`、`Jog`、`ExecuteTrajectory`、`ResetFault`。
- 每条命令带 `session_id`、唯一 `command_id`、客户端时间戳和协议版本。
- RPC 只返回接收结果；最终状态通过遥测事件回传。

### 遥测通道

每帧包含：

| 字段 | 约束 |
| --- | --- |
| `sequence` | 单调递增，用于丢包和乱序检测 |
| `timestamp_ns` | 仿真时钟或硬件时钟，统一纳秒 |
| `source` | `mock`、`mujoco`、`ros2`、`driver` |
| `quality` | `valid`、`stale`、`limited`、`fault` |
| `joint_position` / `joint_velocity` | 弧度、弧度每秒 |
| `tf` / `trajectory` / `sensors` | 与同一时间线关联 |

UI 只读取最新快照，不等待每一帧；后台使用有界队列，队列满时丢弃旧遥测而不是阻塞控制线程。

## 实施阶段

| 阶段 | 内容 | 验收条件 |
| --- | --- | --- |
| M0 基线 | Windows 原生 egui、中文字体、现代主题、模型资源 | 未设置 `DISPLAY` 也能启动和操作 UI |
| M1 协议冻结 | Protobuf/JSON schema、会话、心跳、命令确认、遥测帧 | Mock 客户端可完成握手和版本拒绝 |
| M2 数据源抽象 | UI 与 Mock、通道、gRPC/WebSocket 实现解耦 | UI 不引用 ROS 2 类型；可替换数据源 |
| M3 Headless 仿真 | C++ `SimulationDriver`、Mock 状态循环和可选 MuJoCo 驱动 | Docker 后台运行，无 GUI/X11；MuJoCo 模型可通过 XML 路径加载 |
| M4 嵌入式可视化 | 自定义 eframe 外壳嵌入 Rerun Native Viewer；接收模型、TF、轨迹、图像和点云 | 单窗口显示控制面板与 Rerun 时间线，可暂停、回放、按时间线检查 |
| M5 控制闭环 | Jog/停止/使能命令通道、状态机、停止优先级、超时和故障 | TCP 联调命令可确认；gRPC 生产协议仍需接入会话状态 |
| M6 回归与发布 | 丢包、乱序、断线、限位、帧率和资源许可测试 | 一键启动后台与桌面端，日志可复现 |

## 当前优化顺序

1. 在桌面端引入 `TelemetryFrame` 和可替换 `TelemetrySource`，先用 Mock 实现验证 UI 消费方式。
2. 将序列号、时间戳、来源和质量显示到状态栏，建立端到端观测字段。
3. 增加 C++ 网关的最小协议样例，打通关节状态、心跳、TF 和规划/实际轨迹。
4. 将样例状态计算替换为 MuJoCo 驱动；协议和 UI 不再变化。CMake 已提供 `ARM_CONSOLE_WITH_MUJOCO` 开关和模型路径参数。
5. 先以独立 `embedded_viewer` 入口验证 `re_viewer::App`、模型清单和最小自定义面板，再迁移正式控制界面；Windows UI 不启动任何 X11 客户端。
6. 将实时 Rerun `LogReceiver` 接入网关/遥测桥，同时保留可选 `rerun-recording` feature 写入 RRD；记录器只订阅 UI 遥测快照，不进入控制线程。
7. 最后接入真实驱动；在此之前只使用 MuJoCo、回放和 Mock 验证控制状态机。

当前推进状态：`embedded_viewer` 已完成最小自定义面板、Rerun gRPC 接收器和 `re_viewer::App` 生命周期接入。并行构建不受项目配置限制；首次构建前需要确保 Windows 页面文件已启用并有足够磁盘空间。

当前已完成：桌面端中文字体与现代主题、`TelemetryFrame`/Mock/有界通道适配器、C++ 共用的 `arm_console.proto`，以及不注入 `DISPLAY` 的 OpenRAVE Compose 和 ROS 2 headless 检查脚本。桌面端还提供了独立线程 TCP JSON 数据源和使能/停止/Jog 控制命令；`cpp/mock_gateway` 可在没有 X11 的情况下输出六关节、TF、规划轨迹和实际轨迹快照。现已提供基于 ROS 2 Jazzy Desktop Full 镜像的 Compose 服务，可直接启动该网关验证 Windows 数据链路；C++ 侧已抽出 `SimulationDriver`，并提供 MuJoCo 3.12.0 派生镜像和 `scene.xml` 加载入口。MuJoCo 镜像已完成真实 SDK 编译和运行验证，首帧前向运动学、包含夹爪的 10 条父节点相对 TF 及 Jog 控制均可通过 `scripts/verify-gateway.ps1` 检查。Rerun 记录桥已接入桌面端：可选 feature 写入关节、TF、轨迹、模型资源、编码图像和点云，模型清单为 STL 提供材质颜色和坐标帧映射，`rerun_sample` 可离线生成可回放的 `.rrd` 文件。下一步将同一记录边界接入正式 gRPC 流，并在网关侧增加图像/点云的有界采集与降采样策略。

## 完成定义

- Windows 上启动桌面端不需要 VcXsrv、`DISPLAY` 或容器内 GUI。
- 控制面板和 Rerun 3D/时间线位于同一个原生窗口和渲染循环中。
- C++ 仿真器输出的关节状态可在 UI 表格、曲线和 Rerun 时间线同时看到。
- UI 下发的命令能看到 `accepted → executing → completed/rejected` 完整状态。
- 心跳、断线、丢帧、数据过期和安全停止均有可见状态且不阻塞界面。
- 同一套协议可替换 Mock、MuJoCo、ROS 2 桥和未来驱动，不绑定机器人品牌。
