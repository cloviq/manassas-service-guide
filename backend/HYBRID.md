# Catalog and hybrid search

Use root Make targets, which read the local env file without shell execution:
`make migrate`, `make data-import`, `make index`,
then `make api`. Each operation is explicit. The default alias is `manassas-catalog`.

Indexing snapshots DB records, reads keyset pages, embeds name/description as plain
text (no prefixes; 3500-character input bound and model 512-token truncation), and
writes a UUID-suffixed physical index. It refreshes, checks counts, switches the
alias and reads it back, then deletes the replaced indexes. Failure may leave an
unaliased staging index. Bounds: 2-hour indexing deadline, 60-second model request,
batches 1–128, dimensions 1–4096, catalog <=100000 records / 128 MiB.

Retrieval combines BM25 AND matches with a bounded Lucene HNSW cosine leg, exact
category/type filters and deterministic reciprocal-rank fusion (k=60; UUID tie
break). See [SEARCH-RELEVANCE.md](SEARCH-RELEVANCE.md). The semantic leg is capped
at 100 candidates and gated by minimum cosine 0.615 for the configured BGE model.
The threshold is corpus- and model-specific; recalibrate when either changes.
Totals describe lexical matches plus bounded semantic candidates, not all relevant
documents. Model/vector failures return keyword search with a warning; transport
failures use database fallback. Empty browse reads PostgreSQL.

`/api/services`: query <=200 characters, category <=200 bytes, type defaults to
service, limit 1–100 (default 50), offset 0–100000. Types: all, service, place,
route, outage, infrastructure. Projection/grouping happens before pagination.

Import accepts an array or `{records:[...]}`, validates IDs/types/bounds/URLs,
upserts transactionally, protects seed IDs, never deletes records/tickets and
reads back imported IDs. Missing provider_name becomes `Operator not verified`.
Raw attributes persist locally but are excluded from public JSON and embeddings.
Source geometry remains opaque Esri JSON until validated map conversion.

Run non-ignored tests with `cargo test`. Ignored live tests need real services
and an imported catalog; they compare against `catalog/expectations.json`, which
`make data-fetch` writes from the downloaded data (see the root README).
