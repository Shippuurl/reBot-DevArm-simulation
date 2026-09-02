#include "rebot_sdk/client.hpp"

#include <cstdlib>
#include <iostream>

int main(int argc, char** argv) {
    const std::string address = argc > 1 ? argv[1] : "127.0.0.1:50053";
    auto channel = grpc::CreateChannel(address, grpc::InsecureChannelCredentials());
    rebot::sdk::ArmPlannerClient planner(std::move(channel));
    rebot::sdk::PoseTarget target;
    target.position_m = {0.25, 0.0, 0.30};

    rebot::sdk::IKResult ik;
    auto status = planner.solve_ik(target, &ik);
    if (!status.ok() || !ik.success) {
        std::cerr << "IK failed: " << (status.ok() ? ik.reason : status.error_message()) << '\n';
        return EXIT_FAILURE;
    }
    rebot::sdk::PoseTarget goal;
    goal.position_m = {0.20, 0.05, 0.32};
    rebot::sdk::TrajectoryPlanResult plan;
    status = planner.plan_trajectory(target, goal, &plan);
    if (!status.ok() || !plan.success || plan.points.size() < 2) {
        std::cerr << "trajectory failed: " << (status.ok() ? plan.reason : status.error_message()) << '\n';
        return EXIT_FAILURE;
    }
    std::cout << "cpp_planner_sdk=OK ik_joints=" << ik.joint_position_rad.size()
              << " trajectory_points=" << plan.points.size()
              << " solver=" << plan.metadata.solver << '\n';
    return EXIT_SUCCESS;
}
