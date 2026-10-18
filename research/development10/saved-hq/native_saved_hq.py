"""Native HQ factory4CF480/slot30 ->574490, required virtual580F70, Load574040.
Original andv14, all three explicit x87 PCs; declared stream/allocator/free/
owner-name/log leaves from the pinned preexisting saved-component harness.
No floating arithmetic is expected on this component path. Live restore uses
this same HqState kernel after insertion, preserving Load failure prefixes.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
OLD=ROOT/'research/phase8-saved-components';sys.path.insert(0,str(OLD))
import native_hq as h
import native_machine as m
from unicorn.x86_const import UC_X86_REG_FPCW
sha=lambda b:hashlib.sha256(b).hexdigest()
ORIGINAL=m.P;V14=m.V
QA=HERE/'tutorial02-v3.json'
def cases():
 rows=h.cases()
 q=json.loads(QA.read_text());raw=bytes(q['payload']['value']['Hq']['bytes'])
 rows.append({'name':'actual_tutorial02_hq','table_max':q['updatable_table_bound'],'occupied':[],
  'slots_530':q['source_slots_530'],'owner_name':'unreached synthetic display name',
  'kingdom':{'badges':q['nation_badge_count'],'nation':'unreached synthetic Nation name'},
  'payload':raw.hex(),'updatable_id':int.from_bytes(raw[4:8],'little',signed=True),
  'actual_owner_id':q['owner_e8'],'actual_nation':q['nation'],
  'scope':'Payload, constructor slot count, reached Nation badge count and table bound from actual tutorial02-v3 receipt. Owner/Nation display text and occupied slots are unreached laboratory fields because updatableID=-1 and badge3 is valid.'})
 return rows
class Machine(h.HqMachine):
 def run(self):
  self.call(0x4cf480,self.owner,[30]);hq=self.r(self.owner+0xd8)
  assert hq and self.r(hq)==0xa3ca4c and self.r(hq+8)==0xa3ca24 and self.r(hq+4)==self.owner
  assert self.call(0x580f70,hq,[])==0xffffffff
  after_ctor=self.state(hq)
  self.call(0x574040,hq,[self.ctx])
  out={'after_ctor':after_ctor,'outcome':self.outcome,'consumed':self.at,'calls':self.calls}
  if self.outcome is None or self.outcome['kind']!='NullDereference':out['state']=self.state(hq)
  uid=self.c['updatable_id']
  if uid is not None and 0<=uid<=self.c['table_max']+1:
   v=self.r(self.table_slots+4*uid);out['table_slot']='hq+8' if v==hq+8 else v
  if self.outcome is None:
   labels={hq:'component',self.r(hq+0x14):'slots_buffer',self.r(hq+0x20):'pending_buffer'}
   for i in range(self.r(hq+0x24)):labels[self.r(self.r(hq+0x20)+4*i)]='pending_item'
   frees=[];before_free=[]
   def free(machine):
    address=machine.arg(0);frees.append(labels.get(address,'other'))
    if address==hq:before_free.append(self.state(hq))
    machine.ret(0)
   self.seam(0x6f7430,free)
   self.call(0x5739e0,hq,[1]);assert len(before_free)==1
   out['destructor']={'state':before_free[0],'frees':frees,'scope':'State sampled BEFORE the final component free callback; direct scalar deleting destructor5739E0->573890, not PreDestroy574550 or removal of external attachments.'}
  return out

def main():
 rows=[];leaves=set()
 for image,pe,other in [('original',ORIGINAL,V14),('v14',V14,ORIGINAL)]:
  m.P=pe;m.V=other
  for pc,cw in m.CW.items():
   for case in cases():
    machine=Machine(case);machine.u.reg_write(UC_X86_REG_FPCW,cw)
    native=machine.run();leaves.update(machine.seams)
    rows.append({'image':image,'precision':pc,'input':case,'native':native})
   print(image,pc,len(cases()),flush=True)
 pins={hex(a):raw for a,raw in sorted(m.VISITED.items()) if a not in leaves}
 for a,raw in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(raw))).hex()==raw==V14.range(int(a,16),len(bytes.fromhex(raw))).hex()
 out={'schema':1,'scope':__doc__,'producer_sha256':sha(Path(__file__).read_bytes()),'legacy_harness_sha256':sha((OLD/'native_machine.py').read_bytes()),'legacy_hq_producer_sha256':sha((OLD/'native_hq.py').read_bytes()),'original_sha256':sha(ORIGINAL.b),'v14_sha256':sha(V14.b),'passed':True,'actual_tutorial_receipt_sha256':sha(QA.read_bytes()),'cases':rows,'executed_pins_excluding_leaf_entries':pins,'leaf_entries':[hex(a) for a in sorted(leaves)]}
 (HERE/'native-saved-hq.json').write_text(json.dumps(out,indent=2)+'\n')
 print('PASS',len(rows),'cases',len(pins),'pins',flush=True)
if __name__=='__main__':main()
