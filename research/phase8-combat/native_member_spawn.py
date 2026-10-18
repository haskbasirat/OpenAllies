"""Native oracle for organization member spawning 0x57AD30 up to the actor
creation seam 0x4FBF40 (recorded: descriptor bytes, owner, slot; returns the
case's result). Owner virtual +108 is recorded; the CString manager 0x856370
returns a fake manager whose nil string is the empty name.
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402


def main():
    rng = random.Random(0x57AD30)
    m = h.Machine('Double53')
    m.u.mem_map(0, 0x1000)
    manager = m.alloc(0x10)
    nil = m.alloc(0x20)
    m.w(nil, manager)
    m.w(nil + 0xC, 1000)
    m.w(manager, m.vtable({0xC: lambda mm: mm.ret(0, nil)}))
    m.hooks[0x856370] = lambda mm: mm.ret(0, manager)
    world = m.alloc(0x100)
    m.w(h.WORLD_ROOT, world)
    cases = []
    for index in range(40):
        count = rng.randint(1, 5)
        slot = rng.randrange(count)
        occupied = rng.random() < 0.3
        result = rng.choice((0, 1))
        flag = rng.choice((0, 1, 0x101))
        org = m.alloc(0x100)
        owner = m.alloc(0x100)
        allegiance = 0x6000 + index
        calls = []
        m.w(owner, m.vtable({0x108: lambda mm, a=allegiance: (calls.append('allegiance'), mm.ret(0, a))}))
        orgdef = m.alloc(0x40)
        types = m.alloc(4 * count)
        defs = m.alloc(4 * count)
        members = m.alloc(4 * count)
        for i in range(count):
            m.w(types + 4 * i, count - 1 - i)
            m.w(defs + 4 * (count - 1 - i), 0x7000 + i)
        if occupied:
            m.w(members + 4 * slot, m.alloc(0x40))
        m.w(org + 4, owner)
        m.w(org + 0x18, orgdef)
        m.w(orgdef + 0x28, types)
        m.w(org + 0x1C, defs)
        m.w(org + 0x28, members)

        def create(mm, calls=calls, result=result):
            d = mm.arg(0)
            raw = bytes(mm.u.mem_read(d, 0x54))
            words = list(struct.unpack('<21I', raw))
            calls.append({'create': {'world': mm.reg(UC_X86_REG_ECX) == world, 'owner': mm.arg(1) == owner,
                                     'slot': mm.arg(2), 'words': words,
                                     'name_empty': mm.r(d + 0x1C) == nil + 0x10}})
            mm.ret(12, result)
        m.hooks[0x4FBF40] = create
        al = m.call(0x57AD30, ecx=org, args=(slot, flag), pop=8) & 0xFF
        cases.append({'count': count, 'slot': slot, 'occupied': occupied, 'flag': flag, 'result': result,
                      'allegiance': allegiance, 'definition': 0x7000 + slot, 'returned': al, 'calls': calls})
    h.report('native-member-spawn-fixtures.json', '0x57AD30 member spawn up to 0x4FBF40 (recorded).', cases,
             m.visited)


if __name__ == '__main__':
    main()
