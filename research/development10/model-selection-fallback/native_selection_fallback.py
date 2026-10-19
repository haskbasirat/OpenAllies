"""Original/v14 5A94F0 KKC_NiAVObjResource::ComputeSelectionCuboid, no-SelectionBox
branch, over the actual Berlin models that reach it, plus scalar edge cases.

Part 1 (root world bound): for each model's paired raw graph projection
(assets-cache/model-native-graphs, the representation the model backend is
checked against), world transforms execute 45CAB0, each NiTriShape/NiTriStrips
data sphere executes 89FBF0, and parents merge child bounds with 89FAD0. The
traversal (preorder compose, children in slot order, a zero-radius child skipped,
the first contributing child copied) is explicit transport, the same rule the
port's 86E8D0 update uses; it is not a whole NiNode::UpdateWorldBound execution.
Part 2 (5A94F0): executes 5A94F0 from its entry. Named seams: 42C070 Bone
lookup (returns the selection_box entry), the root's vf40 GetObjectByName
(returns NULL, or a non-NiGeometry object whose real RTTI walk runs), the
root's vf48 Update(0,0) (records arguments), 404ED0 (records diagnostics).
"""
from pathlib import Path
import hashlib, json, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h  # noqa: E402

GRAPHS = ROOT / 'assets-cache/model-native-graphs'
MODELS = json.loads((HERE / 'berlin-no-selection-models.json').read_text())
f = lambda v: struct.unpack('<I', struct.pack('<f', v))[0]


def wide(read, a):
    out = []
    while True:
        v = struct.unpack('<H', read(a + 2 * len(out), 2))[0]
        if v == 0:
            return ''.join(map(chr, out))
        out.append(v)


class Machine(h.Machine):
    def __init__(self, pc, image):
        super().__init__(pc)
        if image is not h.P:
            original_end = (h.P.image_size + 4095) & ~4095
            image_end = (image.image_size + 4095) & ~4095
            if image_end > original_end:
                self.u.mem_map(image.base + original_end, image_end - original_end)
            self.u.mem_write(image.base, image.b[:4096])
            for vs, r, raw, off in image.sections:
                if raw:
                    self.u.mem_write(image.base + r, image.b[off:off + min(raw, max(vs, raw))])
        self.image = image
        self.stub_seq = h.STUBS + 0x400
        self.events = []

    def stub(self, handler):
        a = self.stub_seq
        self.stub_seq += 16
        self.seams[a] = handler
        return a

    def words(self, a, n):
        return self.r32(a, n)

    def put(self, a, ws):
        self.w32(a, *ws)

    def text(self, value):
        units = list(map(ord, value))
        data = self.alloc(16 + 2 * (len(units) + 1))
        self.w32(data, 0, len(units), len(units), 1)
        self.u.mem_write(data + 16, struct.pack('<' + 'H' * (len(units) + 1), *units, 0))
        return data + 16

    # ---- part 1 -----------------------------------------------------------
    def compose(self, parent, local):
        p, l, out = self.alloc(52), self.alloc(52), self.alloc(52)
        self.put(p, parent)
        self.put(l, local)
        self.call(0x45cab0, p, [out, l])
        return self.words(out, 13)

    def transform(self, sphere, world):
        src, t, out = self.alloc(16), self.alloc(52), self.alloc(16)
        self.put(src, sphere)
        self.put(t, world)
        self.call(0x89fbf0, out, [src, t])
        return self.words(out, 4)

    def merge(self, current, other):
        a, b = self.alloc(16), self.alloc(16)
        self.put(a, current)
        self.put(b, other)
        self.call(0x89fad0, a, [b])
        return self.words(a, 4)

    def root_bound(self, graph):
        nodes = graph['nodes']
        order, work = [], [graph['roots'][0]]
        while work:
            i = work.pop()
            order.append(i)
            work.extend(c for c in reversed(nodes[i]['children']) if c is not None)
        world, sphere = {}, {}
        for i in order:
            n = nodes[i]
            world[i] = n['local'] if n['parent'] is None else self.compose(world[n['parent']], n['local'])
            if n['kind'] in ('NiTriShape', 'NiTriStrips') and n['sphere'] is not None:
                sphere[i] = self.transform(n['sphere'], world[i])
        for i in reversed(order):
            n = nodes[i]
            if n['kind'] in ('NiTriShape', 'NiTriStrips'):
                continue
            current = None
            for c in n['children']:
                if c is None or c not in sphere:
                    continue
                s = sphere[c]
                if s[3] & 0x7fffffff == 0:
                    continue
                current = list(s) if current is None else self.merge(current, s)
            if current is not None:
                sphere[i] = current
        return sphere.get(order[0])

    # ---- part 2 -----------------------------------------------------------
    def selection(self, sphere, flags, named_node):
        resource = self.alloc(0xa0)
        self.w32(resource + 8, flags)
        self.w32(resource + 0x38, self.text('Doodads/Fixture.nif'))
        root = self.alloc(0x100)
        rvt = self.alloc(0x60)
        self.w32(root, rvt)
        self.put(root + 0x28, sphere)
        named = 0
        if named_node:
            named = self.alloc(0x40)
            nvt = self.alloc(0x10)
            rtti = self.alloc(8)
            self.w32(rtti, 0, 0xb43e10)  # NiNode-like class: chain never reaches B43CF0
            self.w32(nvt + 4, self.stub(lambda _: self.ret(0, rtti)))
            self.w32(named, nvt)

        def get_object(_):
            self.events.append(['vf40_GetObjectByName', wide(lambda a, n: bytes(self.u.mem_read(a, n)), self.arg())])
            self.ret(4, named)
        self.w32(rvt + 0x40, self.stub(get_object))
        self.w32(rvt + 0x48, self.stub(lambda _: (self.events.append(['vf48_Update', self.arg(0), self.arg(1)]), self.ret(8))))
        registry = self.alloc(0x600)
        holder = self.alloc(0x40)
        entry = self.alloc(0x40)
        self.w32(entry + 0x1c, self.text('SelectionBox'))
        self.w32(holder + 0x580, registry)
        self.w32(0xb36530, holder)

        def bone_lookup(_):
            assert self.ecx() == registry
            self.events.append(['42C070', wide(lambda a, n: bytes(self.u.mem_read(a, n)), self.arg())])
            self.ret(4, entry)
        self.seams[0x42c070] = bone_lookup

        def diag(_):
            channel, file, line, function, fmt, name = self.r32(self.esp() + 4, 6)
            rd = lambda a, n: bytes(self.u.mem_read(a, n))
            self.events.append(['404ED0', hex(channel), wide(rd, file), line, wide(rd, function), wide(rd, fmt), wide(rd, name)])
            self.ret()
        self.seams[0x404ed0] = diag
        self.call(0x5a94f0, resource, [root])
        return dict(cuboid=self.words(resource + 0x4c, 6), events=self.events)


def edge_cases():
    yield 'origin', [0, 0, 0, f(1)]
    yield 'x outside', [f(2), 0, 0, f(1)]
    yield 'y outside', [0, f(-3), 0, f(1)]
    yield 'z only outside', [0, 0, f(9), f(1)]
    yield 'x touching', [f(0.4), 0, 0, f(1)]
    yield 'zero radius at origin', [0, 0, 0, 0]
    yield 'zero radius off axis', [f(1), f(1), 0, 0]
    yield 'negative zero center', [0x80000000, 0x80000000, 0, f(2)]
    yield 'tiny', [0x00000001, 0x80000001, 0x3f800001, 0x00800000]
    yield 'large', [f(1e30), f(-1e30), f(3e38), f(3e38)]
    yield 'rounding', [0x3f800001, 0xbf7fffff, 0x3dcccccd, 0x3f7fffff]


def main():
    rows = []
    models = []
    for source in MODELS:
        index = json.loads((GRAPHS / 'index.json').read_text())
        entry = next((v for v in index['graphs'].values() if v['source'].lower() == source.lower()), None)
        if entry is None or not (GRAPHS / entry['artifact']).exists():
            models.append(dict(source=source, excluded='no paired graph projection'))
            continue
        raw = (GRAPHS / entry['artifact']).read_bytes()
        assert hashlib.sha256(raw).hexdigest() == entry['sha256']
        graph = json.loads(raw)
        kinds = sorted({n['kind'] for n in graph['nodes']})
        #Same static domain the port's 86E8D0 update admits (its supported()):
        #skinned graphs and other node kinds (NiBillboardNode, particles) refuse.
        if any(k not in ('NiNode', 'NiTriShape', 'NiTriStrips') for k in kinds) or any(n['skin'] for n in graph['nodes']) or len(graph['roots']) != 1:
            models.append(dict(source=source, excluded='outside the port update domain (skin or node kind)', kinds=kinds))
            continue
        assert not any(n['name'] == 'SelectionBox' for n in graph['nodes'])
        models.append(dict(source=source, source_sha256=entry['source_sha256'], projection_sha256=entry['sha256'], kinds=kinds, graph=graph))
    cases = []
    for m in models:
        if 'graph' in m:
            cases.append(('model', m['source'], m))
    for name, sphere in edge_cases():
        cases.append(('edge', name, sphere))
    for kind, name, data in cases:
        for flags, named in ((0x410, False), (0xc10, False), (0xc10, True)) if kind == 'edge' else ((0x1d10, False), (0x410, False)):
            for pc in h.PRECISION:
                outs = []
                for image in (h.P, h.V):
                    m = Machine(pc, image)
                    sphere = m.root_bound(data['graph']) if kind == 'model' else data
                    if sphere is None:
                        out = dict(root_sphere=None)
                    else:
                        out = dict(root_sphere=sphere, **m.selection(sphere, flags, named))
                    outs.append(out)
                assert outs[0] == outs[1], (name, pc)
                rows.append(dict(kind=kind, name=name, flags=flags, named_non_geometry=named, precision=pc, output=outs[0]))
    static = []
    for a, n in ((0x5a94f0, 0x342), (0x45cab0, 0xb0), (0x89fbf0, 0x7a), (0x89fad0, 0x120), (0xa4635c, 4), (0x9f32dc, 4), (0xb32d94, 4)):
        raw = h.P.range(a, n)
        assert raw == h.V.range(a, n)
        static.append(dict(address=hex(a), size=n, sha256=hashlib.sha256(raw).hexdigest()))
    executed = [dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())]
    out = dict(schema=1, scope=__doc__, original_sha256=h.ORIGINAL_SHA, v14_sha256=h.V14_SHA,
               models=[{k: v for k, v in m.items() if k != 'graph'} for m in models], rows=rows,
               static_pins=static, executed_address_byte_pins=executed)
    raw = (json.dumps(out, indent=1) + '\n').encode()
    path = HERE / 'native-selection-fallback.json'
    if '--write' in sys.argv:
        path.write_bytes(raw)
    else:
        assert path.read_bytes() == raw, 'fixture differs from fresh native replay'
    print(json.dumps(dict(models=sum('graph' in m for m in models), excluded=sum('excluded' in m for m in models), rows=len(rows),
                          machines=2 * len(rows), executed_pins=len(executed), sha256=hashlib.sha256(raw).hexdigest())))


if __name__ == '__main__':
    main()
