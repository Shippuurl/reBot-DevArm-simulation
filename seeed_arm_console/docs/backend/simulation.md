# 规划与仿真后端

## Pinocchio + ProxSuite

规划服务运行在 Python 3 headless 环境中；ROS 2 Jazzy 只提供可选的薄编排适配层：

1. Pinocchio 从 B601-RS URDF 建立模型并计算 FK、雅可比和末端误差。
2. ProxSuite 求解带关节限位的增量 QP。
3. 每次迭代将目标位姿、末端位置、误差和关节值写入 Rerun。
4. 规划输出附带请求 ID、模型 SHA 摘要、求解器、耗时、限位结果和碰撞检查状态。

`PoseTarget` 的四元数使用 `x,y,z,w` 顺序；四个字段全为零时表示仅位置目标，提供单位四元数时同时优化姿态误差。

`SolveIK` 会按请求 seed、neutral、中位姿和确定性偏置最多尝试 4 个初值，优先选择同时满足位姿误差、碰撞余量与关节限位的候选；选中初值索引记录在 `PlanningMetadata.random_seed`，便于 Rerun 和故障复盘。

`PlanTrajectory` 使用 cubic smoothstep 连接起点和终点，保证首尾速度为零。服务根据 `--max-joint-speed`（默认 1 rad/s）和 `--max-joint-acceleration`（默认 2 rad/s²）选择轨迹时长，并在请求采样率下返回速度字段；每个采样点再经过 ProxSuite 约束投影，满足关节盒和相邻点速度盒。该实现仍是确定性的规划原型，后续可扩展为带加速度/碰撞约束的完整时空优化器。

正式服务接口使用 `protocol/arm_console.proto` 中的 `ArmPlanner.SolveIK` 和 `ArmPlanner.PlanTrajectory`；当前 Python 入口既作为服务实现，也作为无界面单次规划参考实现，二者共用同一数据边界。

入口：

```bash
scripts/run_planner_server.sh
```

正式规划服务已加载 Coal/Pinocchio 碰撞几何，并在请求启用时返回碰撞对、最近距离和
阈值判定。外部工程通过 SDK 调用服务；仓库内的 SDK 冒烟入口为
`scripts/run_planner_smoke.sh`，不会复制规划实现。

## MuJoCo 与 ArmGateway

MuJoCo 网关加载 `assets/robot/b601_rs/mujoco/scene.xml`，固定输出 6 个关节和 10 条 TF。正式链路是 `arm_console.proto` 定义的 gRPC：`127.0.0.1:50051` 提供 `Handshake`、`Command` 和 `SubscribeTelemetry`。C++ 服务直接锁定并调用 `SimulationDriver`，因此 JSON 与 gRPC 不会各自维护一份仿真状态。

`TelemetryFrame` 同时携带 MuJoCo 当前接触几何对、距离和法向力（最多 64 对）；Viewer/转发器只记录接触计数、最大法向力和最小距离等有界诊断量，不把 Rerun 作为安全 watchdog。该字段已完成编译链路，并已在 Compose MuJoCo 容器中完成运行回归。

图像与点云采用同一有界策略：每帧最多 4 个图像和 4 个点云，单个编码图像不超过 8 MiB 且尺寸不超过 4096×4096；点云在 Rust telemetry 边界按均匀步长降采样到最多 50,000 点，并同步保留颜色索引。超限图像会整帧丢弃（不截断压缩数据），超限点云则降采样后再写入 Rerun。MuJoCo 已接入无图形上下文的 `overhead_depth` 深度点云；真实 RGB 相机渲染仍待接入，接入时必须遵守同样预算。

MuJoCo 场景现在提供无图形上下文的 `overhead_depth` 固定相机。驱动在每个采样周期按
32×24 像素网格调用 `mj_ray`，将命中点转换为世界坐标后填入 `point_clouds`；这条路径
默认启用，可通过 `MUJOCO_ENABLE_DEPTH_SENSOR=0` 关闭。它是规划/避障诊断用的深度点云，
不等同于 RGB 渲染；真实相机图像接入仍需独立的渲染后端和设备预算审核。

Rust Viewer 启动时执行 gRPC 握手；握手失败或后续遥测流因网关重启/网络抖动结束时，都会以 250 ms 起始、5 s 封顶的指数退避重新握手并建立订阅。重连期间 UI 保留最近一帧并显示“重连中”，不会把 Rerun 或 Viewer 状态误判为设备安全状态。

Viewer 的 Rerun proxy 和 recording store 使用统一的字节历史预算，达到上限后优先淘汰最早的动态遥测 chunk，静态模型优先保留。默认上限为 `256MiB`；设置 `RERUN_DEBUG_MODE=1` 可切换为 `64MiB` 和 30 Hz 遥测，`RERUN_HISTORY_LIMIT`、`RERUN_TELEMETRY_RATE_HZ` 可分别覆盖缓存和订阅频率。该预算限制 recording 历史，不保证 RSS 立即下降，也不承担控制安全职责。

`ExecuteTrajectoryCommand` 的推荐流程是先发送 `dry_run=true`，确认网关返回 `ACCEPTED` 后再发送 `dry_run=false`。仿真执行适配器将轨迹按时间戳线性采样，MuJoCo 每个遥测周期调用 `mj_forward` 重算 TF 与接触；`Stop` 清除执行队列，`ResetFault` 清除仿真停止状态。该适配器不代表真实设备伺服实现，真机仍需独立的硬件 watchdog、急停和限位链路。

ArmGateway 对 `CommandHeader.client_timestamp_ns` 做新鲜度保护：非零时间戳必须在当前墙钟前后 5 秒/1 秒窗口内，过期或过度超前的控制命令会在进入驱动前拒绝。时间戳为 0 仅用于本地兼容诊断客户端；真机适配不得依赖该豁免，并仍需独立的硬件 watchdog。

执行控制还提供三个请求级动作：

- `Pause`：冻结当前轨迹进度。Mock 和 MuJoCo 驱动都会保持当前位置、将关节速度置零，并拒绝暂停期间的 Jog 与新轨迹请求；遥测仍保持输出，因此 Viewer 可以显示系统处于暂停状态。
- `Resume`：恢复暂停前的轨迹。恢复操作会丢弃暂停期间累积的 wall-clock 时间，不让 `mj_step` 或轨迹采样一次性追赶，从而避免恢复瞬间的跳变。
- `SpeedScale`：调整执行倍率，合法范围为 `[0.1, 2.0]`（默认 `1.0`）。倍率只影响轨迹时间推进，不放宽驱动侧的关节限位、有限值、单调时间戳和 2 rad/s 轨迹速度安全检查；越界值直接返回 `REJECTED`。

推荐的控制顺序是 `dry_run → ExecuteTrajectory → SpeedScale（可选）→ Pause/Resume（可选）→ Stop → ResetFault`。暂停/恢复和倍率是仿真控制语义，不能替代真实设备的伺服暂停、急停或硬件安全回路；真机适配必须重新定义失联、急停和 watchdog 行为。

换行 JSON 只在 `127.0.0.1:50052` 作为可关闭的 legacy adapter，供 Rerun 转发器和旧脚本使用：

```bash
python3 scripts/gateway_grpc_smoke.py
cargo run --features rerun-recording --bin mujoco_rerun_bridge
```

`mujoco_rerun_bridge` 默认连接 50052；可通过 `MUJOCO_GATEWAY_ADDR` 指向其他 JSON 兼容端点。Pinocchio/ProxSuite `ArmPlanner` 默认使用 50053，避免与网关端口冲突。

## 对外 SDK 边界

外部 Python、C++、Rust 或 ROS 2 工程只依赖官方 SDK，通过 `arm.console.v1`
调用 `ArmGateway` 和 `ArmPlanner`。SDK 返回 transport-neutral 数据类型，不暴露
Pinocchio、ProxSuite、MuJoCo、URDF 或 Rerun 对象。当前仓库提供 Python SDK v0.1、
C++ SDK v0.1 原型和 Rust SDK v0.1 源码包；TLS/认证和多租户仍是交付前工作。

## 安全边界

候选轨迹提交前必须检查：关节位置/速度限位、碰撞、轨迹时间单调性、数据新鲜度和仿真状态。规划服务重启不得影响 Viewer 或 MuJoCo。
