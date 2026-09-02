#!/usr/bin/env bash
set -euo pipefail

container_name=${CPP_SDK_CONTAINER:-arm-console-gateway}
build_dir=${CPP_SDK_BUILD_DIR:-/tmp/rebot-arm-sdk-build}
planner_address=${CPP_SDK_PLANNER_ADDRESS:-}

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required for the C++ SDK smoke (or build sdk/cpp with local gRPC packages)" >&2
  exit 2
fi
if [[ "$(docker inspect -f '{{.State.Running}}' "$container_name" 2>/dev/null || true)" != "true" ]]; then
  echo "running container $container_name was not found; start the MuJoCo Compose gateway first" >&2
  exit 2
fi

docker exec "$container_name" bash -lc \
  "cmake -S /work/seeed_arm_console/sdk/cpp -B '$build_dir' -DREBOT_SDK_BUILD_EXAMPLE=ON && \
   cmake --build '$build_dir' -j2 && \
   '$build_dir'/rebot_sdk_gateway_example 127.0.0.1:50051"

if [[ -n "$planner_address" ]]; then
  docker exec "$container_name" bash -lc \
    "'$build_dir'/rebot_sdk_planner_example '$planner_address'"
else
  echo "cpp_planner_sdk=SKIP (set CPP_SDK_PLANNER_ADDRESS to a planner endpoint reachable from the container)"
fi
