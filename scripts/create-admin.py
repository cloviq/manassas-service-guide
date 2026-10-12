#!/usr/bin/env python3
"""Bootstrap an admin account in the running stack; never print the password."""
import os
import secrets
import subprocess
from pathlib import Path
root = Path(__file__).resolve().parent.parent
folder = root / "secrets"
target = folder / "admin-credentials"
user = os.environ.get("ADMIN_USER") or "admin"
if target.exists():
    raise SystemExit("secrets/admin-credentials already exists; create more accounts from the UI.")
folder.mkdir(mode=0o700, exist_ok=True)
password = secrets.token_urlsafe(18)
subprocess.run(["bash", "scripts/infra.sh", "exec", "-T", "api", "service-guide", "create-user", user, "admin"],
               cwd=root, input=password + "\n", text=True, check=True)
fd = os.open(target, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
with os.fdopen(fd, "w") as out:
    out.write(f"{user}:{password}\n")
print(f"Password for '{user}' saved to secrets/admin-credentials (0600).")
