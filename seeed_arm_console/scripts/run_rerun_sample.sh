#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export RERUN_GRPC_URL="${RERUN_GRPC_URL:-rerun+http://127.0.0.1:9876/proxy}"
exec cargo run --features rerun-recording --bin rerun_sample
