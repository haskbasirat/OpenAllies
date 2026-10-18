"""Read-only original/v14 equality and instruction decoding for bounded leaves."""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
md=Cs(CS_ARCH_X86,CS_MODE_32)
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
rows=[]
for fixture,producer,count,pins,excluded in[
 ('native-tail-fields.json','native_tail_fields.py',48,108,{0x6f7810,0x6f7430,0x43fcf1}),
 ('native-trigger-vfx.json','native_trigger_vfx.py',45,138,{0x6f7810,0x6f7430,0x427bd0,0x43ff36,0x494380,0x494480}),
]:
 doc=json.loads((HERE/fixture).read_text());assert doc['producer_sha256']==H(HERE/producer)
 assert len(doc['rows'])==count and len(doc['pins'])==pins
 for address,raw in doc['pins'].items():
  a=int(address,16);assert a not in excluded
  data=bytes.fromhex(raw);assert h.P.range(a,len(data))==data==h.V.range(a,len(data))
  decoded=list(md.disasm(data,a));assert len(decoded)==1 and decoded[0].size==len(data)
 rows.append(dict(fixture=fixture,sha256=H(HERE/fixture),producer=producer,producer_sha256=H(HERE/producer),
  paired_rows=count,native_executions=2*count,executed_instruction_pins=pins,excluded_service_or_observed_entry_pcs=[hex(a)for a in sorted(excluded)]))
output=dict(schema=1,scope='Bounded original/v14 executed instruction equality+single-instruction decode only. No seam/cutoff execution, wholeVWorld, camera, source resource40 null, nativeGPU or AD readiness claim.',fixtures=rows)
data=(json.dumps(output,indent=2)+'\n').encode();path=HERE/'pin-audit.json'
if '--write'in sys.argv:path.write_bytes(data)
else:assert path.read_bytes()==data
print(json.dumps(dict(executions=sum(r['native_executions']for r in rows),pins=sum(r['executed_instruction_pins']for r in rows),sha256=hashlib.sha256(data).hexdigest())))
