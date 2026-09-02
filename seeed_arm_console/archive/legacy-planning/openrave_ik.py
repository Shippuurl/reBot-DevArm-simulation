#!/usr/bin/env python3
"""Minimal headless OpenRAVE IK smoke test for reBot-DevArm."""
import argparse, json, os, sys, time

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--target', nargs=3, type=float, default=[0.25, 0.0, 0.30])
    ap.add_argument('--model', default='assets/robot/b601_rs/urdf/00-arm-rs_asm-v3.urdf')
    args = ap.parse_args()
    try:
        import openravepy as rave
    except ImportError as exc:
        print(json.dumps({'success': False, 'error': 'openravepy unavailable', 'detail': str(exc)})); return 2
    env = rave.Environment(); env.SetViewer('')
    robot = env.ReadRobotXMLFile(os.path.abspath(args.model))
    if robot is None: raise RuntimeError('failed to load robot model')
    env.Add(robot); manip = robot.GetActiveManipulator()
    if manip is None: raise RuntimeError('model has no active manipulator')
    robot.SetActiveDOFs(manip.GetArmIndices())
    target = rave.matrixFromPose([1, 0, 0, 0, *args.target])
    started = time.monotonic(); solution = manip.FindIKSolution(target, rave.IkFilterOptions.CheckEnvCollisions)
    elapsed = (time.monotonic() - started) * 1000
    result = {'success': solution is not None, 'elapsed_ms': elapsed, 'solver': 'openrave', 'model': args.model}
    if solution is not None: result.update(joint_position_rad=[float(x) for x in solution], collision_free=True, within_limits=True)
    print(json.dumps(result)); return 0 if solution is not None else 1

if __name__ == '__main__': sys.exit(main())
