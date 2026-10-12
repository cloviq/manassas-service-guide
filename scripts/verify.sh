#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
set -a
source "${ENV_FILE:-backend/dev.env}"
set +a
export OPENSEARCH_INDEX="${CATALOG_INDEX:-manassas-catalog}"
export EMBEDDING_URL="${EMBEDDING_URL:-http://127.0.0.1:3002/v1/embeddings}"
export EMBEDDING_MODEL="${EMBEDDING_MODEL:-BAAI/bge-small-en-v1.5}"
export EMBEDDING_DIMENSION="${EMBEDDING_DIMENSION:-384}"
export EMBEDDING_BATCH_SIZE="${EMBEDDING_BATCH_SIZE:-16}"
cd backend
exec bash ../scripts/verify_hybrid.sh
