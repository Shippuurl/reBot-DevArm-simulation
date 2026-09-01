# 快速开始

本文以 Win11 主机 + Docker Desktop + ROS 2 Jazzy 为例。真实机械臂尚未连接时，使用 Simulation/Mock 模式即可完成 UI 和协议开发。桌面端和 Rerun Viewer 原生运行在 Win11，不需要 X11 转发。

## 1. 检查前置条件

- Windows 11、Docker Desktop（Linux containers）。
- Rust stable、Cargo 和 Git。
- 已准备的 ROS 2 Jazzy 容器 `rebot-ros2-jazzy`。
- Rerun Viewer（可选）直接运行在 Win11；ROS 2、MuJoCo 和网关使用无界面模式。

```powershell
docker version
rustc --version
cargo --version
```

## 2. 启动无界面后端容器

不要删除或重建正在使用的容器。先查看状态：

```powershell
docker ps --format 'table {{.Names}}\t{{.Image}}\t{{.Status}}'
```

OpenRAVE 规划容器和 ROS 2 容器可分别启动。容器只提供 TCP 数据服务，不映射 X11 socket，也不需要设置 `DISPLAY`：

```powershell
docker start rebot-ros2-jazzy
docker start openrave-dev
```

也可以使用项目脚本检查 ROS 2 容器的 headless 环境：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\ros2-headless.ps1 -Action check
```

如果是第一次部署，使用 Docker Compose 或 PowerShell 脚本创建容器，并将工作区挂载到 `/work`。

## 3. 启动无界面 C++ 数据网关（可选）

项目包含一个不依赖 ROS 2、X11 或 VcXsrv 的 C++ 网关样例。你提供的 ROS 2 Jazzy Desktop Full 镜像已经可以直接作为 mock 构建和运行环境，Compose 会把 `50051` 仅映射到本机回环地址：

```powershell
cd D:\JazzyCWork\seeed_arm_console
docker compose -f .\docker-compose.gateway.yml up -d
docker compose -f .\docker-compose.gateway.yml logs --tail 30
```

也可以使用脚本：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\gateway.ps1 -Action start
```

脚本会等待网关编译完成并确认 `127.0.0.1:50051` 已可连接，再启动桌面端。

网关在容器内通过本机 TCP 输出 MuJoCo 风格的六关节 JSON 遥测。若需要在 Windows 主机构建原生可执行文件，再使用以下命令：

```powershell
cd D:\JazzyCWork\seeed_arm_console\cpp\mock_gateway
cmake -S . -B build
cmake --build build --config Release
.\build\Release\arm_console_mock_gateway.exe 50051
```

然后在右侧“数据流”选择“TCP 网关”，地址保持 `127.0.0.1:50051` 并点击“连接”。没有 C++ 编译器时可先使用“本地模拟”验证界面。

需要加载项目中的 MuJoCo XML 模型时，使用派生镜像配置：

```powershell
docker compose -f .\docker-compose.gateway.yml -f .\docker-compose.mujoco.yml up -d --build
```

该配置在 Jazzy 基础镜像上安装 MuJoCo 3.12.0，加载 `assets/robot/b601_rs/mujoco/scene.xml`，仍以 headless 模式运行。若只使用基础镜像，则网关自动回退到 mock 驱动。

也可执行：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\gateway.ps1 -Action mujoco-start
```

MuJoCo 派生镜像构建需要容器能访问下载地址；网络受限时设置 `$env:MUJOCO_URL` 为可访问的镜像 URL，并同步对应的 SHA-256 参数。

启动后可以用项目自带脚本验证真实 MuJoCo 数据链路和控制命令：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\verify-gateway.ps1 -ExpectedSource mujoco -CheckControl
```

脚本会检查 6 个关节、至少 10 条 TF（含左右夹爪）、`actual_trajectory`，并发送一次 Jog 等待 `accepted` 确认。预期输出包含：

```text
telemetry=OK source=mujoco ... tf=10 actual=1
control=OK jog accepted
```

## 4. 编译并运行上位机

```powershell
cd D:\JazzyCWork\seeed_arm_console
cargo check --offline
cargo run
```

上位机默认显示 Simulation 模式和六个 Mock 关节。TCP 网关连接后，表格、曲线和场景会消费网关发送的最新快照；生产环境再将该样例替换为依据 `protocol/arm_console.proto` 实现的 gRPC 网关。

## 5. 启动嵌入式 Rerun Viewer

上位机可以把 Rerun Native Viewer 嵌入同一 `eframe` 窗口。首次启用 `native_viewer` 需要联网下载 Viewer 依赖；国内镜像未缓存全部依赖时不要使用 `--offline`：

```powershell
cargo run --features embedded-viewer --bin embedded_viewer
```

该入口包含一个自定义 egui 面板，并在 `127.0.0.1:9876` 接收 Rerun gRPC 数据。支持 Rerun SDK 的数据源连接此地址后，数据会显示在内嵌 Viewer 中；当前 C++ 网关仍使用 `50051` JSON 控制/遥测端口，Rerun 转发将在后续阶段接入。当前 `assets/robot` 模型记录和实时遥测桥接沿用同一 Rerun 实体路径。

## 6. 启用 Rerun 记录（可选）

Rerun 依赖会增加编译时间和二进制体积。启用记录功能并运行桌面端：

```powershell
cargo run --features rerun-recording
```

在右侧点击“开始录制”后，记录会保存到 `recordings/robot-<timestamp>.rrd`。记录包含关节位置/速度、TF、规划和实际轨迹，以及可用的模型资源；协议中的 `images` 和 `point_clouds` 字段会分别映射为图像实体与 `Points3D`。写盘失败不会影响控制通道。

安装 Win11 原生 Viewer 并打开记录：

```powershell
py -m pip install rerun-sdk==0.36.3
rerun recordings\robot-<timestamp>.rrd
```

没有网关时，也可以直接生成样例记录：

```powershell
cargo run --features rerun-recording --bin rerun_sample -- recordings/sample.rrd
rerun recordings\sample.rrd
```

## 7. 常见问题

### 中文显示为方框

程序会自动查找 `C:\Windows\Fonts\simhei.ttf`、微软雅黑和项目内 Noto Sans SC。若仍缺字，安装开源 CJK 字体并放入 `assets/fonts`，然后重启程序。

### UI 启动但没有实时数据

确认后端容器正在运行、端口映射正确、协议版本一致。UI 启动成功并不代表已经连接 ROS 2 图；连接状态必须由网关心跳确认。

### `re_viewer` 报 `E0463: can't find crate`

这通常是页面文件不足或构建中断后留下了不完整的 Rerun 元数据。先确保 Windows 页面文件已启用，再清理 Cargo 生成物并重新构建：

```powershell
cargo clean
cargo check --features embedded-viewer --bin embedded_viewer
```

### 图形转发

项目不依赖 VcXsrv、Xming 或容器内 RViz。Rerun Viewer 推荐直接运行在 Win11；容器中的 ROS 2、MuJoCo 和网关保持 headless。
