# 开发目录约定

本项目的日常开发核心是 `seeed_arm_console`。其他目录属于模型资料、第三方源码或外部规划服务，不应直接混入 Rust/C++ 业务代码。

```text
seeed_arm_console/
├── src/                 Rust UI、遥测和 Rerun 桥接
├── cpp/mock_gateway/    MuJoCo/Mock 网关
├── protocol/            protobuf 协议定义
├── assets/              运行时模型、网格和场景
├── scripts/             Linux/Windows 启动与验证脚本
├── docs/                架构和开发文档
├── recordings/          本地 Rerun 实验记录（不作为源码依赖）
├── target/              Cargo 生成目录
└── cpp/**/build-*       CMake 生成目录
```

仓库外的参考目录：

- `reBot-DevArm/`：硬件、CAD 和上游资料；运行时只引用已整理到 `assets/` 的模型。
- `scripts/pinocchio_proxsuite_rerun.py`：Pinocchio/ProxSuite 规划原型与 Rerun 输出。
- `eguiLibrary/`：第三方 egui 实验库，与仿真运行链路无关。

## 日常入口

只从仓库根目录打开 `reBot-DevArm-sim.code-workspace`。VS Code 已排除 `target`、`build-*`、CAD 和大网格文件；不要手动编辑这些生成目录。

## 运行产物

临时 `.rrd` 记录放在 `recordings/`。需要长期保存的实验记录建议复制到工作区外的 `records/`，不要让实验输出成为模型或源码的隐式依赖。

## 依赖边界

- MuJoCo 网关通过 `cpp/mock_gateway` 和 `assets/robot/b601_rs/mujoco` 运行。
- Rerun 通过 Rust feature `rerun-recording` 启用。
- Pinocchio/ProxSuite 以 ROS 2 Jazzy headless 服务接入，不引入 MoveIt，也不复制进 `src/`。
