from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
md=Cs(CS_ARCH_X86,CS_MODE_32);doc=json.loads((HERE/'native-projection.json').read_text())
assert doc['producer_sha256']==H(HERE/'native_projection.py')
decoded=[]
for address,bits in doc['pins'].items():
 a=int(address,16);raw=bytes.fromhex(bits)
 assert h.P.range(a,len(raw))==h.V.range(a,len(raw))==raw
 ins=list(md.disasm(raw,a));assert len(ins)==1 and ins[0].size==len(raw)
 decoded.append(dict(address=address,bytes=bits,instruction=ins[0].mnemonic+' '+ins[0].op_str))
assert len(doc['rows'])==153 and len(decoded)==94 and not doc['seams']
out=dict(schema=1,rows=153,native_executions=306,decoded_equal_executed_pins=decoded,
 fixture_sha256=H(HERE/'native-projection.json'),source_sha256=H(ROOT/'crates/aa-runtime/src/view_environment_projection.rs'),
 tests_sha256=H(ROOT/'crates/aa-runtime/src/view_environment_projection_tests.rs'),
 claim='All pins are reached in successful rows; interrupted reads do not enlarge the successful instruction set. No native child seam. No whole environment or VWorld readiness claim.')
data=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'pin-audit.json'
if '--write'in sys.argv:p.write_bytes(data)
else:assert p.read_bytes()==data
print(json.dumps(dict(pins=len(decoded),sha256=hashlib.sha256(data).hexdigest())))
