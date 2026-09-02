# reBot Arm C++ SDK

这是 `arm.console.v1` 的 C++ 客户端边界。公共头文件只包含 gRPC channel 和
transport-neutral 数据结构；protobuf 生成类型只在 SDK 实现内部使用。外部工程不
需要依赖平台的 Rerun Viewer、MuJoCo、Pinocchio、ProxSuite、URDF 或 ROS 2。

## 构建

需要 C++17、Protobuf 开发包和 gRPC C++ 开发包：

```bash
sudo apt-get install -y libgrpc++-dev libprotobuf-dev protobuf-compiler-grpc
cmake -S sdk/cpp -B /tmp/rebot-arm-sdk-build
cmake --build /tmp/rebot-arm-sdk-build -j2
sudo cmake --install /tmp/rebot-arm-sdk-build --prefix /usr/local
```

构建会从仓库的 `protocol/arm_console.proto` 生成私有 C++ stubs，并生成
`rebot_sdk_gateway_example`。安装还会导出 `rebot::rebot_arm_sdk` CMake target；发布
SDK 时应把静态库、头文件和 CMake package 一起归档，消费方不需要 `protoc`。

## 网关调用

```cpp
#include "rebot_sdk/client.hpp"

auto channel = grpc::CreateChannel(
    "127.0.0.1:50051", grpc::InsecureChannelCredentials());
rebot::sdk::ArmGatewayClient gateway(channel, "pick-cell-controller");

// Optional auth/audit metadata is copied to every RPC:
// rebot::sdk::ArmGatewayClient gateway(channel, "pick-cell-controller",
//     {{"authorization", "Bearer <token>"}});

rebot::sdk::ConnectionInfo info;
if (!gateway.handshake(&info).ok()) { /* handle transport error */ }

rebot::sdk::CommandAck ack;
gateway.enable(true, &ack);
gateway.subscribe_telemetry(20, [](const rebot::sdk::TelemetryFrame& frame) {
    // Consume values; return false to cancel this stream.
    return frame.sequence < 100;
});
gateway.stop(false, &ack);
```

`ArmPlannerClient` 提供 `solve_ik` 和 `plan_trajectory`，返回同一组独立数据类型；
规划结果必须经过网关的 dry-run 和最终安全复核。单位是米、弧度、弧度/秒和纳秒，
四元数使用 `x,y,z,w`。

构建还会生成 `rebot_sdk_planner_example`，默认连接 `127.0.0.1:50053`，用于验证
IK 和轨迹响应：

```bash
/tmp/rebot-arm-sdk-build/rebot_sdk_planner_example
```

`ArmGatewayClient` 会话由服务端握手创建并自动用于控制/遥测；每个客户端会得到独立
的 session。控制命令自动填入当前 Unix 纳秒时间戳，服务端仍负责所有限位、碰撞、
时序和新鲜度检查。

## TLS

SDK 不决定安全策略，channel 由消费方创建。跨网络时使用
`grpc::SslCredentials(grpc::SslCredentialsOptions{...})`，并在网关侧启用 TLS、客户
端身份认证、授权、会话吊销和审计。当前本地仿真端点是不加密的回环服务，不能直接
暴露到局域网或互联网。

完整 API 以 `include/rebot_sdk/client.hpp` 为准；遥测流断开时返回 gRPC status，
应用应按退避策略重新握手和订阅，不要把断流当作安全停机。

客户端对象不保证并发调用安全；一个对象建议由一个控制线程和一个遥测消费线程
协调使用，或在上层串行化调用。不同对象可以共享同一个 gRPC channel。
