# Local embedding service

CPU ONNX embeddings with BAAI/bge-small-en-v1.5, 384 dimensions, FastEmbed 0.8.1
and ONNX Runtime 1.30.0. Requirements are pinned for Python 3.12; verify wheel
availability on your platform. Model weights are downloaded at first startup and
remain in ignored `.cache/` or the Compose `model_cache` volume.

```sh
python3.12 -m venv .venv
.venv/bin/python -m pip install -r requirements.txt
./run.sh
```

Host binding defaults to 127.0.0.1:3002. Compose sets BIND_HOST=0.0.0.0 internally
without publishing the embedding port. Stop foreground service with Ctrl-C.
`GET /health` reports model/dimension/provider. `POST /v1/embeddings` accepts string
or string-array input and optional matching model, returning indexed float vectors.

Queries/documents use the same plain-text `embed()` path without prefixes; tokenizer
truncates at 512 tokens, uses CLS pooling and L2 normalization. Re-embed all records
when changing model/preprocessing. Max 32 texts / request, 8192 characters / text,
131072-byte body; microbatches of 8; serial HTTP handling; two native threads.
No input persistence or body access logging. Generic errors omit submitted text.
This is a trusted local service without TLS/auth/CORS, not a public endpoint.

Upstream weights: https://huggingface.co/BAAI/bge-small-en-v1.5
ONNX conversion: https://huggingface.co/Qdrant/bge-small-en-v1.5-onnx-Q
Check the downloaded artifact's license and notices before redistributing it.
FastEmbed resolves its upstream source; this application does not pin a model
snapshot or bundle weights.

```sh
.venv/bin/python -m pytest -q   # real model; may download it
.venv/bin/python benchmark.py  # binds port 3002, measures, then stops subprocess
.venv/bin/python -m pip check
```

The benchmark writes ignored `verification.json`. Latency/memory results depend
on the actual host. Offline container tests stub the embedding package only to
verify listener configuration; they do not establish inference correctness.
