# Manassas Service Guide

A resident-facing guide to City of Manassas, Virginia services. You can search
for a service task in plain language ("my garbage cart is broken", "snow on my
street") or explore City-published GIS layers on a map: streets, snow routes,
leaf and bulk-waste collection areas, parks, trails, bike facilities,
watersheds, residential parking districts and points of interest.

This is an independent student project, **not a City of Manassas website**.
Requests entered in its demo form are stored locally and never sent to the City.
Always confirm current details at the official links shown next to each result.

Code and documentation are MIT licensed ([LICENSE](LICENSE)). The City GIS data
is downloaded from public ArcGIS services when you run the project; see
[DATA-REUSE.md](DATA-REUSE.md) and [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

## Architecture

| Component | Technology | Role |
|---|---|---|
| `backend/` | Rust, Axum, SQLx | HTTP API, accounts/sessions, catalog import, search indexing, map projection |
| PostgreSQL 17 | | Source of truth: service tasks, imported GIS records, accounts, demo tickets |
| OpenSearch 2.19 | BM25 + Lucene HNSW (cosine) | Hybrid search index, fused with reciprocal-rank fusion |
| `embeddings/` | Python, FastEmbed / ONNX Runtime | Local CPU embeddings (BAAI/bge-small-en-v1.5, 384 dimensions) |
| `frontend/` | React, TypeScript, Vite, MapLibre GL | Sign-in, service finder, map explorer, admin user management |
| `catalog/sources.json` | JSON | Source registry shared by the collectors, backend and frontend |
| `scripts/` | Python 3 (stdlib), Bash | GIS collectors, data bootstrap, developer launcher, offline tests |

Docker Compose (`compose.yaml`, project name `manassas-service-guide`) runs five
services: `postgres`, `opensearch`, `embeddings`, `api` and `web` (nginx serving
the built SPA and proxying `/api` to the API). Only the web UI (5174), PostgreSQL
(5433) and OpenSearch (9200) are published, and only on `127.0.0.1`. The API and
embedding service are reachable only inside the Compose network.

Search combines a BM25 keyword leg (with a small set of civic synonyms such as
garbage/trash/refuse) and a semantic vector leg gated by a minimum cosine
similarity, so unrelated queries return nothing instead of the whole catalog.
When the model or index is unavailable the API falls back to keyword or database
search and labels the result. See [backend/HYBRID.md](backend/HYBRID.md) and
[backend/SEARCH-RELEVANCE.md](backend/SEARCH-RELEVANCE.md).

## Prerequisites

For the container stack (`make up`):

- Docker with Compose v2 (plugin or standalone). On macOS, `make vm` starts a
  2 CPU / 4 GiB [Colima](https://github.com/abiosoft/colima) VM if you do not
  already use Docker Desktop or another engine.
- GNU Make 3.81+ (the macOS system `make` works).
- Python 3 (`python3`). It creates the env file, runs the standard-library GIS
  collectors and creates the first admin. Override with `PYTHON=...` if needed.
- Network access to container registries, Hugging Face (model download) and the
  City's public ArcGIS services.

Host development additionally needs stable Rust/Cargo with a native linker,
Node.js 22+ with npm, and Python 3.12 with `venv`/`pip` (the embedding
dependency pins require 3.12; run host setup with `make setup PYTHON=python3.12`). No Make target installs global tools.

## Quick start

```sh
git clone <this repository> && cd <checkout>
make vm      # macOS with Colima only; skip if Docker is already running
make up      # build, start, load data, create the first admin
cat secrets/admin-credentials   # username:password for the first admin
```

Then open **http://127.0.0.1:5174** and sign in.

`make up` does the following:

1. `make env` creates `backend/dev.env` with random credentials (mode 0600) if it
   does not exist. An existing file is never changed.
2. Builds the `api`, `embeddings` and `web` images one at a time to limit memory.
3. Starts all five services and waits for their health checks. The API applies
   database migrations on startup. These create the schema and 14 service tasks
   (trash and recycling, snow, power outages, water and sewer, stormwater,
   utility billing, and street, pothole and streetlight reporting) whose text
   summarizes the cited official City pages. The first embedding startup
   downloads the model into the `model_cache` volume.
4. Runs `scripts/stack-data.sh ensure`, which skips any step that is already done:
   - downloads and verifies the base GIS catalog if `catalog/records.json` is
     missing, and downloads the map-expansion layers if they are missing or the
     registry changed (`scripts/catalog_map_expand.py --check`);
   - imports the catalog if the database has fewer catalog records than the
     downloaded files or the expansion layers were just re-downloaded (an
     idempotent upsert that keeps service tasks and tickets), then restarts the
     API so the map picks up the new geometry;
   - rebuilds the search index after an import, or otherwise runs
     `index --if-stale`, which rebuilds only if the index alias is missing, was
     built with an older search schema or model, or its document count differs
     from the database;
   - creates the first admin with `scripts/create-admin.py` if there are no
     accounts. The username is `admin` and a random password is written to
     `secrets/admin-credentials` (mode 0600, git-ignored); it is never printed.

The first run takes a while: it compiles the Rust API, downloads the model and
the GIS catalog, and embeds every record. Later runs only restart containers.

Other stack commands:

```sh
make status            # container status
make logs              # recent logs
make down              # stop the stack; named volumes and the model cache are kept
SKIP_DATA=1 make up    # start services without the data/admin step
make stack-import      # force a catalog re-import (in a container, no host Cargo)
make stack-index       # force a full search reindex
make docker-build      # build images only
```

`make down` never deletes volumes. To start completely fresh, remove the Compose
volumes yourself (`bash scripts/infra.sh down -v`), which
deletes the database, the search index and all accounts. Set `DOCKER_CONTEXT=name`
to target a specific Docker context without changing your global default.

## Accounts and administration

Every `/api` route except sign-in and sign-out requires a
signed-in account; `/api/admin/*` also requires the `admin` role.

- Accounts are stored in PostgreSQL (`users`, migration 0004) with Argon2id
  password hashes. Passwords must be 12–256 characters.
- Sessions are random tokens in an `HttpOnly; Secure; SameSite=Strict` cookie,
  stored server-side only as a SHA-256 hash and valid for 7 days.
- Admins see a **Manage users** page to create accounts, change roles, reset
  passwords and delete accounts. Admins cannot demote or delete themselves, so at
  least one admin always remains. A password reset signs that user out of all
  their sessions (except your current one when you reset your own password).
- nginx rate-limits sign-in attempts (20 per minute, burst 10).

To create the first admin under a different name, run `SKIP_DATA=1 make up` and
then `make admin ADMIN_USER=name`. `scripts/create-admin.py` refuses to run if
`secrets/admin-credentials` already exists; create further accounts in the UI.

The cookie is marked `Secure`. Chrome, Firefox and current Safari treat
`http://127.0.0.1` as a secure context, so sign-in works there over plain HTTP.

## Local development (host servers)

Run the API, embeddings and frontend on the host while PostgreSQL and OpenSearch
run in Docker. Stop the full stack first (`make down`) so ports do not collide.

```sh
make check-tools       # verify installed tools; installs nothing
make setup             # npm ci, embeddings/.venv with pinned packages, backend/dev.env
make build             # Rust debug build and frontend/dist
make infra-up          # PostgreSQL and OpenSearch only
make migrate           # apply migrations
make embeddings        # shell 1: http://127.0.0.1:3002 (first run downloads the model)
make api               # shell 2: http://127.0.0.1:3001
make web               # shell 3: Vite dev server; prints its loopback URL
```

`make api`, `make migrate`, `make data-import` and `make index` read
`backend/dev.env` through `scripts/dev.py`, which parses literal `KEY=value` lines
and never executes the file as shell. Exported variables override file values.
`make infra-down` stops PostgreSQL and OpenSearch without deleting volumes.

The host stack shares the database volume with the container stack, so accounts
created by `make up` also work here. On a database without accounts, create one
with the API binary (the password is read from the first line of stdin):

```sh
cd backend && set -a && . ./dev.env && set +a
printf '%s\n' 'choose-a-long-password' | cargo run --locked -- create-user yourname admin
```

`make setup SETUP_VERIFY=1` checks installed dependencies without reinstalling.
Run `setup`, `build` and `test` as separate commands.

## Data pipeline

All data-source definitions live in [`catalog/sources.json`](catalog/sources.json);
see [catalog/README.md](catalog/README.md) for the registry format, the files
each step writes and the provenance/exclusion rules. Downloaded catalog files are
git-ignored and regenerated locally.

```sh
make data-fetch        # network: download, verify and summarize the public GIS catalog
make data-import       # host: import catalog/records.json (CATALOG_FILE=path to override)
make index             # host: full search reindex (needs infra and embeddings)
make stack-import      # container stack: import records.json and map_expansion.json
make stack-index       # container stack: full search reindex
```

`make data-fetch` runs, in order:

1. `scripts/catalog_collect.py`: the base collector (registry entries with
   `"dataset": "base"`), resumable 100-ID batches with SHA-256 hashes.
2. `scripts/catalog_verify.py`: offline reconciliation of every saved batch.
3. `scripts/catalog_map_expand.py`: the map-expansion collector (`"dataset": "expansion"`).
4. `scripts/catalog_map_test.py`: integrity checks plus a live readback of
   source-record links.
5. `scripts/catalog_expectations.py`: writes `catalog/expectations.json`, the
   per-source counts, map extent and sample street group used by the live tests.

`make data-import` covers only the base file by default; import the expansion
layers with `make data-import CATALOG_FILE=catalog/map_expansion.json`.
`stack-import` imports both. Import is an idempotent upsert that never deletes
rows, so features the City later removes stay in the database until you start
from fresh volumes; the live tests assume the database matches the downloaded
files. Indexing writes a new physical index and switches the
`manassas-catalog` alias only after verifying it, then deletes the indexes it replaced.
Restart the API after importing geometry, because the map projection is built
once per process.

For a quick importer check without network access,
[`examples/catalog.json`](examples/catalog.json) holds fictional records:

```sh
cargo run --locked --manifest-path backend/Cargo.toml -- validate-import examples/catalog.json
```

## HTTP API

- `GET /api/services?q=&category=&type=service&limit=20&offset=0` returns
  `{items,total,limit,offset,search_mode,warning?}`. `search_mode` is `hybrid`,
  `keyword` or `database`. Types: `service` (default), `place`, `route`, `outage`,
  `infrastructure`, `all`. Limit 1–100 (API default 50).
- `GET /api/services/{id}` returns one record with source-specific `details` and
  `links`. Street segments are grouped into one result per street.
- `GET /api/map/features?bbox=w,s,e,n&layers=…&limit=…` returns GeoJSON for the
  registry's map layers; `GET /api/map/features/{id}` returns one feature or a
  whole street group.
- `POST /api/tickets` stores a local demo request (`{description,service_id?,contact_email?}`).
- `POST /api/auth/login`, `POST /api/auth/logout`, `GET /api/auth/me`, and
  `/api/admin/users` (admin only).

Raw source attributes are kept in the database for audit but are never returned
by the API or embedded. Details: [backend/MAP-API.md](backend/MAP-API.md),
[backend/MAP-EXPANSION.md](backend/MAP-EXPANSION.md),
[backend/SOURCE-DETAILS.md](backend/SOURCE-DETAILS.md),
[backend/STREET-GROUPING.md](backend/STREET-GROUPING.md) and
[frontend/README.md](frontend/README.md).

## Tests

```sh
make test
```

runs the offline suites: non-ignored Rust tests (`cargo test --locked --offline`;
run `make build` first to populate the Cargo cache; `frontend/node_modules` is
installed with `npm ci` on first use), frontend Vitest, the catalog
normalizer and registry checks (`scripts/catalog_test.py`), Make workflow checks
(`scripts/workflow_test.py`) and Compose/Docker contract checks
(`scripts/container_test.py`). None of them needs Docker, network access,
downloaded data or the model.

### Live tests

The backend's ignored tests run against real services and the downloaded catalog.
They read `catalog/expectations.json` (written by `make data-fetch` and `make up`) instead of
hardcoded counts or IDs, so they follow whatever the City published when you
downloaded. Prerequisites:

1. a downloaded catalog and its expectations file (`make up` or `make data-fetch`);
2. the catalog imported and indexed (`make up` does both);
3. for the search tests, a host embedding service on port 3002 (`make setup`,
   then `make embeddings`), because the Compose embedding service is not published.

Then export the env file and run the tests from `backend/`:

```sh
cd backend && set -a && . ./dev.env && set +a
cargo test --test services_live -- --ignored        # needs DATABASE_URL
cargo test --test map_live --test map_expansion_live --test details_live -- --ignored
cargo test --test street_groups_live --test search_relevance_live --test live_hybrid -- --ignored
cargo test --test auth_http -- --ignored             # creates and deletes its own accounts
RELEVANCE_REPORT=relevance-results.json cargo test --test relevance -- --ignored --nocapture
```

`DATABASE_URL` is required by every live test; the search tests also use
`OPENSEARCH_URL`, `OPENSEARCH_INDEX` and the `EMBEDDING_*` variables, all of which
`backend/dev.env` provides. `relevance` is a labeled evaluation that reports
misses rather than failing on them. `tests/live.rs` migrates and reindexes, and
`scripts/verify.sh` migrates, reindexes and runs lint, build and live checks;
read them before running either against data you care about.

Embedding-model tests: `cd embeddings && .venv/bin/python -m pytest -q`.
Browser checks for the frontend are described in [frontend/README.md](frontend/README.md).

## Configuration overrides

`PYTHON=python3`, `ENV_FILE=path`, `CATALOG_FILE=path` (import),
`CATALOG_INDEX=alias` (backend commands), `DOCKER_CONTEXT=name`, `SETUP_VERIFY=1`,
`SKIP_DATA=1` (`make up`) and `ADMIN_USER=name` (`make admin`). Relative paths
resolve from the repository root. Do not print `docker compose config` without
`--quiet`; it expands the credentials.

## Limitations

- OpenSearch runs with its security plugin disabled and every port is bound to
  loopback. The stack is meant for local use, not public hosting.
- GIS layers are City-published snapshots. They do not establish eligibility,
  schedules, current operators or live status; historical water-main breaks are
  not outage alerts.
- The semantic similarity threshold was calibrated for this model and corpus; see
  [backend/SEARCH-RELEVANCE.md](backend/SEARCH-RELEVANCE.md) before changing either.
