from pathlib import Path
import sys,json,hashlib,subprocess
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-native-camera'));import native_camera_reset as R
from capstone import Cs,CS_ARCH_X86,CS_MODE_32,CS_MODE_64
md=Cs(CS_ARCH_X86,CS_MODE_32);wide=Cs(CS_ARCH_X86,CS_MODE_64)
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
subprocess.run([sys.executable,str(HERE/'make_isolated.py')],check=True)
doc=json.loads((HERE/'native-target.json').read_text());assert doc['producer_sha256']==H(HERE/'native_target.py')
assert len(doc['rows'])==123 and len(doc['pins'])==588
for address,raw in doc['pins'].items():
 a=int(address,16);data=bytes.fromhex(raw)
 assert R.original.range(a,len(data))==data==R.modified.range(a,len(data))
 decoded=list(md.disasm(data,a));assert len(decoded)==1 and decoded[0].size==len(data)
assert len(doc['hardware_transports'])==174
assert all(t['op']=='fsin'and t['control_word']in R.PRECISIONS.values()for t in doc['hardware_transports'])
shim=bytes.fromhex(doc['hardware_shims']['fsin']['bytes']);assert hashlib.sha256(shim).hexdigest()==doc['hardware_shims']['fsin']['sha256']
ops=[i.mnemonic for i in wide.disasm(shim,0)];assert ops[0]=='fxsave64'and ops[-2:]==['fxrstor64','ret']and ops.count('fsin')==1
out=dict(schema=1,scope='Read-only original/v14 decoded executed pins and exact-source isolated parent substitutions; THIS-CPU FSIN transport only. No full7016A0/directory/groundshape owner/VWorld/AD readiness.',
 fixture_sha256=H(HERE/'native-target.json'),producer_sha256=H(HERE/'native_target.py'),paired_rows=123,native_executions=246,decoded_pins=588,hardware_fsin_transports=174,
 isolated='isolated.rs retains exact parent State and numeric helper substrings; isolated_full.rs is the actual parent source with child paths relocated and only unrelated backend/render_lists modules omitted. make_isolated.py verifies both byte-for-text transforms.',
 source_sha256=H(ROOT/'crates/aa-runtime/src/native_camera_target.rs'),tests_sha256=H(ROOT/'crates/aa-runtime/src/native_camera_target_tests.rs'),parent_current_sha256=H(ROOT/'crates/aa-runtime/src/native_camera.rs'),
 fsin_shim_sha256=hashlib.sha256(shim).hexdigest())
data=(json.dumps(out,indent=2)+'\n').encode();path=HERE/'pin-audit.json'
if '--write'in sys.argv:path.write_bytes(data)
else:assert path.read_bytes()==data
print(hashlib.sha256(data).hexdigest())
