"""Original5307A0 decision handoff with required Performer, scoring and Mind virtual leaves."""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from native_lifecycle import LifecycleMachine,cases as lifecycle_cases,mp,VT
from native_constructors import original,modified
from unicorn.x86_const import *
DECISION,ACTOR,PERFORMER,ABILITY,POINT,RETURN,VALUE=0x4500000,0x4600000,0x4610000,0x4620000,0x4630000,0x4640000,0x4641000
RECONSIDER,FAILED_TARGET,FAILED_NONE=0x4700400,0x4700410,0x4700420
ptr=lambda base,i:0 if i is None else base+i*0x100
ident=lambda base,p:None if p==0 else (p-base)//0x100
f64=lambda f:struct.unpack('<Q',struct.pack('<d',f))[0]
sha=lambda b:hashlib.sha256(b).hexdigest()
class DecisionMachine(LifecycleMachine):
 def __init__(self,c):
  super().__init__(c);self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.write_world()
  self.w(VT+0x17c,RECONSIDER);self.w(VT+0x140,FAILED_TARGET);self.w(VT+0x144,FAILED_NONE)
  self.u.mem_write(RETURN,b'\xdd\x05'+struct.pack('<I',VALUE)+b'\xc2\x04\x00')
  for i,b in enumerate(c['point'] or[0,0]):self.w(POINT+i*4,b)
 def write_world(self):
  self.w(mp(1)+4,ptr(ACTOR,self.c['arena']['minds']['1']['actor']));self.w(mp(1)+0x118,self.c['bombard_mode'])
  for k,a in self.c['actors'].items():self.w(ptr(ACTOR,int(k))+0x74,ptr(PERFORMER,a['performer']));self.w(ptr(ACTOR,int(k))+0x14,a['id'])
  d=self.c['decision'];self.w(DECISION,ptr(ABILITY,d['ability']));self.w(DECISION+4,d['kind']);self.w(DECISION+8,d['status']);self.w(DECISION+0x10,ptr(ACTOR,d['target']))
 def snapshot_decision(self):return {'ability':ident(ABILITY,self.r(DECISION)),'kind':self.r(DECISION+4),'status':self.r(DECISION+8),'target':ident(ACTOR,self.r(DECISION+0x10))}
 def event_decision(self,name,args):
  self.c['decision']=self.snapshot_decision();self.events.append([name,*args,copy.deepcopy(self.c['decision'])]);mutations=self.c['mutations']
  if mutations and mutations[0]['event']==name:
   m=mutations.pop(0);self.c['decision'].update(m.get('decision',{}))
   if 'mind_actor' in m:self.c['arena']['minds']['1']['actor']=m['mind_actor']
   for k,a in m.get('actors',{}).items():self.c['actors'][k].update(a)
   self.write_world()
 def hook(self,u,pc,size,data):
  if original.base<=pc<original.base+original.image_size:self.visited[pc]=bytes(u.mem_read(pc,size))
  cx=u.reg_read(UC_X86_REG_ECX);s=u.reg_read(UC_X86_REG_ESP)
  if pc==0x57f290:
   point=self.r(s+16);request={'ability':ident(ABILITY,self.r(s+4)),'event_kind':self.r(s+8),'target':ident(ACTOR,self.r(s+12)),'point':None if point==0 else[self.r(point),self.r(point+4)]}
   self.event_decision('start',[ident(PERFORMER,cx),request]);self.ret(int(self.c['start_result']),16)
  elif pc in [0x52ed30,0x57e670]:
   name='weight' if pc==0x52ed30 else 'other';args=[1] if name=='weight' else[ident(PERFORMER,cx),ident(ABILITY,self.r(s+4))]
   self.event_decision(name,args);self.u.mem_write(VALUE,struct.pack('<Q',self.c[name+'_bits']));u.reg_write(UC_X86_REG_EIP,RETURN)
  elif pc in [RECONSIDER,FAILED_TARGET,FAILED_NONE,0x50b250,0x52d820,0x52ef40]:
   name={RECONSIDER:'reconsider',FAILED_TARGET:'failed_target',FAILED_NONE:'failed_none',0x50b250:'cancel',0x52d820:'wait',0x52ef40:'pursue'}[pc]
   self.event_decision(name,[1,self.r(s+4)] if pc==FAILED_TARGET else[1]);self.ret(pop=4 if pc in [FAILED_TARGET,0x52d820,0x52ef40]else 0)
  elif pc==0x404f60:self.ret()
  else:super().hook(u,pc,size,data)
 def run_decision(self):
  self.reset(0);self.u.reg_write(UC_X86_REG_ECX,mp(1));self.w(self.STACK+4,DECISION);self.w(self.STACK+8,POINT if self.c['point'] is not None else 0)
  self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[self.c['precision']])
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(0x5307a0,self.STOP,count=15000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+12
  assert [self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)]
  assert not self.c['mutations'],self.c['name'];return {'decision':self.snapshot_decision(),'events':self.events}
def cases():
 arena=copy.deepcopy(lifecycle_cases()[0]['arena']);arena['minds']['1']['actor']=1
 base={'name':'base','arena':arena,'decision':{'ability':1,'kind':6,'status':0,'target':3},'actors':{'1':{'performer':1,'id':0x10001},'2':{'performer':2,'id':0x20002},'3':{'performer':3,'id':0x30003}},'point':[0x3f800000,0xc0200000],'bombard_mode':0,'start_result':True,'weight_bits':f64(1),'other_bits':f64(2),'mutations':[]}
 rows=[]
 def add(name,change):c=copy.deepcopy(base);c['name']=name;change(c);rows.append(c)
 for status in [0,1,2,3,4,0xffffffff]:
  for target in [None,3]:
   for mode in [0,1,2,3]:add(f'status{status}_target{target}_mode{mode}',lambda c,s=status,t=target,m=mode:(c['decision'].update(status=s,target=t),c.update(bombard_mode=m)))
 for result in [False,True]:
  for target in [None,3]:add(f'start{result}_target{target}',lambda c,r=result,t=target:(c.update(start_result=r),c['decision'].update(target=t)))
 for weight,other in [(1,1),(1,0),(1,2),(0,-1),(-1,0),(1+2**-25,1),(1-2**-25,1),(1+2**-24,1+2**-23),(1+2**-23,1+2**-23)]:add(f'weights{weight}_{other}',lambda c,w=weight,o=other:c.update(weight_bits=f64(w),other_bits=f64(o)))
 add('point_null',lambda c:c.update(point=None))
 add('start_mutates_fields',lambda c:c.update(mutations=[{'event':'start','decision':{'ability':2,'target':2,'status':7}}]))
 add('failed_start_mutates_target',lambda c:c.update(start_result=False,mutations=[{'event':'start','decision':{'target':None,'status':7}}]))
 add('weight_changes_actor_and_ability',lambda c:c.update(mutations=[{'event':'weight','mind_actor':2,'decision':{'ability':3}}]))
 add('other_changes_state',lambda c:c.update(mutations=[{'event':'other','decision':{'status':7}}]))
 add('reconsider_changes_state',lambda c:c.update(mutations=[{'event':'reconsider','decision':{'status':0}}]))
 for p in ['Single24','Double53','Extended64']:
  for c in rows:yield {**copy.deepcopy(c),'name':c['name']+'_'+p,'precision':p}
def main():
 rows=[];visited={}
 for c in cases():
  try:m=DecisionMachine(c);result=m.run_decision()
  except Exception:print(c['name']);raise
  rows.append({'input':c,'output':result});visited.update(m.visited)
 for a,b in visited.items():assert b==original.range(a,len(b))==modified.range(a,len(b)),hex(a)
 d={'schema':1,'method':__doc__,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())],'cases':rows}
 p=HERE/'native-decision.json';p.write_text(json.dumps(d,indent=2),encoding='utf-8');print(json.dumps({'cases':len(rows),'sha256':sha(p.read_bytes())}))
if __name__=='__main__':main()
