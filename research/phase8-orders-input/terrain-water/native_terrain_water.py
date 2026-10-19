"""Producer of native-terrain-water.json: block water geometry on real maps.

For two original maps with water, Campaign/tutorial02 (8935 water tiles, a saved
GMap+5C triangulation of 457 cells, so cached and regular gtiles mix) and
(2v2)Berlin (lake and river, empty saved cache, so every water gtile takes
0x7376B0's regular quad), this runs the ORIGINAL code in both pinned images
at PC24/PC53/PC64. (3v3)Firefight has a saved cache but no class-2 tile, so
738280 never reads it.

  721330 + 723E00   TerrainMap over the map's full block grid (as in
                    ../terrain-map), with laboratory heights.
  720AD0            mesh load of every block that touches water.
  7208D0 720A93     737470 + 738280 (body grouping by the water tile's
                    VariationData, 738200), 7377F0 (55B9E0 cache cells or
                    7376B0 quads; 737600/7372A0 vertex hash with the original
                    7BA3C0/606750 table), 736AB0 (z = 550210 - 0.5 and the block
                    +80/+84 widening).
  71D840            end-to-end rays at water and shore points of loaded blocks.

The GMap tiles are the map's own: each tile's palette variation is resolved
with the original carried-variation fallback (as research/phase6-spatial/
original_map_terrain.py). There is one laboratory VariationData per distinct
variation id, and its TerrainData+1C class comes from the variation metadata.
The saved cache arrays come from the map's TERRAIN suffix (the phase6 reader).
Seams:
  - everything in ../terrain-map/native_terrain_map.py;
  - 55B9E0 (Python lookup over the parsed entries; the native lookup itself is
    proven by research/phase6-water-geometry);
  - 87FB40 (NiTriShape over 8D0550's data layout: +C u16 vertex count,
    +20/+24/+28/+2C arrays, +3C u16 triangle count, +40 = 3 x that, +44 u16
    triangles);
  - 7377F0's Ni property tail 737B9F -> 738112 (static scan: stores only to
    the new Ni objects and water object +C/+8) and 738119 -> 73819B;
  - 550210 (laboratory base heights, recorded) and 4F0E50 (laboratory ground);
  - 4B7490 (no colour table; 736AB0 then takes the TerrainData+78 colour path).

  python native_terrain_water.py --write | (replay) | --dry
"""
from pathlib import Path
import hashlib, json, math, random, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
GAME = ROOT.parent
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
sys.path.insert(0, str(ROOT / 'research/phase8-orders-input/terrain-map'))
sys.path.insert(0, str(ROOT / 'research/phase6-water-geometry'))
sys.path.insert(0, str(ROOT / 'research/phase6-mapheader'))
import native_machine as nm  # noqa: E402
from native_machine import f32, original, modified  # noqa: E402
import native_terrain_map as tm  # noqa: E402
from unicorn.x86_const import UC_X86_REG_EIP  # noqa: E402
import native_water_geometry as wg  # noqa: E402
from inspect_header import Reader, read_chunks  # noqa: E402

OUT = HERE / 'native-terrain-water.json'
SOURCE = GAME / 'reconstruction/assets/original/Data'
MAPS = ['Maps/Campaign/tutorial02.RMP', 'Maps/(2v2)Berlin.RMP']
RANGES = {'738280': (0x738280, 0x7385b6), '738200': (0x738200, 0x738272), '7377F0': (0x7377f0, 0x7381f9),
          '7376B0': (0x7376b0, 0x7377e2), '737600': (0x737600, 0x7376ac), '7372A0': (0x7372a0, 0x73730b),
          '7371B0': (0x7371b0, 0x73724c), '736AB0': (0x736ab0, 0x736c5e), '7BA3C0': (0x7ba3c0, 0x7ba42b),
          '736C60': (0x736c60, 0x736d4f), '736E70': (0x736e70, 0x736e7a), '71B9B0': (0x71b9b0, 0x71bc0d)}


def flt(bits):
    return struct.unpack('<f', struct.pack('<I', bits & 0xffffffff))[0]


def map_tiles(path):
    """(width, height, extents, [(variation id, class)] per tile, cache)."""
    meta = json.loads((ROOT / 'research/phase6-spatial/native-real-map-terrain.json').read_text())['maps'][path]
    types = json.loads((ROOT / 'research/engine-baseline/terrain-variation-metadata.json').read_text())['variations']
    render = json.loads((ROOT / 'research/phase6-terrain/terrain-render-metadata.json').read_text())
    raw = read_chunks(SOURCE / path)['TERRAIN'][0]
    assert hashlib.sha256(raw).hexdigest() == meta['terrain_sha256']
    r = Reader(raw)
    version, resolution, biome = r.u32(), r.u32(), r.string()
    palette = [r.string() for _ in range(r.u32())]
    count, stride = r.u32(), r.u32()
    records = []
    for _ in range(count):
        b = r.take(stride)
        records.append(struct.unpack('<BBHBB', b)[0] if version == 6 else struct.unpack('<HHHBB', b)[0])
    assert r.at == meta['suffix_offset']

    def resolve(key):
        if key not in render['variants']:
            return None
        return (key, types[render['variants'][key]['canonical']]['native_type'])
    defs = [resolve(k) for k in palette]
    previous = resolve(render['biome_defaults'][biome]['variation_id'])
    tiles = []
    for index in records:
        d = defs[index] or previous
        tiles.append(d)
        previous = d
    layout = meta['layout']
    start, vertices, indices, entries = wg.parse(raw[meta['suffix_offset']:])
    cache = dict(vertices=list(struct.unpack('<%dI' % (len(vertices) // 4), vertices)),
                 indices=list(struct.unpack('<%dI' % (len(indices) // 4), indices)), entries=entries)
    return layout, tiles, cache


class WaterLab(tm.Lab):
    def __init__(self, precision, spec, tiles, cache):
        super().__init__(precision, spec)
        m, S = self.m, self.seams
        del m.seams[0x7377F0]               # the base seams stub 7377F0 out; run it here
        gmap = m.r(m.r(tm.GWORLD) + 0x30)
        w, h = spec['tiles_size']
        m.w(gmap + 0xc, f32(float(spec['gmap_extent'])))
        ids = sorted(set(t[0] for t in tiles))
        self.variation_index = {k: i for i, k in enumerate(ids)}
        variations = {}
        for k in ids:
            data = m.alloc(0x100)
            m.w(data + 0x1c, dict(tiles)[k] & 0xffffffff)
            var = m.alloc(0x40)
            m.w(var + 0x1c, data)
            variations[k] = var
        arr = m.alloc(0x30 * w * h)
        for i, (k, _) in enumerate(tiles):
            m.w(arr + 0x30 * i, variations[k])
        m.w(gmap + 0x1c, w)
        m.w(gmap + 0x20, h)
        m.w(gmap + 0x28, arr)
        # GMap+5C saved cache: +6C index array, +78 vertex pairs, 55B9E0 lookup.
        m.w(gmap + 0x6c, m.array(cache['indices']))
        m.w(gmap + 0x78, m.array(cache['vertices']))
        self.cells = {(e[0], e[1]): e for e in cache['entries']}   # later duplicates win (head insertion)

        def lookup(m):
            xy, out = m.arg(1), m.arg(2)
            e = self.cells.get((m.r(xy), m.r(xy + 4)))
            S.count('55B9E0 cache lookup')
            if e is None:
                m.ret(0, 8)
            else:
                for i, v in enumerate(e):
                    m.w(out + 4 * i, v)
                m.ret(1, 8)
        m.seam(0x55B9E0, lookup)
        self.base = {}

        def base_height(m):
            x, y = m.arg(1), m.arg(2)
            v = base_value(x, y)
            self.base[(x, y)] = v
            S.count('550210 base height')
            m.ret_st0_f32(v, 8)
        m.seam(0x550210, base_height)
        self.ground = {}

        def ground(m):
            xy = m.arg(1)
            x, y, flags = m.r(xy), m.r(xy + 4), m.arg(2)
            v = ground_value(x, y, flags)
            self.ground[(x, y, flags)] = v
            S.count('4F0E50 ground')
            m.ret_st0_f32(v, 8)
        m.seam(0x4F0E50, ground)
        m.seam(0x4B7490, lambda m: (S.count('4B7490 colour table'), m.ret(0, 4)))
        m.seam(0x4A6AD0, lambda m: (S.count('4A6AD0'), m.ret(0, 0)))
        av = m.r(tm.AVARS)
        m.w(av + 0x14, m.array([f32(1.0)]))

        def geometry(m):                    # 87FB40 thiscall(9 args), ret 0x24
            S.count('87FB40 NiTriShape')
            count, verts, normals, colours, uvs, _, _, tris, idx = [m.arg(i) for i in range(1, 10)]
            ni = S.fake(0x100, refcount=0x100)
            data = m.alloc(0x60)
            m.w(ni + 0xcc, data)
            m.u.mem_write(data + 0xc, struct.pack('<H', count & 0xffff))
            for off, ptr in ((0x20, verts), (0x24, normals), (0x28, colours), (0x2c, uvs), (0x44, idx)):
                m.w(data + off, ptr)
            m.u.mem_write(data + 0x3c, struct.pack('<H', tris & 0xffff))
            m.w(data + 0x40, 3 * (tris & 0xffff))
            m.ret(ni, 0x24)
        m.seam(0x87FB40, geometry)
        m.seam(0x737B9F, lambda m: m.u.reg_write(UC_X86_REG_EIP, 0x738112))
        m.seam(0x738119, lambda m: m.u.reg_write(UC_X86_REG_EIP, 0x73819B))
        m.seam(0x720C79, lambda m: m.u.reg_write(UC_X86_REG_EIP, 0x72100E))

    def load(self, cell):
        m = self.m
        c = self.cell(cell)
        assert m.r(c + 0x44) == 0
        m.call(0x720AD0, ecx=c, pop=0)

    def water(self, cell):
        m = self.m
        c = self.cell(cell)
        m.wb(c + 0x14f, 0)
        m.call(0x737470, ecx=c + 0x8c, pop=0)
        m.call(0x738280, ecx=c + 0x8c, pop=0, count=50_000_000)

    def water_state(self, cell):
        m = self.m
        c = self.cell(cell)
        objects = []
        data, count = m.r(c + 0x90), m.r(c + 0x94)
        for k in range(count):
            obj = m.r(data + 4 * k)
            geo = m.r(obj + 4)
            d = m.r(geo + 0xcc)
            n = struct.unpack('<H', bytes(m.u.mem_read(d + 0xc, 2)))[0]
            tri = struct.unpack('<H', bytes(m.u.mem_read(d + 0x3c, 2)))[0]
            nv = max(n, 0)
            verts = list(struct.unpack('<%dI' % (3 * nv), bytes(m.u.mem_read(m.r(d + 0x20), 12 * nv)))) if nv else []
            tris = list(struct.unpack('<%dH' % (3 * tri), bytes(m.u.mem_read(m.r(d + 0x44), 6 * tri)))) if tri else []
            objects.append(dict(block=m.r(obj) == c, vertex_count=n, triangle_count=tri, index_count=m.r(d + 0x40),
                                vertices=verts, triangles=tris))
        return dict(water=m.rb(c + 0x9c), min=m.r(c + 0x80), max=m.r(c + 0x84), objects=objects)


def ground_value(x_bits, y_bits, flags):
    x, y = flt(x_bits), flt(y_bits)
    v = 0.75 + 0.5 * math.sin(x / 9.0) * math.cos(y / 11.0)
    return f32(v + (0.25 if flags == 0x123 else 0.0))


def base_value(x_bits, y_bits):
    x, y = flt(x_bits), flt(y_bits)
    return f32(0.375 + 0.25 * math.sin(x / 13.0 + y / 17.0))


def heights(layout, tiles, step):
    """Laboratory VWorld+BC samples: low under water tiles, higher on land."""
    w, h = layout['width'], layout['height']
    cols, rows = layout['extent_i32'][0] // 2 + 1, layout['extent_i32'][1] // 2 + 1
    out = []
    for sy in range(rows):
        for sx in range(cols):
            tx, ty = min(sx, w - 1), min(sy, h - 1)
            water = tiles[ty * w + tx][1] == 2
            base = -1.25 if water else 1.0
            out.append(f32(base + 0.125 * (((sx * 7 + sy * 13) % 9) - 4) / 4))
    return dict(columns=cols, rows=rows, step=f32(step), inverse=f32(1 / step), heights=out, limits=[f32(-7.5), f32(9.25)])


def spec_for(path):
    layout, tiles, cache = map_tiles(path)
    e = layout['extent_i32']
    spec = dict(extents=[f32(float(e[0])), f32(float(e[1]))], gtile=layout['cell_size_bits'], block=8, quad=2,
                heightfield=heights(layout, tiles, 2.0), tiles_size=[layout['width'], layout['height']], gmap_extent=e[0])
    return spec, tiles, cache


def water_blocks(spec, tiles, cache, limit):
    """Blocks touching water, alternating ones with and without saved cells."""
    w, h = spec['tiles_size']
    g = 4
    bx, by = w // g, h // g
    cached = set((e[0], e[1]) for e in cache['entries'])
    with_cells, without = [], []
    for y in range(by):
        for x in range(bx):
            hit = any(0 <= gx < w and 0 <= gy < h and tiles[gy * w + gx][1] == 2
                      for gy in range(y * g - 1, y * g + g + 1) for gx in range(x * g - 1, x * g + g + 1))
            if hit:
                own = any((gx, gy) in cached for gy in range(y * g, y * g + g) for gx in range(x * g, x * g + g))
                (with_cells if own else without).append(y * bx + x)
    rng = random.Random(len(tiles))
    rng.shuffle(with_cells)
    rng.shuffle(without)
    out = []
    while len(out) < limit and (with_cells or without):
        for src in (with_cells, without):
            if src and len(out) < limit:
                out.append(src.pop())
    return sorted(out)


def rays(rng, lab, cells, n):
    out = []
    pool = []
    for cell in cells:
        st = lab.water_state(cell)
        for obj in st['objects'][:1]:
            v, t = obj['vertices'], obj['triangles']
            for k in range(0, min(len(t), 30), 3):
                pool.append([v[3 * t[k + j]:3 * t[k + j] + 3] for j in range(3)])
    rng.shuffle(pool)
    for tri in pool[:n]:
        wts = [rng.random() for _ in range(3)]
        s = sum(wts)
        target = [sum(flt(tri[j][i]) * wts[j] for j in range(3)) / s for i in range(3)]
        theta = rng.uniform(0, 2 * math.pi)
        slope = rng.choice([0.4, 1.0, 3.0, 8.0])
        d = [math.cos(theta), math.sin(theta), -slope]
        back = rng.uniform(2.0, 8.0) / slope
        eye = [target[i] - back * d[i] for i in range(3)]
        out.append(dict(eye=[f32(x) for x in eye], direction=[f32(x) for x in d], secondary=rng.random() < 0.6, margin=0))
    return out


def run_case(precision, path, spec, tiles, cache, limit):
    lab = WaterLab(precision, spec, tiles, cache)
    built = lab.construct()
    assert not built['asserts']
    cells = water_blocks(spec, tiles, cache, limit)
    row = dict(precision=precision, map=path, cells=cells, loads=[], waters=[])
    for c in cells:
        lab.load(c)
    for c in cells:
        lab.water(c)
        row['waters'].append(lab.water_state(c))
    mf = tm.map_fields(lab.m, lab.map)
    row['map'] = dict(min=mf['min'], max=mf['max'])
    rng = random.Random(len(cells) * 31 + len(path))
    row['rays'] = []
    for r in rays(rng, lab, cells, 60):
        r['native'] = lab.ray(r['eye'], r['direction'], r['secondary'], r['margin'])
        row['rays'].append(r)
    row['ground'] = [[x, y, fl, v] for (x, y, fl), v in sorted(lab.ground.items())]
    row['base'] = [[x, y, v] for (x, y), v in sorted(lab.base.items())]
    lab.m.verify_v14()
    return row, lab.m.visited, lab.seams.calls


def main():
    specs, data = {}, {}
    for path in MAPS:
        spec, tiles, cache = spec_for(path)
        specs[path] = dict(spec, tiles=[[t[0], t[1]] for t in tiles], cache=cache)
        data[path] = (spec, tiles, cache)
    rows, visited, seams = [], {}, {}
    limit = int(sys.argv[sys.argv.index('--limit') + 1]) if '--limit' in sys.argv else 48
    for precision in nm.PRECISIONS:
        for path in MAPS:
            spec, tiles, cache = data[path]
            row, v, calls = run_case(precision, path, spec, tiles, cache, limit)
            visited.update(v)
            for k, c in calls.items():
                seams[k] = seams.get(k, 0) + c
            rows.append(row)
            print(precision, path, len(row['cells']), sum(len(w['objects']) for w in row['waters']),
                  sum(r['native']['result'] for r in row['rays']), flush=True)
    from capstone import Cs, CS_ARCH_X86, CS_MODE_32
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    coverage = {}
    for fn, (lo, hi) in RANGES.items():
        listed = [i.address for i in md.disasm(original.range(lo, hi - lo), lo) if i.mnemonic != 'int3']
        missing = [hex(a) for a in listed if a not in visited]
        coverage[fn] = dict(instructions=len(listed), executed=len(listed) - len(missing), missing=missing)
    for a, b in visited.items():
        assert b == original.range(a, len(b)) == modified.range(a, len(b)), hex(a)
    doc = dict(nm.header(__file__, 'unicorn original+v14 images, PC24/PC53/PC64'), scope=__doc__, specs=specs, rows=rows,
               coverage=coverage, seam_calls=dict(sorted(seams.items())), executed_pins=len(visited),
               harness_sha256=nm.sha(Path(nm.__file__).read_bytes()), base_producer_sha256=nm.sha(Path(tm.__file__).read_bytes()))
    data = (json.dumps(doc, separators=(',', ':')) + '\n').encode()
    if '--dry' in sys.argv:
        for k, v in coverage.items():
            print(k, v['executed'], '/', v['instructions'], ' '.join(v['missing'][:16]))
    elif '--write' in sys.argv:
        with OUT.open('xb') as f:
            f.write(data)
    else:
        assert OUT.read_bytes() == data, 'native-terrain-water.json differs from this producer'
    print(json.dumps(dict(rows=len(rows), rays=sum(len(r['rays']) for r in rows), executed_pins=len(visited),
                          bytes=len(data), sha256=hashlib.sha256(data).hexdigest())))


if __name__ == '__main__':
    main()
