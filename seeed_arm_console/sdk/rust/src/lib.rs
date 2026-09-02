//! Public Rust client SDK for the `arm.console.v1` gRPC services.
//!
//! The SDK deliberately exposes transport-neutral value types instead of
//! protobuf messages. Consumers do not need the platform Viewer, MuJoCo,
//! Pinocchio, ProxSuite, URDF, or ROS 2 workspace. A channel can be supplied
//! by the caller to select insecure or TLS transport.

use std::time::{SystemTime, UNIX_EPOCH};

use tonic::{
    Request, Status,
    metadata::MetadataValue,
    transport::{Channel, Endpoint},
};

pub const PROTOCOL_VERSION: &str = "arm.console.v1";
pub type Metadata = Vec<(String, String)>;

mod generated {
    tonic::include_proto!("arm.console.v1");
}

#[derive(Clone, Debug, PartialEq)]
pub struct PoseTarget {
    pub position_m: [f64; 3],
    pub rotation_xyzw: [f64; 4],
    pub frame_id: String,
}

impl PoseTarget {
    pub fn new(position_m: [f64; 3]) -> Self {
        Self {
            position_m,
            rotation_xyzw: [0.0; 4],
            frame_id: "world".to_owned(),
        }
    }
}

impl Default for PoseTarget {
    fn default() -> Self {
        Self::new([0.0; 3])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllowedCollisionPair {
    pub first: String,
    pub second: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrajectoryPoint {
    pub time_from_start_ns: u64,
    pub position_rad: Vec<f64>,
    pub velocity_rad_s: Vec<f64>,
}

impl TrajectoryPoint {
    pub fn new(time_from_start_ns: u64, position_rad: impl Into<Vec<f64>>) -> Self {
        Self {
            time_from_start_ns,
            position_rad: position_rad.into(),
            velocity_rad_s: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transform {
    pub parent: String,
    pub child: String,
    pub translation_m: [f64; 3],
    pub rotation_xyzw: [f64; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Contact {
    pub first_geom: String,
    pub second_geom: String,
    pub distance_m: f64,
    pub normal_force_n: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageFrame {
    pub sensor: String,
    pub width: u32,
    pub height: u32,
    pub encoding: String,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PointCloud {
    pub sensor: String,
    pub positions_xyz: Vec<[f32; 3]>,
    pub colors_rgba: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TelemetryFrame {
    pub sequence: u64,
    pub timestamp_ns: u64,
    pub source: String,
    pub quality: String,
    pub sim_time_ns: u64,
    pub wall_time_ns: u64,
    pub joint_position_rad: Vec<f64>,
    pub joint_velocity_rad_s: Vec<f64>,
    pub tf: Vec<Transform>,
    pub planned_trajectory: Vec<TrajectoryPoint>,
    pub actual_trajectory: Vec<TrajectoryPoint>,
    pub images: Vec<ImageFrame>,
    pub point_clouds: Vec<PointCloud>,
    pub contacts: Vec<Contact>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionInfo {
    pub session_id: String,
    pub protocol_version: String,
    pub source: String,
    pub dof: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandAck {
    pub command_id: String,
    pub status: String,
    pub reason: String,
}

impl CommandAck {
    pub fn accepted(&self) -> bool {
        self.status == "ACCEPTED"
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollisionSummary {
    pub checked: bool,
    pub collision_free: bool,
    pub checked_pairs: u32,
    pub contacts: Vec<String>,
    pub minimum_distance_m: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanningMetadata {
    pub model_version: String,
    pub solver: String,
    pub random_seed: u64,
    pub elapsed_ns: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IKResult {
    pub request_id: String,
    pub success: bool,
    pub joint_position_rad: Vec<f64>,
    pub within_limits: bool,
    pub collision: CollisionSummary,
    pub metadata: PlanningMetadata,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrajectoryPlanResult {
    pub request_id: String,
    pub success: bool,
    pub points: Vec<TrajectoryPoint>,
    pub collision: CollisionSummary,
    pub metadata: PlanningMetadata,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GatewayCommand {
    Enable {
        enabled: bool,
    },
    Stop {
        emergency: bool,
    },
    Jog {
        joint_index: u32,
        step_rad: f64,
        speed_limit_rad_s: f64,
    },
    ExecuteTrajectory {
        points: Vec<TrajectoryPoint>,
        dry_run: bool,
    },
    ResetFault,
    Pause,
    Resume,
    SpeedScale {
        scale: f64,
    },
}

pub struct ArmGatewayClient {
    client: generated::arm_gateway_client::ArmGatewayClient<Channel>,
    client_name: String,
    metadata: Metadata,
    connection: Option<ConnectionInfo>,
}

impl ArmGatewayClient {
    /// Connect to an endpoint such as `http://127.0.0.1:50051`.
    pub async fn connect(
        endpoint: impl Into<String>,
        client_name: impl Into<String>,
    ) -> Result<Self, tonic::transport::Error> {
        let endpoint = Endpoint::new(endpoint.into())?;
        let channel = endpoint.connect().await?;
        Ok(Self::from_channel(channel, client_name))
    }

    /// Construct a client from a caller-owned channel. Use this for TLS,
    /// custom roots, interceptors, or channels shared by multiple services.
    pub fn from_channel(channel: Channel, client_name: impl Into<String>) -> Self {
        Self {
            client: generated::arm_gateway_client::ArmGatewayClient::new(channel),
            client_name: client_name.into(),
            metadata: Vec::new(),
            connection: None,
        }
    }

    pub fn with_metadata(mut self, metadata: impl IntoIterator<Item = (String, String)>) -> Self {
        self.metadata.extend(metadata);
        self
    }

    pub fn connection(&self) -> Option<&ConnectionInfo> {
        self.connection.as_ref()
    }

    /// Attach an existing server session to this client, useful when a
    /// control task uses a separate channel from the telemetry task.
    pub fn set_session_id(&mut self, session_id: impl Into<String>) {
        let session_id = session_id.into();
        if session_id.is_empty() {
            self.connection = None;
            return;
        }
        self.connection = Some(ConnectionInfo {
            session_id,
            protocol_version: PROTOCOL_VERSION.to_owned(),
            source: String::new(),
            dof: 0,
        });
    }

    pub async fn handshake(&mut self) -> Result<ConnectionInfo, Status> {
        if self.client_name.is_empty() || self.client_name.chars().any(char::is_whitespace) {
            return Err(Status::invalid_argument(
                "client_name must be non-empty and contain no whitespace",
            ));
        }
        let mut request = Request::new(generated::ConnectRequest {
            client_name: self.client_name.clone(),
            protocol_version: PROTOCOL_VERSION.to_owned(),
        });
        add_metadata(request.metadata_mut(), &self.metadata);
        let reply = self.client.handshake(request).await?.into_inner();
        if !reply.protocol_version.is_empty() && reply.protocol_version != PROTOCOL_VERSION {
            return Err(Status::unimplemented(format!(
                "unsupported gateway protocol {}; expected {}",
                reply.protocol_version, PROTOCOL_VERSION
            )));
        }
        let connection = ConnectionInfo {
            session_id: reply.session_id,
            protocol_version: if reply.protocol_version.is_empty() {
                PROTOCOL_VERSION.to_owned()
            } else {
                reply.protocol_version
            },
            source: enum_name_source(reply.source),
            dof: reply.dof,
        };
        self.connection = Some(connection.clone());
        Ok(connection)
    }

    async fn ensure_session(&mut self) -> Result<(), Status> {
        if self.connection.is_none() {
            self.handshake().await?;
        }
        Ok(())
    }

    pub async fn command(
        &mut self,
        command: GatewayCommand,
        command_id: impl Into<String>,
    ) -> Result<CommandAck, Status> {
        self.ensure_session().await?;
        let connection = self
            .connection
            .as_ref()
            .expect("ensure_session establishes a connection");
        let command_id = command_id.into();
        let mut request = generated::ControlCommand {
            header: Some(generated::CommandHeader {
                session_id: connection.session_id.clone(),
                command_id: if command_id.is_empty() {
                    request_id()
                } else {
                    command_id
                },
                client_timestamp_ns: now_ns(),
            }),
            payload: None,
        };
        request.payload = Some(command_payload(command));
        let mut rpc = Request::new(request);
        add_metadata(rpc.metadata_mut(), &self.metadata);
        let reply = self.client.command(rpc).await?.into_inner();
        Ok(CommandAck {
            command_id: reply.command_id,
            status: enum_name_ack(reply.status),
            reason: reply.reason,
        })
    }

    pub async fn enable(
        &mut self,
        enabled: bool,
        command_id: impl Into<String>,
    ) -> Result<CommandAck, Status> {
        self.command(GatewayCommand::Enable { enabled }, command_id)
            .await
    }
    pub async fn stop(
        &mut self,
        emergency: bool,
        command_id: impl Into<String>,
    ) -> Result<CommandAck, Status> {
        self.command(GatewayCommand::Stop { emergency }, command_id)
            .await
    }
    pub async fn jog(
        &mut self,
        joint_index: u32,
        step_rad: f64,
        speed_limit_rad_s: f64,
        command_id: impl Into<String>,
    ) -> Result<CommandAck, Status> {
        self.command(
            GatewayCommand::Jog {
                joint_index,
                step_rad,
                speed_limit_rad_s,
            },
            command_id,
        )
        .await
    }
    pub async fn execute_trajectory(
        &mut self,
        points: Vec<TrajectoryPoint>,
        dry_run: bool,
        command_id: impl Into<String>,
    ) -> Result<CommandAck, Status> {
        self.command(
            GatewayCommand::ExecuteTrajectory { points, dry_run },
            command_id,
        )
        .await
    }
    pub async fn reset_fault(
        &mut self,
        command_id: impl Into<String>,
    ) -> Result<CommandAck, Status> {
        self.command(GatewayCommand::ResetFault, command_id).await
    }
    pub async fn pause(&mut self, command_id: impl Into<String>) -> Result<CommandAck, Status> {
        self.command(GatewayCommand::Pause, command_id).await
    }
    pub async fn resume(&mut self, command_id: impl Into<String>) -> Result<CommandAck, Status> {
        self.command(GatewayCommand::Resume, command_id).await
    }
    pub async fn speed_scale(
        &mut self,
        scale: f64,
        command_id: impl Into<String>,
    ) -> Result<CommandAck, Status> {
        self.command(GatewayCommand::SpeedScale { scale }, command_id)
            .await
    }

    pub async fn subscribe_telemetry(
        &mut self,
        max_rate_hz: u32,
    ) -> Result<TelemetryStream, Status> {
        if max_rate_hz > 200 {
            return Err(Status::invalid_argument(
                "max_rate_hz must be between 0 and 200",
            ));
        }
        self.ensure_session().await?;
        let session_id = self
            .connection
            .as_ref()
            .expect("ensure_session establishes a connection")
            .session_id
            .clone();
        let mut request = Request::new(generated::TelemetryRequest {
            session_id,
            max_rate_hz,
        });
        add_metadata(request.metadata_mut(), &self.metadata);
        let stream = self.client.subscribe_telemetry(request).await?.into_inner();
        Ok(TelemetryStream { inner: stream })
    }
}

pub struct TelemetryStream {
    inner: tonic::codec::Streaming<generated::TelemetryFrame>,
}

impl TelemetryStream {
    pub async fn message(&mut self) -> Result<Option<TelemetryFrame>, Status> {
        Ok(self.inner.message().await?.map(telemetry_from_proto))
    }
}

pub struct ArmPlannerClient {
    client: generated::arm_planner_client::ArmPlannerClient<Channel>,
    metadata: Metadata,
}

impl ArmPlannerClient {
    pub async fn connect(endpoint: impl Into<String>) -> Result<Self, tonic::transport::Error> {
        let endpoint = Endpoint::new(endpoint.into())?;
        let channel = endpoint.connect().await?;
        Ok(Self::from_channel(channel))
    }

    pub fn from_channel(channel: Channel) -> Self {
        Self {
            client: generated::arm_planner_client::ArmPlannerClient::new(channel),
            metadata: Vec::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: impl IntoIterator<Item = (String, String)>) -> Self {
        self.metadata.extend(metadata);
        self
    }

    pub async fn solve_ik(
        &mut self,
        target: PoseTarget,
        options: IKOptions,
    ) -> Result<IKResult, Status> {
        let request = generated::IkRequest {
            request_id: if options.request_id.is_empty() {
                request_id()
            } else {
                options.request_id
            },
            target: Some(pose_to_proto(target)),
            seed_position_rad: options.seed_position_rad,
            check_collisions: options.check_collisions,
            minimum_distance_threshold_m: options.minimum_distance_threshold_m,
            assembly_phase: assembly_phase(options.assembly_phase.as_deref())? as i32,
            allowed_collision_pairs: options
                .allowed_collision_pairs
                .into_iter()
                .map(pair_to_proto)
                .collect(),
        };
        let mut rpc = Request::new(request);
        add_metadata(rpc.metadata_mut(), &self.metadata);
        Ok(ik_from_proto(self.client.solve_ik(rpc).await?.into_inner()))
    }

    pub async fn plan_trajectory(
        &mut self,
        start: PoseTarget,
        goal: PoseTarget,
        options: TrajectoryOptions,
    ) -> Result<TrajectoryPlanResult, Status> {
        let request = generated::TrajectoryPlanRequest {
            request_id: if options.request_id.is_empty() {
                request_id()
            } else {
                options.request_id
            },
            start: Some(pose_to_proto(start)),
            goal: Some(pose_to_proto(goal)),
            max_rate_hz: options.max_rate_hz,
            dry_run: options.dry_run,
            check_collisions: options.check_collisions,
            minimum_distance_threshold_m: options.minimum_distance_threshold_m,
            assembly_phase: assembly_phase(options.assembly_phase.as_deref())? as i32,
            allowed_collision_pairs: options
                .allowed_collision_pairs
                .into_iter()
                .map(pair_to_proto)
                .collect(),
        };
        let mut rpc = Request::new(request);
        add_metadata(rpc.metadata_mut(), &self.metadata);
        Ok(trajectory_from_proto(
            self.client.plan_trajectory(rpc).await?.into_inner(),
        ))
    }
}

#[derive(Clone, Debug, Default)]
pub struct IKOptions {
    pub request_id: String,
    pub seed_position_rad: Vec<f64>,
    pub check_collisions: bool,
    pub minimum_distance_threshold_m: f64,
    pub assembly_phase: Option<String>,
    pub allowed_collision_pairs: Vec<AllowedCollisionPair>,
}

#[derive(Clone, Debug)]
pub struct TrajectoryOptions {
    pub request_id: String,
    pub max_rate_hz: u32,
    pub dry_run: bool,
    pub check_collisions: bool,
    pub minimum_distance_threshold_m: f64,
    pub assembly_phase: Option<String>,
    pub allowed_collision_pairs: Vec<AllowedCollisionPair>,
}

impl Default for TrajectoryOptions {
    fn default() -> Self {
        Self {
            request_id: String::new(),
            max_rate_hz: 20,
            dry_run: false,
            check_collisions: false,
            minimum_distance_threshold_m: 0.0,
            assembly_phase: None,
            allowed_collision_pairs: Vec::new(),
        }
    }
}

fn add_metadata(metadata: &mut tonic::metadata::MetadataMap, values: &Metadata) {
    for (key, value) in values {
        if let (Ok(key), Ok(value)) = (
            tonic::metadata::MetadataKey::<tonic::metadata::Ascii>::from_bytes(key.as_bytes()),
            MetadataValue::try_from(value.as_str()),
        ) {
            metadata.insert(key, value);
        }
    }
}

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

fn request_id() -> String {
    format!("rebot-sdk-rust-{}", now_ns())
}

fn enum_name_source(value: i32) -> String {
    generated::SourceKind::try_from(value)
        .map(|v| v.as_str_name().to_owned())
        .unwrap_or_else(|_| format!("UNKNOWN_{value}"))
}

fn enum_name_quality(value: i32) -> String {
    generated::SampleQuality::try_from(value)
        .map(|v| v.as_str_name().to_owned())
        .unwrap_or_else(|_| format!("UNKNOWN_{value}"))
}

fn enum_name_ack(value: i32) -> String {
    generated::AckStatus::try_from(value)
        .map(|v| v.as_str_name().to_owned())
        .unwrap_or_else(|_| format!("UNKNOWN_{value}"))
}

fn assembly_phase(value: Option<&str>) -> Result<generated::AssemblyPhase, Status> {
    match value.unwrap_or_default().to_ascii_uppercase().as_str() {
        "" => Ok(generated::AssemblyPhase::Unspecified),
        "APPROACH" => Ok(generated::AssemblyPhase::Approach),
        "MATE" => Ok(generated::AssemblyPhase::Mate),
        "RETRACT" => Ok(generated::AssemblyPhase::Retract),
        other => Err(Status::invalid_argument(format!(
            "unknown assembly phase: {other}"
        ))),
    }
}

fn pose_to_proto(value: PoseTarget) -> generated::PoseTarget {
    generated::PoseTarget {
        position_x_m: value.position_m[0],
        position_y_m: value.position_m[1],
        position_z_m: value.position_m[2],
        rotation_x: value.rotation_xyzw[0],
        rotation_y: value.rotation_xyzw[1],
        rotation_z: value.rotation_xyzw[2],
        rotation_w: value.rotation_xyzw[3],
        frame_id: value.frame_id,
    }
}

fn pair_to_proto(value: AllowedCollisionPair) -> generated::AllowedCollisionPair {
    generated::AllowedCollisionPair {
        first: value.first,
        second: value.second,
    }
}

fn point_to_proto(value: TrajectoryPoint) -> generated::TrajectoryPoint {
    generated::TrajectoryPoint {
        time_from_start_ns: value.time_from_start_ns,
        position_rad: value.position_rad,
        velocity_rad_s: value.velocity_rad_s,
    }
}

fn point_from_proto(value: generated::TrajectoryPoint) -> TrajectoryPoint {
    TrajectoryPoint {
        time_from_start_ns: value.time_from_start_ns,
        position_rad: value.position_rad,
        velocity_rad_s: value.velocity_rad_s,
    }
}

fn command_payload(value: GatewayCommand) -> generated::control_command::Payload {
    match value {
        GatewayCommand::Enable { enabled } => {
            generated::control_command::Payload::Enable(generated::EnableCommand { enabled })
        }
        GatewayCommand::Stop { emergency } => {
            generated::control_command::Payload::Stop(generated::StopCommand { emergency })
        }
        GatewayCommand::Jog {
            joint_index,
            step_rad,
            speed_limit_rad_s,
        } => generated::control_command::Payload::Jog(generated::JogCommand {
            joint_index,
            step_rad,
            speed_limit_rad_s,
        }),
        GatewayCommand::ExecuteTrajectory { points, dry_run } => {
            generated::control_command::Payload::ExecuteTrajectory(
                generated::ExecuteTrajectoryCommand {
                    points: points.into_iter().map(point_to_proto).collect(),
                    dry_run,
                },
            )
        }
        GatewayCommand::ResetFault => {
            generated::control_command::Payload::ResetFault(generated::ResetFaultCommand {})
        }
        GatewayCommand::Pause => {
            generated::control_command::Payload::Pause(generated::PauseCommand {})
        }
        GatewayCommand::Resume => {
            generated::control_command::Payload::Resume(generated::ResumeCommand {})
        }
        GatewayCommand::SpeedScale { scale } => {
            generated::control_command::Payload::SpeedScale(generated::SpeedScaleCommand { scale })
        }
    }
}

fn telemetry_from_proto(value: generated::TelemetryFrame) -> TelemetryFrame {
    TelemetryFrame {
        sequence: value.sequence,
        timestamp_ns: value.timestamp_ns,
        source: enum_name_source(value.source),
        quality: enum_name_quality(value.quality),
        sim_time_ns: value.sim_time_ns,
        wall_time_ns: value.wall_time_ns,
        joint_position_rad: value.joint_position_rad,
        joint_velocity_rad_s: value.joint_velocity_rad_s,
        tf: value
            .tf
            .into_iter()
            .map(|item| Transform {
                parent: item.parent,
                child: item.child,
                translation_m: [
                    item.translation_x_m,
                    item.translation_y_m,
                    item.translation_z_m,
                ],
                rotation_xyzw: [
                    item.rotation_x,
                    item.rotation_y,
                    item.rotation_z,
                    item.rotation_w,
                ],
            })
            .collect(),
        planned_trajectory: value
            .planned_trajectory
            .into_iter()
            .map(point_from_proto)
            .collect(),
        actual_trajectory: value
            .actual_trajectory
            .into_iter()
            .map(point_from_proto)
            .collect(),
        images: value
            .images
            .into_iter()
            .map(|item| ImageFrame {
                sensor: item.sensor,
                width: item.width,
                height: item.height,
                encoding: item.encoding,
                data: item.data,
            })
            .collect(),
        point_clouds: value
            .point_clouds
            .into_iter()
            .map(|item| PointCloud {
                sensor: item.sensor,
                positions_xyz: item
                    .positions_xyz
                    .chunks_exact(3)
                    .map(|xyz| [xyz[0], xyz[1], xyz[2]])
                    .collect(),
                colors_rgba: item.colors_rgba,
            })
            .collect(),
        contacts: value
            .contacts
            .into_iter()
            .map(|item| Contact {
                first_geom: item.first_geom,
                second_geom: item.second_geom,
                distance_m: item.distance_m,
                normal_force_n: item.normal_force_n,
            })
            .collect(),
    }
}

fn collision_from_proto(value: generated::CollisionSummary) -> CollisionSummary {
    CollisionSummary {
        checked: value.checked,
        collision_free: value.collision_free,
        checked_pairs: value.checked_pairs,
        contacts: value.contacts,
        minimum_distance_m: value.minimum_distance_m,
    }
}

fn metadata_from_proto(value: generated::PlanningMetadata) -> PlanningMetadata {
    PlanningMetadata {
        model_version: value.model_version,
        solver: value.solver,
        random_seed: value.random_seed,
        elapsed_ns: value.elapsed_ns,
    }
}

fn ik_from_proto(value: generated::IkResponse) -> IKResult {
    IKResult {
        request_id: value.request_id,
        success: value.success,
        joint_position_rad: value.joint_position_rad,
        within_limits: value.within_limits,
        collision: collision_from_proto(value.collision.unwrap_or_default()),
        metadata: metadata_from_proto(value.metadata.unwrap_or_default()),
        reason: value.reason,
    }
}

fn trajectory_from_proto(value: generated::TrajectoryPlanResponse) -> TrajectoryPlanResult {
    TrajectoryPlanResult {
        request_id: value.request_id,
        success: value.success,
        points: value.points.into_iter().map(point_from_proto).collect(),
        collision: collision_from_proto(value.collision.unwrap_or_default()),
        metadata: metadata_from_proto(value.metadata.unwrap_or_default()),
        reason: value.reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pose_defaults_to_world_and_unspecified_orientation() {
        let pose = PoseTarget::new([1.0, 2.0, 3.0]);
        assert_eq!(pose.frame_id, "world");
        assert_eq!(pose.rotation_xyzw, [0.0; 4]);
    }

    #[test]
    fn command_ack_reports_acceptance() {
        let ack = CommandAck {
            command_id: "x".into(),
            status: "ACCEPTED".into(),
            reason: String::new(),
        };
        assert!(ack.accepted());
    }

    #[test]
    fn point_cloud_ignores_incomplete_tail() {
        let frame = telemetry_from_proto(generated::TelemetryFrame {
            point_clouds: vec![generated::PointCloudFrame {
                sensor: "depth".into(),
                positions_xyz: vec![1.0, 2.0, 3.0, 4.0],
                ..Default::default()
            }],
            ..Default::default()
        });
        assert_eq!(frame.point_clouds[0].positions_xyz, vec![[1.0, 2.0, 3.0]]);
    }

    #[test]
    fn unknown_assembly_phase_is_rejected() {
        let error = assembly_phase(Some("ASSEMBLE")).expect_err("invalid phase accepted");
        assert_eq!(error.code(), tonic::Code::InvalidArgument);
    }
}
