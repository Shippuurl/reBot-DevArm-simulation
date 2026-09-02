use rebot_arm_sdk::ArmGatewayClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = std::env::var("ARM_GATEWAY_GRPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:50051".to_owned());
    let mut gateway = ArmGatewayClient::connect(endpoint, "rebot-sdk-rust-example").await?;
    let connection = gateway.handshake().await?;
    println!(
        "connected session={} source={} dof={}",
        connection.session_id, connection.source, connection.dof
    );

    let enable = gateway.enable(true, "rust-example-enable").await?;
    println!("enable={} reason={}", enable.status, enable.reason);
    let mut stream = gateway.subscribe_telemetry(20).await?;
    if let Some(frame) = stream.message().await? {
        println!(
            "telemetry sequence={} joints={} quality={}",
            frame.sequence,
            frame.joint_position_rad.len(),
            frame.quality
        );
    }
    let stop = gateway.stop(false, "rust-example-stop").await?;
    println!("stop={} reason={}", stop.status, stop.reason);
    Ok(())
}
