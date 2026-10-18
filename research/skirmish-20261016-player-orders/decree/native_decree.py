"""Original/v14 actor decree61E220 and generic ctor4D9C80/reader61EBA0. Actor150/154/action-bit dispatch and44DEA0/4D1900 execute unmodified. Required group/delivery stops are explicit; decoder primitive bit-input seams supply typed fixture values (not wire-codec proof). Command table is pinned separately executed native39 registration output."""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import *
P=h.P;V=h.V;H=lambda b:hashlib.sha256(b).hexdigest()
REF=HERE.parent/'command-ui/native-actual-controls.json';D=json.loads(REF.read_text());DEFS=D['runs'][0]['definitions']
assert len(DEFS)==39
class Stop(Exception):pass
class Machine(h.Machine):
 def _hook(self,u,pc,size,arg):
  if pc==0x4cf430 and hasattr(self,'actor_trace'):
   self.actor_trace.append('permit');assert self.arg()==0 and self.arg(2)==0
   if self.stop_at=='permit':raise Stop('permit')
  if pc==0x61e23d and hasattr(self,'actor_trace'):self.permitted=(u.reg_read(UC_X86_REG_EAX)&255)!=0
  if pc==0x4d1900 and hasattr(self,'actor_trace'):self.actor_trace.append('linked_group')
  super()._hook(u,pc,size,arg)
 def actor_case(self,c):
  a=self.alloc(0x200);cmd=self.alloc(0x40);d=self.alloc(0x60);order=self.alloc(12);ref=order+4;group=self.alloc(0x200);reg=self.alloc(0x60004)
  self.w32(0xb364c0,reg);self.w32(a,0xa273f0);self.w32(a+0xf0,c['actions']&0xffffffff,c['actions']>>32);self.w32(a+0x164,0x10002)
  self.u.mem_write(reg+4+2*2,(1 if c['group']!='stale'else 2).to_bytes(2,'little'));self.w32(reg+0x20004+2*4,group if c['group']!='none'else 0)
  self.w32(cmd+8,d,0x40);self.w32(d+0xc,c['kind']);self.w32(ref,cmd,a if c['present']else 0)
  self.actor_trace=[];self.stop_at=c.get('stop');self.permitted=None
  def prep(m):
   self.actor_trace.append('prepare_group');assert m.u.reg_read(UC_X86_REG_ECX)==group and m.arg()==a
   if self.stop_at=='prepare_group':raise Stop('prepare_group')
   m.ret(4)
  def deliver(m):
   self.actor_trace.append('deliver');assert m.u.reg_read(UC_X86_REG_ECX)==cmd and m.arg()==a
   if self.stop_at=='deliver':raise Stop('deliver')
   m.ret(4)
  self.seams[0x4dff30]=prep;self.seams[0x4d9b60]=deliver
  status='complete'
  try:self.call(0x61e220,order,[]);assert self.final_esp==h.STACK+4 and self.fpu_top()==0
  except Stop as e:status=str(e)
  return{'status':status,'events':self.actor_trace,'allowed':self.permitted}
 def decoded_case(self,c):
  cmd=self.alloc(0x40);self.u.mem_write(cmd,b'\xcd'*0x40);self.call(0x4d9c80,cmd,[]);assert self.final_esp==h.STACK+4
  manager=self.alloc(0x40);array=self.alloc(39*4);self.w32(0xb3aa58,manager);self.w32(manager+0x24,array,39);defs=[]
  for i,row in enumerate(DEFS):
   d=self.alloc(0x40);self.w32(d+0xc,row['mind_type']);self.w32(d+0x18,row['component_slot']);self.w32(d+0x2c,row['behavior_flags']);self.w32(array+i*4,d);defs.append(d)
  f=c['flags'];has=lambda b:(f>>b)&1
  q=[('bounded',0x800,f)]
  if has(6):q.append(('bounded',40,39 if c['command']is None else c['command']))
  for b,names in [(0,['word']),(2,['x','y']),(3,['aux_x','aux_y']),(4,['scalar'])]:
   if has(b):q.extend(('raw',32,c[k])for k in names)
  if has(5):q.append(('raw',1,1))
  if has(9):q.append(('raw',1,c['boolean']))
  q.append(('raw',1,c['trailing']));self.reads=[]
  def read(m,kind):
   tag,n,value=q.pop(0);assert tag==kind,(tag,kind);assert m.arg(2)==n,(m.arg(2),n);self.reads.append([tag,n,value]);target=m.arg(1);m.u.mem_write(target,value.to_bytes(4,'little')[:(n+7)//8]if kind=='raw'else value.to_bytes(4,'little'));m.ret()
  self.seams[0x411fa0]=lambda m:read(m,'bounded');self.seams[0x40d5d0]=lambda m:read(m,'raw')
  self.call(0x61eba0,0,[self.alloc(0x20),cmd]);assert not q and self.final_esp==h.STACK+4 and self.fpu_top()==0
  p=self.r32(cmd+8);out={'command':None if p==0 else defs.index(p),'flags':self.r32(cmd+0xc),'word':self.r32(cmd+0x10),'object':self.r32(cmd+0x14),'position':self.r32(cmd+0x18,2),'auxiliary':self.r32(cmd+0x20,2),'scalar':self.r32(cmd+0x28),'parameter':self.r32(cmd+0x2c),'parameters':self.r32(cmd+0x34),'extra':self.r32(cmd+0x38),'boolean':self.r8(cmd+0x30),'trailing':self.r8(cmd+0x3c)}
  return{'fields':out,'reads':self.reads}
def main():
 cases=[dict(present=p,kind=k,actions=a,group=g,stop=s)for p,k,a,g,s in itertools.product([False,True],[0,5,33,38],[0,(1<<64)-1],['none','stale','valid'],[None,'permit','prepare_group','deliver'])]
 masks=[0,1,4,8,16,32,64,512,1024,0x5c,0x41,0x65c,0x67d]
 decoded=[dict(flags=f,command=d,word=0xfedcba98,x=0x80000000,y=0x7fc12345,aux_x=0x3f800000,aux_y=0xc0200000,scalar=0xffffffff,boolean=b,trailing=t)for f,d,b,t in itertools.product(masks,[None,0,5,38],[0,1],[0,1])]
 rows=[]
 for image,label in [(P,'original'),(V,'v14')]:
  h.P=image
  for pc in h.PRECISION:
   for c in cases:rows.append({'image':label,'precision':pc,'kind':'actor','input':c,'output':Machine(pc).actor_case(c)})
   for c in decoded:rows.append({'image':label,'precision':pc,'kind':'decoded','input':c,'output':Machine(pc).decoded_case(c)})
 h.P=P
 for va,raw in h.VISITED.items():assert bytes.fromhex(raw)==P.range(va,len(bytes.fromhex(raw)))==V.range(va,len(bytes.fromhex(raw)))
 p=HERE/'native-decree.json';p.open('x').write(json.dumps({'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'native39_reference_sha256':H(REF.read_bytes()),'rows':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())]},indent=1)+'\n');print(json.dumps({'rows':len(rows),'pins':len(h.VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()


