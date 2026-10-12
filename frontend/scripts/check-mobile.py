"""Real-browser mobile check; no API mocks or synthetic service records."""
import json
import os
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'evidence'
OUT.mkdir(exist_ok=True)
reports = []
interface_errors = []
with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    for width in (320, 390):
        page = browser.new_page(viewport={'width': width, 'height': 844}, device_scale_factor=1)
        page.on('pageerror', lambda error: interface_errors.append(str(error)))
        page.on('console', lambda message: interface_errors.append(message.text) if message.type == 'error' else None)
        page.goto(os.environ.get('FRONTEND_URL', 'http://127.0.0.1:5174'), wait_until='networkidle')
        for state in ('landing', 'search', 'intake'):
            if state == 'search':
                page.get_by_role('button', name='Search', exact=True).click()
                page.wait_for_function("!document.querySelector('[role=status]').textContent.includes('Searching')")
            if state == 'intake':
                page.get_by_text('Try demo intake', exact=True).click()
            metrics = page.evaluate('''() => ({
              viewport: innerWidth, document: document.documentElement.scrollWidth,
              controls: [...document.querySelectorAll('button, input, textarea, select, summary, a')]
                .filter(e => e.getClientRects().length && e.checkVisibility())
                .map(e => ({name: e.textContent || e.id, width: e.getBoundingClientRect().width, height: e.getBoundingClientRect().height})),
              body: document.body.innerText
            })''')
            page.screenshot(path=str(OUT / f'mobile-{width}-{state}.png'), full_page=True)
            reports.append({'width': width, 'state': state, **metrics})
        page.close()
    browser.close()
(OUT / 'mobile-metrics.json').write_text(json.dumps(reports, indent=2))
assert not interface_errors, f'Browser errors: {interface_errors}'
for report in reports:
    print(json.dumps(report))
    assert report['document'] <= report['viewport'], f"Horizontal overflow at {report['width']} / {report['state']}"
    for control in report['controls']:
        assert control['height'] >= 44 and control['width'] >= 44, f"Small target at {report['width']}: {control}"
print('PASS: 320px and 390px; landing/search/intake; no horizontal overflow; all visible controls at least 44×44px.')
