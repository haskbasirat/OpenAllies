"""Native object-class B4, whole4EE700 and4EA200 cached-coordinate leaves.

All6 known saved Object class vtables execute actual leaves; radius virtual4
and shape virtualC are NOT replaced. Native32-bit tile pointers are normalized
only for these nonoverflowing coordinate cases, matching the growable logical
tile-index interface. Finite geometry cases only. No zone-effect body claim.
"""
from pathlib import Path
import argparse,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified,f32
from unicorn import UC_HOOK_MEM_READ
sha=lambda b:hashlib.sha256(b).hexdigest()
TABLES={2:0xa2a2a0,3:0xa273f0,4:0xa299d8,6:0xa28a00,7:0xa28b88,9:0xa28e90}
def run(image,pc,c):
 m=Machine(pc)
 old=(original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 actor,definition,world,map_,tiles=[m.alloc(n)for n in[0x200,0x600,0x100,0x100,4096]]
 table=TABLES[c['class']];m.w(actor,table);m.w(actor+4,definition);m.w(actor+8,c['flags'])
 m.w(definition+0x3c,c['radius']);m.w(actor+0x44,c['xy'][0]);m.w(actor+0x48,c['xy'][1])
 m.w(0xb36534,world if c['flags']&0x20000400==0x20000400 else 0xdeadc000)
 m.w(world+0x30,map_);m.w(map_+8,c['cell']);m.w(map_+0x1c,c['width']);m.w(map_+0x28,tiles)
 reads=[]
 def memory(u,access,address,size,value,data):
  if address==map_+8:reads.append('cell')
  if address==definition+0x3c:reads.append('radius')
 m.u.hook_add(UC_HOOK_MEM_READ,memory)
 b4=m.call(m.r(table+0xb4),actor)&255
 multi=m.call(0x4ee700,actor)&255
 assert reads==(['cell','radius']if c['flags']&0x20000400==0x20000400 else [])
 m.w(0xb36534,world)
 tile=m.call(0x4ea200,actor)
 delta=(tile-tiles)&0xffffffff
 if delta&0x80000000:delta-=1<<32
 assert delta%48==0
 assert delta//48==c['width']*c['xy'][1]+c['xy'][0]
 return dict(b4=b4!=0,multi=multi!=0,reads=reads,flat=delta//48),m.visited
def main():
 p=argparse.ArgumentParser();p.add_argument('--create',action='store_true');a=p.parse_args()
 rows=[];pins={};tables=[]
 for cls,table in TABLES.items():
  for offset in[4,12,0xb4]:
   raw=original.range(table+offset,4);assert raw==modified.range(table+offset,4)
   tables.append(dict(class_=cls,address=hex(table+offset),bytes=raw.hex()))
 cases=[]
 for flags,radius,cell in [(0,f32(10),f32(8)),(0x400,f32(10),f32(8)),(0x20000000,f32(10),f32(8)),(0x20000400,f32(1.999),f32(8)),(0x20000400,f32(2),f32(8)),(0x20000400,f32(3),f32(8.9)),(0x20000400,f32(-1),f32(8))]:
  for xy in [[0,0],[4,2],[-1,0],[2,-1]]:
   cases.append(dict(flags=flags,radius=radius,cell=cell,width=7,xy=xy))
 for name,image in[('original',original),('v14',modified)]:
  for pc in['Single24','Double53','Extended64']:
   for cls in TABLES:
    for base in cases:
     c=dict(base,class_=cls);c['class']=c.pop('class_')
     out,visited=run(image,pc,c);pins.update(visited)
     rows.append(dict(image=name,precision=pc,input=c,native=out))
 out=dict(scope=__doc__,original_sha256=sha(original.b),v14_sha256=sha(modified.b),tables=tables,rows=rows,instructions=[])
 for address,raw in sorted(pins.items()):
  assert raw==original.range(address,len(raw))==modified.range(address,len(raw)),hex(address)
  out['instructions'].append(dict(address=hex(address),bytes=raw.hex()))
 path=HERE/'native-actor-zone-reads.json'
 if a.create:
  with path.open('x',encoding='utf-8')as f:json.dump(out,f,indent=2);f.write('\n')
 else:assert out==json.loads(path.read_text(encoding='utf-8'))
 print(json.dumps(dict(rows=len(rows),pins=len(pins),fixture_sha256=sha(path.read_bytes()))))
if __name__=='__main__':main()
