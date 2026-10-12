import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi, test, expect } from 'vitest';
import App from './App';
// WebGL is unavailable in jsdom. Only this test file substitutes the renderer.
const renderer = vi.hoisted(() => ({handlers: {} as Record<string, Function>, fit: vi.fn(), data: vi.fn(), layer: vi.fn()}));
vi.mock('maplibre-gl', () => ({setWorkerUrl: vi.fn(), Map: class {
 on(event: string, a: unknown, b?: Function) {renderer.handlers[event] = b || a as Function; if(event === 'load') queueMicrotask(() => renderer.handlers.load()); return this;}
 off() {} addControl() {} addSource() {} addLayer(layer:unknown) {renderer.layer(layer);} getSource() {return {setData: renderer.data};}
 getBounds() {return {toArray: () => [[-77.5,38.7],[-77.4,38.8]]};} fitBounds = renderer.fit;
 remove() {} resize() {} }, NavigationControl: class {}, AttributionControl: class {}}));
test('collection and recreation groups default to leaves only and request exact tokens', async () => {
 const fetcher=vi.fn(async (_url:string)=>({ok:true,json:async()=>collection}));vi.stubGlobal('fetch',fetcher);
 render(<App/>);const user=userEvent.setup();await user.click(screen.getByRole('button',{name:'Explore map'}));
 expect(screen.getByRole('checkbox',{name:'Seasonal loose leaves'})).toBeChecked();
 expect(screen.getByRole('checkbox',{name:'Collection'})).toBePartiallyChecked();
 expect(screen.getByRole('checkbox',{name:'Recreation'})).not.toBeChecked();
 await user.click(screen.getByRole('checkbox',{name:'Recreation'}));
 await waitFor(()=>expect(decodeURIComponent(fetcher.mock.calls.at(-1)?.[0] || '')).toContain('layers=streets,snow,leaves,parks,trails,bike_racks,bike_routes'));
 expect(screen.getByRole('checkbox',{name:'Bulk waste area'})).not.toBeChecked();
 await user.click(screen.getByText('Collection guidance · leaves, yard waste & trash'));
 expect(screen.getByRole('link',{name:'Yard waste information'})).toHaveAttribute('href',expect.stringContaining('yard_waste.php'));
});
test('renderer supports polygon fills and outlines, points and lines', async()=>{
 vi.stubGlobal('fetch',vi.fn(async()=>({ok:true,json:async()=>collection})));render(<App/>);
 await userEvent.setup().click(screen.getByRole('button',{name:'Explore map'}));
 await waitFor(()=>expect(renderer.layer.mock.calls.map(c=>c[0].type)).toContain('fill'));
 expect(renderer.layer.mock.calls.map(c=>c[0].type)).toContain('circle');
 expect(renderer.layer.mock.calls.map(c=>c[0].type)).toContain('line');
});
test('map click decodes safe detail objects and distinguishes information from evidence', async()=>{
 vi.stubGlobal('fetch',vi.fn(async()=>({ok:true,json:async()=>collection})));render(<App/>);
 await userEvent.setup().click(screen.getByRole('button',{name:'Explore map'}));
 const {act}=await import('@testing-library/react');
 await act(async()=>renderer.handlers.click({features:[{properties:{name:'Section 1',layer:'leaves',source_key:'Leaf_Collection_Sections/0',details:JSON.stringify({DESCRIPT:'October 5–9',NOTES:'private'}),links:JSON.stringify({source_record:'https://example.org/record/1',service_information:'https://example.org/info',service_action:null})}}]}));
 expect(screen.getByText('Published collection windows')).toBeVisible();expect(screen.getByText('October 5–9')).toBeVisible();
 expect(screen.getByRole('link',{name:'Exact source record'})).toHaveAttribute('href','https://example.org/record/1');
 expect(screen.getByRole('link',{name:'Service information'})).toHaveAttribute('href','https://example.org/info');
 expect(screen.queryByText('private')).not.toBeInTheDocument();
});
test('listing details render source-specific fields with historical dates and separate links', async()=>{
 const item={id:'break-1',name:'Historical break',record_type:'outage',source_key:'watermain_breaks_view/0',details:{LOCDESC:'Main Street',INCIDENTDATE:1601510400000,NOTES:'private'},links:{source_record:'https://example.org/record/1',service_information:'https://example.org/info',service_action:null}};
 vi.stubGlobal('fetch',vi.fn(async(url:string)=>({ok:true,json:async()=>url.includes('/api/services/')?item:{items:[item],total:1,search_mode:'database'}})));
 render(<App/>);const user=userEvent.setup();await user.click(screen.getByRole('button',{name:'Search'}));
 await user.click(await screen.findByRole('button',{name:'View details'}));
 const detail=within(await screen.findByRole('region',{name:'Service details'}));
 expect(detail.getByText('Location')).toBeVisible();expect(detail.getByText('Main Street')).toBeVisible();
 expect(detail.getByText(/2020-10-01/)).toBeVisible();expect(detail.getByText(/not current outage or repair status/)).toBeVisible();
 expect(detail.getByRole('link',{name:'Exact source record'})).toHaveAttribute('href','https://example.org/record/1');
 expect(detail.getByRole('link',{name:'Service information'})).toHaveAttribute('href','https://example.org/info');
 expect(screen.queryByText('private')).not.toBeInTheDocument();
});
const collection = {type: 'FeatureCollection', features: [], truncated: false};
const snowNames = ['Tertiary Route', 'Second Run Route', 'First Run Route', 'Snow Emergency Route'];
test('snow parent and children support tri-state selection and native keyboard control', async () => {
 vi.stubGlobal('fetch',vi.fn(async()=>({ok:true,json:async()=>collection})));
 render(<App/>); const user=userEvent.setup(); await user.click(screen.getByRole('button',{name:'Explore map'}));
 const parent=screen.getByRole('checkbox',{name:'Snow routes'});
 const children=snowNames.map(name=>screen.getByRole('checkbox',{name}));
 expect(parent).toBeChecked(); children.forEach(child=>expect(child).toBeChecked());
 children[0].focus(); await user.keyboard('[Space]');
 expect(children[0]).not.toBeChecked(); expect(parent).toBePartiallyChecked();
 expect(screen.getByRole('checkbox',{name:'Streets'})).toBeChecked();
 parent.focus(); await user.keyboard('[Space]');
 expect(parent).toBeChecked(); expect(parent).not.toBePartiallyChecked(); children.forEach(child=>expect(child).toBeChecked());
 await user.keyboard('[Space]');
 expect(parent).not.toBeChecked(); children.forEach(child=>expect(child).not.toBeChecked());
 await user.tab(); expect(children[0]).toHaveFocus(); await user.keyboard('[Space]');
 expect(parent).toBePartiallyChecked(); expect(children[0]).toBeChecked();
});
test('requests only selected snow types before the server cap and skips disabled sources', async () => {
 const fetcher=vi.fn(async (_url:string)=>({ok:true,json:async()=>collection})); vi.stubGlobal('fetch',fetcher);
 render(<App/>); const user=userEvent.setup(); await user.click(screen.getByRole('button',{name:'Explore map'}));
 const params=()=>new URL(fetcher.mock.calls.at(-1)?.[0] || '/', 'http://localhost').searchParams;
 await waitFor(()=>expect(params().get('snow_types')).toBe('0,1,2,3'));
 await user.click(screen.getByRole('checkbox',{name:'Second Run Route'}));
 await waitFor(()=>expect(params().get('snow_types')).toBe('0,2,3'));
 expect(params().get('layers')).toBe('streets,snow,leaves');
 await user.click(screen.getByRole('checkbox',{name:'Streets'}));
 await waitFor(()=>expect(params().get('layers')).toBe('snow,leaves'));
 await user.click(screen.getByRole('checkbox',{name:'Seasonal loose leaves'}));
 await user.click(screen.getByRole('checkbox',{name:'Snow routes'})); // partial -> all
 await waitFor(()=>expect(params().get('snow_types')).toBe('0,1,2,3'));
 await user.click(screen.getByRole('checkbox',{name:'Snow routes'}));
 expect(await screen.findByText('Choose a layer to see mapped records.')).toBeVisible();
 const count=fetcher.mock.calls.length; await new Promise(resolve=>setTimeout(resolve,350)); expect(fetcher).toHaveBeenCalledTimes(count);
 await user.click(screen.getByRole('checkbox',{name:'First Run Route'}));
 await waitFor(()=>expect(params().get('snow_types')).toBe('2'));
 expect(params().get('layers')).toBe('snow');
 await user.click(screen.getByRole('checkbox',{name:'Snow routes'})); // partial -> all
 await user.click(screen.getByRole('checkbox',{name:'Snow routes'})); // all -> none
 await user.click(screen.getByRole('checkbox',{name:'Streets'}));
 await waitFor(()=>expect(params().get('layers')).toBe('streets'));
 expect(params().has('snow_types')).toBe(false);
});
test('renders independent source colors matching a single selected-layer legend', async () => {
 const sources=['Centerline',...snowNames.map((_,index)=>`Snow_Routes/${index}`)];
 const features=sources.map((source_key,index)=>({type:'Feature',geometry:{type:'LineString',coordinates:[[-77.48,38.74],[-77.46,38.76]]},properties:{id:String(index),name:'Test route',layer:index?'snow':'streets',classification:'same arbitrary label',source_key}}));
 vi.stubGlobal('fetch',vi.fn(async()=>({ok:true,json:async()=>({...collection,features})})));
 render(<App/>); const user=userEvent.setup(); await user.click(screen.getByRole('button',{name:'Explore map'}));
 await screen.findByText('5 mapped features in view');
 const rendered=()=>renderer.data.mock.calls.at(-1)?.[0].features;
 const colors=rendered().map((f:any)=>f.properties.display_color);
 expect(new Set(colors).size).toBe(5);
 const legend=within(screen.getByLabelText('Map legend'));
 expect(legend.queryByText('Snow routes')).not.toBeInTheDocument();
 ['Streets',...snowNames].forEach((name,index)=>{
  expect(legend.getAllByText(name)).toHaveLength(1);
  expect(legend.getByText(name).querySelector('i')).toHaveStyle({background:colors[index]});
 });
 await user.click(screen.getByRole('checkbox',{name:'Second Run Route'}));
 await screen.findByText('4 mapped features in view');
 expect(rendered().map((f:any)=>f.properties.source_key)).not.toContain('Snow_Routes/1');
 expect(legend.queryByText('Second Run Route')).not.toBeInTheDocument();
 expect(rendered().find((f:any)=>f.properties.source_key==='Snow_Routes/2').properties.display_color).toBe(colors[3]);
});
test('fetches viewport layers with abort signal and reports truncation', async () => {
 const fetcher = vi.fn(async (_url: string) => ({ok:true,json:async () => ({...collection,truncated:true})})); vi.stubGlobal('fetch',fetcher);
 render(<App/>); const user=userEvent.setup(); await user.click(screen.getByRole('button',{name:'Explore map'}));
 await waitFor(() => expect(fetcher).toHaveBeenCalledWith(expect.stringContaining('/api/map/features?bbox='),expect.objectContaining({signal:expect.any(AbortSignal)})));
 expect(await screen.findByText(/Some features omitted/)).toBeVisible();
 await user.click(screen.getByRole('checkbox',{name:'Snow routes'}));
 await waitFor(() => expect(fetcher.mock.calls.at(-1)?.[0]).toContain('layers=streets'));
});
test('route result resolves all feature geometry and fits bounds with source sheet', async () => {
 const feature={type:'Feature',geometry:{type:'LineString',coordinates:[[-77.48,38.74],[-77.46,38.76]]},properties:{id:'street:test',name:'Test street',layer:'streets',source_key:'Centerline',source_record_id:'42',source_url:'https://example.org/source'}};
 const fetcher=vi.fn(async (url:string) => ({ok:true,json:async()=>url.includes('/api/map/')?{...collection,features:[feature]}:{items:[{id:'street:test',name:'Test street',record_type:'route'}],search_mode:'database'}}));vi.stubGlobal('fetch',fetcher);
 vi.stubGlobal('scrollTo',vi.fn()); render(<App/>);const user=userEvent.setup(); await user.click(screen.getByRole('button',{name:'Search'}));await user.click(await screen.findByRole('button',{name:'Show on map'}));
 await waitFor(()=>expect(fetcher).toHaveBeenCalledWith('/api/map/features/street%3Atest',expect.any(Object)));
 await waitFor(()=>expect(renderer.fit).toHaveBeenCalled());
 expect(await screen.findByRole('link',{name:'Exact source record'})).toHaveAttribute('href','https://example.org/source');
});
test('aborts stale viewport requests on layer changes and reports failures', async () => {
 const fetcher=vi.fn((_url:string,_options?:RequestInit)=>new Promise<Response>(()=>{}));vi.stubGlobal('fetch',fetcher);
 render(<App/>);const user=userEvent.setup();await user.click(screen.getByRole('button',{name:'Explore map'}));
 await waitFor(()=>expect(fetcher).toHaveBeenCalled());const signal=fetcher.mock.calls[0][1]?.signal;
 await user.click(screen.getByRole('checkbox',{name:'Streets'}));expect(signal?.aborted).toBe(true);
 fetcher.mockImplementation(async()=>({ok:false} as Response));
 expect(await screen.findByRole('alert')).toHaveTextContent('Could not load mapped records');
});
test('tap opens the source sheet without interpreting live status', async () => {
 vi.stubGlobal('fetch',vi.fn(async()=>({ok:true,json:async()=>collection})));render(<App/>);const user=userEvent.setup();await user.click(screen.getByRole('button',{name:'Explore map'}));
 const {act}=await import('@testing-library/react');await act(async()=>renderer.handlers.click({features:[{properties:{name:'Snow source',layer:'snow',source_key:'Snow/0',source_record_id:'7',source_url:'https://example.org/7'}}]}));
 expect(screen.getByRole('heading',{name:'Snow source'})).toBeVisible();expect(screen.getByRole('link',{name:'Exact source record'})).toHaveAttribute('href','https://example.org/7');
 await user.click(screen.getByRole('button',{name:'Close map details'}));expect(screen.queryByRole('heading',{name:'Snow source'})).not.toBeInTheDocument();
});
test('defaults to services and preserves search when switching map modes', async () => {
 vi.stubGlobal('fetch', vi.fn(async () => ({ok: true, json: async () => collection})));
 render(<App/>); const user = userEvent.setup();
 expect(screen.getByRole('button', {name:'Find a service'})).toHaveAttribute('aria-pressed','true');
 await user.type(screen.getByRole('searchbox'), 'water');
 await user.click(screen.getByRole('button', {name:'Explore map'}));
 expect(screen.getByRole('region', {name:'Manassas map'})).toBeVisible();
 await user.click(screen.getByRole('button', {name:'Find a service'}));
 expect(screen.getByRole('searchbox')).toHaveValue('water');
});
test('layer panel and multi-type groups collapse without losing selections; new layers request their tokens', async () => {
 const fetcher=vi.fn(async (_url:string)=>({ok:true,json:async()=>({type:'FeatureCollection',features:[]})}));vi.stubGlobal('fetch',fetcher);
 render(<App/>); const user=userEvent.setup(); await user.click(screen.getByRole('button',{name:'Explore map'}));
 const snowTypes=screen.getByRole('button',{name:'Snow routes types'});
 expect(snowTypes).toHaveAttribute('aria-expanded','true');
 await user.click(snowTypes);
 expect(snowTypes).toHaveAttribute('aria-expanded','false');
 expect(screen.queryByRole('checkbox',{name:'First Run Route'})).not.toBeInTheDocument();
 expect(screen.getByRole('checkbox',{name:'Snow routes'})).toBeChecked();
 const panel=screen.getByRole('button',{name:/Map layers/});
 expect(panel).toHaveTextContent('6 of 14 on');
 await user.click(panel);
 expect(panel).toHaveAttribute('aria-expanded','false');
 expect(screen.queryByRole('checkbox',{name:'Streets'})).not.toBeInTheDocument();
 await user.click(panel);
 expect(screen.getByRole('checkbox',{name:'Streets'})).toBeChecked();
 // The group's own collapsed state survives collapsing the whole panel.
 expect(screen.getByRole('button',{name:'Snow routes types'})).toHaveAttribute('aria-expanded','false');
 await user.click(screen.getByRole('checkbox',{name:'Parking districts'}));
 await user.click(screen.getByRole('checkbox',{name:'City places'}));
 await waitFor(()=>expect(decodeURIComponent(fetcher.mock.calls.at(-1)?.[0] || '')).toMatch(/layers=[^&]*parking,places/));
 expect(panel).toHaveTextContent('8 of 14 on');
});
