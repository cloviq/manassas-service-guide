#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# Honor DOCKER_CONTEXT if supplied; otherwise Docker uses its current context.
if docker compose version >/dev/null 2>&1; then
  compose=(docker compose)
elif command -v docker-compose >/dev/null 2>&1; then
  compose=(docker-compose)
else
  printf '%s\n' 'Install Docker Compose v2 (plugin or standalone); no global installs are performed.' >&2
  exit 1
fi
if [[ "${1:-}" == --check-tools ]]; then
  "${compose[@]}" version
  exit
fi
ENV_FILE="${ENV_FILE:-backend/dev.env}"
if [[ ! -f "$ENV_FILE" ]]; then
  printf '%s\n' 'Missing private environment. Run: make env' >&2
  exit 1
fi
exec "${compose[@]}" --env-file "$ENV_FILE" "$@"
