# Resident street projection

The database and OpenSearch retain original source records. The API read model groups only `record_type=route` records from sources marked `street_grouping` in the registry (`Centerline_COM/0`) with a nonblank name, normalizing whitespace and case. Other sources, layers, types and blank names remain separate. Grouping happens at read time, so stored records, the search index and embeddings are per segment.

`projection::project` is shared by database browse/fallback and the complete OpenSearch candidate list after fusion, before total/offset/limit. Database reads use one repeatable-read snapshot and an explicit 100000-record bound (oversized results fail rather than advertise a partial total). This is an in-memory backend projection over all filtered rows, not frontend/per-page deduplication or SQL materialization. If the catalog grows materially, move this read model into an indexed projection without changing the raw-source tables. The lexical leg already retrieves all matching candidates, so the index mapping and documents need no street-level fields. Grouping does not re-rank: each street occupies its first candidate's position, with the minimum member UUID as its detail link.

Lists expose `segment_count`, but omit member arrays. A detail request using ANY original member UUID resolves to the whole street. Detail `segments` retains every member UUID, source record ID, official source URL and geometry; raw attributes are never serialized. Top-level geometry is null for grouped streets, avoiding misrepresenting a representative segment as the entire street. The frontend displays a segment-count badge and native keyboard-accessible details listing official segment source links.

## Verification scope

Offline projection tests cover source/type distinctions, blank names, normalization,
UUID identity, geometry/provenance preservation and filters. The ignored
`street_groups_live` tests use the sample street group in
`catalog/expectations.json` (the largest multi-segment street whose name matches no
other record) to check grouping, counts and pagination against the imported data.
Bounded semantic retrieval may produce only a subset of indexed segments; detail
lookup resolves the full group. Search totals disclose this candidate scope.
