"""Original57F4D0/57DB90 and4C8650/4C86D0 construction.
Native base constructors, vector growth, angle/main-power getters execute.
Only allocation/free, actor ability notification and property refresh are hooks.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine as Base,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
PERF,OWNER,DEF,ARRAY,LOCAL,SHARED,TARRAY,TINFO,PROP=range(0x4000000,0x4009000,0x1000)
bits=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
class Machine(Base):
 def __init__(self,c):
  super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.c=copy.deepcopy(c)
  self.u.mem_map(0,4096);self.u.mem_map(0x4000000,0x40000);self.heap=0x4020000;self.events=[];self.actions=[];self.visited={}
  self.w(DEF+0x2b0,c.get('speed',0));self.w(OWNER+4,DEF);self.w(OWNER+0x78,PROP if c['properties'] else 0);self.w(DEF+0x1ec,ARRAY);self.w(DEF+0x1f0,len(c['abilities']));self.w(DEF+0x1e0,c['energy'])
  for i,a in enumerate(c['abilities']):
   self.w(ARRAY+i*4,LOCAL+i*0x80);self.w(LOCAL+i*0x80,SHARED+i*0x100);self.w(SHARED+i*0x100+0x64,a['main_power'])
  self.w(DEF+0x1f8,TARRAY);self.w(DEF+0x1fc,len(c['angles']))
  for i,a in enumerate(c['angles']):self.w(TARRAY+i*4,TINFO+i*8);self.w(TINFO+i*8+4,a)
  for i,v in enumerate(c['startup']):self.w(0xb279c0+i*4,v)
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+i*4)
 def ret(self,value=0,pop=0):
  s=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.r(s));self.u.reg_write(UC_X86_REG_ESP,s+4+pop);self.u.reg_write(UC_X86_REG_EAX,value)
 def action(self,p):
  d=self.r(p+4);local=self.r(p+8)
  return {'definition':100+(d-SHARED)//0x100 if d else 0,'local_definition':200+(local-LOCAL)//0x80 if local else None,'use_count':self.r(p+0xc),'last_use':self.r(p+0x10),'word14':self.r(p+0x14),'enabled':bool(self.u.mem_read(p+0x18,1)[0]),'recharge_bypass':bool(self.u.mem_read(p+0x19,1)[0]),'fuzzy_range':self.r(p+0x1c),'power':self.r(p+0x20),'summoned_ids':[],'word28':self.r(p+0x28)}
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:
   raw=original.range(pc,size);assert raw==modified.range(pc,size);self.visited[pc]=raw.hex()
  if pc==0x6f7810:
   p=self.heap;self.heap+=(self.arg()+15)&~15;self.ret(p)
  elif pc==0x6f7430:self.ret()
  elif pc in [0x4c86ae,0x4c872c]:
   a=self.action(u.reg_read(UC_X86_REG_ESI));self.actions.append(a);self.events.append({'name':'allocate','ability':a})
  elif pc==0x4d1350:
   self.events.append({'name':'notify','owner':1});self.w(DEF+0x1e0,self.c['energy_after_notify']);self.ret()
  elif pc==0x581420:
   self.events.append({'name':'refresh','properties':9,'immediate':bool(self.arg())});self.ret(pop=4)
 def call_native(self,entry,ecx,arg=None):
  self.reset(0);self.u.reg_write(UC_X86_REG_ECX,ecx)
  if arg is not None:self.w(self.STACK+4,arg)
  regs={r:0x12ee0000+i for i,r in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
  for r,v in regs.items():self.u.reg_write(r,v)
  self.u.emu_start(entry,self.STOP,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP
  assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+(8 if arg is not None else 4)
  assert all(self.u.reg_read(r)==v for r,v in regs.items())
  return self.u.reg_read(UC_X86_REG_EAX)
 def run(self):
  if self.c['kind']=='projectile':
   assert self.call_native(0x580ed0,PERF,OWNER)==PERF
   return {'owner':1,'source_id':self.r(PERF+0x10),'spawn_bone':self.r(PERF+0x14),'target_id':self.r(PERF+0x18),'origin':[self.r(PERF+0x1c+i*4)for i in range(3)],'target':[self.r(PERF+0x28+i*4)for i in range(3)],'ability':self.action(PERF+0x34),'remaining':self.r(PERF+0x60),'duration':self.r(PERF+0x64),'speed':self.r(PERF+0x68),'started_at':self.r(PERF+0x6c),'event_kind':self.r(PERF+0x70)}
  if self.c['kind']=='shared':
   assert self.call_native(0x4c86d0,PERF,SHARED)==PERF
   return {'actions':self.actions,'events':self.events}
  assert self.call_native(0x57f4d0,PERF,OWNER)==PERF
  state={'owner':1,'stage':self.r(PERF+0x10),'abilities':list(range(1,self.r(PERF+0x18)+1)),'mana':self.r(PERF+0x20),'target_id':self.r(PERF+0x28),'target_position':[self.r(PERF+0x2c),self.r(PERF+0x30)],'active_ability':None,'active_index':-1,'started_at':self.r(PERF+0x3c),'animation_length':self.r(PERF+0x40),'event_kind':self.r(PERF+0x44),'animation':None,'turret_index':-1}
  ts=[]
  for i in range(self.r(PERF+0x50)):
   p=self.r(self.r(PERF+0x4c)+i*4);ts.append({'current_angle':self.r(p),'target_angle':self.r(p+4),'time':self.r(p+8),'pending':bool(self.u.mem_read(p+0xc,1)[0]),'target':[self.r(p+0x10+j*4) for j in range(3)]})
  owned={'state':state,'maximum_mana':self.r(PERF+0x24),'turrets':ts,'turret_update_pending':bool(self.u.mem_read(PERF+0x5c,1)[0]),'word60':self.r(PERF+0x60)}
  ok=bool(self.call_native(0x57db90,PERF,0)&255)
  return {'owned':owned,'actions':self.actions,'events':self.events,'init_return':ok}
def cases():
 rows=[]
 for count in [0,1,2,3,4,10]:
  for props in [False,True]:
   rows.append({'name':f'abilities_{count}_properties_{props}','kind':'performer','energy':bits(25),'energy_after_notify':bits(37.25),'abilities':[{'main_power':bits((i-2)*1.375)} for i in range(count)],'angles':list(map(bits,[0,1.5707963705062866,6.283185307179586,-2,19.717]))[:count],'startup':list(map(bits,[17.1,-4.3333333])),'properties':props})
 for power in [-3,0,.333333333,1e-30,99999.99]:rows.append({'name':'shared_'+str(power),'kind':'shared','energy':0,'energy_after_notify':0,'abilities':[{'main_power':bits(power)}],'angles':[],'startup':[0,0],'properties':False})
 for speed in [0,-1,.333333333,1e-30,1,999999.5]:rows.append({'name':'projectile_'+str(speed),'kind':'projectile','speed':bits(speed),'energy':0,'energy_after_notify':0,'abilities':[],'angles':[],'startup':[0,0],'properties':False})
 return [{**copy.deepcopy(c),'precision':pc,'name':c['name']+'_'+pc}for pc in ['Single24','Double53','Extended64']for c in rows]
def main():
 rows=[];visited={}
 for c in cases():
  try:m=Machine(c);out=m.run()
  except Exception as e:raise RuntimeError(c['name']+' '+hex(m.u.reg_read(UC_X86_REG_EIP)))from e
  rows.append({'input':c,'output':out});visited.update(m.visited)
 report={'schema':1,'passed':True,'original_sha256':hashlib.sha256(original.b).hexdigest(),'accepted_sha256':hashlib.sha256(modified.b).hexdigest(),'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'method':__doc__,'cases':rows,'visited_windows':[{'address':hex(a),'bytes':b}for a,b in sorted(visited.items())]}
 (HERE/'native-constructor-fixtures.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8');print(json.dumps({'passed':True,'cases':len(rows)}))
if __name__=='__main__':main()
