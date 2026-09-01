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

struct SimulationSnapshot {
    std::uint64_t timestamp_ns = 0;
    const char* source = "mock";
    const char* quality = "valid";
    std::array<double, kJointCount> position_rad{};
    std::array<double, kJointCount> velocity_rad_s{};
    std::vector<TransformState> tf;
    std::vector<TrajectoryState> planned_trajectory;
    std::vector<TrajectoryState> actual_trajectory;
};

class SimulationDriver {
public:
    virtual ~SimulationDriver() = default;
    virtual SimulationSnapshot sample(double elapsed_seconds) = 0;
    virtual const char* name() const = 0;
    virtual bool enable(bool enabled, std::string& reason) = 0;
    virtual bool stop(std::string& reason) = 0;
    virtual bool jog(std::size_t joint_index, double step_rad, std::string& reason) = 0;
};

std::unique_ptr<SimulationDriver> make_simulation_driver(const std::string& model_path,
                                                         std::string& error);

std::unique_ptr<SimulationDriver> make_mock_simulation_driver();

#ifdef ARM_CONSOLE_WITH_MUJOCO
std::unique_ptr<SimulationDriver> make_mujoco_simulation_driver(const std::string& model_path,
                                                                std::string& error);
#endif

}  // namespace arm_console
