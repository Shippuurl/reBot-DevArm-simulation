# C++ SDK 接入指南

`rebot_arm_sdk` 为 C++17 应用提供 `ArmGateway` 和 `ArmPlanner` 客户端。库内部处理
protobuf，业务代码使用 `rebot::sdk` 中的值类型和 gRPC 状态。

## 构建与安装

在仓库根目录执行：

```bash
sudo apt-get install -y libgrpc++-dev libprotobuf-dev protobuf-compiler-grpc
cmake -S sdk/cpp -B /tmp/rebot-arm-sdk-build
cmake --build /tmp/rebot-arm-sdk-build -j2
sudo cmake --install /tmp/rebot-arm-sdk-build --prefix /usr/local
```

构建会编译网关和规划示例；安装步骤导出 `rebot::rebot_arm_sdk` CMake target。业务工程
链接该 target 即可使用。

在运行中的 MuJoCo Compose 网关中验证 SDK：

```bash
scripts/run_cpp_sdk_smoke.sh
```

规划服务位于其他容器或主机时，设置 `CPP_SDK_PLANNER_ADDRESS` 可让冒烟测试继续运行规划
示例。

## 网关调用

```cpp
#include "rebot_sdk/client.hpp"
#include <iostream>
#include <stdexcept>
#include <vector>

int main() {
    auto channel = grpc::CreateChannel(
        "127.0.0.1:50051", grpc::InsecureChannelCredentials());
    rebot::sdk::ArmGatewayClient gateway(channel, "pick-cell-controller");

    rebot::sdk::ConnectionInfo info;
    auto status = gateway.handshake(&info);
    if (!status.ok()) {
        throw std::runtime_error(status.error_message());
    }

    rebot::sdk::CommandAck ack;
    if (!gateway.enable(true, &ack).ok() || !ack.accepted()) {
        throw std::runtime_error("gateway enable rejected: " + ack.reason);
    }

    status = gateway.subscribe_telemetry(20, [](const rebot::sdk::TelemetryFrame& frame) {
        std::cout << "sequence=" << frame.sequence << '\n';
        // 回调返回 false 可结束本次订阅。
        return frame.sequence < 100;
    });
    if (!status.ok()) {
        throw std::runtime_error(status.error_message());
    }
    gateway.stop(false, &ack);
}
```

轨迹按“预检 → 执行”提交：

```cpp
// 在已完成握手的 ArmGatewayClient 上调用
std::vector<rebot::sdk::TrajectoryPoint> points = {
    {0, {0, 0, 0, 0, 0, 0}, {}},
    {2'000'000'000, {0.2, 0, 0, 0, 0, 0}, {}},
};
gateway.execute_trajectory(points, /*dry_run=*/true, &ack);
if (ack.accepted()) {
    gateway.execute_trajectory(points, /*dry_run=*/false, &ack);
}
```

可用控制方法：`enable`、`jog`、`execute_trajectory`、`pause`、`resume`、`speed_scale`、
`stop` 和 `reset_fault`。执行倍率范围为 `0.1–2.0`。

## 规划调用

```cpp
#include "rebot_sdk/client.hpp"

int main() {
    auto planner_channel = grpc::CreateChannel(
        "127.0.0.1:50053", grpc::InsecureChannelCredentials());
    rebot::sdk::ArmPlannerClient planner(planner_channel);

    rebot::sdk::PoseTarget target;
    target.position_m = {0.25, 0.0, 0.30};
    rebot::sdk::IKResult result;
    auto status = planner.solve_ik(
        target, &result, "pick-ik", {}, true, 0.02);
    if (status.ok() && result.success) {
        // 把候选轨迹交给 ArmGateway 做预检和执行。
    }
    return status.ok() && result.success ? 0 : 1;
}
```

位置用米，关节角用弧度，速度用弧度/秒，四元数按 `x,y,z,w` 排列。目标坐标系默认为
`world`；装配阶段支持 `APPROACH`、`MATE` 和 `RETRACT`。

规划示例默认连接 `127.0.0.1:50053`：

```bash
/tmp/rebot-arm-sdk-build/rebot_sdk_planner_example
```

## 会话、遥测与错误

`handshake` 返回独立 `session_id`，客户端会在后续控制和遥测请求中自动携带。控制命令
默认附带当前 Unix 纳秒时间戳；网关接受当前时间前 5 秒至后 1 秒的非零时间戳。

所有方法返回 `grpc::Status`。网关拒绝命令时，查看 `CommandAck.status` 和
`CommandAck.reason`；遥测流的网络状态由 `subscribe_telemetry` 返回。一个客户端对象
按单一控制线程顺序调用，回调中只做轻量处理。流断开后重新握手，按 250 ms 起步、5 s
封顶的退避策略订阅。

## TLS

本机仿真使用回环连接。跨主机时由应用创建 TLS channel，并按部署要求附加 metadata：

```cpp
grpc::SslCredentialsOptions tls;
tls.pem_root_certs = "<CA PEM contents>";
auto channel = grpc::CreateChannel(
    "robot.example:50051", grpc::SslCredentials(tls));
rebot::sdk::ArmGatewayClient gateway(
    channel, "pick-cell-controller", {{"authorization", "Bearer <token>"}});
```

证书签发、权限、网络隔离和真实设备安全配置见[安全部署](/deployment/security)。

## 版本

当前 SDK 为 `0.1.0`，协议为 `arm.console.v1`。协议字段和兼容规则见
[SDK 与协议边界](/architecture/sdk-boundary)。
