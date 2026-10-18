import sys, struct
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-combat'))
import harness as h
def u32(va): return struct.unpack('<I', h.ORIGINAL.at(va, 4))[0]
VT = {0: 0xa3b0e8, 1: 0xa3e280, 2: 0xa3b4d8, 3: 0xa3ce88, '4org': 0xa335a0, '4air': 0xa32710, '4elem': 0xa35810,
      5: 0xa3df40, 6: 0xa3e3dc, 7: 0xa3d434, 8: 0xa3c6f0, 9: 0xa3c690, 10: 0xa3c14c, 11: 0xa400c0, 12: 0xa40198,
      13: 0xa3fac8, 14: 0xa3eea0, 15: 0xa3b950, 16: 0xa3e6d0, 17: 0xa3c974, 18: 0xa3b2a8, 19: 0xa3cd00,
      20: 0xa3bbd4, 21: 0xa3eb58, 22: 0xa3c568, 23: 0xa3af78, 24: 0xa40100, 25: 0xa3cc6c, 26: 0xa3c9e4,
      27: 0xac7ed0, 28: 0xa3e144, 29: 0xa3b6f8, 30: 0xa3ca4c}
n = int(sys.argv[1], 16) if len(sys.argv) > 1 else 0x38
print('slot  ' + ' '.join(f'+{o:02x}    ' for o in range(0, n, 4)))
for k, vt in VT.items():
    print(f'{str(k):5s} ' + ' '.join(f'{u32(vt + o):06x}' for o in range(0, n, 4)))
