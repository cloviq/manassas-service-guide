#!/usr/bin/env python3
"""Small, shell-free developer launcher; private values never enter Make recipes."""
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent


def load_env(path):
    """Read literal KEY=value (optional matching quotes), never execute shell code."""
    result = {}
    for number, line in enumerate(path.read_text().splitlines(), 1):
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        if line.startswith('export '):
            line = line[7:]
        key, separator, value = line.partition('=')
        if not separator or not re.fullmatch(r'[A-Za-z_][A-Za-z0-9_]*', key):
            raise ValueError(f'Invalid environment assignment on line {number}; values omitted')
        if value.startswith(('"', "'")):
            if len(value) < 2 or value[-1] != value[0]:
                raise ValueError(f'Unmatched environment quote on line {number}')
            value = value[1:-1]
        result[key] = value
    return result


def run(args, cwd=ROOT, **kwargs):
    subprocess.run([str(arg) for arg in args], cwd=cwd, check=True, **kwargs)


def check_python(python):
    run([python, '-c', 'import sys; sys.exit(0 if sys.version_info[:2] == (3, 12) else "Embedding lock requires Python 3.12; set PYTHON=python3.12")'])


def check_tools():
    for command in ('cargo', 'rustc', 'node', 'npm', 'docker', 'bash'):
        if not shutil.which(command):
            raise ValueError(f'Missing {command}; install prerequisites yourself (see README)')
    check_python(sys.executable)
    run(['node', '-e', 'if(Number(process.versions.node.split(".")[0]) < 22) { console.error("Node 22+ required"); process.exit(1) }'])
    run(['bash', 'scripts/infra.sh', '--check-tools'])
    print('Required tools available; no daemon or model started.')


def setup():
    python = ROOT / 'embeddings/.venv/bin/python'
    if os.environ.get('SETUP_VERIFY') == '1':
        if not python.is_file():
            raise ValueError('Missing embeddings/.venv; run make setup')
        check_python(python)
        run(['npm', 'ls', '--depth=0'], cwd=ROOT / 'frontend')
        # Query package metadata only: do not import fastembed or load its model.
        code = ('import importlib.metadata as m, pathlib; '
                'pins=[line.split("==") for line in pathlib.Path("requirements.txt").read_text().splitlines() if line.strip()]; '
                'assert all(m.version(name)==version for name,version in pins), "Embedding pins differ; run make setup"')
        run([python, '-c', code], cwd=ROOT / 'embeddings')
        run([python, '-m', 'pip', 'check'])
        print('Installed local dependencies verified without reinstalling or inference.')
        return
    # Sequential dependency installation, even when make runs with -j.
    run(['npm', 'ci'], cwd=ROOT / 'frontend')
    if not python.exists():
        run([sys.executable, '-m', 'venv', ROOT / 'embeddings/.venv'])
    check_python(python)
    run([python, '-m', 'pip', 'install', '-r', 'requirements.txt'], cwd=ROOT / 'embeddings')
    run([python, '-m', 'pip', 'check'])


def backend(command):
    env = load_env(ROOT / os.environ.get('ENV_FILE', 'backend/dev.env'))
    env.update(os.environ)  # Explicit exported overrides win over file defaults.
    env['OPENSEARCH_INDEX'] = env.get('CATALOG_INDEX', 'manassas-catalog')
    args = ['cargo', 'run', '--locked', '--', {'api': 'serve', 'data-import': 'import'}.get(command, command)]
    if command == 'data-import':
        catalog = ROOT / os.environ.get('CATALOG_FILE', 'catalog/records.json')
        if not catalog.is_file():
            raise ValueError('Missing catalog snapshot; run make data-fetch or set CATALOG_FILE')
        args.append(str(catalog))
    os.chdir(ROOT / 'backend')
    os.execvpe(args[0], args, env)


def main():
    command = sys.argv[1]
    if command == 'check-tools':
        check_tools()
    elif command == 'setup':
        setup()
    elif command in ('migrate', 'api', 'data-import', 'index'):
        backend(command)
    else:
        raise ValueError('Unknown developer command')


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
    except subprocess.CalledProcessError as error:
        sys.exit(error.returncode)
