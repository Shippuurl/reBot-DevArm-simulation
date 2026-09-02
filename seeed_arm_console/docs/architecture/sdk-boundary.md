# SDK 与协议边界

SDK 是业务程序的公共入口。三个 SDK 都实现同一份 `arm.console.v1` gRPC 协议，并把
protobuf 消息转换为语言原生类型；Planner、Gateway、MuJoCo 和 Rerun 的内部实现留在
平台服务中。

## 服务地址

| 服务 | 默认地址 | 能力 |
| --- | --- | --- |
| `ArmGateway` | `127.0.0.1:50051` | 握手、控制、轨迹执行、遥测 |
| `ArmPlanner` | `127.0.0.1:50053` | IK、轨迹候选、规划元数据 |
| Rerun 接收端 | `127.0.0.1:9876` | Viewer 显示、记录和回放 |

50052 是面向验证脚本和转发器的本机 JSON 诊断端口；业务 SDK 统一连接 50051。

## 调用顺序

```text
连接 Gateway → Handshake → Enable
目标位姿   → Planner → 轨迹候选 → Gateway 预检 → Gateway 执行
                                              └─ TelemetryFrame
```

`Handshake` 返回 `ConnectionInfo` 和独立 `session_id`。后续控制、遥测请求由 SDK 自动
携带会话；控制命令同时带 `command_id` 和 Unix 纳秒时间戳。网关接受当前时间前 5 秒至
后 1 秒的非零时间戳。

## 数据类型

| 数据 | 来源 | SDK 类型 |
| --- | --- | --- |
| 会话和连接信息 | `ArmGateway` | `ConnectionInfo` |
| 控制确认 | `ArmGateway` | `CommandAck` |
| 关节、TF、轨迹、接触、图像、点云 | `ArmGateway` | `TelemetryFrame` |
| IK 结果和轨迹候选 | `ArmPlanner` | `IKResult`、`TrajectoryPlanResult` |

单位固定为米、弧度、弧度/秒和 Unix 纳秒；四元数排列为 `x,y,z,w`。点云位置按每组三个
浮点数解释，图像携带编码后的字节。

## 各语言入口

- **Python**：`sdk/python`，同步客户端；适合脚本和业务服务。
- **C++**：`sdk/cpp`，CMake target 和 gRPC 客户端。
- **Rust**：`sdk/rust`，异步 tonic 客户端。
- **ROS 2**：`ros2_ws/src/pinocchio_planner`，用于启动和编排服务。

业务代码引用 SDK 的公开头文件、模块和类型。Rerun 实体树、模型清单和 recording 由
Viewer 数据模型单独维护，控制协议保持聚焦于服务数据。

## 协议演进

协议源文件是 `protocol/arm_console.proto`。新增字段保持兼容并保留已有编号；删除字段、
改变单位或改变方法语义时，发布新的协议版本和对应 SDK 主版本。排障时记录协议版本、
请求 ID、命令 ID 和服务端 `reason`。

跨主机的 TLS、凭据、网络和设备安全配置集中在[安全部署](/deployment/security)。

语言指南：[Python SDK](/sdk/python) · [C++ SDK](/sdk/cpp) · [Rust SDK](/sdk/rust)
