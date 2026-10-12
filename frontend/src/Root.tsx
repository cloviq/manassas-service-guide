import { useEffect, useState } from 'react';
import App from './App';
import AdminUsers from './AdminUsers';
import Login, { type Account } from './Login';
// The server enforces sessions and roles; this only chooses what to render.
export default function Root() {
 const [account, setAccount] = useState<Account | null>();
 const [view, setView] = useState<'guide' | 'admin'>('guide');
 useEffect(() => {
  let live = true;
  fetch('/api/auth/me', {headers: {Accept: 'application/json'}})
   .then(async response => { if (live) setAccount(response.ok ? await response.json() : null); })
   .catch(() => { if (live) setAccount(null); });
  return () => { live = false; };
 }, []);
 async function signOut() {
  await fetch('/api/auth/logout', {method: 'POST'}).catch(() => undefined);
  setView('guide'); setAccount(null);
 }
 if (account === undefined) return <main aria-busy="true"><p role="status">Loading…</p></main>;
 if (account === null) return <Login onSignedIn={signedIn => {setView('guide'); setAccount(signedIn);}}/>;
 const admin = account.role === 'admin';
 const accountBar = <div className="account-bar"><span className="muted">Signed in as <strong>{account.username}</strong>{admin && ' · admin'}</span>{admin && <button type="button" onClick={() => setView(view === 'admin' ? 'guide' : 'admin')}>{view === 'admin' ? 'Back to guide' : 'Manage users'}</button>}<button type="button" onClick={() => void signOut()}>Sign out</button></div>;
 return <>{view === 'admin' && admin && <AdminUsers account={account} accountBar={accountBar}/>}<div hidden={view !== 'guide'}><App accountBar={accountBar}/></div></>;
}
