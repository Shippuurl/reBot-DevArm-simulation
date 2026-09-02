# ArmGateway 仿真实现

`arm_console_mock_gateway` 是无图形界面的 `ArmGateway` 实现，内置确定性的 Mock 驱动和
MuJoCo 驱动。它为 SDK、Viewer 和自动化测试提供控制、轨迹执行、故障复位与遥测。

## 端口

| 端口 | 协议 | 用途 |
| ---: | --- | --- |
| 50051 | gRPC | `ArmGateway` 控制与遥测 |
| 50052 | 换行 JSON | 本机诊断和 Rerun 转发 |

## 构建

### Compose

在仓库根目录运行：

```bash
docker compose -f docker-compose.gateway.yml \
  -f docker-compose.mujoco.yml up -d --build
```

Compose 构建 MuJoCo 3.12.0、Protobuf/gRPC 和网关，加载
`assets/robot/b601_rs/mujoco/scene.xml`。端口发布到主机回环地址，服务以无图形界面模式运行。

停止服务：

```bash
docker compose -f docker-compose.gateway.yml \
  -f docker-compose.mujoco.yml down
```

### Ubuntu 原生构建

```bash
sudo apt-get update
sudo apt-get install -y libgrpc++-dev libprotobuf-dev protobuf-compiler-grpc
cmake -S cpp/mock_gateway -B cpp/mock_gateway/build \
  -DARM_CONSOLE_WITH_GRPC=ON
cmake --build cpp/mock_gateway/build -j2
```

启用本机 MuJoCo 安装：

```bash
cmake -S cpp/mock_gateway -B cpp/mock_gateway/build-mujoco \
  -DARM_CONSOLE_WITH_GRPC=ON \
  -DARM_CONSOLE_WITH_MUJOCO=ON \
  -DMUJOCO_ROOT=/opt/mujoco
cmake --build cpp/mock_gateway/build-mujoco -j2
```

## 运行

Mock 驱动：

```bash
cpp/mock_gateway/build/arm_console_mock_gateway 50051
```

MuJoCo 驱动需要传入场景文件：

```bash
cpp/mock_gateway/build-mujoco/arm_console_mock_gateway 50051 \
  assets/robot/b601_rs/mujoco/scene.xml
```

容器内监听所有接口时设置 `ARM_CONSOLE_BIND_ADDRESS=0.0.0.0` 和
`ARM_CONSOLE_GRPC_BIND_ADDRESS=0.0.0.0`。用 `ARM_CONSOLE_ENABLE_JSON=0` 可关闭 50052；
用 `MUJOCO_ENABLE_DEPTH_SENSOR=0` 可关闭 32×24 的 `overhead_depth` 射线采样。

## 验证

```bash
scripts/run_gateway_grpc_smoke.sh
python3 scripts/verify_gateway.py
```

gRPC 冒烟测试覆盖握手、会话、使能、Jog、轨迹预检/执行、暂停/恢复、倍率边界、停止、
故障复位和遥测；JSON 脚本检查 50052 的数据与控制。

## 执行与遥测

轨迹最多 2000 点，首点时间为 0，时间单调，每点包含 6 个有限关节值，并遵守驱动模型的
关节限位；点间速度上限为 2 rad/s。轨迹先以 `dry_run=true` 预检，再以 `false` 正式入队。`Pause` 冻结轨迹
时间，`Resume` 继续，`SpeedScale` 范围为 `[0.1, 2.0]`。

遥测包含 6 个关节、10 条 TF、规划/实际轨迹、接触摘要和深度点云；图像和点云按协议预算
处理，超限图像丢弃、点云降采样。每次握手返回独立 `session_id`，客户端命令自动带上会话
和时间戳。

## JSON 示例

50052 接收换行 JSON：

```json
{"type":"enable","enabled":true}
{"type":"jog","joint_index":0,"step_rad":0.05}
{"type":"stop"}
```

业务集成请使用 50051 gRPC SDK；跨主机的 TLS、凭据、网络隔离和设备安全见
[安全部署](../../docs/deployment/security.md)。
