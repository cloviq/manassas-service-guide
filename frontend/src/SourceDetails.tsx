import {Fragment} from 'react';
import ExternalLink from './ExternalLink';
import { dateFields, fieldLabels, sourceNotice } from './sources';
export type SourceLinks = {source_record:string; service_information:string|null; service_action:null};
const labels=fieldLabels;
// Render only the documented resident projection; never raw attributes.
export default function SourceDetails({details,sourceKey}:{details?:Record<string,string|number>;sourceKey?:string}) {
 const rows=Object.entries(details || {}).filter(([key,value])=>Object.prototype.hasOwnProperty.call(labels,key) && ((typeof value==='string' && value.trim()!=='' && value.length<=2000 && !/[\u0000-\u0008\u000b\u000c\u000e-\u001f]/.test(value)) || (typeof value==='number' && Number.isFinite(value))));
 const format=(key:string,value:string|number)=>{
  if(dateFields.includes(key) && typeof value==='number') {
   const date=new Date(value);return Number.isFinite(date.getTime())?date.toISOString().replace('T',' ').replace('.000Z',' UTC'):'Date unavailable';
  }
  return String(value);
 };
 return <>{rows.length>0 && <dl className="source-details">{rows.map(([key,value])=><Fragment key={key}><dt>{labels[key]}</dt><dd>{format(key,value)}</dd></Fragment>)}</dl>}
 {sourceNotice(sourceKey) && <p className="notice">{sourceNotice(sourceKey)}</p>}
 </>;
}
export function SourceRecordLinks({links,sourceUrl}:{links?:SourceLinks;sourceUrl?:string}) {
 return <div className="source-links"><ExternalLink url={links?.source_record || sourceUrl || ''}>Exact source record</ExternalLink>{links?.service_information && links.service_information!==links.source_record && <ExternalLink url={links.service_information}>Service information</ExternalLink>}</div>;
}
