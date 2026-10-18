"""Independent original/v14 instruction audit for the borrowed mean producer.

This reads the two pinned PEs. It does not launch the game or certify the
source lifecycle, profile, active application FP context, or ActorData CVs.
"""
from pathlib import Path
import json
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "research/phase8-economy"))
from harness import ORIGINAL, ACCEPTED, sha
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

decoder = Cs(CS_ARCH_X86, CS_MODE_32)
ranges = []

def check(start, end, expected):
    raw = ORIGINAL.at(start, end - start)
    assert raw == ACCEPTED.at(start, end - start)
    decoded = {i.address: (i.mnemonic, i.op_str) for i in decoder.disasm(raw, start)}
    for address, instruction in expected.items():
        assert decoded[address] == instruction, (hex(address), decoded[address], instruction)
    ranges.append({"start": start, "end": end, "sha256": sha(raw),
                   "checked_instructions": {hex(k): list(v) for k, v in expected.items()}})

check(0x4bbf90, 0x4bc5d4, {
    0x4bbfb3: ("mov", "eax, dword ptr [esi + 0x68]"),
    0x4bbfb6: ("add", "eax, 8"),
    0x4bbfc4: ("mov", "eax, dword ptr [eax + 8]"),
    0x4bbfdc: ("call", "0x6f7810"),
    0x4bc05d: ("mov", "dword ptr [esi + 0x74], ebx"),
    0x4bc060: ("mov", "dword ptr [esi + 0x78], ebx"),
    0x4bc063: ("mov", "dword ptr [esi + 0x7c], ebx"),
    0x4bc066: ("mov", "dword ptr [esi + 0x80], ebx"),
    0x4bc06c: ("mov", "dword ptr [esi + 0x84], ebx"),
    0x4bc072: ("mov", "dword ptr [esi + 0x88], ebx"),
    0x4bc096: ("mov", "eax, dword ptr [ebp + 0x174]"),
    0x4bc0a5: ("call", "0x45e6d0"),
    0x4bc0aa: ("fcomp", "dword ptr [esi + 0x74]"),
    0x4bc0b6: ("call", "0x45e6d0"),
    0x4bc0bb: ("fstp", "dword ptr [esi + 0x74]"),
    0x4bc0be: ("mov", "eax, dword ptr [ebp + 0x174]"),
    0x4bc100: ("mov", "eax, dword ptr [ebp + 0x1f0]"),
    0x4bc120: ("mov", "eax, dword ptr [ebp + 0x1ec]"),
    0x4bc129: ("mov", "ecx, dword ptr [edi]"),
    0x4bc12b: ("fld", "dword ptr [ecx + 0x64]"),
    0x4bc140: ("call", "0x4be3f0"),
    0x4bc180: ("mov", "edx, dword ptr [edi + 4]"),
    0x4bc189: ("call", "0x4be680"),
    0x4bc18e: ("fadd", "dword ptr [esp + 0x24]"),
    0x4bc199: ("call", "0x4be4b0"),
    0x4bc1ac: ("call", "0x4be4b0"),
    0x4bc2ce: ("mov", "edi, dword ptr [edi + 8]"),
    0x4bc2e8: ("fld", "dword ptr [ecx]"),
    0x4bc2f0: ("call", "0x96cc4c"),
    0x4bc317: ("fadd", "dword ptr [esi + 0x78]"),
    0x4bc31a: ("fstp", "dword ptr [esi + 0x78]"),
    0x4bc332: ("fdivr", "dword ptr [0x9f32e0]"),
    0x4bc34c: ("call", "0x96cc4c"),
    0x4bc3b2: ("mov", "al, byte ptr [0xb3b4f8]"),
    0x4bc3cb: ("fstp", "dword ptr [esi + 0x80]"),
    0x4bc41f: ("call", "0x855d30"),
    0x4bc427: ("mov", "byte ptr [0xb3b4f8], al"),
    0x4bc432: ("fsub", "dword ptr [esi + 0x80]"),
    0x4bc43e: ("fdiv", "st(1)"),
    0x4bc440: ("fstp", "dword ptr [esi + 0x8c]"),
    0x4bc446: ("fstp", "dword ptr [esi + 0x90]"),
    0x4bc46b: ("call", "0x404f60"),
    0x4bc53f: ("fld", "dword ptr [esi + 0x8c]"),
    0x4bc564: ("call", "0x404f60"),
})
check(0x4bd800, 0x4bd845, {
    0x4bd804: ("call", "0x4bb480"),
    0x4bd80b: ("call", "0x4bb130"),
    0x4bd812: ("call", "0x4bb730"),
    0x4bd819: ("call", "0x4bbf90"),
    0x4bd820: ("call", "0x4bb590"),
})
check(0x4bb590, 0x4bb719, {
    0x4bb5d9: ("call", "0x4b2e60"),
    0x4bb6e2: ("cmp", "dword ptr [ecx + 0x1e4], 0xbf800000"),
    0x4bb6ee: ("call", "0x4618b0"),
})
doc = {"schema": 1, "original_sha256": sha(ORIGINAL.data), "v14_sha256": sha(ACCEPTED.data),
       "audit_script_sha256": sha(Path(__file__).read_bytes()), "ranges": ranges,
       "qualification": "Static independent instruction audit. Separate dynamic replay is required; 4BB590 metadata does not claim its runtime/source binding is implemented."}
path = HERE / "independent-instruction-audit.json"
path.write_text(json.dumps(doc, indent=2) + "\n")
print(json.dumps({"ranges": len(ranges), "checked_instructions": sum(len(r["checked_instructions"]) for r in ranges), "sha256": sha(path.read_bytes())}))
