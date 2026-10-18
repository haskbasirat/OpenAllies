"""Execute original52D820/52EF40, real52D660/529C30/4CF110 and576AD0.
Explicit fixture leaves: ability recharge/range/utility, MoveThink537B90,
Mover stop5767C0, default stance, target movement/speed, scheduler and Mind
decision virtuals. Their events are required boundaries, not a live world.
Also execute isolated576AD0 across all three nearest x87 PC modes.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine as Base,original,modified,Cs,CS_ARCH_X86,CS_MODE_32
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
MIND,ACTOR,MOVER,VT,WORLD,ABILITY,LOCAL,DEF,PERF,DECISION,VALUE,RETURN=range(0x4000000,0x400c000,0x1000)
RECONSIDER,SCHEDULE,TARGET_DECISION,SPEED=range(0x400d000,0x400d040,0x10)
bits=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
f64=lambda f:struct.unpack('<Q',struct.pack('<d',f))[0]
ptr=lambda base,i:0 if i is None else base+i*0x100
ident=lambda base,p:None if not p else (p-base)//0x100
sha=lambda b:hashlib.sha256(b).hexdigest()
class Machine(Base):
 def __init__(self,c):
  self.c=copy.deepcopy(c);super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.u.mem_map(0x4000000,0x20000);self.events=[];self.visited={}
  self.w(MIND,VT);self.w(MIND+8,VT+0x300);self.w(VT+0x304,SCHEDULE);self.w(VT+0x17c,RECONSIDER);self.w(VT+0x1ac,TARGET_DECISION);self.w(VT+0x18,SPEED)
  self.w(MIND+0x7c,VT+0x400);self.w(MIND+0x80,MIND);self.w(VT+0x40c,0x537b90);self.w(0xb36534,WORLD)
  for pop in [0,4]:self.u.mem_write(RETURN+pop*0x10,b'\xdd\x05'+struct.pack('<I',VALUE)+(b'\xc2'+struct.pack('<H',pop) if pop else b'\xc3'))
  self.write_world()
  for i in [1,2,3,4]:
   p=ptr(MOVER,i);self.w(p+4,ptr(ACTOR,i));self.w(p+0x44,0xdeadbeef);self.w(p+0x48,bits(-777))
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def put(self,a,*v):self.u.mem_write(a,struct.pack('<'+'I'*len(v),*(x&0xffffffff for x in v)))
 def write_world(self):
  c=self.c;self.w(MIND+4,ptr(ACTOR,c['mind_actor']));self.w(MIND+0x118,c['bombard_mode']);self.w(WORLD+0xe8,c['delta'])
  d=c['decision'];self.put(DECISION,ptr(ABILITY,d['ability']),d['kind'],d['status'],0,ptr(ACTOR,d['target']))
  for i,a in c['actors'].items():
   i=int(i);p=ptr(ACTOR,i);self.w(p,VT);self.w(p+4,ptr(DEF,i));self.w(p+0x14,a['id']);self.w(p+0x6c,ptr(MOVER,a['mover']));self.w(p+0xe4,ptr(MOVER,a['active']));self.w(p+0x74,ptr(PERF,a['performer']));self.w(p+0x78,ptr(DEF,i)+0x80 if a['modifier'] else 0);self.put(p+0x2c,*a['facing']);self.w(ptr(MOVER,i)+0x24,a['requested_speed']);self.w(ptr(ABILITY,i)+8,ptr(LOCAL,i))
 def snapshot(self):
  return {'mind_actor':ident(ACTOR,self.r(MIND+4)),'decision':{'ability':ident(ABILITY,self.r(DECISION)),'kind':self.r(DECISION+4),'status':self.r(DECISION+8),'target':ident(ACTOR,self.r(DECISION+0x10))}}
 def event(self,name,args):
  state=self.snapshot();self.events.append([name,*args,copy.deepcopy(state)]);self.c.update(state)
  if self.c['mutations'] and self.c['mutations'][0]['event']==name:
   m=self.c['mutations'].pop(0);self.c['decision'].update(m.get('decision',{}))
   for k in ['mind_actor','bombard_mode','delta']:
    if k in m:self.c[k]=m[k]
   for i,a in m.get('actors',{}).items():self.c['actors'][i].update(a)
   self.write_world()
 def ret(self,value=0,pop=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value&0xffffffff);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+pop)
 def floating(self,value,pop):
  self.u.mem_write(VALUE,struct.pack('<Q',value));self.u.reg_write(UC_X86_REG_EIP,RETURN+pop*0x10)
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:self.visited[pc]=bytes(u.mem_read(pc,size))
  cx=u.reg_read(UC_X86_REG_ECX);s=u.reg_read(UC_X86_REG_ESP);arg=lambda i=0:self.r(s+4+i*4)
  if pc==0x4c83e0:self.event('recharge',[ident(ABILITY,cx),ident(ACTOR,arg())]);self.floating(self.c['recharge'],4)
  elif pc==0x5767c0:self.event('stop',[ident(MOVER,cx),arg(),arg(1)]);self.ret(pop=8)
  elif pc==0x4cfc80:self.event('stance',[ident(ACTOR,cx)]);self.ret()
  elif pc==SCHEDULE:self.event('schedule',[arg(),arg(1)]);self.ret(pop=8)
  elif pc==0x4be6c0:self.event('weight',[ident(LOCAL,cx),ident(DEF,arg())]);self.floating(self.c['weight'],4)
  elif pc==0x57e670:assert arg()==0;self.event('maximum',[ident(PERF,cx)]);self.floating(self.c['maximum'],4)
  elif pc==RECONSIDER:self.event('reconsider',[]);self.ret()
  elif pc==0x537b90:self.event('think',[]);self.ret()
  elif pc==0x4c8270:self.event('range',[ident(ABILITY,cx),None if arg()==0 else ident(DEF,arg()-0x9c)]);self.floating(self.c['range'],4)
  elif pc==0x4cede0:self.event('moving',[ident(ACTOR,cx)]);self.ret(int(self.c['moving']))
  elif pc==SPEED:self.event('speed',[ident(ACTOR,cx)]);self.floating(self.c['speed'],0)
  elif pc==0x576ad0 and self.c['operation']!='target':self.event('target',[ident(MOVER,cx),ident(ACTOR,arg()),arg(1)])
  elif pc==TARGET_DECISION:self.event('decision',[ident(ACTOR,arg())]);self.ret(self.c['target_decision'],4)
  elif pc==0x404f90:self.ret()
 def run(self):
  self.reset(0);operation=self.c['operation'];entry={'wait':0x52d820,'pursue':0x52ef40,'target':0x576ad0}[operation]
  self.u.reg_write(UC_X86_REG_ECX,ptr(MOVER,1) if operation=='target' else MIND)
  if operation=='target':self.put(self.STACK+4,ptr(ACTOR,self.c['decision']['target']),self.c['target_range'])
  else:self.w(self.STACK+4,DECISION)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  try:self.u.emu_start(entry,self.STOP,count=20000)
  except Exception:print(self.c['name'],hex(self.u.reg_read(UC_X86_REG_EIP)));raise
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+(12 if operation=='target' else 8)
  assert [self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)];assert not self.c['mutations']
  return {**self.snapshot(),'events':self.events,'movers':{str(i):[self.r(ptr(MOVER,i)+0x44),self.r(ptr(MOVER,i)+0x48)]for i in [1,2,3,4]}}
def cases():
 base={'name':'base','operation':'wait','mind_actor':1,'decision':{'ability':1,'kind':1,'status':1,'target':3},'actors':{str(i):{'id':(i<<16)|i,'mover':i,'active':i,'performer':i,'modifier':True,'facing':[bits(1),0],'requested_speed':bits(i*2.375)}for i in [1,2,3,4]},'delta':bits(.1),'bombard_mode':0,'recharge':f64(.3),'weight':f64(1),'maximum':f64(2),'range':f64(20),'moving':False,'speed':f64(3.75),'target_decision':5,'mutations':[],'target_range':bits(5)}
 rows=[]
 def add(name,op,change=lambda c:None):c=copy.deepcopy(base);c.update(name=name,operation=op);change(c);rows.append(c)
 for delay in [-1.,-0.,0.,2**-151,.1,.6,struct.unpack('<f',struct.pack('<I',0x3f19999a))[0],.9]:add('delay'+str(delay),'wait',lambda c,d=delay:c.update(recharge=f64(d)))
 for weight,maximum in [(1,1),(1,0),(1+2**-25,1+2**-26),(1-2**-25,1),(-2,-1),(0,-0.)]:add('weight'+str((weight,maximum)),'wait',lambda c,w=weight,m=maximum:c.update(weight=f64(w),maximum=f64(m)))
 for mode in [0,2]:
  for kind in [1,6]:
   for active in [None,1]:add(f'wait_{mode}_{kind}_{active}','wait',lambda c,m=mode,k=kind,a=active:(c.update(bombard_mode=m),c['decision'].update(kind=k),c['actors']['1'].update(active=a)))
 add('no_mover','wait',lambda c:c['actors']['1'].update(mover=None,active=None))
 for moving in [False,True]:
  for facing in [[1,0],[-1,0],[0,1],[.3,.7]]:
   for outcome in [-1,4,5,6,0x7fffffff]:add(f'pursue_{moving}_{facing}_{outcome}','pursue',lambda c,m=moving,f=facing,o=outcome:(c.update(moving=m,target_decision=o),c['actors']['3'].update(facing=list(map(bits,f)))))
 add('inactive_mover','pursue',lambda c:c['actors']['1'].update(active=None))
 add('absent_mover_inactive','pursue',lambda c:c['actors']['1'].update(mover=None,active=2))
 for event,op,change in [('think','pursue',{'mind_actor':2,'decision':{'ability':2,'target':4}}),('speed','pursue',{'mind_actor':2,'delta':bits(.875),'decision':{'target':4}}),('schedule','wait',{'mind_actor':2,'decision':{'ability':2}}),('schedule','wait',{'bombard_mode':2}),('stop','wait',{'mind_actor':2}),('decision','pursue',{'decision':{'status':7}})]:
  add(f'mutate_{event}_{change}',op,lambda c,e=event,m=change:c.update(moving=True,mutations=[{'event':e,**m}]))
 for r,d,s in [(5,.1,2),(0,0,3),(-0.,-0.,3),(-5,.5,-3),(1,1,1),(1,1.0000001192092896,.9999999403953552),(2**-149,2**-80,2**-70)]:
  for t in [None,3]:add(f'target_{r}_{d}_{s}_{t}','target',lambda c,r=r,d=d,s=s,t=t:(c.update(target_range=bits(r),delta=bits(d)),c['actors']['1'].update(requested_speed=bits(s)),c['decision'].update(target=t)))
 for p in ['Single24','Double53','Extended64']:
  for c in rows:yield {**copy.deepcopy(c),'name':c['name']+'_'+p,'precision':p}
def main():
 rows=[];visited={}
 for c in cases():m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 for a,b in visited.items():assert b==original.range(a,len(b))==modified.range(a,len(b)),hex(a)
 doc={'schema':1,'method':__doc__,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'cases':rows,'visited_instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())]}
 (HERE/'native-motion-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf8')
 md=Cs(CS_ARCH_X86,CS_MODE_32);assembly=[]
 for a,end in [(0x52d820,0x52d901),(0x52ef40,0x52f009),(0x576ad0,0x576b1f),(0x4c8270,0x4c8293),(0x529c30,0x529c37),(0x52d660,0x52d6b4),(0x4cf110,0x4cf11e)]:
  b=original.range(a,end-a);assert b==modified.range(a,end-a);assembly.extend(f'{i.address:08x} {i.mnemonic} {i.op_str}'for i in md.disasm(b,a))
 (HERE/'assembly.txt').write_text('\n'.join(assembly)+'\n',encoding='utf8');print(json.dumps({'passed':True,'cases':len(rows)}))
if __name__=='__main__':main()

