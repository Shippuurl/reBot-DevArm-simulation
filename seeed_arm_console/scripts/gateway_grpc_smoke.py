#!/usr/bin/env python3
"""Smoke-test the canonical C++ ArmGateway gRPC endpoint."""
import argparse
import importlib
import os
import sys
import tempfile
import time
from pathlib import Path

try:
    import grpc
    from grpc_tools import protoc
except ModuleNotFoundError as exc:
    raise SystemExit(
        f"缺少 {exc.name}，请先执行: python3 -m pip install -r requirements-planning.txt"
    ) from exc


def bindings():
    proto = str(Path(__file__).resolve().parents[1] / "protocol" / "arm_console.proto")
    out = tempfile.mkdtemp(prefix="arm-gateway-proto-")
    if protoc.main([
        "protoc",
        f"-I{os.path.dirname(proto)}",
        f"--python_out={out}",
        f"--grpc_python_out={out}",
        proto,
    ]) != 0:
        raise RuntimeError("failed to generate Python protobuf bindings")
    sys.path.insert(0, out)
    return importlib.import_module("arm_console_pb2"), importlib.import_module("arm_console_pb2_grpc")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--address", default="127.0.0.1:50051")
    args = parser.parse_args()
    pb, rpc = bindings()
    with grpc.insecure_channel(args.address) as channel:
        gateway = rpc.ArmGatewayStub(channel)
        reply = gateway.Handshake(
            pb.ConnectRequest(client_name="gateway-grpc-smoke", protocol_version="arm.console.v1"),
            timeout=5,
        )
        if reply.dof != 6 or not reply.session_id:
            raise RuntimeError(f"invalid handshake response: {reply}")

        second_reply = gateway.Handshake(
            pb.ConnectRequest(client_name="gateway-grpc-smoke-second", protocol_version="arm.console.v1"),
            timeout=5,
        )
        if not second_reply.session_id or second_reply.session_id == reply.session_id:
            raise RuntimeError("gateway did not issue independent client sessions")

        command = pb.ControlCommand(
            header=pb.CommandHeader(
                session_id=reply.session_id,
                command_id="smoke-enable",
                client_timestamp_ns=time.time_ns(),
            ),
            enable=pb.EnableCommand(enabled=True),
        )
        ack = gateway.Command(command, timeout=5)
        if ack.status != pb.ACCEPTED:
            raise RuntimeError(f"enable command rejected: {ack}")

        now_ns = time.time_ns()
        stale = gateway.Command(
            pb.ControlCommand(
                header=pb.CommandHeader(
                    session_id=reply.session_id,
                    command_id="smoke-stale-command",
                    client_timestamp_ns=now_ns - 10_000_000_000,
                ),
                enable=pb.EnableCommand(enabled=True),
            ),
            timeout=5,
        )
        if stale.status != pb.REJECTED:
            raise RuntimeError(f"stale command was unexpectedly accepted: {stale}")
        future = gateway.Command(
            pb.ControlCommand(
                header=pb.CommandHeader(
                    session_id=reply.session_id,
                    command_id="smoke-future-command",
                    client_timestamp_ns=now_ns + 10_000_000_000,
                ),
                enable=pb.EnableCommand(enabled=True),
            ),
            timeout=5,
        )
        if future.status != pb.REJECTED:
            raise RuntimeError(f"future command was unexpectedly accepted: {future}")

        jog = gateway.Command(
            pb.ControlCommand(
                header=pb.CommandHeader(session_id=reply.session_id, command_id="smoke-jog"),
                jog=pb.JogCommand(joint_index=0, step_rad=0.01),
            ),
            timeout=5,
        )
        if jog.status != pb.ACCEPTED:
            raise RuntimeError(f"jog command rejected: {jog}")

        dry_run = gateway.Command(
            pb.ControlCommand(
                header=pb.CommandHeader(session_id=reply.session_id, command_id="smoke-plan"),
                execute_trajectory=pb.ExecuteTrajectoryCommand(
                    dry_run=True,
                    points=[pb.TrajectoryPoint(time_from_start_ns=0, position_rad=[0.0] * reply.dof)],
                ),
            ),
            timeout=5,
        )
        if dry_run.status != pb.ACCEPTED:
            raise RuntimeError(f"dry-run trajectory rejected: {dry_run}")

        # Submit a short kinematic execution and observe it through the same
        # telemetry stream.  The simulation adapter owns the final safety
        # validation (limits, finite values, monotonic time and 2 rad/s cap),
        # so this exercises the planner-to-gateway hand-off rather than only
        # protobuf parsing.
        execute = gateway.Command(
            pb.ControlCommand(
                header=pb.CommandHeader(session_id=reply.session_id, command_id="smoke-execute"),
                execute_trajectory=pb.ExecuteTrajectoryCommand(
                    dry_run=False,
                    points=[
                        pb.TrajectoryPoint(time_from_start_ns=0, position_rad=[0.0] * reply.dof),
                        pb.TrajectoryPoint(
                            time_from_start_ns=2_000_000_000,
                            position_rad=[0.4, 0.0, 0.0, 0.0, 0.0, 0.0],
                            velocity_rad_s=[0.2, 0.0, 0.0, 0.0, 0.0, 0.0],
                        ),
                    ],
                ),
            ),
            timeout=5,
        )
        if execute.status != pb.ACCEPTED:
            raise RuntimeError(f"trajectory execution rejected: {execute}")

        speed_scale = gateway.Command(
            pb.ControlCommand(
                header=pb.CommandHeader(session_id=reply.session_id, command_id="smoke-speed-scale"),
                speed_scale=pb.SpeedScaleCommand(scale=1.5),
            ),
            timeout=5,
        )
        if speed_scale.status != pb.ACCEPTED:
            raise RuntimeError(f"speed scale update rejected: {speed_scale}")
        invalid_speed_scale = gateway.Command(
            pb.ControlCommand(
                header=pb.CommandHeader(session_id=reply.session_id, command_id="smoke-invalid-speed-scale"),
                speed_scale=pb.SpeedScaleCommand(scale=3.0),
            ),
            timeout=5,
        )
        if invalid_speed_scale.status != pb.REJECTED:
            raise RuntimeError(f"unsafe speed scale was unexpectedly accepted: {invalid_speed_scale}")

        invalid_trajectory = gateway.Command(
            pb.ControlCommand(
                header=pb.CommandHeader(session_id=reply.session_id, command_id="smoke-invalid-trajectory"),
                execute_trajectory=pb.ExecuteTrajectoryCommand(
                    dry_run=False,
                    points=[pb.TrajectoryPoint(time_from_start_ns=1, position_rad=[0.0] * reply.dof)],
                ),
            ),
            timeout=5,
        )
        if invalid_trajectory.status != pb.REJECTED:
            raise RuntimeError(f"unsafe trajectory was unexpectedly accepted: {invalid_trajectory}")

        invalid_jog = gateway.Command(
            pb.ControlCommand(
                header=pb.CommandHeader(session_id=reply.session_id, command_id="smoke-invalid-jog"),
                jog=pb.JogCommand(joint_index=0, step_rad=1.0),
            ),
            timeout=5,
        )
        if invalid_jog.status != pb.REJECTED:
            raise RuntimeError(f"unsafe jog was unexpectedly accepted: {invalid_jog}")

        stream = gateway.SubscribeTelemetry(
            pb.TelemetryRequest(session_id=reply.session_id, max_rate_hz=20), timeout=5
        )
        frame = None
        observed_execution = False
        stream_iter = iter(stream)
        try:
            frame = next(stream_iter)
        except StopIteration:
            pass
        if frame is None:
            raise RuntimeError("telemetry stream returned no frames")
        if frame.sequence == 0 or frame.wall_time_ns == 0 or len(frame.joint_position_rad) != reply.dof:
            raise RuntimeError(f"invalid telemetry frame: {frame}")
        pause = gateway.Command(
            pb.ControlCommand(
                header=pb.CommandHeader(session_id=reply.session_id, command_id="smoke-pause"),
                pause=pb.PauseCommand(),
            ),
            timeout=5,
        )
        if pause.status != pb.ACCEPTED:
            raise RuntimeError(f"pause command rejected: {pause}")
        paused_a = next(stream_iter)
        paused_b = next(stream_iter)
        paused_time_delta = abs(
            paused_b.actual_trajectory[0].time_from_start_ns
            - paused_a.actual_trajectory[0].time_from_start_ns
        ) if paused_a.actual_trajectory and paused_b.actual_trajectory else 0
        if paused_time_delta > 5_000_000:
            raise RuntimeError(f"trajectory advanced while paused: delta_ns={paused_time_delta}")
        resume = gateway.Command(
            pb.ControlCommand(
                header=pb.CommandHeader(session_id=reply.session_id, command_id="smoke-resume"),
                resume=pb.ResumeCommand(),
            ),
            timeout=5,
        )
        if resume.status != pb.ACCEPTED:
            raise RuntimeError(f"resume command rejected: {resume}")
        resumed_a = next(stream_iter)
        resumed_b = next(stream_iter)
        if not resumed_a.actual_trajectory or not resumed_b.actual_trajectory:
            raise RuntimeError("telemetry did not expose actual trajectory timing")
        if resumed_b.actual_trajectory[0].time_from_start_ns <= resumed_a.actual_trajectory[0].time_from_start_ns:
            raise RuntimeError("trajectory did not resume after pause")
        for candidate in (frame, paused_a, paused_b, resumed_a, resumed_b):
            if candidate.joint_position_rad and candidate.joint_position_rad[0] > 0.005:
                observed_execution = True
                break
        if not observed_execution:
            raise RuntimeError("trajectory execution was not visible in telemetry")
        if not frame.contacts:
            raise RuntimeError("MuJoCo telemetry did not expose contact summaries")
        if not frame.point_clouds:
            raise RuntimeError("MuJoCo telemetry did not expose the depth point cloud")
        depth_cloud = next(
            (cloud for cloud in frame.point_clouds if cloud.sensor == "overhead_depth"), None
        )
        if depth_cloud is None or len(depth_cloud.positions_xyz) < 3:
            raise RuntimeError(f"invalid MuJoCo depth point cloud: {frame.point_clouds}")
        stream.cancel()
        stopped = gateway.Command(
            pb.ControlCommand(
                header=pb.CommandHeader(session_id=reply.session_id, command_id="smoke-stop"),
                stop=pb.StopCommand(emergency=False),
            ),
            timeout=5,
        )
        if stopped.status != pb.ACCEPTED:
            raise RuntimeError(f"stop command rejected: {stopped}")
        reset = gateway.Command(
            pb.ControlCommand(
                header=pb.CommandHeader(session_id=reply.session_id, command_id="smoke-reset-fault"),
                reset_fault=pb.ResetFaultCommand(),
            ),
            timeout=5,
        )
        if reset.status != pb.ACCEPTED:
            raise RuntimeError(f"reset_fault rejected: {reset}")
        print(
            f"gateway_grpc=OK source={pb.SourceKind.Name(frame.source).lower()} "
            f"dof={len(frame.joint_position_rad)} sequence={frame.sequence} tf={len(frame.tf)} "
            f"contacts={len(frame.contacts)} depth_points={len(depth_cloud.positions_xyz) // 3} "
            "controls=enable,jog,dry_run,execute,pause,resume,speed_scale,stop,reset_fault "
            "invalid_jog=rejected invalid_trajectory=rejected invalid_speed_scale=rejected "
            "timestamp_guard=valid_accepted,stale_rejected,future_rejected"
            f" sessions=independent"
        )


if __name__ == "__main__":
    main()
