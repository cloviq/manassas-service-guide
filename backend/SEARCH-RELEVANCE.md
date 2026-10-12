# Bounded semantic retrieval and abstention

## Why a semantic gate

Reciprocal-rank fusion orders candidates; it cannot decide whether any candidate
is relevant. An ungated vector leg scores every record that survives the
type/category filters, so an unrelated query such as `pizza delivery` would
return the whole filtered catalog. The semantic leg therefore abstains below a
minimum similarity, and the gate is applied before fusion.

The semantic leg uses Lucene HNSW with internal exact type/category filters,
`k=100`, `size=100` and a minimum cosine of 0.615 (`MIN_SEMANTIC_COSINE` in
`src/search.rs`). It is a single search request. The 100-candidate bound is a
retrieval and latency budget, not a relevance test. The lexical leg retrieves
complete AND matches under a 100000-record bound, so every matching street
segment reaches the shared projection and grouping. Fusion is RRF with k=60 and
a stable ID tie break.

`search_mode=hybrid` means both legs ran, even when both correctly returned no
candidates. Model/vector failures return `keyword` with a warning; search
transport failures fall back to `database`. The gate is enabled only for
BAAI/bge-small-en-v1.5 at 384 dimensions; other model/dimension combinations
fall back explicitly until calibrated.

`total` is computed after rejection, fusion, deduplication and resident
projection, before pagination. `total_scope=lexical_plus_bounded_semantic_candidates`
and `semantic_candidate_limit=100` distinguish it from an exhaustive count of
every semantically related document. Keyword/database modes expose
`lexical_matches` / `database_matches`. ANN-only street results may contain only a
subset of a street's segments; the detail endpoint resolves the complete group.

## Calibration

OpenSearch's Lucene cosine space reports `score = (1 + cosine) / 2`, so the query's
`min_score` is `(1 + 0.615) / 2`. `measure_relevance.py` checks this transform
against direct dot products of the stored and query vectors (tolerance 0.00001)
rather than assuming it.

0.615 sits between the highest-scoring unrelated queries (gibberish, `buy concert
tickets`, `passport renewal`, `pizza delivery`, all below about 0.59 on this
corpus) and the weakest clear positive (a snow route for `snow`, about 0.64). It is
a conservative abstention policy chosen on a small labeled sample, not a
relevance probability. Lowering it to rescue one paraphrase risks admitting
unrelated results, so any change needs the negative queries rerun. Recalibrate
whenever the corpus, model or index mapping changes.

Some ambiguous paraphrases score in the same range as unrelated queries (for
example `when does the truck come`, `dispose of an old sofa`) and therefore
abstain rather than return a wrong top result.

## Lexical civic synonyms

Single everyday words that City text never uses sit just under the gate: in
calibration `garbage`, `refuse` and `rubbish` peaked at about 0.60, against
0.615 for the gate and about 0.58 for `pizza delivery`. Lowering the gate would
leave a margin of about 0.01, so the gap is closed on the lexical side instead.
The keyword leg uses a search-time `civic_search` analyzer (a `synonym_graph`
filter built from `CIVIC_SYNONYMS` in `src/search.rs`) with a small set of
equivalence groups: garbage/trash/refuse/rubbish, bin/can/cart/container/toter,
recycle/recycling/recyclables, pickup/collection, leaf/leaves, couch/sofa and
plow/plowing/plowed. Matching remains exact AND over the expanded terms, so a
synonym can only match text that contains an equivalent word.

The analyzer is defined in the index settings, so changing the list requires a
reindex. `SEARCH_SCHEMA_VERSION` is bumped with such changes, which makes
`index --if-stale` (run by `make up`) rebuild automatically; `make stack-index`
forces it. An index without the analyzer fails lexical queries and the API falls
back to database search.

## Service tasks and the `snow` query

Migration 0005 adds three source-linked snow service tasks (Streets &
sidewalks). `snow` restricted to services returns those tasks and no trash task;
`snow` across all types ranks a snow service first and still returns every
snow-route resource. `tests/search_relevance_live.rs` asserts this, that
`garbage` reaches the trash tasks through the civic synonyms, and that the
unrelated queries return nothing.

## Measuring

- `measure_relevance.py` queries the local embedding service and OpenSearch
  (read-only) for a fixed set of positive, ambiguous and unrelated queries, checks
  the score transform and writes the ignored `semantic-calibration.json`.
- `tests/relevance.rs` (ignored) runs a fixed labeled query set through the real
  API and prints top-1 results. With `RELEVANCE_REPORT=path` it also writes a JSON
  report. It reports misses rather than asserting that every paraphrase works.
- `tests/search_relevance_live.rs` and `tests/live_hybrid.rs` (ignored) assert
  abstention, filtering and pagination against the real services.

Offline tests use local mock HTTP servers and check contracts, not model quality.
See the root README for the environment the live tests need.
