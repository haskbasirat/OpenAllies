"""Native oracle for the Performer (component +74) turret intention 0x57DD90
and its clear 0x57DEA0. Original code runs, including 0x4C81E0, 0x529C30,
0x45DD30 and 0x4C8300; recorded seams: 0x4C8F20 (ability admission: the
context bytes and check_range are recorded, the case decides the result, and
the hook scribbles the context to show the per-ability reinitialisation) and
0x50AC00 (Updatable activation on Performer +8).
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

TURRETS = 4


def main():
    rng = random.Random(0x57DD90)
    m = h.Machine('Double53')
    m.u.mem_map(0, 0x1000)
    cases = []
    for index in range(60):
        events = []
        count = rng.randint(0, 5)
        turret_slots = [rng.choice((-1, 0, 1, 2, 3)) for _ in range(count)]
        tracks = [rng.random() < 0.7 for _ in range(TURRETS)]
        results = [rng.choice((0, 0, 3, 7)) for _ in range(count)]
        active = rng.choice([None] + list(range(count))) if count else None
        latch = rng.random() < 0.25
        with_target = rng.random() < 0.5
        event_kind = rng.choice((0, 1, 2, 5))
        point = [h.f32(rng.uniform(-500, 500)), h.f32(rng.uniform(-500, 500))]
        target_position = [h.f32(rng.uniform(-500, 500)) for _ in range(3)]
        initial = [[h.f32(rng.uniform(-9, 9)) for _ in range(3)] + [rng.choice((0, 1))] for _ in range(TURRETS)]

        perf = m.alloc(0x80)
        owner = m.alloc(0x80)
        definition = m.alloc(0x400)
        m.w(perf + 4, owner)
        m.w(owner + 4, definition)
        tdefs = m.alloc(4 * TURRETS)
        for t in range(TURRETS):
            td = m.alloc(0x40)
            m.wb(td + 0x31, int(tracks[t]))
            m.w(tdefs + 4 * t, td)
        m.w(definition + 0x1F8, tdefs)
        m.w(definition + 0x1FC, TURRETS)
        abilities = m.alloc(4 * max(count, 1))
        handles = []
        for a in range(count):
            ab = m.alloc(0x40)
            ad = m.alloc(0x80)
            m.w(ab + 8, ad)
            m.w(ad + 0x54, turret_slots[a] & 0xFFFFFFFF)
            m.w(abilities + 4 * a, ab)
            handles.append(ab)
        m.w(perf + 0x14, abilities)
        m.w(perf + 0x18, count)
        m.w(perf + 0x34, handles[active] if active is not None else 0)
        records = m.alloc(4 * TURRETS)
        recs = []
        for t in range(TURRETS):
            rec = m.alloc(0x20)
            m.w(rec + 0x10, initial[t][0])
            m.w(rec + 0x14, initial[t][1])
            m.w(rec + 0x18, initial[t][2])
            m.wb(rec + 0xC, initial[t][3])
            m.w(records + 4 * t, rec)
            recs.append(rec)
        m.w(perf + 0x4C, records)
        m.wb(perf + 0x5C, int(latch))
        target = 0
        if with_target:
            target = m.alloc(0x40)
            for k in range(3):
                m.w(target + 0x20 + 4 * k, target_position[k])
        pt = m.alloc(8)
        m.w(pt, point[0])
        m.w(pt + 4, point[1])

        def admission(mm, events=events, handles=handles, results=results):
            ability = handles.index(mm.reg(UC_X86_REG_ECX))
            ctx = mm.arg(0)
            words = list(struct.unpack('<12I', bytes(mm.u.mem_read(ctx, 0x30))))
            events.append({'can_use': ability, 'check_range': mm.arg(1), 'context': words})
            mm.u.mem_write(ctx, b'\xA5' * 0x30)
            mm.ret(8, results[ability])

        def activate(mm, events=events, perf=perf):
            events.append({'activate': mm.reg(UC_X86_REG_ECX) - perf, 'arg': mm.arg(0)})
            mm.ret(4)
        m.hooks[0x4C8F20] = admission
        m.hooks[0x50AC00] = activate
        m.call(0x57DD90, ecx=perf, args=(event_kind, target, pt), pop=12)
        after = [[m.r(r + 0x10), m.r(r + 0x14), m.r(r + 0x18), m.rb(r + 0xC)] for r in recs]
        latch_after = m.rb(perf + 0x5C)
        m.call(0x57DEA0, ecx=perf, pop=0)
        cleared = [m.rb(r + 0xC) for r in recs]
        cases.append({'event_kind': event_kind, 'turret_slots': turret_slots, 'tracks': tracks,
                      'results': results, 'active': active, 'latch': latch, 'target': with_target,
                      'target_position': target_position, 'point': point, 'initial': initial,
                      'events': events, 'after': after, 'latch_after': latch_after, 'cleared': cleared,
                      'owner_handle': owner, 'target_handle': target})
    h.report('native-turret-intention-fixtures.json',
             '0x57DD90 Performer turret intention and 0x57DEA0 clear (4C8F20/50AC00 recorded).',
             cases, m.visited)


if __name__ == '__main__':
    main()
