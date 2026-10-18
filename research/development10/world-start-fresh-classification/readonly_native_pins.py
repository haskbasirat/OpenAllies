"""Read-only PE/previous-oracle inspection; no game execution or build.

Writes only a new research artifact. Existing fixtures are read unchanged.
"""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as native
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
H=lambda b:hashlib.sha256(b).hexdigest()
cs=Cs(CS_ARCH_X86,CS_MODE_32)

def pin(address,size):
    original=native.original.range(address,size)
    modified=native.modified.range(address,size)
    assert original==modified,hex(address)
    return dict(va=hex(address),bytes=original.hex(),
        instructions=[dict(va=hex(i.address),mnemonic=i.mnemonic,operands=i.op_str)
            for i in cs.disasm(original,address)])

ctor_path=ROOT/'research/skirmish-20261017-map-actor-creation/native-constructor.json'
ctor=json.loads(ctor_path.read_bytes())
ctor_pin=next(row for row in ctor['instructions'] if row['va']=='0x4d6be8')
assert ctor_pin['bytes']=='c706f073a200'
tables=[]
for label,vt,want in [('Actor',0xa273f0,0x6a71d0),('Kingdom',0xa299d8,0x6a71d0),
    ('Group',0xa29490,0x6a71d0),('Object',0xa2a2a0,0x6a71f0),('Doodad',0xa28e90,0x6a71f0)]:
    raw=native.original.range(vt+0xb4,4)
    assert raw==native.modified.range(vt+0xb4,4)
    target=struct.unpack('<I',raw)[0]
    assert target==want
    tables.append(dict(class_name=label,vtable=hex(vt),slot='0xb4',address=hex(vt+0xb4),bytes=raw.hex(),target=hex(target)))
getter_path=ROOT/'research/skirmish-20261016-world-start/world-start-getters.json'
getter=json.loads(getter_path.read_bytes())
booleans=[row for row in getter['cases'] if row['case']['kind']=='actor_boolean']
assert len(booleans)==9
document=dict(scope=__doc__,original_sha256=H(native.original.b),v14_sha256=H(native.modified.b),
    fresh_inspection_execution_count=0,build_count=0,
    callsite=pin(0x4f1eb8,14),actor_constructor_vptr=pin(0x4d6be8,6),
    native_true=pin(0x6a71d0,3),native_false=pin(0x6a71f0,3),tables=tables,
    existing_constructor_oracle=dict(path=str(ctor_path.relative_to(ROOT)).replace('\\','/'),
        sha256=H(ctor_path.read_bytes()),rows=len(ctor['rows']),executions=2*len(ctor['rows']),
        executed_vptr_pin=ctor_pin,scope='Actual source direct4D6BB0 constructor/4E9FC0; unowned bounded laboratory, not whole map creation.'),
    existing_getter_oracle=dict(path=str(getter_path.relative_to(ROOT)).replace('\\','/'),
        sha256=H(getter_path.read_bytes()),rows=len(getter['cases']),executions=getter['executions'],
        actor_boolean_rows=booleans,scope='No seams in real mapped table target; these pure integer leaf probes use Double53, not an all-PC execution claim.'))
target=HERE/'native-pins.json'
with target.open('x')as out:json.dump(document,out,indent=2);out.write('\n')
print(json.dumps(dict(path=str(target.relative_to(ROOT)),sha256=H(target.read_bytes()),
    tables=len(tables),fresh_native_executions=0,builds=0)))
