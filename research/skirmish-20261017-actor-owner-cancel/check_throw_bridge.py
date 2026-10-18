"""Read-only original/v14 code-link receipt, distinct from executed oracle pins.

Verifies direct calls5A0910->5A0300->CxxThrow and constructed ThrowInfo/vtable.
Does not execute the exception runtime, CString formatter or full SEH unwind.
"""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import original,modified
result=[]
for name,image in [('original',original),('v14',modified)]:
    def call(a):
        raw=image.range(a,5);assert raw[0]==0xe8
        return (a+5+struct.unpack('<i',raw[1:])[0])&0xffffffff
    assert call(0x5a0983)==0x5a0300
    assert call(0x5a0375)==0x96bef5
    assert image.range(0x5a0363,5)==b'\x68'+struct.pack('<I',0xaee03c)
    assert image.range(0x5a036d,8)==b'\xc7\x44\x24\x10'+struct.pack('<I',0xa43da4)
    result.append(dict(image=name,image_sha256=hashlib.sha256(image.b).hexdigest(),
        pins=[dict(address=hex(a),bytes=image.range(a,n).hex()) for a,n in [(0x5a0910,0x78),(0x5a0300,0x7a)]]))
path=HERE/'throw-bridge.json'
if '--create' in sys.argv:
    with path.open('x',encoding='utf-8') as f:json.dump(dict(scope=__doc__,images=result),f,indent=2);f.write('\n')
else:assert json.loads(path.read_text(encoding='utf-8'))==dict(scope=__doc__,images=result)
print(json.dumps(dict(mode='created' if '--create' in sys.argv else 'read_only_replay',sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
