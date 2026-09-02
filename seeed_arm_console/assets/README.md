# B601 资源

`assets/` 收纳 Seeed reBot Arm B601-DM / B601-RS 的 URDF、网格、MuJoCo 场景和 Viewer
界面资源。运行时入口和资源校验由 `assets/manifest.json` 记录。

## 目录

```text
assets/
├── licenses/                 字体与模型许可文本
├── fonts/                    Inter、JetBrains Mono、Noto Sans SC
├── design/                   颜色、间距、圆角和图标令牌
├── icons/                    可复用 UI 图标
├── robot/
│   ├── b601_dm/              B601-DM URDF、网格和 Rerun 清单
│   └── b601_rs/              B601-RS URDF、网格、MuJoCo 场景和 Rerun 清单
└── ui/                       应用欢迎页和品牌图片
```

## 模型入口

| 模型 | URDF / 场景 | 网格 | 用途 |
| --- | --- | --- | --- |
| B601-DM（夹爪） | `robot/b601_dm/urdf/reBot_B601_DM_with_gripper.urdf` | `robot/b601_dm/meshes/` | Rerun 模型与 TF |
| B601-DM（固定末端） | `robot/b601_dm/urdf/reBot-DevArm_fixend.urdf` | `robot/b601_dm/meshes_fixend/` | 固定末端模型 |
| B601-RS | `robot/b601_rs/urdf/00-arm-rs_asm-v3.urdf` | `robot/b601_rs/meshes/` | Rerun 模型与规划 |
| B601-RS MuJoCo | `robot/b601_rs/mujoco/scene.xml` | XML 引用 `../meshes/` | 基础动力学场景 |
| B601-RS 抓取场景 | `robot/b601_rs/mujoco/rs_grasp_scene.xml` | XML 引用 `../meshes/` | 桌面、方块、相机和夹爪 |

每个 URDF、网格集合和 MuJoCo XML 按版本配套使用。长度采用米，角度采用弧度。

## Rerun 与 MuJoCo

Rerun 记录器读取模型清单，解析 URDF 的 link/joint 和网格，再写入 `robot/frames/<link>`
实体。运行时 TF、关节、轨迹和传感器沿用同一实体树。MuJoCo 从
`robot/b601_rs/mujoco/` 加载 XML，场景中的网格路径相对于该目录解析。

STL 保留上游几何精度；需要减小传输体积时，在构建缓存中转换为 glTF/GLB，并保留 STL
作为源文件。

## UI 资源

`fonts/`、`design/` 和 `icons/` 可供 Viewer 或其他 egui 应用复用，与控制协议和 SDK
独立。设计令牌位于 `design/tokens.json`；通用 PNG 图标位于 `icons/reusable/`。

## 来源与许可

- 硬件和 DM 模型：[`reBot-DevArm`](https://gitee.com/long-yongjun9930/reBot-DevArm)，
  本地提交 `b868a7f38e9039a32f5a393cfedea3606efb00ed`，许可文本见
  `licenses/reBot-DevArm-LICENSE`（CERN-OHL-W-2.0）。
- ROS 2 URDF/网格：Seeed [`reBotArmController_ROS2`](https://github.com/Seeed-Projects/reBotArmController_ROS2)，
  本地提交 `e134941b71236523e831f15b470bc81186b0649f`。上游包声明 Apache-2.0，
  对外分发前请按上游仓库再次核对许可文件。
- RS MuJoCo 模型：[`LAN-GER/reBot-B601-RS-for-mujoco_sim`](https://github.com/LAN-GER/reBot-B601-RS-for-mujoco_sim)，
  本地提交 `1249cb6efdf393ba636056fc41df30dc6ba389aa`。上游未附许可证，取得授权前仅
  用于本地开发。

字体的来源和许可证见 [`licenses/FONTS.md`](licenses/FONTS.md)。清单中的 70 个核心文件
记录了大小和 SHA-256；资源变更后请同步更新 `manifest.json` 并复核许可。
