"""Download verified public map layers separately; never modify records.json.
Run from any directory: python3 scripts/catalog_map_expand.py [--check]
Layers, names, text, exclusions and allowlists come from catalog/sources.json
(`dataset: expansion`). Raw snapshots are audit-only.
"""
import concurrent.futures, datetime, hashlib, json, math, pathlib, re, sys, urllib.parse, urllib.request, uuid
ROOT=pathlib.Path(__file__).resolve().parents[1]; CAT=ROOT/'catalog'
sys.path.insert(0,str(ROOT/'scripts'))
from catalog_sources import PUBLISHER, SOURCES, dataset, fingerprint, layer_url, service_url, sha256, stored_fields
SPECS=[s['key'] for s in dataset('expansion')]
# Bump when record-building code changes; registry edits are detected by fingerprint.
COLLECTOR_VERSION=1
FINGERPRINT=fingerprint('expansion',COLLECTOR_VERSION)
DEFAULT_INSTRUCTIONS='Consult the exact official source record; confirm current access with the provider.'
class Fields(dict):
    def __missing__(self,key): return ''
def dump(p,d): p.parent.mkdir(parents=True,exist_ok=True); p.write_text(json.dumps(d,ensure_ascii=False,indent=2))
def get(url):
    with urllib.request.urlopen(url,timeout=45) as r: d=json.load(r)
    if 'error' in d: raise RuntimeError((url,d['error']))
    return d

def tokens(text,stop): return {t for t in re.findall(r'[a-z0-9]+',str(text or '').lower()) if t not in stop and len(t)>1}
def meters(a,b): return math.hypot((a[0]-b[0])*math.cos(math.radians((a[1]+b[1])/2))*111320,(a[1]-b[1])*110540)
def reference_places(rule):
    """Point records from the base catalog that a layer must not duplicate."""
    stop=set(rule['stopwords']); records=json.loads((CAT/'records.json').read_text())
    return [((r['geometry']['x'],r['geometry']['y']),tokens(r['name'],stop),r['name']) for r in records
            if r['source_key'].startswith(rule['source_prefix']) and 'x' in (r.get('geometry') or {})]
def exclusion(spec,a,geometry,places):
    """Reason a feature is excluded, or None. Excluded features stay in the raw snapshot."""
    rules=spec.get('exclude',{})
    for field,value in rules.get('require_values',{}).items():
        if a.get(field)!=value: return f'{field} is not {value}'
    if any(not a.get(field) for field in rules.get('require_fields',[])): return 'missing '+' or '.join(rules['require_fields']).lower()
    if rules.get('require_point') and 'x' not in (geometry or {}): return 'missing point geometry'
    duplicate=rules.get('duplicate_of')
    if duplicate:
        mine=tokens(a.get(spec['title_field']),set(duplicate['stopwords']))
        for point,theirs,name in places:
            d=meters((geometry['x'],geometry['y']),point); shared=len(mine&theirs)
            # Same place already imported from the reference layer (names differ by source).
            if any(d<=r['max_meters'] and shared>=r['min_shared_tokens'] for r in duplicate['rules']):
                return f'duplicate of reference place "{name}" ({round(d)} m)'
    return None

def collect(key):
    family,layer=key.split('/'); url=layer_url(key); spec=SOURCES[key]
    allowed,title=stored_fields(spec),spec['title_field']; out=CAT/'map_expansion_raw'/family/layer
    meta=get(url+'?f=json'); dump(out/'metadata.json',meta)
    type_names={str(t['id']):t['name'] for t in meta.get('types') or []}
    duplicate=spec.get('exclude',{}).get('duplicate_of'); places=reference_places(duplicate) if duplicate else []; excluded=[]
    oid=meta['objectIdField']; fields={f['name'] for f in meta['fields']}; assert set(allowed)<=fields,(key,set(allowed)-fields)
    def query(**kw): return url+'/query?'+urllib.parse.urlencode({'f':'json',**kw})
    before=get(query(where='1=1',returnCountOnly='true'))['count']
    ids=get(query(where='1=1',returnIdsOnly='true'))['objectIds'] or []
    records=[]; seen=[]; batches=[]
    for offset in range(0,len(ids),100):
        q=query(objectIds=','.join(map(str,ids[offset:offset+100])),outFields='*',returnGeometry='true',outSR=4326)
        data=get(q); assert not data.get('exceededTransferLimit'); path=out/f'batch_{offset:06}.json'; dump(path,data)
        batches.append({'path':str(path.relative_to(CAT)),'url':q,'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'count':len(data['features'])})
        for f in data['features']:
            a=f['attributes']; source_id=str(a[oid]); seen.append(a[oid])
            reason=exclusion(spec,a,f.get('geometry'),places)
            if reason: excluded.append({'source_record_id':source_id,'reason':reason}); continue
            public={k:a[k] for k in allowed if a.get(k) is not None}
            source_url=query(objectIds=source_id,outFields=','.join([oid]+allowed),returnGeometry='true',outSR=4326)
            values=Fields({k:str(v).strip() for k,v in a.items() if v is not None}); values['_id']=source_id
            if 'type_label' in spec:
                label=type_names.get(str(a.get(spec['type_label']['field'])))
                values['_type_label']=label if label and not label.isdigit() else spec['type_label']['fallback']
            desc=(spec['description_template'].format_map(values) if 'description_template' in spec else meta['name']+' — city-published mapped resource. Published attributes are a snapshot, not proof of access, eligibility or current conditions.')
            if 'name_template' in spec and values[title]: name=spec['name_template'].format_map(values)
            elif 'name_fallback' in spec and not values[title]: name=spec['name_fallback'].format_map(values)
            else: name=values[title] or meta['name']
            official=a.get(spec['official_url_field']) if 'official_url_field' in spec else None
            if not str(official or '').startswith('https://'): official=None
            instructions=a.get(spec['instructions_field']) if 'instructions_field' in spec else spec.get('instructions',DEFAULT_INSTRUCTIONS)
            records.append({'id':str(uuid.uuid5(uuid.NAMESPACE_URL,url+'#'+source_id)), 'name':name,'description':desc,'category':spec['category'],'record_type':spec['record_type'],'provider_name':PUBLISHER['provider_name'],'jurisdiction':PUBLISHER['map_jurisdiction'],'official_url':official or 'https://www.arcgis.com/home/item.html?id='+str(meta.get('serviceItemId','')),'source_url':source_url,'source_key':key,'source_record_id':source_id,'instructions':instructions,'attributes':public,'geometry':f.get('geometry'),'default_visible':spec.get('default_visible',False),'schedule_scope':spec.get('schedule_scope')})
    if 'disambiguate' in spec:
        # Repeated generic names ("Public Parking") are disambiguated by a published field.
        rule=spec['disambiguate']; names=[r['name'] for r in records]
        for r in records:
            if names.count(r['name'])>1: r['name']+=' — '+str(r['attributes'][rule['field']]).split(rule['strip_from'])[0].strip().rstrip('.,')
    after=get(query(where='1=1',returnCountOnly='true'))['count']; after_ids=get(query(where='1=1',returnIdsOnly='true'))['objectIds'] or []
    assert before==after==len(ids)==len(seen)==len(set(seen)); assert set(ids)==set(seen)==set(after_ids)
    # Use service-level item IDs if absent on layer metadata.
    service=get(service_url(family)+'?f=json'); item=service.get('serviceItemId'); dump(out/'service.json',service)
    for r in records:
        if not r['official_url'] or r['official_url'].endswith('id='): r['official_url']='https://www.arcgis.com/home/item.html?id='+item if item else url
    return records,{'source_key':key,'url':url,'metadata_url':url+'?f=json','object_id_field':oid,'excluded':excluded,'count_before':before,'count_after':after,'unique_ids':len(set(seen)),'exact_id_set_verified':True,'geometry_type':meta['geometryType'],'allowlisted_fields':allowed,'schedule_fields':spec.get('schedule_fields',[]),'batches':batches,'service_item_id':item}

def is_current():
    """True when the local snapshot was built from the current registry, collector and base catalog
    (duplicate exclusion compares against records.json)."""
    try: manifest=json.loads((CAT/'map_expansion_manifest.json').read_text())
    except (OSError,ValueError): return False
    return ((CAT/'map_expansion.json').is_file() and (CAT/'records.json').is_file()
            and manifest.get('registry_fingerprint')==FINGERPRINT
            and manifest.get('base_records_sha256')==sha256(CAT/'records.json'))
if __name__=='__main__' and sys.argv[1:]==['--check']: sys.exit(0 if is_current() else 1)
if __name__=='__main__':
    original=hashlib.sha256((CAT/'records.json').read_bytes()).hexdigest()
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool: result=list(pool.map(collect,SPECS))
    records=[r for rr,_ in result for r in rr]; assert len({r['id'] for r in records})==len(records)
    dump(CAT/'map_expansion.json',records)
    manifest={'registry_fingerprint':FINGERPRINT,'retrieved_at_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'records_count':len(records),'layers_count':len(result),'sources':[s for _,s in result],'geometry_format':'Esri JSON EPSG:4326, not GeoJSON','base_records_sha256':original,'map_expansion_sha256':hashlib.sha256((CAT/'map_expansion.json').read_bytes()).hexdigest(),'limitations':['Snapshot is not transactional across layers.','No regular trash collection polygons found.','No inference of yard-waste schedule from bulk or leaf polygons.','Only registry sources marked default_visible are shown by default; other resources are optional overlays.','Excluded features (with reasons) are listed per source; they remain in map_expansion_raw.','Raw attributes in map_expansion_raw are audit-only; do not index or serialize them.','Public access is not a redistribution license; review official item terms.']}
    assert hashlib.sha256((CAT/'records.json').read_bytes()).hexdigest()==original
    dump(CAT/'map_expansion_manifest.json',manifest)
    print(json.dumps({'count':len(records),'layers':[(s['source_key'],s['count_after']) for _,s in result],'excluded':sum(len(s['excluded']) for _,s in result),'base_unchanged':True},indent=2))
