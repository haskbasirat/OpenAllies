"""Actual451F90 Actor /44F320 Organization Init suffix with same UI RNG.

The completed457A40 base is an explicit supplied child, not a claim of resource
or common Init completion. Runtime B4/108, source38/260,451F00 palette copy and
85D840 execute original bytes. Positive Track creation stops BEFORE42D7B0;
allocator transport is inherited and no positive Track or allocator parity is
claimed. Native owner RGB/AVar30 and pose words are explicit laboratory inputs.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
class Stop(Exception):pass
class ActorInit(Machine):
 def hook(self,u,pc,size,user):
  if getattr(self,'armed',False):
   if pc==0x457a40:
    assert self.ecx()==self.view
    self.events.append('base457A40')
    self.ret(0,0x87654300|self.c['base']);return
   if pc==0x42d7b0:
    self.events.append('track42D7B0');raise Stop('42D7B0')
  super().hook(u,pc,size,user)
def run(m,precision,c):
 m.armed=False;m.fpu(precision);m.actual={};m.record_actual=True;m.events=[];m.c=c
 m.view=m.alloc(0x174);m.u.mem_write(m.view,b'\xa5'*0x174)
 m.w(m.view,0xa01ef0 if c['organization']else 0xa02270)
 m.data=m.alloc(0x600);m.w(m.data,0xa05540 if c['source_actor']else 0xa13318)
 m.w(m.data+0x260,c['tracks']);m.w(m.view+0x10,m.data)
 pose=[0x3f800001,0x80000000,0x47ffffff,0xbf19999a,0x3f4ccccd,0]
 for i,word in enumerate(pose):m.w(m.view+0x38+4*i,word)
 m.thing=0
 owner=m.alloc(0x340);color=m.alloc(0x30);m.w(owner+0x1e8,color)
 owner_rgb=[0x3dcccccd,0x3f000000,0x3f800000]
 for i,v in enumerate(owner_rgb):m.w(color+0x18+4*i,v)
 if c['thing']!='None':
  m.thing=m.alloc(0x350);m.w(m.thing,{'Object':0xa2a2a0,'Actor':0xa273f0,'Kingdom':0xa299d8}[c['thing']])
  m.w(m.thing+0xe8,owner if c['owner']else 0)
 m.w(m.view+0x14,m.thing)
 avars=m.alloc(0x100);avar=m.alloc(0x20);m.w(0xb39990,avars);m.w(avars+0x78,avar)
 for i,v in enumerate([0x3e800000,0x3f400000,0x3f000001]):m.w(avar+4+4*i,v)
 app=m.alloc(0x200);m.w(0xb3653c,app);m.w(app+0xd4,c['seed'])
 m.armed=True;result=None;error=None
 try:result=m.call(0x44f320 if c['organization']else 0x451f90,this=m.view)&0xff
 except Stop as e:error=str(e)
 fields={hex(o):m.r(m.view+o)for o in [0xc8,0xcc,0xd0,0xd4,0xd8,0xdc,0xe0,0xe4,0xe8,0xec,0xf0,0xf4,0xf8,0xfc,0x118,0x11c,0x120,0x124,0x144,0x148,0x14c]}
 return dict(result=result,error=error,fields=fields,seed=m.r(app+0xd4),events=m.events),m.actual.copy()
def main():
 rows=[];pins={};cases=[]
 for organization in [False,True]:
  for thing in ['None','Object','Actor','Kingdom']:
   for owner in [False,True]:
    for source_actor,tracks in [(False,9),(True,0),(True,-1),(True,1)]:
     cases.append(dict(organization=organization,thing=thing,owner=owner,source_actor=source_actor,tracks=tracks,seed=0x8e4b12a5,base=1))
  cases.append(dict(organization=organization,thing='Actor',owner=True,source_actor=True,tracks=1,seed=0x12345678,base=0))
  for seed in [0,1,0xffffffff,0x12345678,0x9182a3b4]:
   cases.append(dict(organization=organization,thing='Actor',owner=True,source_actor=True,tracks=0,seed=seed,base=1))
 for image in ['original','v14']:
  m=ActorInit(image)
  for precision in PC:
   for c in cases:
    output,actual=run(m,precision,c);pins.update(actual);rows.append(dict(image=image,precision=precision,input=c,output=output))
 assert [{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']==[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14']
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 data_pins=[]
 for va,size in [(0x9fda98,8),(0x9fdaa0,8)]+[(vt+o,4)for vt in [0xa02270,0xa01ef0]for o in [0x70,0xc8]]:
  raw=PE0.range(va,size);assert raw==PE1.range(va,size);data_pins.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions,data_pins=data_pins)
 out=HERE/'native-actor-init.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
