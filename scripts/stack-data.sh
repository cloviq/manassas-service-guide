#!/usr/bin/env bash
# Catalog import/index against the running Compose stack with the API image; no
# host Cargo. `ensure` (run by `make up`) makes a fresh checkout usable and is a
# no-op when everything is already in place.
# Credentials pass via a private temp env file and are never printed.
set -euo pipefail
cd "$(dirname "$0")/.."
PYTHON="${PYTHON:-python3.12}"
api=$(bash scripts/infra.sh ps -q api)
if [[ -z "$api" ]]; then
  printf '%s\n' 'Stack is not running; run make up first.' >&2
  exit 1
fi
image=$(docker inspect -f '{{.Config.Image}}' "$api")
network=$(docker inspect -f '{{range $name, $_ := .NetworkSettings.Networks}}{{$name}}{{end}}' "$api")
envfile=$(mktemp)
trap 'rm -f "$envfile"' EXIT
docker inspect -f '{{range .Config.Env}}{{println .}}{{end}}' "$api" \
  | grep -E '^(DATABASE_URL|OPENSEARCH_|EMBEDDING_)' > "$envfile"
# Indexing holds the whole catalog snapshot; the API's 256m limit is too small.
run=(docker run --rm --network "$network" --env-file "$envfile" -m 1g -v "$PWD/catalog:/catalog:ro" "$image")

sql() {
  bash scripts/infra.sh exec -T postgres sh -c 'psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -tAc "$0"' "$1"
}
import_catalog() {
  "${run[@]}" import /catalog/records.json
  if [[ -f catalog/map_expansion.json ]]; then
    "${run[@]}" import /catalog/map_expansion.json
  fi
}
ensure() {
  # 1. Public GIS snapshot (read-only, verified). Each part is rebuilt only when
  # missing or out of date with the registry; saved batches make re-runs cheap.
  local refreshed=0
  if ! "$PYTHON" scripts/catalog_collect.py --check; then
    echo '==> Downloading public GIS catalog (missing or registry changed)'
    "$PYTHON" scripts/catalog_collect.py >/dev/null
    "$PYTHON" scripts/catalog_verify.py >/dev/null
    refreshed=1
  fi
  # Expansion layers depend on the registry and on records.json (duplicate exclusion).
  if ! "$PYTHON" scripts/catalog_map_expand.py --check; then
    echo '==> Downloading map-expansion layers (missing or out of date)'
    "$PYTHON" scripts/catalog_map_expand.py >/dev/null
    "$PYTHON" scripts/catalog_map_test.py >/dev/null
    refreshed=1
  fi
  # Summary of the downloaded catalog that the live tests compare against.
  if ! "$PYTHON" scripts/catalog_expectations.py --check; then
    "$PYTHON" scripts/catalog_expectations.py >/dev/null
  fi
  # 2. Import when the database is missing catalog records (import is idempotent).
  local expected imported
  expected=$("$PYTHON" -c 'import json,sys; print(sum(len(json.load(open(f))) for f in sys.argv[1:]))' \
    catalog/records.json catalog/map_expansion.json)
  imported=$(sql "SELECT count(*) FROM services WHERE source_key <> 'seed'")
  local reindex=0
  if [[ "$imported" -lt "$expected" || "$refreshed" -eq 1 ]]; then
    echo "==> Importing catalog ($imported of $expected records present)"
    import_catalog
    reindex=1
    # The API caches the map projection on first use; reload it with the new records.
    bash scripts/infra.sh restart api >/dev/null
    bash scripts/infra.sh up -d --no-build --wait --wait-timeout 300 api >/dev/null
  else
    echo "==> Catalog already imported ($imported records)"
  fi
  # 3. (Re)build the search index only if missing, outdated or out of sync.
  # Imported records may have changed text without changing the count, so any
  # import forces a full rebuild; otherwise only a stale index is rebuilt.
  if [[ "$reindex" -eq 1 ]]; then
    echo '==> Rebuilding search index after import'
    "${run[@]}" index
  else
    echo '==> Checking search index'
    "${run[@]}" index --if-stale
  fi
  # 4. First admin account so someone can sign in.
  if [[ "$(sql 'SELECT count(*) FROM users')" -eq 0 ]]; then
    if [[ -f secrets/admin-credentials ]]; then
      mv secrets/admin-credentials "secrets/admin-credentials.stale-$(date +%Y%m%d%H%M%S)"
    fi
    echo '==> No accounts yet; creating the first admin'
    "$PYTHON" scripts/create-admin.py
  fi
}

case "${1:-}" in
  import)
    if [[ ! -f catalog/records.json ]]; then
      printf '%s\n' 'Missing catalog/records.json; run make data-fetch.' >&2
      exit 1
    fi
    import_catalog
    ;;
  index) "${run[@]}" index ;;
  ensure) ensure ;;
  *) printf '%s\n' 'Usage: stack-data.sh import|index|ensure' >&2; exit 2 ;;
esac
