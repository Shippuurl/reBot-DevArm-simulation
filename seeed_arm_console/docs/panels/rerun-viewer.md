# Rerun 数据方案

Rerun 是唯一的观察、记录和回放层，不承担控制和规划决策。

## 启动

```bash
cargo run --features embedded-viewer --bin rebot_sim_viewer
```

Viewer 在 `127.0.0.1:9876` 接收 gRPC 数据。

Viewer 会在建立自身 `arm_gateway_grpc` recording 后自动读取 B601-RS 模型清单，将 25 个 STL 网格作为静态实体写入 `robot/frames/<link>/model`。模型和 ArmGateway 的动态 TF 位于同一个 recording，因此机械臂会随实时遥测运动；`rerun_sample` 只保留为独立样例和离线 `.rrd` 生成工具。

## 调试模式与环形历史

内嵌 Viewer 的 gRPC 接收器配置了按字节计量的历史上限，达到上限后优先丢弃最早的动态遥测，静态模型优先保留。默认值为 `256MiB`；调试时建议使用：

```bash
RERUN_DEBUG_MODE=1 RERUN_HISTORY_LIMIT=64MiB RERUN_TELEMETRY_RATE_HZ=30 \
  cargo run --features embedded-viewer --bin rebot_sim_viewer
```

等价的仓库脚本为：

```bash
scripts/run_viewer_debug.sh
```

`RERUN_HISTORY_LIMIT` 可单独设置为 `64MiB`、`256MB` 或百分比，`RERUN_TELEMETRY_RATE_HZ` 可单独设置 1–200 Hz。缓存按 chunk/字节回收，不保证固定帧数；Rerun 或系统分配器释放内存后 RSS 也可能暂时保持高水位。

需要观察 GC 时可运行：

```bash
RUST_LOG=re_grpc_server=info,re_viewer::app::logic=info \
  scripts/run_viewer_debug.sh
```

日志出现 `Dropping the oldest log messages` 即表示达到预算并开始淘汰。请确认系统中只
运行一个 Viewer 进程。

使用内嵌 Viewer 时，ArmGateway gRPC 遥测已经由 Viewer 自身记录，不要同时运行
`mujoco_rerun_bridge` 或 `rerun_sample` 作为第二个实时源；否则会看到额外的 recording
和更高的缓存增长。

## 实体树

![Rerun 实体树](/diagrams/rerun-entity-tree.svg)

源文件：[rerun-entity-tree.puml](https://github.com/your-org/seeed-arm-console/blob/main/docs/diagrams/rerun-entity-tree.puml)

link 必须通过 `world → robot/frames/<link>` 的变换链连接到视图目标帧；所有长度使用米，角度使用弧度，坐标系为右手 Z-up。

## 数据源

`rebot_sim_viewer` 启动时以 Rust gRPC client 连接 `ArmGateway` 50051：握手后订阅遥测，并把关节、TF、规划/实际轨迹、接触诊断和 `overhead_depth` 点云（每帧最多 768 点）写入当前 Rerun 记录。左侧控制面板的使能、停止和 J1 Jog 仅用于仿真联调。

Pinocchio/ProxSuite 规划服务的 SDK 冒烟：

```bash
scripts/run_planner_smoke.sh
```

该脚本通过 Python SDK 调用已运行的 `ArmPlanner`，不会在本地复制规划算法或生成
第二套 protobuf 客户端。规划服务若设置 `RERUN_GRPC_URL`，会把规划诊断写入同一
Viewer recording。

MuJoCo 遥测转发器：

```bash
cargo run --features rerun-recording --bin mujoco_rerun_bridge
```

两者均通过 `RERUN_GRPC_URL` 连接 Viewer，默认值为 `rerun+http://127.0.0.1:9876/proxy`。

`rerun_sample` 在线推送完成后只报告 gRPC 目标，不会检查不存在的本地 `.rrd`；取消
`RERUN_GRPC_URL` 才会进入离线文件模式。

## 记录原则

- 规划结果与实际轨迹使用独立实体路径 `planning/planned_trajectory` 和 `planning/actual_trajectory`，便于叠加比较。
- 每帧保留序列号和设备/仿真时间戳。
- 模型清单、URDF 和 MuJoCo 场景作为静态记录保存。
- 图像和点云使用有界缓存；记录失败不得阻塞控制路径。
- 规划与实际轨迹同时存在时，转发器记录 `diagnostics/trajectory_error/joint_rad`、`diagnostics/trajectory_error/rms_rad` 和 `diagnostics/trajectory_error/max_abs_rad`。
