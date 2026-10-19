"""Original/v14 KKC_UI press pick: view element vt+0x38 (GROUP 0x5E2B50,
every other class 0x5E1680), widget vt+0x7C (0x5EDD30 parent walk, tooltip
class A508B0 0x6A71D0) and the UI+0x60 admission 0x5EFA80, executed
unchanged on laboratory widget/view-element trees.

Memory laid out as the original objects: view element +0 vtable (A50CB8
GROUP, A50BD8 DUMMY/base), +0C owner widget, +20..+2C outer L/T/R/B,
+54..+60 client L/T/R/B, +88 flags (bit 2 VISIBLE), GROUP +C8/+CC list
head/tail (node +0 element, +4 previous, +8 next, +C key) and +D0
transparent; widget +0 vtable (A52350 base, A508B0 tooltip), +8 parent,
+14 top level, +24 flags (bit 21 intercept_pick); KKC_UI [B3653C] +74 root,
+8C focus list (node +0 widget). No seams: every executed instruction is the
original's and is pinned byte-identical in both images.
"""
from pathlib import Path
import sys, json, hashlib, random, struct
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
from native_machine import Machine as Base, original, modified, PRECISIONS
from unicorn.x86_const import UC_X86_REG_EIP

UI = 0xB3653C
VT_GROUP, VT_BASE = 0xA50CB8, 0xA50BD8
VT_WIDGET, VT_TOOLTIP = 0xA52350, 0xA508B0
NAN = 0x7fc00000


def f2b(x):
    return struct.unpack('<I', struct.pack('<f', x))[0]


class Machine(Base):
    def __init__(self, image, precision):
        super().__init__(precision); self.image = image
        if image == 'v14':
            old = (original.image_size + 0xfff) & ~0xfff; new = (modified.image_size + 0xfff) & ~0xfff
            if new > old: self.u.mem_map(original.base + old, new - old)
            for vs, rva, raw, off in modified.sections: self.u.mem_write(modified.base + rva, modified.b[off:off + raw])
        self.actual = {}

    def _hook(self, u, pc, size, data):
        super()._hook(u, pc, size, data)
        if pc not in self.seams and u.reg_read(UC_X86_REG_EIP) == pc and original.base <= pc < original.base + original.image_size:
            self.actual[pc] = bytes(u.mem_read(pc, size)).hex()


def build(m, case):
    ws = []
    for w in case['widgets']:
        p = m.alloc(0x100); ws.append(p)
        m.w(p, VT_TOOLTIP if w['vt_7c_true'] else VT_WIDGET); m.w(p + 0x24, w['flags_24'])
    for p, w in zip(ws, case['widgets']):
        m.w(p + 8, ws[w['parent_08']] if w['parent_08'] is not None else 0)
        m.w(p + 0x14, ws[w['top_14']] if w['top_14'] is not None else 0)
    vs = [m.alloc(0xe0) for _ in case['ves']]
    for p, v in zip(vs, case['ves']):
        m.w(p, VT_GROUP if v['group_d0'] is not None else VT_BASE)
        m.w(p + 0xc, ws[v['owner_0c']] if v['owner_0c'] is not None else 0)
        for i in range(4):
            m.w(p + 0x20 + 4 * i, v['outer_20'][i]); m.w(p + 0x54 + 4 * i, v['client_54'][i])
        m.w(p + 0x88, v['flags_88'])
        if v['group_d0'] is not None:
            m.wb(p + 0xd0, 1 if v['group_d0'] else 0)
            nodes = [m.alloc(0x10) for _ in v['children']]
            for k, (n, c) in enumerate(zip(nodes, v['children'])):
                m.w(n, vs[c]); m.w(n + 4, nodes[k - 1] if k else 0); m.w(n + 8, nodes[k + 1] if k + 1 < len(nodes) else 0)
                m.w(n + 0xc, f2b(-3.4028234663852886e38))
            m.w(p + 0xc8, nodes[0] if nodes else 0); m.w(p + 0xcc, nodes[-1] if nodes else 0)
    return ws, vs


def run(m, case):
    m.seams = {}; m.actual = {}
    ws, vs = build(m, case)
    index = {p: i for i, p in enumerate(ws)}
    out = []
    for q in case['queries']:
        r = m.call(0x5e2b50 if case['ves'][q['ve']]['group_d0'] is not None else 0x5e1680,
                   vs[q['ve']], (q['x'], q['y'], q['flag']), pop=12)
        assert r == 0 or r in index, hex(r)
        out.append(index.get(r))
    ui = m.alloc(0x200); m.w(UI, ui)
    admits = []
    for a in case['admits']:
        m.w(ui + 0x74, ws[a['root_74']])
        if a['focus_8c'] is None:
            m.w(ui + 0x8c, 0)
        else:
            node = m.alloc(0x10); m.w(node, ws[a['focus_8c']]); m.w(ui + 0x8c, node)
        admits.append(m.call(0x5efa80, ws[a['widget']], ()) & 0xff)
    return dict(picks=out, admits=admits)


def rect(rng, parent=None):
    """L, T, R, B bits: usually nested in `parent` (L/T/R/B floats), sometimes
    anywhere, with equal edges, empty rects, -0.0 and NaN edges."""
    if parent is not None and all(z == z for z in parent) and rng.random() < 0.8:
        pl, pt, pr, pb = parent
        w, h = max(pr - pl, 0), max(pb - pt, 0)
        l = pl + rng.choice([0, 0, 0.25, 0.5]) * w + rng.choice([0, 0, -1, 1])
        t = pt + rng.choice([0, 0, 0.25, 0.5]) * h + rng.choice([0, 0, 0.75])
        r = l + rng.choice([0, 1, 0.25, 0.5, 1.0]) * w if rng.random() < 0.9 else pr
        b = t + rng.choice([0, 1, 0.25, 0.5, 1.0]) * h if rng.random() < 0.9 else pb
    else:
        l = rng.choice([0, 10, 50, 100, 200, 400]) + rng.choice([0, 0, 0.5, -0.25])
        t = rng.choice([0, 10, 50, 100, 300]) + rng.choice([0, 0, 0.75])
        r = l + rng.choice([0, 1, 20, 100, 300, 800])
        b = t + rng.choice([0, 1, 20, 100, 300, 600])
    v = [f2b(l), f2b(t), f2b(r), f2b(b)]
    if rng.random() < 0.04:
        v[rng.randrange(4)] = NAN
    if rng.random() < 0.05:
        v = [f2b(-0.0) if x == f2b(0.0) else x for x in v]
    return v


def floats(bits):
    return [struct.unpack('<f', struct.pack('<I', w))[0] for w in bits]


def tree(rng):
    nw = rng.randrange(1, 9)
    widgets = []
    for i in range(nw):
        parent = None if i == 0 else rng.randrange(i)
        flags = 0x80007 | (0x200000 if rng.random() < 0.2 else 0)
        top = rng.choice([None, 0, i, parent if parent is not None else 0])
        widgets.append(dict(parent_08=parent, flags_24=flags, vt_7c_true=rng.random() < 0.08, top_14=top))
    ves = []

    def element(owner, depth, parent):
        k = len(ves); group = depth < 3 and rng.random() < 0.4
        outer = rect(rng, parent); client = rect(rng, floats(outer)) if rng.random() < 0.7 else list(outer)
        ves.append(dict(group_d0=(rng.random() < 0.5) if group else None, flags_88=0x1085 if rng.random() < 0.85 else 0x1081,
                        outer_20=outer, client_54=client, owner_0c=owner if rng.random() < 0.95 else None, children=[]))
        if group:
            ves[k]['children'] = [element(owner, depth + 1, floats(client if rng.random() < 0.5 else outer)) for _ in range(rng.randrange(0, 4))]
        return k

    def widget_group(w, parent):
        k = len(ves)
        outer = rect(rng, parent) if parent is not None else [f2b(0.0), f2b(0.0), f2b(1024.0), f2b(768.0)]
        client = rect(rng, floats(outer)) if rng.random() < 0.5 else list(outer)
        ves.append(dict(group_d0=True, flags_88=0x1087 if rng.random() < 0.9 else 0x1083, outer_20=outer, client_54=client,
                        owner_0c=w, children=[]))
        kids = [element(w, 1, floats(outer)) for _ in range(rng.randrange(0, 3))]
        for c in range(nw):
            if widgets[c]['parent_08'] == w:
                kids.append(widget_group(c, floats(outer)))
        ves[k]['children'] = kids
        return k

    root = widget_group(0, None)
    return widgets, ves, root


def cases():
    rng = random.Random(0x5C3BC0)
    out = []
    for n in range(260):
        widgets, ves, root = tree(rng)
        queries = []
        for _ in range(12):
            v = rng.randrange(len(ves)) if rng.random() < 0.3 else root
            x = rng.choice([-5, 0, 10, 50, 99, 100, 101, 150, 200, 300, 450, 700, 900])
            y = rng.choice([-5, 0, 10, 50, 100, 120, 300, 301, 400, 700])
            u = rng.random()
            if u < 0.6:
                # a pixel inside (or on an edge of) one of the tree's rects
                r = rng.choice(ves)[rng.choice(['outer_20', 'client_54'])]
                l, t, rr, b = (struct.unpack('<f', struct.pack('<I', w))[0] for w in r)
                if all(z == z for z in (l, t, rr, b)) and rr >= l and b >= t:
                    x = rng.choice([l, rr, int(l) + rng.randrange(0, int(rr - l) + 1)])
                    y = rng.choice([t, b, int(t) + rng.randrange(0, int(b - t) + 1)])
            queries.append(dict(ve=v, x=f2b(float(x)), y=f2b(float(y)), flag=rng.choice([0, 1, 1, 1])))
        admits = []
        for _ in range(6):
            admits.append(dict(widget=rng.randrange(len(widgets)), focus_8c=rng.choice([None] + list(range(len(widgets)))),
                               root_74=rng.randrange(len(widgets))))
        out.append(dict(widgets=widgets, ves=ves, root=root, queries=queries, admits=admits))
    return out


def main():
    rows = []; pins = {}; inputs = cases()
    for image in ['original', 'v14']:
        for precision in PRECISIONS:
            m = Machine(image, precision)
            for c in inputs:
                out = run(m, c); pins.update(m.actual); rows.append(dict(image=image, precision=precision, input=c, output=out))
                m.heap = 0x20000000  # every case lays out its own objects
    a = [{k: v for k, v in r.items() if k != 'image'} for r in rows if r['image'] == 'original']
    b = [{k: v for k, v in r.items() if k != 'image'} for r in rows if r['image'] == 'v14']
    assert a == b
    for va, raw in pins.items():
        assert bytes.fromhex(raw) == original.range(va, len(bytes.fromhex(raw))) == modified.range(va, len(bytes.fromhex(raw))), hex(va)
    independent = all(len({json.dumps(r['output'], sort_keys=True) for r in a if r['input'] is c}) == 1 for c in inputs)
    doc = dict(schema=1, scope=__doc__, producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               native_runs=len(rows), precision_independent=independent,
               rows=[dict(input=r['input'], output=r['output']) for r in rows if r['image'] == 'original' and r['precision'] == 'Double53'],
               instructions=[dict(va=hex(x), bytes=y) for x, y in sorted(pins.items())])
    path = HERE / 'native-press-pick.json'
    data = (json.dumps(doc, indent=1) + '\n').encode()
    if '--write' in sys.argv: path.write_bytes(data)
    else: assert path.read_bytes() == data
    picks = sum(len(r['output']['picks']) for r in doc['rows']); hits = sum(1 for r in doc['rows'] for p in r['output']['picks'] if p is not None)
    print(json.dumps(dict(rows=len(rows), inputs=len(inputs), picks=picks, hits=hits, pins=len(pins), independent=independent,
                          sha256=hashlib.sha256(data).hexdigest())))


if __name__ == '__main__':
    main()
