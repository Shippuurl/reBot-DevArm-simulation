# Headless C++ 网关示例

`arm_console_mock_gateway` 是一个不依赖 ROS 2、X11 或图形驱动的最小数据源。它在 TCP 端口上按 50 Hz 输出换行分隔的 JSON 遥测帧，桌面端可以直接选择“TCP 网关”连接它。

网关现在使用 `SimulationDriver` 抽象。默认构建运行确定性的 mock 驱动；如果启用 MuJoCo，传入 XML 模型路径即可读取 `mjData` 中的关节和刚体状态。无论使用哪种驱动，TCP 数据格式保持一致。

## 构建

推荐直接使用项目提供的 ROS 2 Jazzy 镜像：

```powershell
cd D:\JazzyCWork\seeed_arm_console
docker compose -f .\docker-compose.gateway.yml up -d
```

Compose 会在容器内完成 CMake 构建并保持网关持续运行。需要停止时执行 `docker compose -f .\docker-compose.gateway.yml stop`。

如果希望在 Windows 主机生成原生可执行文件：

```powershell
cd D:\JazzyCWork\seeed_arm_console\cpp\mock_gateway
cmake -S . -B build
cmake --build build --config Release
```

启用已安装的 MuJoCo SDK：

```powershell
cmake -S . -B build-mujoco -DARM_CONSOLE_WITH_MUJOCO=ON -DMUJOCO_ROOT=C:\mujoco
cmake --build build-mujoco --config Release
```

Linux/容器中使用同样的 CMake 命令即可。程序只依赖标准库和系统 socket API。

项目还提供基于 ROS 2 Jazzy Desktop Full 的 MuJoCo 派生镜像配置。首次构建并启动：

```powershell
cd D:\JazzyCWork\seeed_arm_console
docker compose -f .\docker-compose.gateway.yml -f .\docker-compose.mujoco.yml up -d --build
```

该配置下载 MuJoCo 3.12.0、加载 `assets/robot/b601_rs/mujoco/scene.xml`，并继续只暴露 `127.0.0.1:50051`，不启动任何图形窗口。

从项目根目录验证容器内的真实 MuJoCo 数据和控制闭环：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\verify-gateway.ps1 -ExpectedSource mujoco -CheckControl
```

预期输出：

```text
telemetry=OK source=mujoco ... tf=10 actual=1
control=OK jog accepted
```

如果 Docker 构建环境无法访问 GitHub，可先将 MuJoCo 压缩包放到可访问的镜像地址，并覆盖 `MUJOCO_URL`；压缩包仍会使用固定 SHA-256 校验，校验值变化时需同步更新 Compose 参数。

## 运行

```powershell
.\build\Release\arm_console_mock_gateway.exe 50051
```

使用 MuJoCo 驱动时，第二个参数传入 XML 模型路径；也可以设置 `ARM_CONSOLE_MODEL`：

```powershell
./build-mujoco/Release/arm_console_mock_gateway.exe 50051 D:\JazzyCWork\seeed_arm_console\assets\robot\b601_rs\mujoco\scene.xml
```

启动 `seeed_arm_console`，在右侧“数据流”选择“TCP 网关”，地址填 `127.0.0.1:50051`，点击“连接”。关闭桌面端或点击“断开”后，网关会回到监听状态。

## 帧格式

每行是一个 JSON 对象，单位为弧度、弧度/秒和纳秒：

```json
{"sequence":1,"timestamp_ns":20000000,"source":"mock","quality":"valid","joint_position_rad":[0,0,0,0,0,0],"joint_velocity_rad_s":[0.336,0.32,0.28,0.2,0.1,0.05]}
```

每帧还包含 `tf`（10 个链路变换，含左右夹爪）以及各一个规划点和实际点，方便直接检查时间线数据。这里的 TF 是确定性运动学占位值，不代表真实模型的动力学结果。

## 控制联调

TCP 客户端可以向同一连接发送换行分隔的控制 JSON，网关会返回确认帧：

```json
{"type":"enable","enabled":true}
{"type":"jog","joint_index":0,"step_rad":0.05}
{"type":"stop"}
```

确认帧为 `{"type":"ack","status":"accepted|rejected","reason":"..."}`。网关会校验关节索引；生产控制仍需迁移到 `arm_console.proto` 定义的会话、心跳、限位和速度检查。

这是用于打通 UI 的传输样例。生产网关应按照 `protocol/arm_console.proto` 生成 gRPC 服务，并把 MuJoCo、ROS 2 或回放数据映射到同一组字段；TCP JSON 不作为最终控制协议。
