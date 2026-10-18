"""Original tutorial SAI control, stack-command construction and actor routing.
Actor/component virtual callbacks are explicit mutable fixture services.
"""
from pathlib import Path
import copy,hashlib,json,sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine as Base,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
H=lambda b:hashlib.sha256(b).hexdigest()
A,K,N,S,T,C,D,G,VT,HOOK=0x4000000,0x4020000,0x4030000,0x4040000,0x4050000,0x4060000,0x4070000,0x4080000,0x4090000,0x40ff000
P=lambda key:0 if key is None else A+key*0x300
SP=lambda key:0 if key is None else S+key*0x100
class Machine(Base):
 def __init__(self,case):
  super().__init__();self.u.mem_map(0,0x1000);self.u.mem_map(A,0x100000);self.c=copy.deepcopy(case);self.actions=copy.deepcopy(case.get('mutations',{}));self.events=[];self.executed={};self.command=None
  self.u.hook_add(UC_HOOK_CODE,self.hook);self.reset(0);self.w(0xb36530,G);self.w(G+0x298,G+0x800);self.w(G+0x800+24,D if case['definition'] else 0);self.w(D+0x18,case['slot'])
  self.w(VT+0x38,HOOK);self.w(VT+0x158,HOOK+16);self.w(VT+0x15c,HOOK+32);self.w(VT+0x200+0x24,HOOK+48);self.w(VT+0x300+0x68,HOOK+64);self.w(VT+0x400,HOOK+80)
  for key,a in case['actors'].items():
   key=int(key);p=P(key);self.w(p,VT);self.w(p+8,a['flags']);self.w(p+0x154,SP(a['sai']))
   if a['component'] is not None:self.w(p+0x60+case['slot']*4,C+a['component']*16);self.w(C+a['component']*16,VT+0x200)
   if a['organization_parent'] is not None:self.w(p+0x7c,G+0x1000+key*0x100);self.w(G+0x1000+key*0x100+0x44,P(a['organization_parent']))
  self.w(K+8,case['kingdom_flags']);self.w(K+0x284,0 if case['head']is None else N+case['head']*16)
  for key,n in case['nodes'].items():p=N+int(key)*16;self.w(p,P(n['actor']));self.w(p+4,0 if n['next']is None else N+n['next']*16)
  for key,s in case['shared'].items():
   p=SP(int(key));self.w(p,VT+0x400);self.w(p+4,s['references']);self.w(p+16,0 if s['target']is None else T+s['target']*16)
   if s['target']is not None:self.w(T+s['target']*16,VT+0x300)
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+4*i)
 def ret(self,n=0,value=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+n)
 def event(self,name,args):
  self.events.append({'name':name,'args':args})
  for m in self.actions.pop(name+':'+str(args[0])if args else name,[]):
   kind=m['kind'];key=m.get('key');value=m['value']
   if kind=='next':self.w(N+key*16+4,0 if value is None else N+value*16);self.c['nodes'][str(key)]['next']=value
   elif kind=='sai':self.w(P(key)+0x154,SP(value));self.c['actors'][str(key)]['sai']=value
   elif kind=='references':self.w(SP(key)+4,value)
   elif kind=='component':self.w(P(key)+0x60+self.c['slot']*4,0 if value is None else C+value*16);self.w(C+value*16,VT+0x200)if value is not None else None
   elif kind=='shared34':self.w(self.command+0x34,SP(value))
   elif kind=='definition':self.w(self.command+8,D+0x100);self.w(D+0x100+0x18,value)
   else:raise ValueError(kind)
 def command_value(self):
  p=self.command;v=self.r(p+8)
  return {'references':self.r(p+4),'definition':None if v==0 else(1 if v==D else 2),'flags':self.r(p+12),'words10_2c':[self.r(p+x)for x in range(16,48,4)],'byte30':self.u.mem_read(p+48,1)[0],'shared34':None if self.r(p+52)==0 else(self.r(p+52)-S)//256,'word38':self.r(p+56),'byte3c':self.u.mem_read(p+60,1)[0]}
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:self.executed[pc]=size
  ecx=u.reg_read(UC_X86_REG_ECX)
  if pc==0x4d9e20:self.events.append({'name':'construct','args':[self.arg()]})
  if pc==0x4d9b60:self.command=ecx;self.events.append({'name':'command','value':self.command_value()})
  if pc==HOOK:
   key=(ecx-A)//0x300;definition=1 if self.arg()==D else 2;self.event('admit',[key,definition]);self.ret(4,int(self.c['actors'][str(key)]['admit']));return
  if pc==HOOK+16:
   key=(ecx-A)//0x300;self.event('virtual158',[key]);self.ret(value=int(self.c['actors'][str(key)]['virtual158']));return
  if pc==HOOK+32:
   key=(ecx-A)//0x300;self.event('virtual15c',[key]);self.ret(value=P(self.c['actors'][str(key)]['virtual15c']));return
  if pc==HOOK+48:
   component=(ecx-C)//16;self.command=self.arg();self.event('process',[component]);self.ret(4,int(self.c['process_result']));return
  if pc==HOOK+64:
   target=(ecx-T)//16;shared=(self.arg()-S)//256;self.event('notify',[target,shared,*[self.arg(i)for i in range(1,5)],self.r(SP(shared)+4)]);self.ret(20);return
  if pc==HOOK+80:self.event('destroy',[(ecx-S)//256,self.arg()]);self.ret(4);return
  if pc==0x404f30:self.events.append({'name':'null_diagnostic','args':[]});self.ret();return
 def run(self):
  entry={'enable':0x542080,'disable_kingdom':0x5432c0,'disable_actor':0x543380,'disable_actor_state':0x79e980,'process':0x4d9b60}[self.c['operation']]
  target=None if self.c['target'] is None else(K if self.c['target']=='kingdom'else P(self.c['target']))
  ecx=self.STATE
  if self.c['operation']=='process':
   self.w(self.STACK,self.STOP);self.w(self.STACK+4,6);self.u.reg_write(UC_X86_REG_ESP,self.STACK);self.u.reg_write(UC_X86_REG_ECX,self.STATE);self.u.emu_start(0x4d9e20,self.STOP,count=10000)
  self.w(self.STACK,self.STOP);self.w(self.STACK+4,target or 0);self.u.reg_write(UC_X86_REG_ESP,self.STACK);self.u.reg_write(UC_X86_REG_ECX,ecx)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(entry,self.STOP,count=1000000);assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
  assert [self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)]
  return {'returned':None if self.c['operation']=='disable_actor_state'else bool(self.u.reg_read(UC_X86_REG_EAX)&255),'kingdom_flags':self.r(K+8),'actors':{k:{'flags':self.r(P(int(k))+8),'sai':None if self.r(P(int(k))+0x154)==0 else(self.r(P(int(k))+0x154)-S)//256}for k in self.c['actors']},'references':{k:self.r(SP(int(k))+4)for k in self.c['shared']},'events':self.events}
def cases():
 a=lambda component:{'flags':0x1800000,'sai':None,'component':component,'admit':True,'virtual158':False,'virtual15c':None,'organization_parent':None}
 base={'name':'direct','operation':'disable_actor','target':1,'definition':True,'slot':4,'kingdom_flags':0x80000000,'head':1,'nodes':{'1':{'actor':1,'next':2},'2':{'actor':2,'next':None}},'actors':{'1':a(1),'2':a(2),'3':a(3)},'shared':{'1':{'references':1,'target':1},'2':{'references':1,'target':None}},'process_result':True,'mutations':{}}
 rows=[]
 for op,target in [('enable',None),('enable',1),('enable','kingdom'),('disable_actor',None),('disable_actor',1),('disable_kingdom',None),('disable_kingdom','kingdom'),('process',None),('process',1)]:
  c=copy.deepcopy(base);c.update(name=op+'_'+str(target),operation=op,target=target);c['actors']['1']['flags']|=0x20;c['kingdom_flags']|=0x20;rows.append(c)
 for refs,target in [(0,None),(0,1),(1,1),(0xffffffff,1),(17,None)]:
  c=copy.deepcopy(base);c['name']='shared_'+str(refs)+'_'+str(target);c['actors']['1']['sai']=1;c['shared']['1']={'references':refs,'target':target};rows.append(c)
 for branch in ['org','virtual','no_parent','reject','false_handler','null_node','head_empty','shared_cleanup','mutable_next','mutable_component','replace_sai','notify_refs']:
  c=copy.deepcopy(base);c['name']=branch
  if branch in ['org','virtual','no_parent']:c['actors']['1']['component']=None;c['actors']['1']['organization_parent']=2
  if branch=='virtual':c['actors']['1']['virtual158']=True;c['actors']['1']['virtual15c']=3
  if branch=='no_parent':c['actors']['1']['virtual158']=True
  if branch=='reject':c['actors']['1']['admit']=False;c['actors']['1']['organization_parent']=2
  if branch=='false_handler':c['process_result']=False
  if branch in ['null_node','head_empty','mutable_next']:c['operation']='disable_kingdom';c['target']='kingdom'
  if branch=='null_node':c['nodes']['1']['actor']=None
  if branch=='head_empty':c['head']=None
  if branch=='shared_cleanup':c['mutations']={'process:1':[{'kind':'shared34','value':2}]}
  if branch=='mutable_next':c['mutations']={'process:1':[{'kind':'next','key':1,'value':None}]}
  if branch=='mutable_component':c['mutations']={'admit:1':[{'kind':'component','key':1,'value':3}]}
  if branch in ['replace_sai','notify_refs']:
   c['actors']['1']['sai']=1;c['mutations']={'notify:1':[{'kind':'sai','key':1,'value':2}]if branch=='replace_sai'else[{'kind':'references','key':1,'value':1}]}
  rows.append(c)
 for slot in [0,3,7,15,30]:
  c=copy.deepcopy(base);c['name']='slot_'+str(slot);c['slot']=slot;rows.append(c)
 return rows
def main():
 rows=[];visited={}
 for c in cases():
  m=Machine(c);got=m.run();rows.append({'input':c,'native':got});visited.update(m.executed)
 pins=[]
 for a,n in sorted(visited.items()):
  raw=original.range(a,n);assert raw==modified.range(a,n);pins.append({'va':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(original.b),'v14_sha256':H(modified.b),'fixtures':rows,'native_instructions_and_service_entries':pins,'scope':__doc__+' All wrapper/control/constructor/routing/refcount code executes original bytes. Admission, parent/component/SAI target virtuals and object destruction are explicit callback services; null diagnostic suppressed. No component behavior or full tutorial claim.'}
 (OUT/'native-control.json').write_text(json.dumps(doc,indent=2)+'\n');print({'passed':True,'cases':len(rows)})
if __name__=='__main__':main()

