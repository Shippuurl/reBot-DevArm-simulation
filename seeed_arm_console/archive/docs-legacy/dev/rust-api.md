# Rust API 文档

Rust API 使用 Cargo 的 `cargo doc` 生成，并单独托管在 GitHub Pages。这样可以保留完整的 crate、trait、类型和函数索引，而不会把底层自动生成内容混入 VitePress 的使用指南。

## 在线文档

> **发布地址待配置**：当前仓库还没有绑定 GitHub 组织和 Pages 域名。创建仓库后，将下面链接中的 `your-org` 替换为实际 GitHub 组织或用户名即可。

[打开 Rust API 文档（GitHub Pages）](https://your-org.github.io/seeed-arm-console/rustdoc/)

建议的最终地址格式：

```text
https://<github-owner>.github.io/seeed-arm-console/rustdoc/
```

## 本地生成

在工作区根目录执行：

```powershell
cargo doc --workspace --no-deps --open
```

如果只需要生成静态文件：

```powershell
cargo doc --workspace --no-deps
```

结果位于 `target/doc/`。发布脚本应将整个目录复制到 Pages 的 `rustdoc/` 子目录，并保留 `index.html` 和各 crate 子目录。

如果构建机已经下载可选依赖，也可以使用 `--all-features` 生成包含 Rerun SDK 的 API：

```powershell
cargo doc --workspace --no-deps --all-features
```

## 文档范围

Rustdoc 适合记录：

- `robot_workspace::telemetry` 的 `TelemetryFrame`、数据源 trait 和 TCP 适配器。
- UI 工作区中可复用的布局、主题和图标实现。
- 后续拆分出的控制协议与驱动适配器公共类型。

用户使用流程、系统架构和安全说明仍以本 VitePress 文档为准。
