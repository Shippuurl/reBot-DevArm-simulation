#pragma once

#include <array>
#include <cstdint>
#include <memory>
#include <string>
#include <vector>

namespace arm_console {

constexpr std::size_t kJointCount = 6;

struct TransformState {
    std::string parent;
    std::string child;
    std::array<double, 3> translation_m{};
    std::array<double, 4> rotation_xyzw{0.0, 0.0, 0.0, 1.0};
};

struct TrajectoryState {
    std::uint64_t time_from_start_ns = 0;
    std::array<double, kJointCount> position_rad{};
    std::array<double, kJointCount> velocity_rad_s{};
};

struct ContactState {
    std::string first_geom;
    std::string second_geom;
    double distance_m = 0.0;
    double normal_force_n = 0.0;
};

// Depth-camera output represented as XYZ points in world coordinates.  The
// gateway keeps this structure independent from protobuf so the same sensor
// sample can be exposed through gRPC and the legacy JSON adapter.
struct PointCloudState {
    std::string sensor;
    std::vector<std::array<float, 3>> positions_xyz;
    std::vector<std::uint32_t> colors_rgba;
};

struct SimulationSnapshot {
    std::uint64_t timestamp_ns = 0;
    const char* source = "mock";
    const char* quality = "valid";
    std::array<double, kJointCount> position_rad{};
    std::array<double, kJointCount> velocity_rad_s{};
    std::vector<TransformState> tf;
    std::vector<TrajectoryState> planned_trajectory;
    std::vector<TrajectoryState> actual_trajectory;
    std::vector<ContactState> contacts;
    std::vector<PointCloudState> point_clouds;
};

class SimulationDriver {
public:
    virtual ~SimulationDriver() = default;
    virtual SimulationSnapshot sample(double elapsed_seconds) = 0;
    virtual const char* name() const = 0;
    virtual bool enable(bool enabled, std::string& reason) = 0;
    virtual bool stop(std::string& reason) = 0;
    virtual bool jog(std::size_t joint_index, double step_rad, std::string& reason) = 0;
    // Submit a validated trajectory to the simulation execution adapter. A
    // dry-run validates the command without changing driver state; a normal
    // submission is sampled kinematically by the next telemetry frames.
    virtual bool execute_trajectory(const std::vector<TrajectoryState>& points,
                                    bool dry_run, std::string& reason) = 0;
    virtual bool reset_fault(std::string& reason) = 0;
    virtual bool pause(std::string& reason) = 0;
    virtual bool resume(std::string& reason) = 0;
    virtual bool set_speed_scale(double scale, std::string& reason) = 0;
};

std::unique_ptr<SimulationDriver> make_simulation_driver(const std::string& model_path,
                                                         std::string& error);

std::unique_ptr<SimulationDriver> make_mock_simulation_driver();

#ifdef ARM_CONSOLE_WITH_MUJOCO
std::unique_ptr<SimulationDriver> make_mujoco_simulation_driver(const std::string& model_path,
                                                                std::string& error);
#endif

}  // namespace arm_console
