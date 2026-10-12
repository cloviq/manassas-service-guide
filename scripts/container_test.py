"""Offline deployment contract tests: no Docker daemon, model or credentials."""
from pathlib import Path
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
        self.assertEqual(compose.count('restart: unless-stopped'), 5)
        self.assertEqual(compose.count('healthcheck:'), 5)
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

    def test_stack_is_local_only(self):
        compose = (ROOT / 'compose.yaml').read_text()
        self.assertIn('name: manassas-service-guide\n', compose)
        published = [line.strip() for line in compose.splitlines() if line.strip().startswith('- "') and ':' in line and '/' not in line]
        self.assertIn('- "127.0.0.1:5174:80"', published)
        self.assertTrue(all(p.startswith('- "127.0.0.1:') for p in published), published)

if __name__ == '__main__':
    unittest.main()
