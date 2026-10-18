"""Original7ABC30 topology preparation; resolved depth and alloc/I/O are seams."""
from pathlib import Path
import json,sys,struct,hashlib
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase7-navigation'))
from topology_oracle import Machine,H,original,modified,read_chunks
from unicorn.x86_const import *
def prepare(data,layout,flags,depth):
 m=Machine(data,layout,flags);m.call(0x7aa9b0,m.graph,[m.stream-0x3c,m.grid]);consumed=m.r.at
 setting=m.alloc(4);m.write(setting,depth);m.write(0xb40e44,setting);m.write(0xb40e48,1)
 m.call(0x7abc30,m.graph);count=m.read(m.graph+0x18);areas=[];bottlenecks=[];neighbors=[]
 for i in range(count):
  node=m.read(m.read(m.graph+0x14)+i*4);areas.append(m.read(node+12));bottlenecks.append(bool(m.u.mem_read(node+0x6c,1)[0]))
  neighbors.append([m.read(m.read(m.read(node+0x74)+j*4))for j in range(m.read(node+0x70))])
 cache=m.read(m.graph+0x40);lengths=[];raw=b''
 for i in range(m.read(cache)):
  row=m.read(cache+4)+i*8;length=m.read(row);lengths.append(length);raw+=bytes(m.u.mem_read(m.read(row+4),length))
 return {'consumed':consumed,'prepared':bool(m.u.mem_read(m.graph+5,1)[0]),'areas':areas,'bottlenecks':bottlenecks,'neighbors':neighbors,'cache_lengths':lengths,'cache_sha256':H(raw)},m.executed
def main():
 small=json.loads((ROOT/'research/phase7-navigation/synthetic-topology.json').read_text());fixtures=[];visited={}
 for row in small['fixtures']:
  if row['precision']!='Double53':continue
  for depth in [-1,0,1,2,5,9]:
   result,pins=prepare(bytes.fromhex(row['input_hex']),row['layout'],row['flags20'],depth);visited.update(pins)
   fixtures.append({'name':row['name']+'_depth'+str(depth),'depth':depth,'input_hex':row['input_hex'],'layout':row['layout'],'world_version':row['world_version'],'native':result})
 # Include source region topology while preserving the actual static tile flags.
 sys.path.insert(0,str(ROOT/'research/phase6-spatial'));from original_map_terrain import parse
 proof=json.loads((ROOT/'research/phase7-navigation/native-topology.json').read_text())
 meta=json.loads((ROOT/'research/phase6-terrain/terrain-render-metadata.json').read_text());types=json.loads((ROOT/'research/engine-baseline/terrain-variation-metadata.json').read_text())['variations'];maps={}
 for path in ['Maps/(2v2)Berlin.RMP','Maps/Campaign/axis01_crete.RMP','Maps/Campaign/axis11_midway.RMP']:
  row=proof['maps'][path];raw=read_chunks(ROOT.parent/'reconstruction/assets/original/Data'/path)['TERRAIN'][0]
  _,_,biome,palette,records,_,end,_=parse(raw);previous=meta['variants'][meta['biome_defaults'][biome]['variation_id']];flags=[]
  for index,flag,_,_ in records:
   d=meta['variants'].get(palette[index],previous);previous=d;f=(flag&~0x70)|(types[d['canonical']]['native_type']<<4)
   flags.append(f|0x300 if f&1 else f|(0 if d['land']else 0x100)|(0 if d['water']else 0x200))
  result,pins=prepare(raw[end:],row['layout'],flags,5);visited.update(pins);maps[path]={'terrain_sha256':H(raw),'suffix_offset':end,'depth':5,'layout':row['layout'],'world_version':row['world_version'],'native':result};print('validated',path,flush=True)
 blockpins=[]
 for a,n in sorted(visited.items()):
  raw=original.range(a,n);assert raw==modified.range(a,n);blockpins.append({'va':hex(a),'bytes':raw.hex()})
 svars=ROOT.parent/'reconstruction/assets/original/Data/Game/SVars.tgi';assert b'StaticMapBottleneckDetectionDepth\t= 5' in svars.read_bytes()
 doc={'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'topology_oracle_sha256':H((ROOT/'research/phase7-navigation/topology_oracle.py').read_bytes()),'original_sha256':H(original.b),'v14_sha256':H(modified.b),'source_svar':{'path':str(svars),'sha256':H(svars.read_bytes()),'depth':5},'fixtures':fixtures,'maps':maps,'native_basic_block_pins':blockpins,
 'scope':'Fresh original7AA9B0 load then complete7ABC30 adjacency/areas/bottlenecks/cache initialization. Required resolved SVar cache is supplied explicitly. Allocation/free/version/byte transport remain seams. No7A0D40 engine readiness or later flag1 cache filling claim.'}
 (OUT/'native-preparation.json').write_text(json.dumps(doc,indent=2)+'\n');print({'passed':True,'small_cases':len(fixtures),'maps':len(maps)})
if __name__=='__main__':main()
