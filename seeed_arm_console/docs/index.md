---
layout: home

hero:
  name: Seeed Arm Console
  text: Rust 机器人上位机
  tagline: 面向 ROS 2 Jazzy、仿真与真实机械臂的可观测控制工作台
  actions:
    - theme: brand
      text: 快速开始
      link: /guide/quick-start
    - theme: alt
      text: 查看架构
      link: /architecture/c4-model

features:
  - icon: 🦀
    title: Rust + egui
    details: 原生 Windows/Linux 桌面应用，控制面板、Jog、报警和停靠布局共享一套代码。
  - icon: 🤖
    title: ROS 2 Jazzy
    details: ROS 2、OpenRAVE 与 MuJoCo 运行在可复现的 Docker 环境中，UI 与实时后端解耦。
  - icon: 📈
    title: Rerun 可观测性
    details: 记录机器人模型、关节状态、TF、图像、点云和规划/实际轨迹；Rerun 不承担实时控制。
  - icon: 🧩
    title: 可替换适配层
    details: 通过标准控制协议连接 Mock、仿真器、真实驱动和未来的 gRPC/WebSocket 网关。
---

## 项目定位

Seeed Arm Console 是一个面向教学、仿真和工业原型验证的机器人上位机框架。它将桌面 UI、控制协议、ROS 2 桥接、规划、仿真和可视化拆成可替换的模块。

当前仓库状态：

| 能力 | 状态 | 说明 |
| --- | --- | --- |
| egui 控制面板 | 已有骨架 | 连接、使能、Jog、Telemetry、故障标签可运行 |
| 中文字体回退 | 已实现 | 内置 Noto Sans SC，Inter/JetBrains Mono 负责英文与数据 |
| ROS 2 Jazzy Docker | 已准备 | 通过桥接容器连接控制协议 |
| 遥测数据源抽象 | 已实现 | `TelemetryFrame` 可替换 Mock、通道和网关实现 |
| Rerun 记录桥 | 预留接口 | 逐步接入模型、TF、图像、点云和轨迹 |
| gRPC/WebSocket 网关 | 协议已冻结 | 共用定义见 `protocol/arm_console.proto` |
| X11 / VcXsrv | 不需要 | Windows UI 与 Rerun Viewer 原生运行 |
| OpenRAVE headless | 已准备 | 用于规划/IK；不引入 MoveIt |
| MuJoCo 仿真 | 规划中 | 作为驱动适配器和回放验证环境 |

## 文档导航

- [快速开始](/guide/quick-start)：启动 Rust UI、ROS 2 Jazzy 和仿真后端。
- [无 X11 实施计划](/plan)：数据通道、C++ 网关、Rerun 和验收顺序。
- [C4 模型](/architecture/c4-model)：系统上下文、容器、组件和代码级边界。
- [4+1 视图](/architecture/4plus1)：逻辑、开发、进程、物理和场景视图。
- [控制面板](/panels/control)：连接、使能、故障复位和安全要求。
- [gRPC API](/backend/grpc-api)：协议对象、版本策略和错误模型。
- [Rust API 文档](/dev/rust-api)：跳转到 GitHub Pages 上由 `cargo doc` 生成的 crate API。

## 安全声明

本项目是控制与可视化框架，不替代经过认证的机器人安全控制器。真实设备接入前必须验证急停、使能回路、限位、速度限制、通信超时和断电行为。所有示例默认使用 Simulation 或 Mock 模式。
