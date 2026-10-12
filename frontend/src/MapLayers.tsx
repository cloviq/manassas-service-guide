import { useEffect, useRef, useState } from 'react';
import { allMapSources, mapLayerGroups } from './sources';
export { allMapSources, defaultMapSources, mapLayerGroups } from './sources';

type Group = typeof mapLayerGroups[number];
function LayerGroup({group, enabled, onChange}: {group:Group; enabled:string[]; onChange:(ids:string[], checked:boolean)=>void}) {
 const parent = useRef<HTMLInputElement>(null);
 const ids = group.children.map(child=>child.id);
 const count = ids.filter(id=>enabled.includes(id)).length;
 const mixed = count>0 && count<ids.length;
 const [open,setOpen]=useState(true);
 useEffect(()=>{if(parent.current) parent.current.indeterminate=mixed;},[mixed]);
 return <div className="map-layer-group">
  <div className="map-layer-row">{ids.length>1 ? <button type="button" className="map-toggle-button" aria-expanded={open} aria-controls={`${group.id}-children`} aria-label={`${group.label} types`} title={open?'Collapse':'Expand'} onClick={()=>setOpen(!open)}><span aria-hidden="true" className="chevron">▾</span></button> : <span className="map-toggle-spacer" aria-hidden="true"/>}
  <label><input ref={parent} type="checkbox" checked={count===ids.length} aria-checked={mixed?'mixed':count===ids.length} aria-controls={ids.length>1?`${group.id}-children`:undefined} onChange={event=>onChange(ids,event.target.checked)}/>{group.label}</label></div>
  {ids.length>1 && <div id={`${group.id}-children`} hidden={!open} role="group" aria-label={`${group.label} types`} className="map-layer-children">
   {group.children.map(child=><label key={child.id}><input type="checkbox" checked={enabled.includes(child.id)} onChange={event=>onChange([child.id],event.target.checked)}/><span className="map-swatch" aria-hidden="true" style={{background:child.color}}/>{child.label}</label>)}
  </div>}
 </div>;
}
export default function MapLayers({enabled,onChange}:{enabled:string[];onChange:(ids:string[],checked:boolean)=>void}) {
 const [open,setOpen]=useState(true);
 // Groups stay mounted while collapsed so their own expanded state is kept.
 return <fieldset className="map-toggles"><legend><button type="button" className="map-panel-toggle" aria-expanded={open} aria-controls="map-layer-list" onClick={()=>setOpen(!open)}><span aria-hidden="true" className="chevron">▾</span>Map layers<span className="map-layer-count">{enabled.length} of {allMapSources.length} on</span></button></legend>
  <div id="map-layer-list" hidden={!open}>{mapLayerGroups.map(group=><LayerGroup key={group.id} group={group} enabled={enabled} onChange={onChange}/>)}</div></fieldset>;
}
