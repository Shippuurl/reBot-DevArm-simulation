# reBot Arm SDK

三个 SDK 共享 `arm.console.v1` 协议，覆盖 `ArmPlanner` 的规划请求和 `ArmGateway` 的
控制、轨迹执行与遥测订阅。业务程序按语言选择一个客户端即可。

| SDK | 版本 | 入口 |
| --- | --- | --- |
| Python | 0.1.0 | [`python/`](python/) · [接入指南](../docs/sdk/python.md) |
| C++ | 0.1.0 | [`cpp/`](cpp/) · [接入指南](../docs/sdk/cpp.md) |
| Rust | 0.1.0 | [`rust/`](rust/) · [接入指南](../docs/sdk/rust.md) |

标准调用顺序是：连接网关 → `Handshake` → `Enable` → 规划 → 预检（`dry_run=true`）→ 正式执行 →
从遥测确认结果。协议字段、单位和兼容规则见[SDK 与协议边界](../docs/architecture/sdk-boundary.md)。

跨主机连接需要在服务端和客户端配置 TLS、凭据及网络策略，详见[安全部署](../docs/deployment/security.md)。
