# Python SDK

`rebot-arm-sdk` 是平台对外发布的 Python 客户端。外部工程只需要知道
`ArmGateway`/`ArmPlanner` 的 gRPC 地址和协议，不需要安装或导入平台内部的
MuJoCo、Pinocchio、ProxSuite、URDF、Rerun 或 ROS 2。

## 安装

从当前仓库安装开发版：

```bash
python3 -m pip install ./sdk/python
```

发布版安装方式为：

```bash
python3 -m pip install rebot-arm-sdk
```

SDK 要求 Python 3.10 及以上，运行时依赖 `grpcio` 和 `protobuf`。protobuf
Python 代码已随 SDK 打包，消费方不需要安装 `protoc` 或复制平台仓库。

建议在外部工程使用自己的虚拟环境；SDK 不会修改 ROS 2、Rerun 或系统 Python
环境：

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install rebot-arm-sdk
```

## 网关控制与遥测

```python
from rebot_sdk import ArmGatewayClient, TrajectoryPoint

with ArmGatewayClient("127.0.0.1:50051", client_name="pick-cell-controller") as gateway:
    connection = gateway.connect()
    print(connection.source, connection.dof, connection.session_id)

    if not gateway.enable().accepted:
        raise RuntimeError("gateway refused enable")

    points = [
        TrajectoryPoint(0, (0.0,) * connection.dof),
        TrajectoryPoint(2_000_000_000, (0.2, 0.0, 0.0, 0.0, 0.0, 0.0)),
    ]
    # 先验证，不改变仿真/设备状态；正式执行仍由服务端再次安全检查。
    if not gateway.execute_trajectory(points, dry_run=True).accepted:
        raise RuntimeError("trajectory dry-run rejected")
    gateway.execute_trajectory(points)

    for frame in gateway.subscribe_telemetry(max_rate_hz=20):
        print(frame.sequence, frame.joint_position_rad)
        break

    gateway.stop()
```

可用网关方法：

| 方法 | 用途 |
| --- | --- |
| `connect()` | 握手并取得 `ConnectionInfo`（协议、来源、DOF、会话） |
| `enable(enabled=True)` | 仿真/设备使能或去使能 |
| `jog(joint_index, step_rad)` | 单关节小步进 |
| `execute_trajectory(points, dry_run=False)` | 提交候选轨迹；推荐先 dry-run |
| `pause()` / `resume()` | 暂停或恢复轨迹执行 |
| `speed_scale(scale)` | 设置执行倍率，服务端限制为 0.1–2.0 |
| `stop(emergency=False)` | 停止当前执行队列 |
| `reset_fault()` | 请求清除可恢复故障 |
| `subscribe_telemetry(max_rate_hz=50)` | 订阅关节、TF、轨迹、接触和传感器数据 |

`TelemetryFrame` 是 SDK dataclass。单位固定为米、弧度和纳秒；四元数使用
`x,y,z,w` 顺序。点云位置为三元组，图像为编码后的 `bytes`，SDK 不会替消费方
创建 Rerun 实体。

### 会话和时间戳

`ArmGatewayClient` 在每次握手后缓存服务端返回的独立 `session_id`，控制和遥测请求
自动带上该会话；同一网关上的其他客户端重新握手不会使当前会话失效。控制命令默认填入当前 Unix 纳秒时间戳；服务端会拒绝超过 5 秒
的旧命令以及超前超过 1 秒的命令。只有本机兼容诊断才可以显式传入
`client_timestamp_ns=0`，真机适配不能依赖这个豁免。

仿真服务会清理超过 1 小时无活动的会话，并限制会话表为 1024 个；这不是生产身份
认证或租户隔离机制。

构造函数的 `client_name` 用于服务端日志和后续授权策略，必须是非空且不含空白的短
字符串；它不是身份认证凭据。

客户端关闭上下文或调用 `close()` 后会释放 gRPC channel。遥测流结束时，
`subscribe_telemetry()` 抛出 `RebotRpcError`；生产应用应捕获该异常、重新建立
客户端/握手并以退避策略重新订阅，不要把断流解释为设备安全停机。

## 规划服务

规划服务不执行控制命令，只返回候选结果：

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
    if plan.success:
        print(len(plan.points), plan.metadata.solver)
```

`PoseTarget` 默认使用 `world` 坐标系；位置单位为米，关节单位为弧度。目标四元
数全为零表示不约束姿态；若提供非零四元数，应使用单位四元数。装配阶段可将
`assembly_phase="MATE"` 并显式提供 `allowed_collision_pairs`，但这只改变规划
阶段的允许接触，网关执行前仍会进行二次安全检查。

推荐的距离余量是规划 0.02 m、运行监控 0.005–0.01 m；精密装配切换到受控接触
和力/力矩 watchdog。SDK 的 `CommandAck` 只表示服务端接受或拒绝请求，不替代
急停、硬件限位、伺服 watchdog 或安全 PLC。

## TLS、身份和网络边界

本机仿真默认使用回环地址和不加密 gRPC。跨主机部署必须启用 TLS，并在网络层
限制控制端口：

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

当前仿真网关仍使用 `InsecureServerCredentials()`、未绑定身份的会话和受信任网络假设，
不能直接暴露到局域网或互联网。对外发布前必须完成服务端 TLS、客户端身份认证、
授权策略、独立会话生命周期、审计和多租户隔离；SDK 的 TLS 参数只是客户端能力，
不会自动提升不安全服务端的安全等级。

## 错误处理和兼容性

RPC 失败统一映射为 `RebotRpcError`，原始 gRPC 状态码保存在 `.code`：

```python
from rebot_sdk import RebotRpcError

try:
    with ArmGatewayClient("127.0.0.1:50051") as gateway:
        gateway.enable()
except RebotRpcError as exc:
    print(exc.code, exc)
```

握手会校验 `arm.console.v1`。新增 protobuf 字段保持向后兼容；删除字段、修改
语义或不兼容的单位变化必须发布新的协议版本和对应 SDK 主版本。应用应记录
`ConnectionInfo.protocol_version`、请求 ID、服务端 `reason` 和规划元数据，便于
审计与回放。

## 当前支持范围

| 语言/层 | 状态 |
| --- | --- |
| Python SDK | v0.1，已覆盖网关控制/遥测和规划 RPC |
| C++ SDK | v0.1 原型已可从源码构建；待发布二进制/包和兼容矩阵 |
| Rust SDK | v0.1 源码包，提供异步网关/规划客户端；[中文指南](/sdk/rust) |
| ROS 2 适配层 | 可选原型，调用同一 SDK/gRPC 服务，不进入平台核心 |

SDK 版本、协议兼容矩阵、TLS 配置和示例应在每次发布时同步更新；外部工程不应
复制 `protocol/arm_console.proto` 以外的平台源码。
