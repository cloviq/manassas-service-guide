"""Offline integrity checks plus bounded live record-link readback."""
import concurrent.futures, hashlib, json, pathlib, sys, urllib.parse, urllib.request
ROOT=pathlib.Path(__file__).resolve().parents[1]; CAT=ROOT/'catalog'
sys.path.insert(0,str(ROOT/'scripts'))
from catalog_sources import SOURCES, stored_fields

def main():
    records=json.loads((CAT/'map_expansion.json').read_text()); manifest=json.loads((CAT/'map_expansion_manifest.json').read_text())
    assert len(records)==manifest['records_count']==len({r['id'] for r in records})
    assert hashlib.sha256((CAT/'records.json').read_bytes()).hexdigest()==manifest['base_records_sha256']
    assert hashlib.sha256((CAT/'map_expansion.json').read_bytes()).hexdigest()==manifest['map_expansion_sha256']
    for source in manifest['sources']:
        rr=[r for r in records if r['source_key']==source['source_key']]
        excluded=source.get('excluded',[])
        assert len(rr)+len(excluded)==source['count_before']==source['count_after']==source['unique_ids']
        assert all(e['reason'] for e in excluded)
        assert not {e['source_record_id'] for e in excluded} & {r['source_record_id'] for r in rr}
        assert len({r['source_record_id'] for r in rr})==len(rr)
        for b in source['batches']: assert hashlib.sha256((CAT/b['path']).read_bytes()).hexdigest()==b['sha256']
    for r in records:
        spec=SOURCES[r['source_key']]
        assert set(r['attributes']) <= set(stored_fields(spec))
        assert r['geometry']
        p=urllib.parse.parse_qs(urllib.parse.urlsplit(r['source_url']).query)
        assert p['objectIds']==[r['source_record_id']] and '*' not in p['outFields'][0]
        assert r['official_url'].startswith('https://')
        assert r['default_visible']==spec.get('default_visible',False)
        for field in spec.get('schedule_fields',[]): assert r['attributes'].get(field)
    oid_field={s['source_key']:s.get('object_id_field') for s in manifest['sources']}
    # Each exact source link returns exactly its named record; every record, no sampling.
    def check(r):
        with urllib.request.urlopen(r['source_url'],timeout=40) as response: d=json.load(response)
        assert len(d['features'])==1
        a=d['features'][0]['attributes']; oid=oid_field.get(r['source_key']) or next(k for k in a if k.lower()=='objectid')
        assert str(a[oid])==r['source_record_id']
        return r['id']
    with concurrent.futures.ThreadPoolExecutor(max_workers=12) as pool: checked=list(pool.map(check,records))
    result={'records':len(records),'layers':len(manifest['sources']),'exact_record_links_read_back':len(checked),'allowlists_passed':True,'base_records_unchanged':True,'batch_hashes_passed':True}
    (CAT/'map_expansion_verification.json').write_text(json.dumps(result,indent=2)); print(json.dumps(result,indent=2))
if __name__=='__main__': main()
