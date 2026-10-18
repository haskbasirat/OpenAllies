"""Full56F280/529C30/50AC60/periodic queue insertion and4B2830.

Original/v14 run independently with the real Economy and embedded Updatable
vtables. Allocation remains intercepted; queue/RNG/list code and the actual
inactive debug-recorder returns execute. Their fixture fields are controlled;
this is not proof of the native debug-recorder startup/active configuration.
Pre-existing events are inserted by the actual native queue wrapper before
the measured entry. No Economy refresh570020 occurs in this Init method.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase6-queues'))
import native_queues as q
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from unicorn.x86_const import *
H=lambda b:hashlib.sha256(b).hexdigest()
LEAVES={0x6f7810,0x6f7430,0x404f90}

class NativeQueue(q.Machine):
 def hook(self,u,pc,size,user):
  if pc in (0x77dc50,0x77da70):
   #Explicit inactive debug-recorder fixture, separate from the SAI owner.
   #Unlike the inherited harness, execute the actual early-return body.
   assert u.reg_read(UC_X86_REG_ECX)in self.observers
   assert bytes(u.mem_read(u.reg_read(UC_X86_REG_ECX),1))==b'\0'
   raw=q.ORIGINAL.range(pc,size);assert raw==q.ACCEPTED.range(pc,size)
   q.VISITED[pc]=raw.hex()
   return
  super().hook(u,pc,size,user)

def run(c,image):
 original=q.ORIGINAL
 q.ORIGINAL=original if image=='original'else q.ACCEPTED
 try:m=NativeQueue(c)
 finally:q.ORIGINAL=original
 #Explicit present inactive debug recorders. Their allocation/startup and
 #active tracing are not claimed here, and they are NOT the SAI B36528 owner.
 m.observers=[m.alloc(0x20),m.alloc(0x20)]
 for va,owner in zip([0xb3655c,0xb36560],m.observers):m.write(va,owner)
 def guard(u,access,address,size,value,user):
  assert address>=0x1000,('null-page access',hex(address),size)
 m.u.hook_add(UC_HOOK_MEM_READ|UC_HOOK_MEM_WRITE,guard)
 actor=m.alloc(0x180);definition=m.alloc(0x600);component=m.alloc(0x58)
 m.write(component,0xa3c690);m.write(component+4,actor);m.write(component+8,0xa3c668)
 m.write(actor+4,definition);m.u.mem_write(definition+0x348,bytes([int(c['supply_required'])]))
 m.owners={1:component+8};m.names={component+8:1}
 if c['scheduled']:m.call(0x50ac60,component+8,[99])
 before=dict(rng=m.read(m.world+0xd4),queues=m.snapshot())
 #A nonnull native head must avoid529C30/ActorData348 altogether.
 if c['scheduled']:m.write(actor+4,0xdead0001)
 keep=[(UC_X86_REG_EBX,0x11223344),(UC_X86_REG_ESI,0x33445566),(UC_X86_REG_EDI,0x55667788),(UC_X86_REG_EBP,0x778899aa)]
 for reg,value in keep:m.u.reg_write(reg,value)
 result=m.call(0x56f280,component)
 assert result&255==1
 assert all(m.u.reg_read(reg)==value for reg,value in keep)
 assert (m.u.reg_read(UC_X86_REG_FPSW)>>11)&7==0
 after=dict(rng=m.read(m.world+0xd4),queues=m.snapshot(),pending=m.read(component+0xc)!=0)
 m.call(0x4b2830,component)
 assert after==dict(rng=m.read(m.world+0xd4),queues=m.snapshot(),pending=m.read(component+0xc)!=0)
 return dict(before=before,after=after,result=True)

def main():
 rows=[]
 for pc,scheduled,supply,seed in itertools.product([24,53,64],[False,True],[False,True],[0,1,0x12345678,0xffffffff]):
  c=dict(precision=pc,scheduled=scheduled,supply_required=supply,seed=seed,counts=[1,16,80,80,80],clock=dict(begin_bits=0,end_bits=q.bits(.0625),step_bits=q.bits(.0625)))
  a=run(c,'original');b=run(c,'v14');assert a==b
  rows.append(dict(input=c,output=a))
 actual=HERE/'actual-economy-init-current.json';actual_rows=[]
 if not actual.exists():actual=HERE/'actual-economy-init.json'
 if actual.exists():
  for case in json.loads(actual.read_text())['cases']:
   c=case['input'];a=run(c,'original');b=run(c,'v14');assert a==b==case['output']
   actual_rows.append(dict(actor=case['actor'],source_precision=case['source_precision'],input=c,output=a,scope=case['scope']))
 pins=[dict(va=hex(a),bytes=b)for a,b in sorted(q.VISITED.items())if a not in LEAVES]
 leaves=[dict(va=hex(a),bytes=b)for a,b in sorted(q.VISITED.items())if a in LEAVES]
 data=[]
 for va in [0xa3c690+8,0xa3c690+0x14,0xa3c668+8,0xa30be0+0xc]:
  raw=q.ORIGINAL.range(va,4);assert raw==q.ACCEPTED.range(va,4)
  data.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),queue_harness_sha256=H((ROOT/'research/phase6-queues/native_queues.py').read_bytes()),original_sha256=H(q.ORIGINAL.b),v14_sha256=H(q.ACCEPTED.b),cases=rows,machines=(len(rows)+len(actual_rows))*2,executed_pins=pins,intercepted_entry_pins=leaves,data_pins=data)
 if actual.exists():doc.update(actual_input_sha256=H(actual.read_bytes()),actual_rows=actual_rows)
 raw=(json.dumps(doc,indent=1)+'\n').encode();out=HERE/'native-economy-init.json'
 if '--write'in sys.argv:out.write_bytes(raw)
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),machines=doc['machines'],executed_pins=len(pins),intercepted_entry_pins=len(leaves),sha256=H(raw),immutable='--write'not in sys.argv)))
if __name__=='__main__':main()
