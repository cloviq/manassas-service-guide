"""Normalize public GIS records; raw attributes are audit-only, not UI copy."""
import uuid
from urllib.parse import urlencode
from catalog_sources import LINKS, PUBLISHER, SOURCES

def normalize(feature, source):
    a = feature['attributes']
    key = source['source_key']
    oid = str(a[source['oid_field']])
    spec = SOURCES.get(key, {})
    category, kind = spec.get('category', 'Other services'), spec.get('record_type', 'place')
    # Explicit allowlist only: no staff, editor, work-order or arbitrary JSON text.
    lowered = {k.lower(): v for k, v in a.items()}
    candidates = ('name', 'fullname', 'full_name', 'streetname', 'street_name', 'st_name', 'roadname', 'route_name')
    title = next((str(lowered[k]).strip() for k in candidates if lowered.get(k) and str(lowered[k]).strip()), None)
    name = title or f"{source['name']} — source record {oid}"
    if 'description_fields' in spec:
        parts = [str(a[k]).strip() for k in spec['description_fields'] if a.get(k)]
        description = '; '.join(parts) or spec['description_fallback']
    elif 'description' in spec:
        description = spec['description']
    else:
        description = f"{source['name']}: city-published {'infrastructure asset' if kind == 'infrastructure' else 'mapped route or collection area'} record. Geometry does not determine your service provider, eligibility, schedule, or request destination."
    official = a.get(spec['official_url_field']) if 'official_url_field' in spec else None
    if not isinstance(official, str) or not official.startswith(('https://', 'http://')):
        official = LINKS[spec['official_url_link']] if 'official_url_link' in spec else PUBLISHER['hub_url']
    return {
        'id': str(uuid.uuid5(uuid.NAMESPACE_URL, source['url'] + '#' + oid)),
        'name': name, 'description': description, 'category': category,
        'provider_name': PUBLISHER['provider_name'], 'jurisdiction': PUBLISHER['jurisdiction'],
        'official_url': official, 'source_url': source['url'] + '/query?' + urlencode({'f':'json','objectIds':oid,'outFields':'*','returnGeometry':'true','outSR':4326}),
        'instructions': 'Consult the linked official source for current guidance. This catalog is a dated GIS snapshot, not a service-routing or live-status determination.',
        'record_type': kind, 'source_key': key, 'source_record_id': oid,
        'attributes': a, 'geometry': feature.get('geometry'),
    }
