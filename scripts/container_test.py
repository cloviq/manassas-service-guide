"""Offline deployment contract tests: no Docker daemon, model or credentials."""
from pathlib import Path
import hashlib
import json
import subprocess
import unittest

ROOT = Path(__file__).resolve().parent.parent

class ContainerTests(unittest.TestCase):
    def test_embedding_bind_default_and_override(self):
        import importlib.util
        import os
        import sys
        from unittest.mock import patch, MagicMock
        spec = importlib.util.spec_from_file_location('container_embedding', ROOT / 'embeddings/service.py')
        assert spec is not None and spec.loader is not None
        module = importlib.util.module_from_spec(spec)
        with patch.dict(sys.modules, fastembed=MagicMock()):
            spec.loader.exec_module(module)
        with patch.object(module, 'Engine'), patch.dict(os.environ, {}, clear=True):
            with module.make_server(port=0) as server:
                self.assertEqual(server.server_address[0], '127.0.0.1')
            with patch.dict(os.environ, BIND_HOST='0.0.0.0'):
                with module.make_server(port=0) as server:
                    self.assertEqual(server.server_address[0], '0.0.0.0')

    def test_dockerfiles_support_standard_builder(self):
        for folder in ('backend', 'frontend', 'embeddings'):
            self.assertNotIn('--mount=', (ROOT / folder / 'Dockerfile').read_text())

    def test_full_stack_wiring(self):
        compose = (ROOT / 'compose.yaml').read_text()
        for service in ('postgres', 'opensearch', 'embeddings', 'api', 'web'):
            self.assertIn(f'  {service}:\n', compose)
        self.assertEqual(compose.count('restart: unless-stopped'), 6)
        self.assertEqual(compose.count('healthcheck:'), 6)
        for value in ('127.0.0.1:5174:80', 'http://opensearch:9200',
                      'http://embeddings:3002/v1/embeddings', 'manassas-catalog',
                      'pg_data:/var/lib/postgresql/data', 'search_data:/usr/share/opensearch/data'):
            self.assertIn(value, compose)
        self.assertNotIn(':3002:3002', compose)
        for target in ('up', 'down', 'status', 'logs', 'docker-build'):
            result = subprocess.run(['make', '-n', target], cwd=ROOT, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotIn('down -v', result.stdout)
            self.assertNotIn('dev.py index', result.stdout)
            self.assertNotIn('catalog_collect', result.stdout)
        nginx = (ROOT / 'frontend/nginx.conf').read_text()
        self.assertIn('proxy_pass http://api:3001;', nginx)
        self.assertIn('try_files $uri $uri/ /index.html;', nginx)

    def test_sessions_replace_basic_auth(self):
        nginx = (ROOT / 'frontend/nginx.conf').read_text()
        self.assertNotIn('auth_basic', nginx)
        self.assertIn('location = /api/auth/login {\n        limit_req zone=login', nginx)
        compose = (ROOT / 'compose.yaml').read_text()
        self.assertNotIn('htpasswd', compose)
        self.assertIn('http://127.0.0.1:3001/healthz', compose)
        self.assertIn('secrets/', (ROOT / '.gitignore').read_text().splitlines())
        result = subprocess.run(['make', '-n', 'admin'], cwd=ROOT, capture_output=True, text=True)
        self.assertIn('create-admin.py', result.stdout)

    def test_up_ensures_data_idempotently_with_opt_out(self):
        up = subprocess.run(['make', '-n', 'up'], cwd=ROOT, capture_output=True, text=True).stdout
        self.assertIn('scripts/stack-data.sh ensure', up)
        self.assertIn('if [ -z "$(SKIP_DATA)" ]', (ROOT / 'Makefile').read_text())
        self.assertLess(up.index('--wait'), up.index('stack-data.sh ensure'))
        script = (ROOT / 'scripts/stack-data.sh').read_text()
        for guard in ('[[ ! -f catalog/records.json ]]', 'index --if-stale',
                      "SELECT count(*) FROM users", '"$imported" -lt "$expected"',
                      'catalog_map_expand.py --check', 'restart api',
                      'reindex=1'):
            self.assertIn(guard, script)

    def test_waste_journey_requires_existing_session(self):
        nginx = (ROOT / 'frontend/nginx.conf').read_text()
        journey = nginx.split('location = /waste-collection.html {', 1)
        self.assertEqual(len(journey), 2, 'The static learning page must retain the sign-in boundary')
        protected = journey[1].split('}', 1)[0]
        self.assertIn('auth_request /_waste_auth;', protected)
        self.assertIn('error_page 401 = @waste_sign_in;', protected)
        redirect = nginx.split('location @waste_sign_in {', 1)[1].split('}', 1)[0]
        self.assertIn('absolute_redirect off;', redirect)
        self.assertIn('return 302 /;', redirect)
        self.assertIn('try_files $uri =404;', protected)
        self.assertIn('Cache-Control "private, no-store"', protected)
        check = nginx.split('location = /_waste_auth {', 1)[1].split('}', 1)[0]
        self.assertIn('internal;', check)
        self.assertIn('proxy_pass http://api:3001/api/auth/me;', check)
        self.assertIn('proxy_pass_request_body off;', check)
        self.assertIn('proxy_set_header Cookie $http_cookie;', check)

    def test_existing_migration_checksums_are_immutable(self):
        # SHA-384 values recorded by SQLx in the original deployed database.
        # Even editing a comment in one of these files prevents API startup.
        expected = {
            '0001_catalog.sql': '89dd7904c102ae499525e04a74511bac097535a10ac49876b72a1e543d3109706c94da40f8f97f21cac9b62014b284c1',
            '0003_expanded_catalog.sql': '2a4e12577088093518e377d66eb4b58270afe65c528623c0d80c137b1f8c896819058e46c5577b3893e92f37452d0d61',
            '0004_accounts.sql': '5def60e3e3e06af39d067401af8073f89b660b87b0c86989d83542a21987aa5890518c854871f84f6dfe3fee1922da38',
            '0005_snow_services.sql': 'a31c43992f6c6a82d7d65c2fce0cc764b034516e406747fef39ad99089c5915f0c1702dc5b68e66fa8dcbba6605a5c16',
            '0006_utility_and_street_services.sql': 'ebd08930e6eb6809323986d0e1d1b8b47aa224f61e97f4d3016904769e0332564509bd45a3d89810cadbf531dce8b356',
        }
        for name, checksum in expected.items():
            with self.subTest(migration=name):
                self.assertEqual(hashlib.sha384((ROOT / 'backend/migrations' / name).read_bytes()).hexdigest(), checksum)

    def test_funnel_retains_identity_and_targets_authenticated_web(self):
        compose = (ROOT / 'compose.yaml').read_text()
        tailscale = compose.split('  tailscale:\n', 1)[1].split('\nvolumes:', 1)[0]
        for setting in ('profiles: [funnel]', 'ts_state:/var/lib/tailscale',
                        'TS_AUTH_ONCE: "true"', 'TS_HOSTNAME: manassas',
                        './secrets/ts-authkey:/run/secrets/ts-authkey:ro'):
            self.assertIn(setting, tailscale)
        self.assertNotIn('ports:', tailscale)
        serve = json.loads((ROOT / 'tailscale/serve.json').read_text())
        host = '${TS_CERT_DOMAIN}:443'
        self.assertEqual(serve['TCP']['443'], {'HTTPS': True})
        self.assertEqual(serve['Web'][host]['Handlers']['/'], {'Proxy': 'http://web:80'})
        self.assertEqual(serve['AllowFunnel'], {host: True})
        up = subprocess.check_output(['make', '-n', 'up'], cwd=ROOT, text=True)
        self.assertNotIn('--profile funnel', up)
        for target, command in (('funnel-up', 'up -d --no-build --wait'),
                                ('funnel-down', 'stop tailscale'),
                                ('down', 'down'), ('status', 'ps'), ('logs', 'logs')):
            dry_run = subprocess.check_output(['make', '-n', target], cwd=ROOT, text=True)
            self.assertIn(f'--profile funnel {command}', dry_run)
            self.assertNotIn('down -v', dry_run)

    def test_installed_project_volumes_and_loopback_ports_are_preserved(self):
        compose = (ROOT / 'compose.yaml').read_text()
        self.assertIn('name: manassas-service-guide-v2\n', compose)
        volumes = compose.split('\nvolumes:\n', 1)[1]
        for name in ('pg_data', 'search_data', 'model_cache', 'ts_state'):
            self.assertIn(f'  {name}:\n', volumes)
        published = [line.strip() for line in compose.splitlines() if line.strip().startswith('- "') and ':' in line and '/' not in line]
        self.assertIn('- "127.0.0.1:5174:80"', published)
        self.assertTrue(all(p.startswith('- "127.0.0.1:') for p in published), published)

if __name__ == '__main__':
    unittest.main()
