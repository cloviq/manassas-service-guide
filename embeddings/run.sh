#!/bin/sh
set -eu
cd "$(dirname "$0")"
if [ ! -x .venv/bin/python ]; then
  printf '%s\n' 'Missing venv: python3.12 -m venv .venv && .venv/bin/python -m pip install -r requirements.txt' >&2
  exit 1
fi
exec .venv/bin/python service.py
