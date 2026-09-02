# pinocchio_planner

ROS 2 Jazzy 的最小编排包。它不复制规划算法，而是通过 `ros2 launch` 启动仓库根目录的 `scripts/planner_grpc_server.py`，使 Pinocchio + ProxSuite 服务可以和 MuJoCo/Rerun 一起编排。规划服务仍保持 ROS-agnostic，便于在 Docker 或主机上独立重启。

```bash
source /opt/ros/jazzy/setup.bash
cd ros2_ws
colcon build --symlink-install --packages-select pinocchio_planner
source install/setup.bash
ros2 launch pinocchio_planner planner.launch.py
```

默认监听 `127.0.0.1:50053`，规划安全余量默认 `0.02 m`；MATE 阶段无显式覆盖时使用 `0.001 m`。可用 `REBOTS_PLANNER_SCRIPT` 和 `REBOTS_ARM_URDF` 指向其他 checkout 或模型。
