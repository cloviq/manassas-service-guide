import { useState } from 'react';
export interface Account { id: string; username: string; role: 'user' | 'admin' }
export default function Login({onSignedIn}: {onSignedIn: (account: Account) => void}) {
 const [username, setUsername] = useState('');
 const [password, setPassword] = useState('');
 const [error, setError] = useState('');
 const [pending, setPending] = useState(false);
 async function submit() {
  if (pending) return;
  setPending(true); setError('');
  try {
   const response = await fetch('/api/auth/login', {method: 'POST', headers: {'Content-Type': 'application/json', Accept: 'application/json'}, body: JSON.stringify({username: username.trim(), password})});
   if (response.ok) { onSignedIn(await response.json()); return; }
   setError(response.status === 401 ? 'Incorrect username or password.' : response.status === 429 ? 'Too many sign-in attempts. Wait a minute and try again.' : 'Could not sign in. Please try again.');
  } catch { setError('Could not reach the server. Please try again.'); }
  setPassword(''); setPending(false);
 }
 return <main className="login"><header><span className="wordmark">MANASSAS / SERVICE GUIDE</span><span className="pill">Independent prototype</span></header><section className="card login-card" aria-labelledby="login-title"><h1 id="login-title">Sign in</h1><p>Access is limited to invited accounts. Ask an administrator if you need one.</p><form onSubmit={e => {e.preventDefault(); void submit();}}><label htmlFor="login-username">Username</label><input id="login-username" autoComplete="username" autoCapitalize="none" spellCheck={false} required value={username} onChange={e => setUsername(e.target.value)}/><label htmlFor="login-password">Password</label><input id="login-password" type="password" autoComplete="current-password" required value={password} onChange={e => setPassword(e.target.value)}/>{error && <p role="alert">{error}</p>}<button className="primary" disabled={pending}>{pending ? 'Signing in…' : 'Sign in'}</button></form></section></main>;
}
