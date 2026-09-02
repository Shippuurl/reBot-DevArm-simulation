#include "rebot_sdk/client.hpp"

#include "arm_console.grpc.pb.h"

#include <algorithm>
#include <atomic>
#include <cctype>
#include <chrono>
#include <cmath>
#include <utility>

namespace rebot::sdk {
namespace {

namespace Proto = ::arm::console::v1;

void deadline(grpc::ClientContext& context, int timeout_ms) {
    if (timeout_ms > 0) {
        context.set_deadline(std::chrono::system_clock::now() +
                             std::chrono::milliseconds(timeout_ms));
    }
}

std::uint64_t now_ns() {
    return static_cast<std::uint64_t>(std::chrono::duration_cast<std::chrono::nanoseconds>(
        std::chrono::system_clock::now().time_since_epoch()).count());
}

std::string request_id(std::string value) {
    static std::atomic<std::uint64_t> next{0};
    if (!value.empty()) return value;
    return "rebot-sdk-cpp-" + std::to_string(now_ns()) + "-" + std::to_string(++next);
}

void pose_to_proto(const PoseTarget& source, Proto::PoseTarget* target) {
    target->set_position_x_m(source.position_m[0]);
    target->set_position_y_m(source.position_m[1]);
    target->set_position_z_m(source.position_m[2]);
    target->set_rotation_x(source.rotation_xyzw[0]);
    target->set_rotation_y(source.rotation_xyzw[1]);
    target->set_rotation_z(source.rotation_xyzw[2]);
    target->set_rotation_w(source.rotation_xyzw[3]);
    target->set_frame_id(source.frame_id);
}

void point_to_proto(const TrajectoryPoint& source, Proto::TrajectoryPoint* target) {
    target->set_time_from_start_ns(source.time_from_start_ns);
    for (const auto value : source.position_rad) target->add_position_rad(value);
    for (const auto value : source.velocity_rad_s) target->add_velocity_rad_s(value);
}

TrajectoryPoint point_from_proto(const Proto::TrajectoryPoint& source) {
    TrajectoryPoint target;
    target.time_from_start_ns = source.time_from_start_ns();
    target.position_rad.assign(source.position_rad().begin(), source.position_rad().end());
    target.velocity_rad_s.assign(source.velocity_rad_s().begin(), source.velocity_rad_s().end());
    return target;
}

CollisionSummary collision_from_proto(const Proto::CollisionSummary& source) {
    CollisionSummary target;
    target.checked = source.checked();
    target.collision_free = source.collision_free();
    target.checked_pairs = source.checked_pairs();
    target.contacts.assign(source.contacts().begin(), source.contacts().end());
    target.minimum_distance_m = source.minimum_distance_m();
    return target;
}

PlanningMetadata metadata_from_proto(const Proto::PlanningMetadata& source) {
    PlanningMetadata target;
    target.model_version = source.model_version();
    target.solver = source.solver();
    target.random_seed = source.random_seed();
    target.elapsed_ns = source.elapsed_ns();
    return target;
}

CommandAck ack_from_proto(const Proto::CommandAck& source) {
    return {source.command_id(), Proto::AckStatus_Name(source.status()), source.reason()};
}

Proto::AssemblyPhase assembly_phase(std::string value) {
    if (value.empty()) return Proto::ASSEMBLY_PHASE_UNSPECIFIED;
    std::transform(value.begin(), value.end(), value.begin(),
                   [](unsigned char character) { return static_cast<char>(std::toupper(character)); });
    if (value == "APPROACH") return Proto::APPROACH;
    if (value == "MATE") return Proto::MATE;
    if (value == "RETRACT") return Proto::RETRACT;
    return Proto::ASSEMBLY_PHASE_UNSPECIFIED;
}

void collision_pairs_to_proto(const std::vector<AllowedCollisionPair>& source,
                              google::protobuf::RepeatedPtrField<Proto::AllowedCollisionPair>* target) {
    for (const auto& item : source) {
        auto* pair = target->Add();
        pair->set_first(item.first);
        pair->set_second(item.second);
    }
}

TelemetryFrame frame_from_proto(const Proto::TelemetryFrame& source) {
    TelemetryFrame target;
    target.sequence = source.sequence();
    target.timestamp_ns = source.timestamp_ns();
    target.sim_time_ns = source.sim_time_ns();
    target.wall_time_ns = source.wall_time_ns();
    target.source = Proto::SourceKind_Name(source.source());
    target.quality = Proto::SampleQuality_Name(source.quality());
    target.joint_position_rad.assign(source.joint_position_rad().begin(),
                                     source.joint_position_rad().end());
    target.joint_velocity_rad_s.assign(source.joint_velocity_rad_s().begin(),
                                       source.joint_velocity_rad_s().end());
    for (const auto& item : source.tf()) {
        target.tf.push_back({item.parent(), item.child(),
                             {item.translation_x_m(), item.translation_y_m(), item.translation_z_m()},
                             {item.rotation_x(), item.rotation_y(), item.rotation_z(), item.rotation_w()}});
    }
    for (const auto& item : source.planned_trajectory()) target.planned_trajectory.push_back(point_from_proto(item));
    for (const auto& item : source.actual_trajectory()) target.actual_trajectory.push_back(point_from_proto(item));
    for (const auto& item : source.images()) {
        target.images.push_back({item.sensor(), item.width(), item.height(), item.encoding(),
                                 {item.data().begin(), item.data().end()}});
    }
    for (const auto& item : source.point_clouds()) {
        PointCloud cloud;
        cloud.sensor = item.sensor();
        for (int index = 0; index + 2 < item.positions_xyz_size(); index += 3) {
            cloud.positions_xyz.push_back({item.positions_xyz(index), item.positions_xyz(index + 1),
                                           item.positions_xyz(index + 2)});
        }
        cloud.colors_rgba.assign(item.colors_rgba().begin(), item.colors_rgba().end());
        target.point_clouds.push_back(std::move(cloud));
    }
    for (const auto& item : source.contacts()) {
        target.contacts.push_back({item.first_geom(), item.second_geom(), item.distance_m(),
                                   item.normal_force_n()});
    }
    return target;
}

}  // namespace

class ArmGatewayClient::Impl {
public:
    explicit Impl(std::shared_ptr<grpc::Channel> channel, std::string name, Metadata values)
        : stub(Proto::ArmGateway::NewStub(std::move(channel))), client_name(std::move(name)),
          metadata(std::move(values)) {}

    void add_metadata(grpc::ClientContext& context) const {
        for (const auto& item : metadata) context.AddMetadata(item.first, item.second);
    }

    grpc::Status ensure_session(int timeout_ms) {
        if (!connection.session_id.empty()) return grpc::Status::OK;
        ConnectionInfo ignored;
        return handshake(&ignored, timeout_ms);
    }

    grpc::Status handshake(ConnectionInfo* out, int timeout_ms) {
        if (out == nullptr) return grpc::Status(grpc::StatusCode::INVALID_ARGUMENT, "out is null");
        grpc::ClientContext context;
        deadline(context, timeout_ms);
        add_metadata(context);
        Proto::ConnectRequest request;
        request.set_client_name(client_name);
        request.set_protocol_version(kProtocolVersion);
        Proto::ConnectReply reply;
        const auto status = stub->Handshake(&context, request, &reply);
        if (!status.ok()) return status;
        if (!reply.protocol_version().empty() && reply.protocol_version() != kProtocolVersion) {
            return grpc::Status(grpc::StatusCode::UNIMPLEMENTED, "unsupported gateway protocol");
        }
        connection = {reply.session_id(), reply.protocol_version().empty() ? kProtocolVersion : reply.protocol_version(),
                      Proto::SourceKind_Name(reply.source()), reply.dof()};
        *out = connection;
        return grpc::Status::OK;
    }

    grpc::Status command(const std::function<void(Proto::ControlCommand*)>& set_payload,
                         CommandAck* out, int timeout_ms) {
        if (out == nullptr) return grpc::Status(grpc::StatusCode::INVALID_ARGUMENT, "out is null");
        auto status = ensure_session(timeout_ms);
        if (!status.ok()) return status;
        grpc::ClientContext context;
        deadline(context, timeout_ms);
        add_metadata(context);
        Proto::ControlCommand request;
        auto* header = request.mutable_header();
        header->set_session_id(connection.session_id);
        header->set_command_id(request_id({}));
        header->set_client_timestamp_ns(now_ns());
        set_payload(&request);
        Proto::CommandAck reply;
        status = stub->Command(&context, request, &reply);
        if (status.ok()) *out = ack_from_proto(reply);
        return status;
    }

    std::unique_ptr<Proto::ArmGateway::Stub> stub;
    std::string client_name;
    Metadata metadata;
    ConnectionInfo connection;
};

ArmGatewayClient::ArmGatewayClient(std::shared_ptr<grpc::Channel> channel, std::string client_name,
                                   Metadata metadata)
    : impl_(std::make_unique<Impl>(std::move(channel), std::move(client_name), std::move(metadata))) {}
ArmGatewayClient::~ArmGatewayClient() = default;
ArmGatewayClient::ArmGatewayClient(ArmGatewayClient&&) noexcept = default;
ArmGatewayClient& ArmGatewayClient::operator=(ArmGatewayClient&&) noexcept = default;

grpc::Status ArmGatewayClient::handshake(ConnectionInfo* out, int timeout_ms) {
    return impl_->handshake(out, timeout_ms);
}

grpc::Status ArmGatewayClient::enable(bool enabled, CommandAck* out, int timeout_ms) {
    return impl_->command([enabled](Proto::ControlCommand* request) {
        request->mutable_enable()->set_enabled(enabled);
    }, out, timeout_ms);
}

grpc::Status ArmGatewayClient::stop(bool emergency, CommandAck* out, int timeout_ms) {
    return impl_->command([emergency](Proto::ControlCommand* request) {
        request->mutable_stop()->set_emergency(emergency);
    }, out, timeout_ms);
}

grpc::Status ArmGatewayClient::jog(std::uint32_t joint_index, double step_rad,
                                   double speed_limit_rad_s, CommandAck* out, int timeout_ms) {
    return impl_->command([joint_index, step_rad, speed_limit_rad_s](Proto::ControlCommand* request) {
        auto* value = request->mutable_jog();
        value->set_joint_index(joint_index);
        value->set_step_rad(step_rad);
        value->set_speed_limit_rad_s(speed_limit_rad_s);
    }, out, timeout_ms);
}

grpc::Status ArmGatewayClient::execute_trajectory(const std::vector<TrajectoryPoint>& points,
                                                  bool dry_run, CommandAck* out, int timeout_ms) {
    return impl_->command([&points, dry_run](Proto::ControlCommand* request) {
        auto* value = request->mutable_execute_trajectory();
        value->set_dry_run(dry_run);
        for (const auto& item : points) point_to_proto(item, value->add_points());
    }, out, timeout_ms);
}

grpc::Status ArmGatewayClient::reset_fault(CommandAck* out, int timeout_ms) {
    return impl_->command([](Proto::ControlCommand* request) {
        request->mutable_reset_fault();
    }, out, timeout_ms);
}
grpc::Status ArmGatewayClient::pause(CommandAck* out, int timeout_ms) {
    return impl_->command([](Proto::ControlCommand* request) {
        request->mutable_pause();
    }, out, timeout_ms);
}
grpc::Status ArmGatewayClient::resume(CommandAck* out, int timeout_ms) {
    return impl_->command([](Proto::ControlCommand* request) {
        request->mutable_resume();
    }, out, timeout_ms);
}
grpc::Status ArmGatewayClient::speed_scale(double scale, CommandAck* out, int timeout_ms) {
    return impl_->command([scale](Proto::ControlCommand* request) {
        request->mutable_speed_scale()->set_scale(scale);
    }, out, timeout_ms);
}

grpc::Status ArmGatewayClient::subscribe_telemetry(
    std::uint32_t max_rate_hz, const std::function<bool(const TelemetryFrame&)>& callback,
    int timeout_ms) {
    if (!callback) return grpc::Status(grpc::StatusCode::INVALID_ARGUMENT, "callback is empty");
    auto status = impl_->ensure_session(timeout_ms > 0 ? timeout_ms : 5000);
    if (!status.ok()) return status;
    grpc::ClientContext context;
    deadline(context, timeout_ms);
    impl_->add_metadata(context);
    Proto::TelemetryRequest request;
    request.set_session_id(impl_->connection.session_id);
    request.set_max_rate_hz(max_rate_hz);
    auto reader = impl_->stub->SubscribeTelemetry(&context, request);
    Proto::TelemetryFrame frame;
    bool callback_cancelled = false;
    while (reader->Read(&frame)) {
        if (!callback(frame_from_proto(frame))) {
            callback_cancelled = true;
            context.TryCancel();
            break;
        }
    }
    const auto final_status = reader->Finish();
    if (callback_cancelled && final_status.error_code() == grpc::StatusCode::CANCELLED) {
        return grpc::Status::OK;
    }
    return final_status;
}

const ConnectionInfo* ArmGatewayClient::connection() const noexcept {
    return &impl_->connection;
}

class ArmPlannerClient::Impl {
public:
    explicit Impl(std::shared_ptr<grpc::Channel> channel, Metadata values)
        : stub(Proto::ArmPlanner::NewStub(std::move(channel))), metadata(std::move(values)) {}

    void add_metadata(grpc::ClientContext& context) const {
        for (const auto& item : metadata) context.AddMetadata(item.first, item.second);
    }

    std::unique_ptr<Proto::ArmPlanner::Stub> stub;
    Metadata metadata;
};

ArmPlannerClient::ArmPlannerClient(std::shared_ptr<grpc::Channel> channel, Metadata metadata)
    : impl_(std::make_unique<Impl>(std::move(channel), std::move(metadata))) {}
ArmPlannerClient::~ArmPlannerClient() = default;
ArmPlannerClient::ArmPlannerClient(ArmPlannerClient&&) noexcept = default;
ArmPlannerClient& ArmPlannerClient::operator=(ArmPlannerClient&&) noexcept = default;

grpc::Status ArmPlannerClient::solve_ik(
    const PoseTarget& target, IKResult* out, std::string request_id_value,
    const std::vector<double>& seed_position_rad, bool check_collisions,
    double minimum_distance_threshold_m, std::string assembly_phase_value,
    const std::vector<AllowedCollisionPair>& allowed_collision_pairs, int timeout_ms) {
    if (out == nullptr) return grpc::Status(grpc::StatusCode::INVALID_ARGUMENT, "out is null");
    if (!assembly_phase_value.empty() &&
        assembly_phase(assembly_phase_value) == Proto::ASSEMBLY_PHASE_UNSPECIFIED) {
        return grpc::Status(grpc::StatusCode::INVALID_ARGUMENT, "unknown assembly phase");
    }
    grpc::ClientContext context;
    deadline(context, timeout_ms);
    impl_->add_metadata(context);
    Proto::IKRequest request;
    request.set_request_id(request_id(std::move(request_id_value)));
    pose_to_proto(target, request.mutable_target());
    request.set_check_collisions(check_collisions);
    request.set_minimum_distance_threshold_m(minimum_distance_threshold_m);
    request.set_assembly_phase(assembly_phase(std::move(assembly_phase_value)));
    for (const auto value : seed_position_rad) request.add_seed_position_rad(value);
    collision_pairs_to_proto(allowed_collision_pairs, request.mutable_allowed_collision_pairs());
    Proto::IKResponse reply;
    const auto status = impl_->stub->SolveIK(&context, request, &reply);
    if (!status.ok()) return status;
    out->request_id = reply.request_id();
    out->success = reply.success();
    out->joint_position_rad.assign(reply.joint_position_rad().begin(), reply.joint_position_rad().end());
    out->within_limits = reply.within_limits();
    out->collision = collision_from_proto(reply.collision());
    out->metadata = metadata_from_proto(reply.metadata());
    out->reason = reply.reason();
    return status;
}

grpc::Status ArmPlannerClient::plan_trajectory(
    const PoseTarget& start, const PoseTarget& goal, TrajectoryPlanResult* out,
    std::string request_id_value, std::uint32_t max_rate_hz, bool dry_run,
    bool check_collisions, double minimum_distance_threshold_m,
    std::string assembly_phase_value,
    const std::vector<AllowedCollisionPair>& allowed_collision_pairs, int timeout_ms) {
    if (out == nullptr) return grpc::Status(grpc::StatusCode::INVALID_ARGUMENT, "out is null");
    if (!assembly_phase_value.empty() &&
        assembly_phase(assembly_phase_value) == Proto::ASSEMBLY_PHASE_UNSPECIFIED) {
        return grpc::Status(grpc::StatusCode::INVALID_ARGUMENT, "unknown assembly phase");
    }
    grpc::ClientContext context;
    deadline(context, timeout_ms);
    impl_->add_metadata(context);
    Proto::TrajectoryPlanRequest request;
    request.set_request_id(request_id(std::move(request_id_value)));
    pose_to_proto(start, request.mutable_start());
    pose_to_proto(goal, request.mutable_goal());
    request.set_max_rate_hz(max_rate_hz);
    request.set_dry_run(dry_run);
    request.set_check_collisions(check_collisions);
    request.set_minimum_distance_threshold_m(minimum_distance_threshold_m);
    request.set_assembly_phase(assembly_phase(std::move(assembly_phase_value)));
    collision_pairs_to_proto(allowed_collision_pairs, request.mutable_allowed_collision_pairs());
    Proto::TrajectoryPlanResponse reply;
    const auto status = impl_->stub->PlanTrajectory(&context, request, &reply);
    if (!status.ok()) return status;
    out->request_id = reply.request_id();
    out->success = reply.success();
    out->points.clear();
    for (const auto& item : reply.points()) out->points.push_back(point_from_proto(item));
    out->collision = collision_from_proto(reply.collision());
    out->metadata = metadata_from_proto(reply.metadata());
    out->reason = reply.reason();
    return status;
}

}  // namespace rebot::sdk
