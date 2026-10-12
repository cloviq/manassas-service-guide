import json
import unittest
import uuid
try:
    from catalog_normalize import normalize
except ImportError:
    normalize = None

class NormalizeTests(unittest.TestCase):
    def test_historical_asset_safe_and_stable(self):
        self.assertIsNotNone(normalize, 'normalizer must be implemented')
        f = {'attributes': {'OBJECTID': 7, 'Creator': 'INTERNAL_PERSON'}, 'geometry': {'x': -77, 'y': 38}}
        source = {'source_key': 'watermain_breaks_view/0', 'url': 'https://example.org/0', 'name': 'Water main breaks', 'oid_field': 'OBJECTID'}
        r = normalize(f, source)
        self.assertEqual(uuid.UUID(r['id']).version, 5)
        self.assertEqual(r['id'], normalize(f, source)['id'])
        self.assertEqual(r['record_type'], 'outage')
        self.assertEqual(r['category'], 'Water & sewer')
        self.assertIn('Historical', r['description'])
        self.assertIn('not current', r['description'])
        self.assertNotIn('INTERNAL_PERSON', r['description'])
        self.assertEqual(r['attributes'], f['attributes'])
        self.assertEqual(r['geometry'], f['geometry'])
        self.assertEqual(r['provider_name'], 'City of Manassas GIS — data publisher (not a routing determination)')

    def test_registry_drives_category_and_official_link(self):
        from catalog_sources import LINKS, SOURCES
        bulk = next(k for k, s in SOURCES.items() if s.get('official_url_link'))
        f = {'attributes': {'OBJECTID': 1}, 'geometry': None}
        r = normalize(f, {'source_key': bulk, 'url': 'https://example.org/1', 'name': 'Area', 'oid_field': 'OBJECTID'})
        self.assertEqual(r['category'], SOURCES[bulk]['category'])
        self.assertEqual(r['official_url'], LINKS[SOURCES[bulk]['official_url_link']])

class ExpansionTests(unittest.TestCase):
    """Offline: fake ArcGIS responses exercise registry-driven names, text and exclusions."""
    def run_collect(self, key, features, types=(), base=()):
        import tempfile, pathlib, urllib.parse
        from unittest.mock import patch
        import catalog_map_expand as expand
        tmp = pathlib.Path(tempfile.mkdtemp())
        (tmp / 'records.json').write_text(json.dumps(list(base)))
        spec = expand.SOURCES[key]
        names = set(expand.stored_fields(spec)) | {'OBJECTID', 'Status', 'TYPE'}
        meta = {'name': 'Layer', 'objectIdField': 'OBJECTID', 'geometryType': 'esriGeometryPoint',
                'fields': [{'name': n} for n in names], 'types': [{'id': i, 'name': n} for i, n in types], 'serviceItemId': 'abc'}
        def get(url):
            q = urllib.parse.parse_qs(urllib.parse.urlsplit(url).query)
            if q.get('returnCountOnly'): return {'count': len(features)}
            if q.get('returnIdsOnly'): return {'objectIds': [f['attributes']['OBJECTID'] for f in features]}
            if 'objectIds' in q: return {'features': features}
            return meta
        with patch.object(expand, 'CAT', tmp), patch.object(expand, 'get', get):
            return expand.collect(key)

    def test_parking_keeps_only_approved_districts(self):
        records, source = self.run_collect('ParkingDistricts2/0', [
            {'attributes': {'OBJECTID': 1, 'District_Desc': 'Zone A', 'Type': 'Permit', 'Hours': '8-5.', 'Status': 'Approved', 'Info': 'https://example.org/p'}, 'geometry': {'rings': []}},
            {'attributes': {'OBJECTID': 2, 'District_Desc': 'Zone B', 'Status': 'Proposed'}, 'geometry': {'rings': []}},
            {'attributes': {'OBJECTID': 3, 'District_Desc': ' ', 'Status': 'Approved', 'Info': 'http://insecure'}, 'geometry': {'rings': []}}])
        self.assertEqual([r['name'] for r in records], ['Parking district — Zone A', 'Parking district 3'])
        self.assertIn('Posted restriction: 8-5.', records[0]['description'])
        self.assertEqual(records[0]['official_url'], 'https://example.org/p')
        self.assertTrue(records[1]['official_url'].endswith('id=abc'))
        self.assertEqual(source['excluded'], [{'source_record_id': '2', 'reason': 'Status is not Approved'}])

    def test_points_of_interest_dedupe_label_and_disambiguate(self):
        base = [{'source_key': 'Manassas_Government_Services_WFL1/0', 'name': 'City Hall', 'geometry': {'x': -77.47, 'y': 38.75}}]
        point = lambda oid, name, x, address, t=1: {'attributes': {'OBJECTID': oid, 'NAME': name, 'ADDRESS': address, 'TYPE': t}, 'geometry': {'x': x, 'y': 38.75}}
        records, source = self.run_collect('Points_of_Interest_2023/0', [
            point(1, 'Manassas City Hall', -77.4701, '9027 Center St'),
            point(2, 'Public Parking', -77.40, '1 Main St Manassas VA', 2),
            point(3, 'Public Parking', -77.41, '2 Main St.', 2),
            point(4, 'Museum', -77.42, '')], types=[(1, 'Government'), (2, '2')], base=base)
        self.assertEqual([r['name'] for r in records], ['Public Parking — 1 Main St', 'Public Parking — 2 Main St'])
        self.assertTrue(records[0]['description'].startswith('Point of interest at 1 Main St Manassas VA'))
        self.assertEqual([e['source_record_id'] for e in source['excluded']], ['1', '4'])
        self.assertIn('duplicate of reference place "City Hall"', source['excluded'][0]['reason'])

class RegistryTests(unittest.TestCase):
    def test_registry_is_internally_consistent(self):
        from catalog_sources import LINKS, REGISTRY, allowed_fields
        keys = [s['key'] for s in REGISTRY['sources']]
        self.assertEqual(len(keys), len(set(keys)))
        groups = {g['id'] for g in REGISTRY['map_groups']}
        layers = {l['id']: l for l in REGISTRY['map_layers']}
        self.assertTrue(all(l['group'] in groups for l in layers.values()))
        self.assertTrue(all(l['geometry'] in {'point', 'line', 'polygon'} for l in layers.values()))
        mapped = {l: [] for l in layers}
        for s in REGISTRY['sources']:
            self.assertIn(s['dataset'], {'base', 'expansion'}, s['key'])
            self.assertRegex(s['key'], r'^[A-Za-z0-9_]+/\d+$')
            if s['dataset'] == 'expansion':
                self.assertIn('title_field', s, s['key'])
            if 'official_url_link' in s:
                self.assertIn(s['official_url_link'], LINKS)
            self.assertTrue(allowed_fields(s) or 'details' not in s, s['key'])
            if 'map' in s:
                mapped[s['map']['layer']].append(s)
        for layer_id, sources in mapped.items():
            self.assertTrue(sources, layer_id)
            if 'variant_param' in layers[layer_id]:
                self.assertTrue(all('variant' in s['map'] and 'color' in s['map'] for s in sources), layer_id)
            else:
                self.assertIn('color', layers[layer_id])

if __name__ == '__main__':
    unittest.main()
