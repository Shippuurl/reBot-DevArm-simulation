#pragma once

#ifdef ARM_CONSOLE_WITH_GRPC

#include "arm_console.grpc.pb.h"
#include "simulation.hpp"

#include <grpcpp/grpcpp.h>

#include <chrono>
#include <cstdint>
#include <atomic>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <unordered_map>

namespace arm_console::gateway {

// gRPC is the canonical gateway transport. The legacy newline-delimited JSON
// adapter in main.cpp is intentionally kept on a separate port while clients
// migrate.
class ArmGatewayService final : public ::arm::console::v1::ArmGateway::Service {
public:
    ArmGatewayService(SimulationDriver& driver, std::mutex& driver_mutex);

    ::grpc::Status Handshake(::grpc::ServerContext* context,
                             const ::arm::console::v1::ConnectRequest* request,
                             ::arm::console::v1::ConnectReply* reply) override;
    ::grpc::Status Command(::grpc::ServerContext* context,
                           const ::arm::console::v1::ControlCommand* request,
                           ::arm::console::v1::CommandAck* reply) override;
    ::grpc::Status SubscribeTelemetry(
        ::grpc::ServerContext* context,
        const ::arm::console::v1::TelemetryRequest* request,
        ::grpc::ServerWriter<::arm::console::v1::TelemetryFrame>* writer) override;

private:
    using SessionClock = std::chrono::steady_clock;

    std::string create_session();
    bool touch_session(const std::string& session_id);
    void prune_sessions_locked(SessionClock::time_point now);

    SimulationDriver& driver_;
    std::mutex& driver_mutex_;
    std::mutex session_mutex_;
    std::unordered_map<std::string, SessionClock::time_point> sessions_;
    std::uint64_t next_session_id_ = 0;
    std::atomic<std::uint64_t> sequence_{0};
    std::chrono::steady_clock::time_point started_;
};

class ServerHandle {
public:
    ServerHandle() = default;
    ~ServerHandle();
    ServerHandle(const ServerHandle&) = delete;
    ServerHandle& operator=(const ServerHandle&) = delete;

    void shutdown();

private:
    friend std::unique_ptr<ServerHandle> start_server(ArmGatewayService&,
                                                       std::uint16_t,
                                                       std::string&);
    std::unique_ptr<::grpc::Server> server_;
    std::thread wait_thread_;
};

std::unique_ptr<ServerHandle> start_server(ArmGatewayService& service,
                                           std::uint16_t port,
                                           std::string& error);

}  // namespace arm_console::gateway

#endif  // ARM_CONSOLE_WITH_GRPC
