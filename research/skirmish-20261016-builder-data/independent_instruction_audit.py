"""Independent byte/instruction checks for the bounded Builder source review.

No game files are written and no original process is launched. These checks
are separate from the author's fixture producer. Full constructor/LoadData
execution and unrelated deferred callbacks are not certified by this audit.
"""
from pathlib import Path
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
def exact(start, end, expected):
    actual = instructions(start, end)
    for address, instruction in expected.items():
        assert actual[address] == instruction, (hex(address), actual[address], instruction)
    checks.append({"start": start, "end": end, "sha256": sha(ORIGINAL.at(start, end-start)),
                   "checked_instructions": {hex(k): list(v) for k, v in expected.items()}})

exact(0x4604c0, 0x460542, {
    0x4604d7: ("call", "0x4bdba0"),
    0x4604f6: ("test", "byte ptr [edi + 0x174], 1"),
    0x460513: ("mov", "ebx, dword ptr [esi + 0x504]"),
    0x46051b: ("call", "0x6f7810"),
    0x460520: ("mov", "dword ptr [eax], edi"),
    0x460522: ("mov", "dword ptr [eax + 4], ebx"),
    0x460525: ("mov", "ecx, dword ptr [esi + 0x508]"),
    0x460530: ("mov", "dword ptr [esi + 0x504], eax"),
    0x460536: ("jne", "0x46053e"),
    0x460538: ("mov", "dword ptr [esi + 0x508], eax"),
})
exact(0x461820, 0x46182d, {
    0x461820: ("lea", "eax, [ecx + 4]"),
    0x461823: ("mov", "ecx, dword ptr [ecx + 0xc]"),
    0x461827: ("call", "0x4604c0"),
})
exact(0x469980, 0x4699db, {
    0x4699a9: ("call", "0x4624b0"),
    0x4699bc: ("mov", "dword ptr [esi], 0xa09b30"),
})
exact(0x468d20, 0x468d4d, {
    0x468d28: ("call", "0x464bd0"),
    0x468d41: ("call", "0x410c40"),
})
exact(0x4bdba0, 0x4bdbd9, {
    0x4bdba6: ("call", "0x427bd0"),
    0x4bdbb5: ("call", "dword ptr [edx + 0x38]"),
    0x4bdbc8: ("cmp", "eax, 0x2b"),
})
exact(0x4624b0, 0x462504, {0x4624e1: ("xor", "ebx, ebx")})
exact(0x462cf9, 0x462d05, {
    0x462cf9: ("mov", "dword ptr [esi + 0x504], ebx"),
    0x462cff: ("mov", "dword ptr [esi + 0x508], ebx"),
})
vtable_checks = []
for vtable, predicate in ((0xa05540, 0x4f0500), (0xa09b30, 0x4f0500), (0xa13318, 0x4c9fa0)):
    a = ORIGINAL.at(vtable + 0x38, 4)
    assert a == ACCEPTED.at(vtable + 0x38, 4)
    assert struct.unpack("<I", a)[0] == predicate
    vtable_checks.append({"vtable": vtable, "predicate38": predicate, "sha256": sha(a)})
exact(0x4f0500, 0x4f0506, {0x4f0500: ("mov", "eax, 1"), 0x4f0505: ("ret", "")})
exact(0x4c9fa0, 0x4c9fa3, {0x4c9fa0: ("xor", "eax, eax"), 0x4c9fa2: ("ret", "")})

doc = {"schema": 1, "original_sha256": sha(ORIGINAL.data), "v14_sha256": sha(ACCEPTED.data),
       "audit_script_sha256": sha(Path(__file__).read_bytes()), "checks": checks,
       "vtable_checks": vtable_checks,
       "qualification": "Metadata/instruction audit; constructor and full LoadData are not dynamically reexecuted here."}
output = HERE / "independent-instruction-audit.json"
output.write_text(json.dumps(doc, indent=2) + "\n")
print(json.dumps({"instruction_ranges": len(checks), "vtable_slots": len(vtable_checks), "sha256": sha(output.read_bytes())}))
