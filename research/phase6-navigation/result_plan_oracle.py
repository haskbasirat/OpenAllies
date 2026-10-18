"""Original ResultPlan controller and turning geometry with explicit world seams."""
from pathlib import Path
import copy,hashlib,json,random,struct,sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase5-navigation'))
import native_oracle as base
from terrain_uv_oracle import Pe,ORIGINAL_SHA,ACCEPTED_SHA
from unicorn.x86_const import *
TRIGGER,TARGET,REPLACEMENT,HIT,COMP,REGISTRY=[0x2020000+i*0x1000 for i in range(5)]+[0x2600000]
SPECIAL=base.STUB+0x200;PROPERTY=base.STUB+0x210;FAILED=base.STUB+0x220
bits=base.bits
class Machine(base.Machine):
 def __init__(self,pe,c):
  n=copy.deepcopy(base.cases()[0]);n['precision']=c['precision'];n['state']=c['state']['movement'];super().__init__(pe,n);self.plan=c
  self.u.mem_map(REGISTRY,0x50000);self.put(0xb364c0,REGISTRY);self.put(base.ACTOR,base.VT);self.put(base.VT+0xb4,SPECIAL);self.put(base.VT+0x108,PROPERTY);self.put(base.VT+0x124,FAILED)
  self.put(base.ACTOR+0x20,*c['actor']['position_bits']);self.put(base.ACTOR+0x2c,*c['actor']['facing_bits']);self.put(base.MOVER+0x30,c['actor']['turn_radius_bits']);self.put(base.STATE+0x28,c['state']['attempts']);self.put(base.STATE+0x34,*c['state']['previous_destination_bits'])
  for obj in [TRIGGER,TARGET,REPLACEMENT,HIT]:self.put(obj,base.VT,0x77770000+obj)
  for obj in [TRIGGER,HIT]:self.put(obj+0x90,int(c['special_components'].get(str(obj),False)))
  self.u.mem_write(REGISTRY+4+7*2,struct.pack('<H',1 if c['target_valid'] else 2));self.put(REGISTRY+0x20004+7*4,TARGET)
  self.u.mem_write(REGISTRY+4+8*2,struct.pack('<H',1));self.put(REGISTRY+0x20004+8*4,HIT)
  if c['replacement']:self.put(TARGET+0x7c,COMP);self.put(COMP+0x40,REPLACEMENT)
  self.region_index=0;self.property_index=0;self.events=[];self.executed={}
 def event(self,name,**payload):
  self.events.append({'name':name,**payload})
  change=self.plan.get('mutations',{}).get(name,{})
  for key,value in change.get('movement',{}).items():
   off,count=base.STATE_FIELDS[key];self.put(base.STATE+off,*(value if count>1 else [value]))
  for key,value in change.get('actor',{}).items():
   addr={'position_bits':base.ACTOR+0x20,'facing_bits':base.ACTOR+0x2c,'turn_radius_bits':base.MOVER+0x30}[key];self.put(addr,*(value if isinstance(value,list) else[value]))
 def hook(self,u,a,size,_):
  if 0x400000<=a<0xc00000:self.executed[a]=size
  if a==base.STOP:u.emu_stop();return
  if a==SPECIAL:
   obj=u.reg_read(UC_X86_REG_ECX);result=self.plan['special_types'].get(str(obj),False);self.event('SpecialType',object=obj,result=result);self.ret(value=int(result));return
  if a==PROPERTY:
   result=self.plan['properties'][min(self.property_index,len(self.plan['properties'])-1)];self.property_index+=1;self.event('Property',object=u.reg_read(UC_X86_REG_ECX),result=result);self.ret(value=result);return
  if a==FAILED:self.event('Failed');self.ret();return
  if a==0x4f2220:
   result=self.plan['regions'][min(self.region_index,len(self.plan['regions'])-1)];self.region_index+=1;self.event('Region',point=self.args(2),result=result);self.ret(8,result);return
  if a==0x554350:
   addr=u.reg_read(UC_X86_REG_ECX);obj,arg=self.args(2);assert arg==0;self.event('Context',object=obj);self.put(addr,*self.plan['context_words']);self.ret(8,addr);return
  if a==0x553ce0:
   out,context,x,y=self.args(4);self.event('Sweep',context=self.ints(context,22),goal=[x,y]);self.put(out,self.plan['hit']['kind'],self.plan['hit']['actor_id'],0,0);self.ret(16,out);return
  if a==0x5313f0:self.event('ContinueTurn',turn=bool(self.args(1)[0]));self.ret(4);return
  if a==0x531520:self.event('ContinueCurrent',argument=self.args(1)[0]);self.ret(4);return
  if a==0x76b9a0:self.event('PropertyActive',property=self.args(1)[0],result=self.plan['property_active']);self.ret(4,int(self.plan['property_active']));return
  if a==0x53e1f0:
   path,x,y,gx,gy,context,option=self.args(7);assert path==base.PATH
   self.event('Solve',context=self.ints(context,22),request={'start_bits':[x,y],'goal_bits':[gx,gy],'option_word':option},result=self.plan['solve_result']);self.ret(28,self.plan['solve_result']);return
  if a==0x52fd90:self.event('Offset');self.ret();return
  if a==0x5300a0:self.event('ActionGo',speed_bits=self.args(1)[0]);self.ret(4);return
  super().hook(u,a,size,_)
 def run(self):
  stack=0x3008000;self.put(stack,base.STOP,self.plan['trigger'] or 0,self.plan['argument']);self.u.reg_write(UC_X86_REG_ESP,stack);self.u.reg_write(UC_X86_REG_ECX,base.STATE)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(0x5336b0,base.STOP,count=100000)
  assert self.u.reg_read(UC_X86_REG_ESP)==stack+12 and all(self.u.reg_read(r)==0x11111111*(i+1) for i,r in enumerate(regs))
  move={}
  for name,(off,count) in base.STATE_FIELDS.items():v=self.ints(base.STATE+off,count);move[name]=v if count>1 else v[0]
  move['path']=self.read_path(base.PATH)
  state={'movement':move,'attempts':self.ints(base.STATE+0x28)[0],'previous_destination_bits':self.ints(base.STATE+0x34,2)}
  if state['attempts']>=0x80000000:state['attempts']-=0x100000000
  actor={'object':base.ACTOR,'position_bits':self.ints(base.ACTOR+0x20,2),'facing_bits':self.ints(base.ACTOR+0x2c,2),'turn_radius_bits':self.ints(base.MOVER+0x30)[0]}
  return {'state':state,'actor':actor,'events':self.events,'abi_preserved':True}
def cases():
 movement=copy.deepcopy(base.cases()[0]['state']);movement.update(destination_bits=list(map(bits,[20,20])),phase=1,mode=0)
 words=[0]*22;words[9]=0xabcdef;words[14]=0x20
 seed={'name':'base','precision':'Double53','state':{'movement':movement,'attempts':0,'previous_destination_bits':[bits(99),bits(98)]},'actor':{'object':base.ACTOR,'position_bits':[bits(10),bits(10)],'facing_bits':[bits(1),0],'turn_radius_bits':bits(4)},'trigger':TRIGGER,'argument':0x12345678,'special_types':{},'special_components':{},'target_valid':True,'replacement':False,'regions':[1,1],'properties':[0],'property_active':False,'context_words':words,'hit':{'kind':0,'actor_id':0x10008},'solve_result':1,'mutations':{}}
 rows=[]
 def add(name,**parts):
  c=copy.deepcopy(seed);c['name']=name
  for k,v in parts.items():
   if k=='movement':c['state']['movement'].update(v)
   elif k in ('state','actor'):c[k].update(v)
   else:c[k]=v
  rows.append(c)
 add('normal');add('failure',solve_result=0);add('partial_success',solve_result=2)
 for tries in [2,3,4,-1,2147483647]:add('attempt_'+str(tries),state={'attempts':tries})
 for phase in [0,1,2]:add('near_phase_'+str(phase),movement={'mode':2,'phase':phase,'destination_bits':[bits(11),bits(11)]})
 for target in [True,False]:add('target_'+str(target),target_valid=target,replacement=True)
 for prop,active in [([1,2],True),([1,0],True),([1,2],False)]:add('property_'+str(prop)+'_'+str(active),properties=prop,property_active=active)
 near={'mode':2,'phase':1,'destination_bits':[bits(11),bits(11)]}
 add('near_different_region',movement=near,regions=[1,2]);add('near_no_trigger',movement=near,regions=[1,2],trigger=None)
 add('near_special_trigger',movement=near,regions=[1,2],special_types={str(TRIGGER):True},special_components={str(TRIGGER):True})
 for special,component in [(False,False),(True,False),(True,True)]:add('hit_'+str(special)+'_'+str(component),movement=near,regions=[1,2],hit={'kind':2,'actor_id':0x10008},special_types={str(HIT):special},special_components={str(HIT):component})
 add('context_mutates_destination',mutations={'Context':{'movement':{'destination_bits':[bits(22),bits(24)]}}})
 add('property_mutates_position',properties=[1,2],property_active=True,mutations={'Property':{'actor':{'position_bits':[bits(12),bits(14)]}}})
 add('offset_mutates_phase',mutations={'Offset':{'movement':{'phase':7}}})
 for facing in [[1,0],[-1,0],[0,1],[0,-1]]:
  for delta in [[1,1],[1,-1],[-1,1],[-1,-1],[4,4],[0,8],[0,0]]:add('turn_'+str(facing)+'_'+str(delta),movement={'phase':0,'destination_bits':[bits(10+delta[0]),bits(10+delta[1])]},actor={'facing_bits':list(map(bits,facing))})
 rng=random.Random(5336)
 for i in range(30):add('fractional_'+str(i),movement={'phase':0,'destination_bits':[bits(rng.uniform(5,15)),bits(rng.uniform(5,15))]},actor={'facing_bits':list(map(bits,[rng.uniform(-1,1),rng.uniform(-1,1)])),'turn_radius_bits':bits(rng.uniform(.1,10))})
 return [{**copy.deepcopy(c),'name':c['name']+'_'+p,'precision':p} for p in ['Single24','Double53','Extended64'] for c in rows]
def main():
 pe=Pe(ROOT.parent/'AA.exe',ORIGINAL_SHA);v14=Pe(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe',ACCEPTED_SHA);fixtures=[];executed={}
 for c in cases():
  try:m=Machine(pe,c);output=m.run()
  except Exception as e:raise RuntimeError(c['name']) from e
  fixtures.append({'input':c,'output':output});executed.update(m.executed)
 for a,size in executed.items():assert pe.at(a,size)==v14.at(a,size)
 doc={'schema':1,'passed':True,'original_sha256':ORIGINAL_SHA,'v14_sha256':ACCEPTED_SHA,'oracle_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'case_count':len(fixtures),'seams':['required world/controller callbacks','collision descriptor constructor','Solve leaf separately covered by whole-solver oracle'],'fixtures':fixtures}
 (OUT/'result-plan-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n');(OUT/'result-plan-ranges.json').write_text(json.dumps([{'address':hex(a),'bytes':pe.at(a,n).hex()} for a,n in sorted(executed.items())],indent=2)+'\n');print(json.dumps({'passed':True,'cases':len(fixtures)}))
if __name__=='__main__':main()
