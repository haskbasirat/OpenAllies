"""View/actor leaves of the orders input that the view directory binds, run as
ORIGINAL AA.exe code under Unicorn (no seams: every reached instruction is
native, objects carry the real class vtables):

  view vt+0x9C  of VObject A02798 / VActor A02270 / organization A01EF0
  45B7A0        view+14 thing and its vt+0xB4, for the GThing class vtables
                that 4F9C10 constructs (runtime class 2..10)
  45B7D0        view+14 NULL -> 1, else Thing+8 & 1
  4D12C0        Thing+8 bit29, else 4D10A0 on +100/+104
  4CE6C0        +7C NULL and +80 -> +10 nonzero -> that, else self
  58CB30        view+10

Default compares the fixture; --write regenerates it.
"""
from pathlib import Path
import sys, json, hashlib, itertools
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm

VIEW_VTABLES = {'VObject': 0xa02798, 'VActor': 0xa02270, 'VOrganization': 0xa01ef0}
#4F9C10 runtime class -> constructor PRIMARY vtable (the Thing+0 store; class6
#4DB610 also stores A289D8 at +60 and class8 4DE350 A29464 at +180).
THING_VTABLES = {2: 0xa2a2a0, 3: 0xa273f0, 4: 0xa299d8, 5: 0xa29780, 6: 0xa28a00, 7: 0xa28b88, 8: 0xa29490, 9: 0xa28e90, 10: 0xa28d18}


def machine(pc, image):
    m = nm.Machine(pc)
    if image == 'v14':
        old = (nm.original.image_size + 4095) & ~4095
        new = (nm.modified.image_size + 4095) & ~4095
        if new > old:
            m.u.mem_map(nm.original.base + old, new - old)
        for vs, r, size, off in nm.modified.sections:
            if size:
                m.u.mem_write(nm.modified.base + r, nm.modified.b[off:off + size])
    return m


def cases():
    for name in VIEW_VTABLES:
        yield {'leaf': 'pickable_9c', 'view': name}
    for cls in THING_VTABLES:
        yield {'leaf': '45b7a0', 'class': cls, 'word_58': 0}
    for w in [1, 0x7fffffff, 0x80000000, 0xffffffff]:
        yield {'leaf': '45b7a0', 'class': 6, 'word_58': w}
    yield {'leaf': '45b7a0', 'class': None, 'word_58': 0}
    for flags in [0, 1, 0x1000, 0x20000001, 0xfffffffe]:
        yield {'leaf': '45b7d0', 'present': True, 'flags': flags}
    yield {'leaf': '45b7d0', 'present': False, 'flags': 0}
    for flags, low, high in itertools.product([0, 1 << 29, 0xdfffffff], [0, 1 << 25, 1 << 26, 0xf9ffffff], [0, 4, 0x10, 0xffffffeb]):
        yield {'leaf': '4d12c0', 'flags': flags, 'state_100': low, 'state_104': high}
    for org, element, parent in itertools.product([False, True], [False, True], [False, True]):
        yield {'leaf': '4ce6c0', 'organization_7c': org, 'element_80': element, 'element_parent_10': parent}
    yield {'leaf': '58cb30'}


def run(c, pc, image):
    m = machine(pc, image)
    leaf = c['leaf']
    if leaf == 'pickable_9c':
        v = m.alloc(0x100)
        vt = VIEW_VTABLES[c['view']]
        m.w(v, vt)
        entry = m.r(vt + 0x9c)
        return {'value': m.call(entry, v, []) & 0xff, 'entry': hex(entry)}, m.visited
    if leaf == '45b7a0':
        v = m.alloc(0x100)
        if c['class'] is None:
            m.w(v + 0x14, 0)
        else:
            t = m.alloc(0x200)
            m.w(t, THING_VTABLES[c['class']])
            m.w(t + 0x58, c['word_58'])
            m.w(v + 0x14, t)
        out = m.call(0x45b7a0, v, [])
        return {'value': int(out != 0), 'returns_thing': out == m.r(v + 0x14) and out != 0,
                'b4': hex(m.r(THING_VTABLES[c['class']] + 0xb4)) if c['class'] else None}, m.visited
    if leaf == '45b7d0':
        v = m.alloc(0x100)
        t = m.alloc(0x100)
        m.w(t + 8, c['flags'])
        m.w(v + 0x14, t if c['present'] else 0)
        return {'value': m.call(0x45b7d0, v, []) & 0xff}, m.visited
    if leaf == '4d12c0':
        t = m.alloc(0x200)
        m.w(t + 8, c['flags'])
        m.w(t + 0x100, c['state_100'])
        m.w(t + 0x104, c['state_104'])
        return {'value': m.call(0x4d12c0, t, []) & 0xff}, m.visited
    if leaf == '4ce6c0':
        a = m.alloc(0x100)
        org = m.alloc(0x100)
        element = m.alloc(0x40)
        m.w(a + 0x7c, m.alloc(0x40) if c['organization_7c'] else 0)
        m.w(element + 0x10, org if c['element_parent_10'] else 0)
        m.w(a + 0x80, element if c['element_80'] else 0)
        out = m.call(0x4ce6c0, a, [])
        return {'result': 'organization' if out == org else 'self' if out == a else hex(out)}, m.visited
    if leaf == '58cb30':
        v = m.alloc(0x40)
        m.w(v + 0x10, 0x12345678)
        return {'value': m.call(0x58cb30, v, [])}, m.visited
    raise ValueError(leaf)


def main():
    rows, pins = [], {}
    for pc in nm.PRECISIONS:
        for c in cases():
            pair = []
            for image in ['original', 'v14']:
                out, visited = run(c, pc, image)
                pair.append(out)
                pins.update(visited)
            assert pair[0] == pair[1]
            rows.append({'precision': pc, 'input': c, 'output': pair[0]})
    for a, b in pins.items():
        assert b == nm.original.range(a, len(b)) == nm.modified.range(a, len(b))
    doc = {**nm.header(__file__, __doc__), 'rows': rows, 'instructions': nm.instructions(pins),
           'view_vtables': {k: hex(v) for k, v in VIEW_VTABLES.items()},
           'thing_vtables': {str(k): hex(v) for k, v in THING_VTABLES.items()}}
    path = HERE / 'native-view-order-leaves.json'
    if '--write' in sys.argv:
        path.write_text(json.dumps(doc, indent=1))
    else:
        assert json.loads(path.read_text()) == doc, 'fixture differs'
    print(json.dumps({'rows': len(rows), 'pins': len(pins), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
