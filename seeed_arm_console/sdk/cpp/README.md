# reBot Arm C++ SDK

`rebot_arm_sdk` 是 `arm.console.v1` 的 C++17 客户端库。`rebot::sdk` 提供
`ArmGatewayClient`、`ArmPlannerClient` 及对应的值类型，protobuf 代码封装在库内。

## 构建与安装

```bash
sudo apt-get install -y libgrpc++-dev libprotobuf-dev protobuf-compiler-grpc
cmake -S sdk/cpp -B /tmp/rebot-arm-sdk-build
cmake --build /tmp/rebot-arm-sdk-build -j2
sudo cmake --install /tmp/rebot-arm-sdk-build --prefix /usr/local
```

安装导出 `rebot::rebot_arm_sdk` CMake target。运行中的 MuJoCo 网关可用以下脚本验证：

```bash
scripts/run_cpp_sdk_smoke.sh
```

## 最短调用

```cpp
#include "rebot_sdk/client.hpp"

int main() {
    auto channel = grpc::CreateChannel(
        "127.0.0.1:50051", grpc::InsecureChannelCredentials());
    rebot::sdk::ArmGatewayClient gateway(channel, "pick-cell-controller");
    rebot::sdk::ConnectionInfo info;
    if (!gateway.handshake(&info).ok()) {
        return 1;
    }
    rebot::sdk::CommandAck ack;
    if (!gateway.enable(true, &ack).ok() || !ack.accepted()) {
        return 1;
    }
    if (!gateway.stop(false, &ack).ok() || !ack.accepted()) {
        return 1;
    }
    return 0;
}
```

轨迹按 `dry_run=true` 预检、`dry_run=false` 执行的顺序提交。网关控制、规划、会话和
TLS 示例见 [C++ SDK 接入指南](../../docs/sdk/cpp.md)；协议详情见[SDK 与协议边界](../../docs/architecture/sdk-boundary.md)。
