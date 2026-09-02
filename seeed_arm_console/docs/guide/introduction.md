# 项目简介

reBot-DevArm 是面向 B601-RS 的规划、动力学仿真与可视化工作站。它把计算、执行和观察
拆成独立服务，业务程序只需连接公开的 gRPC 接口即可完成一条控制链路。

## 典型流程

```text
目标位姿 → ArmPlanner → 候选轨迹 → ArmGateway 预检 → MuJoCo 执行
                                                    ↓
                                             Rerun Viewer 观察
```

规划结果永远先经过网关预检，再进入执行队列。Planner 返回碰撞摘要，Gateway 在入队前
检查执行输入和运动边界，脚本、Viewer 和自动化测试沿用同一条调用链。

## 组件

| 组件 | 负责什么 | 入口 |
| --- | --- | --- |
| `ArmPlanner` | FK、雅可比、IK、轨迹采样和碰撞摘要 | `127.0.0.1:50053` |
| `ArmGateway` | 会话、命令校验、轨迹执行和遥测 | `127.0.0.1:50051` |
| `rebot_sim_viewer` | 3D 模型、时间线、曲线、记录和回放 | `127.0.0.1:9876` |
| Python / C++ / Rust SDK | 业务程序的统一客户端 | `sdk/` |
| ROS 2 适配包 | 启动和编排规划服务 | `ros2_ws/src/pinocchio_planner` |

## 技术栈

- Pinocchio：从 URDF 建立运动学模型，计算 FK、雅可比和 IK。
- ProxSuite：处理关节盒、速度盒和增量 QP 约束。
- Coal：规划阶段的几何距离与碰撞检查。
- MuJoCo：执行轨迹并生成关节、TF、接触和深度点云。
- Rerun：把模型、状态、轨迹和传感器帧放到同一时间轴。
- Rust、eframe：承载 Viewer 和仿真控制面板。

## 数据约定

长度用米，角度用弧度，速度用弧度/秒，时间用 Unix 纳秒。四元数按 `x,y,z,w` 排列；
规划目标默认位于 `world` 坐标系。B601-RS 的 URDF、网格和 MuJoCo 场景版本需要对应，
资源清单见 `assets/manifest.json`。

## 仓库入口

| 目录 | 内容 |
| --- | --- |
| `assets/` | URDF、网格、MuJoCo 场景、字体和 UI 资源 |
| `protocol/` | `arm.console.v1` 协议源文件 |
| `cpp/mock_gateway/` | Mock / MuJoCo 网关实现 |
| `scripts/` | 服务启动和冒烟测试脚本 |
| `sdk/` | Python、C++、Rust 客户端 |
| `src/` | Rust Viewer、Rerun 记录和数据处理 |

先运行[仿真工作站](/guide/simulation)，再按需要阅读[系统架构](/architecture/c4-model)
或对应的 SDK 指南。
