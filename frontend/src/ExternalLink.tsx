import type { ReactNode } from 'react';
export default function ExternalLink({url, children, fallback = 'Source link unavailable'}: {url: string; children: ReactNode; fallback?: string}) {
 let safe = false;
 try { const parsed = new URL(url); safe = ['https:', 'http:'].includes(parsed.protocol) && !parsed.username && !parsed.password; } catch { /* Not an absolute web URL. */ }
 return safe ? <a href={url} target="_blank" rel="noopener noreferrer">{children}</a> : <span className="muted">{fallback}</span>;
}
