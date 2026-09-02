from __future__ import annotations

import unittest

from rebot_sdk import ArmGatewayClient, PoseTarget, TrajectoryPoint
from rebot_sdk._generated import arm_console_pb2 as pb
from rebot_sdk.client import _telemetry_from_proto


class SdkUnitTests(unittest.TestCase):
    def test_pose_and_trajectory_are_transport_neutral(self) -> None:
        target = PoseTarget((0.25, 0.0, 0.30))
        self.assertEqual(target.frame_id, "world")
        point = TrajectoryPoint(0, (0.0,) * 6)
        self.assertEqual(point.time_from_start_ns, 0)

    def test_address_rejects_shell_line_breaks(self) -> None:
        with self.assertRaises(ValueError):
            ArmGatewayClient("rerun+http://127.0.0.1:9876/\nproxy")
        with self.assertRaises(ValueError):
            ArmGatewayClient(client_name="external project")

    def test_telemetry_conversion_includes_depth_cloud(self) -> None:
        frame = pb.TelemetryFrame(
            sequence=3,
            timestamp_ns=4,
            source=pb.MUJOCO,
            quality=pb.VALID,
            joint_position_rad=[0.0] * 6,
            joint_velocity_rad_s=[0.0] * 6,
            point_clouds=[
                pb.PointCloudFrame(sensor="overhead_depth", positions_xyz=[1.0, 2.0, 3.0])
            ],
        )
        converted = _telemetry_from_proto(frame)
        self.assertEqual(converted.source, "MUJOCO")
        self.assertEqual(converted.point_clouds[0].positions_xyz, ((1.0, 2.0, 3.0),))


if __name__ == "__main__":
    unittest.main()
