# reBot Arm Python SDK

`rebot-arm-sdk` is the public Python client for the platform gRPC contract. It
does not import or expose the platform's Viewer, MuJoCo, Pinocchio, ProxSuite,
URDF, or ROS 2 implementation. External projects only need a reachable
`ArmGateway` and/or `ArmPlanner` endpoint.

## Install from this checkout

```bash
python3 -m pip install ./sdk/python
```

The SDK requires Python 3.10 or newer, `grpcio`, and `protobuf`. The generated
protobuf modules are bundled, so consumers do not need `protoc` or a checkout
of the platform repository.

## Gateway example

```python
from rebot_sdk import ArmGatewayClient

with ArmGatewayClient("127.0.0.1:50051", client_name="pick-cell-controller") as gateway:
    info = gateway.connect()
    print(info.source, info.dof)
    print(gateway.enable().status)

    for frame in gateway.subscribe_telemetry(max_rate_hz=20):
        print(frame.sequence, frame.joint_position_rad)
        if frame.sequence >= 10:
            break

    gateway.stop()
```

`ArmGatewayClient` sends a fresh Unix-nanosecond command timestamp by default.
Pass a short, whitespace-free `client_name` to identify the consuming
application in server logs and future authorization policy.
The server still owns all safety checks; an SDK acknowledgement is not a
replacement for a hardware watchdog or emergency stop.

## Planner example

```python
from rebot_sdk import ArmPlannerClient, PoseTarget

with ArmPlannerClient("127.0.0.1:50053") as planner:
    result = planner.solve_ik(PoseTarget((0.25, 0.0, 0.30)))
    if result.success:
        print(result.joint_position_rad)
```

The public dataclasses are deliberately transport-neutral. `PoseTarget`
angles use an `x,y,z,w` quaternion, lengths are meters, joint values are
radians, and timestamps are nanoseconds.

## TLS and metadata

Local simulation uses insecure gRPC on loopback. For a remotely reachable
service, enable TLS and provide the CA/client material:

```python
gateway = ArmGatewayClient(
    "robot.example:50051",
    secure=True,
    root_certificates=open("ca.pem", "rb").read(),
    certificate_chain=open("client.pem", "rb").read(),
    private_key=open("client-key.pem", "rb").read(),
    metadata=(("authorization", "Bearer <token>"),),
)
```

The current simulation gateway remains loopback-oriented and uses insecure
credentials. TLS, per-client identity, authorization, and network policy are
required before exposing a production gateway outside a trusted network.

## Compatibility

The SDK sends `arm.console.v1` during handshake and rejects a server that
reports another protocol version. Additive protobuf fields remain compatible;
breaking changes require a new protocol package/version and a corresponding
SDK release.
