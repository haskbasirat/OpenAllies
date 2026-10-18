"""Immutable frozen SharedSet native replay plus individual pin decoding.
This reviewer launches no Windows debuggee and writes only its own evidence.
"""
from pathlib import Path
import sys,json,hashlib,importlib.util
HERE=Path(__file__).resolve().parent;BASE=HERE.parent;ROOT=BASE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as n
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
findings=BASE/'findings.json';assert H(findings)=='8b3d3a65c0dbcf6cf56bd2e922faa1692584b5295210336a1e1dd5127d57ed0e'
f=json.loads(findings.read_text());before={p:H(ROOT/p)for p in f['source_pins']}
assert before==f['source_pins']
spec=importlib.util.spec_from_file_location('frozen_shared_producer',BASE/'native_defaults.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
fixture=json.loads((BASE/'native-defaults.json').read_text());m.h.VISITED.clear();replayed=0
for row in fixture['rows']:
 for image in(m.ORIGINAL,m.V14):
  m.h.P=image;actual=m.Machine(row['precision']).run(row['input']);assert actual==row['output'];replayed+=1
m.h.P=m.ORIGINAL
md=Cs(CS_ARCH_X86,CS_MODE_32);decoded=[]
seams={0x6f7810,0x6f7430,0x8c3f20}
actualpins={hex(a):raw for a,raw in sorted(m.h.VISITED.items())if a not in seams}
assert actualpins==fixture['pins']
for address,bits in actualpins.items():
 a=int(address,16);raw=bytes.fromhex(bits);assert n.P.range(a,len(raw))==n.V.range(a,len(raw))==raw
 instructions=list(md.disasm(raw,a));assert len(instructions)==1 and instructions[0].size==len(raw)
 decoded.append(dict(address=address,bytes=bits,instruction=instructions[0].mnemonic+' '+instructions[0].op_str))
assert len(decoded)==628 and replayed==48
assert {p:H(ROOT/p)for p in before}==before
out=dict(schema=1,accepted_native_prefix=True,executions=replayed,paired_rows=24,executed_decoded_equal_pins=decoded,
 excluded_entries=['6F7810 allocation transport','6F7430 free transport','8C3F20 observed cutoff BEFORE later881600 initialization'],
 freeze_findings_sha256=H(findings),source_pins=before,
 qualification='Rust ownership intentionally replaces heap/refcount ABI. Fog seed is observed only creation through cutoff. No later-global immutability, wholeNi/Camera/view/AD readiness claim.')
data=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'shared-native-replay.json'
if '--write'in sys.argv:p.write_bytes(data)
else:assert p.read_bytes()==data
print(json.dumps(dict(executions=replayed,pins=len(decoded),sha256=hashlib.sha256(data).hexdigest())))
