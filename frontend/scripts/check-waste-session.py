#!/usr/bin/env python3
"""Optional Nginx session-boundary smoke test. No live data or network in container.
Run after the frontend build. Requires Docker and official nginx:1.28-alpine.
DOCKER may name the Docker executable; DOCKER_CONFIG/DOCKER_HOST work normally.
"""
from pathlib import Path
import os
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
MOCK = r"""
# Test-only session endpoint; never installed in production.
server {
 listen 127.0.0.1:8081;
 location = /api/auth/me {
  if ($http_cookie = "session=test-valid") { return 200 '{}'; }
  if ($http_cookie = "session=test-forbidden") { return 403; }
  if ($http_cookie = "session=test-unavailable") { return 503; }
  return 401;
 }
}
"""
CHECK = r"""#!/bin/sh
set -eu
nginx -t
nginx
request() {
  method="$1"; path="$2"; cookie="$3"; expected="$4"
  { printf '%s %s HTTP/1.1\r\nHost: guide.example\r\nX-Forwarded-Proto: https\r\nCookie: %s\r\nConnection: close\r\n\r\n' "$method" "$path" "$cookie"; sleep 0.3; } | nc -w 2 127.0.0.1 80 > /tmp/response
  head -1 /tmp/response
  if [ "$expected" = 302 ]; then
    grep -q '^Location: /' /tmp/response
    ! grep -q '^Location: http' /tmp/response
  fi
  grep -q "HTTP/1.1 $expected" /tmp/response
}
request GET /waste-collection.html '' 302
grep -q '^Location: /' /tmp/response
! grep -q '^Location: http' /tmp/response
request GET /waste-collection.html session=invalid 302
request GET /waste-collection.html session=test-valid 200
grep -q 'Where does it go' /tmp/response
grep -q 'Cache-Control: private, no-store' /tmp/response
request GET /waste-collection.html session=test-forbidden 403
request GET /waste-collection.html session=test-unavailable 500
request GET /_waste_auth session=test-valid 404
request HEAD /waste-collection.html '' 302
request HEAD /waste-collection.html session=test-valid 200
request GET /waste-collection.html?source=test '' 302
request GET //waste-collection.html '' 302
request GET /a/../waste-collection.html '' 302
request GET /%77aste-collection.html '' 302
echo 'PASS: session gate, fail-closed responses, internal protection, HEAD, query and normalized paths'
"""

if __name__ == '__main__':
    dist = ROOT / 'frontend/dist'
    if not (dist / 'waste-collection.html').is_file():
        raise SystemExit('Run npm run build in frontend first.')
    with tempfile.TemporaryDirectory(prefix='manassas-waste-session-') as directory:
        temp = Path(directory)
        config = (ROOT / 'frontend/nginx.conf').read_text().replace(
            'http://api:3001', 'http://127.0.0.1:8081') + MOCK
        (temp / 'default.conf').write_text(config)
        (temp / 'check.sh').write_text(CHECK)
        subprocess.run([
            os.environ.get('DOCKER', 'docker'), 'run', '--rm', '--network', 'none',
            '-v', f'{temp}/default.conf:/etc/nginx/conf.d/default.conf:ro',
            '-v', f'{temp}/check.sh:/tmp/check.sh:ro',
            '-v', f'{dist}:/usr/share/nginx/html:ro',
            '--entrypoint', 'sh', 'nginx:1.28-alpine', '/tmp/check.sh',
        ], check=True)
