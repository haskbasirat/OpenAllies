"""Native oracle for the overlap escape point 4D1820(&point) and its search
4D04D0(other, &point), with the real 4CF210 snapshot and 409230 normalize,
all precisions.

The actor's +110 overlap list (nodes [GTID, 0, next at +8], tail +114) holds
live and stale GTIDs against the [B364C0] registry; 6F7430 (free) is
recorded. The spatial query 554350 / 552250 / 553BA0 is a recorded seam
answering from a per-case occupancy script. The x87 FCOS/FSIN leaf of the
emulator is tabulated per precision for k = 1..7 (Unicorn computes these
transcendentals through binary64, so the table is the leaf both sides use;
the angle itself is checked by the test).

Usage: native_overlap_escape.py  (AA.exe is located by the shared harness)
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESP  # noqa: E402


def f32(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def ext_hex(raw):
    mantissa, top = struct.unpack('<QH', raw)
    return {'mantissa': f'{mantissa:016x}', 'sign_exponent': top}


def transcendentals(m, precision):
    """fild k; fmul [A019F0]; fld st0; fstp tbyte A; fld st0; fcos; fstp tbyte C; fsin; fstp tbyte S."""
    out = []
    base = h.Machine.STUBS + 0x800
    k_at, a_at, c_at, s_at = base + 0x100, base + 0x110, base + 0x120, base + 0x130
    code = (b'\xdb\x05' + struct.pack('<I', k_at) + b'\xd8\x0d' + struct.pack('<I', 0xA019F0)
            + b'\xd9\xc0' + b'\xdb\x3d' + struct.pack('<I', a_at)
            + b'\xd9\xc0\xd9\xff' + b'\xdb\x3d' + struct.pack('<I', c_at)
            + b'\xd9\xfe' + b'\xdb\x3d' + struct.pack('<I', s_at) + b'\xc3')
    m.u.mem_write(base, code)
    for k in range(1, 8):
        m.w(k_at, k)
        sp = h.Machine.STACK - 0x80
        m.w(sp, h.Machine.STOP)
        m.u.reg_write(UC_X86_REG_ESP, sp)
        m.u.emu_start(base, h.Machine.STOP, count=20)
        out.append({'precision': precision, 'k': k, 'angle': ext_hex(m.u.mem_read(a_at, 10)),
                    'cos': ext_hex(m.u.mem_read(c_at, 10)), 'sin': ext_hex(m.u.mem_read(s_at, 10))})
    return out


def coordinate(rng):
    return f32(rng.choice((0.0, 10.0, rng.uniform(-50, 50), rng.uniform(100, 3000))))


def near(rng, x):
    roll = rng.random()
    v = struct.unpack('<f', struct.pack('<I', x))[0]
    if roll < 0.12:
        return x
    if roll < 0.2:
        return f32(v + rng.choice((1e-5, -3e-5, 2e-6)))
    return f32(v + rng.uniform(-4, 4))


def main():
    rng = random.Random(0x4D1820)
    table, cases, visited = [], [], {}
    for precision in h.PRECISIONS:
        m = h.Machine(precision)
        table += transcendentals(m, precision)
        for _ in range(160):
            events = []
            m.u.mem_write(m.HEAP, bytes(m.cursor - m.HEAP))
            m.cursor = m.HEAP
            m.next_virtual = m.VIRTUALS
            m.hooks = {}
            registry = m.alloc(0x20004 + 0x100 * 4)
            m.w(h.REGISTRY_ROOT, registry)
            world = m.alloc(0x100)
            m.w(h.WORLD_ROOT, world)
            spatial = m.alloc(0x40)
            m.w(world + 0x30, spatial)
            n = rng.randint(1, 4)
            things = []
            for i in range(n):
                t = m.alloc(0x100)
                shape = m.alloc(0x10)
                radius = f32(rng.choice((0.5, 1.0, 2.5, rng.uniform(0.1, 6))))
                m.w(shape + 4, radius)
                m.w(t, m.vtable({0xC: lambda mm, s=shape: mm.ret(0, s)}))
                pos = [coordinate(rng), coordinate(rng)]
                m.w(t + 0x20, pos[0])
                m.w(t + 0x24, pos[1])
                m.w(t + 0x2C, rng.getrandbits(32))
                m.w(t + 0x30, rng.getrandbits(32))
                generation = rng.getrandbits(16)
                live = rng.random() < 0.7
                m.u.mem_write(registry + 4 + (i + 1) * 2, generation.to_bytes(2, 'little'))
                m.w(registry + 0x20004 + (i + 1) * 4, t if live else 0)
                things.append({'radius': radius, 'position': pos, 'generation': generation, 'live': live})
            actor = m.alloc(0x200)
            own_shape = m.alloc(0x10)
            own_radius = f32(rng.choice((0.5, 1.0, rng.uniform(0.1, 4))))
            m.w(own_shape + 4, own_radius)
            m.w(actor, m.vtable({0x10: lambda mm, s=own_shape: mm.ret(0, s)}))
            anchor = things[rng.randrange(n)]['position']
            own = [near(rng, anchor[0]), near(rng, anchor[1])]
            m.w(actor + 0x20, own[0])
            m.w(actor + 0x24, own[1])
            overlaps = []
            for _ in range(rng.randint(0, 4)):
                i = rng.randrange(n)
                g = things[i]['generation'] if rng.random() < 0.75 else (things[i]['generation'] + 1) & 0xFFFF
                overlaps.append((g << 16) | (i + 1))
            head, last = 0, 0
            for gtid in reversed(overlaps):
                node = m.alloc(0xC)
                m.w(node, gtid)
                m.w(node + 8, head)
                if not last:
                    last = node
                head = node
            m.w(actor + 0x110, head)
            m.w(actor + 0x114, last)
            occupied = [rng.random() < 0.55 for _ in range(8)]
            if rng.random() < 0.15:
                occupied = [True] * 8
            query = {}

            def start(mm, ev=events, a=actor):
                assert (mm.arg(0), mm.arg(1)) == (a, 0)
                query['this'] = mm.reg(UC_X86_REG_ECX)
                mm.ret(8)

            def place(mm, ev=events):
                assert mm.reg(UC_X86_REG_ECX) == query['this']
                ev.append(['query', mm.arg(0), mm.arg(1)])
                mm.ret(8)

            def first(mm, ev=events, occ=occupied, s=spatial):
                assert mm.reg(UC_X86_REG_ECX) == s and mm.arg(1) == query['this']
                tries = sum(1 for e in ev if e[0] == 'query')
                out = mm.arg(0)
                mm.w(out, 0x77 if occ[tries - 1] else 0)
                mm.ret(8, out)

            def free(mm, ev=events):
                ev.append(['free'])
                mm.ret(0)
            m.hooks[0x554350] = start
            m.hooks[0x552250] = place
            m.hooks[0x553BA0] = first
            m.hooks[0x6F7430] = free
            point = m.alloc(8)
            m.w(point, 0xDEADBEEF)
            m.w(point + 4, 0xDEADBEEF)
            result = m.call(0x4D1820, ecx=actor, args=(point,), pop=4) & 0xFF
            remaining, node = [], m.r(actor + 0x110)
            while node:
                remaining.append(m.r(node))
                node = m.r(node + 8)
            tail_ok = m.r(actor + 0x114) == (last if remaining else 0)
            cases.append({'precision': precision, 'things': things, 'own_radius': own_radius, 'position': own,
                          'overlaps': overlaps, 'occupied': occupied, 'result': result,
                          'point': [m.r(point), m.r(point + 4)], 'remaining': remaining,
                          'tail_ok': tail_ok,
                          'events': events})
        visited.update(m.visited)
    h.report('native-overlap-escape-fixtures.json',
             '4D1820 overlap list walk, 4CF210 snapshot, 4D04D0 escape search (409230), all precisions.',
             cases, visited, extra={'transcendentals': table})


if __name__ == '__main__':
    main()
