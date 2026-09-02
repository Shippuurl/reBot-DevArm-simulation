# reBot Arm Rust SDK

`rebot-arm-sdk` 是 `arm.console.v1` 的异步 Rust 客户端。它提供网关控制、遥测流、IK 和
轨迹规划 API，公共接口使用 Rust 值类型。

## 构建与验证

```bash
cargo test --manifest-path sdk/rust/Cargo.toml
scripts/run_rust_sdk_smoke.sh
RUST_SDK_RUN_PLANNER=1 scripts/run_rust_sdk_smoke.sh
```

在业务工程中可使用路径依赖：

```toml
rebot-arm-sdk = { path = "../seeed_arm_console/sdk/rust" }
```

当前版本为 `0.1.0`，最低 Rust 版本为 1.85。

## 网关示例

```rust
use rebot_arm_sdk::{ArmGatewayClient, GatewayCommand};

let mut gateway = ArmGatewayClient::connect(
    "http://127.0.0.1:50051",
    "pick-cell-controller",
).await?;
let info = gateway.handshake().await?;
let ack = gateway.enable(true, "enable").await?;
assert!(ack.accepted());
let mut stream = gateway.subscribe_telemetry(20).await?;
if let Some(frame) = stream.message().await? {
    println!("{} {:?}", frame.sequence, frame.joint_position_rad);
}
gateway.command(GatewayCommand::Stop { emergency: false }, "stop").await?;
```

轨迹先用 `dry_run: true` 预检，再提交正式执行。会话、重连和规划示例见
[Rust SDK 接入指南](../../docs/sdk/rust.md)。

## TLS

`connect` 接受 HTTP(S) endpoint；自定义根证书、mTLS 和 metadata 使用 tonic channel 与
`from_channel`。部署清单见[安全部署](../../docs/deployment/security.md)。
