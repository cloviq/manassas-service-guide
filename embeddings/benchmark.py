"""Run a real service subprocess, benchmark via HTTP, then stop it."""
import json
import subprocess
import time
import urllib.request
from pathlib import Path
import numpy as np

ROOT = Path(__file__).resolve().parent
BASE = 'http://127.0.0.1:3002'


def request(path, payload=None):
    req = urllib.request.Request(BASE + path,
        data=None if payload is None else json.dumps(payload).encode(),
        headers={'Content-Type': 'application/json'})
    with urllib.request.urlopen(req, timeout=30) as response:
        return json.load(response)


def main():
    with (ROOT / 'benchmark-service.log').open('w') as log:
        started = time.perf_counter()
        process = subprocess.Popen([str(ROOT / 'run.sh')], cwd=ROOT, stdout=log, stderr=log)
        try:
            deadline = time.monotonic() + 120
            while True:
                if process.poll() is not None:
                    raise RuntimeError('Service exited; inspect benchmark-service.log')
                try:
                    health = request('/health')
                    break
                except (OSError, ValueError):
                    if time.monotonic() >= deadline:
                        raise
                    time.sleep(.1)
            startup = (time.perf_counter() - started) * 1000
            listening = subprocess.check_output(['lsof', '-nP', '-a', '-p', str(process.pid), '-iTCP', '-sTCP:LISTEN'], text=True)
            assert '127.0.0.1:3002' in listening
            samples = []
            for _ in range(20):
                before = time.perf_counter()
                response = request('/v1/embeddings', {'input': 'Where can I get free groceries in Manassas?'})
                samples.append((time.perf_counter() - before) * 1000)
            texts = ['Where can I get free groceries in Manassas?',
                     'A food pantry provides free groceries and emergency food to local families.',
                     'A telescope observes distant galaxies and stellar radiation.']
            before = time.perf_counter()
            batch = request('/v1/embeddings', {'input': texts, 'model': health['model']})
            batch_ms = (time.perf_counter() - before) * 1000
            vectors = np.array([row['embedding'] for row in batch['data']])
            assert vectors.shape == (3, 384) and np.isfinite(vectors).all()
            assert np.allclose(np.linalg.norm(vectors, axis=1), 1, atol=1e-5)
            assert np.allclose(vectors[0], response['data'][0]['embedding'], atol=1e-6)
            relevant = float(vectors[0] @ vectors[1])
            irrelevant = float(vectors[0] @ vectors[2])
            assert relevant > irrelevant + .1
            rss_kib = int(subprocess.check_output(['ps', '-o', 'rss=', '-p', str(process.pid)], text=True))
            before = time.perf_counter()
            stress = request('/v1/embeddings', {'input': ['food pantry ' * 680] * 16})
            stress_ms = (time.perf_counter() - before) * 1000
            assert len(stress['data']) == 16
            rss_stress = int(subprocess.check_output(['ps', '-o', 'rss=', '-p', str(process.pid)], text=True))
            report = {'health': health, 'cached_startup_ms': startup,
                'requests': len(samples), 'single_first_ms': samples[0],
                'single_median_ms': float(np.median(samples)), 'single_p95_ms': float(np.percentile(samples, 95)),
                'batch_3_ms': batch_ms, 'rss_mib': rss_kib / 1024,
                'batch_16_long_text_ms': stress_ms, 'rss_after_long_text_mib': rss_stress / 1024,
                'related_cosine': relevant, 'unrelated_cosine': irrelevant,
                'norms': np.linalg.norm(vectors, axis=1).tolist(),
                'binding': listening.strip(), 'numerical_checks': 'passed'}
            (ROOT / 'verification.json').write_text(json.dumps(report, indent=2) + '\n')
            print(json.dumps(report, indent=2))
        finally:
            process.terminate()
            process.wait(timeout=15)
    assert 'Where can' not in (ROOT / 'benchmark-service.log').read_text()


if __name__ == '__main__':
    main()
