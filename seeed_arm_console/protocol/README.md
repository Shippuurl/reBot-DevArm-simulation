# 控制协议

`arm_console.proto` 是 `ArmPlanner`、`ArmGateway` 和 SDK 共用的协议源文件，package 为
`arm.console.v1`。协议单位统一为米、弧度、弧度/秒和 Unix 纳秒。

## 服务

| 服务 | RPC | 作用 |
| --- | --- | --- |
| `ArmGateway` | `Handshake` | 建立会话并返回协议、驱动来源和 DOF |
| `ArmGateway` | `Command` | 使能、Jog、轨迹、暂停、恢复、倍率、停止、故障复位 |
| `ArmGateway` | `SubscribeTelemetry` | server-streaming 状态帧 |
| `ArmPlanner` | `SolveIK` | 返回关节 IK 候选 |
| `ArmPlanner` | `PlanTrajectory` | 返回带时间和速度的轨迹候选 |

## 控制约定

1. 客户端先调用 `Handshake`，保存返回的 `session_id`；
2. 每个命令携带 `session_id`、`command_id` 和 Unix 纳秒 `client_timestamp_ns`；
3. 轨迹先以 `dry_run=true` 预检，再以 `false` 正式提交；
4. 命令接受状态查看 `CommandAck`，执行进度通过 `TelemetryFrame` 确认。

网关接受当前时间前 5 秒至后 1 秒的非零时间戳。仿真会话空闲 1 小时回收，最多保留
1024 个会话。`SpeedScale` 范围为 `[0.1, 2.0]`。

轨迹约束：最多 2000 个点；首点时间为 0；时间单调；每点包含 6 个有限关节值；显式
速度字段为空或包含 6 个有限值；位置遵守驱动模型的关节限位，点间速度上限为 2 rad/s。

## 遥测帧

`TelemetryFrame` 携带序列号、时间戳 `timestamp_ns`、`sim_time_ns`、`wall_time_ns`、
`source` 和 `quality`，并可附带：

- 关节位置和速度；
- 父子坐标变换（四元数按 `x,y,z,w`）；
- 规划轨迹和实际轨迹；
- 接触摘要、图像和点云。

传感器预算为每帧最多 4 个图像和 4 个点云；单张图像不超过 8 MiB、4096×4096；点云
最多 50,000 点。超限图像丢弃，点云均匀降采样。

## 代码生成与演进

Python、C++ 和 Rust SDK 在各自构建过程中生成私有服务代码，业务程序使用 SDK 的值类型。
新增字段保留已有编号并保持向后兼容；删除字段、改变单位或改变方法语义时，发布新的
协议版本和对应 SDK 主版本。

`cpp/mock_gateway` 的 50052 端口提供换行 JSON 诊断数据，供验证脚本和 Rerun 转发器使用。
控制集成请使用 50051 gRPC。跨主机配置见[安全部署](../docs/deployment/security.md)。
