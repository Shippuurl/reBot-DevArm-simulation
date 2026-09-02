# Rust SDK

`rebot-arm-sdk` 是平台对外的 Rust 客户端边界。它与 Python/C++ SDK 使用同一份
`arm.console.v1` 协议，protobuf 生成类型留在 SDK 内部，外部工程只依赖值类型和
gRPC 客户端。

## 安装与构建

当前仓库提供源码包：

```bash
cargo test --manifest-path sdk/rust/Cargo.toml
scripts/run_rust_sdk_smoke.sh
# 同时运行规划示例：
RUST_SDK_RUN_PLANNER=1 scripts/run_rust_sdk_smoke.sh
```

外部工程可通过 Git 依赖或发布归档引用 `sdk/rust`。SDK 会在构建时从协议源生成
stubs；发布归档应同时记录 SDK 版本、协议版本和 Rust/MSRV 兼容矩阵。当前版本为
`0.1.0`，协议版本为 `arm.console.v1`。

## 网关调用

```rust
use rebot_arm_sdk::{ArmGatewayClient, GatewayCommand};

# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut gateway = ArmGatewayClient::connect(
    "http://127.0.0.1:50051",
    "pick-cell-controller",
).await?;
let info = gateway.handshake().await?;
println!("session={} dof={}", info.session_id, info.dof);

let ack = gateway.enable(true, "enable").await?;
assert!(ack.accepted());

let mut telemetry = gateway.subscribe_telemetry(20).await?;
while let Some(frame) = telemetry.message().await? {
    println!("seq={} joints={}", frame.sequence, frame.joint_position_rad.len());
    break;
}

gateway.command(GatewayCommand::Stop { emergency: false }, "stop").await?;
# Ok(())
# }
```

控制命令还包括 `jog`、`execute_trajectory`、`pause`、`resume`、`speed_scale` 和
`reset_fault`。命令自动填充当前 Unix 纳秒时间戳；网关仍会执行限位、碰撞、时序和
新鲜度检查。`ConnectionInfo.session_id` 每次握手独立生成，不能视为身份凭据。

## 规划调用

`ArmPlannerClient::connect` 连接独立的规划端口：

```rust
use rebot_arm_sdk::{ArmPlannerClient, PoseTarget, IKOptions};

# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut planner = ArmPlannerClient::connect("http://127.0.0.1:50053").await?;
let result = planner.solve_ik(PoseTarget::new([0.25, 0.0, 0.30]), IKOptions {
    check_collisions: true,
    minimum_distance_threshold_m: 0.02,
    ..Default::default()
}).await?;
println!("success={} reason={}", result.success, result.reason);
# Ok(())
# }
```

规划服务不执行控制。成功轨迹必须交给网关 dry-run，并在执行前再次进行服务端安全
复核；装配阶段的 ACM 白名单和距离阈值只影响规划请求，不会绕过网关安全边界。

## TLS、metadata 与重连

默认 `connect` 接收完整 `http://` 或 `https://` endpoint。需要 mTLS、自定义根证书
或拦截器时，启用 `tls-native-roots` 或 `tls-webpki-roots` feature，由调用方用 tonic
构造 `Channel`，再传入 `from_channel`；也可链式调用 `with_metadata` 添加认证/审计
metadata。SDK 不会自动把不安全服务端升级为 TLS。

遥测流断开后，应用应重新创建客户端、握手并按 250 ms 起始、5 s 上限的退避策略重订阅。
断流不能替代急停或硬件 watchdog。跨主机发布前必须完成服务端 TLS、客户端身份、授权、
session 吊销/租约、多租户隔离和审计。

## API 边界

- 不依赖 Rerun、MuJoCo、Pinocchio、ProxSuite、URDF 或 ROS 2。
- `TelemetryFrame` 包含关节、TF、规划/实际轨迹、接触和有限传感器载荷；SDK 不创建
  Rerun 实体。
- Rust Viewer 现在调用同一 SDK，但 Viewer 的 UI 和 Rerun 记录逻辑仍属于平台内部。
