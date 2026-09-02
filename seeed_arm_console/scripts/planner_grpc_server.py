#!/usr/bin/env python3
"""ROS-agnostic ArmPlanner gRPC service backed by Pinocchio + ProxSuite."""
import argparse, hashlib, importlib, json, math, os, sys, tempfile, time
from pathlib import Path
import threading
from concurrent import futures

try:
    import grpc
    from grpc_tools import protoc
except ModuleNotFoundError as exc:
    raise SystemExit(
        f"缺少 {exc.name}，请先执行: python3 -m pip install -r requirements-planning.txt"
    ) from exc


def load_proto(proto_path):
    out = tempfile.mkdtemp(prefix="arm-proto-")
    if protoc.main(["protoc", f"-I{os.path.dirname(proto_path)}", f"--python_out={out}", f"--grpc_python_out={out}", proto_path]) != 0:
        raise RuntimeError("failed to generate Python protobuf bindings")
    sys.path.insert(0, out)
    return importlib.import_module("arm_console_pb2"), importlib.import_module("arm_console_pb2_grpc")


def main():
    repo_root = Path(__file__).resolve().parents[1]
    ap = argparse.ArgumentParser()
    ap.add_argument("--listen", default="127.0.0.1:50053")
    ap.add_argument("--model", default=str(repo_root / "assets/robot/b601_rs/urdf/00-arm-rs_asm-v3.urdf"))
    ap.add_argument(
        "--default-minimum-distance",
        type=float,
        default=0.02,
        help="planning clearance in meters (default: 0.02; MATE defaults to 0.001)",
    )
    ap.add_argument(
        "--max-joint-speed",
        type=float,
        default=1.0,
        help="maximum interpolated joint speed in rad/s (default: 1.0)",
    )
    ap.add_argument(
        "--max-joint-acceleration",
        type=float,
        default=2.0,
        help="maximum interpolated joint acceleration in rad/s^2 (default: 2.0)",
    )
    args = ap.parse_args()
    if not math.isfinite(args.max_joint_speed) or args.max_joint_speed <= 0.0:
        raise SystemExit("--max-joint-speed must be a finite positive value")
    if not math.isfinite(args.max_joint_acceleration) or args.max_joint_acceleration <= 0.0:
        raise SystemExit("--max-joint-acceleration must be a finite positive value")
    try:
        import numpy as np
        import pinocchio as pin
        import proxsuite
        import rerun as rr
    except ImportError as exc:
        raise SystemExit(f"missing planning dependency: {exc}")
    proto, grpc_proto = load_proto(str(repo_root / "protocol/arm_console.proto"))
    model_path = os.path.abspath(args.model)
    model = pin.buildModelFromUrdf(model_path); data = model.createData()
    arm_nv = min(6, model.nv)
    collision_model = collision_data = None
    try:
        # The URDF uses a ROS package:// URI; rewrite it to this checkout's
        # bundled mesh directory for the standalone headless service.
        urdf_text = open(model_path, encoding="utf-8").read().replace(
            "package://rebotarm_bringup/description/meshes_rs/",
            os.path.join(os.path.dirname(os.path.dirname(model_path)), "meshes") + os.sep,
        )
        urdf_file = tempfile.NamedTemporaryFile(mode="w", suffix=".urdf", delete=False)
        urdf_file.write(urdf_text); urdf_file.close()
        collision_model = pin.buildGeomFromUrdf(model, urdf_file.name, pin.GeometryType.COLLISION)
        collision_model.addAllCollisionPairs()
        # Adjacent links share geometry by design; exclude those pairs from
        # self-collision checks to avoid rejecting every valid arm pose.
        excluded_pairs = []
        for pair in list(collision_model.collisionPairs):
            a = collision_model.geometryObjects[pair.first].parentJoint
            b = collision_model.geometryObjects[pair.second].parentJoint
            name_a = collision_model.geometryObjects[pair.first].name
            name_b = collision_model.geometryObjects[pair.second].name
            if abs(a - b) <= 1 or ("gripper" in name_a and "gripper" in name_b):
                excluded_pairs.append(pair)
        # removeCollisionPair mutates the vector, so remove from the end to
        # keep pair indices stable (forward removal can leave stale pairs).
        for pair in reversed(excluded_pairs):
            collision_model.removeCollisionPair(pair)
        collision_data = pin.GeometryData(collision_model)
    except Exception as exc:
        print(json.dumps({"collision_geometry": "unavailable", "reason": str(exc)}), file=sys.stderr)

    def effective_threshold(request):
        """Resolve a safe clearance without ever silently falling back to zero."""
        requested = float(request.minimum_distance_threshold_m)
        if requested > 0.0:
            return requested
        # MATE is the only phase that permits controlled geometric contact;
        # the force/torque watchdog remains a separate execution concern.
        if request.assembly_phase == proto.AssemblyPhase.MATE:
            return 0.001
        return max(0.0, args.default_minimum_distance)

    def collision_report(request, q):
        if collision_model is None or not request.check_collisions:
            return False, [], 0.0, 0
        pin.computeCollisions(model, data, collision_model, collision_data, q, False)
        allowed = {
            frozenset((pair.first.strip(), pair.second.strip()))
            for pair in request.allowed_collision_pairs
            if pair.first.strip() and pair.second.strip()
        }

        def collision_names(name):
            short = name.rsplit("/", 1)[-1]
            variants = {name, short}
            for value in (name, short):
                if value.endswith("_0"):
                    variants.add(value[:-2])
                if value.endswith("_collision"):
                    variants.add(value[:-10])
            return variants

        def is_allowed(first, second):
            return any(
                frozenset((left, right)) in allowed
                for left in collision_names(first)
                for right in collision_names(second)
            )

        contacts = []
        distances = []
        checked_pairs = 0
        for pair, result in zip(collision_model.collisionPairs, collision_data.collisionResults):
            first = collision_model.geometryObjects[pair.first].name
            second = collision_model.geometryObjects[pair.second].name
            if is_allowed(first, second):
                continue
            checked_pairs += 1
            if result.isCollision():
                contacts.append(f"{first}:{second}")
            distance = result.distance_lower_bound
            if np.isfinite(distance):
                distances.append(float(distance))
        return bool(contacts), contacts, (min(distances) if distances else 0.0), checked_pairs
    frame_id = model.getFrameId("gripper_end")
    if frame_id >= model.nframes: frame_id = model.nframes - 1
    with open(model_path, "rb") as stream: model_version = hashlib.sha256(stream.read()).hexdigest()[:16]
    rerun = None
    if os.environ.get("RERUN_GRPC_URL"):
        try:
            rr.init("pinocchio_proxsuite_planner", spawn=False)
            rr.connect_grpc(os.environ["RERUN_GRPC_URL"])
            rerun = rr
        except Exception:
            print(json.dumps({"rerun": "unavailable"}), file=sys.stderr)

    planner_lock = threading.RLock()

    class Planner(grpc_proto.ArmPlannerServicer):
        def SolveIK(self, request, context):
            # Pinocchio Data and Coal GeometryData are mutable workspaces;
            # serialize requests until per-call workspaces are introduced.
            with planner_lock:
                return self._solve_ik(request, context)

        def _solve_ik(self, request, context):
            started = time.monotonic()
            if request.target.frame_id not in ("", "world"):
                return proto.IKResponse(
                    request_id=request.request_id,
                    success=False,
                    within_limits=False,
                    collision=proto.CollisionSummary(checked=False, collision_free=False),
                    metadata=proto.PlanningMetadata(
                        model_version=model_version,
                        solver="pinocchio+proxsuite",
                        elapsed_ns=int((time.monotonic() - started) * 1e9),
                    ),
                    reason="only the world target frame is supported",
                )
            target = np.array([request.target.position_x_m, request.target.position_y_m, request.target.position_z_m])
            target_quaternion = np.array([
                request.target.rotation_x,
                request.target.rotation_y,
                request.target.rotation_z,
                request.target.rotation_w,
            ], dtype=float)
            quaternion_norm = np.linalg.norm(target_quaternion)
            orientation_requested = quaternion_norm >= 1e-9
            if orientation_requested:
                target_quaternion /= quaternion_norm
                target_rotation = pin.Quaternion(
                    target_quaternion[3], target_quaternion[0], target_quaternion[1], target_quaternion[2]
                ).toRotationMatrix()
            else:
                target_rotation = np.eye(3)
            collision_checked = collision_model is not None and request.check_collisions
            threshold = effective_threshold(request)

            # A seed supplied by the caller is tried first, followed by three
            # deterministic alternatives. This keeps the service reproducible
            # while recovering from local minima and self-collision branches.
            seed_candidates = []
            if len(request.seed_position_rad) == arm_nv:
                seed = pin.neutral(model)
                seed[:arm_nv] = np.asarray(request.seed_position_rad, dtype=float)
                seed_candidates.append(seed)
            neutral = pin.neutral(model)
            seed_candidates.append(neutral)
            midpoint = neutral.copy()
            midpoint[:arm_nv] = 0.5 * (
                model.lowerPositionLimit[:arm_nv] + model.upperPositionLimit[:arm_nv]
            )
            seed_candidates.append(midpoint)
            alternate = midpoint.copy()
            span = model.upperPositionLimit[:arm_nv] - model.lowerPositionLimit[:arm_nv]
            alternate[:arm_nv] = np.clip(
                midpoint[:arm_nv] + np.array([0.35, -0.25, 0.25, -0.2, 0.2, -0.15])[:arm_nv] * span,
                model.lowerPositionLimit[:arm_nv], model.upperPositionLimit[:arm_nv],
            )
            seed_candidates.append(alternate)
            unique_seeds = []
            for seed in seed_candidates:
                if not any(np.allclose(seed[:arm_nv], previous[:arm_nv]) for previous in unique_seeds):
                    unique_seeds.append(seed)

            def solve_candidate(seed, candidate_index):
                q = seed.copy()
                qp = proxsuite.proxqp.dense.QP(arm_nv, 0, 0)
                error = np.ones(3) * np.inf
                orientation_error = np.ones(3) * np.inf
                for iteration in range(80):
                    pin.forwardKinematics(model, data, q); pin.updateFramePlacements(model, data)
                    frame = data.oMf[frame_id]
                    current = frame.translation.copy()
                    error = target - current
                    orientation_error = (
                        pin.log3(target_rotation @ frame.rotation.T)
                        if orientation_requested else np.zeros(3)
                    )
                    if rerun:
                        rerun.set_time("ik_candidate", sequence=candidate_index)
                        rerun.set_time("iteration", sequence=iteration)
                        rerun.log("planning/ee_position", rerun.Points3D([current]))
                        rerun.log("planning/error_m", rerun.Scalars(float(np.linalg.norm(error))))
                        rerun.log("planning/orientation_error_rad", rerun.Scalars(float(np.linalg.norm(orientation_error))))
                    if np.linalg.norm(error) < 1e-4 and np.linalg.norm(orientation_error) < 1e-4:
                        break
                    full_jacobian = pin.computeFrameJacobian(
                        model, data, q, frame_id, pin.ReferenceFrame.LOCAL_WORLD_ALIGNED
                    )[:6, :arm_nv]
                    task_error = np.concatenate((error, orientation_error))
                    h = full_jacobian.T @ full_jacobian + 1e-6 * np.eye(arm_nv)
                    g = -full_jacobian.T @ task_error
                    qp.init(h, g, None, None, None, None); qp.solve()
                    dq_arm = np.clip(
                        np.asarray(qp.results.x) * 0.25,
                        model.lowerPositionLimit[:arm_nv] - q[:arm_nv],
                        model.upperPositionLimit[:arm_nv] - q[:arm_nv],
                    )
                    dq = np.zeros(model.nv); dq[:arm_nv] = dq_arm
                    q = pin.integrate(model, q, dq)
                position_error = float(np.linalg.norm(error))
                orientation_error_norm = float(np.linalg.norm(orientation_error))
                pose_success = position_error < 1e-3 and (
                    not orientation_requested or orientation_error_norm < 1e-3
                )
                in_collision, contacts, minimum_distance, checked_pairs = collision_report(request, q)
                margin_ok = not collision_checked or minimum_distance >= threshold
                valid = pose_success and not in_collision and margin_ok
                score = position_error + orientation_error_norm
                if in_collision:
                    score += 100.0
                elif not margin_ok:
                    score += max(0.0, threshold - minimum_distance) * 100.0
                return {
                    "q": q,
                    "pose_success": pose_success,
                    "valid": valid,
                    "score": score,
                    "in_collision": in_collision,
                    "contacts": contacts,
                    "minimum_distance": minimum_distance,
                    "checked_pairs": checked_pairs,
                    "candidate_index": candidate_index,
                }

            candidates = [solve_candidate(seed, index) for index, seed in enumerate(unique_seeds)]
            selected = min(candidates, key=lambda candidate: (not candidate["valid"], candidate["score"]))
            q = selected["q"]
            success = selected["pose_success"]
            in_collision = selected["in_collision"]
            contacts = selected["contacts"]
            minimum_distance = selected["minimum_distance"]
            checked_pairs = selected["checked_pairs"]
            elapsed = int((time.monotonic() - started) * 1e9)
            if rerun:
                rerun.log("planning/ik_selected_candidate", rerun.Scalars(float(selected["candidate_index"])))
                rerun.log("planning/joint_position_rad", rerun.Scalars(q[:arm_nv].tolist()))
                rerun.log("planning/collision_checked", rerun.Scalars(float(collision_checked)))
                rerun.log("planning/collision_free", rerun.Scalars(float(collision_checked and not in_collision)))
                rerun.log("planning/collision_pairs", rerun.Scalars(float(checked_pairs)))
                rerun.log("planning/collision_minimum_distance_m", rerun.Scalars(minimum_distance))
                rerun.log("planning/collision_distance_threshold_m", rerun.Scalars(threshold))
            margin_ok = not collision_checked or minimum_distance >= threshold
            collision = proto.CollisionSummary(checked=collision_checked, collision_free=not in_collision and margin_ok, checked_pairs=checked_pairs, contacts=contacts, minimum_distance_m=minimum_distance)
            reason = "collision detected" if in_collision else ("minimum collision distance not met" if collision_checked and not margin_ok else ("" if success else "target not reached"))
            return proto.IKResponse(request_id=request.request_id, success=success and not in_collision and margin_ok, joint_position_rad=q[:arm_nv], within_limits=success, collision=collision, metadata=proto.PlanningMetadata(model_version=model_version, solver="pinocchio+proxsuite", random_seed=selected["candidate_index"], elapsed_ns=elapsed), reason=reason)

        def PlanTrajectory(self, request, context):
            with planner_lock:
                return self._plan_trajectory(request, context)

        def _plan_trajectory(self, request, context):
            started = time.monotonic()
            start_req = proto.IKRequest(request_id=request.request_id + ":start", target=request.start, check_collisions=request.check_collisions, minimum_distance_threshold_m=request.minimum_distance_threshold_m, assembly_phase=request.assembly_phase, allowed_collision_pairs=request.allowed_collision_pairs)
            goal_req = proto.IKRequest(request_id=request.request_id + ":goal", target=request.goal, check_collisions=request.check_collisions, minimum_distance_threshold_m=request.minimum_distance_threshold_m, assembly_phase=request.assembly_phase, allowed_collision_pairs=request.allowed_collision_pairs)
            start = self.SolveIK(start_req, context)
            goal = self.SolveIK(goal_req, context)
            if not start.success or not goal.success:
                endpoint_collision = proto.CollisionSummary(
                    checked=start.collision.checked or goal.collision.checked,
                    collision_free=False,
                    checked_pairs=max(start.collision.checked_pairs, goal.collision.checked_pairs),
                    contacts=list(dict.fromkeys(list(start.collision.contacts) + list(goal.collision.contacts))),
                    minimum_distance_m=min(
                        value for value in (start.collision.minimum_distance_m, goal.collision.minimum_distance_m)
                        if value > 0.0
                    ) if any(value > 0.0 for value in (start.collision.minimum_distance_m, goal.collision.minimum_distance_m)) else 0.0,
                )
                return proto.TrajectoryPlanResponse(request_id=request.request_id, success=False, collision=endpoint_collision, metadata=proto.PlanningMetadata(model_version=model_version, solver="pinocchio+proxsuite", elapsed_ns=int((time.monotonic() - started) * 1e9)), reason="start or goal IK failed")
            delta = np.asarray(goal.joint_position_rad) - np.asarray(start.joint_position_rad)
            max_speed = float(args.max_joint_speed)
            max_acceleration = float(args.max_joint_acceleration)
            max_delta = float(np.max(np.abs(delta)))
            # A cubic smoothstep has zero velocity at both endpoints while
            # retaining closed-form speed/acceleration bounds:
            #   max(|v|) = 1.5 * |delta| / T
            #   max(|a|) = 6.0 * |delta| / T^2
            # Choose T from both limits, then keep the existing two-second
            # minimum so short moves remain observable in the Viewer.
            duration_s = max(
                2.0,
                1.5 * max_delta / max_speed,
                math.sqrt(6.0 * max_delta / max_acceleration),
            )
            rate_hz = max(1, min(200, int(request.max_rate_hz or 20)))
            count = max(2, min(2000, int(math.ceil(duration_s * rate_hz)) + 1))
            points = []
            trajectory_collision_free = True
            trajectory_contacts = []
            trajectory_min_distance = float("inf")
            trajectory_checked_pairs = 0
            threshold = effective_threshold(request)
            # Use ProxSuite for the trajectory projection as well as IK.  Each
            # sample minimizes the distance to the smoothstep reference while
            # enforcing joint boxes and a per-segment velocity box around the
            # previously accepted sample.  The closed-form duration makes the
            # reference feasible, so this remains deterministic but still
            # protects against future changes to the interpolator.
            trajectory_h = np.eye(arm_nv)
            trajectory_c = np.eye(arm_nv)
            previous_position = np.asarray(start.joint_position_rad, dtype=float)
            segment_dt = duration_s / max(count - 1, 1)
            for index in range(count):
                alpha = index / (count - 1)
                blend = alpha * alpha * (3.0 - 2.0 * alpha)
                blend_velocity = 6.0 * alpha * (1.0 - alpha) / duration_s
                reference = np.asarray(
                    [a + blend * (b - a) for a, b in zip(start.joint_position_rad, goal.joint_position_rad)],
                    dtype=float,
                )
                lower = np.asarray(model.lowerPositionLimit[:arm_nv], dtype=float).copy()
                upper = np.asarray(model.upperPositionLimit[:arm_nv], dtype=float).copy()
                if index > 0:
                    max_step = max_speed * segment_dt
                    lower = np.maximum(lower, previous_position - max_step)
                    upper = np.minimum(upper, previous_position + max_step)
                if np.any(lower > upper):
                    return proto.TrajectoryPlanResponse(
                        request_id=request.request_id,
                        success=False,
                        collision=proto.CollisionSummary(checked=False, collision_free=False),
                        metadata=proto.PlanningMetadata(
                            model_version=model_version,
                            solver="pinocchio+proxsuite",
                            elapsed_ns=int((time.monotonic() - started) * 1e9),
                        ),
                        reason="trajectory velocity/joint bounds are infeasible",
                    )
                trajectory_qp = proxsuite.proxqp.dense.QP(arm_nv, 0, arm_nv)
                trajectory_qp.init(
                    trajectory_h,
                    -reference,
                    None,
                    None,
                    trajectory_c,
                    lower,
                    upper,
                )
                trajectory_qp.solve()
                optimized = np.asarray(trajectory_qp.results.x, dtype=float)
                if optimized.shape != (arm_nv,) or not np.all(np.isfinite(optimized)):
                    return proto.TrajectoryPlanResponse(
                        request_id=request.request_id,
                        success=False,
                        collision=proto.CollisionSummary(checked=False, collision_free=False),
                        metadata=proto.PlanningMetadata(
                            model_version=model_version,
                            solver="pinocchio+proxsuite",
                            elapsed_ns=int((time.monotonic() - started) * 1e9),
                        ),
                        reason="ProxSuite trajectory projection failed",
                    )
                previous_position = optimized
                positions = optimized.tolist()
                velocities = [blend_velocity * (b - a) for a, b in zip(start.joint_position_rad, goal.joint_position_rad)]
                points.append(proto.TrajectoryPoint(time_from_start_ns=int(alpha * duration_s * 1e9), position_rad=positions, velocity_rad_s=velocities))
                if request.check_collisions and collision_model is not None:
                    candidate = pin.neutral(model)
                    candidate[:arm_nv] = np.asarray(positions)
                    in_collision, contacts, minimum_distance, checked_pairs = collision_report(request, candidate)
                    trajectory_collision_free &= not in_collision and minimum_distance >= threshold
                    trajectory_contacts.extend(contacts)
                    trajectory_min_distance = min(trajectory_min_distance, minimum_distance)
                    trajectory_checked_pairs = max(trajectory_checked_pairs, checked_pairs)
                if rerun:
                    rerun.set_time("trajectory_point", sequence=index)
                    rerun.log("planning/planned_trajectory/joint_position_rad", rerun.Scalars(positions))
            endpoint_collision = proto.CollisionSummary(
                checked=start.collision.checked or goal.collision.checked,
                collision_free=(trajectory_collision_free and start.collision.collision_free and goal.collision.collision_free),
                checked_pairs=max(trajectory_checked_pairs, start.collision.checked_pairs, goal.collision.checked_pairs),
                contacts=list(dict.fromkeys(trajectory_contacts + list(start.collision.contacts) + list(goal.collision.contacts))),
                minimum_distance_m=(trajectory_min_distance if np.isfinite(trajectory_min_distance) else min(
                    (value for value in (start.collision.minimum_distance_m, goal.collision.minimum_distance_m) if value > 0.0),
                    default=0.0,
                )),
            )
            return proto.TrajectoryPlanResponse(request_id=request.request_id, success=endpoint_collision.collision_free, points=points, collision=endpoint_collision, metadata=proto.PlanningMetadata(model_version=model_version, solver="pinocchio+proxsuite", elapsed_ns=int((time.monotonic() - started) * 1e9)), reason="" if endpoint_collision.collision_free else "trajectory collision or clearance violation")

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=4))
    grpc_proto.add_ArmPlannerServicer_to_server(Planner(), server); server.add_insecure_port(args.listen); server.start()
    print(json.dumps({"service": "ArmPlanner", "listen": args.listen, "model_version": model_version}), flush=True)
    server.wait_for_termination()


if __name__ == "__main__": main()
