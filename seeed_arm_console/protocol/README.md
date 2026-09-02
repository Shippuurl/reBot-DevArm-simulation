# 控制协议

`arm_console.proto` 是规划服务、MuJoCo 网关和 Rerun 工作站之间的唯一 v1 数据边界。

- 控制使用 unary RPC；遥测使用 server-streaming；连接握手使用 `ArmGateway.Handshake`。
- 所有运动命令带 `session_id`、`command_id` 和时间戳。
- 每次 `Handshake` 返回独立的短期 `session_id`；仿真网关默认在 1 小时无活动后清理并
  将会话表限制为 1024，新的客户端握手不会使已有客户端会话失效。生产服务应把会话
  绑定到认证身份并提供吊销。
- `CommandHeader.client_timestamp_ns` 使用 Unix 纳秒；非零值由网关校验，超过 5 秒未送达或超前超过 1 秒的命令返回 `REJECTED`。值为 0 表示未指定，仅保留给本地兼容诊断客户端。
- 遥测帧带 `sequence`、兼容别名 `timestamp_ns`、`sim_time_ns`、`wall_time_ns`、`source`、`quality`。
- MuJoCo 遥测可带有界的 `ContactState`（几何对、距离、法向力）诊断字段。
- `ArmPlanner` 提供 Pinocchio/ProxSuite 的 IK 和轨迹规划；规划响应必须携带模型版本、求解器、耗时和碰撞摘要。
- `SolveIK` 的 `PlanningMetadata.random_seed` 在当前确定性多初值实现中表示选中的候选索引（0–3），用于复现和诊断。
- `ExecuteTrajectoryCommand` 支持先 `dry_run` 校验、再提交仿真执行；网关限制最多 2000 点，要求首点时间为 0、时间单调、6 关节有限值、关节限位和 2 rad/s 速度上限。
- `PauseCommand` / `ResumeCommand` 控制当前仿真轨迹的冻结与恢复。暂停不会中断遥测流，驱动冻结轨迹时间并拒绝 Jog/新轨迹；恢复会跳过暂停期间的 wall-clock backlog。
- `SpeedScaleCommand.scale` 调整执行倍率，网关接受范围为 `[0.1, 2.0]`，只改变轨迹进度，不改变安全限位；越界值返回 `REJECTED`。
- `ImageFrame` / `PointCloudFrame` 是有界诊断数据：消费者每帧最多处理 4 个图像和 4 个点云，图像上限 8 MiB/4096×4096，点云上限 50,000 点；超限数据必须丢弃或降采样。
- `PlanTrajectory` 返回的轨迹由 Pinocchio/ProxSuite 生成：cubic smoothstep 提供首尾零速度，逐点 QP 投影施加关节盒和分段速度盒；网关仍需执行最终安全复核。
- 角度使用弧度，长度使用米；网关不启动任何 GUI。

C++/ROS 2 侧使用 protoc 和 gRPC 插件生成服务代码；Rust 侧使用 tonic 生成同一协议的类型。协议变更必须递增 package 版本，不复用已有字段编号。

`cpp/mock_gateway` 仍提供换行分隔 JSON 作为 legacy 适配层，供 Rerun 转发器联调。该格式不替代 gRPC 协议，也不承载生产控制命令。
