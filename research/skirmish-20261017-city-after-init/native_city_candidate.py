"""Whole564300 and5640A0 rejection prefixes, cutoff before4549C0.

No eligibility result is supplied by a seam. Actual Actor/Kingdom owner108
getter runs twice where native requires it. Major candidates stop at the
first unbound redirect child; no list append/schedule completion is claimed.
"""
from pathlib import Path
import argparse,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
sha=lambda b:hashlib.sha256(b).hexdigest()
def run(c,image,pc):
 m=Machine(pc)
 old=(original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 city,city_actor,target,kingdom=[m.alloc(n)for n in[0x24,0x200,0x200,0x300]]
 target=0 if c['target']=='null' else city_actor if c['target']=='self' else target
 m.w(city+4,city_actor);m.w(city+0x18,0xdeadc000);m.w(city+0x1c,0xdeadc004)
 if target:
  m.w(target,0xa273f0 if c['class']==3 else 0xa299d8);m.w(target+8,c['flags'])
  m.w(target+0xe8,kingdom if c['owner'] else 0)
 m.w(kingdom+0x190,c['nonmajor'])
 reads=[];writes=[];terminal=['rejected']
 def access(u,access,address,size,value,data):
  if address in[city+0x18,city+0x1c]:raise AssertionError('list read before eligibility completed')
  if target and address==target+8:reads.append('flags')
  if target and address==target+0xe8:reads.append('owner')
  if address==kingdom+0x190:reads.append('nonmajor')
 def write(u,access,address,size,value,data):
  if city<=address<city+0x24:writes.append([address-city,size,value])
 m.u.hook_add(UC_HOOK_MEM_READ,access);m.u.hook_add(UC_HOOK_MEM_WRITE,write)
 def cutoff(_):terminal[0]='redirect_required';m.u.emu_stop()
 m.seam(0x4549c0,cutoff)
 try:m.call(0x564300,city,[target])
 except AssertionError:
  if terminal[0]!='redirect_required':raise
 assert not writes
 return dict(terminal=terminal[0],reads=reads,writes=writes),m.visited
def main():
 p=argparse.ArgumentParser();p.add_argument('--create',action='store_true');a=p.parse_args()
 bases=[dict(target='null',flags=0,owner=True,nonmajor=0),dict(target='self',flags=0,owner=True,nonmajor=0),dict(target='other',flags=1,owner=True,nonmajor=0),dict(target='other',flags=0xffffffff,owner=True,nonmajor=0),dict(target='other',flags=0,owner=False,nonmajor=0),dict(target='other',flags=0,owner=True,nonmajor=1),dict(target='other',flags=0,owner=True,nonmajor=0x80000000),dict(target='other',flags=0,owner=True,nonmajor=0)]
 rows=[];pins={}
 for name,image in[('original',original),('v14',modified)]:
  for pc in['Single24','Double53','Extended64']:
   for cls in[3,4]:
    for b in bases:
     c=dict(b,class_=cls);c['class']=c.pop('class_');out,visited=run(c,image,pc);pins.update(visited);rows.append(dict(image=name,precision=pc,input=c,native=out))
 out=dict(scope=__doc__,original_sha256=sha(original.b),v14_sha256=sha(modified.b),rows=rows,instructions=[])
 for address,raw in sorted(pins.items()):
  assert raw==original.range(address,len(raw))==modified.range(address,len(raw)),hex(address)
  out['instructions'].append(dict(address=hex(address),bytes=raw.hex()))
 path=HERE/'native-city-candidate.json'
 if a.create:
  with path.open('x',encoding='utf-8')as f:json.dump(out,f,indent=2);f.write('\n')
 else:assert out==json.loads(path.read_text(encoding='utf-8'))
 print(json.dumps(dict(rows=len(rows),pins=len(pins),fixture_sha256=sha(path.read_bytes()))))
if __name__=='__main__':main()
