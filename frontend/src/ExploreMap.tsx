import { useEffect, useMemo, useRef, useState } from 'react';
import MapLayers, { allMapSources, defaultMapSources } from './MapLayers';
import { mapSources, sourceFor, variantParams } from './sources';
import * as maplibregl from 'maplibre-gl';
import type { GeoJSONSource, Map as LibreMap } from 'maplibre-gl';
import type { FeatureCollection, Geometry } from 'geojson';
import SourceDetails, {SourceRecordLinks, type SourceLinks} from './SourceDetails';
import CollectionGuidance from './CollectionGuidance';
// Vite must bundle the module worker and its relative shared imports.
// @ts-expect-error Vite worker URL query
import workerUrl from 'maplibre-gl/dist/maplibre-gl-worker.mjs?worker&url';
maplibregl.setWorkerUrl(workerUrl);
import 'maplibre-gl/dist/maplibre-gl.css';

type Properties = {id: string; name: string; layer: string; classification?: string; source_key: string; source_record_id: string; source_url: string; details?:Record<string,string|number>; links?:SourceLinks};
export type MapData = FeatureCollection<Geometry, Properties> & {truncated?: boolean};
const empty: MapData = {type:'FeatureCollection', features:[]};
const sourceStyle = sourceFor;
export default function ExploreMap({featureId = ''}: {featureId?: string}) {
 const [selected,setSelected]=useState<Properties>();
 const [selectionStatus,setSelectionStatus]=useState('');
 const host=useRef<HTMLDivElement>(null), map=useRef<LibreMap | null>(null);
 const [ready,setReady]=useState(false), [enabledSources,setEnabledSources]=useState(defaultMapSources);
 const layers=useMemo(()=>Array.from(new Set(enabledSources.map(id=>mapSources.find(source=>source.id===id)!.layer))),[enabledSources]);
 const changeSources=(ids:string[],checked:boolean)=>setEnabledSources(old=>allMapSources.filter(id=>ids.includes(id)?checked:old.includes(id)));
 const [data,setData]=useState<MapData>(empty), [status,setStatus]=useState('Loading map…'), [error,setError]=useState(''), [mapError,setMapError]=useState('');
 const [retry,setRetry]=useState(0);
 useEffect(() => {
  if(!host.current) return;
  let instance: LibreMap;
  try {
   instance=new maplibregl.Map({container:host.current,center:[-77.475,38.75],zoom:12,maxZoom:19,attributionControl:false,renderWorldCopies:false,
    style:{version:8,sources:{osm:{type:'raster',tiles:['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],tileSize:256,maxzoom:19,attribution:'© <a href="https://www.openstreetmap.org/copyright" target="_blank" rel="noopener">OpenStreetMap</a> contributors'}},layers:[{id:'basemap',type:'raster',source:'osm'}]}});
   map.current=instance;
   instance.addControl(new maplibregl.NavigationControl({showCompass:false}),'top-right');
   instance.addControl(new maplibregl.AttributionControl({compact:false}),'bottom-right');
   instance.on('error', event => {console.error('Map resource error:', event.error);setMapError(`Map resource unavailable: ${event.error.message}`);});
   instance.on('load', () => {
    instance.addSource('routes',{type:'geojson',data:empty});
    instance.addLayer({id:'areas',type:'fill',source:'routes',filter:['==',['geometry-type'],'Polygon'],paint:{'fill-color':['get','display_color'],'fill-opacity':0.18}});
    instance.addLayer({id:'routes',type:'line',source:'routes',filter:['!=',['geometry-type'],'Point'],paint:{'line-color':['get','display_color'],'line-width':['case',['==',['geometry-type'],'Polygon'],2,['==',['get','layer'],'snow'],5,3],'line-opacity':0.85}});
    instance.addLayer({id:'points',type:'circle',source:'routes',filter:['==',['geometry-type'],'Point'],paint:{'circle-color':['get','display_color'],'circle-radius':7,'circle-stroke-color':'#fff','circle-stroke-width':2}});
    instance.addSource('selection',{type:'geojson',data:empty});
    instance.addLayer({id:'selection-area',type:'fill',source:'selection',filter:['==',['geometry-type'],'Polygon'],paint:{'fill-color':'#f2a900','fill-opacity':0.15}});
    instance.addLayer({id:'selection',type:'line',source:'selection',filter:['!=',['geometry-type'],'Point'],paint:{'line-color':'#f2a900','line-width':8,'line-opacity':0.7}});
    instance.addLayer({id:'selection-point',type:'circle',source:'selection',filter:['==',['geometry-type'],'Point'],paint:{'circle-color':'#f2a900','circle-radius':10,'circle-opacity':0.7}});
    for(const layer of ['areas','routes','points']) instance.on('click',layer,event => {
     const p=event.features?.[0]?.properties;if(!p) return;
     // MapLibre serializes nested GeoJSON properties into JSON strings.
     const parse=(value:unknown)=>{if(typeof value!=='string') return value;try{return JSON.parse(value);}catch{return undefined;}};
     setSelected({...p,details:parse(p.details),links:parse(p.links)} as Properties);
    });
    setReady(true);
   });
  } catch {setMapError('Map rendering unavailable. This browser requires WebGL. Service search still works.');return;}
  return () => {instance.remove();map.current=null;};
 },[]);
 useEffect(() => {
  if(!ready || !map.current) return;
  const instance=map.current; let timer: ReturnType<typeof setTimeout>; let controller: AbortController | undefined; let generation=0;
  const schedule=() => {
   clearTimeout(timer); controller?.abort(); const current=++generation;
   (instance.getSource('routes') as GeoJSONSource).setData(empty); setData(empty);setError('');
   if(!layers.length) {setStatus('Choose a layer to see mapped records.');return;}
   setStatus('Loading mapped records…');
   timer=setTimeout(async () => {
    controller=new AbortController();
    try {
     const bbox=instance.getBounds().toArray().flat().join(',');
     const params=new URLSearchParams({bbox,layers:layers.join(',')});
     for(const {layer,param} of variantParams) {
      const variants=mapSources.filter(source=>source.layer===layer && source.variant && enabledSources.includes(source.id)).map(source=>source.variant!);
      if(variants.length) params.set(param,variants.join(','));
     }
     const response=await fetch(`/api/map/features?${params}`,{signal:controller.signal,headers:{Accept:'application/json'}});
     if(!response.ok) throw new Error();
     const result: MapData=await response.json(); if(result.type!=='FeatureCollection'||!Array.isArray(result.features)) throw new Error();
     if(current!==generation || controller.signal.aborted) return;
     // The query filters before the server cap; this guard also prevents stale/older
     // API responses from drawing unchecked sources during a rolling deployment.
     const visible={...result,features:result.features.filter(f=>{const source=sourceStyle(f.properties);return source && enabledSources.includes(source.id);})};
     const styled={...visible,features:visible.features.map(f=>({...f,properties:{...f.properties,display_color:sourceStyle(f.properties)!.color}}))};
     (instance.getSource('routes') as GeoJSONSource).setData(styled);setData(visible);setStatus(`${visible.features.length} mapped features in view`);
    } catch {if(current===generation && !controller.signal.aborted) {setError('Could not load mapped records. Service search is still available.');setStatus('');}}
   },300);
  };
  instance.on('moveend',schedule);schedule();
  return () => {++generation;clearTimeout(timer);controller?.abort();instance.off?.('moveend',schedule);};
 },[ready,layers,enabledSources,retry]);
 useEffect(() => {
  if(!ready || !featureId || !map.current) return;
  const instance=map.current, controller=new AbortController();
  setSelectionStatus('Loading selected route…');
  void (async()=>{try {
   const response=await fetch(`/api/map/features/${encodeURIComponent(featureId)}`,{signal:controller.signal,headers:{Accept:'application/json'}});
   if(!response.ok) throw new Error();
   const result: MapData=await response.json();
   if(controller.signal.aborted) return;
   if(result.type!=='FeatureCollection' || !result.features?.length) throw new Error();
   (instance.getSource('selection') as GeoJSONSource).setData(result);
   const points: number[][]=[];
   const collect=(value: unknown):void=>{if(!Array.isArray(value)) return;if(typeof value[0]==='number' && typeof value[1]==='number') points.push(value);else value.forEach(collect);};
   result.features.forEach(f=>{if('coordinates' in f.geometry) collect(f.geometry.coordinates);});
   if(points.length) {const xs=points.map(p=>p[0]),ys=points.map(p=>p[1]);instance.fitBounds([[Math.min(...xs),Math.min(...ys)],[Math.max(...xs),Math.max(...ys)]],{padding:48,maxZoom:16,duration:0});}
   setSelected(result.features[0].properties);setSelectionStatus(result.truncated ? 'Selected route has omitted segments.' : 'Selected route highlighted in gold.');
  }catch {if(!controller.signal.aborted) setSelectionStatus('Could not load selected route geometry. Return to search and try again.');}})();
  return ()=>controller.abort();
 },[ready,featureId]);
 const legend=mapSources.filter(source=>enabledSources.includes(source.id));
 return <section aria-label="Manassas map" className="map-view"><h1>Explore Manassas</h1><p className="map-note">Published City source records; not live status or verified service eligibility.</p><MapLayers enabled={enabledSources} onChange={changeSources}/><CollectionGuidance/><p role="status">{status}</p>{data.truncated && <p className="notice">Some features omitted. Zoom in to see more complete coverage.</p>}{error && <div role="alert">{error}<button onClick={()=>setRetry(v=>v+1)}>Retry map data</button></div>}{mapError && <p role="alert">{mapError}</p>}<p role="status">{selectionStatus}</p><div ref={host} className="map-canvas" aria-label="Interactive Manassas source map"/><div className="map-legend" aria-label="Map legend">{legend.map(source=><span key={source.id}><i aria-hidden="true" style={{background:source.color}}/>{source.label}</span>)}</div><p className="map-note">Layers are shown together, not merged. Tap an area, point or line for its source. Basemap © OpenStreetMap contributors.</p>{selected && <aside className="map-sheet" aria-label="Selected map feature"><button className="sheet-close" onClick={()=>setSelected(undefined)}>Close map details</button><h2>{selected.name || 'Unnamed source record'}</h2><p>{sourceStyle(selected)?.label || selected.layer}{selected.classification ? ` · ${selected.classification}` : ''}</p><dl><dt>Source dataset</dt><dd>{selected.source_key}</dd><dt>Source record ID</dt><dd>{selected.source_record_id}</dd></dl><SourceDetails details={selected.details} sourceKey={selected.source_key}/><SourceRecordLinks links={selected.links} sourceUrl={selected.source_url}/><p className="map-note">Published geometry only; not live plow activity. Confirm current information with the City.</p></aside>}</section>;
}
