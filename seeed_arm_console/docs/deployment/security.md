# 安全部署

本页说明跨主机和真实设备部署时需要补齐的配置。单机仿真直接按[仿真工作站](/guide/simulation)
运行即可。

## 单机仿真

Compose 将服务绑定到主机回环地址：

| 端口 | 用途 | 默认绑定 |
| ---: | --- | --- |
| 50051 | `ArmGateway` 控制与遥测 | `127.0.0.1` |
| 50052 | JSON 诊断 | `127.0.0.1` |
| 50053 | `ArmPlanner` 规划 | `127.0.0.1` |
| 9876 | Rerun Viewer 接收端 | `127.0.0.1` |

仿真网关使用明文 gRPC，适合开发机和隔离的 CI 网络。

## 跨主机

上线前按四项配置：

1. **TLS**：服务端配置证书；客户端配置 CA。需要双向认证时，再配置客户端证书和私钥。
2. **身份与权限**：为每个业务应用分配独立凭据，分别授权 Gateway 控制、Planner 规划和
   只读遥测。`client_name` 用于日志标识。
3. **网络**：防火墙只开放业务网段需要的端口；50052 和 9876 放在管理网。
4. **审计**：记录握手、会话、命令 ID、操作者、结果和服务端 `reason`，并同步主机时钟。

Python 客户端示例：

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

C++ 和 Rust 通过各自的 gRPC/tonic channel 配置同样的证书和 metadata，见对应的
[C++ SDK](/sdk/cpp) 和 [Rust SDK](/sdk/rust) 指南。

## 真实设备接入

把 Gateway 连接设备驱动前，完成以下验收：

- 急停、使能回路、硬件限位的触发和复位；
- 通信中断、进程退出、断电时的 watchdog 行为；
- 驱动侧的速度、位置、碰撞和时间戳检查；
- 故障恢复、人工确认和审计记录；
- Viewer、Planner 或网络断开时由独立安全链路接管设备。

::: warning 安全门槛
仓库内的网关实现面向仿真。完成急停、限位、watchdog 和断电测试，并通过设备安全审核
后再开放控制端口；Viewer 负责观察和联调。
:::

## 发布检查

| 项目 | 验收条件 |
| --- | --- |
| TLS | 服务端证书校验通过，明文端口关闭 |
| 身份 | 每个应用使用独立凭据，权限最小化 |
| 网络 | 控制端口仅对业务网段开放，诊断端口隔离 |
| 可靠性 | 会话租约、重连退避、时钟同步和审计可观测 |
| 设备安全 | 急停、限位、watchdog、断电和故障恢复完成实机测试 |
