"""Whole original GActor::Load through return for two actual tutorial start_loc records. Original Economy factory/ctor/Load/requirement, owner GTID binding and no-spatial/empty-name/unnumbered tail execute. Prior Thing/Object body and stream/string allocation remain separately-proven seams."""
from pathlib import Path
import sys,json,struct,hashlib
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase6-actor-payloads'))
from native_actor_payload import TailMachine,P,V
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP
state_path=ROOT/'research/phase7-actors/native-actor-states.json'
state_records=json.load(open(state_path))['registration']['records']
class Machine(TailMachine):
 def hook(self,u,pc,size,arg):
  if pc==0x4d8e1f:
   for record in state_records:
    descriptor=0x208c000+record['index']*0x40;self.w(0x208b000+record['index']*4,descriptor);self.w(descriptor+0x2c,record['group'])
  if pc in (0x4cf480,0x50b250):self.executed[pc]=bytes(u.mem_read(pc,size));return
  super().hook(u,pc,size,arg)
rows=[];visited={}
source=json.load(open(Path(__file__).with_name('start-location-source.json')))
for row in source['records']:
 record=row['record'];a=record['actor'];version=a['version'];owner=a['owner_handle'];tail=bytes(record['opaque_tail'])
 for ownable in [True,False]:
  for savedowner in [owner,0]:
   payload=struct.pack('<IIII',version,savedowner,a['state_low'],a['state_high'])+tail
   m=Machine(payload);m.w(m.thing+8,(row['runtime_flags']|(1<<23))if ownable else(row['runtime_flags']&~(1<<23)));m.w(m.thing+0x15c,0xffffffff)
   m.u.mem_write(m.definition+0x1b0,b'\0');m.w(0xb364c0,m.manager);ownerptr=0x208d000;m.w(m.manager+0x20004+(owner&0xffff)*4,ownerptr)
   m.w(0x208a000+0x38,7);m.w(m.world+0x2c,28)
   result=m.run_actor(version,512);m.u.emu_start(0x4d9714,m.stop,count=100000)
   assert m.u.reg_read(UC_X86_REG_EIP)==m.stop and m.u.reg_read(UC_X86_REG_ESP)==m.stack+8 and m.at==len(payload)
   result['owner_present']=m.r(m.thing+0xe8)==ownerptr;result['economy_present']=m.r(m.thing+0x84)!=0;result['state']=[m.r(m.thing+0x100),m.r(m.thing+0x104)]
   assert result['owner_present']==(ownable and savedowner!=0);assert result['economy_present'];visited.update(m.executed)
   rows.append({'source_id':record['serialized_id'],'ownable':ownable,'saved_owner':savedowner,'source_record':record,'tail':row['tail'],'runtime_flags':m.r(m.thing+8),'actor_hex':payload.hex(),'native':result})
for a,b in visited.items():assert b==V.range(a,len(b)),hex(a)
p=Path(__file__).with_name('native-start-location.json');p.write_text(json.dumps({'scope':__doc__,'source_sha256':hashlib.sha256(Path(__file__).with_name('start-location-source.json').read_bytes()).hexdigest(),'state_registration_sha256':hashlib.sha256(state_path.read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'cases':rows,'instructions':[{'address':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())]},indent=2)+'\n');print({'cases':len(rows),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
