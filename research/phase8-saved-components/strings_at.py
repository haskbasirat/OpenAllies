"""Print UTF-16LE (or ASCII) literals at the given VAs of the original image."""
import sys
from disasm import P
for a in sys.argv[1:]:
    va = int(a, 16)
    raw = P.range(va, 512)
    if raw[1] == 0:
        end = next(i for i in range(0, len(raw), 2) if raw[i:i+2] == b'\0\0')
        print(hex(va), repr(raw[:end].decode('utf-16-le')))
    else:
        print(hex(va), repr(raw.split(b'\0')[0].decode('latin1')))
