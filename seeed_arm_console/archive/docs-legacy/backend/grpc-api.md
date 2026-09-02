# gRPC API

gRPC 是 UI 与控制网关之间的传输层。协议对象独立于具体机器人型号，ROS 2 topic/service 的映射由 `ros2-bridge` 实现。字段的唯一来源是仓库根目录的 `protocol/arm_console.proto`；本页只说明边界和使用约束。

## 服务边界

```text
ArmGateway.Connect
ArmGateway.Command
ArmGateway.SubscribeTelemetry
```

控制 RPC 返回“已接受/已拒绝”和 `command_id`，最终执行结果通过状态流回传。不要把“RPC 返回成功”解释为机械臂已经完成动作。

控制命令通过 `ControlCommand` 的 `oneof payload` 表达 Enable、Stop、Jog、ExecuteTrajectory 和 ResetFault。字段命名使用 `snake_case`，角度统一使用弧度，时间使用纳秒整数。协议版本放在握手请求和响应中，禁止静默改变字段语义。

## 遥测流

遥测消息应带有采样时间、来源和序列号：

| 字段 | 作用 |
| --- | --- |
| `sequence` | 检测丢包和乱序 |
| `timestamp_ns` | 与 Rerun 时间线对齐 |
| `source` | mock、mujoco、driver 或 ros2 |
| `tf` / `planned_trajectory` / `actual_trajectory` | 与同一时间戳关联的坐标和轨迹 |
| `images` / `point_clouds` | 传感器图像和米制 XYZ 点云；大帧由有界队列限流 |
| `quality` | valid、stale、limited、fault |

客户端不应阻塞等待每一帧遥测。使用有界订阅队列，UI 丢弃旧帧并保留最新快照。

## 本地联调传输

`cpp/mock_gateway` 提供临时的换行分隔 JSON 服务器，使用与 `TelemetryFrame` 相同的字段名（`joint_position_rad`、`joint_velocity_rad_s`）。它只用于在 Windows 上验证无 X11 的数据链路；正式网关必须实现本页的 gRPC 服务。

## 错误模型

错误至少分为 `INVALID_ARGUMENT`、`NOT_CONNECTED`、`NOT_ENABLED`、`SAFETY_STOP`、`LIMIT_VIOLATION`、`TIMEOUT`、`DRIVER_FAULT` 和 `INTERNAL`。`reason` 用于人类可读信息，机器逻辑应依赖稳定的错误码。

## 安全要求

- 所有运动 RPC 必须带 `session_id` 和唯一 `command_id`。
- 网关验证会话、权限、模式、限位、速度和心跳，不信任 UI 参数。
- `Stop` 和急停相关命令必须具有更高优先级，不能排在普通轨迹后面。
- 连接断开、会话过期或心跳超时后，网关必须拒绝新的运动命令。
