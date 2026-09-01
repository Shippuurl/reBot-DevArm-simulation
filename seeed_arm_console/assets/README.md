# Seeed B601 资源包

本目录收集 Seeed reBot Arm B601-DM / B601-RS 的模型、网格和界面图片，供 Rerun 可视化、MuJoCo 仿真和 egui 应用读取。目录不包含控制器、ROS 节点或 UI 业务代码。

## 目录

```text
assets/
├── licenses/
│   ├── reBot-DevArm-LICENSE
│   └── FONTS.md
├── fonts/                # Inter UI、JetBrains Mono 数据和 Noto Sans SC 中文回退字体
├── design/               # 通用颜色、间距、圆角和图标令牌
├── icons/                # 应用专用图标资源（通用图标由 Rust painter 绘制）
├── robot/
│   ├── b601_dm/
│   │   ├── urdf/
│   │   ├── meshes/          # B601-DM 带夹爪模型
│   │   └── meshes_fixend/   # B601-DM 固定末端模型
│   └── b601_rs/
│       ├── urdf/
│       ├── meshes/
│       └── mujoco/
└── ui/
    ├── rebot_arm_b601.png
    └── reBot-DevArm-banner.png
```

## 通用 UI 资源

`fonts/`、`design/` 和 `icons/` 与机器人模型解耦，可复用于任何仿真或调试工作区。字体和令牌来源于本地 `shadcn-rs` 资源，当前代码通过 `src/design.rs` 和 `src/icons.rs` 使用，不依赖品牌图片或 egui 版本不匹配的组件包。

## 模型与用途

| 模型 | 入口文件 | 网格目录 | 用途 |
| --- | --- | --- | --- |
| B601-DM（带夹爪） | `robot/b601_dm/urdf/reBot_B601_DM_with_gripper.urdf` | `robot/b601_dm/meshes/` | Rerun 机器人模型、TF 和关节状态展示 |
| B601-DM（固定末端） | `robot/b601_dm/urdf/reBot-DevArm_fixend.urdf` | `robot/b601_dm/meshes_fixend/` | 无夹爪末端的 Rerun 模型 |
| B601-RS | `robot/b601_rs/urdf/00-arm-rs_asm-v3.urdf` | `robot/b601_rs/meshes/` | Rerun 模型、TF 和关节状态展示 |
| B601-RS MuJoCo | `robot/b601_rs/mujoco/scene.xml` | XML 中的 `../meshes/` | 基础物理场景 |
| B601-RS 抓取场景 | `robot/b601_rs/mujoco/rs_grasp_scene.xml` | XML 中的 `../meshes/` | 桌面、方块、相机和夹爪抓取场景 |

DM 的两个 URDF 使用不同的网格集合，不能交叉替换。RS 的 URDF、STL 和 MuJoCo XML 也必须作为同一版本使用。

## Rerun 使用方式

Rerun 日志协议不负责解析 URDF 的 `package://` 资源。桥接层需要先解析 URDF，再将每个 link 的网格和变换记录到实体路径。例如：

1. 读取 URDF 中的 link、joint、视觉网格和材质。
2. 将 `package://rebotarm_bringup/description/meshes_b601_gripper/` 映射到 `robot/b601_dm/meshes/`；将 `package://rebotarm_bringup/description/meshes_rs/` 映射到 `robot/b601_rs/meshes/`。固定末端模型使用 `robot/b601_dm/meshes_fixend/` 的独立映射。
3. 将 STL 顶点/索引转换为 Rerun `Mesh3D`（或桥接层支持的 3D 资源格式），以 link 名称作为实体路径。
4. 在每个时间点记录 `Transform3D`；关节状态、TF、轨迹和传感器数据沿用同一实体树。

STL 文件保留上游几何精度；若目标平台需要更小的传输体积，可在构建缓存阶段转换为 glTF/GLB，但不要覆盖本目录的源网格。

## MuJoCo 使用方式

从 `robot/b601_rs/mujoco/` 作为工作目录加载 XML。`00_arm_rs_asm_v3.xml` 的网格目录已经修正为 `../meshes/`；`scene.xml` 和 `rs_grasp_scene.xml` 均引用本地 `00_arm_rs_asm_v3.xml`，不再依赖仓库外的 `third_party` 路径。XML 使用弧度和 SI 长度单位。

## egui 使用方式

`ui/` 中的 PNG 仅用于应用内品牌图、模型缩略图或欢迎页。Rust 中可通过 `include_bytes!` 嵌入，再交给 egui 的纹理加载接口。图标不从机械臂资源包复制，控制面板图标使用 `egui_material_icons` 或 `egui-phosphor` 等字体/矢量库。

## 来源与许可证

- 硬件与 DM 模型来源：[`reBot-DevArm`](https://gitee.com/long-yongjun9930/reBot-DevArm)，本地提交 `b868a7f38e9039a32f5a393cfedea3606efb00ed`。硬件资料采用仓库中的 CERN Open Hardware Licence Version 2 – Weakly Reciprocal，许可证文本见 `licenses/reBot-DevArm-LICENSE`。
- ROS 2 的 URDF/网格来源：Seeed [`reBotArmController_ROS2`](https://github.com/Seeed-Projects/reBotArmController_ROS2)，本地提交 `e134941b71236523e831f15b470bc81186b0649f`。各 ROS 包的 `package.xml` 声明 Apache-2.0，但该工作区没有单独的根许可证文本；公开分发前应再次核对上游许可。
- RS MuJoCo 模型来源：[`LAN-GER/reBot-B601-RS-for-mujoco_sim`](https://github.com/LAN-GER/reBot-B601-RS-for-mujoco_sim)，本地提交 `1249cb6efdf393ba636056fc41df30dc6ba389aa`。上游 README 明确说明当前没有 LICENSE；在取得许可或补充许可证前，RS MuJoCo XML/STL 仅作为本地开发资源，不应直接发布到公开仓库或产品包。

`manifest.json` 保存本目录资源的大小和 SHA-256，变更网格、模型或 UI 资源后请重新生成并复核来源许可。

当前清单包含 70 个资源文件（3 个 URDF、51 个 STL、3 个 MuJoCo XML、2 个 PNG、6 个字体、3 个设计/图标说明和 2 个许可证文本），总大小 149,080,228 字节（约 142.2 MiB）。
