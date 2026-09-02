# reBot Arm Python SDK

`rebot-arm-sdk` 为 Python 业务程序提供 `ArmGateway` 控制/遥测和 `ArmPlanner` 规划客户端。
公共接口是 `rebot_sdk` dataclass 与同步方法。

## 安装与验证

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install ./sdk/python
python -m unittest discover -s sdk/python/tests
```

要求 Python 3.10+；协议生成代码随安装包提供。

## 示例

```python
from rebot_sdk import ArmGatewayClient

with ArmGatewayClient("127.0.0.1:50051", client_name="pick-cell-controller") as gateway:
    info = gateway.connect()
    print(info.source, info.dof, info.session_id)
    print(gateway.enable().status)
    for frame in gateway.subscribe_telemetry(max_rate_hz=20):
        print(frame.sequence, frame.joint_position_rad)
        break
    gateway.stop()
```

轨迹先用 `execute_trajectory(..., dry_run=True)` 预检，再提交正式执行；规划示例见
[Python SDK 接入指南](../../docs/sdk/python.md)。

跨主机 TLS 和 metadata 配置见[安全部署](../../docs/deployment/security.md)。
