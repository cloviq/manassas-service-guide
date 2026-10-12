"""Summarize the downloaded catalog into catalog/expectations.json.

The ignored live tests compare the imported database and API against this file
instead of hardcoded counts, names or IDs, so they follow whatever the public
sources published at download time. Run after data-fetch (make data-fetch does).
"""
import json
from collections import Counter, defaultdict
from pathlib import Path
import sys
from catalog_sources import REGISTRY, SOURCES, sha256

CAT = Path(__file__).resolve().parents[1] / 'catalog'
INPUTS = ('records.json', 'map_expansion.json')
MIGRATIONS = Path(__file__).resolve().parents[1] / 'backend' / 'migrations'

def points(geometry):
    """Every (x, y) vertex of an Esri JSON point, polyline or polygon."""
    if not geometry:
        return
    if 'x' in geometry:
        yield geometry['x'], geometry['y']
    for part in geometry.get('paths', []) + geometry.get('rings', []):
        yield from ((x, y) for x, y, *_ in part)

def street_name(record):
    return ' '.join(record['name'].split()).upper()

def street_group(records):
    """Largest multi-segment street that a search for its name finds unambiguously.

    Search matches record name, description, category and provider, plus the seed
    tasks in the migrations, so the name must appear in none of those outside the
    group. Member names must already be in normalized form so SQL, Python and Rust
    normalization agree.
    """
    grouped = {k for k, s in SOURCES.items() if s.get('street_grouping')}
    seeds = ' '.join(p.read_text() for p in MIGRATIONS.glob('*.sql')).upper()
    groups = defaultdict(list)
    for r in records:
        if r['source_key'] in grouped and r['record_type'] == 'route' and street_name(r):
            groups[(r['source_key'], street_name(r))].append(r)
    # Prefer multi-word names: a single common word is more likely to match semantically.
    for (key, name), members in sorted(groups.items(), key=lambda g: (' ' not in g[0][1], -len(g[1]), g[0][1])):
        ids = {r['id'] for r in members}
        searchable = (f"{r['name']} {r['description']} {r['category']} {r['provider_name']}".upper() for r in records if r['id'] not in ids)
        if (len(members) > 1 and all(r['name'] == name for r in members)
                and name not in seeds and not any(name in text for text in searchable)):
            return {'source_key': key, 'name': name, 'segments': len(members), 'ids': sorted(ids)}
    return None

def main():
    files = [CAT / name for name in INPUTS if (CAT / name).is_file()]
    if not files:
        raise SystemExit('No catalog downloaded; run make data-fetch first.')
    by_file = {f.name: json.loads(f.read_text()) for f in files}
    records = [r for rs in by_file.values() for r in rs]
    mapped = {k for k, s in SOURCES.items() if 'map' in s}
    xy = [p for r in records if r['source_key'] in mapped for p in points(r.get('geometry'))]
    by_source = Counter(r['source_key'] for r in records)
    layers = Counter()
    for key, count in by_source.items():
        if key in mapped:
            layers[SOURCES[key]['map']['layer']] += count
    expectations = {
        'inputs': {f.name: sha256(f) for f in files},
        'records': {name: len(rs) for name, rs in by_file.items()},
        'by_source': dict(sorted(by_source.items())),
        'map': {
            # Envelope of every mapped vertex: a viewport that contains the whole catalog.
            'extent': [min(x for x, _ in xy), min(y for _, y in xy), max(x for x, _ in xy), max(y for _, y in xy)] if xy else None,
            'by_layer': dict(sorted(layers.items())),
            'api_default_layers': [l['id'] for l in REGISTRY['map_layers'] if l.get('api_default')],
        },
        'street_group': street_group(records),
    }
    (CAT / 'expectations.json').write_text(json.dumps(expectations, indent=2) + '\n')
    print(json.dumps({k: v for k, v in expectations.items() if k != 'street_group'} | {'street_group': (expectations['street_group'] or {}).get('name')}, indent=2))

def is_current():
    """True when expectations.json summarizes exactly the current catalog files."""
    try:
        inputs = json.loads((CAT / 'expectations.json').read_text())['inputs']
    except (OSError, ValueError, KeyError):
        return False
    files = {name for name in INPUTS if (CAT / name).is_file()}
    return set(inputs) == files and all(inputs[name] == sha256(CAT / name) for name in files)

if __name__ == '__main__':
    if sys.argv[1:] == ['--check']:
        sys.exit(0 if is_current() else 1)
    main()
