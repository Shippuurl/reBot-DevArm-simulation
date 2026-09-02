# 规划与仿真

规划服务负责“算出一条可行路径”，网关负责“按规则执行这条路径”。两者通过
`arm.console.v1` 连接，业务程序用 SDK 调用。

## ArmPlanner

启动入口是 `scripts/planner_grpc_server.py`，默认监听 `127.0.0.1:50053`。服务启动时
加载 B601-RS URDF、Pinocchio 模型和 Coal 碰撞几何。

### `SolveIK`

1. 用请求中的 seed、neutral、中位姿和确定性偏置生成最多 4 组初值；
2. 用 FK、雅可比和增量 QP 迭代求解关节位置；
3. 按请求检查位姿误差、关节限位和碰撞距离；
4. 返回关节值、碰撞摘要、求解器和耗时。

目标位姿使用 `world` 坐标系，位置单位为米。旋转四元数按 `x,y,z,w` 排列；全零表示
不约束姿态，非零值必须是单位四元数。`assembly_phase` 支持 `APPROACH`、`MATE` 和
`RETRACT`，允许接触的几何对通过 `allowed_collision_pairs` 传入。

### `PlanTrajectory`

服务先分别求解起点和终点，再用 cubic smoothstep 采样关节轨迹。每个采样点经过
ProxSuite 的关节盒和分段速度盒投影，并携带位置、速度和相对时间。默认采样率为 20 Hz，
最大关节速度为 1 rad/s，最大加速度为 2 rad/s²；启动参数可调整速度和加速度上限。

开启 `check_collisions` 后，服务会扫描轨迹点并在响应中给出最小距离、碰撞对和检查数量。
响应中的轨迹是候选结果，提交执行前交给网关做预检。

启动和验证：

```bash
scripts/run_planner_server.sh
scripts/run_planner_smoke.sh
```

## ArmGateway

`cpp/mock_gateway` 将 `SimulationDriver` 接到 gRPC 服务。Compose 默认使用 MuJoCo 驱动，
也可切换为确定性的 Mock 驱动。

### 当前仿真边界

MuJoCo 驱动当前把轨迹采样结果写入 `qpos/qvel`，再调用 `mj_forward` 更新姿态、接触和
传感器数据；尚未接入执行器、PD/力矩控制和经过校准的质量、惯量、摩擦参数。因此现阶段
验证范围是运动学、接触重算、协议和控制闭环，不代表 B601-RS 的额定动力学性能。

| 能力 | 说明 |
| --- | --- |
| `Handshake` | 返回协议版本、驱动来源、DOF 和会话 ID |
| `Command` | 使能、Jog、轨迹、暂停、恢复、倍率、停止、故障复位 |
| `SubscribeTelemetry` | 按请求频率发布状态帧，范围 1–200 Hz |

### 执行检查

网关收到轨迹后按以下顺序检查：

- 点数为 1–2000，首点时间为 0，时间单调；
- 每个点包含 6 个有限关节值，速度字段为空或包含 6 个有限值；
- 关节位置落在驱动模型的限位内（Mock 使用 B601-RS 固定限位）；
- 点间速度和显式速度均不超过 2 rad/s（仿真安全演示上限，不是设备额定速度）。

`dry_run=true` 表示预检，只做检查，不改变驱动状态；检查通过后以 `dry_run=false` 正式提交。
`Pause` 冻结轨迹时间但继续发遥测，`Resume` 从当前位置继续；`SpeedScale` 接受
`[0.1, 2.0]`，`Stop` 清空执行队列，`ResetFault` 恢复可继续仿真的停止状态。

碰撞几何由 Planner 使用 Coal 检查并随规划结果返回，Gateway 不重复计算。接入真实设备时，
设备驱动或独立安全链路负责硬件碰撞保护。

## 遥测与传感器

每帧包含序列号、仿真时间、墙钟时间、6 个关节和 10 条 TF，并可附带：

- 最近一次候选轨迹和当前执行轨迹；
- 最多 64 对接触几何、距离和法向力；
- 图像和点云传感器帧。

MuJoCo 的 `overhead_depth` 使用 32×24 条 `mj_ray` 射线，最多产生 768 个世界坐标点。
通过 `MUJOCO_ENABLE_DEPTH_SENSOR=0` 可关闭该传感器。记录器对图像和点云设有统一上限：
每帧最多 4 个载荷，图像不超过 8 MiB 或 4096×4096，点云最多 50,000 点，超限点云会
均匀降采样。

## 离线视觉策略管线

本项目只采用离线混合：RoboTwin 2.0（SAPIEN/PhysX）生成场景、视觉数据和专家轨迹，
MuJoCo 负责策略回放与控制边界验证。两套引擎不在运行时同步关节或接触状态。

后续以开源具身模型 LingBot-VLA 完成策略训练/推理接入，数据通过独立的
Observation/Action Adapter 转换为本项目的观测和动作格式：

```text
RoboTwin 数据 → Adapter → LingBot-VLA → ArmPlanner → ArmGateway 预检 → MuJoCo 回放
```

Rerun 只承担观察和抽样回放，不作为训练数据源。仿真回放指标稳定后，再进入真机数据采集，
用于 LingBot-VLA 的预训练或微调；真机安全门槛见[安全部署](/deployment/security)。

## Rerun 数据

内嵌 Viewer 直接订阅 50051。需要检查 JSON 数据或把数据转发到其他 Rerun 客户端时，
使用 50052：

```bash
python3 scripts/verify_gateway.py
cargo run --features rerun-recording --bin mujoco_rerun_bridge
```

业务控制和规划请使用 [Python SDK](/sdk/python)、[C++ SDK](/sdk/cpp) 或
[Rust SDK](/sdk/rust)。Viewer 的实体树和记录规则见[Rerun Viewer](/panels/rerun-viewer)。
