"""Read-only public-source discovery; writes only separate catalog evidence."""
import concurrent.futures, json, pathlib, urllib.request, urllib.parse, re
ROOT=pathlib.Path(__file__).resolve().parents[1]
OUT=ROOT/'catalog/map_expansion_evidence'; OUT.mkdir(exist_ok=True)
import sys; sys.path.insert(0,str(ROOT/'scripts'))
from catalog_sources import PUBLISHER, REGISTRY, service_url
DISCOVERY=REGISTRY['discovery']
def fetch(pair):
    name,url=pair
    try:
        text=urllib.request.urlopen(url,timeout=35).read().decode()
        (OUT/(name+'.txt')).write_text(text)
        try: data=json.loads(text)
        except ValueError: data={'html':text}
        return name, {'url':url,'data':data}
    except Exception as e: return name,{'url':url,'error':str(e)}
if __name__=='__main__':
    jobs=[(s,service_url(s)+'?f=json') for s in DISCOVERY['services']]
    jobs += [('page_'+name,url) for name,url in DISCOVERY['pages'].items()]
    jobs += [('org_waste_search','https://www.arcgis.com/sharing/rest/search?'+urllib.parse.urlencode({'f':'json','q':'orgid:'+PUBLISHER['arcgis_org_id']+' ('+DISCOVERY['org_search_terms']+')','num':100}))]
    with concurrent.futures.ThreadPoolExecutor(max_workers=10) as pool: results=dict(pool.map(fetch,jobs))
    (OUT/'discovery.json').write_text(json.dumps(results,indent=2))
    for name,r in results.items():
        d=r.get('data',{})
        if 'html' in d:
            from html import unescape
            t=unescape(re.sub('<[^>]+>',' ',d['html'])); t=re.sub(r'\s+',' ',t)
            (OUT/(name+'_visible.txt')).write_text(t)
            print(name, '\n'.join(t[max(0,m.start()-150):m.end()+550] for m in re.finditer(r'Mondays|Wednesday|Waste Wise|recollect|Tuesday|collection schedule|iframe',t,re.I)))
            print('links', re.findall(r'(?:href|src)=[\"\']([^\"\']+)',d['html'])[-25:])
        else: print(name,json.dumps({k:d.get(k) for k in ['layers','tables','results','error']})[:12000])
