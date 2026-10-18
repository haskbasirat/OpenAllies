from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
doc=json.loads((HERE/'native-actor-fields.json').read_text());assert doc['producer_sha256']==H(HERE/'native_actor.py')
md=Cs(CS_ARCH_X86,CS_MODE_32);pins=[]
for address,bits in doc['pins'].items():
 a=int(address,16);raw=bytes.fromhex(bits);assert h.P.range(a,len(raw))==h.V.range(a,len(raw))==raw
 ins=list(md.disasm(raw,a));assert len(ins)==1 and ins[0].size==len(raw)
 pins.append(dict(address=address,bytes=bits,instruction=ins[0].mnemonic+' '+ins[0].op_str))
for address,target in doc['source_virtual38'].items():
 a=int(address,16);word=int(target,16).to_bytes(4,'little');assert h.P.range(a,4)==h.V.range(a,4)==word
assert len(pins)==269 and len(doc['rows'])==99
assert all(int(p['address'],16)not in(0x455050,0x6f7810,0x6f7430)for p in pins)
out=dict(schema=1,rows=99,executions=198,decoded_equal_executed_pins=pins,
 source_sha256=H(ROOT/'crates/aa-client/src/view_actor_fields.rs'),tests_sha256=H(ROOT/'crates/aa-client/src/view_actor_fields_tests.rs'),fixture_sha256=H(HERE/'native-actor-fields.json'),
 qualifier='Only453F10 tail. Base455050 and allocation/free are explicit excluded seams. Source virtual38 and pitch/list-growth children execute. Interrupted source-read instruction/entry pins also execute in successful rows. No fullbase/derivedInit/publication/GPU/view readiness.')
data=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'pin-audit.json'
if '--write'in sys.argv:p.write_bytes(data)
else:assert p.read_bytes()==data
print(json.dumps(dict(pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
