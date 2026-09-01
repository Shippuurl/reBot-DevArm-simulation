# Rerun 视图

Rerun 是记录和可视化层，用于观察模型、关节状态、TF、图像、点云、规划轨迹和实际轨迹。它不是实时控制协议，也不应作为急停链路。

## 窗口边界

当前桌面端采用 Rerun 官方 `extend_viewer_ui` 模式：外层是项目自己的 `eframe` 应用，内部嵌入 `re_viewer::App`。控制、Jog、状态和参数面板与 Rerun 3D/时间线位于同一个原生窗口和 egui/wgpu 渲染循环中，不需要 X11 或 VcXsrv。

Rerun Viewer 扩展接口不是稳定 API，项目将 Rerun 锁定在 `0.36.3`，升级时必须重新编译验证。`native_viewer` 会增加编译时间和发布体积。

## 推荐实体路径

```text
robot/
  model/manifest
  model/mujoco_scene
  frames/<link>/model
  joints/joint_1 ... joint_6
  frames/base_link, frames/gripper_end, frames/camera
  planned_trajectory
  actual_trajectory
sensors/camera/front/image
sensors/depth/points
diagnostics/faults
events/commands
```

实体路径一旦公开，应保持向后兼容。型号差异使用 metadata 或命名空间表达，不要为每个硬件复制一套 UI。

## 记录内容

| 数据 | 建议频率 | 说明 |
| --- | ---: | --- |
| 关节位置/速度 | 50–200 Hz | 记录最新状态，UI 读取快照 |
| TF | 30–100 Hz | 与机器人时间戳一致 |
| 规划轨迹 | 事件级 | 记录规划版本、校验摘要和完整轨迹 |
| 实际轨迹 | 50–200 Hz | 与规划轨迹使用相同坐标系 |
| 图像 | 采集频率 | 大数据流建议独立记录或降采样 |
| 点云 | 采集频率 | 注意带宽和磁盘容量 |
| 故障/命令 | 事件级 | 必须包含 command_id 和结果 |

## 时间与坐标

优先使用后端单调时钟和设备时间戳，记录时保存时钟来源。所有位姿都要声明父坐标系、单位和右手系约定；当前记录器为每个 `robot/frames/<link>` 写入 `CoordinateFrame`，并使用命名空间 frame id，避免显式 TF 图与实体路径脱节。MuJoCo 网关先把世界位姿转换为父节点相对位姿，再发送到记录器；发现时间跳变或 TF 缺失时在视图中显示诊断标记。

## 集成方式

验证嵌入式 Viewer 外壳（参考 Rerun 官方 [extend_viewer_ui 示例](https://github.com/rerun-io/rerun/tree/0.36.3/examples/rust/extend_viewer_ui)）：

```powershell
cargo run --features embedded-viewer --bin embedded_viewer
```

该入口在 `127.0.0.1:9876` 启动 Rerun gRPC 接收器，并把自定义 egui 面板放在左侧，剩余区域交给 `re_viewer::App`。支持 Rerun SDK 的数据源可直接连接此端口；当前 C++ 网关仍通过控制端口输出 JSON，后续由遥测桥接转发到 Rerun。控制状态和遥测桥接沿用同一入口，不再另起 Rerun 窗口。

实时数据通过 Rerun `LogReceiver`/gRPC 输入，离线记录仍由桌面端的“开始录制”按钮写入 `recordings/`。记录器运行在 UI 的遥测边界之外：写盘失败只结束记录，不会阻塞停止、使能或 Jog 控制。记录会同时写入右手系坐标约定、模型清单中所有 visual mesh、对应 URDF 和 MuJoCo `scene.xml`（资源存在时）。

当前 RS 模型的可视化清单位于 `assets/robot/b601_rs/rerun/model.json`。每个 visual mesh 都记录在 `robot/frames/<link>/model/<index>_<name>` 下，link 的动态 TF 会自动作用到其全部组件；清单中的 `albedo_factor` 会为 STL 补上稳定的材质颜色，修改模型时只需更新清单，不需要修改 Rerun 记录器。记录中还会保存 `robot/model/manifest`，便于回放时核对资源版本。

桌面端默认使用 `assets/robot/b601_rs`。需要测试其它资源目录时，在启动前设置 `ROBOT_MODEL_ROOT`，目录必须包含 `rerun/model.json` 及清单引用的 mesh，例如：

```powershell
$env:ROBOT_MODEL_ROOT = "assets/robot/b601_rs"
cargo run --features rerun-recording
```

同一目录内存在多个变体时，可用 `ROBOT_MODEL_MANIFEST` 选择清单文件（例如 `rerun/model_fixend.json`）。项目已为 `b601_dm` 的夹爪和固定末端资源提供对应清单。

首次安装 Viewer：

```powershell
py -m pip install rerun-sdk==0.36.3
```

生成一份不依赖网关的样例记录：

```powershell
cargo run --features rerun-recording --bin rerun_sample -- recordings/sample.rrd
```

在 Win11 原生打开记录（不需要 X11/VcXsrv）：

```powershell
rerun recordings/sample.rrd
```

也可以在桌面端连接 `127.0.0.1:50051` 后点击“开始录制”，再用相同命令打开生成的 `robot-*.rrd`。Rerun 只负责记录和观察，不参与控制协议。

## 性能策略

- 图像和点云使用有界缓存，达到上限时丢弃旧帧而不是阻塞控制线程。协议中的 `images` 和 `point_clouds` 字段分别映射为 Rerun 图像实体与 `Points3D`；颜色数组只有在与点数量一致时才写入。
- 高频数值使用批量记录或降采样；规划和故障事件必须完整保留。
- 3D 渲染掉帧只影响观察，不改变驱动的控制周期。
