"""Oracle producer for the arithmetic of KKC_GGroup (Source/Sim/GGroup.cpp).

Runs the ORIGINAL code under Unicorn at PC53 (the simulation thread), and proves the
executed bytes equal in the pinned v14 image:

* 0x4DD590  the robust centre of a list of things
* 0x4DD860  a thing's offset from a centre, near or on the edge of the range
* 0x4DF600  the places of a group: the formation's own, the others where they stand
* 0x4DE020  a member's place at a point, with the tile walk 0x4DD030 / 0x4DDDE0 / 0x4DD150
            back to the point when the place is outside the zone
* 0x869D30 with the comparator 0x4DDF30 on 0x4D0780 / 0x579A60 / 0x579980: where 0x4E0940
            inserts a thing in the group's list
* 0x4E0330  the commanded Update's evening of the members' speeds, with its own seams

Seams, all explicit: 0x4F2420 (in the zone: the case's circle, answered and recorded),
0x5581B0 (the walk's constructor, which only stores the map at +4 here). Test objects
live in the laboratory heap; the rule [B39AB0]+18C, the handle table B364C0 and the map
at [B36534]+30 are laboratory objects holding the case's values. No game file is written.

  python native_group.py --write   # create (refuses to overwrite)
  python native_group.py           # replay: byte-identical check
"""
from pathlib import Path
import hashlib, json, random, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm  # noqa: E402
from native_machine import Machine, f32, from_f32  # noqa: E402

OUT = HERE / 'native-group.json'
PC = 'Double53'


def bits(m, a):
    return m.r(a)


class Lab:
    def __init__(self, range_value=18.0):
        m = self.m = Machine(PC)
        self.rules = m.alloc(0x400)
        self.range = m.alloc(4)
        m.w(self.range, f32(range_value))
        m.w(self.rules + 0x18C, self.range)
        m.w(0xB39AB0, self.rules)
        self.table = m.alloc(0x40000)
        m.w(0xB364C0, self.table)
        self.next_slot = 0x40

    def thing(self, x, y, extra=0x200):
        m = self.m
        t = m.alloc(extra)
        m.w(t + 0x20, x)
        m.w(t + 0x24, y)
        return t

    def handle(self, target):
        m = self.m
        slot = self.next_slot
        self.next_slot += 1
        gen = 0x11 + slot
        m.u.mem_write(self.table + 4 + slot * 2, gen.to_bytes(2, 'little'))
        m.w(self.table + 0x20004 + slot * 4, target)
        id_ = gen << 16 | slot
        m.w(target + 0x14, id_)
        return id_


def rnd_coord(r):
    k = r.randrange(5)
    if k == 0:
        return f32(r.uniform(-5.0, 5.0))
    if k == 1:
        return f32(r.uniform(0.0, 2000.0))
    if k == 2:
        return f32(round(r.uniform(0.0, 800.0)) + r.choice([0.0, 0.5, 0.25, 0.1]))
    if k == 3:
        return f32(r.uniform(100.0, 110.0))
    return f32(r.uniform(0.0, 1.0) * 16777216.0)


def center_cases(r):
    rows = []
    for n in list(range(1, 14)) * 25:
        lab = Lab(r.choice([18.0, 25.0, 1.0, 100.0]))
        m = lab.m
        pts = [[rnd_coord(r), rnd_coord(r)] for _ in range(n)]
        if r.random() < 0.3:
            base = [r.uniform(0, 500), r.uniform(0, 500)]
            pts = [[f32(base[0] + r.uniform(-20, 20)), f32(base[1] + r.uniform(-20, 20))] for _ in range(n)]
        things = [lab.thing(p[0], p[1]) for p in pts]
        array = m.array(things)
        lst = m.array([array, n, n])
        out = m.alloc(8)
        m.call(0x4DD590, args=(out, lst), pop=0)
        rows.append({'points': pts, 'range': m.r(lab.range), 'center': [m.r(out), m.r(out + 4)], 'left': m.r(lst + 4)})
        lab.m.verify_v14()
        VISITED.update(m.visited)
    return rows


def offset_cases(r):
    rows = []
    for i in range(300):
        lab = Lab(r.choice([18.0, 25.0, 1.0]))
        m = lab.m
        p = [rnd_coord(r), rnd_coord(r)]
        c = [rnd_coord(r), rnd_coord(r)]
        if i % 5 == 0:
            c = list(p)
        near = r.randrange(2)
        t = lab.thing(p[0], p[1])
        ca = m.array(c)
        out = m.alloc(8)
        m.call(0x4DD860, args=(out, t, near, ca), pop=0)
        rows.append({'p': p, 'c': c, 'near': near, 'range': m.r(lab.range), 'out': [m.r(out), m.r(out + 4)]})
        VISITED.update(m.visited)
    return rows


def group_object(lab, entries, zone=None):
    """entries: [(gtid or 0, offset bits, in_formation byte, near byte)]"""
    m = lab.m
    g = m.alloc(0x200)
    arr = m.alloc(0x10 * max(len(entries), 1))
    for i, (gtid, off, flag, near) in enumerate(entries):
        m.w(arr + i * 0x10, gtid)
        m.w(arr + i * 0x10 + 4, off[0])
        m.w(arr + i * 0x10 + 8, off[1])
        m.wb(arr + i * 0x10 + 0xC, flag)
        m.wb(arr + i * 0x10 + 0xD, near)
    m.w(g + 0x188, arr)
    m.w(g + 0x18C, len(entries))
    return g, arr


def place_cases(r):
    rows = []
    for i in range(300):
        lab = Lab(r.choice([18.0, 25.0, 1.0]))
        m = lab.m
        n = r.randrange(1, 12)
        members = []
        entries = []
        for k in range(n):
            live = r.random() > 0.15
            p = [rnd_coord(r), rnd_coord(r)]
            near = r.randrange(2)
            old = [f32(r.uniform(-3, 3)), f32(r.uniform(-3, 3))]
            if live:
                gtid = lab.handle(lab.thing(p[0], p[1]))
            else:
                gtid = 0xDEAD0001
            members.append({'live': live, 'p': p, 'near': near, 'old': old})
            entries.append((gtid, old, 0, near))
        g, arr = group_object(lab, entries)
        positions = []
        count = r.choice([0, 0, r.randrange(1, 12)])
        if count:
            positions = [[f32(r.uniform(-30, 30)), f32(r.uniform(-30, 30))] for _ in range(count)]
            f = m.alloc(0x28)
            m.w(f + 0x18, m.array([w for q in positions for w in q]))
            m.w(f + 0x1C, count)
            m.w(g + 0x1B0, f)
        center = [rnd_coord(r), rnd_coord(r)]
        a = r.uniform(0, 6.283)
        facing = r.choice([[f32(0.0), f32(-1.0)], [f32(0.6), f32(0.8)], [f32(__import__('math').cos(a)), f32(__import__('math').sin(a))]])
        m.call(0x4DF600, ecx=g, args=(m.array(center), m.array(facing)), pop=8)
        out = [[m.r(arr + k * 0x10 + 4), m.r(arr + k * 0x10 + 8)] for k in range(n)]
        rows.append({'members': members, 'positions': positions, 'center': center, 'facing': facing, 'range': m.r(lab.range), 'out': out})
        VISITED.update(m.visited)
    return rows


def offset_at_cases(r):
    rows = []
    for i in range(300):
        lab = Lab()
        m = lab.m
        shift = r.choice([1, 2, 3])
        cell = float(1 << shift)
        width, height = r.choice([(64, 64), (128, 100)])
        world = m.alloc(0x400)
        m.w(0xB36534, world)
        gmap = m.alloc(0x100)
        m.w(world + 0x30, gmap)
        m.w(gmap + 4, shift)
        m.w(gmap + 8, f32(cell))
        m.w(gmap + 0xC, f32(width * cell))
        m.w(gmap + 0x10, f32(height * cell))
        m.w(gmap + 0x1C, width)
        m.w(gmap + 0x20, height)
        off = [f32(r.uniform(-40, 40)), f32(r.uniform(-40, 40))]
        g, arr = group_object(lab, [(0, off, 1, 1)])
        fa = [f32(r.uniform(-1, 1)), f32(r.uniform(-1, 1))]
        m.w(g + 0x1A8, fa[0])
        m.w(g + 0x1AC, fa[1])
        point = [f32(r.uniform(0, width * cell)), f32(r.uniform(0, height * cell))]
        zone_radius = r.choice([-1.0, 0.0, 4.0, 15.0, 40.0])
        zc = [from_f32(point[0]) + r.uniform(-5, 5), from_f32(point[1]) + r.uniform(-5, 5)]
        zone = m.alloc(0x40)
        m.w(zone + 8, f32(zone_radius))
        zones = m.alloc(0x40)
        m.w(zones + 0x1C, zone)
        m.w(g + 0x148, zones)
        m.w(g + 0x14C, 8)
        tested = []

        def in_zone(m_):
            x, y = m_.arg(2), m_.arg(3)
            tested.append([x, y])
            d = (from_f32(x) - zc[0]) ** 2 + (from_f32(y) - zc[1]) ** 2
            m_.ret(1 if d <= zone_radius * zone_radius else 0, 12)
        m.seam(0x4F2420, in_zone)

        def walk_ctor(m_):
            m_.w(m_.ecx() + 4, gmap)
            m_.ret(m_.ecx(), 0)
        m.seam(0x5581B0, walk_ctor)
        out = m.alloc(8)
        m.call(0x4DE020, ecx=g, args=(out, arr, m.array(point)), pop=12)
        rows.append({'shift': shift, 'width': width, 'height': height, 'offset': off, 'facing': fa, 'point': point,
                     'zone_radius': f32(zone_radius), 'zone_center': [f32(zc[0]), f32(zc[1])], 'tested': tested,
                     'out': [m.r(out), m.r(out + 4)]})
        VISITED.update(m.visited)
    return rows


def order_cases(r):
    """A selection's values through 0x869D30 / 0x4DDF30, with 0x4D0780 on real shapes."""
    rows = []
    for i in range(120):
        lab = Lab()
        m = lab.m
        n = r.randrange(1, 10)
        things = []
        cases = []
        for k in range(n):
            kind = r.randrange(3)
            t = m.alloc(0x200)
            d = m.alloc(0x600)
            m.w(t + 4, d)
            if kind == 0:
                # an element: definition +2C4
                value = r.choice([0, 1, 2, 5, 9])
                m.w(t + 0x80, m.alloc(0x40))
                m.w(d + 0x2C4, value)
                cases.append({'kind': 'element', 'value': value})
            elif kind == 1:
                # an organization: +320 when positive, else 0x579980's element +2C4
                org = m.alloc(0x80)
                m.w(org + 4, t)
                m.w(t + 0x7C, org)
                pri = r.choice([0, 0, 3, 7, -1])
                m.w(d + 0x320, pri)
                slots = r.randrange(1, 5)
                layout = m.alloc(0x40)
                order = list(range(slots))
                r.shuffle(order)
                entries = m.alloc(0x10 * slots)
                elements = m.alloc(4 * slots)
                locked = []
                values = []
                for s in range(slots):
                    lk = r.randrange(4) == 0
                    locked.append(lk)
                    m.wb(entries + s * 0x10 + 0xD, 1 if lk else 0)
                    if r.random() < 0.8:
                        e = m.alloc(0x600)
                        v = r.choice([1, 2, 4, 8])
                        m.w(e + 0x2C4, v)
                        m.w(elements + s * 4, e)
                        values.append(v)
                    else:
                        values.append(None)
                primary = r.choice([-1, 0, slots - 1, slots + 2])
                m.w(layout + 0x18, primary & 0xffffffff)
                m.w(layout + 0x1C, entries)
                m.w(layout + 0x28, m.array(order))
                m.w(layout + 0x2C, slots)
                m.w(org + 0x18, layout)
                m.w(org + 0x1C, elements)
                m.w(org + 0x2C, slots)
                has_any = any(v is not None for v in values)
                # 0x579980 null dereferences nothing when it finds none: +2C4 of null faults.
                if pri <= 0 and not has_any:
                    m.w(elements, m.alloc(0x600))
                    values[0] = 0
                cases.append({'kind': 'organization', 'priority': pri, 'order': order, 'locked': locked, 'values': values, 'primary': primary})
            else:
                cases.append({'kind': 'other'})
            things.append(lab.handle(t))
        base = m.alloc(0x10 * (n + 1))
        listed = []
        indices = []
        for k in range(n):
            key = m.array([things[k], 0, 0, 0])
            out = m.alloc(4)
            for j, h in enumerate(listed):
                m.w(base + j * 0x10, h)
            m.call(0x869D30, args=(key, out, base, 0x10, len(listed), 0x4DDF30), pop=0)
            at = m.r(out)
            indices.append(at)
            listed.insert(at, things[k])
        rows.append({'things': cases, 'indices': indices})
        VISITED.update(m.visited)
    return rows


def speed_cases(r):
    """The commanded Update 0x4E0330 from its busy test on: the seams answer 0x4DEB80 (no
    fight), 0x4DDB60 (the member's flag), the Mind's vt+64, vt+44, vt+48, vt+88, 0x52CE70
    (the path left), 0x515F40 and the group's reschedule."""
    rows = []
    for i in range(300):
        lab = Lab()
        m = lab.m
        m.seam(0x6F7810, lambda m_: m_.ret(m_.alloc(max(m_.arg(1), 4)), 0))
        m.seam(0x6F7430, lambda m_: m_.ret(0, 0))
        m.seam(0x4DEB80, lambda m_: m_.ret(0, 0))
        n = r.randrange(1, 7)
        calls = []
        members = []
        entries = []
        flags = []
        for k in range(n):
            t = lab.thing(f32(r.uniform(0, 500)), f32(r.uniform(0, 500)))
            active = r.random() > 0.2
            left = r.choice([0.0, r.uniform(0, 400), r.uniform(0, 5)])
            top = r.uniform(1.0, 12.0)
            has_org = r.random() > 0.3
            near = r.randrange(2)
            flag_now = r.randrange(2)
            flag_old = flag_now if r.random() > 0.15 else 1 - flag_now
            if has_org:
                org = m.alloc(0x100)
                m.w(org + 0xD8, f32(top))
                m.w(t + 0x7C, org)
            else:
                mover = m.alloc(0x40)
                m.w(mover + 0x24, f32(top))
                m.w(t + 0x6C, mover)
            mind = m.alloc(0x40)
            vt = m.alloc(0x100)
            m.w(mind, vt)
            m.w(t + 0x70, mind)
            m.w(vt + 0x64, m.stub(lambda m_, a=active: m_.ret(1 if a else 0, 0)))
            path = m.alloc(0x10)
            m.w(path, f32(left))
            m.w(vt + 0x44, m.stub(lambda m_, p=path: m_.ret(p, 8)))
            m.w(vt + 0x48, m.stub(lambda m_, k=k: (calls.append(['replan', k]), m_.ret(0, 0))))
            m.w(vt + 0x88, m.stub(lambda m_, k=k: (calls.append(['speed', k, m_.arg(1)]), m_.ret(0, 4))))
            off = [f32(r.uniform(-30, 30)), f32(r.uniform(-30, 30))]
            entries.append((lab.handle(t), off, flag_old, near))
            flags.append(flag_now)
            members.append({'active': active, 'left': f32(left), 'top': f32(top), 'near': near, 'same': flag_old == flag_now, 'offset': off})
        g, arr = group_object(lab, entries)
        order = {'i': 0}

        def flag(m_):
            k = order['i']
            order['i'] += 1
            m_.ret(flags[k], 8)
        m.seam(0x4DDB60, flag)
        m.seam(0x52CE70, lambda m_: m_.ret_st0_f32(m_.r(m_.ecx()), 0))
        m.seam(0x515F40, lambda m_: m_.ret(0, 12))
        rule = m.alloc(4)
        buffer_rule = r.choice([0.0, 5.0, 12.5])
        m.w(rule, f32(buffer_rule))
        m.seam(0x4B2E60, lambda m_: m_.ret(rule, 8))
        together = r.randrange(2)
        m.wb(g + 0x1BC, together)
        fa = [f32(r.uniform(-1, 1)), f32(r.uniform(-1, 1))]
        m.w(g + 0x1A8, fa[0])
        m.w(g + 0x1AC, fa[1])
        m.w(g + 0x1B4, f32(18.0))
        kingdom = m.alloc(0x10)
        m.w(g + 0xE8, kingdom)
        # the leader
        m.w(g + 0x194, entries[0][0])
        zones = m.alloc(0x40)
        m.w(zones + 0x1C, 0)
        m.w(g + 0x148, zones)
        m.w(g + 0x14C, 8)
        cmd = m.alloc(0x40)
        m.w(cmd + 8, m.alloc(0x40))
        aux_on = r.randrange(2)
        aux = r.choice([[0, 0], [f32(0.6), f32(0.8)]])
        target_zone = r.choice([-1.0, 0.0, 8.0, 30.0])
        hostile = r.randrange(2)
        word = 0
        fl = 0
        if r.random() > 0.2:
            target = lab.thing(f32(100.0), f32(100.0))
            tvt = m.alloc(0x200)
            m.w(target, tvt)
            m.w(tvt + 0x140, m.stub(lambda m_, h=hostile: m_.ret(h, 4)))
            z = m.alloc(0x40)
            m.w(z + 8, f32(target_zone))
            tz = m.alloc(0x40)
            m.w(tz + 0x14, z)
            m.w(target + 0x148, tz)
            m.w(target + 0x14C, 8)
            word = lab.handle(target)
            fl |= 1
        else:
            hostile = 0
        if aux_on:
            fl |= 8
        m.w(cmd + 0xC, fl)
        m.w(cmd + 0x10, word)
        m.w(cmd + 0x20, aux[0])
        m.w(cmd + 0x24, aux[1])
        m.w(g + 0x1A4, cmd)
        upd = m.alloc(0x10)
        m.w(g + 0x180, upd)
        m.w(upd + 4, m.stub(lambda m_: (calls.append(['schedule', m_.arg(1)]), m_.ret(0, 8))))
        m.call(0x4E0330, ecx=g, args=(), pop=0)
        rows.append({'members': members, 'together': together, 'facing': fa, 'aux': aux if aux_on else None, 'buffer_rule': f32(buffer_rule),
                     'target_zone': f32(target_zone) if word else None, 'hostile': hostile, 'calls': calls})
        VISITED.update(m.visited)
    return rows


VISITED = {}


def produce():
    r = random.Random(0x4DF600)
    data = {
        'center': center_cases(r),
        'offset': offset_cases(r),
        'place': place_cases(r),
        'offset_at': offset_at_cases(r),
        'order': order_cases(r),
        'speed': speed_cases(r),
    }
    for a, b in VISITED.items():
        assert b == nm.original.range(a, len(b)) == nm.modified.range(a, len(b)), hex(a)
    head = nm.header(__file__, 'Unicorn execution of the original code with explicit seams (see producer docstring)')
    head.update({'precision': PC, 'executed_instructions': len(VISITED), 'cases': data})
    return head


def main():
    data = json.dumps(produce(), indent=None, sort_keys=True, separators=(',', ':')).encode()
    if '--write' in sys.argv:
        if OUT.exists():
            raise SystemExit(f'{OUT} exists; refusing to overwrite')
        OUT.write_bytes(data)
        print('wrote', OUT, len(data), hashlib.sha256(data).hexdigest())
    else:
        old = OUT.read_bytes()
        assert old == data, 'replay differs'
        print('replay identical', hashlib.sha256(data).hexdigest())


if __name__ == '__main__':
    main()
