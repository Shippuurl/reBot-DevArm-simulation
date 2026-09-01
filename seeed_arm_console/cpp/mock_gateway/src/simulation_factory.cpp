#include "simulation.hpp"

namespace arm_console {

std::unique_ptr<SimulationDriver> make_simulation_driver(const std::string& model_path,
                                                         std::string& error) {
#ifdef ARM_CONSOLE_WITH_MUJOCO
    if (!model_path.empty()) {
        if (auto mujoco = make_mujoco_simulation_driver(model_path, error)) {
            return mujoco;
        }
        return nullptr;
    }
#else
    if (!model_path.empty()) {
        error = "MuJoCo support is not enabled; rebuild with -DARM_CONSOLE_WITH_MUJOCO=ON";
        return nullptr;
    }
#endif
    return make_mock_simulation_driver();
}

}  // namespace arm_console
