# 项目简介

reBot-DevArm 仿真工作站用于验证 B601-RS 机械臂的运动学、约束规划、动力学仿真和可观测性。

## 技术栈

| 组件 | 职责 |
| --- | --- |
| 官方 SDK | 外部工程访问 ArmGateway/ArmPlanner 的唯一公共接入面 |
| ROS 2 Jazzy | 可选的 SDK 薄适配层与服务编排，不是平台核心依赖 |
| Pinocchio | URDF 模型、FK、雅可比和运动学计算 |
| ProxSuite | 关节限位、约束 QP 和轨迹优化 |
| MuJoCo | 动力学仿真、控制回放和接触数据 |
| Rerun | 3D、时间线、曲线、记录和回放 |
| Rust/eframe | 主 Viewer 外壳与中文控制面板 |

## 设计原则

规划、仿真和可视化相互独立。外部工程只依赖官方 SDK，不需要安装或理解平台内部的 MuJoCo、Pinocchio、ProxSuite、URDF 或 Rerun。规划结果是候选结果，提交 MuJoCo 前必须再次执行关节限位、碰撞和时序检查。Rerun 只订阅数据，不直接控制机器人。

## 资源约定

默认模型目录为 `assets/robot/b601_rs`，长度使用米，角度使用弧度，坐标使用右手系 Z-up。模型清单位于 `rerun/model.json`。
