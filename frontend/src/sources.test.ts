import { test, expect } from 'vitest';
import { defaultMapSources, mapLayerGroups, sourceFor, variantParams } from './sources';

test('map controls are derived from the source registry', ()=>{
 expect(mapLayerGroups.map(g=>g.id)).toEqual(['streets','snow','collection','recreation','watersheds','parking','places']);
 const snow = mapLayerGroups.find(g=>g.id==='snow')!.children;
 expect(snow.map(c=>c.label)).toEqual(['Tertiary Route','Second Run Route','First Run Route','Snow Emergency Route']);
 expect(snow.every(c=>c.id.includes('/') && c.variant)).toBe(true);
 expect(defaultMapSources).toEqual(['streets',...snow.map(c=>c.id),'leaves']);
 expect(variantParams).toEqual([{layer:'snow',param:'snow_types'}]);
 expect(sourceFor({layer:'snow',source_key:snow[2].id})?.label).toBe('First Run Route');
 expect(sourceFor({layer:'parks',source_key:'Anything/0'})?.id).toBe('parks');
});
