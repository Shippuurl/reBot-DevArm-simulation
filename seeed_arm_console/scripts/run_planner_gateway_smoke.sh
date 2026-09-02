#!/usr/bin/env bash
set -euo pipefail
repo_root=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo_root"
python_bin="${REBOTS_PLANNER_PYTHON:-$repo_root/.venv-planning/bin/python}"
if [[ ! -x "$python_bin" ]]; then
  python_bin=python3
fi
exec "$python_bin" scripts/planner_gateway_smoke.py "$@"
