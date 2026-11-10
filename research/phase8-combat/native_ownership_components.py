"""Native oracle for the kingdom-change component callbacks 0x589050,
0x579F10, 0x584F10 and 0x585FF0.

Original code runs, including the handle table at [0xB364C0]. Recorded
seams: the actor virtual +160 (arguments recorded; for 0x584F10 some calls
shrink the Settlement +1C / +28 counts, as a kingdom change may, to show the
counts are reread), 0x4364C0 (member hash iteration, scripted order) and
0x6F7810 (allocator, for the 0x585FF0 list node).

Usage: native_ownership_components.py [AA.exe] (checked against the accepted SHA-256).
"""
from pathlib import Path
import hashlib
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

if len(sys.argv) > 1:
    assert hashlib.sha256(Path(sys.argv[1]).read_bytes()).hexdigest() == h.ORIGINAL_SHA, sys.argv[1]


def main():
    rng = random.Random(0x584F10)
    m = h.Machine('Double53')
    m.u.mem_map(0, 0x1000)
    registry = m.alloc(0x20004 + 0x10000 * 4)
    m.w(h.REGISTRY_ROOT, registry)
    events = []
    labels = {}
    shrink = {}
    orders = {}
    state = {'slot': 0x20}

    def changed(mm):
        actor = mm.reg(UC_X86_REG_ECX)
        events.append({'change': labels[actor], 'kingdom': mm.arg(0)})
        if actor in shrink:
            where, by = shrink[actor]
            mm.w(where, mm.r(where) - by)
        mm.ret(4)
    vt = m.vtable({0x160: changed})

    def next_member(mm):
        it = mm.reg(UC_X86_REG_ECX)
        order = orders[mm.r(it)]
        index = struct.unpack('<i', struct.pack('<I', mm.r(it + 4)))[0]
        position = order.index(index) + 1 if index != -1 else 0
        mm.w(it + 4, order[position] if position < len(order) else 0x7FFFFFFF)
        mm.ret(0)
    m.hooks[0x4364C0] = next_member
    m.hooks[0x6F7810] = lambda mm: mm.ret(0, mm.alloc(mm.arg(0)))

    def actor(name):
        a = m.alloc(0x40)
        m.w(a, vt)
        labels[a] = name
        return a

    def handle(a, kind):
        state['slot'] += 1
        slot = state['slot']
        generation = 0x3000 + slot
        m.u.mem_write(registry + 4 + slot * 2, struct.pack('<H', generation))
        m.w(registry + 0x20004 + slot * 4, 0 if kind == 'empty' else a)
        value = (generation << 16) | slot
        return value + 0x10000 if kind == 'stale' else value

    cases = []
    for _ in range(40):
        events.clear()
        kind = rng.choice(('live', 'live', 'stale', 'empty', 'zero'))
        kingdom = rng.choice((0, 0x4B000001, 0x4B000002))
        comp = m.alloc(0x40)
        target = actor('linked')
        m.w(comp + 0x34, 0 if kind == 'zero' else handle(target, kind))
        m.call(0x589050, ecx=comp, args=(kingdom,), pop=4)
        cases.append({'function': 'linked', 'kind': kind, 'kingdom': kingdom, 'events': list(events)})
    for _ in range(40):
        events.clear()
        kingdom = rng.choice((0, 0x4B000001))
        comp = m.alloc(0x40)
        count = rng.randint(0, 6)
        arr = m.alloc(4 * 8)
        for k in range(count):
            m.w(arr + 4 * k, actor(f'm{k}'))
        m.w(comp + 0x28, arr)
        order = list(range(count))
        rng.shuffle(order)
        orders[comp] = order
        m.call(0x579F10, ecx=comp, args=(kingdom,), pop=4)
        cases.append({'function': 'organization', 'count': count, 'order': order, 'kingdom': kingdom,
                      'events': list(events)})
    for _ in range(60):
        events.clear()
        shrink.clear()
        kingdom = rng.choice((0, 0x4B000001))
        comp = m.alloc(0x40)
        has = rng.random() < 0.7
        m.w(comp + 0x14, actor('structure') if has else 0)
        lists = {}
        for name, base in (('structures', 0x18), ('segments', 0x24)):
            n = rng.randint(0, 6)
            arr = m.alloc(4 * 8)
            entries = []
            for k in range(n):
                if rng.random() < 0.25:
                    entries.append(None)
                    continue
                a = actor(f'{name[:2]}{k}')
                m.w(arr + 4 * k, a)
                entries.append(f'{name[:2]}{k}')
                if rng.random() < 0.15:
                    by = rng.choice((1, 2))
                    shrink[a] = (comp + base + 4, by)
                    entries[-1] = {'id': entries[-1], 'shrink': by}
            m.w(comp + base, arr)
            m.w(comp + base + 4, n)
            lists[name] = entries
        m.call(0x584F10, ecx=comp, args=(kingdom,), pop=4)
        cases.append({'function': 'wall', 'structure': has, 'lists': lists, 'kingdom': kingdom,
                      'events': list(events)})
    for _ in range(50):
        comp = m.alloc(0x80)
        seen = [rng.choice((0x4B000001, 0x4B000002, 0x4B000003, 0x4B000004)) for _ in range(rng.randint(0, 3))]
        seen = list(dict.fromkeys(seen))
        head = 0
        tail = 0
        for k in reversed(seen):
            node = m.alloc(8)
            m.w(node, k)
            m.w(node + 4, head)
            if head == 0:
                tail = node
            head = node
        m.w(comp + 0x70, head)
        m.w(comp + 0x74, tail)
        kingdom = rng.choice((0x4B000001, 0x4B000002, 0x4B000003, 0x4B000004))
        m.call(0x585FF0, ecx=comp, args=(kingdom,), pop=4)
        after = []
        node = m.r(comp + 0x70)
        last = 0
        while node:
            after.append(m.r(node))
            last = node
            node = m.r(node + 4)
        cases.append({'function': 'seen', 'seen': seen, 'kingdom': kingdom, 'after': after,
                      'tail_is_last': m.r(comp + 0x74) == last})
    h.report('native-ownership-components-fixtures.json',
             '0x589050 / 0x579F10 / 0x584F10 / 0x585FF0 kingdom-change component callbacks '
             '(virtual +160, 4364C0 and 6F7810 recorded).',
             cases, m.visited)


if __name__ == '__main__':
    main()
