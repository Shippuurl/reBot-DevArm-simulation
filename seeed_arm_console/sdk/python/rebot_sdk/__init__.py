"""Official Python SDK for the reBot ArmGateway and ArmPlanner services.

Only the protobuf transport contract is required at runtime.  The package has
no dependency on the platform Viewer, MuJoCo, Pinocchio, ProxSuite, URDF, or
ROS 2 implementation details.
"""

from .client import ArmGatewayClient, ArmPlannerClient, RebotRpcError
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
)

__all__ = [
    "AllowedCollisionPair",
    "ArmGatewayClient",
    "ArmPlannerClient",
    "CollisionSummary",
    "CommandAck",
    "ConnectionInfo",
    "Contact",
    "IKResult",
    "ImageFrame",
    "PlanningMetadata",
    "PointCloud",
    "PoseTarget",
    "RebotRpcError",
    "TelemetryFrame",
    "TrajectoryPlanResult",
    "TrajectoryPoint",
    "TrajectoryState",
    "Transform",
]

__version__ = "0.1.0"
