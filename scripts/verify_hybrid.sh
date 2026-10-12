#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../backend"
# Caller may source the local env file (backend/dev.env); never print its values.
: "${DATABASE_URL:?Set DATABASE_URL}"
: "${OPENSEARCH_URL:?Set OPENSEARCH_URL}"
export OPENSEARCH_INDEX="${OPENSEARCH_INDEX:-manassas-catalog}"
export EMBEDDING_URL="${EMBEDDING_URL:-http://127.0.0.1:3002/v1/embeddings}"
export EMBEDDING_MODEL="${EMBEDDING_MODEL:-BAAI/bge-small-en-v1.5}"
export EMBEDDING_DIMENSION="${EMBEDDING_DIMENSION:-384}"
export EMBEDDING_BATCH_SIZE="${EMBEDDING_BATCH_SIZE:-16}"
cargo fmt --check
cargo test
cargo build
cargo clippy --all-targets -- -D warnings
cargo run -- migrate
cargo run -- index
cargo test --test live_hybrid -- --ignored --nocapture
RELEVANCE_REPORT="${RELEVANCE_REPORT:-relevance-results.json}" cargo test --test relevance -- --ignored --nocapture
