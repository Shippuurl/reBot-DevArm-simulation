# 4+1 视图模型

4+1 视图从不同角度描述同一套架构：逻辑视图、开发视图、进程视图、物理视图，以及贯穿各视图的场景视图。

## 1. 逻辑视图

系统按领域职责分为五个逻辑子系统：

| 子系统 | 责任 | 不负责什么 |
| --- | --- | --- |
| 操作员 UI | 展示状态、采集输入、确认危险操作 | 不决定安全状态，不直接操作驱动 |
| 控制协议 | 定义命令、状态、错误、单位和版本 | 不绑定 TCP、ROS 2 或具体品牌 |
| 安全监督器 | 会话、模式、限位、看门狗、急停和审计 | 不做三维渲染 |
| 执行适配器 | 将通用命令转换为 Mock/MuJoCo/硬件调用 | 不修改上层协议语义 |
| 记录与观察 | Rerun 时间线、曲线、图像和点云 | 不发送运动命令 |

逻辑依赖方向固定为“UI 依赖协议，网关依赖协议，适配器实现协议，记录层订阅状态”。

## 2. 开发视图

```text
Rust workspace
├── crates/console-app
│   ├── app shell / tabs / panels
│   └── font / theme / local UI state
├── crates/control-protocol
│   ├── ArmMode / ArmStatus / JointState
│   └── command and error types
├── crates/ros2-bridge
│   └── ROS 2 message and service mapping
└── crates/rerun-logger
    └── optional Rerun SDK integration
```

基础类型放在 `control-protocol`，避免 UI 与后端各自定义一份状态结构。新 crate 需要明确输入、输出和测试边界，并保持离线 `cargo check --workspace --offline` 可运行。

## 3. 进程视图

```text
Win11
  console-app (60 FPS UI)
       │ gRPC/WebSocket：命令 + 最新遥测
       ▼
control-gateway (独立任务/进程)
       │ ROS 2 DDS 或本地 IPC
       ▼
Docker：ros2-bridge / OpenRAVE / MuJoCo
       │ 50–200 Hz 状态，事件级故障与命令
       ├────────► rerun-logger ───────► Rerun Viewer
       └────────► driver adapter
```

控制网关和记录器可以独立于 UI 重启。UI 断开时，网关按看门狗策略处理；Rerun Viewer 关闭时，不影响运动执行。

## 4. 物理视图

| 节点 | 部署 | 网络/设备 |
| --- | --- | --- |
| Windows 主机 | `console-app`、Rerun Viewer、开发工具 | localhost、Docker Desktop 虚拟网络 |
| ROS 2 Jazzy 容器 | `ros2-bridge`、算法和仿真 | `/work` 挂载、ROS_DOMAIN_ID、必要端口 |
| OpenRAVE 容器 | headless 规划和 ODE | 与桥接容器共享模型/配置 |
| 真实机械臂 | 外部设备（可选） | 厂商驱动专用网络，禁止暴露到公网 |

容器重启策略、卷挂载和端口映射应记录在部署脚本中。Windows UI、Rerun Viewer 和网关数据链路均不需要 X11 转发。

## 5. 场景视图（+1）

### 场景 A：连接并使能

1. UI 发起握手，网关校验版本并分配 `session_id`。
2. 网关订阅 ROS 2/驱动诊断，确认急停、限位和故障状态。
3. 操作员确认后请求使能，网关返回 `command_id` 和结果。
4. UI 与 Rerun 同时记录状态变更事件。

### 场景 B：Jog

1. UI 生成带单位、步长、速度和命令 ID 的 Jog 请求。
2. 安全监督器检查会话、模式、限位和看门狗。
3. 适配器执行一次点动并回传实际关节状态。
4. UI 读取最新快照，Rerun 记录规划/实际状态。

### 场景 C：轨迹执行

轨迹先规划和校验，再由操作员确认执行。执行期间持续比较规划与实际轨迹；超时、碰撞、限位或驱动故障都转入停止/故障状态。

### 场景 D：连接失联

心跳超时后，网关拒绝新命令并执行配置好的停止策略。UI 显示连接质量和最后一帧时间，重连必须创建新会话，不能复用旧执行令牌。
