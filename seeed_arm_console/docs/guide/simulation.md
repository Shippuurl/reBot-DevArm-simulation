# 仿真工作站

本页带你从空环境跑通 B601-RS 的 Viewer、MuJoCo 网关和规划服务。命令均从
`seeed_arm_console/` 仓库根目录执行。

## 环境准备

需要：

- Linux（推荐 Ubuntu 24.04）；
- Docker、Docker Compose v2；
- Rust 1.95+；
- Python 3.10+。

先创建一个用于冒烟测试和规划服务的虚拟环境：

```bash
python3 -m venv .venv-planning
source .venv-planning/bin/activate
python -m pip install --upgrade pip
python -m pip install grpcio grpcio-tools
```

## 启动 Viewer

终端 A：

```bash
cargo run --features embedded-viewer --bin rebot_sim_viewer
```

Viewer 监听 `127.0.0.1:9876`，加载 B601-RS 的 25 个网格，并订阅网关的关节、TF、
轨迹、接触和深度点云。控制台可以发送使能、停止、暂停、恢复和执行倍率命令。

长时间观察时使用有界历史配置：

```bash
scripts/run_viewer_debug.sh
```

该脚本将历史上限设为 `64MiB`、遥测频率设为 `30 Hz`。通过
`RERUN_HISTORY_LIMIT` 和 `RERUN_TELEMETRY_RATE_HZ` 可以覆盖；频率范围为 1–200 Hz。

## 启动 MuJoCo 网关

终端 B：

```bash
docker compose -f docker-compose.gateway.yml \
  -f docker-compose.mujoco.yml up -d --build
```

Compose 构建 MuJoCo 3.12.0 网关，加载
`assets/robot/b601_rs/mujoco/scene.xml`，并把端口发布到主机回环地址：

| 端口 | 服务 |
| ---: | --- |
| 50051 | ArmGateway gRPC |
| 50052 | JSON 诊断与 Rerun 转发 |

网关默认产生 `overhead_depth` 深度点云（32×24，最多 768 点）。只做关节和 TF 回归时，
可关闭传感器并重建容器：

```bash
MUJOCO_ENABLE_DEPTH_SENSOR=0 docker compose \
  -f docker-compose.gateway.yml -f docker-compose.mujoco.yml up -d --build
```

只使用 gRPC 时，将 `ARM_CONSOLE_ENABLE_JSON=0` 传给 Compose 环境即可关闭 50052。

## 验证网关

终端 C：

```bash
scripts/run_gateway_grpc_smoke.sh
```

成功输出包含 `gateway_grpc=OK`、`dof=6`、`tf=10` 和时间戳检查结果。冒烟测试覆盖握手、
使能、Jog、轨迹预检、短轨迹执行、暂停/恢复、倍率边界、停止、故障复位和遥测回读。

轨迹调用遵循固定顺序：

1. `Handshake` 获取会话；
2. `Enable`；
3. `dry_run=true` 预检轨迹；
4. 预检通过后以 `dry_run=false` 执行；
5. 从遥测确认状态，再按需 `Stop` 和 `ResetFault`。

## 启动规划服务

在规划服务所在终端激活环境并安装算法依赖：

```bash
source .venv-planning/bin/activate
python -m pip install -r requirements-planning.txt
scripts/run_planner_server.sh
```

服务监听 `127.0.0.1:50053`。终端 D 运行 Python SDK 冒烟测试：

```bash
scripts/run_planner_smoke.sh
```

验证完整闭环（规划 → 网关预检 → MuJoCo 执行 → 遥测）时运行：

```bash
scripts/run_planner_gateway_smoke.sh
```

规划服务输出候选结果和碰撞摘要；网关在预检和执行阶段检查时间序列、有限值、关节限位
和速度，确认输入满足执行边界后再入队。

## ROS 2 编排（可选）

ROS 2 Jazzy 只负责启动和编排 `ArmPlanner`：

```bash
env -u AMENT_PREFIX_PATH -u COLCON_PREFIX_PATH bash -lc '
  source /opt/ros/jazzy/setup.bash
  cd ros2_ws
  colcon build --symlink-install --packages-select pinocchio_planner
  source install/setup.bash
  ros2 launch pinocchio_planner planner.launch.py
'
```

服务接口和 SDK 调用方式不变。

## JSON 与离线记录

内嵌 Viewer 直接订阅 gRPC。调试 JSON 或生成离线 `.rrd` 时按需运行：

```bash
# 读取 50052 并转发到 Rerun
cargo run --features rerun-recording --bin mujoco_rerun_bridge

# 在线发送到 Viewer；去掉 RERUN_GRPC_URL 则生成本地 .rrd
export RERUN_GRPC_URL='rerun+http://127.0.0.1:9876/proxy'
cargo run --features rerun-recording --bin rerun_sample
```

一个 Viewer 实例只连接一个实时 recording 来源。

## 停止和排查

停止网关：

```bash
docker compose -f docker-compose.gateway.yml \
  -f docker-compose.mujoco.yml down
```

Viewer 没有数据时，依次检查：

1. `scripts/run_gateway_grpc_smoke.sh` 是否通过；
2. Viewer 是否仍监听 `9876`；
3. 规划场景是否需要 `50053`，以及规划服务是否在运行；
4. 网关日志：

   ```bash
   docker compose -f docker-compose.gateway.yml \
     -f docker-compose.mujoco.yml logs --tail=100
   ```

跨主机或真实设备部署见[安全部署](/deployment/security)。
