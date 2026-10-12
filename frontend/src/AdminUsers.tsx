import { useEffect, useState, type ReactNode } from 'react';
import type { Account } from './Login';
interface UserRow extends Account { created_at: string }
const emptyForm = {username: '', password: '', role: 'user'};
async function request(url: string, init: RequestInit = {}) {
 const response = await fetch(url, {...init, headers: {'Content-Type': 'application/json', Accept: 'application/json'}});
 if (!response.ok) {
  const body = await response.json().catch(() => ({}));
  throw new Error(body.error ?? 'Request failed. Please try again.');
 }
 return response;
}
function generatePassword() {
 const bytes = crypto.getRandomValues(new Uint8Array(18));
 return btoa(String.fromCharCode(...bytes)).replace(/\+/g, '-').replace(/\//g, '_');
}
export default function AdminUsers({account, accountBar}: {account: Account; accountBar: ReactNode}) {
 const [users, setUsers] = useState<UserRow[]>();
 const [form, setForm] = useState(emptyForm);
 const [resetId, setResetId] = useState('');
 const [resetPassword, setResetPassword] = useState('');
 const [error, setError] = useState('');
 const [notice, setNotice] = useState('');
 const [pending, setPending] = useState(false);
 async function load() {
  try { setUsers(await (await request('/api/admin/users')).json()); }
  catch (e) { setError((e as Error).message); }
 }
 useEffect(() => { void load(); }, []);
 async function run(action: () => Promise<unknown>, success: string) {
  setPending(true); setError(''); setNotice('');
  try { await action(); setNotice(success); await load(); }
  catch (e) { setError((e as Error).message); }
  finally { setPending(false); }
 }
 const createUser = () => run(async () => {
  await request('/api/admin/users', {method: 'POST', body: JSON.stringify({...form, username: form.username.trim()})});
  setForm(emptyForm);
 }, `Created ${form.role} account “${form.username.trim().toLowerCase()}”. Share the password with them securely.`);
 const setRole = (user: UserRow, role: string) => run(() => request(`/api/admin/users/${user.id}`, {method: 'PATCH', body: JSON.stringify({role})}), `${user.username} is now ${role === 'admin' ? 'an admin' : 'a user'}.`);
 const savePassword = (user: UserRow) => run(async () => {
  await request(`/api/admin/users/${user.id}`, {method: 'PATCH', body: JSON.stringify({password: resetPassword})});
  setResetId(''); setResetPassword('');
 }, `Password updated for ${user.username}.${user.id === account.id ? '' : ' Their other sessions were signed out.'}`);
 const remove = (user: UserRow) => {
  if (!window.confirm(`Delete ${user.username}? They will be signed out immediately.`)) return;
  void run(() => request(`/api/admin/users/${user.id}`, {method: 'DELETE'}), `Deleted ${user.username}.`);
 };
 return <main className="admin"><header><span className="wordmark">MANASSAS / SERVICE GUIDE</span><span className="pill">Admin</span>{accountBar}</header><h1>Manage users</h1>
  <section className="card" aria-labelledby="create-title"><h2 id="create-title">Create a user</h2><form className="admin-create" autoComplete="off" onSubmit={e => {e.preventDefault(); void createUser();}}>
   <div><label htmlFor="new-username">Username</label><input id="new-username" autoCapitalize="none" spellCheck={false} required minLength={3} maxLength={64} pattern="[A-Za-z0-9._\-]+" title="Letters, digits, '.', '_' or '-'" value={form.username} onChange={e => setForm({...form, username: e.target.value})}/></div>
   <div><label htmlFor="new-password">Password</label><div className="inline-field"><input id="new-password" autoComplete="new-password" spellCheck={false} required minLength={12} maxLength={256} value={form.password} onChange={e => setForm({...form, password: e.target.value})}/><button type="button" onClick={() => setForm({...form, password: generatePassword()})}>Generate</button></div><p className="hint">At least 12 characters. Shown in plain text so you can share it.</p></div>
   <div><label htmlFor="new-role">Role</label><select id="new-role" value={form.role} onChange={e => setForm({...form, role: e.target.value})}><option value="user">User</option><option value="admin">Admin</option></select></div>
   <button className="primary" disabled={pending}>Create user</button>
  </form></section>
  {error && <p role="alert">{error}</p>}{notice && <p role="status" className="notice">{notice}</p>}
  <section aria-labelledby="users-title"><h2 id="users-title">Accounts{users ? ` (${users.length})` : ''}</h2>{!users ? !error && <p role="status">Loading accounts…</p> : <div className="table-wrap"><table><thead><tr><th scope="col">Username</th><th scope="col">Role</th><th scope="col">Created</th><th scope="col"><span className="visually-hidden">Actions</span></th></tr></thead><tbody>{users.map(user => {
   const self = user.id === account.id;
   return <tr key={user.id}><td>{user.username}{self && <span className="pill">You</span>}</td><td><select aria-label={`Role for ${user.username}`} value={user.role} disabled={self || pending} title={self ? 'You cannot change your own role' : undefined} onChange={e => void setRole(user, e.target.value)}><option value="user">User</option><option value="admin">Admin</option></select></td><td>{new Date(user.created_at).toLocaleDateString()}</td><td className="row-actions">{resetId === user.id ? <form onSubmit={e => {e.preventDefault(); void savePassword(user);}}><input aria-label={`New password for ${user.username}`} autoComplete="new-password" spellCheck={false} required minLength={12} maxLength={256} value={resetPassword} onChange={e => setResetPassword(e.target.value)}/><button type="button" onClick={() => setResetPassword(generatePassword())}>Generate</button><button className="primary" disabled={pending}>Save</button><button type="button" onClick={() => {setResetId(''); setResetPassword('');}}>Cancel</button></form> : <><button type="button" onClick={() => {setResetId(user.id); setResetPassword('');}}>Reset password</button><button type="button" className="danger" disabled={self || pending} onClick={() => remove(user)}>Delete</button></>}</td></tr>;
  })}</tbody></table></div>}</section>
 </main>;
}
