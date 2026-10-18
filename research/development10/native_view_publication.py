"""Actual43D160/43AC20/43C420 Thing+C view publication and source getters.

Original source vtables and real45D3F0/45DAF0 read supplied descriptor+20 and
ActorData mask174. View factory dispatch executes natively; selected concrete
view constructors and virtual Init/Delete are named service leaves, not proved
Gamebryo/backend implementations. Zero view type returns without reading or
writing the old Thing+C. Failed Init deletes the supplied view then publishes
NULL; successful Init may register flag20 view before publication. Constructor
or Init cutoffs preserve old Thing+C. These are bounded laboratory owners.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/development10/creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_EIP
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
CTORS={2:0x455050,3:0x453f10,4:0x44fdb0,5:0x4505d0,6:0x44a0a0,7:0x446e30,8:0x44d760,9:0x449850,10:0x446420,11:0x446670,12:0x43aa20,13:0x44cce0,14:0x44b410,15:0x44c3b0,16:0x44ce70,17:0x44ad10,18:0x445530,19:0x447be0,20:0x446c60,21:0x44e4b0,22:0x44efc0,23:0x45a820,24:0x45c8b0,25:0x93af90,26:0x93ab40}
SOURCES={'Thing':0xa20770,'Object':0xa13318,'Actor':0xa05540,'Camera':0xa13448,'Doodad':0xa0f6e0}
INIT=0x7700b00;DELETE=INIT+0x10
class Stop(Exception):pass
class Publication(Machine):
 def __init__(self,image):
  self.pub_ready=False;super().__init__(image)
  self.u.hook_add(UC_HOOK_MEM_READ,self.read);self.u.hook_add(UC_HOOK_MEM_WRITE,self.write)
 def read(self,u,access,address,size,value,user):
  if self.pub_ready and self.forbid_mask and address<=self.data+0x174<address+size:raise AssertionError('nonActor mask read')
 def write(self,u,access,address,size,value,user):
  if self.pub_ready and address<=self.thing+0xc<address+size:self.writes.append('view'if value==self.view else value)
 def hook(self,u,pc,size,user):
  if self.pub_ready:
   if pc in CTORS.values():
    assert pc==CTORS[self.selected] and self.arg()==self.data and self.arg(1)==self.thing
    self.events.append('ctor:'+hex(pc))
    if self.cutoff=='ctor':raise Stop()
    self.view=self.ecx();vt=self.alloc(0xd0);self.w(self.view,vt);self.w(vt+0xc8,INIT);self.w(vt+0xc4,DELETE);self.w(self.view+0x54,self.view_flags)
    self.ret(8,self.view);return
   if pc==INIT:
    assert self.ecx()==self.view;self.events.append('init')
    if self.cutoff=='init':raise Stop()
    self.ret(value=int(self.accept));return
   if pc==DELETE:
    assert self.ecx()==self.view and self.arg()==1;self.events.append('delete');self.ret(4);return
  super().hook(u,pc,size,user)

def run(m,pc,c):
 m.pub_ready=False;m.fpu(pc);m.actual={};m.record_actual=True;m.events=[];m.writes=[]
 m.thing=m.alloc(0x40);m.data=m.alloc(0x600);descriptor=m.alloc(0x30);world=m.alloc(0x400)
 m.w(m.thing+4,m.data);m.w(m.thing+0xc,0x12345678);m.w(m.data,SOURCES[c['source']]);m.w(m.data+0x18,descriptor);m.w(descriptor+0x20,c['type']);m.w(m.data+0x174,c.get('mask',0))
 m.forbid_mask=c['source']!='Actor';m.selected=4 if c['source']=='Actor'and c.get('mask',0)&4 else c['type']
 m.cutoff=c.get('cutoff');m.accept=c.get('accept',True);m.view_flags=(1<<20)if c.get('updates')else 0;m.view=0
 m.pub_ready=True;error=None
 try:m.call(0x43d160,[m.thing],this=world,callee_pop=4)
 except Stop:error=m.events[-1]
 nodes=[];p=m.r(world+0x9c)
 while p:nodes.append('view'if m.r(p)==m.view else'unknown');p=m.r(p+4)
 value=m.r(m.thing+0xc)
 return dict(selected=m.selected,error=error,events=m.events,writes=m.writes,link='view'if m.view and value==m.view else value,updates=nodes,tail_view=bool(m.r(world+0xa0))),m.actual.copy()

def main():
 cases=[dict(source='Actor',type=t,mask=0)for t in range(2,27)]
 cases +=[dict(source=s,type=t,mask=64)for s,t in [('Thing',0),('Actor',0),('Object',2),('Camera',22),('Doodad',2)]]
 cases +=[dict(source='Actor',type=0,mask=4),dict(source='Actor',type=3,mask=4)]
 cases +=[dict(source='Actor',type=3,mask=0,accept=a,updates=u)for a in [False,True]for u in [False,True]]
 cases +=[dict(source='Actor',type=3,mask=0,cutoff=c)for c in ['ctor','init']]
 rows=[];pins={}
 for image in ['original','v14']:
  m=Publication(image)
  for pc in PC:
   for c in cases:
    output,visited=run(m,pc,c);pins.update(visited);rows.append(dict(image=image,precision=pc,input=c,output=output))
 assert [{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']==[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14']
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 tables=[]
 for name,va in SOURCES.items():
  raw=PE0.range(va+0x20,4);assert raw==PE1.range(va+0x20,4);tables.append(dict(source=name,va=hex(va+0x20),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions,source_getter_slots=tables)
 p=HERE/'native-view-publication.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executed_pins=len(instructions),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
