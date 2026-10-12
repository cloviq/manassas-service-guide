# Source-specific resident details and link contract

Source details are a read model over locally imported records; see `MAP-EXPANSION.md` for map-layer fields. Per-source allowlists, caveats, narrowed source-record links and resident description templates are defined in [`catalog/sources.json`](../catalog/sources.json); `src/projection.rs` only applies them.

## Response fields

Both service list/detail resources and map feature `properties` include:

- `details: Record<string, string | number>`: safe source-specific fields, retaining published field names. Render as plain text, not HTML. Unknown fields, wrong types, nulls, blanks, control characters and strings over 2000 bytes are omitted. Numeric address/speed/date fields must be finite positive integral JSON numbers; zero address placeholders are omitted. No unit or code translation is inferred.
- `links: {source_record: string, service_information: string | null, service_action: null}`. The source-record link is the exact ArcGIS layer query with `objectIds`; it is not a request/submission endpoint. A distinct non-hub official URL is exposed as service information, not as a verified action; GIS sources provide no action endpoint. `source_url` keeps the original provenance. When a record's `official_url` is only the general City GIS hub, the API points it at the exact source record instead; other information links are returned as stored.

Street groups expose only details identical on every member. Per-segment details remain available in service detail `segments[].details`, alongside original geometry, UUID and source provenance. Never describe a representative segment's address range or speed as the whole street. Browse responses omit the segment array. Snow descriptions use actual NAME + TYPE and exact published classification; street descriptions use FULLSTREET where stored.

Search index documents intentionally omit raw attributes. Ranked results hydrate attributes from PostgreSQL by candidate UUID before the same resident projection, so details work without reindexing. Raw attributes remain excluded from serialization. Nonempty search results therefore require PostgreSQL availability; database failure returns safe 503 rather than fabricating details.

## Field allowlists (base sources)

The registry's `details` entries define these allowlists; the projection exposes only these stored fields. If the publisher changes a layer's schema, update the registry entry. Map-layer sources are listed in `MAP-EXPANSION.md`.

| Source | Strings | Positive integral numbers |
| --- | --- | --- |
| Centerline_COM/0 | FULLSTREET, NAME, PREFIX, TYPE, SUFFIX, ZIPL, ZIPR | L_F_ADD, L_T_ADD, R_F_ADD, R_T_ADD, SPEEDLIMIT |
| Snow_Routes/0..2 | NAME, TYPE | none |
| Snow_Routes/3 | NAME, PREFIX, TYPE, SUFFIX, ZIPL, ZIPR | L_F_ADD, L_T_ADD, R_F_ADD, R_T_ADD |
| Manassas_Government_Services_WFL1/0 | Address2, City, State, Zip, Phone | none |
| watermain_breaks_view/0 | LOCDESC, BREAKLEAK_TYPE | INCIDENTDATE, REPAIRDATE (ArcGIS UTC epoch milliseconds; historical only) |
| COM_StormData_Public_View/3 | Location, DischType, Material, RecvWater | none |
| COM_StormData_Public_View/8 | FacName, BMP_Type | none |
| Manassas_Bulk_Waste_Collection/0 | DistrictName2, Agency (source-listed only), Phone, Website2 (HTTPS validated) | none |

Other imported storm layers store only OBJECTID and therefore cannot truthfully provide richer descriptive fields. Staff/editor/creator/GPS names, contacts, free-form notes/comments, work orders, unverified agency/operator fields, opaque codes and unverified shape-length units are not promoted. Provider and jurisdiction wording comes from the registry's `publisher` entry. Raw source-record pages themselves may expose more source fields; the app's JSON allowlist does not reproduce them.

Snow_Routes/3 uses **OBJECTID_1**, not its separate OBJECTID attribute, as its object ID. `source_record_id` and the `objectIds` provenance use that ID; do not reconstruct links from attributes. The ignored `details_live` test checks every stored link and detail against the source schema.

## Snow subset query

`GET /api/map/features?bbox=-78,38,-77,39&layers=snow&snow_types=0,2&limit=2000`

`snow_types` is optional. Omitted means all four types. Present means a nonempty comma-separated subset of literal IDs `0,1,2,3`; duplicates are harmless. Empty, unknown, padded, negative or nonliteral IDs reject with 400 before DB access. It affects only snow; streets remain controlled by `layers`. To select no snow, omit snow from `layers` rather than sending an empty snow_types value. Filtering happens before limit and truncated calculation. Exact titles: 0 Tertiary Route; 1 Second Run Route; 2 First Run Route; 3 Snow Emergency Route. They are source classifications, not live progress or timing promises.

## Bulk geometry

Bulk collection polygons are separate from yard waste and seasonal leaf areas.
They do not establish address eligibility, a collection day or current provider.
The validated rings-to-GeoJSON conversion is described in `MAP-EXPANSION.md`.
