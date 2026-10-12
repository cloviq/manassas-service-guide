"""Source registry shared by the catalog scripts, backend and frontend."""
import json
from pathlib import Path

PATH = Path(__file__).resolve().parents[1] / 'catalog' / 'sources.json'
REGISTRY = json.loads(PATH.read_text())
PUBLISHER = REGISTRY['publisher']
LINKS = REGISTRY['links']
SOURCES = {s['key']: s for s in REGISTRY['sources']}

def dataset(name):
    return [s for s in REGISTRY['sources'] if s['dataset'] == name]

def layer_url(key):
    family, layer = key.split('/')
    return PUBLISHER['arcgis_services_url'] + family + '/FeatureServer/' + layer

def service_url(family):
    return PUBLISHER['arcgis_services_url'] + family + '/FeatureServer'

def allowed_fields(source):
    """Every attribute the resident projection may show, in registry order."""
    details = source.get('details', {})
    fields = details.get('strings', []) + details.get('numbers', []) + details.get('nonnegative', [])
    return list(dict.fromkeys(fields))

def stored_fields(source):
    """Allowlisted fields plus attributes kept only for record building (links, names)."""
    return list(dict.fromkeys(allowed_fields(source) + source.get('extra_attributes', [])))

# Registry keys that only affect display (map styling, notices, API-time text), not
# collected records. Editing them must not trigger a re-download or reindex.
DISPLAY_ONLY = {'map', 'notice', 'caveats', 'resident_description', 'street_grouping', 'source_link_fields'}

def fingerprint(name, version):
    """Hash of everything in a dataset's registry entries that shapes its records."""
    import hashlib
    entries = [{k: v for k, v in s.items() if k not in DISPLAY_ONLY} for s in dataset(name)]
    shared = {k: PUBLISHER[k] for k in ('provider_name', 'jurisdiction', 'map_jurisdiction', 'hub_url', 'arcgis_services_url')}
    return hashlib.sha256(json.dumps([version, entries, shared, LINKS], sort_keys=True).encode()).hexdigest()

def sha256(path):
    import hashlib
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()
