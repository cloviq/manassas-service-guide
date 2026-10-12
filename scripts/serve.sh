#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# Parse literal assignments without sourcing executable shell or echoing secrets.
exec "${PYTHON:-python3.12}" scripts/dev.py api
