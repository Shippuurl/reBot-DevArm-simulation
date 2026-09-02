# Python SDK 接入指南

`rebot-arm-sdk` 是业务应用调用 `ArmGateway` 和 `ArmPlanner` 的 Python 客户端。它把
gRPC 协议转换成同步方法和 dataclass，适合脚本、测试和业务服务。安装本包后，业务代码
直接连接服务地址即可。

## 安装

环境要求：Python 3.10+。安装包会带上协议生成代码：

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install /path/to/seeed_arm_console/sdk/python
```

运行时依赖为 `grpcio` 和 `protobuf`。

## 最短示例

下面的程序连接本机仿真网关，完成握手、使能、轨迹预检、执行和遥测读取：

```python
from rebot_sdk import ArmGatewayClient, TrajectoryPoint

with ArmGatewayClient("127.0.0.1:50051", client_name="pick-cell-controller") as gateway:
    info = gateway.connect()
    print(f"source={info.source} dof={info.dof} session={info.session_id}")

    if not gateway.enable().accepted:
        raise RuntimeError("enable rejected")

    points = [
        TrajectoryPoint(0, (0.0,) * info.dof),
        TrajectoryPoint(2_000_000_000, (0.2, 0.0, 0.0, 0.0, 0.0, 0.0)),
    ]
    check = gateway.execute_trajectory(points, dry_run=True)
    if not check.accepted:
        raise RuntimeError(f"pre-check rejected: {check.reason}")

    gateway.execute_trajectory(points)
    for frame in gateway.subscribe_telemetry(max_rate_hz=20):
        print(frame.sequence, frame.joint_position_rad)
        break
    gateway.stop()
```

`dry_run=True` 表示预检：只运行网关的轨迹检查，不入队。确认 `accepted` 后再提交正式
执行。`CommandAck` 表示请求是否被接受，执行进度以 `TelemetryFrame` 为准。

## 网关 API

| 方法 | 作用 |
| --- | --- |
| `connect()` | 握手并返回 `ConnectionInfo` |
| `enable(enabled=True)` | 使能或去使能 |
| `jog(joint_index, step_rad)` | 单关节步进 |
| `execute_trajectory(points, dry_run=False)` | 预检或提交轨迹 |
| `pause()` / `resume()` | 暂停或恢复当前轨迹 |
| `speed_scale(scale)` | 调整执行倍率（`0.1–2.0`） |
| `stop(emergency=False)` | 停止当前执行队列 |
| `reset_fault()` | 清除可恢复停止状态 |
| `subscribe_telemetry(max_rate_hz=50)` | 订阅状态流（1–200 Hz） |

SDK 类型包含 `ConnectionInfo`、`CommandAck`、`TelemetryFrame`、`TrajectoryPoint`、
`Transform`、`Contact`、`ImageFrame` 和 `PointCloud`。长度用米，角度用弧度，速度用
弧度/秒，时间用 Unix 纳秒，四元数按 `x,y,z,w` 排列。

## 会话与时间戳

握手后 SDK 保存网关返回的 `session_id`，后续控制和遥测请求自动带上它。每个网关客户端
获得独立会话，`client_name` 用于日志和排障标识。

控制命令默认携带当前 Unix 纳秒时间戳。网关接受当前时间前 5 秒至后 1 秒的非零时间戳，
超出窗口会返回 `REJECTED`。

## 规划

规划服务返回候选结果，不直接执行：

```python
from rebot_sdk import ArmPlannerClient, PoseTarget

with ArmPlannerClient("127.0.0.1:50053") as planner:
    ik = planner.solve_ik(
        PoseTarget((0.25, 0.0, 0.30)),
        check_collisions=True,
        minimum_distance_threshold_m=0.02,
    )
    if not ik.success:
        raise RuntimeError(ik.reason)

    plan = planner.plan_trajectory(
        PoseTarget((0.25, 0.0, 0.30)),
        PoseTarget((0.20, 0.05, 0.32)),
        check_collisions=True,
        minimum_distance_threshold_m=0.02,
    )
    if not plan.success:
        raise RuntimeError(plan.reason)
    print(f"points={len(plan.points)} solver={plan.metadata.solver}")
```

`PoseTarget` 默认使用 `world` 坐标系；全零四元数表示不约束姿态，非零四元数需要归一化。
装配任务可通过 `assembly_phase="APPROACH" | "MATE" | "RETRACT"` 和
`allowed_collision_pairs` 调整碰撞规则。轨迹交给网关预检后再执行。

## 错误与重连

RPC 错误统一抛出 `RebotRpcError`，原始 gRPC 状态码在 `.code`：

```python
from rebot_sdk import ArmGatewayClient, RebotRpcError

try:
    with ArmGatewayClient("127.0.0.1:50051") as gateway:
        gateway.enable()
except RebotRpcError as exc:
    print(exc.code, exc)
```

`subscribe_telemetry()` 是同步迭代器。流结束后重新创建客户端、握手并按退避重订阅，
建议从 250 ms 开始、上限 5 s；恢复后用遥测确认当前执行状态。

## TLS

本机仿真使用回环明文连接。跨主机时在客户端提供 CA、可选客户端证书和 metadata：

```python
from pathlib import Path
from rebot_sdk import ArmGatewayClient

gateway = ArmGatewayClient(
    "robot.example:50051",
    secure=True,
    root_certificates=Path("ca.pem").read_bytes(),
    certificate_chain=Path("client.pem").read_bytes(),
    private_key=Path("client-key.pem").read_bytes(),
    metadata=(("authorization", "Bearer <token>"),),
)
```

服务端证书、身份授权、网络隔离和设备安全配置见[安全部署](/deployment/security)。

::: warning 仿真调试参数
`client_timestamp_ns=0` 是本机仿真诊断参数；真实设备连接使用同步的非零时间戳。
:::

## 版本

当前 SDK 为 `0.1.0`，协议为 `arm.console.v1`。新增 protobuf 字段保持兼容；删除字段、
改变单位或改变方法语义时会发布新的协议版本和 SDK 主版本。
