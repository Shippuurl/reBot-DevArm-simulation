#!/usr/bin/env python3
"""Exercise the Pinocchio planner -> ArmGateway hand-off through the SDK."""
import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "sdk" / "python"))

try:
    from rebot_sdk import ArmGatewayClient, ArmPlannerClient, PoseTarget
except ModuleNotFoundError as exc:
    raise SystemExit(
        f"缺少 {exc.name}，请先执行: python3 -m pip install -r requirements-planning.txt"
    ) from exc


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--planner-address", default="127.0.0.1:50053")
    parser.add_argument("--gateway-address", default="127.0.0.1:50051")
    args = parser.parse_args()
    start = PoseTarget((0.25, 0.0, 0.30))
    goal = PoseTarget((0.20, 0.05, 0.32))

    with ArmPlannerClient(args.planner_address) as planner:
        plan = planner.plan_trajectory(
            start,
            goal,
            request_id="planner-gateway-smoke",
            max_rate_hz=20,
            check_collisions=True,
            assembly_phase="MATE",
        )
    if not plan.success or len(plan.points) < 2:
        raise RuntimeError(f"planner returned no executable trajectory: {plan}")

    with ArmGatewayClient(args.gateway_address, client_name="planner-gateway-smoke") as gateway:
        handshake = gateway.connect()
        enabled = gateway.enable(command_id="planner-gateway-enable")
        if not enabled.accepted:
            raise RuntimeError(f"gateway enable rejected: {enabled}")

        dry_run = gateway.execute_trajectory(
            plan.points, dry_run=True, command_id="planner-gateway-dry-run"
        )
        if not dry_run.accepted:
            raise RuntimeError(f"gateway dry-run rejected planner output: {dry_run}")

        execute = gateway.execute_trajectory(
            plan.points, dry_run=False, command_id="planner-gateway-execute"
        )
        if not execute.accepted:
            raise RuntimeError(f"gateway execution rejected planner output: {execute}")

        stream = gateway.subscribe_telemetry(max_rate_hz=50, timeout=10)
        observed = None
        for frame in stream:
            if len(frame.planned_trajectory) >= len(plan.points) and len(frame.actual_trajectory) == 1:
                observed = frame
                break
        stream.close()
        if observed is None:
            raise RuntimeError("gateway telemetry did not expose the submitted planner trajectory")

        stopped = gateway.stop(command_id="planner-gateway-stop")
        if not stopped.accepted:
            raise RuntimeError(f"gateway stop rejected: {stopped}")
        reset = gateway.reset_fault(command_id="planner-gateway-reset")
        if not reset.accepted:
            raise RuntimeError(f"gateway reset_fault rejected: {reset}")

    print(
        f"planner_gateway=OK planner_points={len(plan.points)} "
        f"telemetry_sequence={observed.sequence} planned_points={len(observed.planned_trajectory)} "
        f"controls=dry_run,execute,stop,reset_fault sdk=python session={handshake.session_id}"
    )


if __name__ == "__main__":
    main()
