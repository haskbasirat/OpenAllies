"""Independent reversed-input replay of Phase6 actor-definition/native constructor evidence."""
from pathlib import Path
import json,sys,struct,time
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE))
import typed_scalar_oracle as scalar
import definition_flags_oracle as flags
import category_oracle as categories
from load_constructors_oracle import Machine,WORDS,string
from unicorn.x86_const import *
from original_creation import P,V,H

def main():
 counts={};inputs={};started=time.monotonic()
 for name in ['typed-scalar-native.json','definition-flags-native.json','categories-native.json','load-constructors-native.json','created-actors-native.json']:
  p=HERE/name;inputs[name]=H(p.read_bytes())
 scalar_proof=json.loads((HERE/'typed-scalar-native.json').read_text())
 for name,word in [('Extended64',0x37f),('Double53',0x27f),('Single24',0x7f)]:
  m=scalar.Machine(word)
  for row in reversed(scalar_proof['cases']):
   if row['precision']!=name:continue
   actual=m.scalar(row['text'],row['kind']);assert actual=={k:v for k,v in row.items()if k!='precision'},row
 counts['scalar_cases']=len(scalar_proof['cases'])
 d=json.loads((HERE/'definition-flags-native.json').read_text());m=flags.Machine(0x27f)
 for row in reversed(d['flag_cases']):assert m.flags(row['initial'],row['values'],row['is_actor'])==row['result'],row
 counts['base_flag_cases']=len(d['flag_cases'])
 for branch in reversed(d['component_branches']):
  for row in reversed(branch['cases']):
   m.ends=[int(branch['store_end'],16),int(branch['absent'],16)]
   m.w(m.definition+0x174,row['initial']);m.w(m.stack,0)
   for r in [UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_ECX,UC_X86_REG_EDX]:m.u.reg_write(r,0)
   m.u.reg_write(UC_X86_REG_EBP,m.definition);m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_EAX,0x2256000 if row['present'] else 0)
   m.u.emu_start(int(branch['start'],16),0,count=1000)
   assert m.u.reg_read(UC_X86_REG_EIP) in m.ends
   assert m.r(m.definition+0x174)==row['result'],(branch,row)
 counts['component_branches']=sum(len(b['cases'])for b in d['component_branches']);m.ends=[]
 for row in reversed(d['performer_filters']):
  m.w(m.definition+0x174,row['mask']);m.w(m.definition+0x1f0,row['ability_count']);m.u.reg_write(UC_X86_REG_EBP,m.definition)
  m.u.emu_start(0x46886c,0x46889e,count=1000);assert m.r(m.definition+0x174)==row['result'],row
 counts['performer_filters']=len(d['performer_filters'])
 m.w(m.definition,0x2256100);m.w(0x225613c,0x4c9fa0);m.w(m.definition+0x18,0x2256200)
 for row in reversed(d['final_actor_flags']):
  m.w(m.definition+0x174,row['mask']);m.w(m.definition+0x228,row['speed_bits']);m.w(0x225620c,row['category_index']);m.w(m.definition+0x1c,row['initial_flags'])
  m.u.reg_write(UC_X86_REG_EBP,m.definition);m.u.reg_write(UC_X86_REG_ESP,m.stack)
  m.u.emu_start(0x468995,0x468a43,count=1000);assert m.r(m.definition+0x1c)==row['result'],row
 counts['final_actor_flags']=len(d['final_actor_flags'])
 c=categories.Machine(0x27f);c.rows=[];owner,manager,entries=0x225a000,0x225b000,0x225c000
 c.w(owner+0x3b4,manager);c.w(manager+0x24,entries);c.w(manager+0x28,0);c.w(manager+0x2c,100);c.w(manager+0x34,64)
 c.invoke(0x476700,owner,[])
 assert c.rows==json.loads((HERE/'categories-native.json').read_text())['categories']
 assert [row['registry_index']for row in c.rows]==list(range(43));counts['category_descriptors']=len(c.rows)
 d=json.loads((HERE/'load-constructors-native.json').read_text())
 for row in reversed(d['cases']):
  kind,saved=row['kind'],row['saved_id'];m=Machine(0x27f);m.u.mem_write(m.obj,b'\xcd'*512);m.call(0x4efb70,[saved])
  m.invoke({'Thing':0x4ef020,'Object':0x4ea690,'Actor':0x4d6d70}[kind],m.obj,[0,m.definition,saved])
  actual={'flags':m.r(m.obj+8),'word_c':m.r(m.obj+12),'word_10':m.r(m.obj+16),'id':m.r(m.obj+20),'name_utf16':string(m,m.r(m.obj+24)),'word_1c_untouched':m.r(m.obj+28),'position_bits':[m.r(m.obj+32+i*4)for i in range(3)],'facing_bits':[m.r(m.obj+44+i*4)for i in range(3)]}
  if kind!='Thing':actual['object']={'associated_handle':m.r(m.obj+0x38),'scale_bits':m.r(m.obj+0x3c),'variant':m.r(m.obj+0x40),'word_44':m.r(m.obj+0x44),'word_48':m.r(m.obj+0x48),'byte_4c':m.u.mem_read(m.obj+0x4c,1)[0],'words_50_to_5c':[m.r(m.obj+0x50+i*4)for i in range(4)]}
  if kind=='Actor':actual['actor']={'components':[m.r(m.obj+0x60+i*4)for i in range(31)],'state':[m.r(m.obj+0x100),m.r(m.obj+0x104)],'string_dc_utf16':string(m,m.r(m.obj+0xdc)),'string_e0_utf16':string(m,m.r(m.obj+0xe0)),'words':{str(i*4):m.r(m.obj+i*4)for i in WORDS}}
  assert actual==row['constructor'];assert m.snapshot()==row['registry']
 counts['saved_constructors']=len(d['cases'])
 tables={t['precision']:t['values']for t in json.loads((ROOT/'research/phase5-actors/creation-native-oracle.json').read_text())['tables']}
 d=json.loads((HERE/'created-actors-native.json').read_text())
 for row in reversed(d['cases']):
  v=row['input'];m=Machine({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[v['precision']]);m.use_table(tables[v['startup_precision']]);m.u.mem_write(m.obj,b'\xcd'*512)
  for offset,value in [(4,m.definition),(0x1c,m.string(v['name_utf16'])),(0x20,v['variant']&0xffffffff),(0x24,v['saved_id']),(0x28,v['owner_word']),(0x30,v['word_30'])]:m.w(m.record+offset,value)
  for i,value in enumerate(v['position_bits']):m.w(m.record+8+i*4,value)
  for i,value in enumerate(v['direction_bits']):m.w(m.record+0x14+i*4,value)
  m.w(m.definition+0x1c,v['definition_flags']);m.w(m.world+0xe0,v['world_begin_bits']);m.w(m.world+0xd4,v['rng_state'])
  m.u.mem_write(m.definition+0x108,bytes([int(v['variations_enabled'])]));m.w(m.definition+0x10c,0x225a000);m.w(m.definition+0x128,v['total_weight_bits']);m.w(m.definition+0x28,v['scale_min_bits']);m.w(m.definition+0x2c,v['scale_max_bits'])
  for i,weight in enumerate(v['weights_bits']):p=0x225b000+i*0x40;m.w(0x225a000+i*4,p);m.w(p+0x2c,weight)
  m.invoke(0x4d6bb0,m.obj,[m.record])
  actual={'flags':m.r(m.obj+8),'name_utf16':string(m,m.r(m.obj+24)),'created_at_bits':m.r(m.obj+28),'position_bits':[m.r(m.obj+32+i*4)for i in range(3)],'facing_bits':[m.r(m.obj+44+i*4)for i in range(3)],'variant':m.r(m.obj+0x40),'actor_words':{str(i*4):m.r(m.obj+i*4)for i in WORDS},'components':[m.r(m.obj+0x60+i*4)for i in range(31)],'state':[m.r(m.obj+0x100),m.r(m.obj+0x104)]}
  assert actual==row['constructor'];assert m.r(m.record+0x28)==row['record_owner_after'];assert m.r(m.world+0xd4)==row['rng_after_constructor']
  assert bool(m.invoke(0x4e9fc0,m.obj,[m.record])&255)==row['initialized']
  assert m.r(m.obj+20)==row['id'];assert m.r(m.obj+0x3c)==row['scale_bits'];assert m.r(m.world+0xd4)==row['rng_after_initialize']
 counts['new_actor_constructors']=len(d['cases'])
 for name,digest in inputs.items():assert H((HERE/name).read_bytes())==digest,'Fixture changed during independent replay'
 report={'schema':1,'passed':True,'reviewer':'terrain_roads','method':'Separate reverse-order replay using native-machine harness; original evidence/production files not modified. Confirmed pointer/ID/class ordering and exact bits at declared seams.','counts':counts,'seconds':time.monotonic()-started,'input_sha256':inputs,'original_sha256':H(P.b),'v14_sha256':H(V.b),'replay_script_sha256':H(Path(__file__).read_bytes())}
 (HERE/'independent-replay.json').write_text(json.dumps(report,indent=2));print(json.dumps(report))
if __name__=='__main__':main()
