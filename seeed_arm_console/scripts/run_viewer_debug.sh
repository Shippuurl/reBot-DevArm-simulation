#!/usr/bin/env bash
set -euo pipefail

# Start the embedded Viewer with a bounded, low-rate recording suitable for
# long-running debugging sessions.  Values can still be overridden by the
# environment without relying on shell-specific inline assignment syntax.
repo_root=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo_root"

export RERUN_DEBUG_MODE=1
export RERUN_HISTORY_LIMIT="${RERUN_HISTORY_LIMIT:-64MiB}"
export RERUN_TELEMETRY_RATE_HZ="${RERUN_TELEMETRY_RATE_HZ:-30}"
export RERUN_GRPC_URL="${RERUN_GRPC_URL:-rerun+http://127.0.0.1:9876/proxy}"

exec cargo run --features embedded-viewer --bin rebot_sim_viewer
