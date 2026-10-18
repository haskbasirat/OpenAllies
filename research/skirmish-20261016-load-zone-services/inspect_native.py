"""Bounded original/v14 instruction and data observations, read-only."""
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "research/phase8-movement"))
import harness as h
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

cs = Cs(CS_ARCH_X86, CS_MODE_32)
for address, size in [(0x557e33, 0x82), (0x59e520, 16), (0x511860, 32),
                      (0x556570, 0x302), (0x552340, 0xfa), (0x54e9b0, 0x90),
                      (0x4ea150, 0x80), (0x4ebc70, 0x1a0), (0x4eb6e0, 0x150)]:
    raw = h.P.range(address, size)
    assert raw == h.V.range(address, size), hex(address)
    print(f"original/v14 identical window {address:08x}+{size:x}")
    for ins in cs.disasm(raw, address):
        print(f"{ins.address:08x}: {ins.bytes.hex()} {ins.mnemonic} {ins.op_str}")
for cls, table in [(2, 0xa2a2a0), (3, 0xa273f0), (4, 0xa299d8)]:
    for slot in [0x64, 0x68, 0x6c, 0x8c, 0x108, 0x148, 0x14c]:
        raw = h.P.range(table + slot, 4)
        assert raw == h.V.range(table + slot, 4)
        entry = int.from_bytes(raw, "little")
        print(f"runtime class{cls} vtable{table:08x}+{slot:x} ->{entry:08x}")
        for ins in cs.disasm(h.P.range(entry, 16), entry):
            print(f"{ins.address:08x}: {ins.bytes.hex()} {ins.mnemonic} {ins.op_str}")
