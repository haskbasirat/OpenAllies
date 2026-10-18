"""Actual546AF0 tutorial world actions, control/admission and field stores.
Actor command admission/handler, property applicability/insert, shared name and
definition lookup are explicit services; no opcode wrapper is replaced.
"""
from pathlib import Path
import copy,hashlib,itertools,json,struct,sys
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
import native_core_program as core
from unicorn.x86_const import *
HOOK=core.BASE+0x9000
class Machine(core.Machine):
 def __init__(self,c):
  self.world_events=[];super().__init__(c)
  self.actor=self.alloc(0x500);self.kingdom=self.alloc(0x500);self.property=self.alloc(0x100);self.component=self.alloc(0x100);self.other=self.alloc(0x100);self.mind=self.alloc(0x100);self.definition=self.alloc(0x100);self.data=self.alloc(0x1000);self.vt=self.alloc(0x300)
  self.w(self.actor,self.vt);self.w(self.actor+8,c['actor_flags']);self.w(self.actor+0x14,0x10003);self.w(self.actor+0x78,self.component if c['has_properties'] else 0);self.w(self.actor+0x70,self.mind)
  self.w(self.vt+0x38,HOOK);self.w(self.mind,self.vt+0x200);self.w(self.vt+0x224,HOOK+16)
  self.w(self.kingdom+8,c['kingdom_flags']);self.u.mem_write(self.kingdom+0x238,bytes([c['victory']]))
  self.w(self.kingdom+0x284,self.linked([self.actor]) if c['kingdom_members'] else 0)
  self.w(0xb36530,self.data);self.w(self.data+0x298,self.data+0x800);self.w(self.data+0x818,self.definition);self.w(self.definition+0x18,4)
  registry=self.alloc(0x60010);self.w(0xb364c0,registry)
  for slot,p in [(2,self.kingdom),(3,self.actor)]:self.u.mem_write(registry+4+slot*2,struct.pack('<H',1));self.w(registry+0x20004+slot*4,p)
  self.w(core.MANAGER+0x10,c['condition_actor']);self.w(core.MANAGER+0x20,self.linked([c['event_actor']]) if c['event_actor'] is not None else 0)
  for field,offset in [('letterbox',0x70),('screenfade',0x7c)]:
   state=c['presentation'][field];self.u.mem_write(core.WORLD+0x4c+offset,bytes([state['enabled']]));self.w(core.WORLD+0x4c+offset+4,state['started_bits']);self.w(core.WORLD+0x4c+offset+8,state['duration_bits'])
  # Invalid property-name warning setting is explicitly disabled in this corpus.
  settings=self.alloc(0x200);flag=self.alloc(4);self.w(0xb39940,settings);self.w(settings+0x104,flag)
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+i*4)
 def call(self,address,owner=0,args=None):
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  if address==0x546af0:
   for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  super().call(address,owner,args)
  if address==0x546af0:assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
 def hook(self,u,a,n,x):
  if 0x400000<=a<0xc00000:self.visited[a]=n
  if a in [0x4f8e90,0x4f8f00]:
   name=self.text(self.r(self.arg()));kind='ActorName' if a==0x4f8e90 else 'KingdomName';self.world_events.append({kind:name});self.ret(4,value=(self.actor if a==0x4f8e90 else self.kingdom) if name==('actor_a' if a==0x4f8e90 else 'kingdom_a') else 0)
  elif a==0x463fa0:
   name=self.text(self.arg());self.world_events.append({'PropertyName':name});self.ret(4,value=self.property if name=='indispensable' else 0)
  elif a==0x5035e0:
   assert [self.arg(),self.arg(1)]==[self.actor,self.property];self.world_events.append({'Applicable':[3,4]})
   if self.c['replace_properties']:self.w(self.actor+0x78,self.other)
   self.ret(value=int(self.c['applicable']))
  elif a==0x581630:
   p=self.arg();component=u.reg_read(UC_X86_REG_ECX);self.world_events.append({'Insert':{'component':6 if component==self.other else 5,'kind':self.r(p+8),'created':self.r(p+4),'definition':4,'source_id':self.r(p+20),'disabled':bool(self.u.mem_read(p+16,1)[0])}});self.ret(4,value=0)
  elif a==HOOK:
   assert u.reg_read(UC_X86_REG_ECX)==self.actor and self.arg()==self.definition;self.world_events.append('Admission');self.ret(4,value=int(self.c['admit']))
  elif a==HOOK+16:
   assert u.reg_read(UC_X86_REG_ECX)==self.mind;q=self.arg();assert self.r(q+8)==self.definition and self.r(q+0xc)==0x40 and self.r(q+0x34)==0;self.world_events.append('Process');self.ret(4,value=int(self.c['process_result']))
  else:super().hook(u,a,n,x)
 def run(self):
  result=super().run();result['world_events']=self.world_events;result['actor_flags']=self.r(self.actor+8);result['kingdom_flags']=self.r(self.kingdom+8);result['victory']=self.byte(self.kingdom+0x238);result['property_component']=0 if not self.r(self.actor+0x78) else 6 if self.r(self.actor+0x78)==self.other else 5
  result['presentation']={field:{'enabled':self.byte(core.WORLD+0x4c+offset),'started_bits':self.r(core.WORLD+0x4c+offset+4),'duration_bits':self.r(core.WORLD+0x4c+offset+8)} for field,offset in [('letterbox',0x70),('screenfade',0x7c)]};return result
def cases():
 base={'time_bits':core.bits(10),'variables':{'duration':core.bits(.25)},'triggers':[{'id':10,'enabled':1,'activation_bits':core.bits(1)}],'actor_flags':0x1800020,'kingdom_flags':0x80000020,'victory':7,'kingdom_members':True,'has_properties':True,'applicable':True,'replace_properties':False,'admit':True,'process_result':False,'condition_actor':0x10003,'event_actor':0x10003,'presentation':{'letterbox':{'enabled':9,'started_bits':0x3f800000,'duration_bits':0xbf800000},'screenfade':{'enabled':8,'started_bits':0x40000000,'duration_bits':0xc0000000}}};rows=[]
 def add(name,op,params,**kw):rows.append({**copy.deepcopy(base),'name':name,'element':{'name':core.NAME[op],'opcode':op,'parameters':params},**kw})
 p=core.param
 for op,value,duration in itertools.product([43,44],[0,1,2,0xffffffff],[0,-1,16777217]):add(f'presentation{op}_{value}_{duration}',op,[p('ma_boolean','mi_boolean',value),p('ma_value','mi_int',duration&0xffffffff)])
 add('presentation_variable',43,[p('ma_boolean','mi_boolean',1),p('ma_value','mi_variable','duration')])
 add('bool_wrong_item_defaults_false',44,[p('ma_boolean','mi_int',1),p('ma_value','mi_fixed',core.bits(2))])
 for op in [61,62,114]:
  for item,value in [('mi_kingdom_named','kingdom_a'),('mi_kingdom_named','missing'),('mi_kingdom_id',0x10002),('mi_kingdom_id',0x20002),('mi_kingdom_any',0)]:
   args=[p('ma_kingdom',item,value)]+([p('ma_boolean','mi_boolean',0)] if op==114 else []);add(f'kingdom{op}_{item}_{value}',op,args)
 for op in [63,64]:
  for item,value in [('mi_actor_named','actor_a'),('mi_actor_named','missing'),('mi_actor_id',0x10003),('mi_actor_id',0x20003),('mi_actor_any',0),('mi_actor_condition',0),('mi_actor_event',0)]:add(f'actor{op}_{item}_{value}',op,[p('ma_actor',item,value)])
 for name,kw in [('no_members',{'kingdom_members':False}),('admission_false',{'admit':False}),('handler_true',{'process_result':True})]:add(name,62,[p('ma_kingdom','mi_kingdom_named','kingdom_a')],**kw)
 for name,kw in [('applicable',{}),('not_applicable',{'applicable':False}),('no_properties',{'has_properties':False}),('replaced_properties',{'replace_properties':True})]:add(name,54,[p('ma_actor','mi_actor_named','actor_a'),p('ma_technology','mi_technology','indispensable')],**kw)
 for actor,property in [('actor_a',''),('missing','indispensable'),('actor_a','missing')]:add(f'property_{actor}_{property}',54,[p('ma_actor','mi_actor_named',actor),p('ma_technology','mi_technology',property)])
 add('empty_current_subject',64,[p('ma_actor','mi_actor_event',0)],event_actor=None)
 add('stale_condition',64,[p('ma_actor','mi_actor_condition',0)],condition_actor=0x20003)
 for pc in ['Single24','Double53','Extended64']:
  for row in rows:yield {**copy.deepcopy(row),'precision':pc,'name':row['name']+'_'+pc}
def main():
 rows=[];visited={}
 for c in cases():
  try:m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
  except Exception as e:raise RuntimeError(c['name']+' at '+hex(m.u.reg_read(UC_X86_REG_EIP))) from e
 windows=[]
 for a,n in sorted(visited.items()):
  raw=core.IMAGE.at(a,n);assert raw==core.ACCEPTED.at(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'original_sha256':core.pe.ORIGINAL_SHA,'v14_sha256':core.pe.ACCEPTED_SHA,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,'visited_windows':windows,'seams':['name table actor/Kingdom helpers with class-valid fixture results','463FA0 SharedProperty lookup; invalid-ID warning setting false','5035E0 applicability;581630 insertion (both separately ported)','actor command admission+38 and component handler+24; no SAI handle in this corpus','string allocation/assignment, variable dictionary lookup and logging from core oracle','heap allocation'],'scope':'Actual546AF0 world opcodes43/44/54/61/62/63/64/114; real selectors, presentation/victory fields, control routing, property admission and instance initialization. Actual live runtime binding remains separate.'}
 path=HERE/'native-world-program.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
