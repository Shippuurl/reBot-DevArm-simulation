# OpenRAVE 与 MuJoCo

本项目不使用 MoveIt。OpenRAVE 负责规划/IK 相关能力，MuJoCo 负责动力学仿真和控制回放；两者都通过统一的驱动适配器接入控制网关。

## OpenRAVE headless

当前 OpenRAVE 容器启用了 C++ 核心、规划/IK 插件和 ODE 碰撞/物理插件，关闭 Qt/OSG GUI。它适合在 Docker 中执行可重复的规划任务：

```text
轨迹请求
  ▼
参数与限位校验
  ▼
OpenRAVE 规划 / IK
  ▼
轨迹摘要、碰撞结果和采样点
```

OpenRAVE 输出必须带模型版本、规划器名称、随机种子（如适用）、耗时和校验摘要。规划结果只代表“可执行候选”，仍要经过网关的二次限位和安全检查。

## MuJoCo

MuJoCo 适合作为 `SimulationDriver`：

- 读取同一套关节、坐标系和限位配置。
- 接受与真实驱动相同的命令消息。
- 按固定控制周期推进仿真并发布遥测。
- 将接触、约束、执行误差和传感器数据写入 Rerun。

仿真时间与墙钟时间必须明确区分。回放测试可以加速仿真，但 UI 仍应看到时间倍率和当前仿真时钟。

## Windows 联调入口

在真正的 MuJoCo 驱动接入前，可构建仓库中的 `cpp/mock_gateway`。该程序以 headless 方式输出 MuJoCo 风格的六关节快照，Windows 桌面端选择 TCP 网关即可观察表格、曲线和场景。它只验证数据通道，最终实现仍应切换到 `arm_console.proto` 定义的 gRPC 服务。

## MuJoCo 派生镜像

基础 Jazzy Desktop Full 镜像不内置 MuJoCo。项目提供 `cpp/mock_gateway/Dockerfile` 和 `docker-compose.mujoco.yml`：在基础镜像上安装 MuJoCo 3.12.0，使用 `-DARM_CONSOLE_WITH_MUJOCO=ON` 编译驱动，并加载 `assets/robot/b601_rs/mujoco/scene.xml`。该派生服务同样不设置 `DISPLAY`，只通过 TCP 输出状态。

当前网关已经用该派生镜像完成真实 MuJoCo 编译和运行验证。启动后可执行：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\verify-gateway.ps1 -ExpectedSource mujoco -CheckControl
```

验证内容包括首帧前向运动学、六关节数组、包含左右夹爪的十条 TF、实际轨迹以及 Jog 控制确认。MuJoCo 驱动初始化时会先调用 `mj_forward`，因此首帧的刚体位姿和单位四元数可直接用于可视化。

## 适配器接口

```text
DriverAdapter
  connect() / disconnect()
  enable() / stop()
  send_joint_command()
  send_trajectory()
  read_telemetry()
  diagnostics()
```

Mock、MuJoCo 和真实驱动实现同一接口，控制状态机不依赖具体实现。硬件专有寄存器、单位转换和重连策略只能出现在 adapter 内部。

## 验证场景

1. 固定初始姿态下的关节 Jog。
2. 接近关节限位时的拒绝行为。
3. 规划轨迹与实际轨迹的误差统计。
4. 仿真暂停、恢复、加速和回放。
5. 驱动断连、看门狗超时和急停后的状态转移。
