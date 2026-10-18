"""Actual5B3550 resource task completion→40D970→5B18C0(state2).

Explicit laboratory prefields represent the same VWorld, UI loading-screen and
task allocations. No resource loading, queue drain/barrier, full43F0A0, renderer
or activation success is manufactured. These are only actual ordered stores.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC

def main():
 rows=[];pins={}
 for image in ['original','v14']:
  m=Machine(image)
  for pc in PC:
   for ac,flag,taskstate in itertools.product([0,1,0xa5],[0,1],[0,1,9]):
    m.fpu(pc);m.actual={};m.record_actual=True
    world=m.alloc(0xb0);screen=m.alloc(0x20);task=m.alloc(0x20)
    m.w(0xb36538,world);m.w(0xb39690+0xe4,screen)
    m.b(world+0xac,ac);m.b(world+0xad,0x5a)
    for off,val in [(0x15,0x31),(0x16,0x32),(0x17,flag)]:m.b(screen+off,val)
    m.w(task+0x14,taskstate);m.w(task+0x18,0x1234abcd)
    m.call(0x5b3550,this=task)
    output=dict(world_ac=m.rb(world+0xac),world_ad=m.rb(world+0xad),screen=[m.rb(screen+x)for x in [0x15,0x16,0x17]],task_state=m.r(task+0x14),task_thread_word=m.r(task+0x18))
    assert output==dict(world_ac=0,world_ad=0x5a,screen=[0x31,0x32,1],task_state=2,task_thread_word=0x1234abcd)
    pins.update(m.actual);rows.append(dict(image=image,precision=pc,input=dict(world_ac=ac,screen17=flag,task_state=taskstate),output=output))
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 data=[]
 for va in [0xa4bba4+0x1c]:
  raw=PE0.range(va,4);assert raw==PE1.range(va,4);data.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions,data_pins=data)
 p=HERE/'native-view-resource-completion.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executed_pins=len(instructions),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
