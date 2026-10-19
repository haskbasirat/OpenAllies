"""Whole 0x71D840 over an explicit, generalized laboratory TerrainMap.

Extends research/skirmish-20261017-terrain-ray/probe_native_ray.py (whose
16x16 lab only ever reached the first walker block): any block grid, quad
count/size, per-cell mesh presence, per-cell water byte with a water
NiTriShape-shaped child (0x736E70 -> 0x736C60), loaded byte and margin.
Every word written to emulated memory is returned in `terrain` so the Rust
replay reads exactly the same words. Reads of the TerrainMap, the cell array,
cells and water objects are recorded by offset. The cell array sits alone at
the start of its own mapped page (CELLS_PAGE, unmapped below), so a negative
cell index faults instead of reading foreign memory: such a row is outside
the defined domain. No seam is installed: all reached bodies execute.
"""
from pathlib import Path
import struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h  # noqa: E402
from unicorn import UC_HOOK_MEM_WRITE, UC_HOOK_MEM_READ  # noqa: E402

UP, XN, YN = [0, 0, h.f2b(1)], [h.f2b(1), 0, 0], [0, h.f2b(1), 0]
READS = {}
CELLS_PAGE = 0x3100000


def f32(x):
    return h.f2b(x)


def terrain_words(spec):
    """Explicit TerrainMap/cell words from a case spec (heights in world z)."""
    bx, by = spec['blocks']
    q, qs = spec['quads'], spec['quad_size']
    gx = bx * q + 1
    heights = spec['heights']
    cells = []
    for y in range(by):
        for x in range(bx):
            i = y * bx + x
            vertices, zs = [], []
            for yy in range(q + 1):
                for xx in range(q + 1):
                    z = heights[(y * q + yy) * gx + x * q + xx]
                    zs.append(z)
                    vertices.append([f32((x * q + xx) * qs), f32((y * q + yy) * qs), f32(z)])
            water = spec.get('water', {}).get(str(i))
            water_mesh = None
            if water is not None:
                x0, y0, size = x * q * qs, y * q * qs, q * qs
                lv = water['levels']
                water_mesh = dict(count=water.get('count', 6), triangles=water.get('triangles', [[0, 1, 2], [1, 3, 2]]),
                                  vertices=[[f32(x0 + dx * size), f32(y0 + dy * size), f32(lv[dy * 2 + dx])] for dy in range(2) for dx in range(2)])
            cells.append(dict(index=[x, y], min=f32(min(zs)), max=f32(max(zs)), stride=q + 1, quads=q, quad_size=qs,
                              mesh=vertices if spec['mesh'][i] else None,
                              water=1 if water is not None and water.get('enabled', True) else 0,
                              water_mesh=water_mesh))
    return dict(up=UP, x_normal=XN, y_normal=YN, size=spec['size'], blocks=[bx, by], block_size=q * qs,
                min=f32(min(heights)), max=f32(max(heights)), loaded=int(spec['loaded']), cells=cells)


def run(terrain, eye, direction, pc='Double53', secondary=True, border=0):
    m = h.Machine(pc)
    t = m.alloc(0x158)
    bx, by = terrain['blocks']
    m.w32(t + 4, *terrain['up'])
    m.w32(t + 16, *terrain['x_normal'])
    m.w32(t + 28, *terrain['y_normal'])
    m.w32(t + 0x74, terrain['size'][0], terrain['size'][1], terrain['block_size'], terrain['block_size'], bx, by)
    m.w32(t + 0x8c, terrain['min'], terrain['max'])
    m.w32(t + 0xa0, terrain['block_size'])
    assert 4 * bx * by <= 0x1000
    m.u.mem_map(CELLS_PAGE, 0x1000)
    cells = CELLS_PAGE
    m.w32(t + 0xb8, cells)
    m.w8(t + 0xbc, terrain['loaded'])
    regions = [('terrain', t, 0x158), ('cells', cells, 4 * bx * by)]
    for i, c in enumerate(terrain['cells']):
        cell = m.alloc(0x150)
        regions.append(('cell', cell, 0x150))
        m.w32(cells + 4 * i, cell)
        m.w32(cell + 0x40, t)
        m.w32(cell + 0x80, c['min'], c['max'])
        m.w32(cell + 0xac, *c['index'])
        m.w32(cell + 0xc8, c['stride'], c['quads'], 1, c['quad_size'])
        m.w8(cell + 0x9c, c['water'])
        if c['mesh'] is not None:
            ni, data = m.alloc(0x100), m.alloc(0x40)
            v = m.alloc(12 * len(c['mesh']))
            m.w32(cell + 0x44, ni)
            m.w32(ni + 0xcc, data)
            m.w32(data + 0x20, v)
            m.w32(v, *[w for p in c['mesh'] for w in p])
        if c['water_mesh'] is not None:
            w = c['water_mesh']
            holder, obj, geom, data = m.alloc(4), m.alloc(0x10), m.alloc(0x100), m.alloc(0x50)
            v = m.alloc(12 * len(w['vertices']))
            idx = m.alloc(2 * 3 * len(w['triangles']))
            m.w32(cell + 0x90, holder)
            m.w32(holder, obj)
            m.w32(obj + 4, geom)
            m.w32(geom + 0xcc, data)
            m.w32(data + 0x20, v)
            m.w32(data + 0x40, w['count'])
            m.w32(data + 0x44, idx)
            m.w32(v, *[x for p in w['vertices'] for x in p])
            m.u.mem_write(idx, struct.pack('<' + 'H' * (3 * len(w['triangles'])), *[i for tri in w['triangles'] for i in tri]))
            regions += [('water_holder', holder, 4), ('water_object', obj, 0x10), ('water_geometry', geom, 0x100), ('water_data', data, 0x50)]
    eye_ptr = m.alloc(12)
    m.w32(eye_ptr, *eye)
    out = m.alloc(12)
    m.w32(out, 0xcdcdcdcd, 0xcdcdcdcd, 0xcdcdcdcd)
    sec = m.alloc(12) if secondary else 0
    if secondary:
        m.w32(sec, 0xabababab, 0xabababab, 0xabababab)
    writes = []

    def write(u, access, address, size, value, data):
        if out <= address < out + 12:
            writes.append(['point', address - out, value & 0xffffffff])
        if secondary and sec <= address < sec + 12:
            writes.append(['secondary', address - sec, value & 0xffffffff])

    def read(u, access, address, size, value, data):
        for kind, base, n in regions:
            if base <= address < base + n:
                READS.setdefault(kind, set()).add(address - base)
                return
    m.u.hook_add(UC_HOOK_MEM_WRITE, write)
    m.u.hook_add(UC_HOOK_MEM_READ, read)
    try:
        result = m.call(0x71d840, t, [eye_ptr, *direction, out, sec, border & 0xffffffff], count=20_000_000)
    except Exception as e:  # native fault: outside the defined domain
        return dict(fault=type(e).__name__ + ':' + str(e), eip=hex(m.u.reg_read(h.UC_X86_REG_EIP)), writes=writes)
    return dict(result=result & 255, out=m.r32(out, 3), secondary=m.r32(sec, 3) if secondary else None,
                writes=writes, esp=m.final_esp - h.STACK)
