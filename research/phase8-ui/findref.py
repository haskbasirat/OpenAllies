"""Find 32-bit little-endian references to given VAs in AA.exe (data and code)."""
import sys, struct
from aadis import P
for a in sys.argv[1:]:
    v = int(a, 16)
    pat = struct.pack('<I', v)
    i = 0
    hits = []
    while True:
        i = P.b.find(pat, i)
        if i < 0: break
        # convert file offset to va
        for vs, r, size, off in P.sections:
            if off <= i < off + size:
                hits.append(hex(P.base + r + i - off))
        i += 1
    print(a, hits)
