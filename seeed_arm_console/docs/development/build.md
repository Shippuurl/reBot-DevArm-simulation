# 源码构建

本页面向需要修改 Viewer、网关、规划服务或 SDK 的开发者。命令从仓库根目录执行；
运行状态和后续任务见[实施计划（进度与验证）](/simulation-work-plan)。

## 工具链

| 工具 | 版本 |
| --- | --- |
| Linux | Ubuntu 24.04 或等效环境 |
| Rust | 1.95+（SDK crate 最低 1.85） |
| Python | 3.10+ |
| Docker | Docker Compose v2 |
| Node.js | 18+（仅构建文档） |

## Viewer

```bash
cargo check --features embedded-viewer
cargo run --features embedded-viewer --bin rebot_sim_viewer
```

CI 可执行核心库和 SDK 测试：

```bash
cargo test --no-default-features
cargo test --manifest-path sdk/rust/Cargo.toml
```

## 规划服务

```bash
python3 -m venv .venv-planning
source .venv-planning/bin/activate
python -m pip install -r requirements-planning.txt
scripts/run_planner_server.sh
```

另一个终端运行规划和闭环冒烟测试：

```bash
scripts/run_planner_smoke.sh
scripts/run_planner_gateway_smoke.sh
```

## MuJoCo 网关

```bash
docker compose -f docker-compose.gateway.yml \
  -f docker-compose.mujoco.yml up -d --build
scripts/run_gateway_grpc_smoke.sh
```

停止容器：

```bash
docker compose -f docker-compose.gateway.yml \
  -f docker-compose.mujoco.yml down
```

## SDK

Python：

```bash
python -m pip install ./sdk/python
python -m unittest discover -s sdk/python/tests
```

C++：

```bash
cmake -S sdk/cpp -B /tmp/rebot-arm-sdk-build
cmake --build /tmp/rebot-arm-sdk-build -j2
scripts/run_cpp_sdk_smoke.sh
```

Rust：

```bash
cargo test --manifest-path sdk/rust/Cargo.toml
scripts/run_rust_sdk_smoke.sh
```

## 文档

```bash
cd docs
npm ci
npm run docs:build
```

产物位于 `docs/.vitepress/dist/`。修改图表时同时更新 `docs/diagrams/*.puml` 和对应的
`docs/public/diagrams/*.svg`。
