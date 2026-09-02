# 仿真工作站引导

本文是 Linux（Ubuntu 24.04）下运行 reBot-DevArm 仿真主线的独立入口。仿真由 MuJoCo 网关提供，Rerun 仅负责显示、记录和回放。

## 运行前检查

```bash
docker --version
python3 --version
cargo --version
```

若 Docker 报 `permission denied ... docker.sock`，将当前用户加入 docker 组后重新登录：

```bash
sudo usermod -aG docker "$USER"
```

## 启动 Viewer

终端一执行并保持运行：

```bash
cargo run --features embedded-viewer --bin rebot_sim_viewer
```

Viewer 的 Rerun gRPC 接收地址为 `127.0.0.1:9876`。

Viewer 启动时会把 `assets/robot/b601_rs/rerun/model.json` 中的模型网格直接加载到实时 `arm_gateway_grpc` recording，并让网格跟随 ArmGateway 的动态 TF。左侧状态显示 `模型: 已加载 25 个网格` 即表示加载成功；不再需要另行运行 `rerun_sample` 才能查看模型。在 Rerun 实体树展开 `robot/frames`，打开 3D 视图并使用“Frame all”即可定位机械臂。可通过 `ROBOT_MODEL_ROOT` 覆盖模型根目录。

Viewer 自带按字节计量的环形历史缓存：默认保留最近 `256MiB` 动态数据，旧遥测 chunk 达到预算后由 Rerun 自动淘汰，静态模型优先保留（只有静态数据本身超过预算时才会进一步淘汰）。调试时可启用更低频率和更小缓存：

```bash
RERUN_DEBUG_MODE=1 RERUN_HISTORY_LIMIT=64MiB RERUN_TELEMETRY_RATE_HZ=30 \
  cargo run --features embedded-viewer --bin rebot_sim_viewer
```

也可以使用仓库脚本启动，避免 zsh 将跨行环境变量误解析为命令：

```bash
scripts/run_viewer_debug.sh
```

该脚本默认使用 `64MiB`/`30 Hz`，仍可通过已有的 `RERUN_HISTORY_LIMIT` 和
`RERUN_TELEMETRY_RATE_HZ` 覆盖。一般不要设置 `RERUN_HISTORY_LIMIT=0`：这会关闭
接收端历史，并可能让 Viewer 的数据块在每轮内存回收中全部被清掉；需要“只看当前”
时应使用较小但非零的预算（建议不低于 `64MiB`，以容纳 B601-RS 静态模型）。

也可以只覆盖单项配置。`RERUN_HISTORY_LIMIT` 接受 `64MiB`、`256MB` 或 `25%` 等 Rerun 格式；`RERUN_TELEMETRY_RATE_HZ` 范围为 1–200。该机制限制的是 recording 历史，不是操作系统 allocator 的 RSS 立即归还，因此 RSS 可能在淘汰后保持高水位，但不会继续按历史数据无限增长。

要确认回收正在发生，可提高日志级别并观察 `Dropping the oldest log messages`：

```bash
RUST_LOG=re_grpc_server=info,re_viewer::app::logic=info \
  scripts/run_viewer_debug.sh
```

同时只保留一个 `rebot_sim_viewer` 进程；多个 Viewer 各自拥有独立的 9876 接收端和
recording，系统 RSS 会按进程数叠加。

## 启动 MuJoCo 网关

终端二执行：

```bash
docker compose -f docker-compose.gateway.yml \
  -f docker-compose.mujoco.yml up -d --build
```

网关的主链路是 gRPC：`127.0.0.1:50051` 提供 `ArmGateway`，加载 `assets/robot/b601_rs/mujoco/scene.xml`，不需要 X11 或 `DISPLAY`。为兼容 Rerun JSON 转发器，默认另开 `127.0.0.1:50052`；可用 `ARM_CONSOLE_ENABLE_JSON=0` 关闭。

MuJoCo 网关默认从 `overhead_depth` 固定相机生成 32×24（最多 768 点）的深度点云，随
gRPC/JSON 遥测发布到 `sensors/overhead_depth/points`。若只测试关节和 TF，可在启动容器
时设置 `MUJOCO_ENABLE_DEPTH_SENSOR=0`；该传感器是无图形上下文的 `mj_ray` 采样，不需要
额外安装 OpenGL。

## 验证遥测与 Jog

终端三执行：

```bash
scripts/run_gateway_grpc_smoke.sh
python3 scripts/verify_gateway.py  # legacy JSON adapter
```

gRPC 冒烟应看到 `gateway_grpc=OK`、`dof=6`、`tf=10`；JSON 兼容脚本应看到 `source=mujoco`、`tf=10` 以及 `control=OK`。

启动 Viewer 后，左侧“控制（仿真）”面板可直接发送使能、停止、Jog、暂停、恢复和执行倍率命令。倍率范围固定为 0.1–2.0，滑块调整后点击“应用”；暂停会冻结轨迹和 MuJoCo 仿真时间，恢复不会追赶暂停期间的墙钟时间。控制命令由 gRPC 客户端自动填入 Unix 纳秒时间戳，网关会拒绝超过 5 秒的旧命令和超前超过 1 秒的命令。

## 推送 Rerun 数据

如果使用独立的 Rerun Viewer（而非上面的内嵌 Viewer），MuJoCo 网关的实时关节遥测可通过
legacy JSON 转发器推送：

```bash
cargo run --features rerun-recording --bin mujoco_rerun_bridge
```

可用 `MUJOCO_GATEWAY_ADDR` 和 `RERUN_GRPC_URL` 覆盖默认地址。

内嵌 Viewer 已经直接通过 ArmGateway gRPC 订阅并记录遥测。使用内嵌 Viewer 时不要再
启动 `mujoco_rerun_bridge` 或 `rerun_sample` 作为同一网关的第二个实时数据源，否则会
创建重复 recording，增加带宽和缓存占用；这两个入口仅用于 legacy JSON 转发或离线样例。

需要验证外部 SDK 到 Viewer 的 gRPC 接收时，可运行 `rerun_sample`。它是独立样例/离线回放
工具，正常实时模型查看无需启动它：

```bash
export RERUN_GRPC_URL='rerun+http://127.0.0.1:9876/proxy'
cargo run --features rerun-recording --bin rerun_sample
```

在线模式结束时会输出 `recording=grpc ... frames=120`，不会在当前目录生成
`sample.rrd`；未设置 `RERUN_GRPC_URL` 时才会写入离线 `.rrd` 文件。

未设置该变量时，样例会写入本地 `.rrd` 文件，可用 Rerun 客户端离线回放。

## Pinocchio + ProxSuite 规划

本节命令用于平台内部规划服务开发。外部工程不需要安装或理解 Pinocchio、ProxSuite、
MuJoCo、URDF 或 Rerun；请使用 [Python SDK](/sdk/python)、[C++ SDK](/sdk/cpp) 或
[Rust SDK](/sdk/rust) 通过同一 gRPC 协议调用。

安装规划依赖：

```bash
python3 -m pip install -r requirements-planning.txt
```

推荐在仓库内使用隔离环境，项目脚本会自动优先选择 `.venv-planning/bin/python`：

```bash
python3 -m venv .venv-planning
source .venv-planning/bin/activate
python -m pip install -r requirements-planning.txt
```

若 `venv` 报告 `ensurepip is not available`，先安装当前 Python 对应的 venv 包（Ubuntu 24.04/Python 3.12）：

```bash
sudo apt-get update
sudo apt-get install -y python3.12-venv
```

启动 `ArmPlanner` gRPC 服务（默认 `127.0.0.1:50053`）：

```bash
scripts/run_planner_server.sh
```

服务启动时从 `protocol/arm_console.proto` 生成 Python stubs。`SolveIK` 已接入 Pinocchio/ProxSuite 和 Coal 几何检查；`PlanTrajectory` 会对起点/终点分别求 IK，并按 `max_rate_hz` 采样生成带速度字段的关节空间候选轨迹（默认 2 秒、20 Hz），每个采样点经过 ProxSuite 关节/分段速度约束投影。轨迹提交前仍必须由网关安全层复核。

需要写入 Rerun 时显式设置 `RERUN_GRPC_URL`；未设置时服务只计算并响应，避免 Viewer 未启动时阻塞规划服务：

```bash
export RERUN_GRPC_URL='rerun+http://127.0.0.1:9876/proxy'
```

保持服务在一个终端运行，并在另一个终端执行 gRPC 冒烟测试：

```bash
# 终端 A
scripts/run_planner_server.sh

# 终端 B
scripts/run_planner_smoke.sh
```

该冒烟脚本通过 `sdk/python` 调用规划服务；不再维护独立的 Pinocchio/ProxSuite
客户端实现。需要把规划结果交接给 MuJoCo 时运行：

```bash
scripts/run_planner_gateway_smoke.sh
```

预期输出包含 `planner=OK`、6 个 IK 关节和至少 2 个轨迹点。

也可以由 ROS 2 Jazzy 编排规划服务（该包不复制算法，仍调用同一个无 ROS 依赖的 gRPC 服务）：

```bash
env -u AMENT_PREFIX_PATH -u COLCON_PREFIX_PATH bash -lc '
  source /opt/ros/jazzy/setup.bash
  cd ros2_ws
  colcon build --symlink-install --packages-select pinocchio_planner
  source install/setup.bash
  ros2 launch pinocchio_planner planner.launch.py
'
```

`ros2_ws/src/pinocchio_planner/config/planner.yaml` 固化了 0.02 m 规划和 0.001 m 装配余量；运行时监控底线为 0.01 m。启动时会优先使用仓库 `.venv-planning/bin/python`。

## 停止与排查

```bash
docker compose -f docker-compose.gateway.yml \
  -f docker-compose.mujoco.yml down
```

查看网关日志：

```bash
docker compose -f docker-compose.gateway.yml -f docker-compose.mujoco.yml logs --tail=100
```

若 Viewer 无数据，确认 Viewer 仍监听 9876、`RERUN_GRPC_URL` 使用英文引号，并检查网关验证脚本是否报告 `source=mujoco`。
