"""Offline acceptance checks against every persisted source batch."""
import hashlib
import json
from pathlib import Path
import uuid
from catalog_normalize import normalize
from catalog_sources import REGISTRY, dataset

ROOT = Path(__file__).resolve().parents[1] / 'catalog'

def main():
    records = json.loads((ROOT / 'records.json').read_text())
    manifest = json.loads((ROOT / 'manifest.json').read_text())
    excluded = json.loads((ROOT / 'exclusions.json').read_text())
    indexed = {(r['source_key'], r['source_record_id']):r for r in records}
    assert len(indexed) == len(records) == manifest['normalized_count']
    assert len({r['id'] for r in records}) == len(records)
    assert manifest['records_sha256'] == hashlib.sha256((ROOT / 'records.json').read_bytes()).hexdigest()
    base = {s['key'] for s in dataset('base')}
    assert {s['source_key'] for s in manifest['sources']} == base
    exclusions = {(e['source_key'], e['excluded_source_record_id']):e for e in excluded}
    total = 0
    fields = {'id','name','description','category','provider_name','jurisdiction','official_url','source_url','instructions','record_type','source_key','source_record_id','attributes','geometry'}
    for source in manifest['sources']:
        features = {}
        for batch in source['batches']:
            path = ROOT / batch['path']
            assert hashlib.sha256(path.read_bytes()).hexdigest() == batch['sha256']
            data = json.loads(path.read_text())
            assert data['spatialReference'].get('latestWkid', data['spatialReference'].get('wkid')) == 4326
            for f in data['features']:
                oid = str(f['attributes'][source['oid_field']])
                assert oid not in features
                features[oid] = f
        expected = json.loads((ROOT / 'raw' / source['source_key'] / 'ids.json').read_text())['objectIds']
        assert set(features) == set(map(str, expected))
        assert len(features) == source['count_before'] == source['count_after'] == source['downloaded_unique_ids']
        total += len(features)
        for oid, f in features.items():
            key = (source['source_key'], oid)
            if key in exclusions:
                e = exclusions[key]
                other = features[e['retained_source_record_id']]
                assert {k:v for k,v in f['attributes'].items() if k != source['oid_field']} == {k:v for k,v in other['attributes'].items() if k != source['oid_field']}
                assert key not in indexed
                continue
            row = indexed[key]
            assert set(row) == fields
            assert row == normalize(f, source)
            assert uuid.UUID(row['id']).version == 5
            assert row['category'] in {s['category'] for s in REGISTRY['sources']}
            assert row['geometry'] is None or isinstance(row['geometry'], dict)
    assert total == manifest['raw_count'] == len(records) + len(excluded)
    print(json.dumps({'verified_layers':len(manifest['sources']), 'verified_families':len({s['source_key'].split('/')[0] for s in manifest['sources']}), 'raw_records':total, 'normalized_records':len(records), 'exclusions':len(excluded), 'records_bytes':(ROOT/'records.json').stat().st_size, 'all_source_ids_geometries_attributes_hashes_and_UUIDs_verified':True}, indent=2))

if __name__ == '__main__':
    main()
