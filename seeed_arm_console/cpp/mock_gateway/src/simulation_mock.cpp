#include "simulation.hpp"

#include <algorithm>
#include <cmath>

namespace arm_console {
namespace {

class MockSimulationDriver final : public SimulationDriver {
public:
    SimulationSnapshot sample(double elapsed_seconds) override {
        SimulationSnapshot snapshot;
        snapshot.timestamp_ns = static_cast<std::uint64_t>(elapsed_seconds * 1'000'000'000.0);

        const double delta_seconds = (last_sample_elapsed_ < 0.0 || skip_delta_after_resume_)
                                         ? 0.0
                                         : std::max(0.0, elapsed_seconds - last_sample_elapsed_);
        skip_delta_after_resume_ = false;
        last_sample_elapsed_ = elapsed_seconds;
        if (trajectory_active_ && !paused_) {
            trajectory_elapsed_seconds_ += delta_seconds * speed_scale_;
        }

        for (std::size_t index = 0; index < kJointCount; ++index) {
            const double phase = elapsed_seconds * 1.4 + static_cast<double>(index) * 0.31;
            snapshot.position_rad[index] = std::sin(phase) * 0.24 + offsets_[index];
            snapshot.velocity_rad_s[index] = (!enabled_ || stopped_) ? 0.0 :
                                                                      std::cos(phase) * 0.336;
        }

        if (!last_trajectory_.empty()) {
            sample_trajectory(snapshot.position_rad, snapshot.velocity_rad_s);
        }
        if (paused_) {
            if (have_sample_) snapshot.position_rad = held_position_;
            snapshot.velocity_rad_s.fill(0.0);
        }
        held_position_ = snapshot.position_rad;
        have_sample_ = true;

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
        actual.time_from_start_ns = static_cast<std::uint64_t>(
            std::max(0.0, trajectory_elapsed_seconds_) * 1'000'000'000.0);
        snapshot.actual_trajectory.push_back(actual);

        if (!last_trajectory_.empty()) {
            snapshot.planned_trajectory = last_trajectory_;
        } else {
            TrajectoryState planned = actual;
            planned.time_from_start_ns = 200'000'000;
            for (std::size_t index = 0; index < kJointCount; ++index) {
                planned.position_rad[index] += planned.velocity_rad_s[index] * 0.2;
            }
            snapshot.planned_trajectory.push_back(planned);
        }
        return snapshot;
    }

    const char* name() const override { return "mock"; }

    bool enable(bool enabled, std::string& reason) override {
        enabled_ = enabled;
        stopped_ = false;
        paused_ = false;
        skip_delta_after_resume_ = true;
        if (!enabled_) {
            trajectory_active_ = false;
            last_trajectory_.clear();
        }
        reason = enabled ? "enabled" : "disabled";
        return true;
    }

    bool stop(std::string& reason) override {
        stopped_ = true;
        trajectory_active_ = false;
        paused_ = false;
        last_trajectory_.clear();
        reason = "stopped";
        return true;
    }

    bool jog(std::size_t joint_index, double step_rad, std::string& reason) override {
        if (joint_index >= kJointCount) {
            reason = "joint_index out of range";
            return false;
        }
        if (!enabled_ || stopped_ || paused_) {
            reason = paused_ ? "gateway is paused" : "gateway is disabled or stopped";
            return false;
        }
        if (!std::isfinite(step_rad) || std::abs(step_rad) > 0.5) {
            reason = "jog step_rad must be finite and within +/-0.5 rad";
            return false;
        }
        offsets_[joint_index] += step_rad;
        trajectory_active_ = false;
        last_trajectory_.clear();
        reason = "jog accepted";
        return true;
    }

    bool execute_trajectory(const std::vector<TrajectoryState>& points, bool dry_run,
                            std::string& reason) override {
        if (!validate_trajectory(points, reason)) return false;
        if (dry_run) {
            reason = "trajectory validated (dry_run)";
            return true;
        }
        if (!enabled_ || stopped_ || paused_) {
            reason = paused_ ? "gateway is paused" : "gateway is disabled or stopped";
            return false;
        }
        last_trajectory_ = points;
        trajectory_elapsed_seconds_ = 0.0;
        trajectory_active_ = true;
        reason = "trajectory execution accepted";
        return true;
    }

    bool reset_fault(std::string& reason) override {
        stopped_ = false;
        trajectory_active_ = false;
        paused_ = false;
        last_trajectory_.clear();
        reason = "fault reset";
        return true;
    }

    bool pause(std::string& reason) override {
        if (!enabled_ || stopped_) {
            reason = "gateway is disabled or stopped";
            return false;
        }
        paused_ = true;
        reason = "paused";
        return true;
    }

    bool resume(std::string& reason) override {
        if (!paused_) {
            reason = "gateway is not paused";
            return false;
        }
        paused_ = false;
        skip_delta_after_resume_ = true;
        reason = "resumed";
        return true;
    }

    bool set_speed_scale(double scale, std::string& reason) override {
        if (!std::isfinite(scale) || scale < 0.1 || scale > 2.0) {
            reason = "speed scale must be finite and within [0.1, 2.0]";
            return false;
        }
        speed_scale_ = scale;
        reason = "speed scale updated";
        return true;
    }

private:
    static bool validate_trajectory(const std::vector<TrajectoryState>& points,
                                    std::string& reason) {
        if (points.empty()) {
            reason = "trajectory must contain at least one point";
            return false;
        }
        std::uint64_t previous_time = 0;
        std::array<double, kJointCount> previous_position{};
        bool first_point = true;
        constexpr double kMaxTrajectorySpeed = 2.0;
        constexpr std::array<double, kJointCount> lower = {-2.8, 0.0, 0.0, -1.57, -1.57, -3.14};
        constexpr std::array<double, kJointCount> upper = {2.8, 3.14, 3.14, 1.57, 1.57, 3.14};
        for (const auto& point : points) {
            const auto segment_start_time = previous_time;
            if ((first_point && point.time_from_start_ns != 0) ||
                point.time_from_start_ns < previous_time) {
                reason = "trajectory timestamps must be monotonic";
                return false;
            }
            for (std::size_t index = 0; index < kJointCount; ++index) {
                if (!std::isfinite(point.position_rad[index]) ||
                    point.position_rad[index] < lower[index] ||
                    point.position_rad[index] > upper[index] ||
                    !std::isfinite(point.velocity_rad_s[index]) ||
                    std::abs(point.velocity_rad_s[index]) > kMaxTrajectorySpeed) {
                    reason = "trajectory contains non-finite or out-of-limit joint values";
                    return false;
                }
                if (!first_point && point.time_from_start_ns > segment_start_time) {
                    const double duration = static_cast<double>(point.time_from_start_ns - segment_start_time) * 1e-9;
                    if (std::abs(point.position_rad[index] - previous_position[index]) /
                            std::max(duration, 1e-9) > kMaxTrajectorySpeed) {
                        reason = "trajectory segment exceeds the 2 rad/s speed limit";
                        return false;
                    }
                }
                previous_position[index] = point.position_rad[index];
            }
            previous_time = point.time_from_start_ns;
            first_point = false;
        }
        return true;
    }

    void sample_trajectory(std::array<double, kJointCount>& position,
                           std::array<double, kJointCount>& velocity) {
        const double elapsed_ns = trajectory_elapsed_seconds_ * 1'000'000'000.0;
        const auto& first = last_trajectory_.front();
        const auto& last = last_trajectory_.back();
        if (elapsed_ns <= static_cast<double>(first.time_from_start_ns)) {
            position = first.position_rad;
            velocity.fill(0.0);
            return;
        }
        if (elapsed_ns >= static_cast<double>(last.time_from_start_ns)) {
            position = last.position_rad;
            velocity.fill(0.0);
            trajectory_active_ = false;
            return;
        }
        for (std::size_t index = 1; index < last_trajectory_.size(); ++index) {
            const auto& right = last_trajectory_[index];
            if (elapsed_ns > static_cast<double>(right.time_from_start_ns)) continue;
            const auto& left = last_trajectory_[index - 1];
            const double span = static_cast<double>(right.time_from_start_ns - left.time_from_start_ns);
            const double alpha = span <= 0.0 ? 1.0 :
                                 (elapsed_ns - static_cast<double>(left.time_from_start_ns)) / span;
            for (std::size_t joint = 0; joint < kJointCount; ++joint) {
                position[joint] = left.position_rad[joint] +
                                  alpha * (right.position_rad[joint] - left.position_rad[joint]);
                velocity[joint] = (right.position_rad[joint] - left.position_rad[joint]) /
                                  std::max(span * 1e-9, 1e-9);
            }
            return;
        }
    }

    std::array<double, kJointCount> offsets_{};
    bool enabled_ = true;
    bool stopped_ = false;
    double last_sample_elapsed_ = -1.0;
    double trajectory_elapsed_seconds_ = 0.0;
    bool trajectory_active_ = false;
    std::vector<TrajectoryState> last_trajectory_;
    bool paused_ = false;
    bool skip_delta_after_resume_ = false;
    bool have_sample_ = false;
    std::array<double, kJointCount> held_position_{};
    double speed_scale_ = 1.0;
};

}  // namespace

std::unique_ptr<SimulationDriver> make_mock_simulation_driver() {
    return std::make_unique<MockSimulationDriver>();
}

}  // namespace arm_console
