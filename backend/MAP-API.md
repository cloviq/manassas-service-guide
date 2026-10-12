# Map features API

Implemented in `src/map.rs` as a read model over imported records. Source-specific details and search hydration are described in [SOURCE-DETAILS.md](SOURCE-DETAILS.md).

## Contract

Layer tokens, snow variant IDs, classifications and geometry kinds come from [`catalog/sources.json`](../catalog/sources.json) (`map_layers` and each source's `map`).

`GET /api/map/features?bbox=west,south,east,north&layers=streets,snow&limit=1000`

- Required `bbox`: four finite WGS84 longitude/latitude numbers. Longitude must be within -180..180, latitude -90..90; west < east and south < north. Antimeridian-crossing boxes are not supported; split them into two requests.
- `layers`: comma-separated layer IDs from the registry's `map_layers` (see MAP-EXPANSION.md). Omitted means the layers marked `api_default` (streets and snow). Empty/unknown values rejected. Names are case-sensitive.
- `snow_types` (the snow layer's `variant_param`): optional nonempty subset of literal IDs `0,1,2,3`, comma-separated. Omitted retains all snow. Duplicates are harmless; invalid/empty values return 400. Applies only to snow, before limit/truncation. For no snow, exclude snow from layers.
- `limit`: integer 1..2000, default 1000. Stable UUID ordering. No offset/pagination: move/zoom viewport when truncated.
- Selection is **inclusive feature-envelope overlap**, not exact line/rectangle intersection. Whole intersecting-envelope features are returned, not geometries clipped to the viewport. A line can therefore have its envelope overlap the viewport without the line itself crossing it.
- Response: GeoJSON `{ "type":"FeatureCollection", "features":[...], "truncated":false }`. `truncated:true` means more matching features exist than the selected limit; never silently treat the partial result as complete.
- Each feature has `type:"Feature"`, top-level `id` (original UUID), `geometry` (GeoJSON `Point`, `LineString`/`MultiLineString` or `Polygon`/`MultiPolygon` depending on the layer, longitude/latitude), and **only** these properties:
  `{id,name,layer,source_key,source_record_id,source_url,classification,details,links}`.
- Layer is one of the supported map tokens in MAP-EXPANSION.md. Raw attributes, staff metadata and inferred operators are never serialized. `details` contains only validated source-specific descriptive scalars; `links` distinguishes source record, service information and unverified/null service action. See SOURCE-DETAILS.md for exact allowlists.

`GET /api/map/features/{id}`

- For streets, the UUID may be any original Centerline member, including the stable grouped search-result UUID. Returns every member of the same source-aware, case/whitespace-normalized street group, in UUID order, with each original ID/provenance and geometry preserved.
- Any other UUID (snow route, polygon, point) returns only that exact record; same-name records outside streets are not grouped.
- Blank-name Centerline records, if present, resolve individually (same grouping policy as the services API).
- Same FeatureCollection envelope, `truncated:false`. Detail deliberately has no viewport/2000-feature cap so it can return the complete group; it is bounded by the 10000-record projection ceiling.
- Unsupported resource or missing UUID: 404. Malformed UUID: 400.

Invalid bbox, layer, snow subtype, limit or malformed query: 400. Database/projection unavailable or unsupported stored geometry: 503 (safe message, no internal errors).

## Sources and lifecycle

See [MAP-EXPANSION.md](MAP-EXPANSION.md) for all supported layer tokens. The API
accepts EPSG:4326 Esri shapes via validated GeoJSON conversion. Unsupported or
invalid geometry fails closed. Features do not establish live status or eligibility.

The first request builds a router-local immutable map projection from PostgreSQL
under a repeatable-read transaction and bounded initialization. Restart the API
after importing map geometry (`make up` does this after an import). Run
`cargo test` for offline contracts. The ignored live tests (`map_live`,
`map_expansion_live`) check an imported catalog against `catalog/expectations.json`.
