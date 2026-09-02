# 系统架构

系统分成三层：业务程序调用 SDK，平台服务完成规划和执行，Viewer 负责观察。SDK、Planner
和 Gateway 通过 `arm.console.v1` 通信，任一服务都可以单独重启。

## 系统上下文

![系统上下文](/diagrams/system-context.svg)

业务程序把目标位姿交给 `ArmPlanner`，收到候选轨迹后调用 `ArmGateway`。网关连接
MuJoCo（或设备驱动），把执行状态以遥测流发布给 Viewer。操作员在 Viewer 中查看模型、
轨迹和诊断；控制面板命令通过网关 SDK 发送。

## 服务

| 服务 | 职责 | 默认入口 |
| --- | --- | --- |
| `ArmPlanner` | FK、雅可比、IK、轨迹采样、碰撞摘要 | `127.0.0.1:50053` |
| `ArmGateway` | 会话、命令校验、执行、遥测分发 | `127.0.0.1:50051` |
| `rebot_sim_viewer` | Rerun 3D、时间线、记录、回放和控制面板 | `127.0.0.1:9876` |

SDK 位于 `sdk/python`、`sdk/cpp` 和 `sdk/rust`。ROS 2 包位于
`ros2_ws/src/pinocchio_planner`，负责启动和编排规划服务。`cpp/mock_gateway` 提供
Mock 与 MuJoCo 两种 `ArmGateway` 驱动。

50052 是逐行 JSON 诊断接口，供验证脚本和 Rerun 转发器使用；业务控制走 50051。

## 请求链路

```text
业务程序
  ├─ SDK ──► ArmPlanner ──► IK / 轨迹候选
  └─ SDK ──► ArmGateway ──► MuJoCo 或设备驱动
                             └─ TelemetryFrame ──► Rerun Viewer
```

一次轨迹执行包含四个动作：

1. 规划服务生成带时间、速度和碰撞摘要的候选点；
2. 业务程序确认碰撞摘要后，把候选点提交给网关做预检；
3. 网关复核会话、时间戳、时序、有限值、限位和速度后入队；碰撞结果沿用规划服务的
   检查结果；
4. 驱动执行并持续发布关节、TF、轨迹、接触和传感器帧。

## 代码边界

- `protocol/arm_console.proto` 是跨语言协议源文件。
- SDK 在库内部处理 protobuf，向业务程序返回语言原生类型。
- Planner 只产生候选结果，Gateway 是执行前的最终输入校验点。
- Planner 使用 Coal 提供碰撞检查和距离摘要；Gateway 不重复计算碰撞几何。
- Viewer 读遥测并写 Rerun recording；模型清单和实体路径由 Viewer 管理。
- 真机驱动在 Gateway 后接入；硬件急停、限位、watchdog 和碰撞保护由设备驱动或独立安全
  链路负责。

字段、单位和版本规则见[SDK 与协议边界](/architecture/sdk-boundary)；算法实现见
[规划与仿真](/backend/simulation)。
