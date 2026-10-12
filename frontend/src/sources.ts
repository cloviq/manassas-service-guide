// Map layers, field labels, notices and official links come from the shared
// source registry (catalog/sources.json), never from source keys in components.
import registry from '../../catalog/sources.json';

type Source = {key:string; notice?:string; map?:{layer:string; variant?:string; label?:string; color?:string; classification:string}};
type Layer = {id:string; group:string; label:string; color?:string; default?:boolean; variant_param?:string};
const sources: Source[] = registry.sources;
const layers: Layer[] = registry.map_layers;

export type MapSource = {id:string; label:string; color:string; layer:string; variant?:string};
// Variant layers expose one child per source (id = source key); other layers are one child.
function children(layer: Layer): MapSource[] {
 if(!layer.variant_param) return [{id:layer.id, label:layer.label, color:layer.color!, layer:layer.id}];
 return sources.filter(s=>s.map?.layer===layer.id).map(s=>({id:s.key, label:s.map!.label ?? s.map!.classification, color:s.map!.color!, layer:layer.id, variant:s.map!.variant}));
}
export const mapLayerGroups = registry.map_groups.map(group=>({...group, children:layers.filter(l=>l.group===group.id).flatMap(children)}));
export const mapSources = mapLayerGroups.flatMap(group=>group.children);
export const allMapSources = mapSources.map(source=>source.id);
export const defaultMapSources = layers.filter(l=>l.default).flatMap(children).map(source=>source.id);
export const variantParams = layers.filter(l=>l.variant_param).map(l=>({layer:l.id, param:l.variant_param!}));
/// Style for a returned map feature: its source key for variant layers, else its layer.
export function sourceFor(p:{layer:string; source_key:string}) {
 return mapSources.find(source=>source.id===p.source_key) ?? mapSources.find(source=>source.id===p.layer && !source.variant);
}
export const fieldLabels: Record<string,string> = registry.field_labels;
export const dateFields: string[] = registry.date_detail_fields;
export const officialLinks = registry.links;
export function sourceNotice(sourceKey?: string) {
 return sources.find(s=>s.key===sourceKey)?.notice;
}
