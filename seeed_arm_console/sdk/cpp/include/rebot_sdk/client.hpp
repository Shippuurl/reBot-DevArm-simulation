#pragma once

// Public C++ SDK boundary for arm.console.v1.
//
// This header intentionally contains no generated protobuf types and no
// dependency on the platform's Viewer, MuJoCo, Pinocchio, ProxSuite or URDF.
// Consumers provide a gRPC channel (insecure or TLS) and exchange the small
// value types below.

#include <array>
#include <cstdint>
#include <functional>
#include <memory>
#include <string>
#include <utility>
#include <vector>

#include <grpcpp/grpcpp.h>

namespace rebot::sdk {

inline constexpr char kProtocolVersion[] = "arm.console.v1";
using Metadata = std::vector<std::pair<std::string, std::string>>;

struct AllowedCollisionPair {
    std::string first;
    std::string second;
};

struct PoseTarget {
    std::array<double, 3> position_m{};
    // x,y,z,w. All zeros means orientation unspecified.
    std::array<double, 4> rotation_xyzw{};
    std::string frame_id = "world";
};

struct TrajectoryPoint {
    std::uint64_t time_from_start_ns = 0;
    std::vector<double> position_rad;
    std::vector<double> velocity_rad_s;
};

struct Transform {
    std::string parent;
    std::string child;
    std::array<double, 3> translation_m{};
    std::array<double, 4> rotation_xyzw{};
};

struct Contact {
    std::string first_geom;
    std::string second_geom;
    double distance_m = 0.0;
    double normal_force_n = 0.0;
};

struct ImageFrame {
    std::string sensor;
    std::uint32_t width = 0;
    std::uint32_t height = 0;
    std::string encoding;
    std::vector<std::uint8_t> data;
};

struct PointCloud {
    std::string sensor;
    std::vector<std::array<float, 3>> positions_xyz;
    std::vector<std::uint32_t> colors_rgba;
};

struct TelemetryFrame {
    std::uint64_t sequence = 0;
    std::uint64_t timestamp_ns = 0;
    std::uint64_t sim_time_ns = 0;
    std::uint64_t wall_time_ns = 0;
    std::string source;
    std::string quality;
    std::vector<double> joint_position_rad;
    std::vector<double> joint_velocity_rad_s;
    std::vector<Transform> tf;
    std::vector<TrajectoryPoint> planned_trajectory;
    std::vector<TrajectoryPoint> actual_trajectory;
    std::vector<ImageFrame> images;
    std::vector<PointCloud> point_clouds;
    std::vector<Contact> contacts;
};

struct ConnectionInfo {
    std::string session_id;
    std::string protocol_version;
    std::string source;
    std::uint32_t dof = 0;
};

struct CommandAck {
    std::string command_id;
    std::string status;
    std::string reason;

    [[nodiscard]] bool accepted() const noexcept { return status == "ACCEPTED"; }
};

struct CollisionSummary {
    bool checked = false;
    bool collision_free = true;
    std::uint32_t checked_pairs = 0;
    std::vector<std::string> contacts;
    double minimum_distance_m = 0.0;
};

struct PlanningMetadata {
    std::string model_version;
    std::string solver;
    std::uint64_t random_seed = 0;
    std::uint64_t elapsed_ns = 0;
};

struct IKResult {
    std::string request_id;
    bool success = false;
    std::vector<double> joint_position_rad;
    bool within_limits = false;
    CollisionSummary collision;
    PlanningMetadata metadata;
    std::string reason;
};

struct TrajectoryPlanResult {
    std::string request_id;
    bool success = false;
    std::vector<TrajectoryPoint> points;
    CollisionSummary collision;
    PlanningMetadata metadata;
    std::string reason;
};

class ArmGatewayClient final {
public:
    // The channel controls transport security. Use grpc::SslCredentials for
    // TLS deployments and grpc::InsecureChannelCredentials only on trusted
    // local networks.
    explicit ArmGatewayClient(std::shared_ptr<grpc::Channel> channel,
                              std::string client_name = "rebot-sdk-cpp",
                              Metadata metadata = {});
    ~ArmGatewayClient();
    ArmGatewayClient(ArmGatewayClient&&) noexcept;
    ArmGatewayClient& operator=(ArmGatewayClient&&) noexcept;
    ArmGatewayClient(const ArmGatewayClient&) = delete;
    ArmGatewayClient& operator=(const ArmGatewayClient&) = delete;

    grpc::Status handshake(ConnectionInfo* out, int timeout_ms = 5000);
    grpc::Status enable(bool enabled, CommandAck* out, int timeout_ms = 5000);
    grpc::Status stop(bool emergency, CommandAck* out, int timeout_ms = 5000);
    grpc::Status jog(std::uint32_t joint_index, double step_rad, double speed_limit_rad_s,
                     CommandAck* out, int timeout_ms = 5000);
    grpc::Status execute_trajectory(const std::vector<TrajectoryPoint>& points, bool dry_run,
                                    CommandAck* out, int timeout_ms = 10000);
    grpc::Status reset_fault(CommandAck* out, int timeout_ms = 5000);
    grpc::Status pause(CommandAck* out, int timeout_ms = 5000);
    grpc::Status resume(CommandAck* out, int timeout_ms = 5000);
    grpc::Status speed_scale(double scale, CommandAck* out, int timeout_ms = 5000);

    // The callback returns true to continue and false to cancel the stream.
    // A server/transport failure is returned by Finish(); callback cancellation
    // normally returns grpc::Status::OK.
    grpc::Status subscribe_telemetry(
        std::uint32_t max_rate_hz, const std::function<bool(const TelemetryFrame&)>& callback,
        int timeout_ms = 0);

    [[nodiscard]] const ConnectionInfo* connection() const noexcept;

private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

class ArmPlannerClient final {
public:
    explicit ArmPlannerClient(std::shared_ptr<grpc::Channel> channel, Metadata metadata = {});
    ~ArmPlannerClient();
    ArmPlannerClient(ArmPlannerClient&&) noexcept;
    ArmPlannerClient& operator=(ArmPlannerClient&&) noexcept;
    ArmPlannerClient(const ArmPlannerClient&) = delete;
    ArmPlannerClient& operator=(const ArmPlannerClient&) = delete;

    grpc::Status solve_ik(const PoseTarget& target, IKResult* out,
                          std::string request_id = {},
                          const std::vector<double>& seed_position_rad = {},
                          bool check_collisions = false,
                          double minimum_distance_threshold_m = 0.0,
                          std::string assembly_phase = {},
                          const std::vector<AllowedCollisionPair>& allowed_collision_pairs = {},
                          int timeout_ms = 15000);
    grpc::Status plan_trajectory(const PoseTarget& start, const PoseTarget& goal,
                                 TrajectoryPlanResult* out,
                                 std::string request_id = {}, std::uint32_t max_rate_hz = 20,
                                 bool dry_run = false, bool check_collisions = false,
                                 double minimum_distance_threshold_m = 0.0,
                                 std::string assembly_phase = {},
                                 const std::vector<AllowedCollisionPair>& allowed_collision_pairs = {},
                                 int timeout_ms = 20000);

private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

}  // namespace rebot::sdk
