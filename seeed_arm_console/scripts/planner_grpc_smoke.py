#!/usr/bin/env python3
"""Smoke-test ArmPlanner through the public Python SDK.

The canonical wire-compatibility test remains ``gateway_grpc_smoke.py``; this
script intentionally does not generate protobuf bindings or import server
implementation details.
"""
import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "sdk" / "python"))

try:
    from rebot_sdk import ArmPlannerClient, PoseTarget
except ModuleNotFoundError as exc:
    raise SystemExit(
        f"缺少 {exc.name}，请先执行: python3 -m pip install -r requirements-planning.txt"
    ) from exc


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--address", default="127.0.0.1:50053")
    args = parser.parse_args()
    pose = lambda x, y, z, frame_id="world": PoseTarget((x, y, z), frame_id=frame_id)

    with ArmPlannerClient(args.address) as planner:
        ik = planner.solve_ik(pose(0.25, 0.0, 0.30), request_id="smoke-ik")
        if ik.request_id != "smoke-ik" or len(ik.joint_position_rad) != 6:
            raise RuntimeError(f"invalid IK response: {ik}")
        if ik.metadata.random_seed >= 4:
            raise RuntimeError(f"IK did not report a valid multi-start candidate: {ik.metadata}")

        oriented = planner.solve_ik(
            PoseTarget((0.3017, 0.0, 0.2177), rotation_xyzw=(0.0, 0.0, 0.0, 1.0)),
            request_id="smoke-orientation",
        )
        if not oriented.success:
            raise RuntimeError(f"orientation target was not reached: {oriented}")

        invalid_frame = planner.solve_ik(
            pose(0.0, 0.0, 0.0, frame_id="link1"), request_id="smoke-invalid-frame"
        )
        if invalid_frame.success or "world" not in invalid_frame.reason:
            raise RuntimeError(f"invalid target frame was unexpectedly accepted: {invalid_frame}")

        checked = planner.solve_ik(
            pose(0.25, 0.0, 0.30), request_id="smoke-collision", check_collisions=True
        )
        if not checked.collision.checked or checked.collision.checked_pairs == 0:
            raise RuntimeError(f"collision check was not performed: {checked}")

        strict = planner.solve_ik(
            pose(0.25, 0.0, 0.30),
            request_id="smoke-strict-margin",
            check_collisions=True,
            minimum_distance_threshold_m=1.0,
        )
        if strict.success or strict.collision.collision_free:
            raise RuntimeError(f"strict clearance was unexpectedly accepted: {strict}")

        plan = planner.plan_trajectory(
            pose(0.25, 0.0, 0.30),
            pose(0.20, 0.05, 0.32),
            request_id="smoke-plan",
        )
        if not plan.success or len(plan.points) < 2:
            raise RuntimeError(f"invalid trajectory response: {plan}")
        if any(len(point.velocity_rad_s) != 6 for point in plan.points) or any(
            later.time_from_start_ns <= earlier.time_from_start_ns
            for earlier, later in zip(plan.points, plan.points[1:])
        ):
            raise RuntimeError("trajectory timing or velocity fields are invalid")
        accelerations = []
        for earlier, later in zip(plan.points, plan.points[1:]):
            dt = (later.time_from_start_ns - earlier.time_from_start_ns) * 1e-9
            accelerations.extend(
                (next_velocity - previous_velocity) / dt
                for previous_velocity, next_velocity in zip(
                    earlier.velocity_rad_s, later.velocity_rad_s
                )
            )
        max_acceleration = max(map(abs, accelerations), default=0.0)
        if max_acceleration > 2.05:
            raise RuntimeError("trajectory acceleration exceeds the 2 rad/s^2 bound")

    print(
        f"planner=OK ik_joints={len(ik.joint_position_rad)} "
        f"ik_candidate={ik.metadata.random_seed} trajectory_points={len(plan.points)} "
        f"collision_pairs={checked.collision.checked_pairs} strict_margin=rejected "
        f"max_acceleration={max_acceleration:.3f} model={plan.metadata.model_version} sdk=python"
    )


if __name__ == "__main__":
    main()
