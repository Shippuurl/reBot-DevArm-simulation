"""Minimal SDK-only gateway consumer; no platform internals are imported."""

from __future__ import annotations

import argparse

from rebot_sdk import ArmGatewayClient


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--address", default="127.0.0.1:50051")
    parser.add_argument("--frames", type=int, default=10)
    args = parser.parse_args()

    with ArmGatewayClient(args.address) as gateway:
        info = gateway.connect()
        print(f"connected source={info.source} dof={info.dof} session={info.session_id}")
        print(f"enable={gateway.enable().status}")
        for index, frame in enumerate(gateway.subscribe_telemetry(max_rate_hz=20)):
            depth_points = sum(len(cloud.positions_xyz) for cloud in frame.point_clouds)
            print(
                f"frame={frame.sequence} joints={len(frame.joint_position_rad)} "
                f"tf={len(frame.tf)} depth_points={depth_points}"
            )
            if index + 1 >= args.frames:
                break
        print(f"stop={gateway.stop().status}")


if __name__ == "__main__":
    main()
