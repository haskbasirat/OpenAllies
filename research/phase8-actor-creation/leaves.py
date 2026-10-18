import sys, struct, subprocess
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-combat'))
import harness as h
from vt_dump import VT
def u32(va): return struct.unpack('<I', h.ORIGINAL.at(va, 4))[0]
for off in (0x4, 0x8, 0xc, 0x14, 0x20):
    s = {}
    for k, vt in VT.items():
        s.setdefault(u32(vt + off), []).append(str(k))
    print(hex(off), {hex(a): v for a, v in s.items()})
