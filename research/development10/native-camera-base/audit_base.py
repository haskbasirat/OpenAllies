from pathlib import Path
import sys,json,hashlib,subprocess
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
md=Cs(CS_ARCH_X86,CS_MODE_32);H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
subprocess.run([sys.executable,str(HERE/'make_isolated.py')],check=True)
doc=json.loads((HERE/'native-base.json').read_text());assert doc['producer_sha256']==H(HERE/'native_base.py')
assert len(doc['rows'])==99 and len(doc['pins'])==822
excluded={0x6f7810}|{int(a,16)for a in doc['global_read_pcs']}
for address,raw in doc['pins'].items():
 a=int(address,16);assert a not in excluded;data=bytes.fromhex(raw)
 assert h.P.range(a,len(data))==data==h.V.range(a,len(data))
 instructions=list(md.disasm(data,a));assert len(instructions)==1 and instructions[0].size==len(data)
for row in doc['rows']:
 if row['output']['outcome']=='ok':assert len(row['output']['trace'])==27
out=dict(schema=1,scope='Original/v14 executed-byte equality/single-instruction decode and exact-parent isolated source transformation. Constructor numeric fields/knownmasks/typedNULLarrays only; no SDKABI/globalNiaccounting/full7016A0/directory/VWorld/AD claim.',
 fixture_sha256=H(HERE/'native-base.json'),producer_sha256=H(HERE/'native_base.py'),source_sha256=H(ROOT/'crates/aa-runtime/src/native_camera_base.rs'),tests_sha256=H(ROOT/'crates/aa-runtime/src/native_camera_base_tests.rs'),
 parent_snapshot_sha256=H(ROOT/'crates/aa-runtime/src/native_camera.rs'),isolated_sha256=H(HERE/'isolated.rs'),paired_rows=99,native_executions=198,decoded_pins=822,
 excluded_service_and_required_read_pcs=[hex(a)for a in sorted(excluded)])
data=(json.dumps(out,indent=2)+'\n').encode();path=HERE/'pin-audit.json'
if '--write'in sys.argv:path.write_bytes(data)
else:assert path.read_bytes()==data
print(hashlib.sha256(data).hexdigest())
