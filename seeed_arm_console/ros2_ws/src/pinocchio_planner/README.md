# pinocchio_planner

这是一个 ROS 2 Jazzy 编排包，用于启动仓库中的 `ArmPlanner` gRPC 服务。算法、模型和
协议位于平台服务；ROS 2 节点负责启动参数与进程编排。

## 构建和启动

```bash
source /opt/ros/jazzy/setup.bash
cd ros2_ws
colcon build --symlink-install --packages-select pinocchio_planner
source install/setup.bash
ros2 launch pinocchio_planner planner.launch.py
```

规划服务默认监听 `127.0.0.1:50053`。启动后可在另一个终端运行：

```bash
cd ..
scripts/run_planner_smoke.sh
```

## 参数

启动文件通过 launch 参数把监听地址、URDF 和默认碰撞距离传给规划服务。常用参数如下：

| 参数 | 默认值 | 作用 |
| --- | --- | --- |
| `listen` | `127.0.0.1:50053` | gRPC 监听地址 |
| `model` | B601-RS URDF | 机器人模型路径 |
| `default_minimum_distance` | `0.02` | 默认碰撞距离余量 |

例如：

```bash
ros2 launch pinocchio_planner planner.launch.py \
  listen:=0.0.0.0:50053 default_minimum_distance:=0.01
```

`config/planner.yaml` 作为参数参考文件，launch 不会自动加载它；需要修改参数时使用上面的
launch 参数。以下环境变量可替换启动目标：

| 变量 | 作用 |
| --- | --- |
| `REBOTS_PLANNER_SCRIPT` | 指定规划服务脚本 |
| `REBOTS_ARM_URDF` | 指定 URDF 模型 |

直接使用平台脚本时，在仓库根目录运行 `scripts/run_planner_server.sh`；它使用同一端口和
协议。
