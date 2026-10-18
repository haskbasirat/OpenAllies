"""Native oracle for 0x57A640 (organization +D8 = minimum member +6C +28,
iterated by 0x4364C0) and its owner +70 virtual +118 notification (seam:
recorded arguments). All precisions.
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402


def f32(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def main():
    rng = random.Random(0x57A640)
    cases, visited = [], {}
    for precision in h.PRECISIONS:
        m = h.Machine(precision)
        for i in range(150):
            count = rng.randint(0, 6)
            slots = []
            for _ in range(count):
                kind = rng.random()
                if kind < 0.2:
                    slots.append('empty')
                elif kind < 0.3:
                    slots.append(None)
                else:
                    slots.append(rng.choice((f32(rng.uniform(0, 20)), f32(5.0), f32(rng.uniform(0, 1e-3)))))
            d8 = rng.choice((0, f32(5.0), f32(rng.uniform(0, 20)), f32(rng.uniform(0, 1e-3))))
            if i % 10 == 0 and any(isinstance(s, int) for s in slots):
                d8 = min(s for s in slots if isinstance(s, int))
            mover = rng.random() < 0.8
            calls = []
            owner = m.alloc(0x100)
            if mover:
                comp = m.alloc(0x40)

                def notify(mm, calls=calls):
                    calls.append([mm.arg(0), mm.arg(1)])
                    mm.ret(8)
                m.w(comp, m.vtable({0x118: notify}))
                m.w(owner + 0x70, comp)
            org = m.alloc(0x100)
            members = m.alloc(4 * max(1, count))
            m.w(org + 4, owner)
            m.w(org + 0x28, members)
            m.w(org + 0x2C, count)
            m.w(org + 0xD8, d8)
            for k, s in enumerate(slots):
                if s == 'empty':
                    m.w(members + 4 * k, 0)
                    continue
                actor = m.alloc(0x100)
                m.w(actor + 0x6C, 0)
                if s is not None:
                    c = m.alloc(0x40)
                    m.w(c + 0x28, s)
                    m.w(actor + 0x6C, c)
                m.w(members + 4 * k, actor)
            m.call(0x57A640, ecx=org)
            cases.append({'precision': precision, 'slots': slots, 'd8': d8, 'mover': mover,
                          'd8_after': m.r(org + 0xD8), 'calls': calls})
        visited.update(m.visited)
    h.report('native-org-speed-fixtures.json',
             '0x57A640 organization speed minimum (+D8) with 0x4364C0 iteration; owner +70 virtual '
             '+118 recorded; all precisions. Slot entries: "empty" (null member), null (member '
             'without +6C) or the +6C +28 binary32 bits.', cases, visited)


if __name__ == '__main__':
    main()
