# 实施计划（进度与验证）

> 更新时间：2026-09-02  ·  操作步骤见[仿真工作站](/guide/simulation)。

本页是仿真主线的进度记录，顶部内容回答“现在交付到哪一步、下一步做什么”，底部保留完整实现和验证历史。

## 当前目标

把 B601-RS 仿真收口为可交付的 SDK-first 基线：业务程序通过 Python、C++ 或 Rust SDK 调用
`ArmPlanner` 和 `ArmGateway`，Rerun Viewer 作为观察层按需接入。

当前 MuJoCo 适配器用于运动学、接触和协议闭环验证；它还不是经过参数校准的动力学性能模型。
在宣称机械臂额定指标前，需要单独完成执行器模型、参数来源和真机对照验证。

视觉策略采用离线混合管线：RoboTwin 2.0 生成视觉数据，LingBot-VLA 负责后续策略训练与
推理，MuJoCo 用于回放和控制边界验证；不开发实时双引擎共仿真。

## 当前阶段

| 阶段 | 状态 | 已具备 | 下一出口 |
| --- | --- | --- | --- |
| 仿真基线 | 稳定性收口 | 6 DOF、TF、轨迹预检/执行、接触、深度点云和单调 recording 时间线 | 完成 30 分钟内存/吞吐回归 |
| SDK Beta | 发布阻塞 | 三语言客户端、示例和闭环 smoke | 发布包、干净环境安装、CI 和协议兼容矩阵 |
| 跨主机部署 | 未开始 | 本机 gRPC 链路和 SDK TLS 参数 | TLS/mTLS、身份授权、审计、重连端到端验收 |
| 仿真保真度 | 未开始 | 运动学与接触数据链路 | 执行器/摩擦/惯量校准，跟踪误差、速度、负载和重复性指标 |
| 离线视觉策略 | 规划中 | RoboTwin 数据、Adapter 和 LingBot-VLA 接入边界 | 一条可回放 episode、动作/观测 schema、策略回放指标 |
| 真机接入 | 安全门槛未通过 | 仿真命令边界和故障复位 | 驱动、急停、限位、watchdog、断链恢复及独立安全审核 |
| 真机数据与预训练 | 等待真机门槛 | 尚未采集真机策略数据 | 使用真机数据对 LingBot-VLA 进行预训练或微调，并完成 Sim-to-Real 评估 |
| 可选增强 | 延后 | 深度点云已在基线中；ROS 2 仅负责编排 | RGB 渲染、ROS 2 topic/service、高阶时空优化按需排期 |

## 下一里程碑

| ID | 交付物 | 退出条件 | 状态 |
| --- | --- | --- | --- |
| P0-1 | 实时 recording 时间线收口 | `frame`/`sim_time` 单调；轨迹相对时间不再反写共享 timeline；运行日志无 `trajectory_time` unsorted/out-of-order | 短测通过，长测待回归 |
| P0-2 | 长时运行回归 | 默认 `512MiB` 预算下连续运行至少 30 分钟，记录 RSS、吞吐、丢帧和重连次数 | 待处理 |
| P0-3 | 仿真指标基线 | 记录参数来源；完成位置/速度/加速度、跟踪误差、接触力、负载和重复性测试，并明确当前模型适用范围 | 待处理 |
| P1-1 | 无 Viewer 的 SDK 验收 | 在干净环境用三语言 SDK 完成握手、规划、预检、执行和遥测 smoke | 部分具备 |
| P1-2 | SDK 发布闭环 | Python wheel、C++ 安装包、Rust crate/source package 可安装；CI 固化构建和测试 | 待处理 |
| P1-3 | 协议与版本策略 | 明确当前/上一协议版本兼容范围，示例与兼容矩阵随版本发布 | 待处理 |
| P1-4 | 跨网络安全部署 | TLS/mTLS、客户端身份、授权、审计、session 吊销和重连测试通过 | 待处理 |
| P1-5 | 离线视觉策略回放 | RoboTwin episode 经 Adapter 输入 LingBot-VLA，策略动作通过 Planner/Gateway 预检并在 MuJoCo 回放 | 规划中 |
| P2-1 | 真机安全门槛 | 驱动、急停、限位、watchdog、断链恢复通过独立安全审核 | 未开始 |
| P2-2 | 真机数据与 LingBot-VLA 预训练 | 完成真机数据采集、标定和数据版本固定；训练/微调结果可复现，并通过 Sim-to-Real 对照评估 | 等待 P2-1 |

## 当前风险与边界

- Rerun 时间线修复已落地：实时帧使用 `frame`/`sim_time`，轨迹相对时间作为字段记录；仍需长时运行确认内存和吞吐。
- 网关当前的 `2 rad/s` 是仿真安全演示上限，不代表 B601-RS 额定速度；两者在指标报告中分开记录。
- 网关和规划服务默认面向本机回环；完成 TLS、身份和授权前，不把控制端口作为跨网络服务发布。
- 逐行 JSON 是 legacy 诊断适配层，主链路和验收以 gRPC/SDK 为准。

## 已验证基线（摘要）

| 领域 | 当前结果 |
| --- | --- |
| ArmGateway | 握手、独立 session、命令时间戳窗口、Enable/Stop/Jog、轨迹预检/执行、Pause/Resume、SpeedScale、ResetFault 和遥测已通过 MuJoCo 回归 |
| ArmPlanner | 多初值 IK、姿态目标、碰撞余量、ProxSuite 轨迹投影、速度/加速度字段和 Planner→Gateway 闭环已通过 |
| SDK | Python/C++/Rust v0.1 客户端、示例和 Viewer Rust 客户端迁移已通过；正式发布包、CI、兼容矩阵未完成 |
| Rerun | 模型、TF、规划/实际轨迹、接触和深度点云已接入；普通模式默认历史预算 `512MiB`，时间线修复已完成，长时回归待执行 |
| ROS 2 | 仅作为启动/编排适配层，可选，不是核心运行依赖 |
| 离线视觉策略 | RoboTwin 2.0 → LingBot-VLA → Planner/Gateway → MuJoCo 回放；实时双引擎共仿真不在范围内 |

## 交付门槛

| 门槛 | 必须满足 |
| --- | --- |
| 时间线稳定性 | `sequence`、`frame` 和 `sim_time` 单调；实时日志不出现 `trajectory_time` unsorted/out-of-order |
| 长时运行 | 默认 `512MiB` 历史预算下连续运行至少 30 分钟，并记录 RSS、吞吐、丢帧和重连次数 |
| SDK 可安装 | 在干净环境安装 Python wheel、C++ 包和 Rust crate/source package，三语言 smoke 均通过 |
| 协议兼容 | 发布包声明当前版本与上一版本的兼容范围，并有自动化检查 |
| 仿真指标 | 明确执行器、惯量、摩擦和接触参数来源；给出位置/速度/加速度、跟踪误差、接触力、负载和重复性数据 |
| 离线策略 | 固定 RoboTwin 与 LingBot-VLA 版本；观测、动作、时间戳可复现；策略输出经过 Planner/Gateway 预检 |
| 真机安全 | 急停、限位、watchdog、断链恢复和故障复位通过独立安全审核后，才允许接入真实设备 |

## 验证入口

以下命令覆盖当前 SDK-first 主链路；每条服务命令请在独立终端运行，Viewer 用于观察，
不作为服务可用性的前置条件。

```bash
docker compose -f docker-compose.gateway.yml \
  -f docker-compose.mujoco.yml up -d --build
scripts/run_gateway_grpc_smoke.sh
# 另开终端启动规划服务，再运行后两项
scripts/run_planner_server.sh
scripts/run_planner_smoke.sh
scripts/run_planner_gateway_smoke.sh
scripts/run_cpp_sdk_smoke.sh
scripts/run_rust_sdk_smoke.sh
cargo run --features embedded-viewer --bin rebot_sim_viewer
```

端口约定：`50051` 为 ArmGateway gRPC，`50053` 为 ArmPlanner，`9876` 为 Rerun Viewer；
`50052` 仅保留给逐行 JSON legacy 诊断适配层。

## 数据流

![规划与仿真数据流](/diagrams/planning-flow.svg)

::: details 已完成项（历史记录）

- Rerun 嵌入式 Viewer 入口 `rebot_sim_viewer`。
- 中文字体、主题和模型资源清单。
- B601-RS MuJoCo 场景、6 关节和 10 条 TF 的网关验证。
- MuJoCo JSON 遥测到 Rerun 的关节位置/速度/接触诊断转发器（仅 legacy 适配层）。
- Pinocchio + ProxSuite IK/QP Rerun 原型。
- `ArmPlanner` protobuf 已定义 IK、轨迹规划、规划元数据和碰撞摘要。
- `planner_grpc_server.py` 已提供多初值 `SolveIK` 和带速度/加速度边界的平滑 `PlanTrajectory` gRPC 实现（默认 50053）。IK 按请求 seed、neutral、中位姿和确定性偏置最多尝试 4 个初值，选择满足位姿、碰撞余量和限位的候选并在元数据返回候选索引；轨迹采用 cubic smoothstep，首尾速度为 0，并由 `--max-joint-speed` / `--max-joint-acceleration` 控制时长。
- ArmPlanner 已对 Pinocchio/Coal 可变计算工作区串行化请求，避免并发 RPC 互相污染；后续可改为每请求独立 Data 以提高吞吐。
- 碰撞白名单过滤改为逆序删除 `CollisionPair`，避免 Pinocchio 可变向量前向删除造成夹爪内部接触漏过滤。
- C++ `ArmGateway` 已接入 `SimulationDriver`，提供 `Handshake`、`Command` 和 `SubscribeTelemetry`。
- `TelemetryFrame` 已补充 MuJoCo 接触摘要（几何对、距离、法向力，最多 64 对）；C++/Rust/Rerun 记录接触计数、最大法向力和最小距离，已完成真实 MuJoCo 容器运行验证。
- 传感器边界已加入统一上限：每帧最多 4 张图像/4 个点云，单图像不超过 8 MiB，图像尺寸不超过 4096×4096，点云在记录前按均匀步长降采样至最多 50,000 点；超限压缩图像直接丢弃而不截断字节流。Rust Viewer gRPC 记录器和 legacy RerunRecorder 均执行同一预算并写入 `sensors/{sensor}/image|points`。
- MuJoCo 场景已加入 `overhead_depth` 固定相机；无图形上下文的网关使用 `mj_ray` 以 32×24 分辨率生成世界坐标深度点云，默认通过 gRPC 与 JSON 同步发布，可用 `MUJOCO_ENABLE_DEPTH_SENSOR=0` 关闭。
- gRPC 网关默认监听 50051；JSON 仅作为可关闭的 legacy adapter 监听 50052。
- `scripts/gateway_grpc_smoke.py` 已验证握手、会话必填、使能/Jog/dry-run/停止命令、6 关节/10 TF 遥测流。
- `SimulationDriver` 已接入 `ExecuteTrajectory` 仿真执行适配：支持 dry-run 与实际轨迹采样，统一执行前检查（6 关节、首点时间为 0、时间单调、有限值、关节限位、2 rad/s 速度上限、最多 2000 点）。MuJoCo 在每个遥测周期将轨迹采样写入 `mjData` 并重新计算 TF/接触；Mock 驱动保持同一行为。
- `ResetFault` 已接入 Mock/MuJoCo 驱动，用于停止状态后的仿真恢复；Jog、Stop 和新轨迹会清理旧执行队列，避免过期命令继续作用。
- `Pause`、`Resume` 和 `SpeedScale` 已接入 Mock/MuJoCo 驱动与 ArmGateway gRPC：暂停时冻结仿真/轨迹时间并拒绝 Jog 与新轨迹，恢复时丢弃暂停期间的 wall-clock backlog；执行倍率限制为 `[0.1, 2.0]`，越界请求拒绝。
- ArmGateway 对非零 `client_timestamp_ns` 增加命令新鲜度保护：允许当前时间前 5 秒至后 1 秒，过期/超前命令在进入驱动前拒绝；零值仅保留给兼容诊断，不能作为真机安全策略。
- 新增 `scripts/planner_gateway_smoke.py` 与 `scripts/run_planner_gateway_smoke.sh`，打通 `PlanTrajectory` → ArmGateway dry-run → 实际 MuJoCo 执行 → `SubscribeTelemetry` 回读 → Stop/ResetFault；MATE 阶段以 0.001 m 余量验证规划结果与网关预检交接。
- 新增 `scripts/restart_integration_smoke.sh`，两次重启 ArmPlanner 并在重启前、重启间和重启后运行 Gateway smoke，验证独立进程生命周期不会影响 MuJoCo 网关。
- `mujoco_rerun_bridge` 已增加 feature-safe fallback：未启用 `rerun-recording` 时仍可参加默认 `cargo test`/CI，只输出启动提示；启用该 feature 时保留 JSON→Rerun 转发能力。
- Rerun 模型资产的每个 `Asset3D` 实体已显式绑定所属 link 的 `CoordinateFrame`，且静态 link 变换显式声明 `child_frame`，修复 `tf#.../model/...` 无法追溯到 `world` 的变换树警告；`rerun_sample` 与在线 `RerunRecorder` 使用同一规则。
- 嵌入式 Viewer 启动时自动将 B601-RS 清单中的 25 个 STL 网格写入实时 `arm_gateway_grpc` recording；模型与 ArmGateway 动态 TF 共用实体树和时间轴，左侧控制台显示模型加载结果，不再要求另启 `rerun_sample` 才能查看机械臂。`rerun_sample` 复用同一模型加载器，避免样例与在线记录的实体树漂移。
- Rerun Viewer 已增加按字节计量的有界历史缓存：历史记录中的默认值为 `256MiB`；当前普通模式默认 `512MiB`，达到预算后由 gRPC proxy 和 Viewer 优先淘汰最早动态 chunk，静态模型优先保留；`RERUN_DEBUG_MODE=1` 默认切换为 `64MiB`/`30Hz`，也可用 `RERUN_HISTORY_LIMIT` 与 `RERUN_TELEMETRY_RATE_HZ` 单独覆盖。
- `scripts/run_planner_server.sh`、`scripts/run_gateway_grpc_smoke.sh` 已加入，自动优先使用 `.venv-planning`，避免 zsh 多行命令拆分。
- Python SDK v0.1 已完成首版产品边界：`sdk/python/rebot_sdk` 只依赖 gRPC/protobuf，提供 `ArmGatewayClient`、`ArmPlannerClient` 和 transport-neutral dataclass；覆盖可配置 client name、握手、会话、控制、轨迹、遥测、IK、规划、TLS 参数、metadata 和统一错误映射。外部工程不需要接触平台内部 Viewer、MuJoCo、Pinocchio、ProxSuite、URDF 或 ROS 2。
- 已新增独立 SDK 指南 `docs/sdk/python.md` 并加入 VitePress 导航；系统上下文 PlantUML 已改为“外部工程 → 官方 SDK → ArmGateway/ArmPlanner”，ROS 2 明确为可选薄适配层，生成 SVG 已同步。
- C++ ArmGateway 会话已从固定共享 ID 改为每次握手独立 session，保留 1 小时无活动 TTL 和 1024 会话上限；第二客户端握手不会使已有控制/遥测客户端失效。此隔离仍不等同于身份认证，生产环境必须绑定 TLS 身份、授权和审计。
- C++ SDK v0.1 原型已加入 `sdk/cpp`：PIMPL 隐藏生成 protobuf，提供 `ArmGatewayClient`（握手、控制、轨迹、遥测）和 `ArmPlannerClient`（IK/轨迹）transport-neutral API，支持调用方传入 insecure/TLS gRPC channel，并提供网关/规划示例。
- 新增 `scripts/run_cpp_sdk_smoke.sh`，在运行中的 MuJoCo Compose 容器内完成 C++ SDK 构建和网关示例回归；可通过 `CPP_SDK_PLANNER_ADDRESS` 追加规划示例。
- Rust SDK v0.1 源码包已加入 `sdk/rust`：protobuf 生成模块保持私有，提供 `ArmGatewayClient`、`ArmPlannerClient`、网关命令、遥测流和规划结果的 transport-neutral API；Viewer 已改为依赖该 SDK，不再维护 `src/grpc_client.rs` 的 Viewer 专用客户端。
- Rust SDK 已提供网关/规划示例，Python `planner_grpc_smoke.py` 与 `planner_gateway_smoke.py` 已改为调用 `sdk/python`，仅 `gateway_grpc_smoke.py` 保留底层 wire-compatibility 覆盖。
- 新增 `scripts/run_rust_sdk_smoke.sh`，默认运行 Rust 网关示例，设置 `RUST_SDK_RUN_PLANNER=1` 时追加规划示例，避免手工拼接 Cargo 参数。
- Rust/Rerun Viewer 控制台已接入 `Pause`、`Resume` 和 `SpeedScale` gRPC 控件；倍率滑块限制在 0.1–2.0，并复用统一命令状态反馈。遥测订阅断流后按 250 ms–5 s 指数退避自动重连，状态栏显示重连状态。
- 旧控制台、Windows 脚本和 OpenRAVE 文档/配置已归档。

:::

::: details 状态总览（历史快照）

| 模块 | 状态 | 说明 |
| --- | --- | --- |
| Rerun Viewer | 可运行 | `rebot_sim_viewer`，gRPC 接收端口 9876；启动时自动加载 25 个模型网格并与实时 TF 使用同一 recording，历史缓存有界 |
| MuJoCo 网关 | 可运行 | 6 关节、10 TF、使能/停止/Jog、ExecuteTrajectory、Pause/Resume、SpeedScale、ResetFault、JSON 和 gRPC 遥测均已在 MuJoCo 3.12.0 容器验证；接触摘要随执行轨迹实时刷新 |
| MuJoCo → Rerun | 部分完成 | 关节、10 条 TF、规划/实际轨迹、误差、接触摘要、`sim_time`/`wall_time` 和 MuJoCo `overhead_depth` 点云已接入；图像/点云支持有界解析、降采样和 Viewer 记录，真实 RGB 相机渲染仍待接入 |
| Pinocchio IK | 可运行 | B601-RS URDF，输出 6 个机械臂关节；零四元数表示仅位置目标，非零四元数参与姿态误差；目标帧限制为 `world` |
| ProxSuite QP | 可运行 | IK 增量 QP；轨迹采样增加逐点 ProxSuite 约束投影（关节盒与分段速度盒），使用 cubic smoothstep 满足基础关节速度/加速度上限并返回速度字段；更高阶时空优化仍可扩展 |
| ArmPlanner gRPC | 原型可用 | 多初值 `SolveIK`、平滑 `PlanTrajectory` 和 smoke client 已通过；Pinocchio/Coal 工作区已串行化，轨迹响应携带连续碰撞摘要，并已有 Planner→Gateway→MuJoCo 及重启联调脚本 |
| ArmGateway gRPC | 仿真原型已验证 | C++ 服务已绑定 `SimulationDriver`；50051 提供握手、控制、轨迹执行、故障复位和遥测，50052 保留 JSON 兼容端口；TLS、授权和真机安全尚未完成 |
| Rust gRPC client / SDK | v0.1 可运行 | `sdk/rust` 提供公共 transport-neutral 网关/规划客户端；Viewer 复用该 SDK，断流后自动重连并将关节/TF/轨迹帧写入自身 Rerun 记录，控制控件保持可用 |
| 碰撞检查 | 原型可用 | Coal 几何已加载；已过滤相邻 link 和夹爪装配接触，并返回碰撞对名称/最近距离；规划默认阈值 0.02 m，MATE 无显式覆盖时 0.001 m，支持请求级 ACM 白名单；Gateway 暂不执行独立运行时碰撞监控 |
| Python SDK | v0.1 可用 | `sdk/python` 提供外部工程的协议级客户端；不依赖平台内部实现 |
| C++ SDK | v0.1 原型可用 | `sdk/cpp` 隐藏 protobuf 生成细节，覆盖 ArmGateway/ArmPlanner；待发布包、CI 和兼容矩阵 |
| ROS 2 Jazzy package | 可选适配层 | `ros2_ws/src/pinocchio_planner` 仅编排独立 ArmPlanner gRPC 服务，不属于平台核心依赖 |
| ArmGateway gRPC | 仿真原型链路已验证 | gRPC 已成为默认链路；协议版本、独立 session（1 小时 TTL、最多 1024 个）、命令时间戳新鲜度、Jog/轨迹边界、限位、dry-run、MuJoCo 执行、Pause/Resume、SpeedScale、ResetFault 及 Planner→Gateway 联调已接入；TLS、授权、真实设备执行适配、硬件 watchdog 和真机链路断连恢复仍待完成 |

:::

::: details 后续工作（历史任务清单）

**P0：完成 Rerun 实时链路**

1. ~~MuJoCo 转发器已补齐 10 条 TF、`sim_time`、`wall_time` 和帧号；协议字段统一仍待完成。~~ 已在 `TelemetryFrame` 统一 `sequence`、`sim_time_ns`、`wall_time_ns`，保留 `timestamp_ns` 兼容别名。
2. ~~接入 MuJoCo 接触摘要与有界诊断。~~ 已完成；图像/点云完成统一上限、超限丢弃与点云降采样，MuJoCo `overhead_depth` 通过 CPU 射线采样接入 32×24 深度点云；真实图形渲染 RGB 相机仍待接入。
3. ~~在同一时间轴叠加 `planned_trajectory`、`actual_trajectory` 和误差统计。~~ 已完成：Rust Viewer、JSON 转发器和 MuJoCo gRPC 遥测均写入规划/实际轨迹及关节误差诊断实体。
4. ~~为调试模式增加有界 recording 历史。~~ 已完成：gRPC proxy 与 Viewer 使用统一字节预算，动态旧 chunk 自动淘汰、静态模型保留，并支持 `RERUN_DEBUG_MODE`、`RERUN_HISTORY_LIMIT` 和 `RERUN_TELEMETRY_RATE_HZ`。

**P1：完善 Pinocchio/ProxSuite 规划**

4. ~~接入完整碰撞几何，过滤相邻 link 的允许接触并报告碰撞对名称和最近距离。~~ 已完成请求级 ACM 白名单、规划默认 0.02 m 和 MATE 阶段 0.001 m 覆盖；Gateway 尚无独立运行时碰撞监控，设备力/力矩 watchdog 仍待接入。
5. ~~增加姿态方向误差、基础关节速度/加速度约束和多初值 IK。~~ 已接入非零目标四元数的姿态误差、Rerun 诊断、4 组确定性 IK 初值、cubic smoothstep 速度和加速度边界；后续可增加更丰富的随机/姿态分支采样。
6. ~~对每个插值轨迹点执行连续碰撞/速度检查。~~ 已完成轨迹点碰撞/余量扫描与速度字段，并加入 ProxSuite 逐点关节盒/分段速度盒投影；带加速度与碰撞约束的完整时空优化仍待完成。

**P1：正式 gRPC 与 ROS 2**

7. ~~用 `arm_console.proto` 实现 C++ `ArmGateway` 服务。~~ 已完成基础 `SimulationDriver` 适配、服务启动和三 RPC。
8. ~~实现 Rust Viewer gRPC 客户端和遥测订阅。~~ 已完成握手、启动失败重试、流断开指数退避重连、控制命令 UI（使能、停止、Jog、Pause、Resume、SpeedScale）及关节/TF/轨迹实体映射；真实硬件控制策略仍需独立安全审核。
9. ~~新增 ROS 2 Jazzy `pinocchio_planner` package、参数和 launch 文件。~~ 已完成最小 ament_python 编排包；ROS topic/service 类型接入规划状态和目标仍待完成。

**P1：官方 SDK**

10. ~~提供外部工程可直接安装的 Python SDK。~~ 已完成 v0.1：打包生成的 protobuf stubs，提供网关控制/遥测、规划 RPC、TLS 参数、metadata、协议版本校验和 `RebotRpcError`；中文接入指南已纳入 VitePress。
11. 将 SDK 的遥测断流重连策略固化为可选 API（当前同步 `subscribe_telemetry()` 将错误映射给调用方，由应用负责退避重订阅），并补充真实 TLS 端到端测试。
12. ~~实现 C++ 公共 SDK 原型。~~ 已完成 `sdk/cpp` PIMPL 客户端、CMake 构建、安装导出和 MuJoCo 网关/规划实连示例；继续完成发布包、CI 和兼容矩阵。~~实现 Rust 公共 SDK。~~ 已完成 `sdk/rust` v0.1 源码包、私有 protobuf、网关/规划客户端、示例与 Viewer 迁移；后续补充发布包、CI、兼容矩阵和 TLS 端到端测试。

**P2：集成验收与交付**

13. ~~验证越界、无解、碰撞、轨迹速度/时间、执行、Pause/Resume、SpeedScale、时间戳新鲜度、Stop/ResetFault。~~ 已完成上述仿真回归；硬件 watchdog 仍待实现。
14. 验证规划服务重启不影响 MuJoCo 与 Viewer：已验证规划服务两次重启不影响 MuJoCo 网关；Viewer 通过独立 gRPC/Rerun 接收端解耦，完整 GUI 生命周期测试待无头渲染环境固化。
15. 固化 CI、启动配置、故障排查和从零部署文档；对外发布前完成网关 TLS、客户端身份绑定、授权、session 吊销/租约策略、多租户隔离和审计。

:::

::: details 验收标准（历史记录）

- 固定目标位姿可得到满足关节限位的候选解。
- 越界、无解和碰撞目标会被拒绝并记录原因。
- 候选轨迹提交 MuJoCo 前执行二次安全检查。
- Viewer 可同时显示模型、TF、规划轨迹、实际轨迹和误差。
- 任一规划服务重启不影响 MuJoCo 和 Viewer 的运行。
- 外部 Python 工程仅安装 SDK 即可完成握手、规划、控制和遥测订阅，不需要平台源码或 ROS 2 环境。

:::

::: details 维护规则与历史验证记录

**维护规则**

后续实施结果、验证记录和状态变更直接更新本计划书；本文件是仿真主线的唯一进度记录。

**当前命令（历史入口）**

```bash
cargo run --features embedded-viewer --bin rebot_sim_viewer
python3 scripts/gateway_grpc_smoke.py       # ArmGateway 50051
python3 scripts/verify_gateway.py           # legacy JSON 50052
python3 scripts/planner_grpc_server.py      # ArmPlanner 50053
python3 scripts/planner_grpc_smoke.py       # Python SDK smoke
# 等价的无环境歧义入口：
scripts/run_planner_server.sh
scripts/run_gateway_grpc_smoke.sh
scripts/run_planner_smoke.sh
scripts/run_planner_gateway_smoke.sh  # Planner → Gateway → MuJoCo 闭环
scripts/restart_integration_smoke.sh  # Planner 重启与 Gateway 稳定性
scripts/run_viewer_debug.sh           # 64MiB/30Hz 有界调试 Viewer
scripts/run_cpp_sdk_smoke.sh          # 容器内 C++ SDK 网关/可选规划示例
scripts/run_rust_sdk_smoke.sh         # Rust SDK 网关；RUST_SDK_RUN_PLANNER=1 追加规划
```

**最近验证记录**

| 日期 | 验证 | 结果 |
| --- | --- | --- |
| 2026-09-01 | CMake mock gateway（无 gRPC）构建 | 通过 |
| 2026-09-01 | CMake `ARM_CONSOLE_WITH_GRPC=ON` + Protobuf/gRPC 生成与链接 | 通过 |
| 2026-09-01 | `gateway_grpc_smoke.py`：Handshake、会话、Enable、Jog、dry-run、Stop、SubscribeTelemetry | 通过：6 DOF、10 TF、时间字段有效 |
| 2026-09-01 | C++ 网关 `0.0.0.0` 容器绑定 + 主机回环客户端 smoke | 通过；默认主机仍绑定 127.0.0.1 |
| 2026-09-01 | ArmGateway gRPC Jog 边界、dry-run 轨迹和 Stop smoke | 通过：非法 ±1.0 rad Jog 被拒绝 |
| 2026-09-01 | Rust `cargo check --features embedded-viewer` | 通过（仅 design 常量 warning） |
| 2026-09-01 | Rust `cargo check --features rerun-recording --bin mujoco_rerun_bridge` | 通过 |
| 2026-09-01 | Rust Viewer gRPC 帧记录（关节、TF、轨迹）编译验证 | 通过 |
| 2026-09-01 | ArmPlanner 默认阈值与 1.0 m 严格余量 smoke | 通过：严格余量被拒绝 |
| 2026-09-01 | ArmPlanner 轨迹采样率、单调时间戳和速度字段 smoke | 通过：41 点、单调时间、6 维速度 |
| 2026-09-01 | ArmPlanner `PlanTrajectory(check_collisions=true)` 轨迹点余量扫描 | 通过：响应携带连续扫描摘要并拒绝违规轨迹 |
| 2026-09-01 | ArmPlanner 目标帧校验 | 通过：非 `world` 请求明确拒绝 |
| 2026-09-01 | ArmPlanner 姿态目标、目标帧、速度字段和全脚本 py_compile 回归 | 通过 |
| 2026-09-01 | ArmPlanner 夹爪内部 ACM 过滤与请求级白名单 | 通过：默认碰撞对 33，`base_link/gripper_end` 白名单使检查对降至 32 |
| 2026-09-01 | ROS 2 Jazzy `colcon build --symlink-install --packages-select pinocchio_planner` | 通过 |
| 2026-09-01 | `ros2 launch pinocchio_planner planner.launch.py` | 通过：ArmPlanner 50053 启动 |
| 2026-09-01 | TelemetryFrame 时间字段与 ContactState 协议生成（C++/Rust） | 通过 |
| 2026-09-01 | MuJoCo 接触摘要 64 对上限与 Rerun 有界诊断路径 | 代码完成；容器运行在 2026-09-02 回归通过 |
| 2026-09-01 | VitePress `npm run docs:build` | 通过 |
| 2026-09-01 | zsh-safe planning wrapper脚本 + gRPC smoke | 通过：`.venv-planning` 自动选择、41 点轨迹 |
| 2026-09-02 | Docker Compose MuJoCo 3.12.0 镜像构建与启动 | 通过：gRPC/JSON 双端口，容器内 CMake + Protobuf/gRPC + MuJoCo 构建成功 |
| 2026-09-02 | `gateway_grpc_smoke.py` 执行扩展 | 通过：dry-run、实际 2 点轨迹、轨迹安全拒绝、遥测观察到 J1 运动、Stop、ResetFault |
| 2026-09-02 | `verify_gateway.py` MuJoCo JSON 回归 | 通过：6 DOF、10 TF、遥测与 Enable/Jog 控制 |
| 2026-09-02 | Planner→Gateway→MuJoCo 联调 smoke | 通过：MATE 0.001 m 规划、41 点 dry-run/实际执行、遥测回读 41 点、Stop/ResetFault |
| 2026-09-02 | ArmPlanner 平滑轨迹加速度回归 | 通过：cubic smoothstep 首尾零速度，smoke 计算离散加速度不超过 2.05 rad/s² |
| 2026-09-02 | 平滑轨迹 Planner→Gateway→MuJoCo 闭环 | 通过：47 点轨迹完成 dry-run、实际执行、遥测回读及 Stop/ResetFault |
| 2026-09-02 | ArmPlanner 多初值 IK 回归 | 通过：最多 4 个确定性初值，响应 `PlanningMetadata.random_seed` 返回选中候选索引，基础 smoke 通过 |
| 2026-09-02 | ArmPlanner ProxSuite 轨迹投影回归 | 通过：每个 smoothstep 采样点经 ProxSuite 关节盒/分段速度盒约束投影，47 点轨迹速度与加速度 smoke 通过 |
| 2026-09-02 | Planner 重启集成回归 | 通过：ArmPlanner 重启 2 次，重启前/间/后 Gateway gRPC smoke 均通过 |
| 2026-09-02 | ArmGateway Pause/Resume 与 SpeedScale 回归 | 通过：1.5 倍率接受、3.0 越界拒绝；暂停期间连续遥测帧轨迹时间冻结，恢复后继续推进 |
| 2026-09-02 | Rust Viewer Pause/Resume/SpeedScale 控件编译回归 | 通过：`cargo check --features embedded-viewer`，控件生成共享协议 payload 并沿用命令状态反馈 |
| 2026-09-02 | Rust Viewer 遥测断流恢复代码回归 | 通过：`cargo check --features embedded-viewer`；订阅异常按 250 ms 起始、5 s 上限退避重连并更新状态栏 |
| 2026-09-02 | 图像/点云有界解析与降采样回归 | 通过：新增 Rust library telemetry tests，超限图像丢弃、点云均匀降采样至 50,000 点且颜色数组保持对齐 |
| 2026-09-02 | Viewer 传感器 Rerun 映射回归 | 通过：gRPC Viewer 记录器对图像/点云执行同一预算，写入 `sensors/{sensor}/image|points`，`cargo check --features embedded-viewer` 通过 |
| 2026-09-02 | Rust 默认/可选 feature 回归 | 通过：`cargo test --no-default-features` 与 `cargo test --features rerun-recording --bin mujoco_rerun_bridge` |
| 2026-09-02 | Rerun 样例记录生成与模型帧绑定 | 通过：生成 17 MB `.rrd` 样例，所有模型实体显式继承 link 坐标帧；Viewer 变换树修复已编译验证 |
| 2026-09-02 | 最终组合回归 | 通过：MuJoCo gRPC/JSON、Planner smoke、Planner→Gateway 闭环、Rust check/test、Python py_compile、VitePress build、`git diff --check` |
| 2026-09-02 | ArmGateway 命令时间戳新鲜度回归 | 代码完成：非零时间戳允许前 5 秒/后 1 秒窗口；gateway smoke 增加过期与未来命令拒绝断言 |
| 2026-09-02 | ArmGateway 时间戳保护容器回归 | 通过：Compose 重建 MuJoCo 3.12.0 镜像；gRPC smoke 输出 `timestamp_guard=valid_accepted,stale_rejected,future_rejected`，JSON verify、Planner smoke 与闭环均通过 |
| 2026-09-02 | ROS 2 Jazzy package 增量构建 | 通过：清理 ROS 环境变量后 `colcon build --symlink-install --packages-select pinocchio_planner` |
| 2026-09-02 | 仿真引导文档同步 | 通过：`docs/guide/simulation.md` 补充 Viewer 控制、时间戳安全窗口和 `.venv-planning` 使用说明，VitePress build 通过 |
| 2026-09-02 | Pause/Resume/SpeedScale 文档与实现一致性回归 | 通过：协议、后端、C++ 网关说明同步；gRPC smoke 验证暂停冻结/恢复推进、1.5 倍率接受和 3.0 越界拒绝 |
| 2026-09-02 | 最新组合回归 | 通过：MuJoCo gRPC/JSON（含 valid/stale/future 时间戳保护）、Planner/ProxSuite 轨迹投影 smoke、Planner→Gateway 闭环、Planner 重启、Rust 7 项单元测试与 feature check、ROS 2 增量构建、Python py_compile、VitePress build、`git diff --check` |
| 2026-09-02 | Viewer 实时模型自动加载 | 代码完成：B601-RS 25 个网格与 gRPC 遥测复用 `arm_gateway_grpc` recording，控制台显示加载状态；`rerun_sample` 复用相同模型加载器；Viewer check、Rerun recording 单测和样例 `.rrd` 生成均通过 |
| 2026-09-02 | Viewer 调试模式环形历史 | 通过：gRPC proxy 与内嵌 Viewer 同步 `RERUN_HISTORY_LIMIT`，`RERUN_DEBUG_MODE=1` 默认 64 MiB/30 Hz，旧动态 chunk 自动淘汰且静态模型保留；Viewer 编译检查通过 |
| 2026-09-02 | Viewer 有界调试启动脚本 | 通过：`scripts/run_viewer_debug.sh` 固化 `64MiB`/`30Hz`/9876 默认值，支持环境变量覆盖；文档补充 `RERUN_HISTORY_LIMIT=0` 和避免重复实时 recording 的说明 |
| 2026-09-02 | `rerun_sample` 在线模式回归 | 通过：设置 `RERUN_GRPC_URL` 时只报告 `recording=grpc`，不再错误读取不存在的本地 `.rrd`；离线模式仍输出文件大小 |
| 2026-09-02 | MuJoCo 深度传感器接入 | 通过：`scene.xml` 增加 `overhead_depth` 相机，驱动使用 `mj_ray` 输出 768 个世界坐标点；gRPC/JSON smoke 均验证 `depth_points=768`，可用 `MUJOCO_ENABLE_DEPTH_SENSOR=0` 关闭 |
| 2026-09-02 | JSON→Rerun 深度点云转发 | 通过：legacy `mujoco_rerun_bridge` 解析 `point_clouds`、执行有限降采样并写入 `sensors/overhead_depth/points`，Rust bridge feature check 通过 |
| 2026-09-02 | Python SDK v0.1 产品化整理 | 通过：SDK 单元测试 3 项、安装/import 验证和 Gateway/Planner 集成验证通过；新增中文 SDK 指南、VitePress 导航、SDK 边界架构图与 SVG，明确 ROS 2 为可选薄适配层 |
| 2026-09-02 | ArmGateway 独立 session 回归 | 通过：每次 Handshake 返回不同 session；同一 gRPC smoke 在第二客户端握手后继续完成控制、遥测、Pause/Resume、Stop/ResetFault；会话 1 小时 TTL/1024 上限代码随 MuJoCo 容器重新编译运行通过 |
| 2026-09-02 | C++ SDK v0.1 原型回归 | 通过：容器内 CMake 生成私有 protobuf/gRPC stubs，`rebot_arm_sdk`、网关和规划示例构建成功；示例完成网关握手/使能/遥测回调/停止/故障复位，以及 ArmPlanner IK/轨迹调用 |
| 2026-09-02 | C++ SDK 安装导出回归 | 通过：`cmake --install` 生成静态库、公共头文件和 `rebot_arm_sdkConfig.cmake`/targets，可供外部 CMake 工程 `find_package` 使用 |
| 2026-09-02 | C++ SDK smoke wrapper | 通过：`scripts/run_cpp_sdk_smoke.sh` 在 `arm-console-gateway` 容器中构建 SDK 并运行网关示例；注入可达的 `CPP_SDK_PLANNER_ADDRESS=172.18.0.1:50054` 后规划示例也返回 IK/47 点轨迹 |
| 2026-09-02 | SDK/平台组合回归 | 通过：Python SDK 3 项单测、Gateway/Planner Python SDK 实连、C++ SDK 网关/规划示例实连、Gateway/JSON/Planner smoke、Rust tests/check、VitePress build 和 `git diff --check` 均通过；仅 Rust 设计常量保留既有 dead-code warning |
| 2026-09-02 | Rust 公共 SDK v0.1 | 通过：`sdk/rust` 私有 protobuf 生成、transport-neutral 网关/规划 API、3 项单测和网关/规划示例构建完成；网关示例实连 MuJoCo 返回独立 session、6 DOF 遥测和控制 ACK，规划示例返回 47 点轨迹 |
| 2026-09-02 | Python SDK smoke 收敛 | 通过：`planner_grpc_smoke.py`、`planner_gateway_smoke.py` 改用 `sdk/python`，底层动态 protobuf 仅保留在 `gateway_grpc_smoke.py` wire-compatibility 测试；规划和 Planner→Gateway→MuJoCo 闭环均通过 |
| 2026-09-02 | Viewer Rust 客户端迁移 | 通过：`rebot_sim_viewer` 改用 `sdk/rust`，`src/grpc_client.rs` 删除；`cargo test --no-default-features`、`cargo check --features embedded-viewer` 与 Rerun 记录路径通过 |
| 2026-09-02 | 规划入口去重与文档同步 | 通过：删除独立 `pinocchio_proxsuite_rerun.py` 入口，统一 `planner_grpc_server.py` 为唯一规划实现；Rerun/仿真/架构/SDK 文档改用 SDK smoke 和 Rust SDK 页面，VitePress 构建通过 |
| 2026-09-02 | Rust SDK smoke wrapper | 通过：`scripts/run_rust_sdk_smoke.sh` 网关示例返回独立 session、6 DOF 遥测和控制 ACK；`RUST_SDK_RUN_PLANNER=1` 追加规划示例并返回 47 点轨迹 |
| 2026-09-02 | Rust SDK TLS/输入边界回归 | 通过：`tls-native-roots` feature 编译成功；未知 `assembly_phase` 返回 `INVALID_ARGUMENT`；Rust SDK 单测增至 4 项 |
| 2026-09-02 | SDK/Viewer/文档最终回归 | 通过：Python planner/闭环 smoke、Rust SDK 网关/规划 wrapper、Gateway gRPC smoke、Rust 默认/embedded checks、Rust SDK 单测、VitePress build 和 `git diff --check` 全部通过；仅既有 design 常量 dead-code warning |
| 2026-09-02 | Rerun 实时时间线回退修复 | 通过：轨迹相对时间改为字段，Viewer 对网关重启后的 `frame/sim_time` 做单调化；`cargo check`、Viewer 单测和 Rerun bridge check 通过；30 分钟长时回归待执行 |

最近一次回归组合（gRPC gateway smoke、Python SDK planner/闭环 smoke、Rust SDK 单测与
Viewer check、Rust SDK 网关/规划示例、C++ SDK 容器 smoke、VitePress build、ROS 2
launch）均通过；仅 Rust 设计常量保留既有 dead-code warning。当前仍未完成 C++/Rust
SDK 正式发布包/CI/兼容矩阵、跨网络 TLS/认证/多租户和真实设备安全链路。

Docker Compose 的 MuJoCo 镜像构建配置已切换为 gRPC + JSON 双端口；2026-09-02 已在当前主机完成实际镜像构建、启动和双协议回归。容器默认持续运行，可用 `docker compose -f docker-compose.gateway.yml -f docker-compose.mujoco.yml down` 停止。

**端口约定（历史记录）**

- `50051`：ArmGateway gRPC（主链路）。
- `50052`：逐行 JSON legacy adapter，供 `mujoco_rerun_bridge` 和旧联调脚本使用；设置 `ARM_CONSOLE_ENABLE_JSON=0` 可关闭。
- `50053`：Pinocchio/ProxSuite ArmPlanner gRPC。
- `9876`：嵌入式 Rerun Viewer 接收端。

主机进程默认只绑定 `127.0.0.1`；Compose 覆盖层显式设置两个 bind address 为 `0.0.0.0`，再由 Docker 仅发布到主机回环地址，避免把控制端口暴露到局域网。

:::
