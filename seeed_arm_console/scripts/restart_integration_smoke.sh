#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo_root"

planner_pid=""
cleanup() {
  if [[ -n "$planner_pid" ]]; then
    kill "$planner_pid" 2>/dev/null || true
    wait "$planner_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

start_planner() {
  ./scripts/run_planner_server.sh >"/tmp/rebot-planner-restart-${1}.log" 2>&1 &
  planner_pid=$!
  # Imports and Pinocchio geometry loading are intentionally outside the
  # gateway process; allow that startup to settle before the RPC probe.
  sleep 3
  kill -0 "$planner_pid"
}

stop_planner() {
  kill "$planner_pid" 2>/dev/null || true
  wait "$planner_pid" 2>/dev/null || true
  planner_pid=""
}

./scripts/run_gateway_grpc_smoke.sh >/tmp/rebot-gateway-before-planner-restart.log
start_planner first
./scripts/run_planner_smoke.sh
stop_planner
./scripts/run_gateway_grpc_smoke.sh >/tmp/rebot-gateway-during-planner-restart.log
start_planner second
./scripts/run_planner_smoke.sh
./scripts/run_gateway_grpc_smoke.sh >/tmp/rebot-gateway-after-planner-restart.log

echo "restart_integration=OK planner_restarts=2 gateway_stable=1"
