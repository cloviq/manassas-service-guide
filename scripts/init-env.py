#!/usr/bin/env python3
"""Create isolated local development credentials, never overwrite or print them."""
import os
import secrets
from pathlib import Path
root = Path(__file__).resolve().parent.parent
password = secrets.token_hex(24)
values = {
    "POSTGRES_USER": "guide_dev",
    "POSTGRES_PASSWORD": password,
    "POSTGRES_DB": "service_guide",
    "DATABASE_URL": f"postgres://guide_dev:{password}@127.0.0.1:5433/service_guide",
    "OPENSEARCH_URL": "http://127.0.0.1:9200",
    "OPENSEARCH_INDEX": "manassas-catalog",
    "EMBEDDING_URL": "http://127.0.0.1:3002/v1/embeddings",
    "EMBEDDING_MODEL": "BAAI/bge-small-en-v1.5",
    "EMBEDDING_DIMENSION": "384",
    "EMBEDDING_BATCH_SIZE": "16",
    "PORT": "3001",
}
path = root / os.environ.get("ENV_FILE", "backend/dev.env")
try:
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
except FileExistsError:
    print("Private environment already exists; left unchanged.")
else:
    with os.fdopen(fd, "w") as target:
        target.write("".join(f"{key}={value}\n" for key, value in values.items()))
    print("Created private environment mode 0600; credentials not printed.")
