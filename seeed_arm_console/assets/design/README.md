# UI 资源

这里存放应用级的视觉令牌和可复用资源，不包含机器人品牌或业务流程。

- `tokens.json`：颜色、间距、圆角、字体和图标基线。Rust 代码中的 `src/design.rs` 使用同一组值。
- `fonts/`：Inter 用于界面文字，JetBrains Mono 用于关节值、时间戳和日志，Noto Sans SC 作为中文回退字体。
- `icons/`：保留给应用专用 SVG 资源；通用导航图标由 `src/icons.rs` 使用 egui painter 绘制，避免引入与 egui 版本绑定的图标字体。

设计参考来自 `eguiLibrary/shadcn-rs-master` 的 tokens、theme 和 icons 实现，当前工程使用 egui 0.36 的原生 API 适配。
