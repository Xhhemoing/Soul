#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
python3 "$SCRIPT_DIR/probe.py" /workspace 2>&1 | tee "$SCRIPT_DIR/TEST_LOG.txt"
