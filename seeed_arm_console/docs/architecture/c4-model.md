# 当前系统架构

## 上下文

![系统上下文](/diagrams/system-context.svg)

外部工程的唯一接入面是官方 SDK。SDK 只封装 `arm.console.v1` gRPC
协议，返回独立于 protobuf 的数据类型；它不携带 Viewer、MuJoCo、Pinocchio、
ProxSuite、URDF 或 ROS 2 运行时。ROS 2 包是可选的薄适配层，负责把 ROS 话题/服务
映射到 SDK，不属于平台核心依赖。

## 组件

| 组件 | 职责 | 当前入口 |
| --- | --- | --- |
| `rebot_sim_viewer` | 平台内部 Rerun Viewer 与自定义面板 | `src/bin/embedded_viewer.rs` |
| `ArmPlanner` | Pinocchio/ProxSuite headless 规划 gRPC 服务 | `scripts/planner_grpc_server.py` |
| `mujoco_rerun_bridge` | legacy JSON 遥测到 Rerun 的兼容转发 | `src/bin/mujoco_rerun_bridge.rs` |
| `cpp/mock_gateway` | MuJoCo 驱动和 gRPC 控制/遥测网关（含 JSON 适配） | `cpp/mock_gateway/` |
| `sdk/python` | 外部工程使用的 Python SDK | `sdk/python/rebot_sdk/` |
| `sdk/cpp` | 外部工程使用的 C++ SDK 原型 | `sdk/cpp/include/rebot_sdk/client.hpp` |
| `sdk/rust` | 外部工程使用的 Rust SDK 源码包 | `sdk/rust/src/lib.rs` |
| `ros2_ws/pinocchio_planner` | 可选 ROS 2 Jazzy → SDK 编排适配层 | `ros2_ws/src/pinocchio_planner/` |
| `protocol` | 控制和遥测协议来源 | `protocol/arm_console.proto` |

规划、仿真、记录各自独立；Rerun 不得反向调用控制命令。规划结果提交仿真前必须再次检查限位、碰撞、速度和时间序列。外部客户端通过 SDK 调用服务，不直接加载平台模型或内部对象。
