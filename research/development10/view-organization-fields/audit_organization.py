from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
doc=json.loads((HERE/'native-organization-fields.json').read_text());assert doc['producer_sha256']==H(HERE/'native_organization.py')
md=Cs(CS_ARCH_X86,CS_MODE_32);pins=[]
for address,bits in doc['pins'].items():
 a=int(address,16);raw=bytes.fromhex(bits);assert h.P.range(a,len(raw))==h.V.range(a,len(raw))==raw
 ins=list(md.disasm(raw,a));assert len(ins)==1 and ins[0].size==len(raw)
 pins.append(dict(address=address,bytes=bits,instruction=ins[0].mnemonic+' '+ins[0].op_str))
assert len(pins)==37 and len(doc['rows'])==24
assert all(int(p['address'],16)not in(0x453f10,0x6f7810)for p in pins)
for row in doc['rows']:
 out=row['output'];assert out['events']==[['base453F10'],['allocate',56]]
 assert out['known_masks']['364']==(0 if row['input']['cutoff'] else 255)
out=dict(schema=1,paired_rows=24,executions=48,decoded_equal_executed_pins=pins,
 source_sha256=H(ROOT/'crates/aa-client/src/view_organization_fields.rs'),tests_sha256=H(ROOT/'crates/aa-client/src/view_organization_fields_tests.rs'),fixture_sha256=H(HERE/'native-organization-fields.json'),
 qualifier='Only44FDB0 tail after separately owned453F10 base. Poisoned allocator slots remain unwritten;16C is byte-only. Allocator-entry cutoff is a prefix observation, not native failure/unwind. No fullInit/publication/GPU/readiness claim.')
data=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'pin-audit.json'
if '--write'in sys.argv:p.write_bytes(data)
else:assert p.read_bytes()==data
print(json.dumps(dict(pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
