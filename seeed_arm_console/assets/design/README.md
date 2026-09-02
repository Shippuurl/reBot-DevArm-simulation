# UI 设计资源

本目录保存 Viewer 使用的视觉令牌和字体约定：

- `tokens.json`：颜色、间距、圆角、字体和图标基线；Rust `src/design.rs` 与之对应；
- `fonts/`：界面使用 Inter，数值和日志使用 JetBrains Mono，中文回退使用 Noto Sans SC；
- `icons/`：按需加载的应用图标，与模型和控制协议分开维护。

设计参考来自 `eguiLibrary/shadcn-rs-master`，当前工程按 egui 0.36 的原生 API 使用。
