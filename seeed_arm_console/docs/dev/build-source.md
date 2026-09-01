# 源码构建

## Rust 工作区

```powershell
cd D:\JazzyCWork\seeed_arm_console
cargo fmt --all
cargo check --workspace --offline
cargo build --offline
```

## 生成 Rust API 文档

发布前可以生成不依赖外部 crate 源码的工作区 API 文档：

```powershell
cargo doc --workspace --no-deps
```

生成结果位于 `target/doc/`。将该目录发布到 GitHub Pages 后，读者可以从[Rust API 文档](/dev/rust-api)页面跳转查看 `robot_workspace` 的类型与函数说明。

如果需要把可选的 Rerun SDK 一并编入 Rustdoc，先完成依赖下载，再执行：

```powershell
cargo doc --workspace --no-deps --all-features
```

## 字体

上位机启动时会自动探测 Win11 系统字体。跨平台发布时，可将开源字体放入 `assets/fonts`：

```text
assets/fonts/
├── NotoSansSC-VF.ttf
├── Inter-Regular.ttf
└── JetBrainsMono-Regular.ttf
```

不要把未确认再分发许可的系统字体提交到仓库。

## ROS 2 Jazzy Docker

```powershell
docker ps --format 'table {{.Names}}\t{{.Image}}\t{{.Status}}'
docker start rebot-ros2-jazzy
docker start openrave-dev
```

容器内检查工作区：

```bash
printenv ROS_DISTRO
echo "$DISPLAY"
ls -la /work
```

启动本项目的 headless C++ 网关：

```powershell
cd D:\JazzyCWork\seeed_arm_console
docker compose -f .\docker-compose.gateway.yml up -d
```

桌面 UI 和 Rerun Viewer 原生运行在 Windows，后端容器不设置 `DISPLAY`，不依赖 X11。

## 编译排错

| 现象 | 检查项 |
| --- | --- |
| Cargo 下载失败 | 确认国内镜像、代理和离线缓存；先执行 `cargo check --offline` |
| 中文方框 | 检查 `C:\Windows\Fonts\simhei.ttf` 或 `assets/fonts` |
| Rerun feature 编译失败 | 确认 `rerun` 依赖已下载，并使用 `cargo check --features rerun-recording` |
| ROS 2 无数据 | 检查容器状态、`ROS_DOMAIN_ID`、DDS 网络和桥接日志 |
| 网关无数据 | 检查 `docker compose ... ps`、`127.0.0.1:50051` 端口和 UI 数据源选择 |

## 构建产物

本地构建生成 `target/`，不应提交到版本库。发布包至少包含可执行文件、配置模板、模型目录和字体许可说明；真实设备凭据通过环境变量或外部密钥管理注入。
