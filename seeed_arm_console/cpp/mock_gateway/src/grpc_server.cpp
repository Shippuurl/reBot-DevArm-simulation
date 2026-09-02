#include "grpc_server.hpp"

#ifdef ARM_CONSOLE_WITH_GRPC

#include <algorithm>
#include <cmath>
#include <cstdlib>
#include <chrono>
#include <cstdint>
#include <thread>

namespace arm_console::gateway {
namespace {

using arm::console::v1::AckStatus;
using arm::console::v1::SampleQuality;
using arm::console::v1::SourceKind;

SourceKind source_kind(const char* source) {
    if (source != nullptr && std::string(source) == "mujoco") return SourceKind::MUJOCO;
    if (source != nullptr && std::string(source) == "ros2") return SourceKind::ROS2;
    if (source != nullptr && std::string(source) == "driver") return SourceKind::DRIVER;
    return SourceKind::MOCK;
}

SampleQuality sample_quality(const char* quality) {
    if (quality != nullptr && std::string(quality) == "stale") return SampleQuality::STALE;
    if (quality != nullptr && std::string(quality) == "limited") return SampleQuality::LIMITED;
    if (quality != nullptr && std::string(quality) == "fault") return SampleQuality::FAULT;
    return SampleQuality::VALID;
}

void append_trajectory(
    const std::vector<TrajectoryState>& source,
    google::protobuf::RepeatedPtrField<arm::console::v1::TrajectoryPoint>* out) {
    for (const auto& item : source) {
        auto* point = out->Add();
        point->set_time_from_start_ns(item.time_from_start_ns);
        for (const double value : item.position_rad) point->add_position_rad(value);
        for (const double value : item.velocity_rad_s) point->add_velocity_rad_s(value);
    }
}

void fill_frame(const SimulationSnapshot& snapshot, std::uint64_t sequence,
                arm::console::v1::TelemetryFrame& frame) {
    frame.set_sequence(sequence);
    frame.set_timestamp_ns(snapshot.timestamp_ns);
    frame.set_sim_time_ns(snapshot.timestamp_ns);
    frame.set_wall_time_ns(static_cast<std::uint64_t>(
        std::chrono::duration_cast<std::chrono::nanoseconds>(
            std::chrono::system_clock::now().time_since_epoch()).count()));
    frame.set_source(source_kind(snapshot.source));
    frame.set_quality(sample_quality(snapshot.quality));
    for (const double value : snapshot.position_rad) frame.add_joint_position_rad(value);
    for (const double value : snapshot.velocity_rad_s) frame.add_joint_velocity_rad_s(value);
    for (const auto& item : snapshot.tf) {
        auto* tf = frame.add_tf();
        tf->set_parent(item.parent);
        tf->set_child(item.child);
        tf->set_translation_x_m(item.translation_m[0]);
        tf->set_translation_y_m(item.translation_m[1]);
        tf->set_translation_z_m(item.translation_m[2]);
        tf->set_rotation_x(item.rotation_xyzw[0]);
        tf->set_rotation_y(item.rotation_xyzw[1]);
        tf->set_rotation_z(item.rotation_xyzw[2]);
        tf->set_rotation_w(item.rotation_xyzw[3]);
    }
    append_trajectory(snapshot.planned_trajectory, frame.mutable_planned_trajectory());
    append_trajectory(snapshot.actual_trajectory, frame.mutable_actual_trajectory());
    for (const auto& item : snapshot.contacts) {
        auto* contact = frame.add_contacts();
        contact->set_first_geom(item.first_geom);
        contact->set_second_geom(item.second_geom);
        contact->set_distance_m(item.distance_m);
        contact->set_normal_force_n(item.normal_force_n);
    }
    for (const auto& cloud : snapshot.point_clouds) {
        auto* output = frame.add_point_clouds();
        output->set_sensor(cloud.sensor);
        for (const auto& point : cloud.positions_xyz) {
            output->add_positions_xyz(point[0]);
            output->add_positions_xyz(point[1]);
            output->add_positions_xyz(point[2]);
        }
        for (const auto color : cloud.colors_rgba) output->add_colors_rgba(color);
    }
}

std::string command_id(const arm::console::v1::ControlCommand& request) {
    return request.has_header() ? request.header().command_id() : std::string{};
}

bool command_timestamp_valid(const arm::console::v1::ControlCommand& request,
                             std::string& reason) {
    if (!request.has_header() || request.header().client_timestamp_ns() == 0) {
        // Zero is the explicitly documented "unspecified" value retained
        // for local diagnostic clients and older adapters.
        return true;
    }
    constexpr std::uint64_t kMaxAgeNs = 5'000'000'000ULL;
    constexpr std::uint64_t kMaxFutureSkewNs = 1'000'000'000ULL;
    const auto now = std::chrono::duration_cast<std::chrono::nanoseconds>(
        std::chrono::system_clock::now().time_since_epoch()).count();
    const auto timestamp = request.header().client_timestamp_ns();
    const auto now_u64 = static_cast<std::uint64_t>(now);
    if (now > 0 && timestamp < now_u64 && now_u64 - timestamp > kMaxAgeNs) {
        reason = "command timestamp is stale (older than 5 seconds)";
        return false;
    }
    if (now > 0 && timestamp > now_u64 && timestamp - now_u64 > kMaxFutureSkewNs) {
        reason = "command timestamp is too far in the future";
        return false;
    }
    return true;
}

}  // namespace

ArmGatewayService::ArmGatewayService(SimulationDriver& driver, std::mutex& driver_mutex)
    : driver_(driver), driver_mutex_(driver_mutex),
      started_(std::chrono::steady_clock::now()) {}

std::string ArmGatewayService::create_session() {
    // Sessions are intentionally opaque to clients. The monotonically
    // increasing suffix prevents collisions without introducing a UUID
    // dependency into the small simulation gateway; production deployments
    // should additionally bind the session to an authenticated identity.
    const auto now = SessionClock::now();
    const auto wall_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(
        std::chrono::system_clock::now().time_since_epoch()).count();
    std::lock_guard lock(session_mutex_);
    prune_sessions_locked(now);
    const auto id = "arm-console-" + std::to_string(++next_session_id_) + "-" +
                    std::to_string(static_cast<std::uint64_t>(wall_ns));
    sessions_[id] = now;
    return id;
}

bool ArmGatewayService::touch_session(const std::string& session_id) {
    if (session_id.empty()) return false;
    const auto now = SessionClock::now();
    std::lock_guard lock(session_mutex_);
    prune_sessions_locked(now);
    const auto found = sessions_.find(session_id);
    if (found == sessions_.end()) return false;
    found->second = now;
    return true;
}

void ArmGatewayService::prune_sessions_locked(SessionClock::time_point now) {
    constexpr auto kSessionTtl = std::chrono::hours(1);
    constexpr std::size_t kMaxSessions = 1024;
    for (auto item = sessions_.begin(); item != sessions_.end();) {
        if (now - item->second > kSessionTtl) {
            item = sessions_.erase(item);
        } else {
            ++item;
        }
    }
    while (sessions_.size() > kMaxSessions) {
        const auto oldest = std::min_element(
            sessions_.begin(), sessions_.end(),
            [](const auto& left, const auto& right) { return left.second < right.second; });
        sessions_.erase(oldest);
    }
}

::grpc::Status ArmGatewayService::Handshake(
    ::grpc::ServerContext*, const arm::console::v1::ConnectRequest* request,
    arm::console::v1::ConnectReply* reply) {
    if (request == nullptr || reply == nullptr) {
        return ::grpc::Status(::grpc::StatusCode::INVALID_ARGUMENT, "missing handshake message");
    }
    if (!request->protocol_version().empty() && request->protocol_version() != "arm.console.v1") {
        return ::grpc::Status(::grpc::StatusCode::UNIMPLEMENTED, "unsupported protocol_version");
    }
    const auto session_id = create_session();
    reply->set_session_id(session_id);
    reply->set_protocol_version("arm.console.v1");
    reply->set_source(source_kind(driver_.name()));
    reply->set_dof(static_cast<std::uint32_t>(kJointCount));
    return ::grpc::Status::OK;
}

::grpc::Status ArmGatewayService::Command(
    ::grpc::ServerContext*, const arm::console::v1::ControlCommand* request,
    arm::console::v1::CommandAck* reply) {
    if (request == nullptr || reply == nullptr) {
        return ::grpc::Status(::grpc::StatusCode::INVALID_ARGUMENT, "missing command message");
    }
    if (!request->has_header() || request->header().session_id().empty()) {
        return ::grpc::Status(::grpc::StatusCode::UNAUTHENTICATED, "command session_id is required");
    }
    if (!touch_session(request->header().session_id())) {
        return ::grpc::Status(::grpc::StatusCode::UNAUTHENTICATED, "unknown session_id");
    }
    reply->set_command_id(command_id(*request));
    std::string reason;
    bool accepted = false;
    if (!command_timestamp_valid(*request, reason)) {
        reply->set_status(AckStatus::REJECTED);
        reply->set_reason(reason);
        return ::grpc::Status::OK;
    }
    {
        std::lock_guard lock(driver_mutex_);
        switch (request->payload_case()) {
            case arm::console::v1::ControlCommand::kEnable:
                accepted = driver_.enable(request->enable().enabled(), reason);
                break;
            case arm::console::v1::ControlCommand::kStop:
                accepted = driver_.stop(reason);
                break;
            case arm::console::v1::ControlCommand::kJog:
                if (!std::isfinite(request->jog().step_rad()) ||
                    std::abs(request->jog().step_rad()) > 0.5) {
                    reason = "jog step_rad must be finite and within +/-0.5 rad";
                } else {
                    accepted = driver_.jog(request->jog().joint_index(), request->jog().step_rad(), reason);
                }
                break;
            case arm::console::v1::ControlCommand::kExecuteTrajectory:
                {
                    constexpr std::size_t kMaxTrajectoryPoints = 2000;
                    const auto& command = request->execute_trajectory();
                    if (command.points().empty()) {
                        reason = "trajectory must contain at least one point";
                    } else if (command.points_size() > static_cast<int>(kMaxTrajectoryPoints)) {
                        reason = "trajectory exceeds the 2000-point safety limit";
                    } else {
                        std::vector<TrajectoryState> points;
                        points.reserve(command.points_size());
                        std::uint64_t previous_time = 0;
                        bool valid = true;
                        for (const auto& point : command.points()) {
                            if (point.position_rad_size() != static_cast<int>(kJointCount) ||
                                (point.velocity_rad_s_size() != 0 &&
                                 point.velocity_rad_s_size() != static_cast<int>(kJointCount)) ||
                                point.time_from_start_ns() < previous_time) {
                                valid = false;
                                break;
                            }
                            previous_time = point.time_from_start_ns();
                            TrajectoryState converted;
                            converted.time_from_start_ns = point.time_from_start_ns();
                            for (std::size_t index = 0; index < kJointCount; ++index) {
                                converted.position_rad[index] = point.position_rad(static_cast<int>(index));
                                converted.velocity_rad_s[index] = point.velocity_rad_s_size() == 0
                                                                       ? 0.0
                                                                       : point.velocity_rad_s(static_cast<int>(index));
                                if (!std::isfinite(converted.position_rad[index]) ||
                                    !std::isfinite(converted.velocity_rad_s[index])) {
                                    valid = false;
                                    break;
                                }
                            }
                            if (!valid) break;
                            points.push_back(converted);
                        }
                        if (!valid) {
                            reason = "trajectory requires 6 joints, finite values and monotonic timestamps";
                        } else {
                            accepted = driver_.execute_trajectory(points, command.dry_run(), reason);
                        }
                    }
                }
                break;
            case arm::console::v1::ControlCommand::kResetFault:
                accepted = driver_.reset_fault(reason);
                break;
            case arm::console::v1::ControlCommand::kPause:
                accepted = driver_.pause(reason);
                break;
            case arm::console::v1::ControlCommand::kResume:
                accepted = driver_.resume(reason);
                break;
            case arm::console::v1::ControlCommand::kSpeedScale:
                accepted = driver_.set_speed_scale(request->speed_scale().scale(), reason);
                break;
            case arm::console::v1::ControlCommand::PAYLOAD_NOT_SET:
                reason = "command payload is required";
                break;
        }
    }
    reply->set_status(accepted ? AckStatus::ACCEPTED : AckStatus::REJECTED);
    reply->set_reason(reason);
    return ::grpc::Status::OK;
}

::grpc::Status ArmGatewayService::SubscribeTelemetry(
    ::grpc::ServerContext* context, const arm::console::v1::TelemetryRequest* request,
    ::grpc::ServerWriter<arm::console::v1::TelemetryFrame>* writer) {
    if (request == nullptr || writer == nullptr) {
        return ::grpc::Status(::grpc::StatusCode::INVALID_ARGUMENT, "missing telemetry message");
    }
    if (request->session_id().empty()) {
        return ::grpc::Status(::grpc::StatusCode::UNAUTHENTICATED, "telemetry session_id is required");
    }
    if (!touch_session(request->session_id())) {
        return ::grpc::Status(::grpc::StatusCode::UNAUTHENTICATED, "unknown session_id");
    }
    const auto rate_hz = std::clamp(request->max_rate_hz() == 0 ? 50u : request->max_rate_hz(), 1u, 200u);
    const auto period = std::chrono::microseconds(1'000'000 / rate_hz);
    while (!context->IsCancelled()) {
        if (!touch_session(request->session_id())) {
            return ::grpc::Status(::grpc::StatusCode::UNAUTHENTICATED, "session expired");
        }
        SimulationSnapshot snapshot;
        {
            std::lock_guard lock(driver_mutex_);
            const auto elapsed = std::chrono::duration<double>(
                std::chrono::steady_clock::now() - started_).count();
            snapshot = driver_.sample(elapsed);
        }
        arm::console::v1::TelemetryFrame frame;
        const auto sequence = ++sequence_;
        fill_frame(snapshot, sequence, frame);
        if (!writer->Write(frame)) break;
        std::this_thread::sleep_for(period);
    }
    return ::grpc::Status::OK;
}

ServerHandle::~ServerHandle() { shutdown(); }

void ServerHandle::shutdown() {
    if (server_) server_->Shutdown();
    if (wait_thread_.joinable()) wait_thread_.join();
    server_.reset();
}

std::unique_ptr<ServerHandle> start_server(ArmGatewayService& service,
                                           std::uint16_t port, std::string& error) {
    auto handle = std::make_unique<ServerHandle>();
    ::grpc::ServerBuilder builder;
    const char* configured_host = std::getenv("ARM_CONSOLE_GRPC_BIND_ADDRESS");
    const std::string host = configured_host == nullptr || configured_host[0] == '\0'
                                 ? "127.0.0.1" : configured_host;
    builder.AddListeningPort(host + ":" + std::to_string(port),
                             ::grpc::InsecureServerCredentials());
    builder.RegisterService(&service);
    handle->server_ = builder.BuildAndStart();
    if (!handle->server_) {
        error = "gRPC server failed to bind " + host + ":" + std::to_string(port);
        return nullptr;
    }
    auto* server = handle->server_.get();
    handle->wait_thread_ = std::thread([server] { server->Wait(); });
    return handle;
}

}  // namespace arm_console::gateway

#endif  // ARM_CONSOLE_WITH_GRPC
