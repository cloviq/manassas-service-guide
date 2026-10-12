import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi, test, expect } from 'vitest';
import App from './App';
import DemoIntake from './DemoIntake';
const item = { id: 'test-task', name: 'Test task', description: 'A server-provided task.', category: 'Water & sewer', provider_name: 'Test provider', jurisdiction: 'Test jurisdiction', official_url: 'https://example.org/action', source_url: 'https://example.org/source' };
const response = (body: unknown) => Promise.resolve({ ok: true, json: async () => body } as Response);
test('paginates twenty records and resets on query, category, and type changes', async () => {
 const fetcher = vi.fn((url: string) => {
  const offset = Number(new URL(url, 'http://test').searchParams.get('offset'));
  return response({items: Array.from({length: offset ? 1 : 20}, (_, i) => ({...item, id: `task-${offset+i}`, name: `Task ${offset+i}`})), total: 21, limit: 20, offset, search_mode: 'database'});
 }); vi.stubGlobal('fetch', fetcher);
 render(<App/>); const user = userEvent.setup();
 await user.click(screen.getByRole('button', {name: 'Search'}));
 expect(await screen.findByRole('status')).toHaveTextContent('Showing 1–20 of 21 resources');
 expect(screen.getByRole('button', {name: 'Previous'})).toBeDisabled();
 await user.click(screen.getByRole('button', {name: 'Next'}));
 expect(await screen.findByText('Task 20')).toBeVisible();
 expect(screen.getByRole('status')).toHaveTextContent('Showing 21–21 of 21 resources');
 expect(screen.getByRole('button', {name: 'Next'})).toBeDisabled();
 await user.click(screen.getByRole('button', {name: 'Previous'}));
 expect(await screen.findByText('Task 0')).toBeVisible();
 for (const change of ['query', 'category', 'type']) {
  await user.click(screen.getByRole('button', {name: 'Next'}));
  await screen.findByText('Task 20');
  if (change === 'query') {await user.type(screen.getByRole('searchbox'), 'water'); await user.click(screen.getByRole('button', {name: 'Search'}));}
  if (change === 'category') await user.click(screen.getByRole('button', {name: 'Electricity'}));
  if (change === 'type') await user.selectOptions(screen.getByRole('combobox', {name: 'Resource type'}), 'all');
  expect(await screen.findByText('Task 0')).toBeVisible();
  expect(fetcher.mock.calls.at(-1)?.[0]).toContain('offset=0');
 }
 expect(screen.getByRole('status')).toHaveTextContent('search candidates');
});
test.each(['place', 'route', 'outage', 'infrastructure'])('labels %s as a source record, not a service', async record_type => {
 const asset = {...item, record_type, source_key: 'public-gis', source_record_id: '42', attributes: {private_email: 'private@example.org'}};
 vi.stubGlobal('fetch', vi.fn((url: string) => response(url.includes('/test-task') ? asset : {items: [asset], search_mode: 'database', total: 1})));
 render(<App/>); const user = userEvent.setup(); await user.selectOptions(screen.getByRole('combobox', {name: 'Resource type'}), 'all');
 expect(await screen.findByText('Data publisher')).toBeVisible();
 expect(screen.queryByText('Responsible provider')).not.toBeInTheDocument();
 expect(screen.getByText(record_type, {selector: '.record-type'})).toBeVisible();
 expect(screen.getByRole('link', {name: 'View official source'})).toHaveAttribute('href', asset.official_url);
 await user.click(screen.getByRole('button', {name: 'View details'}));
 expect(await screen.findByText('public-gis')).toBeVisible();
 expect(screen.getByText('42')).toBeVisible();
 expect(screen.queryByText('private@example.org')).not.toBeInTheDocument();
 expect(screen.queryByText('Try demo intake for this service')).not.toBeInTheDocument();
 if (record_type === 'outage') expect(screen.getByText(/Historical outage record.*not a live outage/)).toBeVisible();
});
test.each(['hybrid', 'keyword', 'database'])('honestly reports %s search only after a response', async search_mode => {
 vi.stubGlobal('fetch', vi.fn(() => response({items: [item], search_mode})));
 render(<App/>); const user = userEvent.setup();
 expect(screen.queryByText(/Hybrid search|Keyword search|Database browse/)).not.toBeInTheDocument();
 await user.click(screen.getByRole('button', {name: 'Search'}));
 const label = search_mode === 'hybrid' ? 'Hybrid search · keyword + semantic' : search_mode === 'keyword' ? 'Keyword search · semantic matching unavailable' : 'Database browse · no semantic ranking';
 expect(await screen.findByText(label)).toBeVisible();
 expect(screen.getByRole('status')).toHaveTextContent('1 resources shown · total unavailable');
 expect(screen.getByRole('button', {name: 'Next'})).toBeDisabled();
});
test('service detail intake attaches only the service ID while general help remains separate', async () => {
 const service = {...item, record_type: 'service'};
 const fetcher = vi.fn((url: string, _options?: RequestInit) => response(url === '/api/tickets' ? {id: 'demo-service', demo: true} : url.includes('/test-task') ? service : {items: [service], search_mode: 'hybrid'})); vi.stubGlobal('fetch', fetcher);
 render(<App/>); const user = userEvent.setup(); await user.click(screen.getByRole('button', {name: 'Search'}));
 await user.click(await screen.findByRole('button', {name: 'View details'}));
 await user.click(await screen.findByText('Try demo intake for this service'));
 const intake = within(screen.getByText('Try demo intake for this service').closest('details')!);
 await user.type(intake.getByRole('textbox', {name: 'Describe your request'}), 'Service draft');
 await user.click(intake.getByRole('button', {name: 'Create demo ticket'}));
 await waitFor(() => expect(fetcher).toHaveBeenLastCalledWith('/api/tickets', expect.objectContaining({body: JSON.stringify({description: 'Service draft', service_id: item.id})})));
 expect(await screen.findByText(/Demo ticket demo-service/)).toBeVisible();
 expect(screen.getByText('Try demo intake')).toBeVisible();
});
test('empty later pages keep truthful counts and allow going back', async () => {
 vi.stubGlobal('fetch', vi.fn((url: string) => response({items: url.includes('offset=20') ? [] : Array.from({length: 20}, (_, i) => ({...item, id: String(i)})), total: url.includes('offset=20') ? 20 : 21, offset: url.includes('offset=20') ? 20 : 0, limit: 20, search_mode: 'database'})));
 render(<App/>); const user = userEvent.setup(); await user.click(screen.getByRole('button', {name: 'Search'}));
 await user.click(await screen.findByRole('button', {name: 'Next'}));
 expect(screen.getByRole('status')).toHaveTextContent('No resources on this page · 20 resources total');
 expect(screen.getByRole('button', {name: 'Previous'})).toBeEnabled();
 expect(screen.getByRole('button', {name: 'Next'})).toBeDisabled();
});
test.each(['place', 'route', 'outage', 'infrastructure'] as const)('intake defense never attaches a %s ID', async record_type => {
 const fetcher = vi.fn(() => response({id: 'general-demo', demo: true})); vi.stubGlobal('fetch', fetcher);
 render(<DemoIntake service={{...item, record_type}}/>); const user = userEvent.setup();
 await user.click(screen.getByText('Try demo intake'));
 await user.type(screen.getByLabelText('Describe your request'), 'General help');
 await user.click(screen.getByRole('button', {name: 'Create demo ticket'}));
 await waitFor(() => expect(fetcher).toHaveBeenLastCalledWith('/api/tickets', expect.objectContaining({body: JSON.stringify({description: 'General help'})})));
});
test.each(['', 'Streets & sidewalks'])('empty snow service search offers explicit all-resource discovery preserving category %s', async category => {
 const route = {...item, id: 'snow-route', name: 'Snow plow route', record_type: 'route', category: 'Streets & sidewalks'};
 const trash = {...item, id: 'trash', name: 'Unrelated trash pickup'};
 const fetcher = vi.fn((url: string) => {
  const params = new URL(url, 'http://test').searchParams;
  const items = params.get('q') === 'snow' ? params.get('type') === 'all' ? [route] : [] : [trash];
  return response({items, total: items.length, offset: 0, search_mode: 'keyword'});
 }); vi.stubGlobal('fetch', fetcher);
 render(<App/>); const user = userEvent.setup();
 await user.click(screen.getByRole('button', {name: 'Search'}));
 expect(await screen.findByText(trash.name)).toBeVisible();
 await user.type(screen.getByRole('searchbox'), 'snow');
 if (category) await user.click(screen.getByRole('button', {name: category}));
 else await user.click(screen.getByRole('button', {name: 'Search'}));
 expect(await screen.findByText('No matching services for “snow”.')).toBeVisible();
 expect(screen.getByRole('combobox', {name: 'Resource type'})).toHaveValue('service');
 expect(screen.queryByText(trash.name)).not.toBeInTheDocument();
 expect(screen.queryByText(route.name)).not.toBeInTheDocument();
 expect(fetcher.mock.calls.every(([url]) => new URL(url, 'http://test').searchParams.get('type') === 'service')).toBe(true);
 const action = screen.getByRole('button', {name: 'Search all resources'});
 action.focus(); await user.keyboard('{Enter}');
 expect(await screen.findByText(route.name)).toBeVisible();
 expect(screen.getByRole('searchbox')).toHaveValue('snow');
 expect(screen.getByRole('combobox', {name: 'Resource type'})).toHaveValue('all');
 if (category) expect(screen.getByRole('button', {name: category})).toHaveAttribute('aria-pressed', 'true');
 const params = new URL(fetcher.mock.calls.at(-1)![0], 'http://test').searchParams;
 expect(Object.fromEntries(params)).toEqual({q: 'snow', category, type: 'all', limit: '20', offset: '0'});
 expect(screen.getByText('route', {selector: '.record-type'})).toBeVisible();
 expect(screen.getByText('Data publisher')).toBeVisible();
 expect(screen.getByRole('status')).toHaveTextContent('Showing 1–1 of 1 search candidates');
 expect(screen.getByRole('link', {name: 'View official source'})).toHaveAttribute('href', route.official_url);
 expect(screen.getByRole('button', {name: 'Show on map'})).toBeVisible();
 expect(screen.queryByText(trash.name)).not.toBeInTheDocument();
 expect(screen.queryByRole('button', {name: 'Search all resources'})).not.toBeInTheDocument();
});

test('resource type filter defaults to services and explicitly browses other records', async () => {
 const fetcher = vi.fn(() => response({items: [], search_mode: 'database'})); vi.stubGlobal('fetch', fetcher);
 render(<App/>); const user = userEvent.setup();
 const filter = screen.getByRole('combobox', {name: 'Resource type'});
 expect(filter).toHaveValue('service');
 for (const type of ['all', 'place', 'route', 'outage', 'infrastructure', 'service']) {
  await user.selectOptions(filter, type);
  await waitFor(() => expect(fetcher).toHaveBeenLastCalledWith(`/api/services?q=&category=&type=${type}&limit=20&offset=0`, expect.any(Object)));
 }
});
test('searches the API and presents server-provided responsibility and official action', async () => {
 const fetcher = vi.fn(() => response({items: [item], search_mode: 'hybrid'})); vi.stubGlobal('fetch', fetcher);
 render(<App/>);
 expect(screen.getByRole('heading', {name: 'How can we help?'})).toBeVisible();
 const user = userEvent.setup();
 await user.type(screen.getByRole('searchbox'), 'leaking pipe');
 await user.click(screen.getByRole('button', {name: 'Search'}));
 await waitFor(() => expect(fetcher).toHaveBeenLastCalledWith('/api/services?q=leaking+pipe&category=&type=service&limit=20&offset=0', expect.any(Object)));
 expect(await screen.findByText('Test provider')).toBeVisible();
 expect(screen.getByText('Test jurisdiction')).toBeVisible();
 expect(screen.getByRole('link', {name: 'Official next action'})).toHaveAttribute('href', item.official_url);
});
test('category selection searches with the displayed category and supports clearing it', async () => {
 const fetcher = vi.fn(() => response({items: [], search_mode: 'hybrid'})); vi.stubGlobal('fetch', fetcher);
 render(<App/>); const user = userEvent.setup();
 for (const category of ['Trash & recycling', 'Water & sewer', 'Electricity', 'Streets & sidewalks', 'Other services']) expect(screen.getByRole('button', {name: category})).toBeVisible();
 await user.click(screen.getByRole('button', {name: 'Water & sewer'}));
 await waitFor(() => expect(fetcher).toHaveBeenLastCalledWith('/api/services?q=&category=Water+%26+sewer&type=service&limit=20&offset=0', expect.any(Object)));
 expect(screen.getByRole('button', {name: 'Water & sewer'})).toHaveAttribute('aria-pressed', 'true');
 await user.click(screen.getByRole('button', {name: 'Water & sewer'}));
 await waitFor(() => expect(fetcher).toHaveBeenLastCalledWith('/api/services?q=&category=&type=service&limit=20&offset=0', expect.any(Object)));
});
test('search announces loading, empty database fallback, and retryable errors', async () => {
 let resolve!: (value: Response) => void;
 const fetcher = vi.fn(() => new Promise<Response>(r => {resolve = r;})); vi.stubGlobal('fetch', fetcher);
 render(<App/>); const user = userEvent.setup(); await user.click(screen.getByRole('button', {name: 'Search'}));
 expect(screen.getByRole('status')).toHaveTextContent('Searching');
 resolve(await response({items: [], search_mode: 'database', warning: 'Search engine unavailable.'}));
 expect(await screen.findByText(/No services found/)).toBeVisible();
 expect(screen.getByText(/Database browse/)).toBeVisible();
 expect(screen.getByText('Search engine unavailable.').closest('details')).not.toHaveAttribute('open');
 fetcher.mockImplementation(() => Promise.resolve({ok: false} as Response));
 await user.click(screen.getByRole('button', {name: 'Search'}));
 expect(await screen.findByRole('alert')).toHaveTextContent('Could not load services');
 expect(screen.queryByText(/No services found/)).not.toBeInTheDocument();
 await user.click(screen.getByRole('button', {name: 'Try again'}));
 expect(fetcher).toHaveBeenCalledTimes(3);
});
test('opens API detail with source verification warning and safe source links', async () => {
 const fetcher = vi.fn((url: string) => response(url.includes('/test-task') ? {...item, instructions: 'Follow the provider instructions.'} : {items: [item], search_mode: 'hybrid'})); vi.stubGlobal('fetch', fetcher);
 render(<App/>); const user = userEvent.setup(); await user.click(screen.getByRole('button', {name: 'Search'}));
 await user.click(await screen.findByRole('button', {name: 'View details'}));
 expect(await screen.findByText('Follow the provider instructions.')).toBeVisible();
 expect(screen.getByText(/Source verification pending/)).toBeVisible();
 expect(fetcher).toHaveBeenLastCalledWith('/api/services/test-task', expect.any(Object));
 expect(screen.getByRole('link', {name: 'Source record'})).toHaveAttribute('rel', 'noopener noreferrer');
 await user.click(screen.getByRole('button', {name: 'Close details'}));
 expect(screen.queryByText('Follow the provider instructions.')).not.toBeInTheDocument();
});
test('submits an explicitly demo general request without browser persistence', async () => {
 const fetcher = vi.fn(() => response({id: 'demo-123', status: 'new', demo: true})); vi.stubGlobal('fetch', fetcher);
 const storage = vi.spyOn(Storage.prototype, 'setItem');
 render(<App/>); const user = userEvent.setup();
 await user.click(screen.getByText('Try demo intake'));
 expect(screen.getByText(/Not sent to the City/)).toBeVisible();
 await user.type(screen.getByLabelText('Describe your request'), 'A test request');
 await user.type(screen.getByLabelText('Email (optional)'), 'test@example.org');
 await user.click(screen.getByRole('button', {name: 'Create demo ticket'}));
 await waitFor(() => expect(fetcher).toHaveBeenCalledWith('/api/tickets', expect.objectContaining({method: 'POST', body: JSON.stringify({description: 'A test request', contact_email: 'test@example.org'})})));
 expect(await screen.findByText(/Demo ticket demo-123/)).toBeVisible();
 expect(screen.getByLabelText('Describe your request')).toHaveValue('');
 expect(screen.getByLabelText('Email (optional)')).toHaveValue('');
 expect(storage).not.toHaveBeenCalled();
});
test.each(['javascript:alert(1)', 'data:text/html,bad', '//example.org', 'not-a-url'])('does not turn unsafe URL %s into an action', async url => {
 vi.stubGlobal('fetch', vi.fn(() => response({items: [{...item, official_url: url}], search_mode: 'hybrid'})));
 render(<App/>); await userEvent.setup().click(screen.getByRole('button', {name: 'Search'}));
 await screen.findByText(item.name);
 expect(screen.queryByRole('link', {name: 'Official next action'})).not.toBeInTheDocument();
 expect(screen.getByText('Official link unavailable')).toBeVisible();
});
test('failed demo requests preserve the draft and allow retry without false success', async () => {
 const fetcher = vi.fn(() => Promise.resolve({ok: false} as Response)); vi.stubGlobal('fetch', fetcher);
 render(<App/>); const user = userEvent.setup(); await user.click(screen.getByText('Try demo intake'));
 await user.type(screen.getByLabelText('Describe your request'), 'Draft request');
 await user.click(screen.getByRole('button', {name: 'Create demo ticket'}));
 expect(await screen.findByRole('alert')).toHaveTextContent('Could not create demo ticket');
 expect(screen.getByLabelText('Describe your request')).toHaveValue('Draft request');
 expect(screen.queryByText(/Demo ticket .* created/)).not.toBeInTheDocument();
});
test('keeps the landing concise with a prototype pill and full provenance in collapsed About', async () => {
 vi.stubGlobal('fetch', vi.fn(() => response({items: [item], search_mode: 'hybrid'})));
 render(<App/>); const user = userEvent.setup();
 expect(screen.getByText('Independent prototype')).toBeVisible();
 await user.tab(); expect(screen.getByRole('button', {name: 'Find a service'})).toHaveFocus();
 await user.tab(); expect(screen.getByRole('button', {name: 'Explore map'})).toHaveFocus();
 await user.tab(); expect(screen.getByRole('searchbox')).toHaveFocus();
 await user.keyboard('test{Enter}'); await screen.findByText(item.name);
 const about = screen.getByText('About this prototype').closest('details');
 expect(about).not.toHaveAttribute('open');
 expect(screen.getByRole('link', {name: 'Test task — source', hidden: true}).closest('details')).toBe(about);
});
test('detail loading failures stay local and are retryable', async () => {
 let resolve!: (value: Response) => void;
 const fetcher = vi.fn((url: string) => url.includes('/test-task') ? new Promise<Response>(r => {resolve = r;}) : response({items: [item], search_mode: 'hybrid'})); vi.stubGlobal('fetch', fetcher);
 render(<App/>); const user = userEvent.setup(); await user.click(screen.getByRole('button', {name: 'Search'}));
 await user.click(await screen.findByRole('button', {name: 'View details'}));
 expect(screen.getByText('Loading details…')).toBeVisible();
 resolve({ok: false} as Response);
 expect(await screen.findByRole('alert')).toHaveTextContent('Could not load details');
 expect(screen.getByText(item.provider_name)).toBeVisible();
 expect(screen.getByRole('button', {name: 'View details'})).toBeEnabled();
});
test('late search responses cannot replace the most recent results', async () => {
 let resolveFirst!: (value: Response) => void;
 vi.stubGlobal('fetch', vi.fn().mockImplementationOnce(() => new Promise<Response>(r => {resolveFirst = r;})).mockImplementationOnce(() => response({items: [{...item, name: 'Newest result'}], search_mode: 'hybrid'})));
 render(<App/>); const user = userEvent.setup(); await user.click(screen.getByRole('button', {name: 'Search'}));
 await user.click(screen.getByRole('button', {name: 'Electricity'}));
 await screen.findByText('Newest result');
 resolveFirst(await response({items: [item], search_mode: 'keyword'}));
 await new Promise(r => setTimeout(r, 10));
 expect(screen.queryByText('Test task')).not.toBeInTheDocument();
 expect(screen.getByText('Newest result')).toBeVisible();
});
test('demo intake blocks blank drafts and duplicate submissions while pending', async () => {
 let resolve!: (value: Response) => void; const fetcher = vi.fn(() => new Promise<Response>(r => {resolve = r;})); vi.stubGlobal('fetch', fetcher);
 render(<App/>); const user = userEvent.setup(); await user.click(screen.getByText('Try demo intake'));
 expect(screen.getByRole('button', {name: 'Create demo ticket'})).toBeDisabled();
 await user.type(screen.getByLabelText('Describe your request'), '  ');
 expect(screen.getByRole('button', {name: 'Create demo ticket'})).toBeDisabled();
 await user.type(screen.getByLabelText('Describe your request'), 'Test');
 await user.click(screen.getByRole('button', {name: 'Create demo ticket'}));
 expect(screen.getByRole('button', {name: 'Creating…'})).toBeDisabled();
 resolve(await response({id: 'demo-456', status: 'new', demo: true}));
 expect(await screen.findByText(/Demo ticket demo-456/)).toBeVisible();
 expect(fetcher).toHaveBeenCalledTimes(1);
});

test('street groups show a count and accessible per-segment provenance, not raw attributes', async () => {
 const street = {...item, name: 'EXAMPLE ROAD (fictional)', record_type: 'route', source_key: 'Centerline_COM/0', segment_count: 14,
  segments: Array.from({length: 14}, (_, i) => ({id: `segment-${i}`, source_record_id: String(i), source_url: `https://example.org/segment/${i}`, attributes: {internal: 'DO NOT DISPLAY'}}))};
 vi.stubGlobal('fetch', vi.fn((url: string) => response(url.includes('/test-task') ? street : {items: [street], search_mode: 'database', total: 1})));
 render(<App/>); const user = userEvent.setup();
 await user.selectOptions(screen.getByRole('combobox', {name: 'Resource type'}), 'all');
 expect(await screen.findByText('14 street segments')).toBeVisible();
 await user.click(screen.getByRole('button', {name: 'View details'}));
 await user.click(await screen.findByText('Street segments and sources (14)'));
 expect(screen.getAllByRole('link', {name: /Segment .* — exact source record/})).toHaveLength(14);
 expect(screen.queryByText('DO NOT DISPLAY')).not.toBeInTheDocument();
});
