"""Original actor permission, selection storage, and live-order constructors.
Explicit seams are40EAA0 gate, optional4DA570 default resolver, heap allocation/
free, and a pre-instruction stop before the unbound positive group store4ECF2D.
No successful Mind subtype/world or complete UI/selection integration claim.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import *
P=h.P;V=h.V;H=lambda b:hashlib.sha256(b).hexdigest()
class Stop(Exception):pass
class Machine(h.Machine):
 def __init__(self,pc):
  super().__init__(pc);self.a=self.alloc(0x200);self.parent=self.alloc(0x200);self.session=self.alloc(0x200);self.view=self.alloc(0x40);self.gd=self.alloc(0x800);self.ptrs={self.a:1,self.parent:2}
  self.w32(0xb36550,self.session);self.w32(0xb36530,self.gd);self.group_stop=False
  self.seams[0x6f7810]=lambda m:m.ret(value=m.alloc(m.arg()))
  self.seams[0x6f7430]=lambda m:m.ret()
 def _hook(self,u,pc,size,arg):
  if pc==0x76ac90:self.events.append('phase')
  if pc==0x4ecf2d and self.group_stop:raise Stop('positive_group_store')
  super()._hook(u,pc,size,arg)
 def actor(self,p,c):
  self.w32(p,0xa299d8 if c.get('class',3)==4 else 0xa273f0);self.w32(p+8,c.get('flags',0));self.w32(p+0x14,self.ptrs[p]);self.w8(p+0x4c,c.get('byte4c',0))
  mask=c.get('action_bits',0);self.w32(p+0xf0,mask&0xffffffff,mask>>32)
  for off,name in [(0x6c,'mover'),(0xac,'mobile'),(0xa0,'recruit'),(0x88,'denizen')]:
   if c.get(name):self.w32(p+off,self.alloc(0x30))
  if c.get('element'):
   e=self.alloc(0x40);self.w32(p+0x80,e);self.w32(e+0x10,self.parent if c.get('element_parent')else 0)
 def permission(self,c):
  self.actor(self.a,c);self.actor(self.parent,c.get('parent',{}));self.w32(self.session+0xf8,c.get('phase',0))
  same=self.alloc(0x400);other=self.alloc(0x400);self.w32(self.a+0xe8,{'same':same,'other':other,'null':0}[c.get('owner','same')])
  command=self.alloc(0x40);definition=self.alloc(0x60);ref=self.alloc(4);self.w32(ref,command);self.w32(command+8,definition,c.get('command_flags',0x40));self.w32(definition+0xc,c.get('kind',5))
  def gate(m):m.events.append('gate');m.ret(value=int(c.get('gate',False)))
  def default(m):m.events.append('default');m.ret(8,value=definition if c.get('default',True)else 0)
  self.seams[0x40eaa0]=gate;self.seams[0x4da570]=default
  out=self.call(0x4cf430,self.a,[same,ref,c.get('selected',1)])&255
  assert self.final_esp==h.STACK+16 and self.fpu_top()==0
  return {'allowed':out!=0,'calls':self.events}
 def selection(self,c):
  self.actor(self.a,c);self.actor(self.parent,{});self.w32(self.session+0xe4,c.get('application',2));self.w32(self.view+0x10,*[0xffffffff]*4)
  head=0;tail=0
  for n in reversed(c.get('initial',[])):
   node=self.alloc(8);self.w32(node,self.a if n==1 else self.parent,head);head=node
   if not tail:tail=node
  self.w32(self.view+4,head,tail);self.w32(self.view+0x20,0)
  entry={'select':0x4ecff0,'deselect':0x4ed080,'clear':0x4ed0d0}[c['operation']];self.group_stop=True
  result='complete'
  try:
   self.call(entry,self.view,[]if c['operation']=='clear'else[self.a if c.get('candidate',True)else 0]);assert self.final_esp==h.STACK+(4 if c['operation']=='clear'else 8)and self.fpu_top()==0
  except Stop as e:result=str(e)
  vals=[];node=self.r32(self.view+4);last=0
  while node:
   assert len(vals)<10;vals.append(self.ptrs[self.r32(node)]);last=node;node=self.r32(node+4)
  assert self.r32(self.view+8)==last
  return {'result':result,'selected':vals,'byte4c':[self.r8(self.a+0x4c),self.r8(self.parent+0x4c)],'mask_words':self.r32(self.view+0x10,4),'group_null':self.r32(self.view+0x20)==0}
 def constructor(self,c):
  table=self.alloc(39*4)
  for kind in range(39):
   d=self.alloc(0x60);self.w32(d+0xc,kind);self.w32(table+4*kind,d)
  self.w32(self.gd+0x298,table);command=self.alloc(0x40);self.u.mem_write(command,b'\xcd'*0x40)
  if c['kind']==5:self.call(0x4ca760,command,[*c['position'],c['speed'],*c['auxiliary']]);assert self.final_esp==h.STACK+24
  else:
   self.call(0x4d9e20,command,[0]);assert self.final_esp==h.STACK+8
   self.actor(self.a,{});self.call(0x4d9bf0,command,[self.a if c['target']else 0]);assert self.final_esp==h.STACK+8
  assert self.fpu_top()==0
  return {'kind':self.r32(self.r32(command+8)+0xc),'flags':self.r32(command+0xc),'word10':self.r32(command+0x10),'object':self.r32(command+0x14),'position':self.r32(command+0x18,2),'auxiliary':self.r32(command+0x20,2),'speed':self.r32(command+0x28),'parameter':self.r32(command+0x2c),'boolean':self.r8(command+0x30),'parameters':self.r32(command+0x34),'extra':self.r32(command+0x38),'trailing':self.r8(command+0x3c)}
def main():
 permissions=[]
 for owner,selected,flags,mover,gate,phase in itertools.product(['same','other','null'],[0,1],[0,4],[False,True],[False,True],[6,7]):permissions.append(dict(owner=owner,selected=selected,flags=flags,mover=mover,gate=gate,phase=phase,action_bits=(1<<64)-1))
 for kind in [0,5,31,32,38,63,64,255,256,257,0xffffffff]:
  for bits in [0,1,1<<5,1<<63,(1<<64)-1]:permissions.append(dict(kind=kind,action_bits=bits))
 for field in ['mobile','recruit','denizen']:
  permissions.append(dict(mover=True,action_bits=1<<5,**{field:True}))
 for flags in [0,4]:permissions.append(dict(element=True,element_parent=True,parent={'flags':flags},action_bits=1<<5))
 for default in [False,True]:permissions.append(dict(command_flags=0,default=default,action_bits=1<<5))
 selections=[]
 for operation,initial,byte,app in itertools.product(['select','deselect','clear'],[[],[1],[2]],[0,255],[2,3]):selections.append(dict(operation=operation,initial=initial,byte4c=byte,application=app))
 selections +=[dict(operation='select',initial=[],candidate=False),dict(operation='select',initial=[],element=True,element_parent=True,application=2),dict(operation='select',initial=[],element=True,element_parent=True,application=3)]
 constructors=[dict(kind=5,position=[0x3f800000,0x80000000],speed=0,auxiliary=[0,0]),dict(kind=5,position=[0x7fc01234,0x7f800000],speed=0xdeadbeef,auxiliary=[0x12345678,0xffffffff]),dict(kind=0,target=False),dict(kind=0,target=True)]
 rows=[]
 for image,label in [(P,'original'),(V,'v14')]:
  h.P=image
  for pc in h.PRECISION:
   for kind,cases,method in [('permission',permissions,'permission'),('selection',selections,'selection'),('constructor',constructors,'constructor')]:
    for c in cases:rows.append({'image':label,'precision':pc,'kind':kind,'input':c,'output':getattr(Machine(pc),method)(c)})
 h.P=P
 for va,raw in h.VISITED.items():assert bytes.fromhex(raw)==P.range(va,len(bytes.fromhex(raw)))==V.range(va,len(bytes.fromhex(raw)))
 out={'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'producer_sha256':H(Path(__file__).read_bytes()),'rows':rows,'vtable_pins':{hex(t):{hex(o):P.u32(t+o)for o in [0x20,0x34,0x38,0x40,0xb4,0x150,0x154]}for t in [0xa273f0,0xa299d8]},'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-player-orders.json';p.open('x').write(json.dumps(out,indent=1)+'\n');print(json.dumps({'rows':len(rows),'pins':len(h.VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
