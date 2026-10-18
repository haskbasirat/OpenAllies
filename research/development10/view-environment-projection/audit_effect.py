from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
doc=json.loads((HERE/'native-effect.json').read_text());assert doc['producer_sha256']==H(HERE/'native_effect.py')
md=Cs(CS_ARCH_X86,CS_MODE_32);pins=[]
for address,bits in doc['pins'].items():
 a=int(address,16);raw=bytes.fromhex(bits);assert h.P.range(a,len(raw))==h.V.range(a,len(raw))==raw
 ins=list(md.disasm(raw,a));assert len(ins)==1 and ins[0].size==len(raw)
 pins.append(dict(address=address,bytes=bits,instruction=ins[0].mnemonic+' '+ins[0].op_str))
for slot,value in doc['virtual_slots'].items():
 a=0xab70a0+int(slot,16);assert h.P.range(a,4)==h.V.range(a,4)==value.to_bytes(4,'little')
assert len(pins)==343 and len(doc['rows'])==234 and not doc['seams']
out=dict(schema=1,rows=234,native_executions=468,decoded_equal_executed_pins=pins,fixture_sha256=H(HERE/'native-effect.json'),
 source_sha256=H(ROOT/'crates/aa-runtime/src/view_texture_effect.rs'),tests_sha256=H(ROOT/'crates/aa-runtime/src/view_texture_effect_tests.rs'),
 typed_dispatch='AB70A0+48=86EED0,+68=86BC30,+64=4B2830; native pointer loads represented by typed class reads in portable owner',
 qualifier='Fresh constructor-null links only; no child/allocator/callback seam. Positive owners remain required. No native ABI/identity counter or readiness claim.')
data=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'effect-pin-audit.json'
if '--write'in sys.argv:p.write_bytes(data)
else:assert p.read_bytes()==data
print(json.dumps(dict(pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
