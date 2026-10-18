"""Unchanged4D1350/565140/4CFCC0 on actual fresh Construction source inputs.

Inputs are declared direct-constructor laboratory boundaries, not an initialized
mission. The sole service seam is4B2E60 returning the captured SAME Settings Bool
cell. No action contributor, virtual parent getter or publication is stubbed.
Native ActorState registration/masks are reused from their sealed native proof.
"""
from pathlib import Path
import json,sys,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
REG=ROOT/'research/phase7-actors/native-actor-states.json'
def one(row,image):
 c=row['input'];m=nm.Machine(row['precision'])
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 actor=m.alloc(0x180);data=m.alloc(0x600);vt=m.alloc(0x200);component=m.alloc(0x30)
 m.w(actor,vt);m.w(actor+4,data);m.w(actor+8,c['flags']);m.w(actor+0x9c,component);m.w(component+4,actor)
 for i,v in enumerate(c['state']):m.w(actor+0x100+4*i,v)
 for i,v in enumerate(c['actions']):m.w(actor+0xf0+4*i,v)
 assert c['owner']==c['object_byte4c']==c['thing_wordc']==0
 for offset,entry in {0x34:0x4d1f90,0x100:0x4ced60,0x108:0x4d1e80,0x158:0x4ce980,0x15c:0x4ce9a0}.items():m.w(vt+offset,entry)
 m.w(data+0x4c0,c['upgrade_count']);m.wb(data+0x4e4,int(c['can_sell']));m.wb(data+0x4e5,int(c['can_raze']))
 sell=m.alloc(4);m.wb(sell,int(c['allow_sell_under_siege']));m.seam(0x4b2e60,lambda mm:mm.ret(sell,8))
 registration=json.loads(REG.read_text())['registration'];gd=m.alloc(0x400);array=m.alloc(42*4)
 m.w(0xb36530,gd);m.w(gd+0x2e4,array);m.w(gd+0x2e8,42);m.u.mem_write(0xb3ac40,struct.pack('<31Q',*registration['masks']))
 for row_ in registration['records']:
  index=row_['index'];state=m.alloc(0x40);m.w(array+index*4,state);m.w(state+0xc,index);m.w(state+0x2c,row_['group']);m.wb(state+0x30,row_['exclusive'])
 m.call(0x4d1350,actor)
 return dict(precision=row['precision'],input=c,actions=[m.r(actor+0xf0),m.r(actor+0xf4)]),m.visited
def main():
 rows=[];pins={}
 for row in json.loads((HERE/'actual-constructor-inputs.json').read_text())['construction_actions']:
  a,va=one(row,'original');b,vb=one(row,'v14');assert a==b==row,(a,b,row);assert va==vb
  for address,raw in va.items():assert raw==nm.original.range(address,len(raw))==nm.modified.range(address,len(raw));pins[address]=raw
  rows.append(a)
 assert rows,'actual Construction source input required'
 out=dict(schema=1,scope=__doc__,rows=rows,instructions=nm.instructions(pins),producer_sha256=nm.sha(Path(__file__).read_bytes()),registration_sha256=nm.sha(REG.read_bytes()))
 raw=(json.dumps(out,indent=2,sort_keys=True)+'\n').encode();path=HERE/'native-construction-actions.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw,'immutable Construction actions differs'
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,instructions=len(pins),sha256=nm.sha(raw))))
if __name__=='__main__':main()
