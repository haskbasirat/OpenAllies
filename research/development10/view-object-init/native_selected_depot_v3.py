"""Actual paired GBRDepotOil raw source fields through native numeric children.

Executes45CAB0 composition,878630/878790 geometry bounds and5A96B2 selection
arithmetic with the paired raw NIF projection's exact source words. Shadow-tag
classification comes from the separately audited source-node mapping; graph
traversal is explicit input transport. This is NOT execution of whole87AA10,
source NiStream parser, controller updates, graph optimization or ResourceTask.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from native_model_spheres import run as scalar
I=[0x3f800000,0,0,0,0x3f800000,0,0,0,0x3f800000,0,0,0,0x3f800000]

def main():
 index=json.loads((ROOT/'assets-cache/model-native-graphs/index.json').read_text());source='Buildings/GBR/GBRDepotOil/GBRDepotOil.nif'
 key,entry=next((k,v)for k,v in index['graphs'].items()if v['source']==source);raw=(ROOT/'assets-cache/model-native-graphs'/f'{key}.json').read_bytes();assert hashlib.sha256(raw).hexdigest()==entry['sha256'];d=json.loads(raw);assert d['schema']==2
 rows=[];pins={}
 for image in ['original','v14']:
  m=Machine(image)
  for pc in PC:
   m.fpu(pc);m.actual={};m.record_actual=True;world=[]
   for node in d['nodes']:
    if node['parent']is None:world.append(node['local']);continue
    parent=m.alloc(52);local=m.alloc(52);out=m.alloc(52)
    for i,w in enumerate(world[node['parent']]):m.w(parent+4*i,w)
    for i,w in enumerate(node['local']):m.w(local+4*i,w)
    m.call(0x45cab0,[out,local],this=parent,callee_pop=8);world.append([m.r(out+4*i)for i in range(13)])
   radial=m.alloc(4);low=m.alloc(4);high=m.alloc(4);footprint=m.alloc(12)
   m.w(low,0x7f7fffff);m.w(high,0xff7fffff)
   geometries=[]
   for i,node in enumerate(d['nodes']):
    if not node['vertices']:continue
    assert node['kind']in ['NiTriShape','NiTriStrips']and not node['skin']
    g=m.alloc(0x110);data=m.alloc(0x40);vt=m.alloc(0x50);dvt=m.alloc(0x50);stub=m.alloc(0x20)
    m.u.mem_write(stub,bytes.fromhex('668b410cc3'));m.w(dvt+0x38,stub);m.w(data,dvt);m.w(data+0xc,len(node['vertices']));array=m.alloc(len(node['vertices'])*12)
    for j,v in enumerate(node['vertices']):
     for k,w in enumerate(v):m.w(array+j*12+k*4,w)
    m.w(data+0x20,array);m.w(g+0xcc,data);m.w(g+0x20,node['flags']);m.w(g,vt)
    m.u.mem_write(stub+8,b'\xb8'+(0xb43cf0).to_bytes(4,'little')+b'\xc3');m.w(vt+4,stub+8)
    for j,w in enumerate(world[i]):m.w(g+0x6c+4*j,w)
    if not node['shadow']:m.call(0x878630,[g,radial,low,high])
    geometries.append((i,g))
   for i,g in geometries:
    assert 'Eq'not in d['nodes'][i]['tags'];m.call(0x878790,[g,footprint,footprint+4,m.r(low)])
   result=dict(radial=m.r(radial),min_z=m.r(low),max_z=m.r(high),footprint=[m.r(footprint+4*i)for i in range(3)])
   pins.update(m.actual)
   selection=next(i for i,n in enumerate(d['nodes'])if n['name']=='SelectionBox')
   cuboid,actual=scalar(m,pc,'selection',d['nodes'][selection]['vertices'],world[selection]);pins.update(actual);result['cuboid']=cuboid
   rows.append(dict(image=image,precision=pc,output=result))
 assert [{k:v for k,v in r.items()if k!='image'}for r in rows[:3]]==[{k:v for k,v in r.items()if k!='image'}for r in rows[3:]]
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   b=PE0.range(va,size);assert b==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=b.hex()))
 doc=dict(scope=__doc__,source=source,source_sha256=key,projection_sha256=entry['sha256'],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions)
 out=HERE/'native-selected-depot-oil-v3.json'
 if '--write'in sys.argv:out.open('x').write(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest(),outputs=[r['output']for r in rows[:3]])))
if __name__=='__main__':main()

