import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi, test, expect } from 'vitest';
import Root from './Root';
const admin = {id: 'admin-id', username: 'matt', role: 'admin'};
const user = {id: 'user-id', username: 'resident', role: 'user'};
const reply = (status: number, body?: unknown) => Promise.resolve({ok: status < 300, status, json: async () => body} as Response);
type Handler = (url: string, init?: RequestInit) => Promise<Response> | undefined;
function serve(handler: Handler) {
 const fetcher = vi.fn((url: string, init?: RequestInit) => handler(url, init) ?? reply(200, {items: [], search_mode: 'database', total: 0}));
 vi.stubGlobal('fetch', fetcher);
 return fetcher;
}
test('shows a sign-in page until credentials are accepted, without admin controls for users', async () => {
 let signedIn = false;
 const fetcher = serve((url, init) => {
  if (url === '/api/auth/me') return signedIn ? reply(200, user) : reply(401, {error: 'Sign in required.'});
  if (url === '/api/auth/login') {
   const body = JSON.parse(String(init?.body));
   signedIn = body.password === 'right-password';
   return signedIn ? reply(200, user) : reply(401, {error: 'Invalid username or password.'});
  }
 });
 render(<Root/>); const actor = userEvent.setup();
 expect(await screen.findByRole('heading', {name: 'Sign in'})).toBeVisible();
 expect(screen.queryByRole('search')).not.toBeInTheDocument();
 await actor.type(screen.getByLabelText('Username'), 'resident');
 await actor.type(screen.getByLabelText('Password'), 'wrong-password');
 await actor.click(screen.getByRole('button', {name: 'Sign in'}));
 expect(await screen.findByRole('alert')).toHaveTextContent('Incorrect username or password.');
 expect(screen.getByLabelText('Password')).toHaveValue('');
 await actor.type(screen.getByLabelText('Password'), 'right-password');
 await actor.click(screen.getByRole('button', {name: 'Sign in'}));
 expect(await screen.findByRole('heading', {name: 'How can we help?'})).toBeVisible();
 expect(screen.getByText('resident')).toBeVisible();
 expect(screen.queryByRole('button', {name: 'Manage users'})).not.toBeInTheDocument();
 expect(fetcher.mock.calls.find(([url]) => url === '/api/auth/login')?.[1]?.body).toBe(JSON.stringify({username: 'resident', password: 'wrong-password'}));
});
test('signing out ends the session and returns to the sign-in page', async () => {
 const fetcher = serve(url => url === '/api/auth/me' ? reply(200, user) : url === '/api/auth/logout' ? reply(204) : undefined);
 render(<Root/>); const actor = userEvent.setup();
 await actor.click(await screen.findByRole('button', {name: 'Sign out'}));
 expect(await screen.findByRole('heading', {name: 'Sign in'})).toBeVisible();
 expect(fetcher).toHaveBeenCalledWith('/api/auth/logout', {method: 'POST'});
});
test('admins manage accounts: create, change role, and cannot demote or delete themselves', async () => {
 const users = [{...admin, created_at: '2026-10-05T04:00:00Z'}, {...user, created_at: '2026-10-05T05:00:00Z'}];
 const fetcher = serve((url, init) => {
  if (url === '/api/auth/me') return reply(200, admin);
  if (url === '/api/admin/users' && init?.method === 'POST') {
   const body = JSON.parse(String(init.body));
   if (body.username === 'taken') return reply(409, {error: 'Username already exists.'});
   users.push({id: 'new-id', username: body.username, role: body.role, created_at: '2026-10-05T06:00:00Z'});
   return reply(201, users.at(-1));
  }
  if (url === '/api/admin/users') return reply(200, users);
  if (url === '/api/admin/users/user-id' && init?.method === 'PATCH') { users[1] = {...users[1], ...JSON.parse(String(init.body))}; return reply(200, users[1]); }
 });
 render(<Root/>); const actor = userEvent.setup();
 await actor.click(await screen.findByRole('button', {name: 'Manage users'}));
 expect(await screen.findByRole('heading', {name: 'Manage users'})).toBeVisible();
 const self = (await screen.findByRole('cell', {name: /^matt/})).closest('tr')!;
 expect(within(self).getByRole('combobox')).toBeDisabled();
 expect(within(self).getByRole('button', {name: 'Delete'})).toBeDisabled();

 await actor.type(screen.getByLabelText('Username'), 'taken');
 await actor.type(screen.getByLabelText('Password'), 'long-enough-pw');
 await actor.click(screen.getByRole('button', {name: 'Create user'}));
 expect(await screen.findByRole('alert')).toHaveTextContent('Username already exists.');
 await actor.clear(screen.getByLabelText('Username'));
 await actor.type(screen.getByLabelText('Username'), 'neighbor');
 await actor.selectOptions(screen.getByLabelText('Role'), 'admin');
 await actor.click(screen.getByRole('button', {name: 'Create user'}));
 expect(await screen.findByText('neighbor')).toBeVisible();
 expect(fetcher.mock.calls.filter(([url, init]) => url === '/api/admin/users' && init?.method === 'POST').at(-1)?.[1]?.body)
  .toBe(JSON.stringify({username: 'neighbor', password: 'long-enough-pw', role: 'admin'}));
 expect(screen.getByLabelText('Username')).toHaveValue('');

 await actor.selectOptions(screen.getByRole('combobox', {name: 'Role for resident'}), 'admin');
 expect(await screen.findByText('resident is now an admin.')).toBeVisible();
 expect(screen.getByRole('combobox', {name: 'Role for resident'})).toHaveValue('admin');

 await actor.click(screen.getByRole('button', {name: 'Back to guide'}));
 expect(screen.getByRole('heading', {name: 'How can we help?'})).toBeVisible();
});
