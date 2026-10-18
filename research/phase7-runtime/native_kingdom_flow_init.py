"""Original 4E5EAB..4E5FB5 Kingdom flow-array reset, including real4849F0 growth.
Prior Actor Init, logging, allocator success and free are fixture seams. This is
not the surrounding Kingdom Init, recruitment, SAI or world publication.
"""
from pathlib import Path
import sys, json
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'research/phase5-actors'))
from original_creation import Machine as Base, P, V, H
from unicorn.x86_const import *

class Machine(Base):
    def hook(self, u, pc, size, user):
        if pc == 0x6f7430:
            self.ret()
        else:
            super().hook(u, pc, size, user)

rows = []
visited = {}
for count in [-1, 0, 1, 7, 8, 9, 17, 33]:
    for length, capacity in [(0, 0), (0, 8), (3, 8), (8, 8), (20, 32)]:
        m = Machine(0x27f)
        m.w(0xb36530, 0x2250000)
        m.w(0x2250000 + 0x38, count & 0xffffffff)
        before = []
        for j, off in enumerate([0x1a0, 0x1ac, 0x1b8]):
            data = m.alloc(capacity * 4) if capacity else 0
            words = [0x3f800000 + (j + 1) * 100 + i for i in range(length)]
            m.w(m.obj + off, data)
            m.w(m.obj + off + 4, length)
            m.w(m.obj + off + 8, capacity)
            for i, value in enumerate(words): m.w(data + i * 4, value)
            before.append({'values': words, 'capacity': capacity})
        m.w(m.stack + 8, m.obj)
        m.u.reg_write(UC_X86_REG_ESP, m.stack)
        m.u.reg_write(UC_X86_REG_EDI, m.obj)
        m.u.reg_write(UC_X86_REG_ESI, 0)
        m.u.emu_start(0x4e5eab, 0x4e5fb5, count=100000)
        assert m.u.reg_read(UC_X86_REG_EIP) == 0x4e5fb5
        assert m.u.reg_read(UC_X86_REG_ESP) == m.stack
        after = []
        for off in [0x1a0, 0x1ac, 0x1b8]:
            data, n, cap = [m.r(m.obj + off + i * 4) for i in range(3)]
            after.append({'values': [m.r(data + i * 4) for i in range(n)], 'capacity': cap})
        rows.append({'resource_count': count, 'before': before, 'after': after})
        visited.update(m.visited)
pins = []
for address, size in sorted(visited.items()):
    if not P.base <= address < P.base + P.image_size: continue
    code = P.range(address, size)
    assert code == V.range(address, size)
    pins.append({'va': hex(address), 'bytes': code.hex()})
out = {'scope': __doc__, 'original_sha256': H(P.b), 'v14_sha256': H(V.b),
       'cases': rows, 'instructions': pins}
path = Path(__file__).with_name('native-kingdom-flow-init.json')
path.write_text(json.dumps(out, indent=2) + '\n', encoding='utf-8')
print({'cases': len(rows), 'sha256': H(path.read_bytes())})
