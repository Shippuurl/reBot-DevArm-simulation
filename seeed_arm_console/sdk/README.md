# reBot Arm SDKs

这些 SDK 是外部工程访问平台 gRPC 服务的唯一公共客户端边界。它们只依赖
`protocol/arm_console.proto` 生成的传输代码，不暴露平台内部的 Rerun Viewer、
MuJoCo、Pinocchio、ProxSuite、URDF 或 ROS 2。

| SDK | 状态 | 入口 |
| --- | --- | --- |
| Python | v0.1 可安装版 | [`python/`](python/)；[中文指南](../docs/sdk/python.md) |
| C++ | v0.1 源码构建原型 | [`cpp/`](cpp/)；[中文指南](../docs/sdk/cpp.md) |
| Rust | v0.1 源码包 | [`rust/`](rust/)；[中文指南](../docs/sdk/rust.md) |

协议版本为 `arm.console.v1`。服务端安全检查、硬件 watchdog、急停和授权策略不由
SDK 取代；跨网络部署必须使用服务端 TLS、客户端身份和受限网络策略。
