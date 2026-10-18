from pathlib import Path
import sys, json, hashlib
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0,str(ROOT / "research/phase8-movement"))
import harness as h
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
cs=Cs(CS_ARCH_X86,CS_MODE_32)
ranges=[(0x43f0a0,0x85),(0x5ae0a0,0x60),(0x5aca50,0xa0),(0x5abff0,0x140),(0x5b0400,0x160),(0x4f28fe,0xb),(0x556570,0x50),(0x4b8eac,0x18),(0x40977f,0x29),(0x9c7fe0,0x2a),(0x9d0340,0x2a),(0x9d0370,0x2a),(0x9d0680,0x2a),(0x9d06e0,0x2a)]
rows=[]
for start,size in ranges:
    raw=h.P.range(start,size)
    assert raw==h.V.range(start,size),hex(start)
    rows.append(dict(address=hex(start),size=size,sha256=hashlib.sha256(raw).hexdigest(),
        bytes=raw.hex(),instructions=[dict(va=hex(i.address),bytes=i.bytes.hex(),mnemonic=i.mnemonic,operands=i.op_str)for i in cs.disasm(raw,start)]))
out=dict(original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,ranges=rows)
path=HERE/"native-ranges.json"
if "--write"in sys.argv:path.write_text(json.dumps(out,indent=2)+"\n")
else:assert json.loads(path.read_text())==out
for r in rows:
    print(r["address"])
    for i in r["instructions"]:print(i["va"],i["mnemonic"],i["operands"])
