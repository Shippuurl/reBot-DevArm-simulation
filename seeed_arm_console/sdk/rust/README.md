# reBot Arm Rust SDK

`rebot-arm-sdk` 是 `arm.console.v1` 的 Rust 客户端。公共 API 只暴露
transport-neutral 数据结构；protobuf 生成模块保持私有，外部工程不需要依赖
平台的 Rerun Viewer、MuJoCo、Pinocchio、ProxSuite、URDF 或 ROS 2。

## 构建

```bash
cargo test --manifest-path sdk/rust/Cargo.toml
scripts/run_rust_sdk_smoke.sh
# 同时运行规划示例：
RUST_SDK_RUN_PLANNER=1 scripts/run_rust_sdk_smoke.sh
```

SDK 从仓库唯一协议源 `protocol/arm_console.proto` 生成内部 stubs。发布包应锁定
SDK 与 `arm.console.v1` 的兼容矩阵；当前版本为 `0.1.0`，仓库暂不上传 crates.io。

## 网关

```rust
use rebot_arm_sdk::{ArmGatewayClient, GatewayCommand};

# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut gateway = ArmGatewayClient::connect(
    "http://127.0.0.1:50051",
    "pick-cell-controller",
).await?;
let connection = gateway.handshake().await?;
gateway.enable(true, "enable").await?;

let mut stream = gateway.subscribe_telemetry(20).await?;
if let Some(frame) = stream.message().await? {
    println!("{} {:?}", frame.sequence, frame.joint_position_rad);
}
gateway.command(GatewayCommand::Stop { emergency: false }, "stop").await?;
# Ok(())
# }
```

`ArmGatewayClient` 也提供 `jog`、`execute_trajectory`、`pause`、`resume`、
`speed_scale` 和 `reset_fault`。每次 `handshake` 建立独立 session；控制命令自动带
当前 Unix 纳秒时间戳。`set_session_id` 允许控制任务在独立 channel 上复用已有会话。
流断开后由应用重新握手并按退避策略调用 `subscribe_telemetry`，不要把网络错误当作
安全停机。

默认 `connect` 使用调用方给出的 HTTP(S) endpoint。需要 mTLS、自定义根证书或拦截器
时，可启用 `tls-native-roots`/`tls-webpki-roots` feature 后通过 tonic 构造 `Channel`，
再使用 `ArmGatewayClient::from_channel`；SDK 不会替服务端开启 TLS 或授权。

## 规划

`ArmPlannerClient` 提供 `solve_ik` 与 `plan_trajectory`，分别接收 `IKOptions` 和
`TrajectoryOptions`，返回 IK 候选、带速度字段的轨迹、碰撞摘要和规划元数据。规划
结果必须先提交 ArmGateway dry-run，再进行最终执行复核。

## 边界

- 单位为米、弧度、弧度/秒和纳秒；四元数顺序为 `x,y,z,w`。
- SDK 不实现自动重连、急停、硬件限位、watchdog 或安全 PLC。
- 跨主机部署必须使用 TLS、客户端身份、授权、session 吊销和审计；当前仿真服务只
  适合回环或受信任网络。
