# Catalog: source registry and GIS acquisition

The City of Manassas publishes its GIS layers through public, unauthenticated
ArcGIS services. The scripts in `../scripts/` download those layers, normalize
them into importer records and keep provenance for local review. Everything in
this directory except `sources.json` and this README is generated and ignored by
Git; `make up` or `make data-fetch` recreates it.

## Source registry

[`sources.json`](sources.json) is the single list of data sources. The
collectors (`scripts/catalog_sources.py`), the backend (`backend/src/registry.rs`,
compiled in) and the frontend (`frontend/src/sources.ts`, bundled) all read it,
so adding or changing a layer is a registry edit rather than a code change. It
defines:

- `publisher`: ArcGIS services URL, organization ID, GeoHub/FAQ links, the
  provider/jurisdiction wording stamped on normalized records, the collector
  user agent and reuse terms.
- `links`: official pages shown to residents (trash, yard waste, leaf lookup).
- `sources`: one entry per ArcGIS layer (`Family/layer`). `dataset` selects the
  collector (`base` → `catalog_collect.py`, `expansion` → `catalog_map_expand.py`).
  Each entry sets the category, record type, resident detail allowlist
  (`details`: `strings`, `numbers`, `nonnegative`), caveats, notices and an
  optional `map` placement (layer, classification, variant and color). Further
  keys shape how records are built:
  - `title_field`, `name_template`, `description_template`, `instructions` /
    `instructions_field` and `official_url_field` build the resident name, text
    and links from allowlisted attributes.
  - `name_fallback`: name used when the title field is blank (for example
    `Parking district {_id}`).
  - `type_label`: `{field, fallback}`; maps a coded field to its published
    domain name (`{_type_label}` in templates), falling back when the code has
    no readable name.
  - `disambiguate`: `{field, strip_from}`; when several records share a generic
    name (for example "Public Parking"), appends a published field such as the
    street address.
  - `extra_attributes`: attributes kept only for record building (names, links)
    in addition to the detail allowlist.
  - `exclude`: rules that drop features before import. `require_values` keeps
    only features with exact field values (parking districts must have
    `Status: Approved`); `require_fields` and `require_point` drop features
    without a name, address or point geometry; `duplicate_of` drops features that
    duplicate a place already imported from another source, using distance and
    shared-name-word rules with a stopword list.
  - base-collector options such as `dedupe_attributes`, `description_fields`,
    `resident_description`, `street_grouping` and `source_link_fields`.
- `map_groups` / `map_layers`: map groups, layer toggles, colors, geometry kind
  and defaults. `default` turns a layer on in the UI; `api_default` marks the
  layers the API returns when a request omits `layers` (streets and snow). A
  layer with `variant_param` (snow routes, `snow_types`) exposes one child per
  source.
- `field_labels`, `date_detail_fields`, `url_detail_fields`: how allowlisted
  attributes are labeled, formatted and validated.
- `discovery`: services and pages probed by `catalog_map_discover.py`.

To add a layer, add a `sources` entry (and a `map` placement if it should appear
on the map), rerun the collector, import, and rebuild the API and web images so
they embed the updated registry. `make test-catalog` checks the registry for
internal consistency and exercises the normalizer and exclusion rules offline.

`catalog_map_expand.py --check` exits 0 only when `map_expansion.json` exists
and the manifest's `registry_fingerprint` matches a hash of the current
`expansion` registry entries plus the collector version. `make up` uses it to
re-download the expansion layers whenever their registry entries change.

## Commands

```sh
python3 scripts/catalog_test.py            # offline normalizer/registry checks
python3 scripts/catalog_collect.py         # network; base sources
python3 scripts/catalog_verify.py          # verifies locally saved batches
python3 scripts/catalog_map_expand.py      # network; requires base records.json
python3 scripts/catalog_map_test.py        # integrity + live readback for expansion
python3 scripts/catalog_expectations.py    # writes expectations.json
cargo run --locked --manifest-path backend/Cargo.toml -- validate-import catalog/records.json
make data-import CATALOG_FILE=catalog/records.json
make data-import CATALOG_FILE=catalog/map_expansion.json
```

`make data-fetch` runs the five collector/verification scripts in order;
`make stack-import` imports both files into the running container stack. Imports
require PostgreSQL with migrations applied. Indexing and an API restart (the map
projection is built once per process) are separate steps; `make up` does all of
them when needed. `examples/catalog.json` is a small fictional file for trying
the importer without network access.

## Base collector

The base collector enumerates every `base` registry layer, compares before/after
counts and exact object ID sets, saves 100-ID batches with SHA-256 hashes, and
resumes matching local batches (delete `raw/<family>/<layer>/batch_*.json` to
force a fresh download). Counts change as the publisher edits its layers. Reused
batches do not form a transactional, edit-time-consistent snapshot.
Government-directory deduplication compares attributes except OBJECTID; geometry
differences and excluded IDs are recorded in `exclusions.json`.

## Map expansion layers

`catalog_map_expand.py` downloads the `expansion` registry entries into separate
files and never modifies `records.json`: seasonal leaf sections, parks, walking
trails, bike racks and routes, MS4 watersheds, residential parking districts and
points of interest.

- `ParkingDistricts2/0`: residential parking districts with their posted hours,
  linked to the City parking page. Only districts with `Status: Approved` are
  imported. The map is not a permit determination.
- `Points_of_Interest_2023/0`: City-published points of interest. Features that
  lack a name, address or point geometry are excluded, as are features that
  duplicate a place already imported from Government Services (the same point,
  or nearby with shared name words; the two sources name places differently).
  Repeated generic names get the street address appended.

Every excluded feature is listed with its reason under its source in
`map_expansion_manifest.json`; the raw feature stays in `map_expansion_raw/`.

Layers reviewed and deliberately not imported:

- `Manassas_AEDs`: many units are mobile (in vehicles) and most fixed units are
  in staff-only shops or offices, so public access is not established.
- `Manassas_Parcels` and `Drainage_Map_view`: publish owner and utility-billing
  fields.

## Generated files

- `records.json` / `map_expansion.json`: flat importer arrays.
- `manifest.json`: retrieval time, totals, per-layer counts, batch hashes, query
  URLs and source metadata for the base catalog.
- `source_registry.json`: source URLs, official links and the reuse caveat.
- `exclusions.json`: base duplicate audit, retained/excluded IDs and geometry
  comparison.
- `map_expansion_manifest.json`: registry fingerprint, per-source counts, ID-set
  checks, allowlisted fields, excluded features with reasons, and the base
  catalog hash (confirming it was not modified).
- `map_expansion_verification.json`: results of `catalog_map_test.py`.
- `raw/<family>/<layer>/` and `map_expansion_raw/`: source metadata, ID lists,
  response batches and receipts, kept for audit only.
- `expectations.json`: written by `catalog_expectations.py` from the downloaded
  files. It records the input file hashes, record counts per file and per source,
  the envelope of all mapped geometry, feature counts per map layer, the API
  default layers, and a sample street group (the largest multi-segment street
  whose name matches no other record, so a search for it is unambiguous). The
  backend's ignored live tests load it through `backend/tests/common/mod.rs`
  instead of hardcoding counts, names or IDs.

## Provenance and limitations

Geometry is opaque Esri JSON in EPSG:4326, not GeoJSON; the backend converts it
through a validated projection. UUIDv5 identities derive from the source layer
URL and object ID. Staff/editor fields, work-order metadata and arbitrary raw
attributes stay out of resident text and embeddings; only registry allowlists
are shown. Some stormwater layers expose only OBJECTID and geometry, so their
text describes the layer rather than individual assets.

Records identify City of Manassas GIS as the data publisher, not a verified
service provider. No point, line, polygon or overlap determines eligibility,
provider, schedule or request routing. The bulk-waste polygon is a mapped
collection area, not per-address instructions, and historical water-main breaks
are not live outage alerts. Public query access is not a redistribution license;
see [DATA-REUSE.md](../DATA-REUSE.md).
