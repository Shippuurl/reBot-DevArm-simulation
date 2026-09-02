#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo_root"

cargo run --manifest-path sdk/rust/Cargo.toml --example gateway_client "$@"
if [[ "${RUST_SDK_RUN_PLANNER:-0}" == "1" ]]; then
  cargo run --manifest-path sdk/rust/Cargo.toml --example planner_client "$@"
fi
