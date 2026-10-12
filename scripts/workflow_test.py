"""Offline workflow smoke tests; never start infrastructure or inference."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent


class WorkflowTests(unittest.TestCase):
    def make(self, *args, env=None):
        return subprocess.run(['make', '--no-print-directory', *args], cwd=ROOT,
                              env=env, text=True, capture_output=True)

    def test_default_is_help(self):
        default = self.make()
        self.assertEqual(default.returncode, 0, default.stderr)
        self.assertEqual(default.stdout, self.make('help').stdout)
        for target in ('setup', 'build', 'test', 'infra-up', 'migrate', 'index'):
            self.assertIn(target, default.stdout)

    def test_parallel_env_is_private_and_preserved(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'private.env'
            env = dict(os.environ, ENV_FILE=str(path), PYTHON='python3')
            # Independent invocations also exercise exclusive creation races.
            workers = [subprocess.Popen(['make', '-j8', 'env'], cwd=ROOT, env=env,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE) for _ in range(4)]
            results = [(worker, worker.communicate()) for worker in workers]
            for worker, (out, err) in results:
                self.assertEqual(worker.returncode, 0, err.decode())
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
            sentinel = b'POSTGRES_PASSWORD=secret$with#characters\n'
            path.write_bytes(sentinel)
            result = self.make('-j8', 'env', env=env)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(path.read_bytes(), sentinel)
            self.assertNotIn('secret', result.stdout + result.stderr)

    def test_safe_dry_runs_and_parallel_setup_order(self):
        for target in ('check-tools', 'setup', 'build', 'test', 'infra-up',
                       'infra-down', 'infra-status', 'migrate', 'api', 'web',
                       'embeddings', 'data-fetch', 'data-import', 'index'):
            result = self.make('-n', '-j8', target)
            self.assertEqual(result.returncode, 0, target + result.stderr)
            self.assertNotIn('down -v', result.stdout)
            self.assertNotIn('unselected-context', result.stdout)
            if target in ('setup', 'build', 'test'):
                self.assertNotIn('dev.py index', result.stdout)
                self.assertNotIn('embeddings/run.sh', result.stdout)
                self.assertNotIn('--ignored', result.stdout)
                self.assertNotIn('catalog_map_test.py', result.stdout)  # live HTTP readbacks
        setup = self.make('-n', '-j8', 'setup').stdout
        self.assertLess(setup.index('check-tools'), setup.index('init-env.py'))
        self.assertLess(setup.index('init-env.py'), setup.index('dev.py setup'))

    def test_env_loader_never_executes_or_expands_values(self):
        import importlib.util
        path_to_module = ROOT / 'scripts/dev.py'
        self.assertTrue(path_to_module.is_file(), 'secure developer launcher is missing')
        spec = importlib.util.spec_from_file_location('dev', path_to_module)
        assert spec is not None and spec.loader is not None
        dev = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(dev)
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'env'
            marker = Path(tmp) / 'executed'
            path.write_text('PASSWORD="$(touch ' + str(marker) + ')$HOME#literal"\n')
            values = dev.load_env(path)
            self.assertEqual(values['PASSWORD'], '$(touch ' + str(marker) + ')$HOME#literal')
            self.assertFalse(marker.exists())
            path.write_text('source /tmp/not-allowed\n')
            with self.assertRaises(ValueError):
                dev.load_env(path)

    def test_infra_preserves_context_and_volumes(self):
        with tempfile.TemporaryDirectory() as tmp:
            docker = Path(tmp) / 'docker'
            docker.write_text('#!/bin/sh\nprintf "%s\\n" "${DOCKER_CONTEXT:-CURRENT}" "$@"\n')
            docker.chmod(0o755)
            (Path(tmp) / 'docker-compose').symlink_to(docker)
            private = Path(tmp) / 'env'
            private.write_text('POSTGRES_PASSWORD=must-not-print\n')
            for context in (None, 'chosen-test-context'):
                env = dict(os.environ, PATH=tmp + ':' + os.environ['PATH'], ENV_FILE=str(private))
                env.pop('DOCKER_CONTEXT', None)
                if context:
                    env['DOCKER_CONTEXT'] = context
                result = subprocess.run(['bash', 'scripts/infra.sh', 'down'], cwd=ROOT,
                                        env=env, text=True, capture_output=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn(context or 'CURRENT', result.stdout)
                self.assertNotIn('unselected-context', result.stdout)
                self.assertNotIn('must-not-print', result.stdout)
                self.assertNotIn('-v', result.stdout.splitlines())


if __name__ == '__main__':
    unittest.main()
