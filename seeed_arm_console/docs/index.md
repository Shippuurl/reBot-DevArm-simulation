---
layout: home
---

# reBot-DevArm 仿真工作站

基于 Rerun 的 Pinocchio + ProxSuite 机器人规划与 MuJoCo 仿真工作站。

## 当前边界

![系统数据流](/diagrams/system-context.svg)

源文件：[system-context.puml](https://github.com/your-org/seeed-arm-console/blob/main/docs/diagrams/system-context.puml)

- Rerun 负责观察、记录和回放，不承担实时控制。
- MuJoCo 网关保持独立，通过本机 gRPC 输出控制和仿真遥测；TCP JSON 仅作为可关闭的 legacy 适配层。
- Pinocchio + ProxSuite 负责无界面规划服务，规划结果必须经过安全检查后才能提交仿真。
- 官方 SDK 是外部工程的唯一接入面；外部工程不依赖平台内部算法、模型或 Rerun 对象。
- ROS 2 Jazzy 只作为可选的 SDK 薄适配层和服务编排入口，不进入平台核心依赖。

## 快速入口

- [仿真工作站引导](/guide/simulation)：从零启动 Viewer、MuJoCo 和规划原型。
- [Rerun 数据方案](/panels/rerun-viewer)：实体树、坐标约定和记录路径。
- [后端仿真边界](/backend/simulation)：Pinocchio、ProxSuite、MuJoCo 的职责。
- [Python SDK](/sdk/python)：外部工程安装、调用、安全和兼容性约定。
- [C++ SDK](/sdk/cpp)：外部 C++ 工程的源码构建和 gRPC 客户端 API。
- [Rust SDK](/sdk/rust)：外部 Rust 工程的异步 gRPC 客户端 API。
- [系统架构](/architecture/c4-model)：组件和数据流边界。
- [最新工作计划](/simulation-work-plan)：里程碑、验收和未完成事项。

## 当前可运行内容

```bash
cargo run --features embedded-viewer --bin rebot_sim_viewer
scripts/run_planner_server.sh
scripts/run_gateway_grpc_smoke.sh
python3 scripts/verify_gateway.py  # legacy JSON adapter
```

真实设备接入前必须单独验证急停、使能、限位、速度限制、通信超时和断电行为。
