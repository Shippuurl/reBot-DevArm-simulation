# 控制协议

`arm_console.proto` 是 C++ 网关与 Windows 桌面端的唯一 v1 数据边界。

- 控制使用 unary RPC；遥测使用 server-streaming。
- 所有运动命令带 `session_id`、`command_id` 和时间戳。
- 遥测帧带 `sequence`、`timestamp_ns`、`source`、`quality`。
- 角度使用弧度，长度使用米；网关不启动任何 GUI。

C++ 侧使用 protoc 和 gRPC C++ 插件生成客户端/服务端代码；Rust 侧后续使用 tonic 生成同一协议的类型。协议变更必须递增 package 版本，不复用已有字段编号。

在 gRPC 网关尚未就绪时，`cpp/mock_gateway` 使用换行分隔 JSON 输出同一组遥测字段，供 Windows UI 做本地联调。该格式是临时适配层，不替代本协议，也不承载生产控制命令。
