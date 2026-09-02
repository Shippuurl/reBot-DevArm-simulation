# Rust SDK 接入指南

`rebot-arm-sdk` 是 `arm.console.v1` 的异步 Rust 客户端，面向 Tokio 服务和实时数据管线。
公共 API 使用 Rust 值类型，网关控制、规划和遥测共用同一套模型。

## 引入

在业务工程的 `Cargo.toml` 中加入源码路径或 Git 依赖：

```toml
[dependencies]
rebot-arm-sdk = { path = "../seeed_arm_console/sdk/rust" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

仓库内验证：

```bash
cargo test --manifest-path sdk/rust/Cargo.toml
scripts/run_rust_sdk_smoke.sh
RUST_SDK_RUN_PLANNER=1 scripts/run_rust_sdk_smoke.sh
```

当前版本为 `0.1.0`，Rust 最低版本为 1.85；根工作区 Viewer 使用 Rust 1.95+。

## 网关调用

```rust
use rebot_arm_sdk::{ArmGatewayClient, GatewayCommand};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut gateway = ArmGatewayClient::connect(
        "http://127.0.0.1:50051",
        "pick-cell-controller",
    ).await?;

    let info = gateway.handshake().await?;
    println!("source={} dof={} session={}", info.source, info.dof, info.session_id);

    let ack = gateway.enable(true, "enable").await?;
    if !ack.accepted() {
        return Err(format!("enable rejected: {}", ack.reason).into());
    }

    let mut stream = gateway.subscribe_telemetry(20).await?;
    if let Some(frame) = stream.message().await? {
        println!("seq={} joints={:?}", frame.sequence, frame.joint_position_rad);
    }

    gateway.command(GatewayCommand::Stop { emergency: false }, "stop").await?;
    Ok(())
}
```

`GatewayCommand` 还支持 `Jog`、`ExecuteTrajectory`、`Pause`、`Resume`、`SpeedScale` 和
`ResetFault`。轨迹提交时先使用 `ExecuteTrajectory { dry_run: true }` 做预检，确认 ACK
后再以 `false` 执行；执行状态从遥测帧读取。

## 规划调用

```rust
use rebot_arm_sdk::{ArmPlannerClient, IKOptions, PoseTarget};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut planner = ArmPlannerClient::connect("http://127.0.0.1:50053").await?;
    let result = planner.solve_ik(
        PoseTarget::new([0.25, 0.0, 0.30]),
        IKOptions {
            check_collisions: true,
            minimum_distance_threshold_m: 0.02,
            ..Default::default()
        },
    ).await?;
    if !result.success {
        eprintln!("IK rejected: {}", result.reason);
        return Ok(());
    }
    println!("joints={:?} solver={}", result.joint_position_rad, result.metadata.solver);
    Ok(())
}
```

`ArmPlannerClient::plan_trajectory` 返回带时间和速度的候选点。目标默认位于 `world`，
位置用米、关节用弧度，四元数按 `x,y,z,w` 排列。装配阶段支持 `APPROACH`、`MATE` 和
`RETRACT`。

## 会话与重连

`handshake` 为客户端建立独立 `session_id`，SDK 会自动把它带入控制和遥测请求。控制
命令自动使用当前 Unix 纳秒时间戳；网关接受当前时间前 5 秒至后 1 秒的非零值。

遥测流结束时，重新创建客户端、握手并按退避策略重新订阅，建议从 250 ms 开始、5 s
封顶。控制和遥测需要并行时，可为每个任务创建独立 channel，并用 `set_session_id` 复用
同一会话。

## TLS 与 metadata

`connect` 接受完整的 `http://` 或 `https://` endpoint。需要自定义根证书、mTLS 或拦截器
时，启用 `tls-native-roots` 或 `tls-webpki-roots` feature，用 tonic 构造 `Channel` 后
传给 `from_channel`：

```rust
// 伪代码：按部署环境配置 Endpoint、TLS 和 metadata
// let channel = Endpoint::from_shared("https://robot.example:50051")?
//     .tls_config(tls_config)?
//     .connect()
//     .await?;
// let gateway = ArmGatewayClient::from_channel(channel, "pick-cell-controller")
//     .with_metadata([(String::from("authorization"), String::from("Bearer <token>"))]);
```

## 数据与错误

`TelemetryFrame`、`TrajectoryPoint`、`ConnectionInfo` 等类型独立于 protobuf。单位为米、
弧度、弧度/秒和 Unix 纳秒；点云、图像和接触数据随帧携带。RPC 错误返回 `tonic::Status`，
流错误由 `TelemetryStream::message()` 返回。

证书、权限、网络隔离和设备安全配置见[安全部署](/deployment/security)。
