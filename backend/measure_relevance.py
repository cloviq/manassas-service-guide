"""Read-only small-query calibration; export backend/dev.env (OPENSEARCH_*, EMBEDDING_*) before running."""
import json, os, urllib.request
from pathlib import Path

def post(url, body):
    req = urllib.request.Request(url, json.dumps(body).encode(), {'Content-Type': 'application/json'})
    return json.load(urllib.request.urlopen(req, timeout=60))

queries = ['snow', 'qzxvplmokn', 'quantum banana spaceship', 'buy concert tickets', 'passport renewal', 'pizza delivery', 'my garbage bin is broken', 'replace damaged container', 'when does the truck come', 'dispose of an old sofa', 'my trash was not picked up', 'recycling pickup was missed', 'yard waste was not collected', 'what day is garbage collection']
model = os.environ['EMBEDDING_MODEL']
vectors = post(os.environ['EMBEDDING_URL'], {'model': model, 'input': queries})['data']
rows = []
for q, vector in zip(queries, sorted(vectors, key=lambda d:d['index'])):
    for kind in ['service', 'all']:
        filters = [] if kind == 'all' else [{'term': {'record_type':kind}}]
        result = post(os.environ['OPENSEARCH_URL']+'/'+os.environ.get('OPENSEARCH_INDEX','manassas-catalog')+'/_search', {'size': 6, '_source':['name','record_type','embedding'], 'query':{'script_score':{'query':{'bool':{'filter':filters}},'script':{'lang':'knn','source':'knn_score','params':{'field':'embedding','query_value':vector['embedding'],'space_type':'cosinesimil'}}}}})
        import math
        for h in result['hits']['hits']:
            a, b = vector['embedding'], h['_source']['embedding']
            cosine = sum(x*y for x,y in zip(a,b))/math.sqrt(sum(x*x for x in a)*sum(y*y for y in b))
            assert abs(cosine - (2*h['_score']-1)) < 0.00001, (cosine, h['_score'])
        hits = [{'name':h['_source']['name'], 'type':h['_source']['record_type'], 'script_score':h['_score'], 'cosine':2*h['_score']-1} for h in result['hits']['hits']]
        rows.append({'query':q,'type':kind,'hits':hits})
        print(q, kind, [(h['name'],round(h['cosine'],4)) for h in hits[:2]])
Path('semantic-calibration.json').write_text(json.dumps({'model':model,'score_transform':'OpenSearch 2.19.4 Lucene knn_score cosinesimil = (1 + cosine) / 2; verified against direct dot products','queries':rows},indent=2)+'\n')
