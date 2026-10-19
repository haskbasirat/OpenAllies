"""Mode-1 circle actor scan of 5901F0: 558720 iterator, 5580F0(x, y, radius)
(552DD0 descriptor, 46D8E0 radius, 552250 position, +38 = 0x4000,
558040(mode 1)) and 558750 next, then 557F90/557FD0 cleanup, on real native
tile lists built by 5536B0. Every query/iterator/exclusion/B4/vf+0 instruction
executes unchanged; only malloc/free are seamed. Members vary the required
0x4000 flag, the vf+10 soft-shape radius, the B4 category, a premarked link,
and include exact-tangency and just-outside positions for the
(soft radius + query radius)^2 >= dx^2 + dy^2 test.
"""
from pathlib import Path
import hashlib, json, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h


def shape(kind, radius, hx=1.25, hy=.75):
    return dict(kind=kind, radius_bits=h.f2b(radius), bounds=[h.f2b(x) for x in [-hx, -hy, hx, hy]])


def next_up(x):
    return struct.unpack('<f', struct.pack('<I', h.f2b(x) + 1))[0]


def run(precision, scenario):
    m = h.Machine(precision)
    m.seams[0x6f7810] = lambda mm: mm.ret(0, mm.alloc(mm.arg()))
    m.seams[0x6f7430] = lambda mm: mm.ret()
    grid = dict(dimensions=[8, 7], coordinate_shift=1, cell_size_bits=h.f2b(2.))
    state = m.alloc(0x100); tiles = m.alloc(56 * 0x30); ids = m.alloc(0x60004)
    world = m.alloc(0x100); points = m.alloc(0x100); point = m.alloc(0x20)
    m.w32(0xb36534, world); m.w32(world + 0x30, state); m.w32(0xb364c0, ids)
    m.w32(0xb39990, points); m.w32(points + 0x64, point); m.w32(point + 4, 0, h.f2b(-1.))
    extent = [h.f2b(17. if scenario & 1 else 16.), h.f2b(15. if scenario & 1 else 14.)]
    m.w32(state + 4, 1, h.f2b(2.), *extent); m.w32(state + 0x1c, 8, 7); m.w32(state + 0x28, tiles)
    # Mask0 descriptor: tile flags never yield a static tile result.
    flags = [0x300 if i % 7 == scenario % 7 else 0x200 for i in range(56)]
    for i, f in enumerate(flags): m.w32(tiles + i * 0x30 + 0x20, f)
    xy = [[5., 5.], [0.5, 0.5], [7., 6.], [6., 6.], [15.5, 13.5], [3., 9.]][scenario % 6]
    radius = [0., .00001, 1., 2., 3.5][scenario % 5]
    softs = [0.5, 1.0, 3.0, 0.25, 2.0, 1.5, 0.75, 1.25, 1.25]
    positions = [[1., 1.], [2., 2.], [3., 3.], [5., 5.], [6.5, 5.], [9., 9.], [13., 11.]]
    # Exact tangency along +x / +y and a just-outside one (inside the map).
    reach = h.b2f(h.f2b(softs[7] + radius))
    positions.append([min(xy[0] + reach, 15.75), xy[1]])
    positions.append([xy[0], min(next_up(xy[1] + h.b2f(h.f2b(softs[8] + radius))), 13.75)])
    members = []; pointers = {}
    for i, position in enumerate(positions):
        key = i + 1; actor = m.alloc(0x200); definition = m.alloc(0x500); vtable = m.alloc(0x180)
        m.u.mem_write(vtable, h.P.range(0xa273f0, 0x180))
        category = not (scenario & 4 and i in [1, 5])
        m.w32(vtable + 0xb4, 0x6a71d0 if category else 0x6a71f0)
        flags_word = (0x20000400 if i % 3 else 0x400) | (0x4000 if (i + scenario) % 4 != 3 else 0)
        facing = [0, h.f2b(-1.)]; reference = ((i + 2) << 16) | key
        blocking = shape(1 if i % 2 else 3, 1.25); soft = shape(1, softs[i], softs[i], softs[i])
        m.w32(actor, vtable, definition, flags_word); m.w32(actor + 0x14, reference)
        m.w32(actor + 0x20, *[h.f2b(x) for x in position], h.f2b(2.), *facing, 0)
        m.w32(definition + 0x1c, flags_word)
        for offset, s in [(0x38, blocking), (0x50, soft)]:
            m.w32(definition + offset, s['kind'], s['radius_bits'], *s['bounds'])
        m.u.mem_write(ids + 4 + key * 2, (i + 2).to_bytes(2, 'little')); m.w32(ids + 0x20004 + key * 4, actor)
        pose_soft = m.alloc(0x18); pose_block = m.alloc(0x18)
        m.w32(pose_soft, definition + 0x50, *facing, *[h.f2b(x) for x in position], 0)
        m.w32(pose_block, definition + 0x38, *facing, *[h.f2b(x) for x in position], 0)
        m.call(0x5536b0, ecx=state, args=(actor, pose_soft, pose_block))
        pointers[key] = actor
        members.append(dict(key=key, reference=reference, actor=dict(definition=key + 10, flags=flags_word,
            position=[h.f2b(x) for x in position], facing=facing), height_bits=h.f2b(2.), facing_z_bits=0,
            cached_tile=[0, 0], blocking=blocking, soft=soft, target=soft, actor_category=category,
            wall_present=False))
    premarked = [3] if scenario & 8 else []
    for key in premarked: m.w32(pointers[key] + 0x10, pointers[key])
    iterator = m.alloc(0xb0)
    position = [h.f2b(v) for v in xy]
    m.call(0x558720, ecx=iterator)
    m.call(0x5580f0, ecx=iterator, args=(*position, h.f2b(radius)))
    found = []
    for _ in range(100):
        actor = m.call(0x558750, ecx=iterator)
        if not actor: break
        found.append(m.r32(actor + 0x14))
    else:
        raise AssertionError('iterator failed to finish')
    m.call(0x557f90, ecx=iterator)
    m.call(0x557fd0, ecx=iterator)
    links = {str(k): (k if m.r32(p + 0x10) == p else None) for k, p in pointers.items()}
    assert all(m.r32(p + 0x10) in (0, p) for p in pointers.values())
    return dict(input=dict(grid=grid, extent_bits=extent, flags=flags, members=members, premarked=premarked,
                           position=position, radius=h.f2b(radius), source_facing=[0, h.f2b(-1.)]),
                native=dict(references=found, links=links))


def main():
    original = h.P; rows = []
    try:
        for label, image in [('original', original), ('v14', h.V)]:
            h.P = image
            for precision in h.PRECISION:
                for scenario in range(30):
                    rows.append(dict(image=label, precision=precision, scenario=scenario, **run(precision, scenario)))
    finally:
        h.P = original
    half = len(rows) // 2
    strip = lambda r: {k: v for k, v in r.items() if k != 'image'}
    assert [strip(r) for r in rows[:half]] == [strip(r) for r in rows[half:]]
    doc = dict(scope=__doc__, pairs=half, executions=len(rows), rows=rows[:half], original_sha256=h.ORIGINAL_SHA,
               v14_sha256=h.V14_SHA, producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               executed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-circle-actors.json'; payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        assert not path.exists(), 'use --check'
        path.write_bytes(payload)
    hits = sum(len(r['native']['references']) for r in rows[:half])
    print(json.dumps(dict(pairs=half, executions=len(rows), hits=hits, pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest(), result='PASS')))


if __name__ == '__main__':
    main()
