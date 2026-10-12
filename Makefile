# GNU Make 3.81+ (macOS and Linux). Never include private env files here.
.DEFAULT_GOAL := help
SHELL := /bin/sh
PYTHON ?= python3
ENV_FILE ?= backend/dev.env
export PYTHON ENV_FILE

.PHONY: help env admin check-tools setup setup-env build build-api build-web test test-api test-web test-catalog test-workflow infra-up infra-down infra-status migrate api web embeddings data-fetch data-import index

check-tools:
	@"$(PYTHON)" scripts/dev.py check-tools

# An explicit chain keeps creation/installation behind checks under make -j.
setup-env: check-tools
	@"$(PYTHON)" scripts/init-env.py

setup: setup-env
	@"$(PYTHON)" scripts/dev.py setup

build: build-api build-web
build-api:
	cargo build --locked --manifest-path backend/Cargo.toml
build-web: frontend/node_modules
	cd frontend && npm run build

test: test-api test-web test-catalog test-workflow test-containers
test-api:
	cargo test --locked --offline --manifest-path backend/Cargo.toml
test-web: frontend/node_modules
	cd frontend && npm test
frontend/node_modules: frontend/package-lock.json
	cd frontend && npm ci && touch node_modules
test-catalog:
	"$(PYTHON)" scripts/catalog_test.py
test-workflow:
	"$(PYTHON)" scripts/workflow_test.py

.PHONY: vm up down status logs stack-import stack-index docker-build host-api host-web host-embeddings test-containers
# macOS: dedicated small VM + Docker context `colima-manassas` (2 CPU / 4 GiB).
vm:
	colima start --profile manassas --cpu 2 --memory 4 --disk 40 --vm-type vz --mount-type virtiofs
# Build services sequentially to bound peak memory.
docker-build: env
	bash scripts/infra.sh build api
	bash scripts/infra.sh build embeddings
	bash scripts/infra.sh build web
# Fresh checkout -> working app: after health checks, `ensure` downloads/imports
# the public catalog, (re)indexes only if stale and creates the first admin.
# Each step is skipped when already done. SKIP_DATA=1 starts services only.
up: docker-build
	bash scripts/infra.sh up -d --no-build --wait --wait-timeout 900
	@if [ -z "$(SKIP_DATA)" ]; then bash scripts/stack-data.sh ensure; fi
down:
	bash scripts/infra.sh down
status:
	bash scripts/infra.sh ps
logs:
	bash scripts/infra.sh logs --tail=100
# Container-only equivalents of data-import/index (no host Cargo needed).
stack-import:
	bash scripts/stack-data.sh import
stack-index:
	bash scripts/stack-data.sh index
host-api: api
host-web: web
host-embeddings: embeddings
test-containers:
	"$(PYTHON)" scripts/container_test.py

infra-up:
	bash scripts/infra.sh up -d --wait --wait-timeout 240 postgres opensearch
infra-down:
	bash scripts/infra.sh stop postgres opensearch
infra-status:
	bash scripts/infra.sh ps

migrate data-import index:
	@"$(PYTHON)" scripts/dev.py $@
api:
	@bash scripts/serve.sh
web:
	cd frontend && npm run dev
embeddings:
	sh embeddings/run.sh

data-fetch:
	"$(PYTHON)" scripts/catalog_collect.py
	"$(PYTHON)" scripts/catalog_verify.py
	"$(PYTHON)" scripts/catalog_map_expand.py
	"$(PYTHON)" scripts/catalog_map_test.py
	"$(PYTHON)" scripts/catalog_expectations.py

env:
	@"$(PYTHON)" scripts/init-env.py
# First admin account in the running stack (later accounts: Manage users page).
admin:
	@"$(PYTHON)" scripts/create-admin.py

help:
	@printf '%s\n' \
	  'Full stack: up / down / status / logs (Docker; named volumes preserved)' \
	  '  vm                macOS: create/start the 2 CPU / 4 GiB Colima VM' \
	  '  up                Build, start, then ensure catalog/index/first admin (SKIP_DATA=1 to skip)' \
	  '  docker-build      Sequential image builds only; no startup or data fetch' \
	  '  stack-import / stack-index  Force catalog import / EXPENSIVE full reindex' \
	  '  host-api / host-web / host-embeddings  Explicit host dev aliases' \
	  'Local development (see README.md for prerequisites and separate-shell startup):' \
	  '  check-tools       Check installed tools; install nothing' \
	  '  setup             Local npm/embedding dependencies + private env (network)' \
	  '  env               Create private env only if absent' \
	  '  admin             Create first admin in running stack (ADMIN_USER=name)' \
	  '  build             Build Rust and frontend; no services or indexing' \
	  '  test              Offline Rust/frontend/catalog/workflow/container suites' \
	  '  infra-up          Start PostgreSQL/OpenSearch (may pull images)' \
	  '  infra-down        Stop containers; KEEP named volumes' \
	  '  infra-status      Show containers in current/selected Docker context' \
	  '  migrate           Apply database migrations (requires infra)' \
	  '  api / web / embeddings  Foreground servers; one shell each' \
	  '  data-fetch        Download, verify and summarize the public GIS catalog (network)' \
	  '  data-import       Import local catalog into DB (requires migrate)' \
	  '  index             EXPENSIVE explicit rebuild (infra + embeddings required)' \
	  'Overrides: PYTHON=python3.12 ENV_FILE=path DOCKER_CONTEXT=name' \
	  '           CATALOG_FILE=path CATALOG_INDEX=alias SETUP_VERIFY=1'
