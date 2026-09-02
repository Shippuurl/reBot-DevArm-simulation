# Headless C++ 网关示例

`arm_console_mock_gateway` 是一个不依赖 X11 或图形驱动的 MuJoCo/Mock 数据源。启用 gRPC 构建时，`ArmGateway` 在 50051 提供正式控制、轨迹执行、故障复位与遥测接口；换行 JSON 仅在 50052 作为可关闭的 legacy adapter。

网关使用 `SimulationDriver` 抽象。默认构建运行确定性的 mock 驱动；启用 MuJoCo 后传入 XML 模型路径即可读取 `mjData` 中的关节和刚体状态。gRPC 和 legacy JSON 共享同一个驱动实例和互斥锁。`ExecuteTrajectory` 在仿真中采用受限的运动学采样适配：最多 2000 点、首点时间必须为 0、时间单调、6 关节有限值、关节限位和 2 rad/s 速度上限均在驱动侧再次检查；MuJoCo 每周期执行 `mj_forward`，因此 TF 和接触摘要与执行状态同步。

## 构建

推荐直接使用项目提供的 ROS 2 Jazzy + MuJoCo 镜像：

```bash
cd /path/to/seeed_arm_console
docker compose -f docker-compose.gateway.yml \
  -f docker-compose.mujoco.yml up -d --build
```

Compose 会在容器内完成 MuJoCo、Protobuf/gRPC 和 CMake 构建并保持网关持续运行。需要停止时执行 `docker compose -f docker-compose.gateway.yml -f docker-compose.mujoco.yml down`。

如果希望在 Ubuntu 主机生成 Mock 原生可执行文件：

```bash
cd cpp/mock_gateway
cmake -S . -B build
cmake --build build -j2
```

启用 gRPC 原生构建前安装开发包：

```bash
sudo apt-get update
sudo apt-get install -y libgrpc++-dev libprotobuf-dev protobuf-compiler-grpc
```

启用已安装的 MuJoCo SDK：

```bash
cmake -S . -B build-mujoco \
  -DARM_CONSOLE_WITH_MUJOCO=ON \
  -DARM_CONSOLE_WITH_GRPC=ON \
  -DMUJOCO_ROOT=/opt/mujoco
cmake --build build-mujoco -j2
```

Linux/容器中使用同样的 CMake 命令即可；gRPC 构建需要 `libgrpc++-dev`、`libprotobuf-dev` 和 `protobuf-compiler-grpc`。

项目还提供基于 ROS 2 Jazzy Desktop Full 的 MuJoCo 派生镜像配置。首次构建并启动：

```bash
cd /path/to/seeed_arm_console
docker compose -f docker-compose.gateway.yml \
  -f docker-compose.mujoco.yml up -d --build
```

该配置下载 MuJoCo 3.12.0、安装 C++ gRPC/Protobuf 开发包、加载 `assets/robot/b601_rs/mujoco/scene.xml`，并暴露 `127.0.0.1:50051`（gRPC）和 `127.0.0.1:50052`（JSON），不启动任何图形窗口。

从项目根目录验证容器内的真实 MuJoCo 数据和控制闭环：

```bash
python3 scripts/gateway_grpc_smoke.py
python3 scripts/verify_gateway.py
```

预期输出：

```text
gateway_grpc=OK source=mujoco dof=6 sequence=1 tf=10
telemetry=OK source=mujoco tf=10 sequence=...
control=OK enable accepted, jog accepted
```

如果 Docker 构建环境无法访问 GitHub，可先将 MuJoCo 压缩包放到可访问的镜像地址，并覆盖 `MUJOCO_URL`；压缩包仍会使用固定 SHA-256 校验，校验值变化时需同步更新 Compose 参数。

## 运行

```bash
./build/arm_console_mock_gateway 50051
```

使用 MuJoCo 驱动时，第二个参数传入 XML 模型路径；也可以设置 `ARM_CONSOLE_MODEL`：

```bash
./build-mujoco/arm_console_mock_gateway 50051 \
  /path/to/seeed_arm_console/assets/robot/b601_rs/mujoco/scene.xml
```

Rust Viewer 默认连接 `ARM_GATEWAY_GRPC_URL=http://127.0.0.1:50051`，启动后执行握手并订阅遥测；`mujoco_rerun_bridge` 默认连接 legacy JSON `127.0.0.1:50052`。

MuJoCo 驱动默认启用 `overhead_depth` 深度相机：它不创建 OpenGL 上下文，而是对场景
几何执行 32×24（768 条）`mj_ray` 射线，输出世界坐标点云。设置
`MUJOCO_ENABLE_DEPTH_SENSOR=0` 可关闭该采样，适合只回归关节/TF 的低带宽测试。

主机运行默认绑定回环地址。容器内请设置 `ARM_CONSOLE_BIND_ADDRESS=0.0.0.0` 和 `ARM_CONSOLE_GRPC_BIND_ADDRESS=0.0.0.0`，并只通过 Docker 的 `127.0.0.1` 发布端口。

## 帧格式

每行是一个 JSON 对象，单位为弧度、弧度/秒和纳秒：

```json
{"sequence":1,"timestamp_ns":20000000,"sim_time_ns":20000000,"wall_time_ns":1720000000000000000,"source":"mujoco","quality":"valid","joint_position_rad":[0,0,0,0,0,0],"joint_velocity_rad_s":[0,0,0,0,0,0],"contacts":[],"point_clouds":[{"sensor":"overhead_depth","positions":[[0.1,0.2,0.0]],"colors_rgba":[]}]}
```

每帧还包含 `tf`（10 个链路变换，含左右夹爪）、各一个规划点和实际点，以及 MuJoCo
`overhead_depth` 的 32×24 射线深度点云（命中点数量最多 768）。这里的 TF 是确定性
运动学占位值，不代表真实模型的动力学结果；深度点云来自真实 MuJoCo 几何射线相交。

## 控制联调

gRPC 客户端在 `ExecuteTrajectoryCommand` 中提交 `TrajectoryPoint` 列表。先将 `dry_run=true` 的候选轨迹送入网关做安全校验，再提交 `dry_run=false` 执行。执行中的关节位置和速度会出现在 `SubscribeTelemetry` 的 `actual_trajectory` 中，最近一次候选轨迹出现在 `planned_trajectory`。`Stop` 会立即清除执行队列；随后可发送 `ResetFault` 清除仿真停止状态。

每次 `Handshake` 都会返回独立的、短期有效的 `session_id`。客户端必须把它带到后续
控制和遥测请求中；一个客户端重新握手不会使其他客户端的会话失效。仿真网关会在
无活动超过 1 小时后清理会话，并将活跃会话总数限制为 1024。该机制只解决受信任仿真
网络中的会话隔离；生产部署仍需要把会话绑定到 TLS 客户端身份、授权策略和审计记录。

轨迹执行期间可以发送以下控制命令：

```text
SpeedScale(scale=1.5)  # 执行倍率，范围 [0.1, 2.0]
Pause()                # 冻结轨迹与 MuJoCo 仿真时间
Resume()               # 继续执行并丢弃暂停期间的时间 backlog
```

`Pause` 会保持当前位置、将速度置零，并拒绝 Jog 与新轨迹；遥测流继续输出。`Resume` 不会让仿真通过 `mj_step` 追赶暂停期间的 wall-clock 时间。倍率只改变轨迹进度，所有关节限位、有限值、时间戳单调性和 2 rad/s 速度上限仍由网关执行；越界倍率被拒绝。上述语义属于仿真适配层，真实设备需要单独实现伺服暂停、急停和 watchdog。

`CommandHeader.client_timestamp_ns` 的非零值会经过网关新鲜度检查：允许当前时间前 5 秒至后 1 秒，过期或过度超前的命令返回 `REJECTED`。本地 smoke/legacy 兼容命令可使用 0，但真实控制端必须提供同步的 Unix 纳秒时间戳。

TCP 客户端可以向同一连接发送换行分隔的控制 JSON，网关会返回确认帧：

```json
{"type":"enable","enabled":true}
{"type":"jog","joint_index":0,"step_rad":0.05}
{"type":"stop"}
```

确认帧为 `{"type":"ack","status":"accepted|rejected","reason":"..."}`。该接口仅用于兼容诊断脚本；新客户端必须使用 `arm_console.proto` 定义的会话、控制和遥测接口。

真实设备仍需独立硬件急停、限位、速度检查和 watchdog；Rerun、Docker 和规划服务不得成为安全闭环的必要条件。
