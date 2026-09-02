#include "rebot_sdk/client.hpp"

#include <cstdlib>
#include <iostream>
#include <memory>
#include <vector>

int main(int argc, char** argv) {
    const std::string gateway_address = argc > 1 ? argv[1] : "127.0.0.1:50051";
    auto channel = grpc::CreateChannel(gateway_address, grpc::InsecureChannelCredentials());
    rebot::sdk::ArmGatewayClient gateway(std::move(channel), "cpp-sdk-example");

    rebot::sdk::ConnectionInfo connection;
    auto status = gateway.handshake(&connection);
    if (!status.ok()) {
        std::cerr << "handshake failed: " << status.error_message() << '\n';
        return EXIT_FAILURE;
    }
    std::cout << "connected source=" << connection.source << " dof=" << connection.dof
              << " session=" << connection.session_id << '\n';

    rebot::sdk::CommandAck ack;
    status = gateway.enable(true, &ack);
    if (!status.ok() || !ack.accepted()) {
        std::cerr << "enable rejected: " << (status.ok() ? ack.reason : status.error_message()) << '\n';
        return EXIT_FAILURE;
    }
    status = gateway.subscribe_telemetry(
        10,
        [](const rebot::sdk::TelemetryFrame& frame) {
            std::cout << "telemetry sequence=" << frame.sequence
                      << " joints=" << frame.joint_position_rad.size() << '\n';
            return false;
        },
        5000);
    if (!status.ok() && status.error_code() != grpc::StatusCode::CANCELLED) {
        std::cerr << "telemetry failed: " << status.error_message() << '\n';
        return EXIT_FAILURE;
    }
    status = gateway.stop(false, &ack);
    if (!status.ok() || !ack.accepted()) {
        std::cerr << "stop rejected: " << (status.ok() ? ack.reason : status.error_message()) << '\n';
        return EXIT_FAILURE;
    }
    status = gateway.reset_fault(&ack);
    if (!status.ok() || !ack.accepted()) {
        std::cerr << "reset_fault rejected: " << (status.ok() ? ack.reason : status.error_message()) << '\n';
        return EXIT_FAILURE;
    }
    std::cout << "cpp_sdk=OK\n";
    return EXIT_SUCCESS;
}
