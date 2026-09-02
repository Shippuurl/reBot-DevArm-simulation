"""Transport-neutral public data types for the reBot gRPC SDK.

The SDK deliberately exposes dataclasses instead of protobuf messages.  This
keeps consumers independent from the platform's generated-code layout and
from the Viewer, Pinocchio, ProxSuite, MuJoCo, and URDF implementations.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable


PROTOCOL_VERSION = "arm.console.v1"


def _vector(values: Iterable[float], *, size: int, name: str) -> tuple[float, ...]:
    result = tuple(float(value) for value in values)
    if len(result) != size:
        raise ValueError(f"{name} must contain {size} values, got {len(result)}")
    return result


@dataclass(frozen=True)
class PoseTarget:
    """A Cartesian target expressed in a named frame (normally ``world``)."""

    position_m: tuple[float, float, float]
    rotation_xyzw: tuple[float, float, float, float] = (0.0, 0.0, 0.0, 0.0)
    frame_id: str = "world"

    def __post_init__(self) -> None:
        object.__setattr__(self, "position_m", _vector(self.position_m, size=3, name="position_m"))
        object.__setattr__(
            self,
            "rotation_xyzw",
            _vector(self.rotation_xyzw, size=4, name="rotation_xyzw"),
        )
        if not self.frame_id:
            raise ValueError("frame_id must not be empty")


@dataclass(frozen=True)
class AllowedCollisionPair:
    first: str
    second: str


@dataclass(frozen=True)
class TrajectoryPoint:
    time_from_start_ns: int
    position_rad: tuple[float, ...]
    velocity_rad_s: tuple[float, ...] = ()

    def __post_init__(self) -> None:
        if self.time_from_start_ns < 0:
            raise ValueError("time_from_start_ns must be non-negative")
        object.__setattr__(self, "position_rad", tuple(float(v) for v in self.position_rad))
        object.__setattr__(self, "velocity_rad_s", tuple(float(v) for v in self.velocity_rad_s))


@dataclass(frozen=True)
class Transform:
    parent: str
    child: str
    translation_m: tuple[float, float, float]
    rotation_xyzw: tuple[float, float, float, float]


@dataclass(frozen=True)
class Contact:
    first_geom: str
    second_geom: str
    distance_m: float
    normal_force_n: float


@dataclass(frozen=True)
class ImageFrame:
    sensor: str
    width: int
    height: int
    encoding: str
    data: bytes


@dataclass(frozen=True)
class PointCloud:
    sensor: str
    positions_xyz: tuple[tuple[float, float, float], ...]
    colors_rgba: tuple[int, ...] = ()


@dataclass(frozen=True)
class TrajectoryState:
    time_from_start_ns: int
    position_rad: tuple[float, ...]
    velocity_rad_s: tuple[float, ...]


@dataclass(frozen=True)
class TelemetryFrame:
    sequence: int
    timestamp_ns: int
    source: str
    quality: str
    sim_time_ns: int
    wall_time_ns: int
    joint_position_rad: tuple[float, ...]
    joint_velocity_rad_s: tuple[float, ...]
    tf: tuple[Transform, ...] = ()
    planned_trajectory: tuple[TrajectoryState, ...] = ()
    actual_trajectory: tuple[TrajectoryState, ...] = ()
    images: tuple[ImageFrame, ...] = ()
    point_clouds: tuple[PointCloud, ...] = ()
    contacts: tuple[Contact, ...] = ()


@dataclass(frozen=True)
class CollisionSummary:
    checked: bool
    collision_free: bool
    checked_pairs: int
    contacts: tuple[str, ...]
    minimum_distance_m: float


@dataclass(frozen=True)
class PlanningMetadata:
    model_version: str
    solver: str
    random_seed: int
    elapsed_ns: int


@dataclass(frozen=True)
class ConnectionInfo:
    session_id: str
    protocol_version: str
    source: str
    dof: int


@dataclass(frozen=True)
class CommandAck:
    command_id: str
    status: str
    reason: str

    @property
    def accepted(self) -> bool:
        return self.status == "ACCEPTED"


@dataclass(frozen=True)
class IKResult:
    request_id: str
    success: bool
    joint_position_rad: tuple[float, ...]
    within_limits: bool
    collision: CollisionSummary
    metadata: PlanningMetadata
    reason: str


@dataclass(frozen=True)
class TrajectoryPlanResult:
    request_id: str
    success: bool
    points: tuple[TrajectoryPoint, ...]
    collision: CollisionSummary
    metadata: PlanningMetadata
    reason: str
