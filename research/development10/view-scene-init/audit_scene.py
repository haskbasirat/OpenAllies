"""Read-only instruction, RTTI and isolated-kernel source audit."""
from pathlib import Path
import json,sys,struct,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'))
import native_machine as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
md=Cs(CS_ARCH_X86,CS_MODE_32)
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
fixture=json.loads((HERE/'native-scene.json').read_text())
for address,raw in fixture['pins'].items():
 a=int(address,16);data=bytes.fromhex(raw);assert h.P.range(a,len(data))==data==h.V.range(a,len(data))
 ins=list(md.disasm(data,a));assert len(ins)==1 and ins[0].size==len(data),(address,raw)
identities=[]
for name,vt,getter,rtti,crt,literal in[
 ('NiNode',0xab6950,0x8676e0,0xb43cd8,0x9e7540,0xab69e8),
 ('NiWireframeProperty',0x9fdc0c,0x43b640,0xb43e00,0x9e78f0,0xab7464),
 ('NiTexturingProperty',0xab72a4,0x86c990,0xb43df4,0x9e78c0,0xab7440),
 ('NiDirectionalLight',0xab99c8,0x888ed0,0xb442dc,0x9e7d40,0xab9a38),
 ('NiTextureEffect',0xab70a0,0x86ba00,0xb43dec,0x9e78a0,0xab719c)]:
 assert int.from_bytes(h.P.range(vt+4,4),'little')==getter
 expected=b'\xb8'+struct.pack('<I',rtti)+b'\xc3';assert h.P.range(getter,6)==expected==h.V.range(getter,6)
 row=dict(name=name,vtable=hex(vt),getter=hex(getter),rtti=hex(rtti),getter_bytes=expected.hex())
 if literal is not None:
  raw=name.encode()+b'\0';assert h.P.range(literal,len(raw))==raw==h.V.range(literal,len(raw))
  window=h.P.range(crt,21);assert window==h.V.range(crt,21)
  assert b'\x68'+struct.pack('<I',literal)in window and b'\xb9'+struct.pack('<I',rtti)in window
  row.update(crt=hex(crt),crt_bytes=window.hex(),literal=hex(literal))
 identities.append(row)
source=(ROOT/'crates/aa-runtime/src/world_revision.rs').read_bytes()
isolated=(HERE/'world_revision_kernel.rs').read_bytes()
assert source.split(b'#[cfg(test)]')[0]==isolated
assert int.from_bytes(h.P.range(0xa886a8,4),'little')==0x38d1b717
assert fixture['producer_sha256']==H(HERE/'native_scene.py')
assert len(fixture['rows'])==51 and len(fixture['numeric'])==72 and len(fixture['constructor_fields'])==18
doc=dict(schema=1,fixture_sha256=H(HERE/'native-scene.json'),producer_sha256=H(HERE/'native_scene.py'),
 executed_instruction_pins=len(fixture['pins']),scope='Original/v14 instruction equality and single-instruction decode; observed cutoff/service entry PCs excluded. No native effect of seam bodies is claimed.',
 paired_native_executions=282,identities=identities,
 isolated_world_revision='Exact production prefix only; unrelated cfg(test) module excluded. Globals test trait has the exact production method signature; all scene code is the actual production file.',
 production_source_sha256=H(ROOT/'crates/aa-runtime/src/view_scene_init.rs'),production_tests_sha256=H(ROOT/'crates/aa-runtime/src/view_scene_init_tests.rs'))
data=(json.dumps(doc,indent=2)+'\n').encode();out=HERE/'pin-audit.json'
if '--write'in sys.argv:out.write_bytes(data)
else:assert out.read_bytes()==data
print(json.dumps(dict(pins=doc['executed_instruction_pins'],identities=len(identities),sha256=hashlib.sha256(data).hexdigest())))
