"""Independent original/v14 instruction audit for the borrowed mean producer.

This reads the two pinned PEs. It does not launch the game or certify the
source lifecycle, profile, active application FP context, or ActorData CVs.
"""
from pathlib import Path
import hashlib
import json
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "research/phase8-economy"))
from harness import ORIGINAL, ACCEPTED, sha
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

decoder = Cs(CS_ARCH_X86, CS_MODE_32)
ranges = []

def digest(instruction):
    """Short digest of (mnemonic, operands): the audit compares it and holds no instruction text."""
    return hashlib.sha256("|".join(instruction).encode()).hexdigest()[:16]

def check(start, end, expected):
    raw = ORIGINAL.at(start, end - start)
    assert raw == ACCEPTED.at(start, end - start)
    decoded = {i.address: (i.mnemonic, i.op_str) for i in decoder.disasm(raw, start)}
    for address, instruction in expected.items():
        assert digest(decoded[address]) == instruction, hex(address)
    ranges.append({"start": start, "end": end, "sha256": sha(raw),
                   "checked_instructions": {hex(k): v for k, v in expected.items()}})

check(0x4bbf90, 0x4bc5d4, {
    0x4bbfb3: "6ca539faed649a0a",
    0x4bbfb6: "1b71715af244e9da",
    0x4bbfc4: "90f6e94c5432af09",
    0x4bbfdc: "8e892065435ed945",
    0x4bc05d: "af9aa95e266954a9",
    0x4bc060: "4d2f2df9decb379b",
    0x4bc063: "5b112ace1088f581",
    0x4bc066: "293e2a500ed16b93",
    0x4bc06c: "016bc5df20ff1041",
    0x4bc072: "b86201447be814f6",
    0x4bc096: "b12a9068e73c1990",
    0x4bc0a5: "6f3c3d7f1da9309f",
    0x4bc0aa: "35ad6305ce6ac799",
    0x4bc0b6: "6f3c3d7f1da9309f",
    0x4bc0bb: "b4ffa451bc95c165",
    0x4bc0be: "b12a9068e73c1990",
    0x4bc100: "c60b91a0d104cf6b",
    0x4bc120: "2c64d6fe3a50aaf9",
    0x4bc129: "a600d4e08f9163f7",
    0x4bc12b: "21bfba728f157145",
    0x4bc140: "09ff0c5cc3477092",
    0x4bc180: "900b6482c9b5114e",
    0x4bc189: "ec9532b5fafc0782",
    0x4bc18e: "88beaf79b8986dc1",
    0x4bc199: "f7c0d953b896db2d",
    0x4bc1ac: "f7c0d953b896db2d",
    0x4bc2ce: "af9f39e62be6079d",
    0x4bc2e8: "68d5fd4e2c213128",
    0x4bc2f0: "89581821e28b44fc",
    0x4bc317: "6756f33c54a2d64a",
    0x4bc31a: "861649f929d83924",
    0x4bc332: "e422e137420ebe95",
    0x4bc34c: "89581821e28b44fc",
    0x4bc3b2: "83fb6a4b8b846b94",
    0x4bc3cb: "e901365cf33ca5e7",
    0x4bc41f: "6b8520f24dc3afb3",
    0x4bc427: "7e4a26b418a8c1bf",
    0x4bc432: "e127482b88fac9b6",
    0x4bc43e: "845caa565335f827",
    0x4bc440: "5f396fa9117da0e0",
    0x4bc446: "e4d244c87666dded",
    0x4bc46b: "2c325942fac6f9e6",
    0x4bc53f: "1dc464bb22290abf",
    0x4bc564: "2c325942fac6f9e6",
})
check(0x4bd800, 0x4bd845, {
    0x4bd804: "4818e91b2b7279e5",
    0x4bd80b: "920d7ff11adb5029",
    0x4bd812: "63f9d699cae2f26f",
    0x4bd819: "5f1244fa62f556ae",
    0x4bd820: "98f6dce5ecdac8be",
})
check(0x4bb590, 0x4bb719, {
    0x4bb5d9: "03a6c4f7a70fef9b",
    0x4bb6e2: "64fd87c9e2227169",
    0x4bb6ee: "fbe3898a7c49b779",
})
doc = {"schema": 1, "original_sha256": sha(ORIGINAL.data), "v14_sha256": sha(ACCEPTED.data),
       "audit_script_sha256": sha(Path(__file__).read_bytes()), "ranges": ranges,
       "qualification": "Static independent instruction audit. Separate dynamic replay is required; 4BB590 metadata does not claim its runtime/source binding is implemented."}
path = HERE / "independent-instruction-audit.json"
path.write_text(json.dumps(doc, indent=2) + "\n")
print(json.dumps({"ranges": len(ranges), "checked_instructions": sum(len(r["checked_instructions"]) for r in ranges), "sha256": sha(path.read_bytes())}))
