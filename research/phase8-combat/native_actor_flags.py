"""Table-driven native oracle for the small actor field helpers 4D07D0
(with the real _allshl 96D710), 4D0960, 4CFFD0, 4CE880, 4CEE40, 4CEFA0,
4D1240, 4D1290, 45E660 and 471A40. Each case lists the words written before
the call, the watched words after it, the returned word and the recorded
virtual calls (+100, release +0(1)).

Usage: native_actor_flags.py  (AA.exe is located by the shared harness)
"""
from pathlib import Path
import random
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402


class Case:
    def __init__(self, m, name):
        self.m = m
        self.name = name
        self.memory = []
        self.watch = []
        self.events = []

    def w(self, address, value, watch=False):
        self.m.w(address, value)
        self.memory.append([address, value & 0xFFFFFFFF])
        if watch:
            self.watch.append(address)

    def done(self, result, **extra):
        out = {'fn': self.name, 'memory': self.memory, 'after': [[a, self.m.r(a)] for a in self.watch],
               'result': result, 'events': self.events}
        out.update(extra)
        return out


def main():
    rng = random.Random(0x4D07D0)
    m = h.Machine('Double53')
    cases = []

    def actor_vt(c, value):
        def v100(mm):
            c.events.append(['virtual100', mm.reg(UC_X86_REG_ECX)])
            mm.ret(0, value)
        return m.vtable({0x100: v100})

    def object_vt(c):
        def release(mm):
            assert mm.arg(0) == 1
            c.events.append(['release', mm.reg(UC_X86_REG_ECX)])
            mm.ret(4)
        return m.vtable({0x0: release})

    for _ in range(80):
        c = Case(m, '4D07D0')
        word = m.alloc(8)
        c.w(word, rng.getrandbits(32), True)
        c.w(word + 4, rng.getrandbits(32), True)
        bit = rng.choice((rng.randrange(70), 255, 256, 257, 0x140, 31, 32, 63, 64))
        flag = rng.choice((0, 1, 0x100, 0xFF))
        m.call(0x4D07D0, ecx=word, args=(bit, flag), pop=8)
        cases.append(c.done(None, this=word, bit=bit, flag=flag))
    for _ in range(20):
        c = Case(m, '4D0960')
        a = m.alloc(0x200)
        c.w(a + 0x100, rng.getrandbits(32) | (0x40000000 if rng.random() < 0.5 else 0))
        cases.append(c.done(m.call(0x4D0960, ecx=a, pop=0) & 0xFF, this=a))
    for _ in range(60):
        c = Case(m, '4CFFD0')
        a = m.alloc(0x200)
        c.w(a + 8, rng.choice((0, 1, 2, 0x101)))
        c.w(a + 0xD8, rng.choice((0, 0, 5)))
        if rng.random() < 0.7:
            ms = m.alloc(0x20)
            c.w(a + 0x80, ms)
            if rng.random() < 0.8:
                org = m.alloc(0x200)
                c.w(ms + 0x10, org)
                c.w(org + 0xD8, rng.choice((0, 7)))
        cases.append(c.done(m.call(0x4CFFD0, ecx=a, pop=0), this=a))
    for _ in range(10):
        c = Case(m, '4CE880')
        a = m.alloc(0x200)
        c.w(a + 0x164, rng.getrandbits(32), True)
        m.call(0x4CE880, ecx=a, pop=0)
        cases.append(c.done(None, this=a))
    for _ in range(40):
        c = Case(m, '4CEE40')
        a = m.alloc(0x200)
        value = rng.choice((0, 0, 9))
        c.w(a, actor_vt(c, value))
        c.w(a + 0x98, rng.choice((0, 0, 3)))
        c.w(a + 0x9C, rng.choice((0, 4)))
        cases.append(c.done(m.call(0x4CEE40, ecx=a, pop=0), this=a, virtual100=value))
    for _ in range(40):
        c = Case(m, '4CEFA0')
        a = m.alloc(0x200)
        value = rng.getrandbits(32)
        c.w(a, actor_vt(c, value))
        if rng.random() < 0.7:
            ms = m.alloc(0x20)
            c.w(a + 0x80, ms)
            if rng.random() < 0.7:
                c.w(ms + 0x10, m.alloc(0x40))
        cases.append(c.done(m.call(0x4CEFA0, ecx=a, pop=0), this=a, virtual100=value))
    for _ in range(80):
        c = Case(m, rng.choice(('4D1240', '4D1240', '4D1290')))
        a = m.alloc(0x200)
        objects = []
        for _ in range(2):
            o = m.alloc(0x10)
            c.w(o, object_vt(c))
            c.w(o + 4, rng.choice((1, 1, 2, 0, 5)), True)
            objects.append(o)
        old = rng.choice((0, objects[0]))
        c.w(a + 0x154, old, True)
        if c.name == '4D1240':
            new = rng.choice((0, objects[0], objects[1], objects[1]))
            m.call(0x4D1240, ecx=a, args=(new,), pop=4)
            cases.append(c.done(None, this=a, object=new))
        else:
            m.call(0x4D1290, ecx=a, pop=0)
            cases.append(c.done(None, this=a))
    for _ in range(60):
        c = Case(m, '45E660')
        d = m.alloc(0x200)
        count = rng.choice((0, 1, 2, 4))
        slots = m.alloc(0x20)
        for k in range(max(count, 1)):
            c.w(slots + 4 * k, rng.choice((0, 0x1234)))
        c.w(d + 0x1BC, slots)
        c.w(d + 0x1C0, count)
        index = rng.choice((-1, 0, 1, 2, 3)) if count else rng.choice((-1, 0))
        if count and index >= count:
            index = -1
        cases.append(c.done(m.call(0x45E660, ecx=d, args=(index & 0xFFFFFFFF,), pop=4), this=d, index=index))
    for _ in range(30):
        c = Case(m, '471A40')
        o = m.alloc(0x40)
        for off in (0x18, 0x1C, 0x20):
            c.w(o + off, rng.getrandbits(32))
        index = rng.choice((-1, 0, 1, 2, 3, 7))
        cases.append(c.done(m.call(0x471A40, ecx=o, args=(index & 0xFFFFFFFF,), pop=4), this=o, index=index))
    h.report('native-actor-flags-fixtures.json',
             '4D07D0, 4D0960, 4CFFD0, 4CE880, 4CEE40, 4CEFA0, 4D1240, 4D1290, 45E660, 471A40.',
             cases, m.visited)


if __name__ == '__main__':
    main()
