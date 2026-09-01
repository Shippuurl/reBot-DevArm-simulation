#include "simulation.hpp"

#include <cmath>

namespace arm_console {
namespace {

class MockSimulationDriver final : public SimulationDriver {
public:
    SimulationSnapshot sample(double elapsed_seconds) override {
        SimulationSnapshot snapshot;
        snapshot.timestamp_ns = static_cast<std::uint64_t>(elapsed_seconds * 1'000'000'000.0);

        for (std::size_t index = 0; index < kJointCount; ++index) {
            const double phase = elapsed_seconds * 1.4 + static_cast<double>(index) * 0.31;
            snapshot.position_rad[index] = std::sin(phase) * 0.24 + offsets_[index];
            snapshot.velocity_rad_s[index] = (!enabled_ || stopped_) ? 0.0 :
                                                                      std::cos(phase) * 0.336;
        }

        snapshot.tf.push_back({"world", "base", {0.0, 0.0, 0.0}, {0.0, 0.0, 0.0, 1.0}});
        const std::array<double, kJointCount> link_lengths = {0.08, 0.11, 0.10,
                                                               0.07, 0.05, 0.04};
        double angle = 0.0;
        for (std::size_t index = 0; index < kJointCount; ++index) {
            angle += snapshot.position_rad[index];
            const std::string parent = index == 0 ? "base" : "link" + std::to_string(index);
            const std::string child = "link" + std::to_string(index + 1);
            snapshot.tf.push_back({parent,
                                   child,
                                   {std::cos(angle) * link_lengths[index],
                                    std::sin(angle) * link_lengths[index],
                                    0.08 + static_cast<double>(index) * 0.01},
                                   {0.0, 0.0, 0.0, 1.0}});
        }
        snapshot.tf.push_back({"link6", "tool", {0.0, 0.0, 0.02}, {0.0, 0.0, 0.0, 1.0}});
        snapshot.tf.push_back({"tool", "gripper_left", {-0.041939, -0.0000734, 0.0},
                               {0.5, -0.5, 0.5000018, 0.4999982}});
        snapshot.tf.push_back({"tool", "gripper_right", {-0.041939, 0.0000734, 0.0},
                               {-0.5, -0.5, -0.5000018, 0.4999982}});

        TrajectoryState actual;
        actual.position_rad = snapshot.position_rad;
        actual.velocity_rad_s = snapshot.velocity_rad_s;
        snapshot.actual_trajectory.push_back(actual);

        TrajectoryState planned = actual;
        planned.time_from_start_ns = 200'000'000;
        for (std::size_t index = 0; index < kJointCount; ++index) {
            planned.position_rad[index] += planned.velocity_rad_s[index] * 0.2;
        }
        snapshot.planned_trajectory.push_back(planned);
        return snapshot;
    }

    const char* name() const override { return "mock"; }

    bool enable(bool enabled, std::string& reason) override {
        enabled_ = enabled;
        stopped_ = false;
        reason = enabled ? "enabled" : "disabled";
        return true;
    }

    bool stop(std::string& reason) override {
        stopped_ = true;
        reason = "stopped";
        return true;
    }

    bool jog(std::size_t joint_index, double step_rad, std::string& reason) override {
        if (joint_index >= kJointCount) {
            reason = "joint_index out of range";
            return false;
        }
        offsets_[joint_index] += step_rad;
        reason = "jog accepted";
        return true;
    }

private:
    std::array<double, kJointCount> offsets_{};
    bool enabled_ = true;
    bool stopped_ = false;
};

}  // namespace

std::unique_ptr<SimulationDriver> make_mock_simulation_driver() {
    return std::make_unique<MockSimulationDriver>();
}

}  // namespace arm_console
