"""Independent byte/instruction checks for the bounded Builder source review.

No game files are written and no original process is launched. These checks
are separate from the author's fixture producer. Full constructor/LoadData
execution and unrelated deferred callbacks are not certified by this audit.
"""
from pathlib import Path
import hashlib
import json
import struct
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "research/phase8-economy"))
from harness import ORIGINAL, ACCEPTED, sha
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

DECODER = Cs(CS_ARCH_X86, CS_MODE_32)

def instructions(start, end):
    a = ORIGINAL.at(start, end - start)
    b = ACCEPTED.at(start, end - start)
    assert a == b, hex(start)
    return {i.address: (i.mnemonic, i.op_str) for i in DECODER.disasm(a, start)}

checks = []
def digest(instruction):
    """Short digest of (mnemonic, operands): the audit compares it and holds no instruction text."""
    return hashlib.sha256("|".join(instruction).encode()).hexdigest()[:16]

def exact(start, end, expected):
    actual = instructions(start, end)
    for address, instruction in expected.items():
        assert digest(actual[address]) == instruction, hex(address)
    checks.append({"start": start, "end": end, "sha256": sha(ORIGINAL.at(start, end-start)),
                   "checked_instructions": {hex(k): v for k, v in expected.items()}})

exact(0x4604c0, 0x460542, {
    0x4604d7: "e8fdeecbafdc14ed",
    0x4604f6: "e24dc776c092897c",
    0x460513: "5a6cf4861a271579",
    0x46051b: "8e892065435ed945",
    0x460520: "3f01dbdd366400e5",
    0x460522: "2a65eb97ffd9ff4f",
    0x460525: "a4b0454879bc29b9",
    0x460530: "0983884499a3b184",
    0x460536: "6c40fbbdadb0a717",
    0x460538: "75933ac0944947cd",
})
exact(0x461820, 0x46182d, {
    0x461820: "6b4d3440cfdd15fe",
    0x461823: "debc12f2caa37168",
    0x461827: "030d1789131c81db",
})
exact(0x469980, 0x4699db, {
    0x4699a9: "3ef6090217f53dee",
    0x4699bc: "19813d5129918ca3",
})
exact(0x468d20, 0x468d4d, {
    0x468d28: "f29b71892494e4c1",
    0x468d41: "3d0e5be279f32604",
})
exact(0x4bdba0, 0x4bdbd9, {
    0x4bdba6: "54f2809846bd477a",
    0x4bdbb5: "4719a74cabc47a3e",
    0x4bdbc8: "d93bfd4fd8304ee5",
})
exact(0x4624b0, 0x462504, {0x4624e1: "490c1d4dd117b476"})
exact(0x462cf9, 0x462d05, {
    0x462cf9: "a19ca66ea00a109d",
    0x462cff: "04c1ce05886293d1",
})
vtable_checks = []
for vtable, predicate in ((0xa05540, 0x4f0500), (0xa09b30, 0x4f0500), (0xa13318, 0x4c9fa0)):
    a = ORIGINAL.at(vtable + 0x38, 4)
    assert a == ACCEPTED.at(vtable + 0x38, 4)
    assert struct.unpack("<I", a)[0] == predicate
    vtable_checks.append({"vtable": vtable, "predicate38": predicate, "sha256": sha(a)})
exact(0x4f0500, 0x4f0506, {0x4f0500: "a6e1276385daf859", 0x4f0505: "d77cdcb9086369e1"})
exact(0x4c9fa0, 0x4c9fa3, {0x4c9fa0: "b0f355e49718529b", 0x4c9fa2: "d77cdcb9086369e1"})

doc = {"schema": 1, "original_sha256": sha(ORIGINAL.data), "v14_sha256": sha(ACCEPTED.data),
       "audit_script_sha256": sha(Path(__file__).read_bytes()), "checks": checks,
       "vtable_checks": vtable_checks,
       "qualification": "Metadata/instruction audit; constructor and full LoadData are not dynamically reexecuted here."}
output = HERE / "independent-instruction-audit.json"
output.write_text(json.dumps(doc, indent=2) + "\n")
print(json.dumps({"instruction_ranges": len(checks), "vtable_slots": len(vtable_checks), "sha256": sha(output.read_bytes())}))
