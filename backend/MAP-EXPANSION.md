# Map layers contract

Every map layer, its source keys and its geometry kind are defined in
[`catalog/sources.json`](../catalog/sources.json) (`map_layers` and each source's
`map` placement). The backend compiles the registry in; data comes from the
locally downloaded and imported catalog.

## Layers

`GET /api/map/features?bbox=-78,38,-77,39&layers=leaves,bulk&limit=2000`

| Layer | Source key | GeoJSON |
|---|---|---|
| streets | Centerline_COM/0 | LineString / MultiLineString |
| snow | Snow_Routes/0..3 | LineString / MultiLineString |
| leaves | Leaf_Collection_Sections/0 | Polygon / MultiPolygon |
| bulk | Manassas_Bulk_Waste_Collection/0 | Polygon / MultiPolygon |
| parks | Parks_Manassas/0 | Polygon / MultiPolygon |
| trails | Manassas_WalkingTrails/0 | LineString / MultiLineString |
| bike_racks | Bike_Facilities_and_Racks/0 | Point |
| bike_routes | Bike_Facilities_and_Racks/1 | LineString / MultiLineString |
| watersheds | MS4_Watershed/0 | Polygon / MultiPolygon |
| parking | ParkingDistricts2/0 | Polygon / MultiPolygon |
| places | Points_of_Interest_2023/0 | Point |

When `layers` is omitted the API returns the registry's `api_default` layers
(streets and snow). The frontend turns on the layers marked `default` (streets,
snow and seasonal leaves) and leaves the rest opt-in; `default` is a UI setting,
not an API property. Unknown, padded or empty layer tokens are rejected. Limit,
`truncated` and detail-by-UUID semantics are the same for every layer; only street
segments are grouped.

Feature properties are exactly nine keys: id, name, layer, source_key,
source_record_id, source_url, classification, details, links. Raw attributes are
never serialized. For bulk-waste areas, `links.source_record` is rebuilt from the
stored object ID with the registry's narrowed `source_link_fields` instead of a
wildcard field list; `source_url` keeps the original provenance. Expansion
source URLs are generated with explicit field lists. `service_action` is always
null; information links are not submission endpoints.

## Safe details

Each source's detail allowlist is its `details` entry in the registry. Nulls,
blanks, unexpected types, audit metadata and oversized strings are dropped. Bike
`Miles` allows finite nonnegative fractions; `Trail` and `LASTUPDATE` must be
positive integral numbers, and no labels or date years are invented. `Website2`
and `AGENCYURL` must be HTTPS URLs without credentials. `DESCRIPT` alone may keep
CR/LF because the published leaf windows are multiline; render it as plain text,
never HTML.

Two registry caveats are added to details:

- `leaves.details.schedule_caveat`: "Published seasonal windows; year not stated.
  Confirm current season on the official map. Not a next-pickup prediction."
  `DESCRIPT` is kept verbatim; no year is chosen and no pickup is calculated.
- `bulk.details.agency_caveat`: the source-listed agency is not an independently
  verified current operator, and the polygon publishes no schedule.

Parks do not establish public access. Bike `Status` is shown as published,
including proposed facilities. Watersheds are context, not incident or
drainage-routing determinations. Parking districts are not permit
determinations. Points of interest are published snapshots, not proof of hours,
access or services.

## Geometry and lifecycle

The converter requires EPSG:4326 (or the collector contract's omitted reference),
validates coordinate ranges, closes rings, derives nesting independent of winding,
emits holes/islands and rejects crossing or degenerate boundaries. It is not a
repair engine for arbitrary invalid or antimeridian geometry. The map cache is
immutable for the API process; restart it after geometry imports.

The ignored `map_expansion_live` test checks the imported layers (IDs, details and
geometry) against `catalog/expectations.json`, generated from the downloaded files.
