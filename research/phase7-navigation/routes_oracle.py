"""Compose original loaded source topology with concrete uncached region routes."""
from pathlib import Path
import json,sys,struct
from topology_oracle import Machine,ROOT,OUT,H,read_chunks,original,modified
from unicorn.x86_const import *
sys.path.insert(0,str(ROOT/'research/phase6-spatial'))
from original_map_terrain import parse

def main():
 source=json.loads((OUT/'native-topology.json').read_text());rows=[];visited={}
 # Types are supplied only to the original post-load primary-island query;
 # cached topology, original rectangle links and route search all execute.
 metadata=json.loads((ROOT/'research/phase6-terrain/terrain-render-metadata.json').read_text());types=json.loads((ROOT/'research/engine-baseline/terrain-variation-metadata.json').read_text())['variations']
 for path in ['Maps/(2v2)Berlin.RMP','Maps/Campaign/axis01_crete.RMP','Maps/Campaign/axis11_midway.RMP']:
  row=source['maps'][path];raw=read_chunks(ROOT.parent/'reconstruction/assets/original/Data'/path)['TERRAIN'][0]
  _,_,biome,palette,records,_,end,_=parse(raw);previous=metadata['variants'][metadata['biome_defaults'][biome]['variation_id']];flags=[]
  for index,flag,_,_ in records:
   d=metadata['variants'].get(palette[index],previous);previous=d;f=(flag&~0x70)|(types[d['canonical']]['native_type']<<4)
   flags.append(f|0x300 if f&1 else f|(0 if d['land']else 0x100)|(0 if d['water']else 0x200))
  for precision in ['Single24','Double53','Extended64']:
   m=Machine(raw[end:],row['layout'],flags,precision);got=m.run()
   ready=m.alloc(0x100);m.write(0xb36528,ready);m.u.mem_write(ready+0x20,b'\1')
   solver=m.alloc(0x100);queue=solver+0x48;strategy=solver+0x34;heap=m.alloc(8192*16)
   for offset,value in [(0,0xa3680c),(4,m.graph),(8,queue),(12,strategy)]:m.write(solver+offset,value)
   m.write(strategy,0xa367b0);m.write(queue,0xa367f0);m.write(queue+4,heap);m.write(queue+12,8192);m.write(queue+16,8192)
   primary=got['primary_island'];members=[i for i,r in enumerate(got['regions'])if r['category']==primary]
   assert members
   pairs=[(members[0],members[-1]),(members[-1],members[0]),(members[len(members)//2],members[0]),(members[0],members[0])]
   outputs=[]
   for start,goal in pairs:
    output=m.alloc(8);m.call(0x53f320,solver,[output,start,goal,0]);status=m.u.reg_read(UC_X86_REG_EAX)
    route=[];entry=m.read(output)
    while entry:route.append(m.read(entry));entry=m.read(entry+8);assert len(route)<=len(got['regions'])
    states=[list(m.u.mem_read(m.read(m.read(m.graph+0x14)+i*4)+0x6d,2))for i in range(len(got['regions']))]
    outputs.append({'start':start,'goal':goal,'status':status,'route':route,'epoch':m.u.mem_read(m.graph+0x3c,1)[0],'node_states':states,'busy':bool(m.u.mem_read(0xb3d43d,1)[0]),'output_linked':bool(m.read(strategy+16))})
   rows.append({'path':path,'precision':precision,'routes':outputs});visited.update(m.executed);print('validated',path,precision,flush=True)
 pins=[]
 for a,n in sorted(visited.items()):
  raw=original.range(a,n);assert raw==modified.range(a,n);pins.append({'va':hex(a),'bytes':raw.hex()})
 report={'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'topology_oracle_sha256':H((OUT/'topology_oracle.py').read_bytes()),'topology_fixture_sha256':H((OUT/'native-topology.json').read_bytes()),'fixtures':rows,'native_basic_block_pins':pins,'scope':'Original loaded topology→adjacency→53F320 concrete flag0 routes,36 sequential searches across3 source maps and3 precision modes. Ready=true is explicit SAI fixture setup after adjacency; full engine startup and tile collision/Solve are not executed.'}
 (OUT/'native-map-routes.json').write_text(json.dumps(report,indent=2)+'\n')
if __name__=='__main__':main()
