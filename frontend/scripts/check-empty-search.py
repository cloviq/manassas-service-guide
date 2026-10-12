"""Browser regression for empty-result guidance, using mocked /api/services responses."""
import json
import os
from pathlib import Path
from urllib.parse import parse_qs, urlparse
from playwright.sync_api import sync_playwright, expect

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'evidence'
OUT.mkdir(exist_ok=True)
URL = os.environ.get('FRONTEND_URL', 'http://127.0.0.1:5174')
ROUTE = dict(id='snow-route', name='Snow plow route', description='Mock route for browser regression.',
             category='Streets & sidewalks', record_type='route', provider_name='Test publisher',
             jurisdiction='Test jurisdiction', official_url='https://example.org/snow', source_url='https://example.org/source')
reports = []
with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    for width in (320, 390, 1280):
        page = browser.new_page(viewport={'width': width, 'height': 844})
        requests = []
        errors = []
        page.on('pageerror', lambda error: errors.append(str(error)))
        def search_response(route):
            params = parse_qs(urlparse(route.request.url).query, keep_blank_values=True)
            requests.append(params)
            items = [ROUTE] if params.get('type') == ['all'] else []
            route.fulfill(json={'items': items, 'total': len(items), 'offset': 0, 'search_mode': 'keyword'})
        page.route('**/api/services?*', search_response)
        page.goto(URL, wait_until='networkidle')
        page.get_by_role('searchbox').fill('snow')
        page.get_by_role('button', name='Streets & sidewalks', exact=True).click()
        expect(page.get_by_text('No matching services for “snow”.', exact=True)).to_be_visible()
        expect(page.get_by_role('combobox', name='Resource type')).to_have_value('service')
        assert len(requests) == 1 and requests[0]['type'] == ['service']
        assert page.locator('article').count() == 0
        action = page.get_by_role('button', name='Search all resources', exact=True)
        box = action.bounding_box()
        assert box and box['width'] >= 44 and box['height'] >= 44
        assert page.evaluate('document.documentElement.scrollWidth <= innerWidth')
        page.screenshot(path=str(OUT / f'empty-search-{width}.png'), full_page=True)
        action.focus()
        page.keyboard.press('Enter')
        expect(page.get_by_role('heading', name='Snow plow route', exact=True)).to_be_visible()
        expect(page.get_by_role('searchbox')).to_have_value('snow')
        expect(page.get_by_role('combobox', name='Resource type')).to_have_value('all')
        expect(page.get_by_role('button', name='Streets & sidewalks', exact=True)).to_have_attribute('aria-pressed', 'true')
        assert requests[-1] == {'q': ['snow'], 'category': ['Streets & sidewalks'], 'type': ['all'], 'limit': ['20'], 'offset': ['0']}
        expect(page.locator('article .record-type')).to_have_text('route')
        expect(page.get_by_role('link', name='View official source', exact=True)).to_have_attribute('href', ROUTE['official_url'])
        expect(page.get_by_role('button', name='Show on map', exact=True)).to_be_visible()
        assert page.evaluate('document.documentElement.scrollWidth <= innerWidth')
        assert not errors, errors
        page.screenshot(path=str(OUT / f'empty-search-all-{width}.png'), full_page=True)
        reports.append({'width': width, 'mocked_api': True, 'requests': requests, 'keyboard_action': 'passed', 'horizontal_overflow': False})
        page.close()
    page = browser.new_page()
    page.goto(URL, wait_until='networkidle')
    page.get_by_role('searchbox').fill('snow')
    page.get_by_role('button', name='Search', exact=True).click()
    expect(page.get_by_role('status')).not_to_have_text('Searching…', timeout=30000)
    reports.append({'live_api': True, 'status': page.get_by_role('status').inner_text(),
                    'cards': page.locator('article h2').all_text_contents(),
                    'empty_state_available': page.get_by_role('button', name='Search all resources', exact=True).count() == 1})
    browser.close()
(OUT / 'empty-search-browser.json').write_text(json.dumps(reports, indent=2))
print(json.dumps(reports, indent=2))
print('PASS: mocked browser regression at 320, 390 and 1280px.')
