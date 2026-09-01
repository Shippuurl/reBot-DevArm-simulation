#include "simulation.hpp"

#include <mujoco/mujoco.h>

#include <algorithm>
#include <array>
#include <cstring>
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
        while (data_->time + model_->opt.timestep <= elapsed_seconds) {
            mj_step(model_, data_);
        }
        SimulationSnapshot snapshot;
        snapshot.timestamp_ns = static_cast<std::uint64_t>(data_->time * 1'000'000'000.0);
        snapshot.source = "mujoco";
        for (std::size_t index = 0; index < kJointCount; ++index) {
            snapshot.position_rad[index] = data_->qpos[qpos_address_[index]];
            snapshot.velocity_rad_s[index] = data_->qvel[qvel_address_[index]];
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
        TrajectoryState actual;
        actual.position_rad = snapshot.position_rad;
        actual.velocity_rad_s = snapshot.velocity_rad_s;
        snapshot.actual_trajectory.push_back(actual);
        return snapshot;
    }

    const char* name() const override { return "mujoco"; }

    bool enable(bool enabled, std::string& reason) override {
        enabled_ = enabled;
        reason = enabled ? "enabled" : "disabled";
        return true;
    }

    bool stop(std::string& reason) override {
        std::fill_n(data_->ctrl, model_->nu, 0.0);
        stopped_ = true;
        reason = "stopped";
        return true;
    }

    bool jog(std::size_t joint_index, double step_rad, std::string& reason) override {
        if (joint_index >= kJointCount) {
            reason = "joint_index out of range";
            return false;
        }
        data_->qpos[qpos_address_[joint_index]] += step_rad;
        mj_forward(model_, data_);
        reason = "jog accepted";
        return true;
    }

private:
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

    mjModel* model_ = nullptr;
    mjData* data_ = nullptr;
    std::array<int, kJointCount> qpos_address_{};
    std::array<int, kJointCount> qvel_address_{};
    bool valid_ = false;
    bool enabled_ = false;
    bool stopped_ = false;
};

}  // namespace

std::unique_ptr<SimulationDriver> make_mujoco_simulation_driver(const std::string& model_path,
                                                                std::string& error) {
    auto driver = std::make_unique<MujocoSimulationDriver>(model_path, error);
    if (!driver->valid()) return nullptr;
    return driver;
}

}  // namespace arm_console
