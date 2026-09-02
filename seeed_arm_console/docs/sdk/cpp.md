# C++ SDK

`sdk/cpp` 是 `arm.console.v1` 的 C++ 客户端原型。公共头文件不暴露生成的
protobuf 类型，外部工程只使用 `rebot::sdk` 下的值类型和 gRPC status；平台内部
的 MuJoCo、Pinocchio、ProxSuite、URDF 和 Rerun 均不在 API 边界内。

## 从源码构建

需要 C++17、Protobuf 和 gRPC C++ 开发包：

```bash
sudo apt-get install -y libgrpc++-dev libprotobuf-dev protobuf-compiler-grpc
cmake -S sdk/cpp -B /tmp/rebot-arm-sdk-build
cmake --build /tmp/rebot-arm-sdk-build -j2
sudo cmake --install /tmp/rebot-arm-sdk-build --prefix /usr/local
```

构建过程从仓库唯一协议源 `protocol/arm_console.proto` 生成私有 stubs，并产生
`rebot_arm_sdk` 静态库、`rebot_sdk_gateway_example` 和 `rebot_sdk_planner_example`。
安装后提供 `rebot::rebot_arm_sdk` CMake target。发布时需要把库、头文件和
CMake package 一起归档，使消费方不必安装 `protoc`。

主机没有 gRPC/Protobuf 开发包时，可在项目的 ROS 2 Jazzy + MuJoCo 容器中执行同样
命令：

```bash
docker exec arm-console-gateway bash -lc \
  'cmake -S /work/seeed_arm_console/sdk/cpp -B /tmp/rebot-arm-sdk-build && \
   cmake --build /tmp/rebot-arm-sdk-build -j2'
```

## 网关客户端

```cpp
#include "rebot_sdk/client.hpp"

auto channel = grpc::CreateChannel(
    "127.0.0.1:50051", grpc::InsecureChannelCredentials());
rebot::sdk::ArmGatewayClient gateway(channel, "pick-cell-controller");

// 可选的认证/审计 metadata 会附加到每个 RPC：
// rebot::sdk::ArmGatewayClient gateway(channel, "pick-cell-controller",
//     {{"authorization", "Bearer <token>"}});

rebot::sdk::ConnectionInfo info;
if (!gateway.handshake(&info).ok()) {
    // inspect grpc::Status and stop before issuing control commands
}

rebot::sdk::CommandAck ack;
gateway.enable(true, &ack);
gateway.subscribe_telemetry(20, [](const rebot::sdk::TelemetryFrame& frame) {
    // positions are radians; return false to stop this stream
    return frame.sequence < 100;
});
gateway.stop(false, &ack);
```

可用方法包括 `jog`、`execute_trajectory`、`pause`、`resume`、`speed_scale`、
`reset_fault` 和 `subscribe_telemetry`。轨迹建议先用 `dry_run=true`，服务端仍会
执行最终限位、碰撞、时序和新鲜度检查。每次握手返回独立 `session_id`，SDK 会在后
续请求中自动使用；`client_name` 只用于服务端日志和未来授权策略，不是认证凭据。

## 规划客户端

`ArmPlannerClient` 提供：

- `solve_ik(PoseTarget, ...)`：返回关节候选、限位结果、碰撞摘要和规划元数据；
- `plan_trajectory(start, goal, ...)`：返回带时间和速度字段的候选轨迹。

`assembly_phase` 只能取空值、`APPROACH`、`MATE` 或 `RETRACT`；其他值会在客户端
返回 `INVALID_ARGUMENT`，避免把拼写错误静默降级为默认阶段。

规划客户端不会执行机器人控制。外部工程应把成功轨迹交给 `ArmGatewayClient` 做
dry-run 和执行前复核。长度使用米，角度使用弧度，四元数为 `x,y,z,w`；默认目标
帧是 `world`。

规划示例默认连接本机 50053：

```bash
/tmp/rebot-arm-sdk-build/rebot_sdk_planner_example
```

网关运行在 Compose 容器时，也可以使用仓库脚本完成容器内构建和网关 smoke：

```bash
scripts/run_cpp_sdk_smoke.sh
```

若规划服务从容器网络可达，设置 `CPP_SDK_PLANNER_ADDRESS` 后脚本会继续运行规划
示例，例如 `CPP_SDK_PLANNER_ADDRESS=172.18.0.1:50053`。

## TLS 与错误

SDK 的 channel 由调用方创建，因此既可使用 `grpc::InsecureChannelCredentials()`
连接本机仿真，也可使用 `grpc::SslCredentials(...)` 连接 TLS 服务。跨主机部署前
必须同时在服务端启用 TLS、客户端身份认证、授权、会话吊销和审计；当前仿真网关
只适合回环或受信任网络。

所有方法返回 `grpc::Status`；遥测回调返回 `false` 时表示主动取消，服务端/网络
错误则原样返回。SDK 不自动把断流解释为安全停机，应用需要按退避策略重新握手和
订阅，并由硬件 watchdog/急停承担真机安全职责。

客户端对象不保证并发调用安全；建议每个对象由单一控制线程使用，遥测回调只做轻量
数据转发，复杂处理交给上层队列。

当前版本是源码构建原型，后续发布工作包括跨平台 CI、安装包、版本兼容矩阵和 TLS
端到端测试；Rust SDK 已提供 v0.1 源码包，见 [Rust SDK](/sdk/rust)。
