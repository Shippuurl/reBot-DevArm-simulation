"""Launch the repository's ROS-agnostic ArmPlanner under ROS 2 Jazzy."""

import os
from pathlib import Path

from launch import LaunchDescription
from launch.actions import DeclareLaunchArgument, ExecuteProcess
from launch.substitutions import LaunchConfiguration


def generate_launch_description():
    package_root = Path(__file__).resolve().parents[4]
    script_default = os.environ.get(
        "REBOTS_PLANNER_SCRIPT", str(package_root / "scripts" / "planner_grpc_server.py")
    )
    model_default = os.environ.get(
        "REBOTS_ARM_URDF", str(package_root / "assets/robot/b601_rs/urdf/00-arm-rs_asm-v3.urdf")
    )
    venv_python = package_root / ".venv-planning" / "bin" / "python"
    python_default = os.environ.get(
        "REBOTS_PLANNER_PYTHON", str(venv_python if venv_python.exists() else "python3")
    )
    return LaunchDescription([
        DeclareLaunchArgument("listen", default_value="127.0.0.1:50053"),
        DeclareLaunchArgument("model", default_value=model_default),
        DeclareLaunchArgument("default_minimum_distance", default_value="0.02"),
        DeclareLaunchArgument("planner_script", default_value=script_default),
        DeclareLaunchArgument("python_executable", default_value=python_default),
        ExecuteProcess(
            cmd=[
                LaunchConfiguration("python_executable"),
                LaunchConfiguration("planner_script"),
                "--listen",
                LaunchConfiguration("listen"),
                "--model",
                LaunchConfiguration("model"),
                "--default-minimum-distance",
                LaunchConfiguration("default_minimum_distance"),
            ],
            output="screen",
        ),
    ])
