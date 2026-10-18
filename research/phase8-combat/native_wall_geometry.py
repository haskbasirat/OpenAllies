"""Native oracle for the wall within-distance geometry (round 5):
585280(wall component; actor) and its point test 5851A0, executed entirely
natively (529C30 included). The intruder radius is the map's +8 plus the
actor's Organization +AC, or the actor virtual +10 data's +4.
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402

f32 = h.f32


def polygon(rng, kind):
    if kind == 'square':
        s = rng.choice((4.0, 10.0, 25.5))
        return [(-s, -s), (-s, s), (s, s), (s, -s)]
    if kind == 'ccw_square':
        s = rng.choice((4.0, 10.0))
        return [(-s, -s), (s, -s), (s, s), (-s, s)]
    if kind == 'triangle':
        return [(0.0, 12.0), (10.0, -6.0), (-10.0, -6.0)]
    if kind == 'random':
        return [(rng.uniform(-20, 20), rng.uniform(-20, 20)) for _ in range(rng.randint(1, 6))]
    return []


def main():
    rng = random.Random(0x585280)
    m = h.Machine('Double53')
    m.u.mem_map(0, 0x1000)
    world = h.install_world(m)
    mapobj = m.alloc(0x40)
    m.w(world + 0x30, mapobj)
    cases = []
    for index in range(400):
        kind = rng.choice(('square', 'ccw_square', 'triangle', 'random', 'empty'))
        verts = polygon(rng, kind)
        wall_pos = [rng.uniform(-50, 50), rng.uniform(-50, 50)]
        organization = rng.random() < 0.4
        tile = rng.choice((0.0, 2.0, 3.5))
        org_radius = rng.choice((0.0, 1.5, 6.0))
        radius = rng.choice((0.0, 0.5, 2.0, 7.0))
        near = rng.random() < 0.08
        if near:
            actor_pos = [wall_pos[0] + rng.choice((0.0, 5e-5)), wall_pos[1]]
        else:
            actor_pos = [wall_pos[0] + rng.uniform(-30, 30), wall_pos[1] + rng.uniform(-30, 30)]
        m.w(mapobj + 8, f32(tile))
        comp = m.alloc(0x40)
        wall_actor = m.alloc(0x100)
        wdef = m.alloc(0x500)
        m.w(comp + 4, wall_actor)
        m.w(wall_actor + 4, wdef)
        m.w(wall_actor + 0x20, f32(wall_pos[0]))
        m.w(wall_actor + 0x24, f32(wall_pos[1]))
        records = m.alloc(0x14 * max(1, len(verts)))
        for i, (x, y) in enumerate(verts):
            m.w(records + 0x14 * i + 4, f32(x))
            m.w(records + 0x14 * i + 8, f32(y))
        m.w(wdef + 0x47C, records)
        m.w(comp + 0x28, len(verts))
        actor = m.alloc(0x100)
        m.w(actor + 0x20, f32(actor_pos[0]))
        m.w(actor + 0x24, f32(actor_pos[1]))
        if organization:
            org = m.alloc(0x100)
            m.w(org + 0xAC, f32(org_radius))
            m.w(actor + 0x7C, org)
        data = m.alloc(0x20)
        m.w(data + 4, f32(radius))
        m.w(actor, m.vtable({0x10: lambda mm, d=data: mm.ret(0, d)}))
        al = m.call(0x585280, ecx=comp, args=(actor,), pop=4) & 0xFF
        point = [f32(rng.uniform(-40, 40)), f32(rng.uniform(-40, 40))]
        direct = m.call(0x5851A0, ecx=comp, args=(point[0], point[1]), pop=8) & 0xFF
        cases.append({'polygon': [[f32(x), f32(y)] for x, y in verts], 'wall': [f32(v) for v in wall_pos],
                      'actor': [f32(v) for v in actor_pos], 'organization': organization, 'tile': f32(tile),
                      'organization_radius': f32(org_radius), 'radius': f32(radius), 'within': al,
                      'point': point, 'point_within': direct})
    h.report('native-wall-geometry-fixtures.json', '585280 / 5851A0 within-wall geometry (round 5).', cases,
             m.visited)
    print(sum(c['within'] for c in cases), sum(c['point_within'] for c in cases))


if __name__ == '__main__':
    main()
