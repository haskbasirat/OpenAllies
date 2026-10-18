"""Source-backed terrain corpus and original load of sampled source tile records."""
from pathlib import Path
import json, struct, sys
from native_terrain import TerrainMachine, ROOT, OUT, W, H, original, modified
sys.path.insert(0,str(ROOT/'research/phase6-mapheader'))
from inspect_header import Reader, decode, read_chunks
SOURCE=ROOT.parent/'reconstruction/assets/original/Data'

def parse(data):
    r=Reader(data);version=r.u32();resolution=r.u32();biome=r.string()
    palette=[r.string()for _ in range(r.u32())];count_at=r.at;count=r.u32();stride=r.u32();start=r.at
    assert version in (6,7) and stride=={6:6,7:8}[version]
    records=[]
    for _ in range(count):
        b=r.take(stride)
        if version==6:
            index,flag,region,byte,_=struct.unpack('<BBHBB',b)
        else:
            index,region,word,flag,_=struct.unpack('<HHHBB',b);byte=word&255
        records.append((index,flag,region,byte))
    return version,resolution,biome,palette,records,start,r.at,count_at

def main():
    types_path=ROOT/'research/engine-baseline/terrain-variation-metadata.json'
    render_path=ROOT/'research/phase6-terrain/terrain-render-metadata.json'
    types=json.loads(types_path.read_text())['variations'];render=json.loads(render_path.read_text())
    source_pins={};maps={};m=TerrainMachine()
    svars=SOURCE/'Game/SVars.tgi';assert b'int MapGTileResolutionBits = 1' in svars.read_bytes()
    for path in sorted((SOURCE/'Maps').rglob('*.rmp')):
        chunks=read_chunks(path);raw=chunks['TERRAIN'][0];header=decode(chunks['MAP_HEADER'][0])
        version,resolution,biome,palette,records,start,end,count_at=parse(raw)
        assert resolution==1
        layout=m.layout(header['creator_size_bits'],1)
        assert layout['width']*layout['height']==len(records)
        def resolve(key):
            if key not in render['variants']:
                return {'id':key,'unresolved':True}
            r=render['variants'][key];t=types[r['canonical']]
            assert t['source_sha256']==r['sha256']
            p=SOURCE/r['path'];source_pins[r['path']]=H(p.read_bytes());assert source_pins[r['path']]==r['sha256']
            return {'id':key,'native_type':t['native_type'],'passability_bits':int(r['land'])|int(r['water'])<<1}
        defs=[resolve(key)for key in palette]
        initial=resolve(render['biome_defaults'][biome]['variation_id']);assert not initial.get('unresolved')
        cells=[];previous=initial;previous_index=None;missing=[]
        for index,flag,region,byte in records:
            d=defs[index]
            if d.get('unresolved'):
                d=previous;missing.append(len(cells))
            else:previous_index=index
            flags=(flag&~0x70)|(d['native_type']<<4)
            if flags&1:flags|=0x300
            else:flags=(flags&~0x300)|(0 if d['passability_bits']&1 else 0x100)|(0 if d['passability_bits']&2 else 0x200)
            cells.append((index,flags,byte,region,previous_index));previous=d
        w,h=layout['width'],layout['height']
        edge=[cells[y*w+x][1] for y in range(h) for x in range(w) if x in(0,w-1)or y in(0,h-1)]
        # Preserve original source-order carry semantics: the first nine cells,
        # not a reordered bag that would invent different missing-ID fallbacks.
        indices=list(range(9))
        sample=raw[:count_at]+W(len(indices),6 if version==6 else 8)+b''.join(raw[start+i*(6 if version==6 else 8):start+(i+1)*(6 if version==6 else 8)]for i in indices)
        actual=m.load(sample,3,3,1,defs,initial)
        for i,native in zip(indices,actual['cells']):
            index,flags,byte,region,resolved=cells[i]
            assert native=={'variation_index':resolved,'flags20':flags,'byte_2c':byte,'runtime_region_word':0},(path,i,native,cells[i])
        maps[path.relative_to(SOURCE).as_posix()]={
            'map_sha256':H(path.read_bytes()),'terrain_sha256':H(raw),'header_sha256':H(chunks['MAP_HEADER'][0]),
            'version':version,'layout':layout,'tile_count':len(records),'palette_count':len(palette),
            'tile_data_offset':start,'suffix_offset':end,'suffix_bytes':len(raw)-end,
            'flags_sha256':H(b''.join(W(c[1])for c in cells)),
            'cell_fields_sha256':H(b''.join(struct.pack('<HIBHI',*c[:4],0xffffffff if c[4] is None else c[4])for c in cells)),
            'fallback_count':len(missing),'missing_ordinals_sha256':H(b''.join(W(i)for i in missing)),
            'initial_variation_id':initial['id'],
            'blocked_outer_ring':all(f&0x301==0x301 for f in edge),
            'sampled_source_indices':indices,'native_sampled_cells':actual['cells'],
        }
    windows=[]
    for pc,size in sorted(m.visited.items()):
        b=original.range(pc,size);assert b==modified.range(pc,size)
        windows.append({'va':hex(pc),'bytes':b.hex()})
    report={'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),
        'resolution_source':{'path':'Game/SVars.tgi','sha256':H(svars.read_bytes()),'value':1},
        'types_metadata_sha256':H(types_path.read_bytes()),'passability_metadata_sha256':H(render_path.read_bytes()),
        'terrain_template':render['terrain_template'],'translation_source':render['translation_source'],
        'definition_sources':source_pins,'maps':maps,'native_executed_windows':windows,
        'scope':'All80 complete raw source layouts/tiles checked against metadata and original constructor layout. The first nine unchanged source-order tile records per map are replayed through original557AD0 tile loader in an explicitly reduced3x3 grid with original complete palette and authored biome default; this is720 native sample cells, not execution of the entire3million-tile corpus or auxiliary suffix. Missing active definitions use the original carried-variation fallback; commented-out historical translations are not reactivated.'}
    target=OUT/'native-real-map-terrain.json';target.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'maps':len(maps),'tiles':sum(r['tile_count']for r in maps.values()),'blocked_rings':sum(r['blocked_outer_ring']for r in maps.values()),'sha256':H(target.read_bytes())}))
if __name__=='__main__':main()
