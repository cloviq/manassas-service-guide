import { useId, useState } from 'react';
import type { Service } from './App';
export default function DemoIntake({service}: {service?: Service}) {
 const formId = useId();
 const serviceId = service?.record_type === 'service' ? service.id : undefined;
 const [description, setDescription] = useState('');
 const [email, setEmail] = useState('');
 const [ticket, setTicket] = useState('');
 const [error, setError] = useState('');
 const [pending, setPending] = useState(false);
 async function submit() {
  if (pending || !description.trim()) return;
  setPending(true);
  setError(''); setTicket('');
  try {
  const response = await fetch('/api/tickets', {method: 'POST', headers: {'Content-Type': 'application/json', Accept: 'application/json'}, body: JSON.stringify({description: description.trim(), ...(serviceId ? {service_id: serviceId} : {}), ...(email.trim() ? {contact_email: email.trim()} : {})})});
  if (!response.ok) throw new Error('Failed');
  const data = await response.json(); setTicket(data.id); setDescription(''); setEmail('');
  } catch {setError('Could not create demo ticket. Your draft is still here; try again.');} finally {setPending(false);}
 }
 return <details className="intake"><summary>{serviceId ? 'Try demo intake for this service' : 'Try demo intake'}</summary><p className="notice">Not sent to the City. This creates a test ticket in the prototype only. Do not include sensitive information or use for emergencies.</p><form autoComplete="off" onSubmit={e => {e.preventDefault(); void submit();}}><label htmlFor={`${formId}-description`}>Describe your request</label><textarea id={`${formId}-description`} required maxLength={5000} value={description} onChange={e => setDescription(e.target.value)}/><label htmlFor={`${formId}-email`}>Email (optional)</label><input id={`${formId}-email`} type="email" autoComplete="off" value={email} onChange={e => setEmail(e.target.value)}/><button disabled={pending || !description.trim()}>{pending ? 'Creating…' : 'Create demo ticket'}</button></form>{error && <p role="alert">{error}</p>}{ticket && <p role="status">Demo ticket {ticket} created · new. Not sent to the City.</p>}</details>;
}
