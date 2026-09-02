use rebot_arm_sdk::{ArmPlannerClient, IKOptions, PoseTarget, TrajectoryOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = std::env::var("ARM_PLANNER_GRPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:50053".to_owned());
    let mut planner = ArmPlannerClient::connect(endpoint).await?;
    let result = planner
        .solve_ik(
            PoseTarget::new([0.25, 0.0, 0.30]),
            IKOptions {
                ..Default::default()
            },
        )
        .await?;
    println!(
        "ik success={} joints={} reason={}",
        result.success,
        result.joint_position_rad.len(),
        result.reason
    );
    if result.success {
        let plan = planner
            .plan_trajectory(
                PoseTarget::new([0.25, 0.0, 0.30]),
                PoseTarget::new([0.20, 0.05, 0.32]),
                TrajectoryOptions::default(),
            )
            .await?;
        println!(
            "trajectory success={} points={} solver={}",
            plan.success,
            plan.points.len(),
            plan.metadata.solver
        );
    }
    Ok(())
}
