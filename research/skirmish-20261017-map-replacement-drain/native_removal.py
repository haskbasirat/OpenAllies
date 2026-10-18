"""Native removal list prefix, signed memory getter and removal caller.

4F1430 executes actual first-match head/tail stores with a named free seam.
4F5370/4F4C40 execute actual control and GTID guards; virtual category/source
boolean getters are explicit laboratory leaves.4EC290 executes actual76AC90
against Session+F8, with named visibility and allocation/refresh/clear children.
4F53D0 executes its actual caller including8C count getter and retained stores;
uninit, dispatcher unlink, destructor and classification prefix are named
children. This does not prove those children or full destruction/creation.
"""
from pathlib import Path
import sys,json,struct,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/development10/creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_EIP
class Stop(Exception):pass
class Removal(Machine):
 def __init__(self,image):
  self.removal_ready=False;super().__init__(image)
 def event(self,name,**fields):
  self.trace.append(dict(call=name,**fields))
  if self.cutoff==name:raise Stop()
 def hook(self,u,pc,size,user):
  if self.removal_ready:
   if pc in self.leaves:
    name,value=self.leaves[pc];self.event(name,value=value);self.ret(value=int(value));return
   if pc==0x6f7430:
    self.event('free',value=self.node_index.get(self.arg(),'other'));self.ret();return
   if pc==0x4ea350:
    assert self.ecx()==self.actor;self.event('visibility',value=self.visible);self.ret(value=int(self.visible));return
   if pc in [0x4ea3e0,0x4ebc70,0x4ebc30]:
    assert self.ecx()==self.actor;self.event({0x4ea3e0:'allocate',0x4ebc70:'refresh',0x4ebc30:'clear'}[pc]);self.ret();return
   if self.mode=='caller':
    if pc==0x4f5370:
     assert self.ecx()==self.world and self.arg()==self.actor;self.event('prefix');self.ret(4);return
    if pc==self.uninit:
     assert self.ecx()==self.actor;self.event('uninit',retain=self.arg());self.ret(4);return
    if pc==0x50c1b0:
     assert self.ecx()==self.dispatcher and self.arg()==self.actor;self.event('dispatcher_unlink');self.ret(4);return
    if pc==self.destructor:
     assert self.ecx()==self.actor and self.arg()==1;self.event('destructor',delete=1);self.ret(4);return
   if self.mode=='actor_prefix':
    if pc==0x4d37e0 and self.r(self.actor+0x130):self.event('timed_list');raise Stop()
    if pc in [0x7a0a40,0x514ce0,0x4d19f0,0x4dff30,0x4e44e0,0x589cc0,0x4ea6f0]:
     self.event({0x7a0a40:'sai_category',0x514ce0:'zone_uninit',0x4d19f0:'owner160',0x4dff30:'owner164',0x4e44e0:'kingdom_remove',0x589cc0:'settlement_uninit',0x4ea6f0:'base_uninit'}[pc]);raise Stop()
    if pc==self.other12c:self.event('other12c');raise Stop()
    if pc==0x4cf390 and self.components_present:self.event('components');raise Stop()
  super().hook(u,pc,size,user)
 def begin(self,pc,mode,case):
  self.removal_ready=False;self.fpu(pc);self.mode=mode;self.trace=[];self.cutoff=case.get('cutoff');self.leaves={};self.node_index={};self.actual={};self.record_actual=True
  self.actor=self.alloc(0x400);self.world=self.alloc(0x400);self.session=self.alloc(0x200);self.dispatcher=self.alloc(0x20)
  self.w(0xb36534,self.world);self.w(0xb36550,self.session);self.w(self.world+0x130,self.dispatcher)
  self.w(self.session+0xf8,case.get('session_phase',3));self.w(self.world+0xd8,case.get('world_phase',0))
  self.visible=case.get('visible',False);self.w(self.actor+0x58,case.get('count',0));self.w(self.actor+0x5c,case.get('allocation',0))
  self.w(self.actor+8,case.get('flags',0x20000800));self.w(self.actor+0x14,0x10007)
  self.vt=self.alloc(0x200);self.w(self.actor,self.vt)
  self.w(self.vt+0x90,0x4ec290);self.w(self.vt+0x8c,0x4ea3d0)
  self.uninit=self.alloc(16);self.destructor=self.alloc(16)
  self.w(self.vt+0xf0,self.uninit);self.w(self.vt+0xe0,self.destructor)
  self.node_index={};self.removal_ready=True
 def leaf(self,name,value):
  p=self.alloc(16);self.leaves[p]=(name,value);return p
 def linked(self,owner,values,kind):
  nodes=[self.alloc(8)for _ in values]
  for i,(p,v)in enumerate(zip(nodes,values)):
   self.w(p,v);self.w(p+4,nodes[i+1]if i+1<len(nodes)else 0);self.node_index[p]=[kind,i]
  self.w(owner,nodes[0]if nodes else 0);self.w(owner+4,nodes[-1]if nodes else 0)
  return nodes
 def readlist(self,owner,translate=lambda x:x):
  p=self.r(owner);out=[];seen=set()
  while p:
   assert p not in seen;seen.add(p);out.append(translate(self.r(p)));p=self.r(p+4)
  tail=self.r(owner+4)
  return dict(values=out,tail=None if not tail else translate(self.r(tail)))
def run_list(m,pc,c):
 m.begin(pc,'list',c);owner=m.alloc(8);m.linked(owner,c['values'],'list');q=m.alloc(4);m.w(q,c['query'])
 returned=m.call(0x4f1430,[q],this=owner,callee_pop=4)
 return dict(returned=returned,list=m.readlist(owner),trace=m.trace),m.actual.copy()
def run_prefix(m,pc,c):
 m.begin(pc,'prefix',c)
 for offset,name in [(0xb0,'b0'),(0xb4,'b4'),(0xbc,'bc'),(0xc0,'c0')]:m.w(m.vt+offset,m.leaf(name,c.get(name,False)))
 definition=m.alloc(0x600);dvt=m.alloc(0x50);m.w(m.actor+4,definition);m.w(definition,dvt);m.w(dvt+0x24,m.leaf('source34',c.get('source34',False)))
 m.w(m.actor+0xd4,1 if c.get('city',False)else 0)
 other=m.alloc(0x400);od=m.alloc(0x600);ovt=m.alloc(0x50);m.w(other+4,od);m.w(od,ovt);m.w(ovt+0x24,m.leaf('other34',c.get('other34',False)))
 registry=m.alloc(0x60010);m.w(0xb364c0,registry);m.w(m.actor+0x158,0x20008)
 generation=3 if c.get('stale',False)else 2
 m.u.mem_write(registry+4+7*2,struct.pack('<H',1));m.w(registry+0x20004+7*4,m.actor)
 m.u.mem_write(registry+4+8*2,struct.pack('<H',generation));m.w(registry+0x20004+8*4,other if c.get('other',False)else 0);m.w(other+0x14,(generation<<16)|8)
 for slot in [7,8]:
  pointer=m.r(registry+0x20004+slot*4)
  if pointer:assert m.r(pointer+0x14)==(int.from_bytes(m.u.mem_read(registry+4+slot*2,2),'little')<<16)|slot
 m.linked(m.world+0x260,c.get('cities',[99,0x10007,0x10007]),'cities')
 old=m.actor;otherptr=m.alloc(4)
 for offset,kind in [(0x188,'areas'),(0x178,'cameras')]:m.linked(m.world+offset,[otherptr,old,old],kind)
 m.call(0x4f5370,[old],this=m.world,callee_pop=4)
 translate=lambda x:'old'if x==old else'other'
 return dict(cities=m.readlist(m.world+0x260),areas=m.readlist(m.world+0x188,translate),cameras=m.readlist(m.world+0x178,translate),trace=m.trace),m.actual.copy()
def run_intrusive(m,pc,c):
 m.begin(pc,'intrusive',c);head=m.world+8;nodes=[m.alloc(12)for _ in c['values']]
 m.w(head,nodes[0]if nodes else 0)
 for i,(p,v)in enumerate(zip(nodes,c['values'])):
  m.w(p,m.actor if v=='old'else 123);m.w(p+4,head if i==0 else nodes[i-1]+8);m.w(p+8,nodes[i+1]if i+1<len(nodes)else 0);m.node_index[p]=['intrusive',i]
 m.call(0x4f3f10,[m.actor],this=m.world,callee_pop=4)
 p=m.r(head);previous=head;values=[];identities=[]
 while p:
  assert m.r(p+4)==previous;values.append('old'if m.r(p)==m.actor else'other');identities.append(nodes.index(p)+1);previous=p+8;p=m.r(p+8)
 return dict(values=values,identities=identities,trace=m.trace),m.actual.copy()
def run_memory(m,pc,c):
 m.begin(pc,'memory',c);err=None
 try:m.call(0x4ec290,this=m.actor)
 except Stop:err=m.cutoff
 return dict(error=err,trace=m.trace),m.actual.copy()
def run_count(m,pc,c):
 m.begin(pc,'count',c);value=m.call(0x4ea3d0,this=m.actor)
 return dict(value=value),m.actual.copy()
def run_caller(m,pc,c):
 m.begin(pc,'caller',c);m.linked(m.world+0x110,[123]if c.get('prior_retained',False)else[],'retained');err=None
 try:m.call(0x4f53d0,[m.actor,c.get('force',0)],this=m.world,callee_pop=8)
 except Stop:err=m.cutoff
 return dict(error=err,trace=m.trace,flags=m.r(m.actor+8),retained=m.readlist(m.world+0x110,lambda x:'old'if x==m.actor else x)),m.actual.copy()
def run_actor_prefix(m,pc,c):
 m.begin(pc,'actor_prefix',c);m.w(m.actor+0x140,9)
 sai=m.alloc(0xa0);m.w(0xb36528,sai);m.b(sai+0x20,int(c.get('sai_ready',False)))
 registry=m.alloc(0x60010);m.w(0xb364c0,registry)
 for off in [0x130,0x144,0x158,0x14c,0x160,0x164,0x168,0xe8,0x100,0x104]:m.w(m.actor+off,c.get(hex(off),0))
 m.w(m.actor+0x100,c.get('states',0));m.other12c=m.alloc(16)
 m.components_present=c.get('mind',False)or c.get('element',False)
 if c.get('mind',False):
  mind=m.alloc(0x60);m.w(mind+0x28,0);m.w(m.actor+0x70,mind)
 if c.get('element',False):
  element=m.alloc(0x60);m.w(element+0x10,0);m.w(m.actor+0x80,element)
 if c.get('companion',False):
  other=m.alloc(0x200);vt=m.alloc(0x140);m.w(other,vt);m.w(vt+0x12c,m.other12c);m.w(other+0x14,0x20008);m.w(m.actor+0x158,0x20008)
  m.u.mem_write(registry+4+8*2,struct.pack('<H',2));m.w(registry+0x20004+8*4,other)
 err=None
 try:m.call(0x4d7580,[c.get('retain',0)],this=m.actor,callee_pop=4)
 except Stop:err=m.trace[-1]['call']
 return dict(error=err,trace=m.trace,flags=m.r(m.actor+8),category140=m.r(m.actor+0x140)),m.actual.copy()
def main():
 cases=[]
 for values,query in [([],7),([7],7),([7,7],7),([1,7,7,3],7),([1,3,7],7),([1,3],7),([0xffffffff,0],0xffffffff)]:cases.append(('list',dict(values=values,query=query)))
 for values in [[],['old'],['old','old'],['other','old','old','other'],['other','old'],['other','other']]:
  for flags in [0,1<<8]:cases.append(('intrusive',dict(values=values,flags=flags)))
 for b0,source34,b4,city,other,other34,stale in [(1,1,0,0,0,0,0),(0,1,0,0,0,0,0),(1,0,1,0,0,0,0),(1,0,1,1,0,0,0),(1,0,1,1,1,0,0),(1,0,1,1,1,1,0),(1,0,1,1,1,1,1)]:
  for bc,c0 in [(0,0),(1,0),(0,1),(1,1)]:cases.append(('prefix',dict(b0=bool(b0),source34=bool(source34),b4=bool(b4),city=bool(city),other=bool(other),other34=bool(other34),stale=bool(stale),bc=bool(bc),c0=bool(c0))))
 for visible,phase in itertools.product([False,True],[0,3,5,6]):cases.append(('memory',dict(visible=visible,session_phase=phase)))
 for cutoff in ['visibility','allocate','refresh','clear']:cases.append(('memory',dict(visible=True,cutoff=cutoff)))
 for count,allocation in itertools.product([0,1,0xffffffff,0x80000000,0x7fffffff],[0,0x12345678]):cases.append(('count',dict(count=count,allocation=allocation)))
 for world_phase,force,count,visible,session_phase,prior in [(0,0,0,False,3,False),(6,0,1,True,3,False),(0,1,1,True,3,False),(0,0,1,False,3,False),(0,0,1,False,3,True),(0,0,0xffffffff,False,3,False),(0,0,0,True,5,False),(0,0,1,True,3,False)]:
  base=dict(world_phase=world_phase,force=force,count=count,visible=visible,session_phase=session_phase,prior_retained=prior)
  cases.append(('caller',base))
  for cutoff in ['prefix','uninit','dispatcher_unlink','destructor']:cases.append(('caller',dict(base,cutoff=cutoff)))
 for c in [{},{'retain':1},{'sai_ready':True},{'0x130':1},{'companion':True},{'0x160':1},{'0xe8':1},{'states':1<<25},{'mind':True},{'element':True}]:cases.append(('actor_prefix',c))
 rows=[];pins={}
 for image in ['original','v14']:
  m=Removal(image)
  for pc in PC:
   for kind,c in cases:
    output,visited=globals()['run_'+kind](m,pc,c);pins.update(visited);rows.append(dict(image=image,precision=pc,kind=kind,input=c,output=output))
 a=[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='original'];b=[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='v14'];assert a==b
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions)
 p=HERE/'native-removal.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executed_pins=len(instructions),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
