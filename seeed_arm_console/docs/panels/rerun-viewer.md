# Rerun Viewer

Viewer 把 B601-RS 的模型、运行状态和诊断放在同一条时间轴上。它包含 Rerun 原生视图和
一个轻量控制面板，控制命令通过 Rust SDK 发给 `ArmGateway`。

## 启动

```bash
cargo run --features embedded-viewer --bin rebot_sim_viewer
```

启动后 Viewer：

- 在 `127.0.0.1:9876` 提供 Rerun 接收端；
- 读取 `assets/robot/b601_rs/rerun/model.json`，加载 25 个静态网格；
- 连接 `127.0.0.1:50051`，订阅关节、TF、轨迹、接触和传感器帧。

网关地址可用 `ARM_GATEWAY_GRPC_URL` 覆盖，模型根目录可用 `ROBOT_MODEL_ROOT` 覆盖。

## 实体树

![Rerun 实体树](/diagrams/rerun-entity-tree.svg)

```text
world/
├─ robot/
│  ├─ model/manifest
│  ├─ frames/<link>/model
│  └─ joints/joint_<n>/{position,velocity}
├─ planning/{target,planned_trajectory}
├─ simulation/{actual_trajectory,contacts}
├─ sensors/<sensor>/{image,points}
└─ diagnostics/trajectory_error
```

静态网格和动态 TF 共用 `world` 坐标系。规划轨迹与实际轨迹使用不同实体，可以直接在
时间轴上比较；长度、角度和四元数分别遵循米、弧度和 `x,y,z,w` 约定。

## 查看和控制

| 区域 | 内容 |
| --- | --- |
| 3D 视图 | 25 个网格、10 条 TF、末端姿态 |
| 时间线/曲线 | 关节位置、速度、规划轨迹、实际轨迹和误差 |
| 诊断 | 接触数量、最小距离、最大法向力、帧质量 |
| 控制台 | 使能、停止、暂停、恢复和执行倍率（`0.1–2.0`） |

暂停时轨迹时间冻结，遥测继续发布；恢复后从当前位置继续。Viewer 重连后恢复观察状态，
控制命令由应用按需重新发送。

## 记录预算

实时 recording 默认保留 `256MiB` 历史，优先保留静态模型。长时间调试使用：

```bash
scripts/run_viewer_debug.sh
```

脚本默认把历史设为 `64MiB`、遥测设为 `30 Hz`。可用 `RERUN_HISTORY_LIMIT` 和
`RERUN_TELEMETRY_RATE_HZ` 覆盖，频率范围为 1–200 Hz。

## 其他记录方式

需要检查 JSON 数据或生成离线记录时，分别运行：

```bash
# JSON → Rerun 转发
cargo run --features rerun-recording --bin mujoco_rerun_bridge

# 在线发送到 Viewer；去掉变量则生成本地 .rrd
export RERUN_GRPC_URL='rerun+http://127.0.0.1:9876/proxy'
cargo run --features rerun-recording --bin rerun_sample
```

一个 Viewer 只连接一个实时数据源。记录器会限制图像和点云大小：图像最大 8 MiB、
4096×4096，点云最多 50,000 点。

跨主机部署的网络和设备安全配置见[安全部署](/deployment/security)。
