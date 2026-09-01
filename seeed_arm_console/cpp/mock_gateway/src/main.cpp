#include <array>
#include <atomic>
#include <chrono>
#include <cmath>
#include <csignal>
#include <cstdint>
#include <cstring>
#include <cerrno>
#include <cstdlib>
#include <iostream>
#include <sstream>
#include <string>
#include <thread>

#include "simulation.hpp"

#ifdef _WIN32
#    define NOMINMAX
#    include <winsock2.h>
#    include <ws2tcpip.h>
using socket_t = SOCKET;
constexpr socket_t invalid_socket = INVALID_SOCKET;
#else
#    include <arpa/inet.h>
#    include <fcntl.h>
#    include <netinet/in.h>
#    include <sys/socket.h>
#    include <unistd.h>
using socket_t = int;
constexpr socket_t invalid_socket = -1;
#endif

namespace {

std::atomic_bool running{true};

void stop_handler(int) { running.store(false); }

void close_socket(socket_t socket) {
#ifdef _WIN32
    closesocket(socket);
#else
    close(socket);
#endif
}

bool set_nonblocking(socket_t socket) {
#ifdef _WIN32
    u_long mode = 1;
    return ioctlsocket(socket, FIONBIO, &mode) == 0;
#else
    const int flags = fcntl(socket, F_GETFL, 0);
    return flags >= 0 && fcntl(socket, F_SETFL, flags | O_NONBLOCK) == 0;
#endif
}

bool would_block() {
#ifdef _WIN32
    const int error = WSAGetLastError();
    return error == WSAEWOULDBLOCK || error == WSAETIMEDOUT;
#else
    return errno == EAGAIN || errno == EWOULDBLOCK;
#endif
}

bool send_all(socket_t socket, const std::string& payload) {
    std::size_t sent = 0;
    while (sent < payload.size() && running.load()) {
        const auto count = send(socket, payload.data() + sent,
                                static_cast<int>(payload.size() - sent), 0);
        if (count <= 0) {
            return false;
        }
        sent += static_cast<std::size_t>(count);
    }
    return sent == payload.size();
}

void append_joint_array(std::ostringstream& json,
                        const std::array<double, arm_console::kJointCount>& values) {
    json << '[';
    for (std::size_t index = 0; index < values.size(); ++index) {
        if (index != 0) json << ',';
        json << values[index];
    }
    json << ']';
}

void append_transform(std::ostringstream& json, bool& first,
                      const arm_console::TransformState& transform) {
    if (!first) json << ',';
    first = false;
    json << "{\"parent\":\"" << transform.parent << "\",\"child\":\""
         << transform.child << "\",\"translation_x_m\":" << transform.translation_m[0]
         << ",\"translation_y_m\":" << transform.translation_m[1]
         << ",\"translation_z_m\":" << transform.translation_m[2]
         << ",\"rotation_x\":" << transform.rotation_xyzw[0]
         << ",\"rotation_y\":" << transform.rotation_xyzw[1]
         << ",\"rotation_z\":" << transform.rotation_xyzw[2]
         << ",\"rotation_w\":" << transform.rotation_xyzw[3] << '}';
}

bool json_number(const std::string& line, const char* key, double& value) {
    const std::string marker = std::string("\"") + key + "\":";
    const auto start = line.find(marker);
    if (start == std::string::npos) return false;
    char* end = nullptr;
    value = std::strtod(line.c_str() + start + marker.size(), &end);
    return end != line.c_str() + start + marker.size();
}

bool handle_command(const std::string& line, arm_console::SimulationDriver& driver,
                    socket_t client) {
    std::string reason;
    bool accepted = false;
    if (line.find("\"type\":\"enable\"") != std::string::npos) {
        const bool enabled = line.find("\"enabled\":true") != std::string::npos;
        accepted = driver.enable(enabled, reason);
    } else if (line.find("\"type\":\"stop\"") != std::string::npos) {
        accepted = driver.stop(reason);
    } else if (line.find("\"type\":\"jog\"") != std::string::npos) {
        double joint = 0.0;
        double step = 0.0;
        if (json_number(line, "joint_index", joint) && json_number(line, "step_rad", step)) {
            accepted = driver.jog(static_cast<std::size_t>(joint), step, reason);
        } else {
            reason = "jog requires joint_index and step_rad";
        }
    } else {
        reason = "unknown command type";
    }

    std::ostringstream ack;
    ack << "{\"type\":\"ack\",\"status\":\""
        << (accepted ? "accepted" : "rejected") << "\",\"reason\":\"" << reason
        << "\"}\n";
    return send_all(client, ack.str());
}

std::string telemetry_json(std::uint64_t sequence,
                            const arm_console::SimulationSnapshot& snapshot) {
    std::ostringstream json;
    json.setf(std::ios::fixed);
    json.precision(6);
    json << "{\"sequence\":" << sequence
         << ",\"timestamp_ns\":"
         << snapshot.timestamp_ns
         << ",\"source\":\"" << snapshot.source << "\",\"quality\":\""
         << snapshot.quality << "\""
         << ",\"joint_position_rad\":";
    append_joint_array(json, snapshot.position_rad);
    json << ",\"joint_velocity_rad_s\":";
    append_joint_array(json, snapshot.velocity_rad_s);

    json << ",\"tf\":[";
    bool first_transform = true;
    for (const auto& transform : snapshot.tf) {
        append_transform(json, first_transform, transform);
    }
    json << ']';

    auto append_trajectory = [&json](const std::vector<arm_console::TrajectoryState>& trajectory) {
        json << '[';
        for (std::size_t index = 0; index < trajectory.size(); ++index) {
            if (index != 0) json << ',';
            json << "{\"time_from_start_ns\":" << trajectory[index].time_from_start_ns
                 << ",\"position_rad\":";
            append_joint_array(json, trajectory[index].position_rad);
            json << ",\"velocity_rad_s\":";
            append_joint_array(json, trajectory[index].velocity_rad_s);
            json << '}';
        }
        json << ']';
    };
    json << ",\"planned_trajectory\":";
    append_trajectory(snapshot.planned_trajectory);
    json << ",\"actual_trajectory\":";
    append_trajectory(snapshot.actual_trajectory);
    json << '}';
    json << '\n';
    return json.str();
}

bool drain_commands(socket_t client, arm_console::SimulationDriver& driver,
                    std::string& pending) {
    char buffer[1024];
    for (;;) {
        const auto count = recv(client, buffer, sizeof(buffer) - 1, 0);
        if (count > 0) {
            buffer[count] = '\0';
            pending.append(buffer, static_cast<std::size_t>(count));
            std::size_t newline = 0;
            while ((newline = pending.find('\n')) != std::string::npos) {
                std::string line = pending.substr(0, newline);
                pending.erase(0, newline + 1);
                if (!line.empty() && !handle_command(line, driver, client)) return false;
            }
            continue;
        }
        if (count == 0) return false;
        if (!would_block()) return false;
        return true;
    }
}

socket_t make_server(std::uint16_t port) {
    const socket_t server = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (server == invalid_socket) return invalid_socket;

    int reuse = 1;
    setsockopt(server, SOL_SOCKET, SO_REUSEADDR,
               reinterpret_cast<const char*>(&reuse), sizeof(reuse));

    sockaddr_in address{};
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = htonl(INADDR_ANY);
    address.sin_port = htons(port);
    if (bind(server, reinterpret_cast<const sockaddr*>(&address), sizeof(address)) != 0 ||
        listen(server, 1) != 0) {
        close_socket(server);
        return invalid_socket;
    }
    return server;
}

}  // namespace

int main(int argc, char** argv) {
#ifdef _WIN32
    WSADATA winsock{};
    if (WSAStartup(MAKEWORD(2, 2), &winsock) != 0) {
        std::cerr << "WSAStartup failed\n";
        return 1;
    }
#endif

    std::uint16_t port = 50051;
    if (argc > 1) {
        const auto parsed = std::stoi(argv[1]);
        if (parsed < 1 || parsed > 65535) {
            std::cerr << "port must be in range 1..65535\n";
            return 2;
        }
        port = static_cast<std::uint16_t>(parsed);
    }

    std::string model_path;
    if (argc > 2) {
        model_path = argv[2];
    } else if (const char* configured_model = std::getenv("ARM_CONSOLE_MODEL")) {
        model_path = configured_model;
    }
    std::string driver_error;
    auto driver = arm_console::make_simulation_driver(model_path, driver_error);
    if (!driver) {
        std::cerr << "simulation driver unavailable: " << driver_error << "\n";
        return 4;
    }

    std::signal(SIGINT, stop_handler);
    std::signal(SIGTERM, stop_handler);
#ifndef _WIN32
    std::signal(SIGPIPE, SIG_IGN);
#endif
    const socket_t server = make_server(port);
    if (server == invalid_socket) {
        std::cerr << "cannot listen on 0.0.0.0:" << port << "\n";
#ifdef _WIN32
        WSACleanup();
#endif
        return 3;
    }

    std::cout << "arm_console_mock_gateway listening on 127.0.0.1:" << port
              << " using " << driver->name() << " driver (newline-delimited JSON)"
              << std::endl;
    while (running.load()) {
        sockaddr_in peer{};
#ifdef _WIN32
        int peer_size = sizeof(peer);
#else
        socklen_t peer_size = sizeof(peer);
#endif
        const socket_t client = accept(server, reinterpret_cast<sockaddr*>(&peer), &peer_size);
        if (client == invalid_socket) {
            if (running.load()) std::this_thread::sleep_for(std::chrono::milliseconds(50));
            continue;
        }
        const bool nonblocking = set_nonblocking(client);
        std::cout << "client connected" << std::endl;
        const auto started = std::chrono::steady_clock::now();
        std::uint64_t sequence = 0;
        std::string pending_commands;
        while (running.load()) {
            const auto now = std::chrono::steady_clock::now();
            const double elapsed = std::chrono::duration<double>(now - started).count();
            const auto snapshot = driver->sample(elapsed);
            if (!send_all(client, telemetry_json(++sequence, snapshot))) break;
            if (nonblocking && !drain_commands(client, *driver, pending_commands)) break;
            std::this_thread::sleep_for(std::chrono::milliseconds(20));
        }
        close_socket(client);
        if (running.load()) std::cout << "client disconnected" << std::endl;
    }

    close_socket(server);
#ifdef _WIN32
    WSACleanup();
#endif
    return 0;
}
