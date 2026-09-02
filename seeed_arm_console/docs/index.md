---
layout: home
hero:
  name: reBot-DevArm
  text: B601-RS 仿真工作站
  tagline: 用统一 SDK 连接规划、仿真和 Rerun 可视化
  actions:
    - theme: brand
      text: 立即运行
      link: /guide/simulation
    - theme: alt
      text: 查看架构
      link: /architecture/c4-model
features:
  - title: 规划
    details: Pinocchio、ProxSuite 和 Coal 提供 IK、轨迹候选与碰撞摘要。
  - title: 执行
    details: ArmGateway 连接 MuJoCo，统一处理轨迹预检、执行和遥测。
  - title: 观察
    details: Rerun Viewer 展示模型、TF、轨迹、接触和传感器数据。
---

## 快速跑通

在仓库根目录打开三个终端：

```bash
# 终端 A：启动 Viewer
cargo run --features embedded-viewer --bin rebot_sim_viewer

# 终端 B：启动 MuJoCo 网关
docker compose -f docker-compose.gateway.yml \
  -f docker-compose.mujoco.yml up -d --build

# 终端 C：验证网关
scripts/run_gateway_grpc_smoke.sh
```

需要规划时，再启动 `scripts/run_planner_server.sh`，然后运行
`scripts/run_planner_smoke.sh`。完整步骤、端口和排查方法见[仿真工作站](/guide/simulation)。

## 一条主线

![系统数据流](/diagrams/system-context.svg)

业务程序通过 Python、C++ 或 Rust SDK 调用 `ArmPlanner` 获取候选轨迹，再把轨迹交给
`ArmGateway` 预检和执行。网关发布的状态由 Rerun Viewer 订阅；Viewer 控制面板通过
Rust SDK 发送命令。

## 从哪里开始

| 目标 | 页面 |
| --- | --- |
| 第一次启动仿真 | [仿真工作站](/guide/simulation) |
| 了解组件和数据流 | [系统架构](/architecture/c4-model) |
| 接入业务程序 | [Python SDK](/sdk/python)、[C++ SDK](/sdk/cpp)、[Rust SDK](/sdk/rust) |
| 调整规划或仿真 | [规划与仿真](/backend/simulation) |
| 查看记录和实体树 | [Rerun Viewer](/panels/rerun-viewer) |
| 查协议字段和单位 | [SDK 与协议边界](/architecture/sdk-boundary) |
| 查看当前进度和验证 | [实施计划（进度与验证）](/simulation-work-plan) |
| 构建、发布和部署 | [源码构建](/development/build)、[安全部署](/deployment/security) |

模型、坐标和单位约定集中在[项目简介](/guide/introduction)；设计取舍记录在[设计决策
（ADR）](/architecture/decisions)。
