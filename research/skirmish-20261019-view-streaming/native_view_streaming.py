"""VGrid view streaming (VWorld+94 On/Near lists and VThing proximity), run as
ORIGINAL AA.exe code under Unicorn:

  42FAD0(block)  the per-terrain-block cell sweep 42FF50 runs before 720080:
                 cell window from block +AC/+B0, TerrainMap+A0 and VGrid+24;
                 per node view: radius max(view+74, vt3C - vt40) (vtFC 5A4ED0
                 -> view+7C Model +74/+78), 43D050(xy, 0x222) height, the
                 sphere test 42F4F0 against VGrid+A0..+FC, 45B2B0, 45B930,
                 45B8F0 (On, +54 bit18) / 45B910 (Near, bit19), 45B4C0/45B0F0
  42F670         4312F0's tail: B39BB8 counter rotation, Near then On walk:
                 45B2E0 -> vt14 455290/451800 -> 45B330 (+18 0/1/2 with
                 vt+D8 45B3D0 -> vt24 45B430, vt+DC 45B3E0 -> vt28 45BB50,
                 vt+E0, vt+E4), vt+B0/A8 (real constants), vt0 for On views
                 at proximity 2, the Near move of proximity 1 On views.

Views carry copies of the real VObject A02798 / VActor A02270 / VOrg A01EF0
vtables. Seams (recorded in call order): view vt+0 (update), vt+D0 (model
request 4553D0/4518A0/44FED0), vt+D4 (model release 459690/4518B0/44F660),
vt+E0 (show), vt+E4 (hide), Data vt+14 (request) / vt+18 (release) at
view+10, and 431550 (the ported view_heightfield sample, answered f32).
Default compares the fixture; --write regenerates it.
"""
from pathlib import Path
import sys, json, hashlib, struct
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm

VTABLES = {'object': 0xa02798, 'actor': 0xa02270, 'org': 0xa01ef0}
f32 = nm.f32


class World:
    def __init__(self, c, pc, image):
        m = self.m = nm.Machine(pc)
        if image == 'v14':
            old = (nm.original.image_size + 4095) & ~4095
            new = (nm.modified.image_size + 4095) & ~4095
            if new > old:
                m.u.mem_map(nm.original.base + old, new - old)
            for vs, r, size, off in nm.modified.sections:
                if size:
                    m.u.mem_write(nm.modified.base + r, nm.modified.b[off:off + size])
        self.c = c
        self.trace = []
        self.views = {}
        self.names = {}
        g = c['grid']
        vis = self.vis = m.alloc(0x200)
        cells = m.alloc(8 * g['columns'] * g['rows'])
        m.w(vis, cells)
        m.w(vis + 0x1c, g['columns'])
        m.w(vis + 0x20, g['rows'])
        m.w(vis + 0x24, g['shift'])
        for i, plane in enumerate(c['planes']):
            for j, v in enumerate(plane):
                m.w(vis + 0xa0 + 16 * i + 4 * j, f32(v))
        for name, v in c['views'].items():
            self.view(name, v)
        for index, names in c.get('cells', {}).items():
            head = 0
            tail = 0
            for name in reversed(names):
                node = m.alloc(8)
                m.w(node, self.views[name])
                m.w(node + 4, head)
                if not head:
                    tail = node
                head = node
            m.w(cells + 8 * int(index), head)
            m.w(cells + 8 * int(index) + 4, tail)
        for offset, key in [(0x100, 'on'), (0x108, 'near')]:
            order = c.get(key, [])
            ptrs = [self.views[n] for n in order]
            for i, p in enumerate(ptrs):
                m.w(p + 0xc, vis + offset)
                m.w(p + 8, ptrs[i - 1] if i else 0)
                m.w(p + 4, ptrs[i + 1] if i + 1 < len(ptrs) else 0)
            m.w(vis + offset, ptrs[-1] if ptrs else 0)
            m.w(vis + offset + 4, ptrs[0] if ptrs else 0)
        terrain = m.alloc(0x200)
        m.w(terrain + 0xa0, c['block_size'])
        m.w(0xb36518, terrain)
        vworld = m.alloc(0x200)
        heightfield = m.alloc(0x40)
        m.w(vworld + 0xbc, heightfield)
        m.w(0xb36538, vworld)
        stats = self.stats = m.alloc(0x60)
        for i, v in enumerate(c.get('stats', [0] * 8)):
            m.w(stats + 0x24 + 4 * i, v)
        m.w(0xb39bb8, stats)

        def height(mm):
            xy = mm.arg(1)
            key = [mm.r(xy), mm.r(xy + 4)]
            self.trace.append({'call': 'height_431550', 'xy': key})
            bits = next(f32(v['height']) for v in c['views'].values() if [f32(v['pos'][0]), f32(v['pos'][1])] == key)
            mm.ret_st0_f32(bits, 4)
        m.seam(0x431550, height)

    def view(self, name, v):
        m = self.m
        p = m.alloc(0x200)
        self.views[name] = p
        self.names[p] = name

        def seam(kind, pop):
            def fn(mm, name=name):
                event = {'call': kind, 'view': name}
                if pop:
                    event['arg'] = mm.arg(1)
                self.trace.append(event)
                mm.ret(0, pop)
            return fn
        vt = m.alloc(0x200)
        m.u.mem_write(vt, bytes(m.u.mem_read(VTABLES[v['type']], 0x200)))
        for slot, kind, pop in [(0, 'update_0', 0), (0xd0, 'model_request_d0', 4), (0xd4, 'model_release_d4', 0),
                                (0xe0, 'show_e0', 0), (0xe4, 'hide_e4', 0)]:
            m.w(vt + slot, m.stub(seam(kind, pop)))
        m.w(p, vt)
        data = m.alloc(0x40)
        dvt = m.alloc(0x40)

        def request(mm, name=name):
            self.trace.append({'call': 'data_request_14', 'view': name, 'delta': mm.arg(1), 'mode': mm.arg(2)})
            mm.ret(0, 8)

        def release(mm, name=name):
            self.trace.append({'call': 'data_release_18', 'view': name, 'delta': mm.arg(1)})
            mm.ret(0, 4)
        m.w(dvt + 0x14, m.stub(request))
        m.w(dvt + 0x18, m.stub(release))
        m.w(data, dvt)
        m.w(p + 0x10, data)
        for i, axis in enumerate(v['pos']):
            m.w(p + 0x38 + 4 * i, f32(axis))
        m.w(p + 0x74, f32(v.get('radius', 0.0)))
        model = m.alloc(0x80)
        top, bottom = v.get('model', [0.0, 0.0])
        m.w(model + 0x74, f32(top))
        m.w(model + 0x78, f32(bottom))
        m.w(p + 0x7c, model)
        m.w(p + 0x9c, 0)
        m.w(p + 0x18, v.get('f18', 0))
        m.w(p + 0x1c, v.get('f1c', 0))
        m.w(p + 0x54, v.get('f54', 0x01000000))
        m.w(p + 0x58, v.get('f58', 0))

    def walk(self, offset, link, start):
        out, p, seen = [], self.m.r(self.vis + offset + start), 0
        while p:
            out.append(self.names[p])
            p = self.m.r(p + link)
            seen += 1
            assert seen < 1000
        return out

    def state(self):
        m = self.m
        out = {'views': {}, 'stats': [m.r(self.stats + 0x24 + 4 * i) for i in range(8)]}
        for name, p in self.views.items():
            owner = m.r(p + 0xc)
            out['views'][name] = {'f18': m.r(p + 0x18), 'f1c': m.r(p + 0x1c), 'f54': m.r(p + 0x54), 'f58': m.r(p + 0x58),
                                  'list': {0: None, self.vis + 0x100: 'on', self.vis + 0x108: 'near'}[owner]}
        for key, offset in [('on', 0x100), ('near', 0x108)]:
            head = self.walk(offset, 4, 4)
            tail = self.walk(offset, 8, 0)
            assert head == list(reversed(tail)), (head, tail)
            out[key] = head
        return out


def run(c, pc, image):
    w = World(c, pc, image)
    m = w.m
    for frame in c['frames']:
        for b in frame['blocks']:
            blk = m.alloc(0x200)
            m.w(blk + 0xac, b['column'])
            m.w(blk + 0xb0, b['row'])
            m.w(blk + 0xb8, b['state'])
            m.call(0x42fad0, w.vis, [blk])
        if frame.get('traverse', True):
            m.call(0x42f670, w.vis, [])
    return {**w.state(), 'trace': w.trace}, m.visited


BOX = [[1, 0, 0, 0.0], [-1, 0, 0, -64.0], [0, 1, 0, 0.0], [0, -1, 0, -64.0], [0, 0, 1, -100.0], [0, 0, -1, -100.0]]
FAR = [[1, 0, 0, 500.0], [-1, 0, 0, -600.0], [0, 1, 0, 500.0], [0, -1, 0, -600.0], [0, 0, 1, -100.0], [0, 0, -1, -100.0]]


def V(t, x, y, z=0.0, **kw):
    return {'type': t, 'pos': [x, y, z], 'height': kw.pop('height', 0.0), **kw}


def cases():
    grid = {'columns': 4, 'rows': 4, 'shift': 5}
    # One block (32 units, the whole cell at shift 5) inside / outside the box.
    for t in VTABLES:
        for planes, state in [(BOX, 0), (BOX, 2), (FAR, 0), (FAR, 1), (FAR, 2)]:
            views = {'a': V(t, 10.0, 12.0, radius=2.0, model=[3.0, 1.0])}
            yield {'grid': grid, 'block_size': 32, 'planes': planes, 'views': views, 'cells': {'0': ['a']},
                   'frames': [{'blocks': [{'column': 0, 'row': 0, 'state': state}]}]}
    # Lifecycle across frames: enter (0->2), stay, leave to Near (2->1), out (1->0).
    for t in VTABLES:
        views = {'a': V(t, 10.0, 12.0, radius=2.0, model=[3.0, 1.0]), 'b': V(t, 40.0, 12.0, radius=1.0)}
        frames = [{'blocks': [{'column': 0, 'row': 0, 'state': 2}, {'column': 1, 'row': 0, 'state': 2}]},
                  {'blocks': [{'column': 0, 'row': 0, 'state': 2}]},
                  {'blocks': []},
                  {'blocks': []},
                  {'blocks': [{'column': 0, 'row': 0, 'state': 2}]}]
        yield {'grid': grid, 'block_size': 32, 'planes': [[1, 0, 0, 0.0], [-1, 0, 0, -32.0], [0, 1, 0, 0.0], [0, -1, 0, -64.0],
                                                        [0, 0, 1, -100.0], [0, 0, -1, -100.0]],
               'views': views, 'cells': {'0': ['a'], '1': ['b']}, 'frames': frames}
        frames2 = [{'blocks': [{'column': 0, 'row': 0, 'state': 1}]}, {'blocks': [{'column': 0, 'row': 0, 'state': 2}]},
                   {'blocks': []}]
        yield {'grid': grid, 'block_size': 32, 'planes': FAR, 'views': {'a': V(t, 10.0, 12.0)},
               'cells': {'0': ['a']}, 'frames': frames2}
    # Height and radius arithmetic at the plane boundary (all precisions).
    for z, h, top, bottom, radius in [(5.0, 1.0, 3.0, 1.0, 0.5), (5.0, 1.0, 1.0, 3.0, 0.5), (0.0, 0.0, 0.0, 0.0, 0.0),
                                      (1.0, 0.1, 0.3, 0.1, 0.2), (100.25, 0.125, 7.75, 0.5, 3.0), (-3.0, 2.0, 1.0, 0.0, 9.0)]:
        for edge in [-1.0, -0.5, 0.0, 0.5, 0.75, 1.0, 1.25, 2.0, 4.0, 1e-7]:
            planes = [[1, 0, 0, 10.0 + edge], [-1, 0, 0, -64.0], [0, 1, 0, 0.0], [0, -1, 0, -64.0],
                      [0, 0, 1, -100.0], [0, 0, -1, -150.0]]
            yield {'grid': grid, 'block_size': 32, 'planes': planes, 'fp': True,
                   'views': {'a': V('object', 10.0, 12.0, z, height=h, radius=radius, model=[top, bottom])},
                   'cells': {'0': ['a']}, 'frames': [{'blocks': [{'column': 0, 'row': 0, 'state': 1}], 'traverse': False}]}
    # Several views in a cell, repeated cells, existing list members and stats.
    views = {'a': V('actor', 3.0, 3.0, f18=2, f1c=0, f58=1), 'b': V('org', 5.0, 4.0, f18=1, f58=1),
             'c': V('object', 6.0, 7.0, f18=0), 'd': V('object', 70.0, 70.0, f18=3, f54=0x01000008)}
    for planes in [BOX, FAR]:
        yield {'grid': grid, 'block_size': 32, 'planes': planes, 'views': views,
               'cells': {'0': ['a', 'b', 'c'], '1': ['b'], '5': ['d']}, 'on': ['a', 'd'], 'near': ['b'],
               'stats': [1, 2, 3, 4, 5, 6, 7, 8],
               'frames': [{'blocks': [{'column': 0, 'row': 0, 'state': 2}, {'column': 1, 'row': 0, 'state': 1},
                                      {'column': 1, 'row': 1, 'state': 2}]}, {'blocks': []}]}
    # Block window larger than one cell (block 64 at shift 5: 2x2 cells), shifted rows.
    yield {'grid': grid, 'block_size': 64, 'planes': BOX,
           'views': {'p': V('actor', 5.0, 40.0), 'q': V('object', 40.0, 40.0), 'r': V('org', 40.0, 5.0)},
           'cells': {'4': ['p'], '5': ['q'], '1': ['r']},
           'frames': [{'blocks': [{'column': 0, 'row': 0, 'state': 2}]}, {'blocks': [{'column': 0, 'row': 0, 'state': 0}]}]}
    # Proximity transitions from every start state through 45B330 directly
    # (the On/Near walk with +1C preset).
    for start in [0, 1, 2, 3]:
        for want in [0, 1, 2]:
            for where in ['on', 'near']:
                for f54 in [0x01000000, 0x01000008]:
                    v = V('object', 1.0, 1.0, f18=start, f1c=want, f58=1 if start else 0,
                          f54=f54 | (0x40000 if where == 'on' else 0x80000))
                    yield {'grid': grid, 'block_size': 32, 'planes': BOX, 'views': {'a': v}, where: ['a'],
                           'frames': [{'blocks': []}]}


def main():
    rows, pins = [], {}
    for c in cases():
        for pc in (nm.PRECISIONS if c.get('fp') else ['Double53']):
            pair = []
            for image in ['original', 'v14']:
                out, visited = run(c, pc, image)
                pair.append(out)
                pins.update(visited)
            assert pair[0] == pair[1], c
            rows.append({'precision': pc, 'input': c, 'output': pair[0]})
    for a, b in pins.items():
        assert b == nm.original.range(a, len(b)) == nm.modified.range(a, len(b))
    doc = {**nm.header(__file__, __doc__), 'rows': rows, 'instructions': nm.instructions(pins)}
    path = HERE / 'native-view-streaming.json'
    if '--write' in sys.argv:
        path.write_text(json.dumps(doc, indent=1))
    else:
        assert json.loads(path.read_text()) == doc, 'fixture differs'
    print(json.dumps({'rows': len(rows), 'pins': len(pins), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
