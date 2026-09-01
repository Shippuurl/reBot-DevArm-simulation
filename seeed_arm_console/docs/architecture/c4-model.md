# C4 模型

C4 用四个层级描述系统：Context、Container、Component 和 Code。本文只描述公开框架的职责与边界，具体硬件型号属于可替换适配器。

## Level 1：系统上下文

```text
                         ┌────────────────────┐
                         │ 操作员 / 开发者     │
                         └─────────┬──────────┘
                                   │ 控制、观察、回放
                                   ▼
┌──────────────┐  控制/遥测  ┌──────────────────────┐  规划/仿真  ┌─────────────────┐
│ Robot /      │◄───────────►│ Robot Console        │◄──────────►│ ROS 2 Jazzy     │
│ Simulator    │             │ desktop application  │            │ + OpenRAVE      │
└──────────────┘             └──────────┬───────────┘            │ + MuJoCo        │
                                        │ 记录流                    └─────────────────┘
                                        ▼
                              ┌──────────────────────┐
                              │ Rerun Viewer/Storage │
                              └──────────────────────┘
```

外部参与者：操作员发出控制请求；机器人或仿真器提供执行反馈；ROS 2 生态提供桥接和算法；Rerun 提供可观测性和回放。

## Level 2：Container

| 容器 | 运行位置 | 职责 | 主要接口 |
| --- | --- | --- | --- |
| `console-app` | Win11 原生 | egui 布局、控制面板、遥测快照、会话状态 | eframe/egui、gRPC/WebSocket |
| `control-protocol` | Rust crate | 命令、状态、错误和单位模型 | Rust 类型；未来生成 protobuf |
| `control-gateway` | Win11 或 Docker | 鉴权、状态机、限位、心跳、命令路由 | gRPC/WebSocket |
| `ros2-bridge` | ROS 2 Jazzy Docker | ROS topic/service/action 与协议互转 | ROS 2、协议消息 |
| `openrave` | Docker | 规划、IK、碰撞检查 | 规划适配器 |
| `mujoco-driver` | Docker/本机 | 动力学仿真和回放 | `DriverAdapter` |
| `rerun-logger` | 后端或独立进程 | 状态、图像、点云和事件记录 | Rerun SDK |
| `rerun-viewer` | Win11 | 三维、曲线和时间线查看 | Rerun 数据流 |

## Level 3：Component

### `console-app`

- `AppShell`：窗口、主题、中文字体、停靠布局。
- `ControlPanel`：连接、使能、停用和故障入口。
- `JogPanel`：关节/笛卡尔 Jog、步长和速度输入。
- `TelemetryPanel`：最新关节快照、TF 摘要和连接质量。
- `RerunPanel`：Viewer 会话、记录状态和打开入口。
- `UiState`：只保存 UI 状态，不保存驱动权威状态。

### `control-gateway`

- `SessionManager`：会话、心跳和权限。
- `SafetySupervisor`：限位、模式、超时、急停和停止优先级。
- `CommandRouter`：把协议命令路由到规划器或驱动。
- `TelemetryFanout`：有界广播和最新状态快照。
- `AuditRecorder`：结构化审计事件和 Rerun 事件。

### `ros2-bridge` 与适配器

- `TopicMapper`：关节状态、TF、图像、点云映射。
- `ServiceActionMapper`：连接、使能、停止和轨迹执行映射。
- `DriverAdapter`：Mock、MuJoCo 和真实硬件的统一抽象。

## Level 4：Code / crate 映射

```text
crates/
  console-app/       egui/eframe 桌面入口与面板
  control-protocol/  ArmMode、ArmStatus、JointState 等领域类型
  ros2-bridge/       ROS 2 适配层（逐步实现）
  rerun-logger/      Rerun 记录封装，feature = rerun-sdk
```

新增代码优先放入职责最小的 crate；UI 不直接依赖 ROS 2 消息类型，后端不依赖 egui 控件类型。

## 关键约束

1. 控制路径：UI → 网关 → 安全监督器 → 驱动。
2. 观察路径：驱动/ROS 2 → 遥测分发 → UI 与 Rerun。
3. Rerun 不得反向调用控制命令。
4. 所有运动请求均需会话、命令 ID、单位和时间戳。
