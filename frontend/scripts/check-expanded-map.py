"""Real Chromium mobile smoke check of the map layer controls against the live API."""
import json, os
from pathlib import Path
from playwright.sync_api import sync_playwright, expect
OUT=Path(__file__).resolve().parents[1]/'evidence'
OUT.mkdir(exist_ok=True)
reports=[]
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True)
 for width in (320,390):
  page=browser.new_page(viewport={'width':width,'height':844})
  errors=[]; responses=[]
  page.on('pageerror',lambda e:errors.append(str(e)))
  page.on('response',lambda r:responses.append({'url':r.url,'status':r.status}) if '/api/map/features?' in r.url else None)
  page.goto(os.environ.get('FRONTEND_URL','http://127.0.0.1:5174'),wait_until='networkidle')
  page.get_by_role('button',name='Explore map',exact=True).click()
  expect(page.get_by_role('checkbox',name='Seasonal loose leaves',exact=True)).to_be_checked()
  expect(page.get_by_role('checkbox',name='Collection',exact=True)).to_have_attribute('aria-checked','mixed')
  expect(page.get_by_role('checkbox',name='Recreation',exact=True)).not_to_be_checked()
  page.wait_for_function("[...document.querySelectorAll('.map-view [role=status],.map-view [role=alert]')].some(e=>/mapped features|Could not load/.test(e.textContent))",timeout=15000)
  page.get_by_role('checkbox',name='Second Run Route',exact=True).uncheck()
  expect(page.get_by_role('checkbox',name='Snow routes',exact=True)).to_have_attribute('aria-checked','mixed')
  page.get_by_role('checkbox',name='Recreation',exact=True).check()
  for name in ('Parks','Walking trails','Bike racks','Bike routes'):
   expect(page.get_by_role('checkbox',name=name,exact=True)).to_be_checked()
  expect(page.get_by_role('checkbox',name='Second Run Route',exact=True)).not_to_be_checked()
  page.get_by_role('checkbox',name='Watersheds',exact=True).check()
  page.get_by_text('Collection guidance · leaves, yard waste & trash',exact=True).click()
  expect(page.get_by_role('link',name='Yard waste information',exact=True)).to_be_visible()
  expect(page.get_by_role('link',name='Trash schedule',exact=True)).to_be_visible()
  page.wait_for_timeout(1500)
  metrics=page.locator('.map-toggles').evaluate('''e=>({width:innerWidth,document:document.documentElement.scrollWidth,targets:[...e.querySelectorAll('label')].map(l=>({name:l.innerText,width:l.getBoundingClientRect().width,height:l.getBoundingClientRect().height}))})''')
  assert metrics['document']<=width,metrics
  assert all(t['width']>=44 and t['height']>=44 for t in metrics['targets']),metrics
  assert not errors,errors
  page.screenshot(path=str(OUT/f'expanded-map-{width}.png'),full_page=True)
  reports.append({'width':width,'metrics':metrics,'responses':responses,'page_errors':errors,'status':page.locator('.map-view').inner_text()})
  page.close()
 browser.close()
(OUT/'expanded-map-browser.json').write_text(json.dumps(reports,indent=2))
print(json.dumps(reports,indent=2))
print('PASS: real mobile browser controls, snow preservation, guidance, 44px labels, no horizontal overflow or page errors. API responses are saved in evidence/expanded-map-browser.json.')
