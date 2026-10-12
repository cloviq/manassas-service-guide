"""Read-only, resumable ArcGIS collection. Run from any directory with Python 3."""
import concurrent.futures
import datetime
import hashlib
import json
from pathlib import Path
import sys
import time
import urllib.parse
import urllib.request
from collections import Counter
from catalog_normalize import normalize
from catalog_sources import LINKS, PUBLISHER, SOURCES, dataset, fingerprint, layer_url

ROOT = Path(__file__).resolve().parents[1] / 'catalog'
BASE_KEYS = [s['key'] for s in dataset('base')]
FAMILIES = {key.split('/')[0] for key in BASE_KEYS}
# Bump when record-building code changes; registry edits are detected by fingerprint.
COLLECTOR_VERSION = 1
FINGERPRINT = fingerprint('base', COLLECTOR_VERSION)

def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix('.tmp')
    tmp.write_text(json.dumps(value, ensure_ascii=False, separators=(',', ':')))
    tmp.replace(path)

def get(url, params):
    url = url + '?' + urllib.parse.urlencode({'f':'json', **params})
    for attempt in range(4):
        try:
            req = urllib.request.Request(url, headers={'User-Agent':PUBLISHER['user_agent']})
            with urllib.request.urlopen(req, timeout=45) as r:
                data = json.load(r)
            if 'error' in data:
                raise RuntimeError(str(data['error']))
            return data, url
        except Exception:
            if attempt == 3:
                raise
            time.sleep(2 ** attempt)

def collect(key):
    family, layer = key.split('/')
    url = layer_url(key)
    directory = ROOT / 'raw' / family / str(layer)
    count, count_url = get(url + '/query', {'where':'1=1', 'returnCountOnly':'true'})
    save(directory / 'count_before.json', count)
    metadata, metadata_url = get(url, {})
    save(directory / 'metadata.json', metadata)
    ids, ids_url = get(url + '/query', {'where':'1=1', 'returnIdsOnly':'true'})
    save(directory / 'ids.json', ids)
    oid = metadata.get('objectIdField') or ids['objectIdFieldName']
    expected_ids = sorted(ids.get('objectIds') or [])
    assert len(set(expected_ids)) == count['count'], (key, 'IDs/count mismatch')
    source = {'source_key':key, 'url':url, 'name':metadata['name'], 'oid_field':oid, 'count_before':count['count'], 'count_query_url':count_url, 'ids_query_url':ids_url, 'metadata_url':metadata_url, 'batches':[]}
    features = []
    batch_size = min(100, metadata.get('maxRecordCount', 1000))
    for offset in range(0, len(expected_ids), batch_size):
        chunk = expected_ids[offset:offset + batch_size]
        path = directory / f'batch_{offset:06d}.json'
        params = {'objectIds':','.join(map(str, chunk)), 'outFields':'*', 'returnGeometry':'true', 'outSR':4326, 'orderByFields':oid + ' ASC'}
        query_url = url + '/query?' + urllib.parse.urlencode({'f':'json', **params})
        data = json.loads(path.read_text()) if path.exists() else None
        if data is None or sorted(f['attributes'][oid] for f in data.get('features', [])) != chunk:
            data, query_url = get(url + '/query', params)
            save(path, data)
        got = data.get('features', [])
        assert sorted(f['attributes'][oid] for f in got) == chunk, (key, offset, 'batch incomplete')
        sr = data.get('spatialReference', {})
        assert sr.get('latestWkid', sr.get('wkid')) == 4326, (key, sr)
        features.extend(got)
        source['batches'].append({'path':str(path.relative_to(ROOT)), 'count':len(got), 'query_url':query_url, 'sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
        save(directory / 'progress.json', {'downloaded':len(features), 'expected':count['count'], 'source':source})
    after, _ = get(url + '/query', {'where':'1=1', 'returnCountOnly':'true'})
    save(directory / 'count_after.json', after)
    actual_ids = [f['attributes'][oid] for f in features]
    assert len(actual_ids) == len(set(actual_ids)) == count['count'] == after['count'], key
    assert set(actual_ids) == set(expected_ids), key
    source.update({'count_after':after['count'], 'downloaded_unique_ids':len(set(actual_ids)), 'verified':True, 'geometry_type':metadata.get('geometryType'), 'copyright_text':metadata.get('copyrightText'), 'editing_info':metadata.get('editingInfo')})
    save(directory / 'verified.json', source)
    print(f'VERIFIED {key}: {len(features)}', flush=True)
    return source, features

def main():
    ROOT.mkdir(parents=True, exist_ok=True)
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
        results = list(pool.map(collect, BASE_KEYS))
    records, exclusions, sources = [], [], []
    for source, features in results:
        seen = {}
        for feature in features:
            if SOURCES[source['source_key']].get('dedupe_attributes'):
                signature = json.dumps({k:v for k,v in feature['attributes'].items() if k != source['oid_field']}, sort_keys=True)
                if signature in seen:
                    kept = seen[signature]
                    exclusions.append({'source_key':source['source_key'], 'excluded_source_record_id':str(feature['attributes'][source['oid_field']]), 'retained_source_record_id':str(kept['attributes'][source['oid_field']]), 'reason':'Exact source attributes duplicate excluding OBJECTID; geometry may differ and both geometries remain in raw snapshots.', 'attributes_equal_excluding_objectid':True, 'geometry_equal':kept.get('geometry') == feature.get('geometry')})
                    continue
                seen[signature] = feature
            records.append(normalize(feature, source))
        source['normalized_count'] = sum(r['source_key'] == source['source_key'] for r in records)
        sources.append(source)
    assert len(sources) == len(BASE_KEYS)
    assert len({r['id'] for r in records}) == len(records)
    save(ROOT / 'records.json', records)
    save(ROOT / 'exclusions.json', exclusions)
    registry = {'publisher':PUBLISHER['name'], 'geohub_url':PUBLISHER['hub_url'], 'faq_url':PUBLISHER['faq_url'], 'links':LINKS, 'terms':PUBLISHER['terms'], 'sources':sources}
    save(ROOT / 'source_registry.json', registry)
    manifest = {'registry_fingerprint':FINGERPRINT, 'retrieved_at_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(), 'families':len(FAMILIES), 'layers':len(sources), 'raw_count':sum(s['downloaded_unique_ids'] for s in sources), 'normalized_count':len(records), 'excluded_count':len(exclusions), 'by_record_type':dict(Counter(r['record_type'] for r in records)), 'by_category':dict(Counter(r['category'] for r in records)), 'geometry_format':'Esri JSON, EPSG:4326; not GeoJSON', 'records_sha256':hashlib.sha256((ROOT / 'records.json').read_bytes()).hexdigest(), 'sources':sources, 'limitations':['Counts are source features, not unique residents, services, assets across overlapping layers, or streets.', 'Historical breaks are not live outages.', 'All attributes are preserved for audit; resident UI and embedding text must not expose internal staff/editor metadata.', 'Government duplicate removal compares all attributes except OBJECTID; geometry differences are explicitly recorded.', 'No geometry-derived routing, provider or jurisdiction determination.', 'Resumed batches can span retrieval times; counts and exact source ID sets verified, but no transactional snapshot or edit-time consistency guarantee.']}
    save(ROOT / 'manifest.json', manifest)
    print(json.dumps({k:v for k,v in manifest.items() if k != 'sources'}, indent=2))

def is_current():
    """True when records.json was built from the current registry and collector."""
    try:
        manifest = json.loads((ROOT / 'manifest.json').read_text())
    except (OSError, ValueError):
        return False
    return (ROOT / 'records.json').is_file() and manifest.get('registry_fingerprint') == FINGERPRINT

if __name__ == '__main__':
    if sys.argv[1:] == ['--check']:
        sys.exit(0 if is_current() else 1)
    main()
