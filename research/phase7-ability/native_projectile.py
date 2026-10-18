"""Original projectile580BC0/580920/580660 through actual Ability/Body hit.
Actor activation/movement, target hit surface and scheduler/FX are required
world leaves. Native travel timing, prediction, interpolation and hit run.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE));import native_execution as execution
weapon=execution.weapon;body=weapon.body;bits=weapon.bits
from unicorn.x86_const import *
COMP,OWNER,DATA=0x20c0000,0x20c1000,0x20c2000
VT,ACTIVATE,HEIGHT,SPEED=0x20c3000,0x20cf000,0x20cf010,0x20cf020
class Machine(execution.Machine):
 def __init__(self,c):
  self.projectile_case=copy.deepcopy(c);super().__init__(c['execution']);self.flight=[]
  self.handle_ptrs[7]=OWNER;self.ptr_handles[OWNER]=7
  self.w(COMP,VT);self.w(COMP+4,OWNER);self.w(VT+0x38,ACTIVATE);self.w(OWNER,VT+0x200);self.w(OWNER+4,DATA);self.w(OWNER+0x14,0x10077)
  self.w(VT+0x200+0x108,execution.ALLEG);self.w(self.r(body.SOURCE)+0xb4,weapon.ACTOR_ELIGIBLE)
  self.put(OWNER+0x20,*c['owner_position']);self.w(OWNER+0xe4,COMP if c['initial_active'] else 0)
  self.w(DATA+0x2b0,c['speed']);self.w(DATA+0x2b4,c['kind']);self.w(DATA+0x2bc,c['life'])
  self.w(COMP+0x68,c['speed']);self.w(COMP+0x44,0xff7fffff);self.u.mem_write(COMP+0x4c,bytes([1]))
  self.w(self.CACHE+0x34,DATA+0x500);self.w(DATA+0x500,c['ratios'][0]);self.w(self.CACHE+0x38,DATA+0x504);self.w(DATA+0x504,c['ratios'][1])
  for h,ptr in [(1,body.SOURCE),(2,self.ACTOR)]:
   self.w(self.r(ptr)+8,HEIGHT);self.w(self.r(ptr)+0x18,SPEED);self.put(ptr+0x2c,*c['motion'][str(h)]['facing'])
  self.register(0x10017,body.SOURCE if c['source_live'] else 0);self.register(777,self.ACTOR if c['target_live'] else 0)
 def context(self,p=weapon.CONTEXT):
  context=super().context(p)
  if context['word2c']==OWNER:context['word2c']=7 # native projectile actor pointer -> fixture handle
  return context
 def register(self,id,ptr):
  reg=self.r(0xb364c0);self.u.mem_write(reg+4+(id&0xffff)*2,struct.pack('<H',id>>16));self.w(reg+0x20004+(id&0xffff)*4,ptr)
 def float_return(self,bits_value,pop=0):
  self.w(DATA+0x600,bits_value);self.u.mem_write(DATA+0x700,b'\xd9\x05'+struct.pack('<I',DATA+0x600)+(b'\xc2'+struct.pack('<H',pop) if pop else b'\xc3'));self.u.reg_write(UC_X86_REG_EIP,DATA+0x700)
 def hook(self,u,pc,size,user):
  if not hasattr(self,'flight'):return super().hook(u,pc,size,user)
  if pc==ACTIVATE:self.flight.append({'name':'activate','result':self.projectile_case['activate']});self.w(OWNER+0xe4,COMP if self.projectile_case['activate'] else 0);self.ret(value=int(self.projectile_case['activate']));return
  if pc in [HEIGHT,SPEED]:
   h=self.ptr_handles[u.reg_read(UC_X86_REG_ECX)];self.float_return(self.projectile_case['motion'][str(h)]['height' if pc==HEIGHT else 'speed']);return
  if pc==0x529c30 and u.reg_read(UC_X86_REG_ECX)==COMP:self.ret(value=DATA);return
  if pc==0x50ac00:self.flight.append({'name':'schedule','event':self.arg()});self.ret(4);return
  if pc==0x5766a0:self.flight.append({'name':'inside','point':[self.arg(),self.arg(1)]});self.ret(8,int(self.projectile_case['inside']));return
  if pc==0x552640:
   p=self.arg();self.put(p,*self.execution_case['clamped']);self.flight.append({'name':'clamp','position':self.execution_case['clamped']});self.ret(4);return
  if pc==0x4eeee0:
   h=self.ptr_handles[u.reg_read(UC_X86_REG_ECX)];self.flight.append({'name':'surface','actor':h,'direction':[self.arg(i)for i in [1,2,3]]});self.put(self.arg(),*self.projectile_case['surface']);self.ret(16,self.arg());return
  if pc==0x4ee5c0:
   ptr=self.arg();position=[self.r(ptr+i*4) for i in range(3)];self.put(OWNER+0x20,*position);self.flight.append({'name':'move','position':position});self.ret(4);return
  if pc==0x4db0b0:self.w(OWNER+0xe4,0);self.flight.append({'name':'deactivate'});self.ret();return
  super().hook(u,pc,size,user)
 def projectile(self):
  return {'owner':7,'source_id':self.r(COMP+0x10),'spawn_bone':self.r(COMP+0x14),'target_id':self.r(COMP+0x18),'origin':[self.r(COMP+0x1c+i*4)for i in range(3)],'target':[self.r(COMP+0x28+i*4)for i in range(3)],'ability':{'definition':10 if self.r(COMP+0x38) else 0,'local_definition':None,'use_count':self.r(COMP+0x40),'last_use':self.r(COMP+0x44),'word14':self.r(COMP+0x48),'enabled':bool(self.u.mem_read(COMP+0x4c,1)[0]),'recharge_bypass':bool(self.u.mem_read(COMP+0x4d,1)[0]),'fuzzy_range':self.r(COMP+0x50),'power':self.r(COMP+0x54),'summoned_ids':[],'word28':self.r(COMP+0x5c)},'remaining':self.r(COMP+0x60),'duration':self.r(COMP+0x64),'speed':self.r(COMP+0x68),'started_at':self.r(COMP+0x6c),'event_kind':self.r(COMP+0x70)}
 def run(self):
  steps=[]
  for op in self.projectile_case['operations']:
   self.w(self.STACK,self.STOP);self.u.reg_write(UC_X86_REG_ESP,self.STACK)
   regs={r:0x88ba0000+i for i,r in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
   for r,v in regs.items():self.u.reg_write(r,v)
   if op['kind']=='start':self.u.reg_write(UC_X86_REG_ECX,COMP);self.put(self.STACK+4,weapon.CONTEXT,body.WEAPON if self.projectile_case['copy_ability'] else 0,42);entry,pop=0x580bc0,16
   else:self.u.reg_write(UC_X86_REG_ECX,COMP+8);self.put(self.STACK+4,op['delta'],0);entry,pop=0x580920,12
   try:self.u.emu_start(entry,self.STOP,count=3000000)
   except Exception as exc:raise RuntimeError(hex(self.u.reg_read(UC_X86_REG_EIP))) from exc
   assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+pop
   for r,v in regs.items():assert self.u.reg_read(r)==v
   steps.append({'projectile':self.projectile(),'position':[self.r(OWNER+0x20+i*4)for i in range(3)],'active':self.r(OWNER+0xe4)==COMP,'flight':copy.deepcopy(self.flight),'actions':copy.deepcopy(self.action_events),'requests':copy.deepcopy(self.deliveries),'body':self.state(),'environment':copy.deepcopy(self.env),'events':copy.deepcopy(self.events)})
  return steps

def base(name):
 e=execution.base(name);e['definition']['projectile']=12;e['context']['target_position']=list(map(bits,[6,8,2]));e['weapon_case']['hit']['weapon']['propagation']={'definition_word38':0,'definition_word4c':0}
 return {'name':name,'precision':'Double53','execution':e,'activate':True,'initial_active':False,'owner_position':list(map(bits,[1,2,3])),'speed':bits(4.7),'kind':0,'life':0,'ratios':list(map(bits,[.5,.75])),'source_live':True,'target_live':True,'inside':True,'copy_ability':True,
  'surface':list(map(bits,[5.7,7.9,2.5])),'motion':{'1':{'height':bits(2),'speed':bits(1.3),'facing':list(map(bits,[.2,.3,.1]))},'2':{'height':bits(1.5),'speed':bits(.73),'facing':list(map(bits,[.7,.2,.3]))}},
  'operations':[{'kind':'start'},{'kind':'tick','delta':bits(.1)},{'kind':'tick','delta':bits(.2)},{'kind':'tick','delta':bits(20)},{'kind':'tick','delta':bits(.1)}]}
def cases():
 rows=[]
 def add(name,edit=None):
  c=base(name)
  if edit:edit(c)
  rows.append(c)
 add('flight_to_body')
 add('activation_failed',lambda c:c.update(activate=False))
 for kind in [1,2,3]:add('kind_'+str(kind),lambda c,k=kind:c.update(kind=k))
 for life in [.1,.3,1.,2.3]:add('life_'+str(life),lambda c,v=life:c.update(life=bits(v)))
 add('exact_zero_waits',lambda c:(c.update(life=bits(1)),c.update(operations=[{'kind':'start'},{'kind':'tick','delta':bits(1)},{'kind':'tick','delta':bits(.0001)}])))
 add('lost_target',lambda c:c.update(target_live=False))
 add('target_airborne',lambda c:c['execution']['actors']['2'].update(flags8=0x5000))
 add('outside_clamped',lambda c:c.update(inside=False))
 add('point_target',lambda c:c['execution']['context'].update(target=None))
 add('no_copied_ability',lambda c:(c.update(copy_ability=False),c.update(operations=[{'kind':'start'},{'kind':'tick','delta':bits(.1)}])))
 for speed in [.013,1.7,1000]:add('speed_'+str(speed),lambda c,v=speed:c.update(speed=bits(v)))
 add('stationary_target',lambda c:c['motion']['2'].update(speed=0))
 add('source_absent',lambda c:(c['execution']['context'].update(source=None),c['execution']['weapon_case']['body_input'].update(source_present=False)))
 return [{**copy.deepcopy(c),'name':c['name']+'_'+p,'precision':p,'execution':{**copy.deepcopy(c['execution']),'precision':p,'weapon_case':{**copy.deepcopy(c['execution']['weapon_case']),'body_input':{**copy.deepcopy(c['execution']['weapon_case']['body_input']),'precision':p}}}} for p in ['Single24','Double53','Extended64']for c in rows]
def main():
 rows=[];visited={}
 for c in cases():
  try:m=Machine(c);out=m.run()
  except Exception as exc:raise RuntimeError(c['name']) from exc
  rows.append({'input':c,'steps':out});visited.update(m.executed)
 doc={'schema':1,'passed':True,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':body.pe.ORIGINAL_SHA,'accepted_sha256':body.pe.ACCEPTED_SHA,'method':__doc__,'cases':rows,'visited_windows':[{'address':hex(a),'bytes':body.IMAGE.at(a,n).hex()} for a,n in sorted(visited.items())]}
 (HERE/'native-projectile-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf-8');print(json.dumps({'passed':True,'sequences':len(rows),'transitions':sum(len(r['steps'])for r in rows)}))
if __name__=='__main__':main()
