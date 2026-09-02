#include "simulation.hpp"

#include <mujoco/mujoco.h>

#include <algorithm>
#include <array>
#include <cmath>
#include <cstring>
#include <cstdlib>
#include <limits>
#include <utility>

namespace arm_console {
namespace {

// MuJoCo exposes body poses in world coordinates. The wire protocol and
// Rerun transform graph use parent-relative poses, so convert each child pose
// into the requested parent frame before publishing it.
std::array<double, 4> quat_conjugate(const double q[4]) {
    return {q[0], -q[1], -q[2], -q[3]};
}

std::array<double, 4> quat_multiply(const std::array<double, 4>& lhs,
                                    const double rhs[4]) {
    return {lhs[0] * rhs[0] - lhs[1] * rhs[1] - lhs[2] * rhs[2] - lhs[3] * rhs[3],
            lhs[0] * rhs[1] + lhs[1] * rhs[0] + lhs[2] * rhs[3] - lhs[3] * rhs[2],
            lhs[0] * rhs[2] - lhs[1] * rhs[3] + lhs[2] * rhs[0] + lhs[3] * rhs[1],
            lhs[0] * rhs[3] + lhs[1] * rhs[2] - lhs[2] * rhs[1] + lhs[3] * rhs[0]};
}

std::array<double, 3> rotate_vector(const std::array<double, 4>& q,
                                    const std::array<double, 3>& vector) {
    const std::array<double, 4> v = {0.0, vector[0], vector[1], vector[2]};
    const auto qv = quat_multiply(q, v.data());
    const auto inverse = quat_conjugate(q.data());
    const auto result = quat_multiply(qv, inverse.data());
    return {result[1], result[2], result[3]};
}

class MujocoSimulationDriver final : public SimulationDriver {
public:
    explicit MujocoSimulationDriver(const std::string& model_path, std::string& error) {
        if (const char* configured = std::getenv("MUJOCO_ENABLE_DEPTH_SENSOR")) {
            depth_sensor_enabled_ = std::string(configured) != "0";
        }
        char buffer[1024] = {};
        model_ = mj_loadXML(model_path.c_str(), nullptr, buffer, sizeof(buffer));
        if (model_ == nullptr) {
            error = buffer[0] == '\0' ? "mj_loadXML failed" : buffer;
            return;
        }
        data_ = mj_makeData(model_);
        if (data_ == nullptr) {
            error = "mj_makeData failed";
            mj_deleteModel(model_);
            model_ = nullptr;
            return;
        }
        // Populate xpos/xquat/qvel for the initial state before the first
        // telemetry frame is emitted.  Without this call MuJoCo leaves the
        // derived body arrays at zero until the first mj_step.
        mj_forward(model_, data_);
        for (std::size_t index = 0; index < kJointCount; ++index) {
            const std::string joint_name = "joint" + std::to_string(index + 1);
            const int joint_id = mj_name2id(model_, mjOBJ_JOINT, joint_name.c_str());
            if (joint_id < 0 || model_->jnt_type[joint_id] != mjJNT_HINGE) {
                error = "missing hinge joint: " + joint_name;
                mj_deleteData(data_);
                mj_deleteModel(model_);
                data_ = nullptr;
                model_ = nullptr;
                return;
            }
            qpos_address_[index] = model_->jnt_qposadr[joint_id];
            qvel_address_[index] = model_->jnt_dofadr[joint_id];
        }
        valid_ = true;
    }

    ~MujocoSimulationDriver() override {
        if (data_ != nullptr) mj_deleteData(data_);
        if (model_ != nullptr) mj_deleteModel(model_);
    }

    bool valid() const { return valid_; }

    SimulationSnapshot sample(double elapsed_seconds) override {
        const double delta_seconds = (last_sample_elapsed_ < 0.0 || skip_delta_after_resume_)
                                         ? 0.0
                                         : std::max(0.0, elapsed_seconds - last_sample_elapsed_);
        skip_delta_after_resume_ = false;
        last_sample_elapsed_ = elapsed_seconds;
        if (trajectory_active_ && !paused_) trajectory_elapsed_seconds_ += delta_seconds * speed_scale_;
        if (!paused_) {
            if (resume_pending_) {
                // Do not integrate a backlog accumulated while paused. The
                // next frame resumes from the current wall-clock instant.
                data_->time = elapsed_seconds;
                mj_forward(model_, data_);
                resume_pending_ = false;
            }
            while (data_->time + model_->opt.timestep <= elapsed_seconds) {
                mj_step(model_, data_);
            }
        }
        SimulationSnapshot snapshot;
        snapshot.timestamp_ns = static_cast<std::uint64_t>(data_->time * 1'000'000'000.0);
        snapshot.source = "mujoco";
        for (std::size_t index = 0; index < kJointCount; ++index) {
            snapshot.position_rad[index] = data_->qpos[qpos_address_[index]];
            snapshot.velocity_rad_s[index] = data_->qvel[qvel_address_[index]];
        }
        if (!last_trajectory_.empty()) {
            sample_trajectory(snapshot.position_rad, snapshot.velocity_rad_s);
            for (std::size_t index = 0; index < kJointCount; ++index) {
                data_->qpos[qpos_address_[index]] = snapshot.position_rad[index];
                data_->qvel[qvel_address_[index]] = snapshot.velocity_rad_s[index];
            }
            // Execution is intentionally kinematic in this adapter. The
            // production driver will replace this with its servo interface;
            // MuJoCo still recomputes contacts and body poses from the exact
            // commanded joint state for every telemetry frame.
            mj_forward(model_, data_);
        }
        if (paused_) {
            snapshot.velocity_rad_s.fill(0.0);
            for (std::size_t index = 0; index < kJointCount; ++index) {
                data_->qvel[qvel_address_[index]] = 0.0;
            }
            mj_forward(model_, data_);
        }
        append_body_transform(snapshot, "world", "base_link");
        for (int index = 1; index <= 6; ++index) {
            const std::string name = "link" + std::to_string(index);
            append_body_transform(snapshot, index == 1 ? "base_link" :
                                  "link" + std::to_string(index - 1), name);
        }
        append_body_transform(snapshot, "link6", "gripper_end");
        // The finger bodies are part of the URDF/MJCF model.  Publishing them
        // keeps the complete visual model aligned in Rerun, including the
        // prismatic gripper joints.
        append_body_transform(snapshot, "gripper_end", "gripper_left");
        append_body_transform(snapshot, "gripper_end", "gripper_right");
        constexpr int kMaxReportedContacts = 64;
        for (int index = 0; index < data_->ncon && index < kMaxReportedContacts; ++index) {
            const auto& contact = data_->contact[index];
            const char* first = mj_id2name(model_, mjOBJ_GEOM, contact.geom[0]);
            const char* second = mj_id2name(model_, mjOBJ_GEOM, contact.geom[1]);
            if (first == nullptr || second == nullptr) continue;
            double force[6] = {};
            mj_contactForce(model_, data_, index, force);
            snapshot.contacts.push_back({first, second, contact.dist, std::abs(force[0])});
        }
        if (depth_sensor_enabled_) append_depth_point_cloud(snapshot);
        TrajectoryState actual;
        actual.position_rad = snapshot.position_rad;
        actual.velocity_rad_s = snapshot.velocity_rad_s;
        actual.time_from_start_ns = static_cast<std::uint64_t>(
            std::max(0.0, trajectory_elapsed_seconds_) * 1'000'000'000.0);
        snapshot.actual_trajectory.push_back(actual);
        if (!last_trajectory_.empty()) snapshot.planned_trajectory = last_trajectory_;
        return snapshot;
    }

    const char* name() const override { return "mujoco"; }

    bool enable(bool enabled, std::string& reason) override {
        enabled_ = enabled;
        if (enabled) stopped_ = false;
        paused_ = false;
        resume_pending_ = true;
        skip_delta_after_resume_ = true;
        if (!enabled_) {
            trajectory_active_ = false;
            last_trajectory_.clear();
        }
        reason = enabled ? "enabled" : "disabled";
        return true;
    }

    bool stop(std::string& reason) override {
        std::fill_n(data_->ctrl, model_->nu, 0.0);
        stopped_ = true;
        trajectory_active_ = false;
        paused_ = false;
        resume_pending_ = true;
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
        const std::string joint_name = "joint" + std::to_string(joint_index + 1);
        const int joint_id = mj_name2id(model_, mjOBJ_JOINT, joint_name.c_str());
        if (joint_id >= 0 && model_->jnt_limited[joint_id]) {
            const double candidate = data_->qpos[qpos_address_[joint_index]] + step_rad;
            if (candidate < model_->jnt_range[2 * joint_id] ||
                candidate > model_->jnt_range[2 * joint_id + 1]) {
                reason = "jog would exceed joint limits";
                return false;
            }
        }
        data_->qpos[qpos_address_[joint_index]] += step_rad;
        mj_forward(model_, data_);
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
        resume_pending_ = true;
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
        for (std::size_t index = 0; index < kJointCount; ++index) {
            data_->qvel[qvel_address_[index]] = 0.0;
        }
        mj_forward(model_, data_);
        reason = "paused";
        return true;
    }

    bool resume(std::string& reason) override {
        if (!paused_) {
            reason = "gateway is not paused";
            return false;
        }
        paused_ = false;
        resume_pending_ = true;
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
    bool validate_trajectory(const std::vector<TrajectoryState>& points,
                             std::string& reason) const {
        if (points.empty()) {
            reason = "trajectory must contain at least one point";
            return false;
        }
        std::uint64_t previous_time = 0;
        std::array<double, kJointCount> previous_position{};
        bool first_point = true;
        constexpr double kMaxTrajectorySpeed = 2.0;
        for (const auto& point : points) {
            const auto segment_start_time = previous_time;
            if ((first_point && point.time_from_start_ns != 0) ||
                point.time_from_start_ns < previous_time) {
                reason = "trajectory timestamps must be monotonic";
                return false;
            }
            for (std::size_t index = 0; index < kJointCount; ++index) {
                const std::string joint_name = "joint" + std::to_string(index + 1);
                const int joint_id = mj_name2id(model_, mjOBJ_JOINT, joint_name.c_str());
                if (!std::isfinite(point.position_rad[index]) ||
                    !std::isfinite(point.velocity_rad_s[index]) ||
                    std::abs(point.velocity_rad_s[index]) > kMaxTrajectorySpeed) {
                    reason = "trajectory contains non-finite joint values";
                    return false;
                }
                if (joint_id >= 0 && model_->jnt_limited[joint_id] &&
                    (point.position_rad[index] < model_->jnt_range[2 * joint_id] ||
                     point.position_rad[index] > model_->jnt_range[2 * joint_id + 1])) {
                    reason = "trajectory would exceed joint limits";
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

    void append_body_transform(SimulationSnapshot& snapshot, const std::string& parent,
                               const std::string& child) const {
        const int body_id = mj_name2id(model_, mjOBJ_BODY, child.c_str());
        if (body_id < 0) return;
        TransformState transform;
        transform.parent = parent;
        transform.child = child;

        const double* child_position = data_->xpos + body_id * 3;
        const double* child_quaternion = data_->xquat + body_id * 4;
        std::array<double, 3> relative_position = {child_position[0], child_position[1],
                                                   child_position[2]};
        std::array<double, 4> relative_quaternion = {child_quaternion[0], child_quaternion[1],
                                                     child_quaternion[2], child_quaternion[3]};

        if (parent != "world") {
            const int parent_id = mj_name2id(model_, mjOBJ_BODY, parent.c_str());
            if (parent_id >= 0) {
                const double* parent_position = data_->xpos + parent_id * 3;
                const double* parent_quaternion = data_->xquat + parent_id * 4;
                const std::array<double, 3> delta = {
                    child_position[0] - parent_position[0],
                    child_position[1] - parent_position[1],
                    child_position[2] - parent_position[2],
                };
                const auto parent_inverse = quat_conjugate(parent_quaternion);
                relative_position = rotate_vector(parent_inverse, delta);
                relative_quaternion = quat_multiply(parent_inverse, child_quaternion);
            }
        }

        std::copy(relative_position.begin(), relative_position.end(),
                  transform.translation_m.begin());
        // MuJoCo stores quaternions as w,x,y,z; the wire format is x,y,z,w.
        transform.rotation_xyzw = {relative_quaternion[1], relative_quaternion[2],
                                   relative_quaternion[3], relative_quaternion[0]};
        snapshot.tf.push_back(std::move(transform));
    }

    void append_depth_point_cloud(SimulationSnapshot& snapshot) const {
        const int camera_id = mj_name2id(model_, mjOBJ_CAMERA, "overhead_depth");
        if (camera_id < 0 || data_->cam_xpos == nullptr || data_->cam_xmat == nullptr) return;

        // Keep the headless sensor intentionally small. It is a depth sample
        // for planning/visual diagnostics, not a photorealistic renderer.
        constexpr int kWidth = 32;
        constexpr int kHeight = 24;
        constexpr double kPi = 3.14159265358979323846;
        const double half_fovy = static_cast<double>(model_->cam_fovy[camera_id]) * kPi / 360.0;
        const double tan_half_fovy = std::tan(half_fovy);
        if (!std::isfinite(tan_half_fovy) || tan_half_fovy <= 0.0) return;
        const double aspect = static_cast<double>(kWidth) / static_cast<double>(kHeight);
        const mjtNum* camera_position = data_->cam_xpos + camera_id * 3;
        const mjtNum* camera_matrix = data_->cam_xmat + camera_id * 9;

        PointCloudState cloud;
        cloud.sensor = "overhead_depth";
        cloud.positions_xyz.reserve(static_cast<std::size_t>(kWidth * kHeight));
        for (int y = 0; y < kHeight; ++y) {
            for (int x = 0; x < kWidth; ++x) {
                const double ndc_x =
                    (2.0 * (static_cast<double>(x) + 0.5) / kWidth) - 1.0;
                const double ndc_y =
                    1.0 - (2.0 * (static_cast<double>(y) + 0.5) / kHeight);
                std::array<mjtNum, 3> local_direction = {
                    static_cast<mjtNum>(ndc_x * aspect * tan_half_fovy),
                    static_cast<mjtNum>(ndc_y * tan_half_fovy),
                    static_cast<mjtNum>(-1.0),
                };
                const mjtNum length = std::sqrt(
                    local_direction[0] * local_direction[0] +
                    local_direction[1] * local_direction[1] +
                    local_direction[2] * local_direction[2]);
                if (!std::isfinite(length) || length <= std::numeric_limits<mjtNum>::epsilon()) {
                    continue;
                }
                for (auto& component : local_direction) component /= length;

                const std::array<mjtNum, 3> world_direction = {
                    camera_matrix[0] * local_direction[0] + camera_matrix[1] * local_direction[1] +
                        camera_matrix[2] * local_direction[2],
                    camera_matrix[3] * local_direction[0] + camera_matrix[4] * local_direction[1] +
                        camera_matrix[5] * local_direction[2],
                    camera_matrix[6] * local_direction[0] + camera_matrix[7] * local_direction[1] +
                        camera_matrix[8] * local_direction[2],
                };
                const mjtNum distance =
                    mj_ray(model_, data_, camera_position, world_direction.data(), nullptr, 1,
                           -1, nullptr, nullptr);
                if (!std::isfinite(distance) || distance <= 0.0) continue;
                cloud.positions_xyz.push_back({
                    static_cast<float>(camera_position[0] + distance * world_direction[0]),
                    static_cast<float>(camera_position[1] + distance * world_direction[1]),
                    static_cast<float>(camera_position[2] + distance * world_direction[2]),
                });
            }
        }
        if (!cloud.positions_xyz.empty()) snapshot.point_clouds.push_back(std::move(cloud));
    }

    mjModel* model_ = nullptr;
    mjData* data_ = nullptr;
    std::array<int, kJointCount> qpos_address_{};
    std::array<int, kJointCount> qvel_address_{};
    bool valid_ = false;
    bool enabled_ = false;
    bool stopped_ = false;
    double last_sample_elapsed_ = -1.0;
    double trajectory_elapsed_seconds_ = 0.0;
    bool trajectory_active_ = false;
    std::vector<TrajectoryState> last_trajectory_;
    bool paused_ = false;
    bool resume_pending_ = false;
    bool skip_delta_after_resume_ = false;
    double speed_scale_ = 1.0;
    bool depth_sensor_enabled_ = true;
};

}  // namespace

std::unique_ptr<SimulationDriver> make_mujoco_simulation_driver(const std::string& model_path,
                                                                std::string& error) {
    auto driver = std::make_unique<MujocoSimulationDriver>(model_path, error);
    if (!driver->valid()) return nullptr;
    return driver;
}

}  // namespace arm_console
