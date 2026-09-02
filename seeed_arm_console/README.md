# reBot-DevArm 仿真工作站

面向 B601-RS 的规划、MuJoCo 运动学/接触仿真和 Rerun 可视化工作站。业务程序通过 Python、
C++ 或 Rust SDK 调用统一的 `arm.console.v1` gRPC 服务。

## 快速运行

```bash
# 终端 A
cargo run --features embedded-viewer --bin rebot_sim_viewer

# 终端 B
docker compose -f docker-compose.gateway.yml \
  -f docker-compose.mujoco.yml up -d --build

# 终端 C
scripts/run_gateway_grpc_smoke.sh
```

规划服务和 Planner → Gateway 闭环：

```bash
scripts/run_planner_server.sh
scripts/run_planner_smoke.sh
scripts/run_planner_gateway_smoke.sh
```

## 文档

从 [`docs/`](docs/) 打开 VitePress 文档：

- [项目简介](docs/guide/introduction.md)：组件、数据约定和仓库布局；
- [仿真工作站](docs/guide/simulation.md)：从启动到排查的完整步骤；
- [系统架构](docs/architecture/c4-model.md)：服务分层和请求链路；
- [Python](docs/sdk/python.md)、[C++](docs/sdk/cpp.md)、[Rust](docs/sdk/rust.md)：SDK 接入；
- [规划与仿真](docs/backend/simulation.md)：算法和执行边界；
- [Rerun Viewer](docs/panels/rerun-viewer.md)：实体树和记录配置；
- [源码构建](docs/development/build.md)：本地构建与验证。

构建文档：

```bash
cd docs
npm ci
npm run docs:dev      # 本地预览
npm run docs:build    # 生产构建
```

## 目录

| 目录 | 内容 |
| --- | --- |
| `assets/` | B601 模型、网格、场景和 UI 资源 |
| `protocol/` | gRPC 协议源文件 |
| `cpp/mock_gateway/` | Mock / MuJoCo 网关 |
| `scripts/` | 服务启动和冒烟测试脚本 |
| `sdk/` | Python、C++、Rust SDK |
| `src/` | Rust Viewer 和 Rerun 记录器 |

实施状态和验证历史集中在[实施计划（进度与验证）](docs/simulation-work-plan.md)；跨主机或真实设备
部署前请阅读[安全部署](docs/deployment/security.md)。
