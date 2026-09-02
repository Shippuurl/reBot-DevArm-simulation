"""Public synchronous clients for the ArmGateway and ArmPlanner services."""

from __future__ import annotations

import time
import uuid
from collections.abc import Iterable, Iterator, Sequence
from typing import Any

import grpc

from ._generated import arm_console_pb2 as pb
from ._generated import arm_console_pb2_grpc as rpc
from .models import (
    AllowedCollisionPair,
    CollisionSummary,
    CommandAck,
    ConnectionInfo,
    Contact,
    IKResult,
    ImageFrame,
    PlanningMetadata,
    PointCloud,
    PoseTarget,
    TelemetryFrame,
    TrajectoryPlanResult,
    TrajectoryPoint,
    TrajectoryState,
    Transform,
    PROTOCOL_VERSION,
)


Metadata = Sequence[tuple[str, str | bytes]]


class RebotRpcError(RuntimeError):
    """A transport or server error with its gRPC status preserved."""

    def __init__(self, message: str, *, code: grpc.StatusCode | None = None) -> None:
        super().__init__(message)
        self.code = code


def _request_id(value: str | None) -> str:
    return value or uuid.uuid4().hex


def _call_unary(method: Any, request: Any, *, timeout: float | None, metadata: Metadata) -> Any:
    try:
        return method(request, timeout=timeout, metadata=metadata)
    except grpc.RpcError as exc:
        raise RebotRpcError(exc.details() or str(exc), code=exc.code()) from exc


def _enum_name(enum_type: Any, value: int) -> str:
    try:
        return enum_type.Name(value)
    except ValueError:
        return f"UNKNOWN_{value}"


def _assembly_phase(value: str | int | None) -> int:
    if value is None:
        return pb.ASSEMBLY_PHASE_UNSPECIFIED
    if isinstance(value, str):
        try:
            return pb.AssemblyPhase.Value(value.upper())
        except ValueError as exc:
            raise ValueError(f"unknown assembly phase: {value!r}") from exc
    return int(value)


def _pose_to_proto(target: PoseTarget) -> pb.PoseTarget:
    return pb.PoseTarget(
        position_x_m=target.position_m[0],
        position_y_m=target.position_m[1],
        position_z_m=target.position_m[2],
        rotation_x=target.rotation_xyzw[0],
        rotation_y=target.rotation_xyzw[1],
        rotation_z=target.rotation_xyzw[2],
        rotation_w=target.rotation_xyzw[3],
        frame_id=target.frame_id,
    )


def _trajectory_to_proto(point: TrajectoryPoint) -> pb.TrajectoryPoint:
    result = pb.TrajectoryPoint(
        time_from_start_ns=point.time_from_start_ns,
        position_rad=point.position_rad,
    )
    if point.velocity_rad_s:
        result.velocity_rad_s.extend(point.velocity_rad_s)
    return result


def _collision_from_proto(value: pb.CollisionSummary) -> CollisionSummary:
    return CollisionSummary(
        checked=value.checked,
        collision_free=value.collision_free,
        checked_pairs=value.checked_pairs,
        contacts=tuple(value.contacts),
        minimum_distance_m=value.minimum_distance_m,
    )


def _metadata_from_proto(value: pb.PlanningMetadata) -> PlanningMetadata:
    return PlanningMetadata(
        model_version=value.model_version,
        solver=value.solver,
        random_seed=value.random_seed,
        elapsed_ns=value.elapsed_ns,
    )


def _trajectory_state_from_proto(value: pb.TrajectoryPoint) -> TrajectoryState:
    return TrajectoryState(
        time_from_start_ns=value.time_from_start_ns,
        position_rad=tuple(value.position_rad),
        velocity_rad_s=tuple(value.velocity_rad_s),
    )


def _telemetry_from_proto(value: pb.TelemetryFrame) -> TelemetryFrame:
    point_clouds = []
    for cloud in value.point_clouds:
        raw = tuple(cloud.positions_xyz)
        positions = tuple(tuple(raw[index : index + 3]) for index in range(0, len(raw) - 2, 3))
        point_clouds.append(
            PointCloud(
                sensor=cloud.sensor,
                positions_xyz=positions,
                colors_rgba=tuple(cloud.colors_rgba),
            )
        )
    return TelemetryFrame(
        sequence=value.sequence,
        timestamp_ns=value.timestamp_ns,
        source=_enum_name(pb.SourceKind, value.source),
        quality=_enum_name(pb.SampleQuality, value.quality),
        sim_time_ns=value.sim_time_ns,
        wall_time_ns=value.wall_time_ns,
        joint_position_rad=tuple(value.joint_position_rad),
        joint_velocity_rad_s=tuple(value.joint_velocity_rad_s),
        tf=tuple(
            Transform(
                parent=item.parent,
                child=item.child,
                translation_m=(item.translation_x_m, item.translation_y_m, item.translation_z_m),
                rotation_xyzw=(item.rotation_x, item.rotation_y, item.rotation_z, item.rotation_w),
            )
            for item in value.tf
        ),
        planned_trajectory=tuple(_trajectory_state_from_proto(item) for item in value.planned_trajectory),
        actual_trajectory=tuple(_trajectory_state_from_proto(item) for item in value.actual_trajectory),
        images=tuple(
            ImageFrame(
                sensor=item.sensor,
                width=item.width,
                height=item.height,
                encoding=item.encoding,
                data=bytes(item.data),
            )
            for item in value.images
        ),
        point_clouds=tuple(point_clouds),
        contacts=tuple(
            Contact(
                first_geom=item.first_geom,
                second_geom=item.second_geom,
                distance_m=item.distance_m,
                normal_force_n=item.normal_force_n,
            )
            for item in value.contacts
        ),
    )


class _ChannelClient:
    def __init__(
        self,
        address: str,
        *,
        secure: bool = False,
        root_certificates: bytes | None = None,
        certificate_chain: bytes | None = None,
        private_key: bytes | None = None,
        metadata: Metadata | None = None,
        channel_options: Sequence[tuple[str, Any]] = (),
    ) -> None:
        if not address or any(character.isspace() for character in address):
            raise ValueError("address must be a non-empty host:port string without whitespace")
        self.address = address
        self.secure = secure or any(
            value is not None for value in (root_certificates, certificate_chain, private_key)
        )
        self.root_certificates = root_certificates
        self.certificate_chain = certificate_chain
        self.private_key = private_key
        self.metadata: Metadata = tuple(metadata or ())
        self.channel_options = tuple(channel_options)
        self._channel: grpc.Channel | None = None

    def _get_channel(self) -> grpc.Channel:
        if self._channel is None:
            if self.secure:
                credentials = grpc.ssl_channel_credentials(
                    root_certificates=self.root_certificates,
                    private_key=self.private_key,
                    certificate_chain=self.certificate_chain,
                )
                self._channel = grpc.secure_channel(
                    self.address, credentials, options=self.channel_options
                )
            else:
                self._channel = grpc.insecure_channel(self.address, options=self.channel_options)
        return self._channel

    def close(self) -> None:
        if self._channel is not None:
            self._channel.close()
            self._channel = None

    def __enter__(self) -> _ChannelClient:
        return self

    def __exit__(self, *_: object) -> None:
        self.close()


class ArmGatewayClient(_ChannelClient):
    """Client for control and telemetry without any Viewer dependency.

    ``address`` is a normal gRPC endpoint such as ``127.0.0.1:50051``.  Use
    ``secure=True`` plus TLS material for deployments outside a trusted local
    network.  The client does not automatically reconnect telemetry streams;
    callers should recreate ``subscribe_telemetry`` after a stream error.
    """

    def __init__(
        self,
        address: str = "127.0.0.1:50051",
        *,
        client_name: str = "rebot-sdk-python",
        **kwargs: Any,
    ) -> None:
        super().__init__(address, **kwargs)
        if not client_name or any(character.isspace() for character in client_name):
            raise ValueError("client_name must be non-empty and contain no whitespace")
        self.client_name = client_name
        self._stub: rpc.ArmGatewayStub | None = None
        self._connection: ConnectionInfo | None = None

    @property
    def connection(self) -> ConnectionInfo | None:
        return self._connection

    def connect(self, *, timeout: float | None = 5.0) -> ConnectionInfo:
        channel = self._get_channel()
        self._stub = rpc.ArmGatewayStub(channel)
        reply = _call_unary(
            self._stub.Handshake,
            pb.ConnectRequest(client_name=self.client_name, protocol_version=PROTOCOL_VERSION),
            timeout=timeout,
            metadata=self.metadata,
        )
        if reply.protocol_version and reply.protocol_version != PROTOCOL_VERSION:
            raise RebotRpcError(
                f"unsupported gateway protocol {reply.protocol_version!r}; expected {PROTOCOL_VERSION}"
            )
        self._connection = ConnectionInfo(
            session_id=reply.session_id,
            protocol_version=reply.protocol_version or PROTOCOL_VERSION,
            source=_enum_name(pb.SourceKind, reply.source),
            dof=reply.dof,
        )
        return self._connection

    def close(self) -> None:
        self._connection = None
        self._stub = None
        super().close()

    def _connected(self, timeout: float | None) -> tuple[rpc.ArmGatewayStub, ConnectionInfo]:
        if self._stub is None or self._connection is None:
            self.connect(timeout=timeout)
        assert self._stub is not None and self._connection is not None
        return self._stub, self._connection

    def _command(
        self,
        field: str,
        payload: Any,
        *,
        command_id: str | None,
        timeout: float | None,
        client_timestamp_ns: int | None,
    ) -> CommandAck:
        stub, connection = self._connected(timeout)
        timestamp = time.time_ns() if client_timestamp_ns is None else int(client_timestamp_ns)
        request = pb.ControlCommand(
            header=pb.CommandHeader(
                session_id=connection.session_id,
                command_id=_request_id(command_id),
                client_timestamp_ns=timestamp,
            )
        )
        getattr(request, field).CopyFrom(payload)
        reply = _call_unary(stub.Command, request, timeout=timeout, metadata=self.metadata)
        return CommandAck(
            command_id=reply.command_id,
            status=_enum_name(pb.AckStatus, reply.status),
            reason=reply.reason,
        )

    def enable(
        self,
        enabled: bool = True,
        *,
        command_id: str | None = None,
        timeout: float | None = 5.0,
        client_timestamp_ns: int | None = None,
    ) -> CommandAck:
        return self._command(
            "enable",
            pb.EnableCommand(enabled=enabled),
            command_id=command_id,
            timeout=timeout,
            client_timestamp_ns=client_timestamp_ns,
        )

    def stop(
        self,
        emergency: bool = False,
        *,
        command_id: str | None = None,
        timeout: float | None = 5.0,
        client_timestamp_ns: int | None = None,
    ) -> CommandAck:
        return self._command(
            "stop",
            pb.StopCommand(emergency=emergency),
            command_id=command_id,
            timeout=timeout,
            client_timestamp_ns=client_timestamp_ns,
        )

    def jog(
        self,
        joint_index: int,
        step_rad: float,
        *,
        speed_limit_rad_s: float = 0.0,
        command_id: str | None = None,
        timeout: float | None = 5.0,
        client_timestamp_ns: int | None = None,
    ) -> CommandAck:
        return self._command(
            "jog",
            pb.JogCommand(
                joint_index=joint_index,
                step_rad=step_rad,
                speed_limit_rad_s=speed_limit_rad_s,
            ),
            command_id=command_id,
            timeout=timeout,
            client_timestamp_ns=client_timestamp_ns,
        )

    def execute_trajectory(
        self,
        points: Iterable[TrajectoryPoint],
        *,
        dry_run: bool = False,
        command_id: str | None = None,
        timeout: float | None = 10.0,
        client_timestamp_ns: int | None = None,
    ) -> CommandAck:
        payload = pb.ExecuteTrajectoryCommand(dry_run=dry_run)
        payload.points.extend(_trajectory_to_proto(point) for point in points)
        return self._command(
            "execute_trajectory",
            payload,
            command_id=command_id,
            timeout=timeout,
            client_timestamp_ns=client_timestamp_ns,
        )

    def reset_fault(
        self,
        *,
        command_id: str | None = None,
        timeout: float | None = 5.0,
        client_timestamp_ns: int | None = None,
    ) -> CommandAck:
        return self._command(
            "reset_fault",
            pb.ResetFaultCommand(),
            command_id=command_id,
            timeout=timeout,
            client_timestamp_ns=client_timestamp_ns,
        )

    def pause(
        self,
        *,
        command_id: str | None = None,
        timeout: float | None = 5.0,
        client_timestamp_ns: int | None = None,
    ) -> CommandAck:
        return self._command(
            "pause",
            pb.PauseCommand(),
            command_id=command_id,
            timeout=timeout,
            client_timestamp_ns=client_timestamp_ns,
        )

    def resume(
        self,
        *,
        command_id: str | None = None,
        timeout: float | None = 5.0,
        client_timestamp_ns: int | None = None,
    ) -> CommandAck:
        return self._command(
            "resume",
            pb.ResumeCommand(),
            command_id=command_id,
            timeout=timeout,
            client_timestamp_ns=client_timestamp_ns,
        )

    def speed_scale(
        self,
        scale: float,
        *,
        command_id: str | None = None,
        timeout: float | None = 5.0,
        client_timestamp_ns: int | None = None,
    ) -> CommandAck:
        return self._command(
            "speed_scale",
            pb.SpeedScaleCommand(scale=scale),
            command_id=command_id,
            timeout=timeout,
            client_timestamp_ns=client_timestamp_ns,
        )

    def subscribe_telemetry(
        self,
        *,
        max_rate_hz: int = 50,
        timeout: float | None = None,
    ) -> Iterator[TelemetryFrame]:
        stub, connection = self._connected(5.0)
        if not 0 <= max_rate_hz <= 200:
            raise ValueError("max_rate_hz must be between 0 and 200")
        try:
            stream = stub.SubscribeTelemetry(
                pb.TelemetryRequest(session_id=connection.session_id, max_rate_hz=max_rate_hz),
                timeout=timeout,
                metadata=self.metadata,
            )
            for frame in stream:
                yield _telemetry_from_proto(frame)
        except grpc.RpcError as exc:
            raise RebotRpcError(exc.details() or str(exc), code=exc.code()) from exc


class ArmPlannerClient(_ChannelClient):
    """Client for the headless planning service.

    Returned values are SDK dataclasses; no planner implementation or Rerun
    object is imported into the consumer process.
    """

    def __init__(self, address: str = "127.0.0.1:50053", **kwargs: Any) -> None:
        super().__init__(address, **kwargs)
        self._stub: rpc.ArmPlannerStub | None = None

    def _get_stub(self) -> rpc.ArmPlannerStub:
        if self._stub is None:
            self._stub = rpc.ArmPlannerStub(self._get_channel())
        return self._stub

    def close(self) -> None:
        self._stub = None
        super().close()

    def solve_ik(
        self,
        target: PoseTarget,
        *,
        request_id: str | None = None,
        seed_position_rad: Iterable[float] = (),
        check_collisions: bool = False,
        minimum_distance_threshold_m: float = 0.0,
        assembly_phase: str | int | None = None,
        allowed_collision_pairs: Iterable[AllowedCollisionPair | tuple[str, str]] = (),
        timeout: float | None = 15.0,
    ) -> IKResult:
        request = pb.IKRequest(
            request_id=_request_id(request_id),
            check_collisions=check_collisions,
            minimum_distance_threshold_m=minimum_distance_threshold_m,
            assembly_phase=_assembly_phase(assembly_phase),
        )
        request.target.CopyFrom(_pose_to_proto(target))
        request.seed_position_rad.extend(seed_position_rad)
        for pair in allowed_collision_pairs:
            if not isinstance(pair, AllowedCollisionPair):
                pair = AllowedCollisionPair(*pair)
            request.allowed_collision_pairs.add(first=pair.first, second=pair.second)
        reply = _call_unary(
            self._get_stub().SolveIK, request, timeout=timeout, metadata=self.metadata
        )
        return IKResult(
            request_id=reply.request_id,
            success=reply.success,
            joint_position_rad=tuple(reply.joint_position_rad),
            within_limits=reply.within_limits,
            collision=_collision_from_proto(reply.collision),
            metadata=_metadata_from_proto(reply.metadata),
            reason=reply.reason,
        )

    def plan_trajectory(
        self,
        start: PoseTarget,
        goal: PoseTarget,
        *,
        request_id: str | None = None,
        max_rate_hz: int = 20,
        dry_run: bool = False,
        check_collisions: bool = False,
        minimum_distance_threshold_m: float = 0.0,
        assembly_phase: str | int | None = None,
        allowed_collision_pairs: Iterable[AllowedCollisionPair | tuple[str, str]] = (),
        timeout: float | None = 20.0,
    ) -> TrajectoryPlanResult:
        request = pb.TrajectoryPlanRequest(
            request_id=_request_id(request_id),
            max_rate_hz=max_rate_hz,
            dry_run=dry_run,
            check_collisions=check_collisions,
            minimum_distance_threshold_m=minimum_distance_threshold_m,
            assembly_phase=_assembly_phase(assembly_phase),
        )
        request.start.CopyFrom(_pose_to_proto(start))
        request.goal.CopyFrom(_pose_to_proto(goal))
        for pair in allowed_collision_pairs:
            if not isinstance(pair, AllowedCollisionPair):
                pair = AllowedCollisionPair(*pair)
            request.allowed_collision_pairs.add(first=pair.first, second=pair.second)
        reply = _call_unary(
            self._get_stub().PlanTrajectory, request, timeout=timeout, metadata=self.metadata
        )
        return TrajectoryPlanResult(
            request_id=reply.request_id,
            success=reply.success,
            points=tuple(
                TrajectoryPoint(
                    time_from_start_ns=item.time_from_start_ns,
                    position_rad=tuple(item.position_rad),
                    velocity_rad_s=tuple(item.velocity_rad_s),
                )
                for item in reply.points
            ),
            collision=_collision_from_proto(reply.collision),
            metadata=_metadata_from_proto(reply.metadata),
            reason=reply.reason,
        )
