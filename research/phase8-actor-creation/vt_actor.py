import sys, struct
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-combat'))
import harness as h
def u32(va): return struct.unpack('<I', h.ORIGINAL.at(va, 4))[0]
for name, vt in [('GActor', 0xa273f0), ('GObject', 0xa2a2a0), ('ActorData', 0xa05540), ('ObjectData', 0xa13318), ('Kingdom', 0xa299d8)]:
    print(name, hex(vt))
    for o in range(0, 0x140, 4):
        print(f'  +{o:03x} {u32(vt+o):06x}', end='\n' if o % 32 == 28 else '')
    print()
