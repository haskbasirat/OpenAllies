"""GWorld::Load post-record stage: the PostLoad pass 4FD3B3..4FD3F1 of
4FC600 run natively over synthetic GTID tables (Unicorn, native_machine.py).

Executed natively: the pass loop and iterator 4EFBE0, the class PostLoad
virtuals (+DC): GThing/Team 943800, GObject 4EAA30, GActor 4D3560 (component
+30 dispatch, 50C510 list pruning), GKingdom 4E4220 (4D19F0, 4E3ED0/4E28D0,
4E1C70, 4D1E80), and the component +30 virtuals Body 561DF0 (561C60, 561B40,
529C30), Properties/Supply 581260, Denizen 569F80 (52A020), Wall 58CB40,
Defender 5676B0 (567140 with a null target, 4CE6C0, 4D0CD0/4F1430, 79E3C0),
Hq 5736B0 (4E2640), and the 943800 no-ops of every other component class.

Round 5 additions executed natively: virtual +12C (GObject 4ECC50, GActor
4CFF70 with ThingData virtual +24 = 462E50 and 4BB000), RecruitSingle items
with their real vtable A29694 (virtual +8 = 4E0EC0) and 4E1C70 / 578DA0 (a
probe stops at 578DAB when the slot index is outside Organization+34:
4E4220 passes -1, undefined), Settlement 586BC0 (45D7C0), GGroup 4DDFA0
(4DDCD0). Extra seams: virtual +50 (4D4A10 / 4EE840), UI agent 79DEB0 /
79E7B0, memory record refresh 4EC4A0, zone move 515F40, map clamp 552640
(a recorded clamp of each coordinate to [0, clamp_max]), CreateGThing
4F9C10 (the GACD block is recorded; scripted results).

Seams (recorded in order): Updatable queue helpers 50AC00 (queue 0 periodic),
50AC60 (queue 1 periodic), 50ACC0 (queue 3 add_after), 50B220 (has),
50B1C0 (remove matching), 50B250 (cancel all) on Component+8 (their own gates,
Updatable vt+8/+C, are 6A71D0 = true for every component reached here); the
world RNG draw 4F02B0+85D840 (scripted values); the 404ED0 log and the 5A1040
map warning. The static of
561B40 (UnitSupplyEnabled SVar pointer B3D654, guard bit B3D658) is preset
as after its first call. Allocation/free: native_machine bump allocator.
"""
from pathlib import Path
import sys, json, struct, copy
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, P, V, VISITED, H, f32, Throw, STACK, G_WORLD
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESP, UC_X86_REG_EIP, UC_X86_REG_EDI, UC_X86_REG_ESI

SLOTS = ['Body', 'Projectile', 'Character', 'Mover', 'Mind', 'Performer', 'Properties', 'Organization',
         'Element', 'Economy', 'Denizen', 'Supply', 'Wall', 'Structure', 'Settlement', 'Construction',
         'Recruit', 'Experience', 'Builder', 'MobileDeployment', 'Defender', 'Research', 'Detection',
         'Aircraft', 'Transport', 'Lair', 'Flare', 'MetaGame', 'Portal', 'City', 'Hq']
# Component vtable, embedded Updatable (+8) vtable.
COMPONENT_VT = {
    'Body': (0xa3b0e8, 0xa3b0c0), 'Projectile': (0xa3e280, 0), 'Character': (0xa3b4d8, 0),
    'Mover': (0xa3ce88, 0), 'Mind': (0xa35810, 0), 'Performer': (0xa3df40, 0),
    'Properties': (0xa3e3dc, 0xa3e3b4), 'Organization': (0xa3d434, 0), 'Element': (0xa3c6f0, 0),
    'Economy': (0xa3c690, 0), 'Denizen': (0xa3c14c, 0xa3c124), 'Supply': (0xa400c0, 0xa40098),
    'Wall': (0xa40198, 0xa40170), 'Structure': (0xa3fac8, 0), 'Construction': (0xa3b950, 0),
    'Recruit': (0xa3e6d0, 0), 'Experience': (0xa3c974, 0), 'Builder': (0xa3b2a8, 0),
    'MobileDeployment': (0xa3cd00, 0), 'Defender': (0xa3bbd4, 0xa3bbac), 'Research': (0xa3eb58, 0),
    'Detection': (0xa3c568, 0), 'Aircraft': (0xa3af78, 0), 'Transport': (0xa40100, 0),
    'Lair': (0xa3cc6c, 0), 'Flare': (0xa3c9e4, 0), 'Portal': (0xa3e144, 0), 'City': (0xa3b6f8, 0),
    'Hq': (0xa3ca4c, 0xa3ca24), 'Settlement': (0xa3eea0, 0xa3ee78)}
# GTID object classes: vtable, size.
CLASS_VT = {'Thing': (0xa2a680, 0x60), 'Team': (0xa29780, 0x6c), 'Object': (0xa2a2a0, 0x60),
            'Actor': (0xa273f0, 0x180), 'Kingdom': (0xa299d8, 0x378), 'Group': (0xa29490, 0x1c0)}
ACTORISH = ('Actor', 'Kingdom', 'Group')
RECRUIT_SINGLE_VT = 0xa29694


def fbits(v):
    return struct.unpack('<f', struct.pack('<I', v))[0]


def clamp_bits(v, top):
    f, t = fbits(v), fbits(top)
    if f < 0.0:
        return 0
    if f > t:
        return top
    return v
LISTS = (0x118, 0x120, 0x128)
PASS_BEGIN, PASS_END = 0x4fd3b3, 0x4fd3f3


def signed(v):
    return v - (1 << 32) if v >> 31 else v


class PostLoadMachine(Machine):
    def __init__(self, case):
        super().__init__(b'', case.get('precision', 'Double53'))
        self.case = case
        self.table = self.alloc(0x60010)
        self.w(0xb364c0, self.table)
        self.w(self.table + 0x6000c, case['max_slot'])
        self.world = self.alloc(0x300)
        self.w(self.world + 0x2c, case.get('world_version', 27))
        self.w(G_WORLD, self.world)
        svar = self.alloc(4)
        self.wb(svar, case.get('unit_supply', 1))
        self.w(0xb3d654, svar)
        self.wb(0xb3d658, self.b(0xb3d658) | 1)
        self.rng = self.alloc(16)
        self.script = list(case.get('random', []))
        gmap = self.alloc(0x40)
        self.w(gmap + 8, case.get('tile_size', f32(32.0)))
        self.w(self.world + 0x30, gmap)
        self.gmap = gmap
        mgr = self.alloc(0x100)
        self.w(mgr + 0x74, case.get('manager_74', f32(64.0)))
        self.w(0xb39f58, mgr)
        if case.get('ui_agent') is not None:
            ui = self.alloc(0x40)
            self.wb(ui + 0x20, case['ui_agent'])
            self.w(0xb36528, ui)
        else:
            self.w(0xb36528, 0)
        self.created = list(case.get('created', []))
        self.clamp_max = case.get('clamp_max', f32(1000.0))
        self.records = {}     # memory record label -> address
        self.zones = {}       # zone object address -> (object label, slot)
        self.group_targets = {}
        self.sched = {}       # component address -> [[queue, data], ...] head first
        self.comp_info = {}   # component address -> (object label, slot name)
        self.addr = {}        # object label -> address
        self.label_of = {}    # address -> label
        self.items = {}       # recruit item address -> (kingdom label, spec)
        self.commands = {}    # command label -> address
        self.sentinel = self.alloc(16)
        self._seams()
        self._build()
        kingdoms = case.get('world_kingdoms', [])
        vec = self.alloc(4 * max(1, len(kingdoms)))
        for i, k in enumerate(kingdoms):
            self.w(vec + 4 * i, self.addr[k] if k else 0)
        self.w(self.world + 0x154, vec)
        self.w(self.world + 0x158, len(kingdoms))

    # ------------------------------------------------------------ helpers
    def ecx(self):
        return self.u.reg_read(UC_X86_REG_ECX)

    def gen(self, slot, generation):
        self.u.mem_write(self.table + 4 + slot * 2, struct.pack('<H', generation))

    def list_write(self, at, values, node_size=8):
        """Singly linked list {head at, tail at+4}; node {+0 value, +4 next}."""
        nodes = []
        for v in values:
            n = self.alloc(node_size)
            if isinstance(v, (list, tuple)):
                for i, x in enumerate(v):
                    self.w(n + 4 * i, x)
            else:
                self.w(n, v)
            nodes.append(n)
        nxt = 4 if node_size == 8 else 8
        for i, n in enumerate(nodes):
            self.w(n + nxt, nodes[i + 1] if i + 1 < len(nodes) else 0)
        self.w(at, nodes[0] if nodes else 0)
        self.w(at + 4, nodes[-1] if nodes else 0)

    def list_read(self, at, width=1, node_size=8):
        out = []
        n = self.r(at)
        nxt = 4 if node_size == 8 else 8
        while n:
            out.append(self.r(n) if width == 1 else [self.r(n + 4 * i) for i in range(width)])
            n = self.r(n + nxt)
        return out

    def sync(self, comp):
        self.w(comp + 0xc, self.sentinel if self.sched[comp] else 0)

    def owner_of(self, comp):
        label, slot = self.comp_info[comp]
        return {'object': label, 'slot': slot}

    # -------------------------------------------------------------- seams
    def _seams(self):
        def periodic(name, queue):
            def h(m):
                comp = m.ecx() - 8
                data = m.arg(0)
                m.sched[comp].insert(0, [queue, data])
                m.sync(comp)
                m.record(name, **m.owner_of(comp), queue=queue, data=data)
                m.ret(4)
            return h
        self.seam(0x50ac00, periodic('50AC00', 0))
        self.seam(0x50ac60, periodic('50AC60', 1))

        def after(m):  # 50ACC0(delay, data): queue 3
            comp = m.ecx() - 8
            delay, data = m.arg(0), m.arg(1)
            m.sched[comp].insert(0, [3, data])
            m.sync(comp)
            m.record('50ACC0', **m.owner_of(comp), queue=3, data=data, delay=delay)
            m.ret(8)
        self.seam(0x50acc0, after)

        def has(m):  # 50B220(queue, data) -> al
            comp = m.ecx() - 8
            q, d = m.arg(0), m.arg(1)
            r = any(e == [q, d] for e in m.sched[comp])
            m.record('50B220', **m.owner_of(comp), queue=q, data=d, result=r)
            m.ret(8, int(r))
        self.seam(0x50b220, has)

        def remove(m):  # 50B1C0(queue, data) -> count
            comp = m.ecx() - 8
            q, d = m.arg(0), m.arg(1)
            before = len(m.sched[comp])
            m.sched[comp] = [e for e in m.sched[comp] if e != [q, d]]
            m.sync(comp)
            n = before - len(m.sched[comp])
            m.record('50B1C0', **m.owner_of(comp), queue=q, data=d, removed=n)
            m.ret(8, n)
        self.seam(0x50b1c0, remove)

        def cancel(m):  # 50B250()
            comp = m.ecx() - 8
            m.sched[comp] = []
            m.sync(comp)
            m.record('50B250', **m.owner_of(comp))
            m.ret(0)
        self.seam(0x50b250, cancel)

        def world_rng(m):  # 4F02B0 thiscall on GWorld -> RNG object
            assert m.ecx() == m.world
            m.ret(0, m.rng)
        self.seam(0x4f02b0, world_rng)

        def draw(m):  # 85D840(rng) cdecl
            assert m.arg(0) == m.rng
            if not m.script:
                raise RuntimeError('random script exhausted')
            v = m.script.pop(0)
            m.record('random', value=v)
            m.ret(0, v)
        self.seam(0x85d840, draw)

        def blocking(m):  # virtual +50(flag): 4D4A10 / 4EE840
            m.record('set_blocking', object=m.label_of[m.ecx()], flag=m.arg(0) & 0xff)
            m.ret(4, 1)
        self.seam(0x4d4a10, blocking)
        self.seam(0x4ee840, blocking)

        def ui_memory(after):
            def h(m):  # 79DEB0 / 79E7B0(record, kingdom) thiscall on [B36528]
                assert m.ecx() == m.r(0xb36528)
                rec = m.arg(0)
                m.record('ui_memory', after=after, record=m.label_of.get(rec) if rec else None,
                         kingdom=m.label_of[m.arg(1)])
                m.ret(8)
            return h
        self.seam(0x79deb0, ui_memory(False))
        self.seam(0x79e7b0, ui_memory(True))

        def refresh(m):  # 4EC4A0(record) thiscall on the object
            m.record('refresh_memory_record', object=m.label_of[m.ecx()], record=m.label_of[m.arg(0)])
            m.ret(4)
        self.seam(0x4ec4a0, refresh)

        def zone_move(m):  # 515F40(x, y, radius) thiscall on a zone
            if m.ecx() == 0:  # the original dereferences the null zone inside 515F40
                m.outcome = {'kind': 'NullDereference', 'pc': '0x515f40'}
                m.u.emu_stop()
                return
            label, slot = m.zones[m.ecx()]
            m.record('515F40', object=label, zone=slot, x=m.arg(0), y=m.arg(1), radius=m.arg(2))
            m.ret(12)
        self.seam(0x515f40, zone_move)

        def clamp(m):  # 552640(&xy) thiscall on [GWorld+30]
            assert m.ecx() == m.gmap
            at = m.arg(0)
            before = [m.r(at), m.r(at + 4)]
            after = [clamp_bits(v, m.clamp_max) for v in before]
            m.w(at, after[0])
            m.w(at + 4, after[1])
            m.record('552640', xy=before, out=after)
            m.ret(4, at)
        self.seam(0x552640, clamp)

        def create(m):  # 4F9C10(&block) thiscall on GWorld
            assert m.ecx() == m.world
            b = m.arg(0)
            if m.r(b + 4) == 0:  # 4F9C92 reads [definition+18]
                m.outcome = {'kind': 'NullDereference', 'pc': '0x4f9c92'}
                m.u.emu_stop()
                return
            owner = m.r(b + 0x28)
            block = {'vtable': m.r(b), 'definition': m.label_of[m.r(b + 4)],
                     'position': [m.r(b + 8), m.r(b + 0xc), m.r(b + 0x10)],
                     'direction': [m.r(b + 0x14), m.r(b + 0x18)],
                     'name_length': m.r(m.r(b + 0x1c) - 12), 'variant': m.r(b + 0x20),
                     'requested_id': m.r(b + 0x24), 'owner': m.label_of[owner] if owner else None,
                     'words_2c_4c': [m.r(b + o) for o in range(0x2c, 0x50, 4)],
                     'bytes_50': [m.b(b + 0x50), m.b(b + 0x51), m.b(b + 0x52)]}
            r = m.created.pop(0)
            m.record('4F9C10', block=block, result=r)
            if r is None:
                m.ret(4, 0)
            else:
                obj = m.alloc(0x20)
                m.w(obj + 0x14, r)
                m.ret(4, obj)
        self.seam(0x4f9c10, create)

        def org_slot(m):  # probe at 578DAB: Organization+34[edi]
            index = signed(m.u.reg_read(UC_X86_REG_EDI))
            comp = m.u.reg_read(UC_X86_REG_ESI)
            count = m.r(comp + 0x38)
            if index < 0 or index >= count:
                m.outcome = {'kind': 'Undefined', 'pc': '0x578dab', 'index': index, 'count': count}
                m.u.emu_stop()
        self.probes[0x578dab] = org_slot

        def logger(m):  # 404ED0(B38B70, file, line, function, fmt, ...)
            fmt = m.read_wide(m.arg(4))
            e = m.record('404ED0', line=m.arg(2), function=m.read_wide(m.arg(3)), format=fmt,
                         args=m.fmt_args(fmt, 5))
            m.logs.append(e)
            m.ret(0)
        self.seam(0x404ed0, logger)

    def command_delete(self, m):
        m.record('command_delete', command=m.label_of[m.ecx()])
        m.ret(4)

    # ------------------------------------------------------------- build
    def _build(self):
        c = self.case
        for slot, generation in c.get('stale', []):
            self.gen(slot, generation)
        for o in c['objects']:
            vt, size = CLASS_VT[o['class']]
            a = self.alloc(size)
            self.w(a, vt)
            oid = (o['gen'] << 16) | o['slot']
            self.w(a + 0x14, oid)
            self.w(a + 8, o.get('flags8', 0))
            self.gen(o['slot'], o['gen'])
            self.w(self.table + 0x20004 + o['slot'] * 4, a)
            self.addr[o['label']] = a
            self.label_of[a] = o['label']
        for o in c['objects']:
            self._fill(o)

    def definition(self, name):
        d = self.alloc(0x500)
        self.w(d, self.def_vt)
        self.w(d + 0x10, self.cstring(name))
        self.label_of[d] = 'def:' + name
        return d

    def _fill(self, o):
        a = self.addr[o['label']]
        if not hasattr(self, 'def_vt'):
            self.def_vt = self.vtable({0x24: 0x462e50})
        d = self.definition(o.get('name', o['label']))
        self.w(d + 0x1c, o.get('def_1c', 0))
        self.wb(d + 0x34, o.get('def_34', 0))
        self.w(d + 0x2a8, o.get('def_2a8', 0))
        self.wb(d + 0x42c, o.get('def_42c', 0))
        entries = o.get('settlement_entries', [])
        ea = self.alloc(0x14 * max(1, len(entries)))
        for i, e in enumerate(entries):
            for j, v in enumerate(e):
                self.w(ea + 0x14 * i + 4 * j, v)
        self.w(d + 0x47c, ea)
        for k, off in enumerate((0x49c, 0x4a0, 0x4a4)):
            names = o.get('settlement_defs', [])
            if k < len(names) and names[k]:
                self.w(d + off, self.definition(names[k]))
        self.w(a + 4, d)
        pos = o.get('pos', [f32(100.0), f32(200.0), f32(5.0)])
        for i, v in enumerate(pos):
            self.w(a + 0x20 + 4 * i, v)
        if o['class'] in ('Object',) + ACTORISH:
            self.w(a + 0x50, o.get('required', 0))
            self.w(a + 0x58, o.get('memory_count', 0) & 0xffffffff)
            mem = self.alloc(4 * 16)
            for idx, label in o.get('memory', {}).items():
                rec = self.alloc(0x20)
                self.records[label] = rec
                self.label_of[rec] = label
                self.w(mem + 4 * int(idx), rec)
            self.w(a + 0x5c, mem)
        if o['class'] not in ACTORISH:
            return
        nz = o.get('zones', 0)
        zv = self.alloc(4 * max(1, nz))
        for i in range(nz):
            z = self.alloc(0x18)
            self.zones[z] = (o['label'], i)
            self.w(zv + 4 * i, z)
        self.w(a + 0x148, zv)
        self.w(a + 0x14c, nz)
        if o['class'] == 'Group':
            t = o.get('target')
            if t is not None:
                ta = self.alloc(0x20)
                self.w(ta + 0xc, t['flags'])
                self.w(ta + 0x10, t['object'])
                self.w(ta + 0x18, t['xy'][0])
                self.w(ta + 0x1c, t['xy'][1])
                self.w(a + 0x1a4, ta)
                self.group_targets[o['label']] = ta
            self.w(a + 0x1b4, o.get('radius_1b4', f32(48.0)))
        for off in LISTS:
            self.list_write(a + off, o.get('lists', {}).get(hex(off), []))
        self.w(a + 0x16c, o.get('word_16c', 0))
        self.w(a + 0xe8, self.addr[o['e8']] if o.get('e8') else 0)
        for name, spec in o.get('components', {}).items():
            self._component(a, o['label'], name, spec)
        if o['class'] == 'Kingdom':
            self._kingdom(a, o)

    def _component(self, owner, label, name, spec):
        vt, uvt = COMPONENT_VT[name]
        comp = self.alloc(0x100)
        self.w(comp, vt)
        self.w(comp + 4, owner)
        self.w(comp + 8, uvt)
        self.w(owner + 0x60 + 4 * SLOTS.index(name), comp)
        self.comp_info[comp] = (label, name)
        self.label_of[comp] = label + '.' + name
        self.sched[comp] = [list(e) for e in spec.get('scheduled', [])]
        self.sync(comp)
        if name == 'Element':
            self.w(comp + 0x10, self.addr[spec['organization']] if spec.get('organization') else 0)
        elif name == 'Mind':
            self.w(comp + 0x30, 0)
        elif name == 'Denizen':
            nodes = []
            for cmd in spec.get('commands', []):
                if cmd is None:
                    nodes.append(0)
                    continue
                x = self.alloc(0x20)
                self.w(x, self.vtable({0: self.command_delete}))
                self.w(x + 4, cmd['refcount'])
                self.list_write(x + 0xc, cmd['members'])
                self.commands[cmd['label']] = x
                self.label_of[x] = cmd['label']
                nodes.append(x)
            self.list_write(comp + 0x40, nodes)
        elif name == 'Defender':
            abil = []
            for target in spec.get('abilities', []):
                x = self.alloc(0x20)
                self.w(x + 0x14, target)
                abil.append(x)
            vec = self.alloc(4 * max(1, len(abil)))
            for i, x in enumerate(abil):
                self.w(vec + 4 * i, x)
            self.w(comp + 0x10, vec)
            self.w(comp + 0x14, len(abil))
            self.w(comp + 0x18, len(abil))
            self.w(comp + 0x1c, spec.get('word_1c', 0))
            self.list_write(comp + 0x24, spec.get('entries', []), node_size=0xc)
        elif name == 'Organization':
            n = spec.get('recruit_slots', 0)
            self.w(comp + 0x34, self.alloc(4 * max(1, n)))
            self.w(comp + 0x38, n)
        elif name == 'Settlement':
            n = spec.get('count', 0)
            self.w(comp + 0x28, n & 0xffffffff)
            self.w(comp + 0x3c, self.alloc(4 * max(1, n)))
        elif name == 'Hq':
            self.w(comp + 0x10, spec.get('index', 0) & 0xffffffff)
            n = spec.get('items', 0)
            vec = self.alloc(4 * max(1, n))
            for i in range(n):
                self.w(vec + 4 * i, self.alloc(0x10))
            self.w(comp + 0x20, vec)
            self.w(comp + 0x24, n)

    def _kingdom(self, k, o):
        groups = o.get('groups', [])
        vec = self.alloc(4 * max(1, len(groups)))
        for i, members in enumerate(groups):
            g = self.alloc(0x20)
            for off in (0, 4, 8):
                self.w(g + off, self.cstring(''))
            buf = self.alloc(4 * max(1, len(members)))
            for j, x in enumerate(members):
                self.w(buf + 4 * j, x)
            self.w(g + 0xc, buf)
            self.w(g + 0x10, len(members))
            self.w(g + 0x14, len(members))
            self.w(vec + 4 * i, g)
        self.w(k + 0x34c, vec)
        self.w(k + 0x350, len(groups))
        self.w(k + 0x190, o.get('word_190', 0))
        self.w(k + 0x18c, o.get('index_18c', 0))
        items = o.get('recruit_items', [])
        ivec = self.alloc(4 * max(1, len(items)))
        for i, spec in enumerate(items):
            it = self.alloc(0x24)
            self.w(it, RECRUIT_SINGLE_VT)
            dd = self.alloc(0x20)
            self.w(dd + 0x10, self.cstring(spec['name']))
            self.w(it + 4, dd)
            self.wb(it + 8, spec.get('byte_8', 0))
            self.w(it + 0xc, 0)
            self.w(it + 0x10, spec.get('organization', 0))
            self.items[it] = (o['label'], spec)
            self.label_of[it] = spec['label']
            self.w(ivec + 4 * i, it)
        self.w(k + 0x214, ivec)
        self.w(k + 0x218, len(items))
        self.w(k + 0x224, o.get('counter_224', 0))
        if o.get('hq_slots') is not None:
            f = self.alloc(0x80)
            self.w(f + 0x40, o['hq_slots'] & 0xffffffff)
            self.w(k + 0x234, f)
        else:
            self.w(k + 0x234, 0)
        self.list_write(k + 0x36c, [x & 0xffffffff for x in o.get('hq_taken', [])])

    # --------------------------------------------------------------- run
    def run(self):
        frame = STACK - 0x400
        self.w(frame + 0x80, self.ctx)
        self.u.reg_write(UC_X86_REG_ESP, frame)
        try:
            self.u.emu_start(PASS_BEGIN, PASS_END, count=5_000_000)
        except Throw:
            pass
        if self.null_accesses and self.outcome is None:
            self.outcome = {'kind': 'NullDereference', 'pc': self.null_accesses[0][0]}
        if self.outcome is None:
            assert self.u.reg_read(UC_X86_REG_EIP) == PASS_END, hex(self.u.reg_read(UC_X86_REG_EIP))
            assert self.u.reg_read(UC_X86_REG_ESP) == frame
        return {'outcome': self.outcome, 'calls': self.calls, 'state': self.snapshot()}

    def snapshot(self):
        out = {}
        for o in self.case['objects']:
            a = self.addr[o['label']]
            s = {}
            if 'required' in o or 'flags8' in o:
                s['flags8'] = self.r(a + 8)
                if o['class'] in ('Object',) + ACTORISH:
                    s['required'] = self.r(a + 0x50)
            if o['class'] == 'Group' and o['label'] in self.group_targets:
                ta = self.group_targets[o['label']]
                s['target'] = {'flags': self.r(ta + 0xc), 'xy': [self.r(ta + 0x18), self.r(ta + 0x1c)]}
            if o['class'] in ACTORISH:
                s['lists'] = {hex(off): self.list_read(a + off) for off in LISTS}
                comps = {}
                for i, name in enumerate(SLOTS):
                    comp = self.r(a + 0x60 + 4 * i)
                    if not comp:
                        continue
                    cs = {'scheduled': self.sched[comp]}
                    if name == 'Mind':
                        x = self.r(comp + 0x30)
                        cs['command'] = self.label_of[x] if x else None
                    elif name == 'Defender':
                        vec, n = self.r(comp + 0x10), self.r(comp + 0x14)
                        cs['ability_targets'] = [self.r(self.r(vec + 4 * j) + 0x14) for j in range(n)]
                        cs['entries'] = self.list_read(comp + 0x24, width=2, node_size=0xc)
                    elif name == 'Settlement':
                        n = signed(self.r(comp + 0x28))
                        cs['things'] = [self.r(self.r(comp + 0x3c) + 4 * j) for j in range(max(0, n))]
                    elif name == 'Hq':
                        cs['index'] = signed(self.r(comp + 0x10))
                        vec, n = self.r(comp + 0x20), self.r(comp + 0x24)
                        cs['items'] = n
                    comps[name] = cs
                s['components'] = comps
            if o['class'] == 'Kingdom':
                vec, n = self.r(a + 0x34c), self.r(a + 0x350)
                groups = []
                for j in range(n):
                    g = self.r(vec + 4 * j)
                    groups.append([self.r(self.r(g + 0xc) + 4 * x) for x in range(self.r(g + 0x10))])
                s['groups'] = groups
                ivec, n = self.r(a + 0x214), self.r(a + 0x218)
                s['recruit_items'] = [{'label': self.label_of[self.r(ivec + 4 * j)],
                                       'available': self.b(self.r(ivec + 4 * j) + 8),
                                       'organization': self.r(self.r(ivec + 4 * j) + 0x10)} for j in range(n)]
                s['counter_224'] = self.r(a + 0x224)
                s['hq_taken'] = [signed(x) for x in self.list_read(a + 0x36c)]
            out[o['label']] = s
        out['commands'] = {k: {'refcount': self.r(x + 4)} for k, x in self.commands.items()}
        return out


# ------------------------------------------------------------------ cases
def gid(gen, slot):
    return (gen << 16) | slot


def actor(label, slot, gen=1, **kw):
    return {'label': label, 'class': 'Actor', 'slot': slot, 'gen': gen, **kw}


def cases():
    rows = []

    def add(name, objects, **kw):
        max_slot = kw.pop('max_slot', max([o['slot'] for o in objects] + [0]))
        rows.append({'name': name, 'objects': objects, 'max_slot': max_slot, **kw})

    ONE, NAN, INF, DEN, NEG0 = f32(1.0), 0x7fc00000, 0x7f800000, 0x00000001, 0x80000000
    add('empty_table', [])
    add('inert_and_objects', [
        {'label': 'thing', 'class': 'Thing', 'slot': 1, 'gen': 1},
        {'label': 'team', 'class': 'Team', 'slot': 2, 'gen': 1},
        {'label': 'obj_ok', 'class': 'Object', 'slot': 3, 'gen': 1, 'required': gid(1, 1)},
        {'label': 'obj_none', 'class': 'Object', 'slot': 5, 'gen': 2, 'required': 0}])
    add('object_required_missing', [
        {'label': 'obj_stale', 'class': 'Object', 'slot': 1, 'gen': 1, 'required': gid(2, 3), 'name': 'trigger_area'},
        {'label': 'target', 'class': 'Object', 'slot': 3, 'gen': 1},
        {'label': 'obj_empty', 'class': 'Object', 'slot': 4, 'gen': 1, 'required': gid(0, 9)},
        {'label': 'obj_destroyed_req', 'class': 'Object', 'slot': 6, 'gen': 1, 'required': gid(1, 7)},
        {'label': 'destroyed', 'class': 'Object', 'slot': 7, 'gen': 1, 'flags8': 1}],
        stale=[[9, 0]])
    add('actor_list_pruning', [
        actor('a', 1, lists={'0x118': [gid(1, 2), gid(1, 3), gid(2, 2), 0, gid(1, 4), gid(1, 2)],
                             '0x120': [gid(1, 3)], '0x128': [gid(1, 9), gid(1, 9)]}),
        actor('b', 2), actor('dead', 3, flags8=1), {'label': 'obj', 'class': 'Object', 'slot': 4, 'gen': 1, 'flags8': 0x40000000}])
    add('actor_required_missing_then_components', [
        actor('a', 1, required=gid(3, 2), name='usa_infantry',
              components={'Character': {}, 'Mover': {}, 'Experience': {}, 'Properties': {}})])
    # Body 561DF0
    sup = dict(def_2a8=ONE)
    add('body_schedules', [actor('a', 1, **sup, components={'Body': {}})])
    add('body_already_scheduled', [actor('a', 1, **sup, components={'Body': {'scheduled': [[3, 0], [1, 0]]}})])
    add('body_unschedules_when_disabled', [actor('a', 1, **sup, components={'Body': {'scheduled': [[1, 0], [1, 1], [1, 0]]}})],
        unit_supply=0)
    add('body_disabled_nothing_scheduled', [actor('a', 1, **sup, components={'Body': {'scheduled': [[1, 1]]}})], unit_supply=0)
    add('body_with_organization', [actor('a', 1, **sup, components={'Body': {'scheduled': [[1, 0]]}, 'Organization': {}})])
    add('body_rates', [actor('nan', 1, def_2a8=NAN, components={'Body': {'scheduled': [[1, 0]]}}),
                       actor('inf', 2, def_2a8=INF, components={'Body': {}}),
                       actor('den', 3, def_2a8=DEN, components={'Body': {}}),
                       actor('neg0', 4, def_2a8=NEG0, components={'Body': {'scheduled': [[1, 0]]}}),
                       actor('neg', 5, def_2a8=f32(-2.0), components={'Body': {}})], random=[])
    add('body_element_and_link', [
        actor('in_org', 1, **sup, components={'Body': {'scheduled': [[1, 0]]}, 'Element': {'organization': 'org'}}),
        actor('org', 2, components={'Organization': {}}),
        actor('loose_element', 3, **sup, components={'Body': {}, 'Element': {}}),
        actor('linked', 4, **sup, word_16c=gid(1, 2), components={'Body': {'scheduled': [[1, 0]]}}),
        actor('stale_link', 5, **sup, word_16c=gid(2, 2), components={'Body': {}}),
        actor('destroyed_link', 6, **sup, word_16c=gid(1, 7), components={'Body': {}}),
        actor('dead', 7, flags8=1)])
    # Properties / Supply 581260
    add('properties_and_supply', [
        actor('a', 1, components={'Properties': {}, 'Supply': {'scheduled': [[0, 5]]}}),
        actor('b', 2, components={'Properties': {'scheduled': [[3, 0]]}, 'Supply': {}})])
    # Wall 58CB40
    add('wall_old_world', [actor('flag', 1, def_42c=1, components={'Wall': {'scheduled': [[1, 0], [3, 2]]}}),
                           actor('noflag', 2, components={'Wall': {'scheduled': [[1, 0]]}})], world_version=25)
    add('wall_current_world', [actor('flag', 1, def_42c=1, components={'Wall': {'scheduled': [[3, 2]]}})], world_version=26)
    # Denizen 569F80
    add('denizen_commands', [
        actor('den', 1, components={'Denizen': {'commands': [
            {'label': 'c1', 'refcount': 1, 'members': [gid(1, 2), gid(1, 3), gid(2, 4), gid(1, 5), gid(1, 2)]},
            None,
            {'label': 'c2', 'refcount': 3, 'members': [gid(1, 3)]}]}}),
        actor('m1', 2, components={'Mind': {}}), actor('m2', 3, components={'Mind': {}}),
        actor('m3', 4, components={'Mind': {}}), actor('nomind', 5)])
    # Defender 5676B0 / 567140
    add('defender_active_schedules', [actor('d', 1, components={'Defender': {'word_1c': 1, 'scheduled': [[3, 0]]}}),
                                      actor('e', 2, components={'Defender': {'word_1c': 7, 'scheduled': [[0, 0]]}}),
                                      actor('f', 3, components={'Defender': {'word_1c': 1}})])
    add('defender_entries', [
        actor('d', 1, components={'Defender': {'word_1c': 1, 'entries': [
            [gid(1, 9), gid(1, 2)],     # dead key, live value: value's +118 loses owner org id
            [gid(1, 9), gid(1, 9)],     # dead key, dead value
            [gid(1, 3), gid(1, 9)],     # live key, dead value -> org_of(key)
            [gid(1, 3), gid(1, 2)],     # live key, live value
            [gid(1, 4), 0],             # live key (in org) -> its organization
            [gid(2, 2), gid(1, 5)]]}}), # stale key, live value without the id
        actor('y', 2, lists={'0x118': [gid(1, 6), gid(1, 1), gid(1, 1)]}),
        actor('k', 3), actor('member', 4, components={'Element': {'organization': 'org'}}),
        actor('z', 5, lists={'0x118': [gid(1, 7)]}), actor('org', 6, components={'Organization': {}}),
        actor('x', 7)])
    add('defender_owner_in_organization', [
        actor('d', 1, components={'Defender': {'word_1c': 1, 'entries': [[gid(1, 9), gid(1, 2)]]},
                                  'Element': {'organization': 'org'}}),
        actor('y', 2, lists={'0x118': [gid(1, 1), gid(1, 3)]}),
        actor('org', 3, components={'Organization': {}})])
    add('defender_idle_abilities', [
        actor('d', 1, components={'Defender': {'word_1c': 0, 'scheduled': [[0, 0], [3, 0]],
                                               'abilities': [gid(1, 2), gid(1, 9), gid(1, 4), gid(1, 2), 0, gid(1, 6)],
                                               'entries': [[gid(1, 6), gid(1, 5)], [gid(1, 2), gid(1, 3)],
                                                           [gid(1, 2), gid(1, 5)]]}}),
        actor('t', 2), actor('v', 3, lists={'0x118': [gid(1, 1), gid(1, 1)]}),
        {'label': 'obj', 'class': 'Object', 'slot': 4, 'gen': 1},
        actor('w', 5, lists={'0x118': [gid(1, 8), gid(1, 1)]}), actor('t2', 6)])
    # Hq 5736B0 / 4E2640
    add('hq_allocates', [
        actor('hq1', 1, e8='k', components={'Hq': {'index': -1, 'items': 3}}),
        actor('hq2', 2, e8='k', components={'Hq': {'index': -1}}),
        actor('hq3', 3, e8='k', components={'Hq': {'index': 2, 'items': 1}}),
        actor('hq4', 4, e8='k', components={'Hq': {'index': -1}}),
        {'label': 'k', 'class': 'Kingdom', 'slot': 6, 'gen': 1, 'hq_slots': 4, 'hq_taken': [1]}],
        random=[9, 6, 0x80000003])
    add('hq_exhausted_and_missing', [
        actor('hq1', 1, e8='full', components={'Hq': {'index': -1}}),
        actor('hq2', 2, e8='nofaction', components={'Hq': {'index': -1}}),
        actor('hq3', 3, components={'Hq': {'index': -1, 'items': 2}}),
        actor('hq4', 4, e8='zero', components={'Hq': {'index': -1}}),
        {'label': 'full', 'class': 'Kingdom', 'slot': 5, 'gen': 1, 'hq_slots': 2, 'hq_taken': [0, 1]},
        {'label': 'nofaction', 'class': 'Kingdom', 'slot': 6, 'gen': 1},
        {'label': 'zero', 'class': 'Kingdom', 'slot': 7, 'gen': 1, 'hq_slots': 0}],
        random=[5])
    # Kingdom 4E4220
    add('kingdom_groups_and_items', [
        {'label': 'k', 'class': 'Kingdom', 'slot': 1, 'gen': 1,
         'lists': {'0x118': [gid(1, 2), gid(1, 9)]},
         'groups': [[gid(1, 9), gid(1, 2), gid(2, 3), gid(1, 3)], [gid(1, 9)], [], [gid(1, 4), gid(3, 4)], [gid(1, 9), gid(5, 9)]],
         'recruit_items': [
             {'label': 'i_done', 'name': 'usa_inf', 'byte_8': 1, 'organization': gid(1, 9)},
             {'label': 'i_none', 'name': 'usa_tank', 'organization': 0},
             {'label': 'i_live', 'name': 'usa_art', 'organization': gid(1, 2)},
             {'label': 'i_dead', 'name': 'usa_jeep', 'organization': gid(1, 9)},
             {'label': 'i_destroyed_k', 'name': 'usa_reg', 'organization': gid(1, 5)},
             {'label': 'i_destroyed2', 'name': 'usa_hq', 'organization': gid(1, 6)}],
         'counter_224': 4, 'hq_slots': 3},
        actor('a2', 2), actor('a3', 3), actor('a4', 4),
        actor('reg', 5, flags8=1, e8='k2'),
        actor('lone', 6, flags8=1),
        {'label': 'k2', 'class': 'Kingdom', 'slot': 7, 'gen': 1, 'counter_224': 10}])
    add('kingdom_item_organization_undefined', [
        {'label': 'k', 'class': 'Kingdom', 'slot': 1, 'gen': 1,
         'recruit_items': [{'label': 'i0', 'name': 'ger_reg', 'organization': gid(1, 2)},
                           {'label': 'i1', 'name': 'ger_inf', 'organization': gid(1, 9)}]},
        actor('reg', 2, flags8=1, e8='k', components={'Organization': {'recruit_slots': 3}})])
    # virtual +12C: 4ECC50 / 4CFF70
    add('required_lost_memory_and_zone', [
        {'label': 'k0', 'class': 'Kingdom', 'slot': 1, 'gen': 1, 'index_18c': 0},
        {'label': 'k1', 'class': 'Kingdom', 'slot': 2, 'gen': 1, 'index_18c': 1},
        {'label': 'k2', 'class': 'Kingdom', 'slot': 3, 'gen': 1, 'index_18c': 2, 'word_190': 1},
        {'label': 'k3', 'class': 'Kingdom', 'slot': 4, 'gen': 1, 'index_18c': 3},
        {'label': 'obj', 'class': 'Object', 'slot': 5, 'gen': 1, 'required': gid(1, 30), 'def_1c': 0x400 | 0x55,
         'flags8': 0x40000003, 'memory_count': 2, 'memory': {'0': 'm0', '1': 'm1', '2': 'm2', '3': 'm3'}},
        actor('farm', 6, required=gid(4, 6), def_1c=0x300, def_34=1, flags8=0x40000000, zones=9,
              memory_count=1, memory={'1': 'f1'}),
        actor('nozone', 7, required=gid(4, 7), def_34=0, zones=0, flags8=0)],
        world_kingdoms=['k0', None, 'k1', 'k2', 'k3'], ui_agent=1)
    add('required_lost_no_ui_no_memory', [
        {'label': 'k0', 'class': 'Kingdom', 'slot': 1, 'gen': 1, 'index_18c': 0},
        {'label': 'obj', 'class': 'Object', 'slot': 2, 'gen': 1, 'required': gid(1, 30), 'def_1c': 0x400,
         'memory_count': 0, 'memory': {'0': 'm0'}},
        {'label': 'obj2', 'class': 'Object', 'slot': 3, 'gen': 1, 'required': gid(1, 30),
         'memory_count': 5, 'memory': {'0': 'n0'}}],
        world_kingdoms=['k0', 'k0'], ui_agent=0)
    add('required_lost_zone_missing', [
        actor('farm', 1, required=gid(4, 6), def_34=1, zones=2)], world_kingdoms=[])
    # Settlement 586BC0(0)
    add('settlement_creates_things', [
        {'label': 'k', 'class': 'Kingdom', 'slot': 1, 'gen': 1},
        actor('village', 2, e8='k', pos=[f32(1000.0), f32(-250.5), f32(12.0)],
              settlement_entries=[[0, f32(10.0), f32(-3.25), f32(0.0), f32(1.0)],
                                  [2, f32(0.1), f32(0.2), f32(-1.0), f32(0.0)],
                                  [1, f32(1e-7), f32(3e7), f32(0.5), f32(0.5)]],
              settlement_defs=['house', 'barn', 'well'],
              components={'Settlement': {'count': 3}})],
        created=[gid(3, 40), None, gid(1, 41)])
    add('settlement_no_owner_empty', [
        actor('village', 1, components={'Settlement': {'count': 0}}),
        actor('village2', 2, settlement_entries=[[1, f32(4.0), f32(4.0), 0, 0]],
              settlement_defs=['house', 'barn'], components={'Settlement': {'count': 1}})],
        created=[gid(1, 50)])
    add('settlement_null_definition', [
        actor('village', 1, settlement_entries=[[0, 0, 0, 0, 0], [3, 0, 0, 0, 0]],
              settlement_defs=['house'], components={'Settlement': {'count': 2}})],
        created=[gid(1, 60)])
    # GGroup 4DDFA0
    add('group_targets', [
        {'label': 'g_pos', 'class': 'Group', 'slot': 1, 'gen': 1, 'zones': 9,
         'target': {'flags': 4, 'object': 0, 'xy': [f32(-5.0), f32(2000.0)]}},
        {'label': 'g_obj', 'class': 'Group', 'slot': 2, 'gen': 1, 'zones': 9, 'radius_1b4': f32(7.5),
         'target': {'flags': 1, 'object': gid(1, 4), 'xy': [0, 0]}},
        {'label': 'g_dead', 'class': 'Group', 'slot': 3, 'gen': 1, 'zones': 9,
         'target': {'flags': 1, 'object': gid(1, 9), 'xy': [0, 0]}},
        actor('tgt', 4, pos=[f32(1500.0), f32(30.0), 0]),
        {'label': 'g_none', 'class': 'Group', 'slot': 5, 'gen': 1, 'zones': 9},
        {'label': 'g_flags0', 'class': 'Group', 'slot': 6, 'gen': 1, 'zones': 9,
         'target': {'flags': 0x10, 'object': gid(1, 4), 'xy': [0, 0]}}])
    add('group_zone_missing', [
        {'label': 'g', 'class': 'Group', 'slot': 1, 'gen': 1, 'zones': 7,
         'target': {'flags': 5, 'object': 0, 'xy': [f32(1.0), f32(2.0)]}}])
    # Ordering: RNG draws and queue adds across slots, gaps, objects beyond max_slot.
    add('pass_order_and_bounds', [
        actor('a', 2, e8='k', components={'Hq': {'index': -1}, 'Properties': {}}),
        {'label': 'gap_obj', 'class': 'Object', 'slot': 4, 'gen': 7, 'required': gid(1, 30)},
        actor('b', 6, **sup, components={'Body': {}, 'Supply': {}}),
        {'label': 'k', 'class': 'Kingdom', 'slot': 8, 'gen': 1, 'hq_slots': 5},
        actor('beyond', 12, components={'Properties': {}})],
        max_slot=10, random=[7])
    add('mixed_tutorial_like', [
        {'label': 'k', 'class': 'Kingdom', 'slot': 1, 'gen': 1, 'hq_slots': 2,
         'components': {'Economy': {}}},
        actor('depot', 3, e8='k', def_2a8=ONE, components={
            'Body': {}, 'Mind': {}, 'Properties': {}, 'Economy': {}, 'Supply': {}, 'Structure': {},
            'Construction': {}, 'Recruit': {}, 'MobileDeployment': {}, 'Defender': {'word_1c': 0}, 'Research': {},
            'Detection': {}, 'Lair': {}, 'City': {}, 'Hq': {'index': -1}}),
        actor('corps', 4, e8='k', components={'Mind': {}, 'Performer': {}, 'Properties': {}, 'Organization': {},
                                              'Experience': {}, 'Detection': {}}),
        actor('unit', 5, e8='k', def_2a8=ONE, components={'Body': {}, 'Character': {}, 'Mover': {}, 'Mind': {},
                                                          'Performer': {}, 'Properties': {}, 'Element': {'organization': 'corps'},
                                                          'Experience': {}, 'Detection': {}}),
        actor('airfield', 6, e8='k', components={'Denizen': {'commands': []}, 'Body': {}, 'Properties': {}})],
        random=[1, 2, 3])
    # The only float operation (561B40 fcomp of a loaded binary32 with 0.0)
    # does not depend on the x87 precision control: PC24/PC64 replicas.
    for base in [r for r in rows if r['name'] in ('body_rates', 'mixed_tutorial_like', 'required_lost_memory_and_zone',
                                                  'settlement_creates_things')]:
        for pc in ('Single24', 'Extended64'):
            rows.append({**copy.deepcopy(base), 'name': base['name'] + '_' + pc, 'precision': pc})
    return rows


def updatable_has_cases():
    """50B220 run natively on hand-built queue entries (0x1C bytes: +0
    Updatable, +4 bucket prev link, +8 bucket next, +C owner prev link, +10
    owner next, +14 data, +18 queue byte; 50ABC0/50AC60 layout)."""
    E = lambda owner, q, d: {'owner': owner, 'queue': q, 'data': d}
    rows = [
        ('empty', {}, [], {'A': []}, 'A', 1, 0),
        ('head_matches', {'a1': E('A', 1, 0)}, [['a1']], {'A': ['a1']}, 'A', 1, 0),
        ('own_second_entry_other_bucket', {'a1': E('A', 1, 5), 'a2': E('A', 1, 0)}, [['a1'], ['a2']],
         {'A': ['a1', 'a2']}, 'A', 1, 0),
        ('other_owner_after_head_in_bucket', {'a1': E('A', 1, 5), 'b1': E('B', 1, 0)}, [['a1', 'b1']],
         {'A': ['a1'], 'B': ['b1']}, 'A', 1, 0),
        ('other_owner_before_head_in_bucket', {'b1': E('B', 1, 0), 'a1': E('A', 1, 5)}, [['b1', 'a1']],
         {'A': ['a1'], 'B': ['b1']}, 'A', 1, 0),
        ('queue_byte_differs', {'a1': E('A', 3, 0)}, [['a1']], {'A': ['a1']}, 'A', 1, 0),
        ('deep_chain', {'a1': E('A', 0, 0), 'b1': E('B', 0, 7), 'c1': E('C', 0, 0)}, [['a1', 'b1', 'c1']],
         {'A': ['a1'], 'B': ['b1'], 'C': ['c1']}, 'A', 0, 0),
        ('own_older_entry_same_bucket', {'a2': E('A', 1, 9), 'x': E('B', 1, 9), 'a1': E('A', 1, 0)},
         [['a2', 'x', 'a1']], {'A': ['a2', 'a1'], 'B': ['x']}, 'A', 1, 0),
    ]
    out = []
    for name, entries, buckets, owners, who, q, d in rows:
        m = Machine(b'')
        upd = {o: m.alloc(0x10) for o in owners}
        addr = {k: m.alloc(0x1c) for k in entries}
        for k, e in entries.items():
            m.w(addr[k], upd[e['owner']])
            m.w(addr[k] + 0x14, e['data'])
            m.wb(addr[k] + 0x18, e['queue'])
        for chain in buckets:
            head = m.alloc(4)
            for i, k in enumerate(chain):
                m.w(addr[k] + 4, head if i == 0 else addr[chain[i - 1]] + 8)
                m.w(addr[k] + 8, addr[chain[i + 1]] if i + 1 < len(chain) else 0)
            m.w(head, addr[chain[0]] if chain else 0)
        for o, chain in owners.items():
            m.w(upd[o] + 4, addr[chain[0]] if chain else 0)
            for i, k in enumerate(chain):
                m.w(addr[k] + 0xc, upd[o] + 4 if i == 0 else addr[chain[i - 1]] + 0x10)
                m.w(addr[k] + 0x10, addr[chain[i + 1]] if i + 1 < len(chain) else 0)
        r = m.call(0x50b220, upd[who], [q, d]) & 0xff
        out.append({'name': name, 'entries': entries, 'buckets': buckets, 'owners': owners,
                    'updatable': who, 'queue': q, 'data': d, 'native_result': bool(r)})
    return out


def build_links(m, entries, buckets, owners):
    """Queue entries with both native link pairs; returns (updatables, entry
    addresses, bucket heads)."""
    upd = {o: m.alloc(0x10) for o in owners}
    addr = {k: m.alloc(0x1c) for k in entries}
    for k, e in entries.items():
        m.w(addr[k], upd[e['owner']])
        m.w(addr[k] + 0x14, e['data'])
        m.wb(addr[k] + 0x18, e['queue'])
    heads = []
    for chain in buckets:
        head = m.alloc(4)
        heads.append(head)
        for i, k in enumerate(chain):
            m.w(addr[k] + 4, head if i == 0 else addr[chain[i - 1]] + 8)
            m.w(addr[k] + 8, addr[chain[i + 1]] if i + 1 < len(chain) else 0)
        m.w(head, addr[chain[0]] if chain else 0)
    for o, chain in owners.items():
        m.w(upd[o] + 4, addr[chain[0]] if chain else 0)
        for i, k in enumerate(chain):
            m.w(addr[k] + 0xc, upd[o] + 4 if i == 0 else addr[chain[i - 1]] + 0x10)
            m.w(addr[k] + 0x10, addr[chain[i + 1]] if i + 1 < len(chain) else 0)
    return upd, addr, heads


def updatable_remove_cases():
    """50B1C0 run natively (50ADE0 and the world cursor fix 4F1A70 natively;
    free 6F7430 is a no-op seam): every entry of the Updatable's own list
    (+4, next +10) with byte +18 == queue and +14 == data is unlinked from
    both lists; the count is returned. The world cursor (+228) moves to the
    removed entry's bucket successor."""
    E = lambda owner, q, d: {'owner': owner, 'queue': q, 'data': d}
    rows = [
        ('none', {'a1': E('A', 1, 5)}, [['a1']], {'A': ['a1']}, 'A', 1, 0, None),
        ('all_three', {'a1': E('A', 1, 0), 'a2': E('A', 1, 0), 'b1': E('B', 1, 0), 'a3': E('A', 1, 0)},
         [['a1', 'b1'], ['a2'], ['a3']], {'A': ['a1', 'a2', 'a3'], 'B': ['b1']}, 'A', 1, 0, None),
        ('middle_of_bucket', {'b1': E('B', 1, 0), 'a1': E('A', 1, 0), 'c1': E('C', 1, 0), 'a2': E('A', 3, 0)},
         [['b1', 'a1', 'c1'], ['a2']], {'A': ['a2', 'a1'], 'B': ['b1'], 'C': ['c1']}, 'A', 1, 0, None),
        ('queue_byte_filter', {'a1': E('A', 0, 0), 'a2': E('A', 1, 0), 'a3': E('A', 3, 0)},
         [['a1'], ['a2'], ['a3']], {'A': ['a1', 'a2', 'a3']}, 'A', 3, 0, None),
        ('cursor_on_removed', {'a1': E('A', 1, 0), 'b1': E('B', 1, 0)}, [['a1', 'b1']],
         {'A': ['a1'], 'B': ['b1']}, 'A', 1, 0, 'a1'),
        ('cursor_elsewhere', {'a1': E('A', 1, 0), 'b1': E('B', 1, 0)}, [['b1', 'a1']],
         {'A': ['a1'], 'B': ['b1']}, 'A', 1, 0, 'b1'),
        ('other_owner_same_values_kept', {'a1': E('A', 1, 7), 'b1': E('B', 1, 7)}, [['a1', 'b1']],
         {'A': ['a1'], 'B': ['b1']}, 'B', 1, 7, None),
    ]
    out = []
    for name, entries, buckets, owners, who, q, d, cursor in rows:
        m = Machine(b'')
        world = m.alloc(0x300)
        m.w(G_WORLD, world)
        upd, addr, heads = build_links(m, entries, buckets, owners)
        m.w(world + 0x228, addr[cursor] if cursor else 0)
        label = {v: k for k, v in addr.items()}
        n = m.call(0x50b1c0, upd[who], [q, d])

        def chain(p, nxt):
            r = []
            while p:
                r.append(label[p])
                p = m.r(p + nxt)
            return r
        cur = m.r(world + 0x228)
        out.append({'name': name, 'entries': entries, 'buckets': buckets, 'owners': owners, 'updatable': who,
                    'queue': q, 'data': d, 'cursor': cursor, 'native_removed': n,
                    'buckets_after': [chain(m.r(h), 8) for h in heads],
                    'owners_after': {o: chain(m.r(u + 4), 0x10) for o, u in upd.items()},
                    'cursor_after': label.get(cur) if cur else None})
    return out


def main():
    out = []
    for c in cases():
        m = PostLoadMachine(c)
        out.append({'input': c, 'native': m.run()})
    has_rows = updatable_has_cases()
    remove_rows = updatable_remove_cases()
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'machine_sha256': H((HERE / 'native_machine.py').read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__,
           'visited_instructions': len(VISITED), 'cases': out, 'updatable_has_50b220': has_rows,
           'updatable_remove_50b1c0': remove_rows}
    path = HERE / 'native-post-load.json'
    path.write_text(json.dumps(doc, indent=1), encoding='utf-8')
    for row in out:
        n = row['native']
        print(row['input']['name'], n['outcome'], [x['seam'] for x in n['calls']])
    print([(r['name'], r['native_result']) for r in has_rows])
    for r in remove_rows:
        print(r['name'], r['native_removed'], r['buckets_after'], r['owners_after'], r['cursor_after'])
    print(json.dumps({'cases': len(out), 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
