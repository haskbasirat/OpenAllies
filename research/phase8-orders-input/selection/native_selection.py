"""Original AA.exe battle selection manager (game UIWorldState) under Unicorn.

The REAL game UIWorldState vtable 0xA61E40 (base 0x9FD648) is installed on a
fresh object with its 0x438E10/0x640640 constructor field values; VWorld
(B36538), the UI manager (B3653C, +0xC4 = an object with the REAL default
game state vtable 0xA57530) and the B364C0 handle table are explicit test
objects. View things and actors are test objects whose virtual slots are
recorded Python seams returning the scenario's values; every other executed
instruction is original. Explicit seams (all recorded):
  0x6F7810 / 0x6F7430   allocator
  0x436AD0 / 0x436B90 / 0x436C50   Selection Add/Remove/Clear request emitters
  0x42A590              voice playback (selection acknowledgement 0x436850
                        itself runs: session B36550+0xE4 = 0, UVar148
                        UnitAcknowledgement = 1)
  0x437EB0              command-mask refresh (HUD; not part of this proof)
  0x5C0E50 / 0x5C0E10   local viewing kingdom / local player
  0x43D240              FindObjectAt (scripted hits; ported/proved separately)
  0x436DD0              box frustum: redirected to the ORIGINAL 0x4367A0
  0x454B50              per-object frustum test (scripted)
  0x439010 / 0x438240   UI group constructor / destructor (CString display
                        text and digits are presentation; name +0 kept)
  0x453470              group floater refresh
  0x438180              KKC_SaveGroupOrder request emitter
Each case runs at Single24/Double53/Extended64 in the original image and the
executed bytes are checked against the v14 image.
Writes native-selection.json next to this file.
"""
from pathlib import Path
import sys, json, random, copy
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm
from native_machine import Machine, f32, from_f32, PRECISIONS
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EIP, UC_X86_REG_ECX

GAME_UIWORLD_VT = 0xA61E40
DEFAULT_STATE_VT = 0xA57530
KINGDOM = 0x0BADC0DE
PLAYER = 0x00501234

class Native:
    def __init__(self, precision, case):
        self.case = case
        m = self.m = Machine(precision)
        self.events = []
        self.heap_free = []
        m.seam(0x6F7810, self.alloc)
        m.seam(0x6F7430, self.free)
        m.seam(0x436AD0, lambda m: self.request('add', pop=4))
        m.seam(0x436B90, lambda m: self.request('remove', pop=4))
        m.seam(0x436C50, lambda m: self.request('clear', pop=4))
        m.seam(0x42A590, self.voice)
        m.seam(0x437EB0, self.masks)
        m.seam(0x5C0E50, lambda m: m.ret(KINGDOM, 0))
        m.seam(0x5C0E10, lambda m: m.ret(PLAYER, 0))
        m.seam(0x43D240, self.find_object_at)
        m.seam(0x436DD0, self.box_frustum)
        m.seam(0x454B50, self.frustum_test)
        m.seam(0x439010, self.group_ctor)
        m.seam(0x438240, lambda m: m.ret(0, 0))
        m.seam(0x453470, self.floater)
        m.seam(0x438180, self.save_group_request)
        # CString manager: vt+0x10 returns itself (shared copies), vt+4 frees.
        self.string_manager = m.alloc(0x10)
        mvt = m.alloc(0x40)
        m.w(mvt + 0x10, m.stub(lambda m: m.ret(m.ecx(), 0)))
        m.w(mvt + 4, m.stub(lambda m: m.ret(0, 4)))
        m.w(self.string_manager, mvt)
        session = m.alloc(0x200)
        m.w(0xB36550, session)
        uvars = m.alloc(0x400)
        on = m.alloc(4)
        m.wb(on, 1)
        m.w(uvars + 0x250, on)
        m.w(0xB39940, uvars)
        # handle table
        self.table = m.alloc(0x40000)
        m.w(0xB364C0, self.table)
        # views and actors
        self.views = []
        self.actors = {}
        for i, v in enumerate(case['views']):
            self.views.append(m.alloc(0x160))
        self.view_index = {a: i for i, a in enumerate(self.views)}
        for i, v in enumerate(case['views']):
            self.build_view(i, v)
        # VGrid list (+0x100) in case order, linked through +8
        grid = m.alloc(0x110)
        order = case.get('grid', list(range(len(self.views))))
        head = 0
        for i in reversed(order):
            m.w(self.views[i] + 8, head)
            head = self.views[i]
        m.w(grid + 0x100, head)
        # VWorld
        self.vworld = m.alloc(0x200)
        m.w(0xB36538, self.vworld)
        m.wb(self.vworld + 0xAD, case.get('vworld_ad', 1))
        m.w(self.vworld + 0x94, grid)
        self.sink140 = m.alloc(0x100)
        m.w(self.vworld + 0x140, self.sink140)
        # UI manager with the real default game state at +0xC4
        self.ui = m.alloc(0x200)
        m.w(0xB3653C, self.ui)
        state = m.alloc(0x10)
        m.w(state, DEFAULT_STATE_VT)
        m.w(self.ui + 0xC4, state)
        # UIWorldState (0x438E10 + 0x640640 stores)
        u = self.uiw = m.alloc(0x60)
        m.w(u, GAME_UIWORLD_VT)
        self.brush = m.alloc(0x30)
        m.w(u + 0x30, self.brush)
        m.w(u + 0x3C, 1)
        self.wildcard = m.alloc(0x24)
        m.w(u + 0x44, self.wildcard)
        m.w(self.vworld + 0x13C, u)
        self.groups = []
        for g in case.get('groups', []):
            self.add_group(g)

    # -- memory --------------------------------------------------------
    def alloc(self, m):
        size = m.arg(1)
        p = m.alloc(max(size, 4))
        m.ret(p, 0)

    def free(self, m):
        self.heap_free.append(m.arg(1))
        m.ret(0, 0)

    def handle(self, index, target):
        gen = 0x11 + index
        idx = 0x40 + index
        self.m.u.mem_write(self.table + 4 + idx * 2, gen.to_bytes(2, 'little'))
        self.m.w(self.table + 0x20004 + idx * 4, target)
        return gen << 16 | idx

    def vtable(self, slots):
        """slots: {offset: fn(m, obj)}; unspecified slots trap."""
        vt = self.m.alloc(0x200)
        for off in range(0, 0x200, 4):
            fn = slots.get(off)
            if fn is None:
                def trap(m, off=off):
                    raise AssertionError('unexpected virtual +%#x on %#x' % (off, m.ecx()))
                self.m.w(vt + off, self.m.stub(trap))
            else:
                self.m.w(vt + off, self.m.stub(fn))
        return vt

    def build_view(self, i, v):
        m = self.m
        p = self.views[i]
        def flag(name, pop):
            def fn(m, name=name, pop=pop):
                self.events.append(['view', name, self.view_index[m.ecx()]])
                m.ret(1 if self.case['views'][self.view_index[m.ecx()]][name] else 0, pop)
            return fn
        vt = self.vtable({0x78: flag('class_78', 0), 0x84: flag('class_84', 0), 0x7C: flag('class_7c', 0),
                          0x80: flag('class_80', 0), 0x9C: flag('pickable', 0), 0x50: flag('virtual_50', 4),
                          0x54: flag('virtual_54', 4)})
        m.w(p, vt)
        m.w(p + 0x10, 0x7000 + v['field10'])
        m.w(p + 0x18, v.get('proximity', 2))
        for k in range(3):
            m.w(p + 0x38 + 4 * k, v['pos'][k])
        m.w(p + 0x54, v['flags'])
        a = v.get('actor')
        if a is None:
            m.w(p + 0x14, 0)
            return
        actor = m.alloc(0x200)
        self.actors[actor] = i
        m.w(p + 0x14, actor)
        def aflag(name, pop):
            def fn(m, name=name, pop=pop):
                idx = self.actors[m.ecx()]
                self.events.append(['actor', name, idx])
                m.ret(1 if self.case['views'][idx]['actor'][name] else 0, pop)
            return fn
        avt = self.vtable({0xB4: aflag('valid_b4', 0), 0x20: aflag('virtual_20', 4), 0x58: aflag('virtual_58', 8),
                           0x84: aflag('virtual_84', 4)})
        m.w(actor, avt)
        m.w(actor + 8, a['flags8'])
        m.w(actor + 0x70, 1 if a['f70'] else 0)
        m.w(actor + 0x90, 1 if a['f90'] else 0)
        m.w(actor + 0x100, a.get('f100', 0))
        m.w(actor + 0x104, a.get('f104', 0))
        m.w(actor + 0x0C, p if a.get('own_view', True) else 0)

    def link_actors(self):
        """Second pass: links need every view/actor allocated."""
        m = self.m
        for i, v in enumerate(self.case['views']):
            a = v.get('actor')
            if a is None:
                continue
            actor = m.r(self.views[i] + 0x14)
            # +0x16C organization link (0x437650)
            if a.get('link16c') is not None:
                rec = m.alloc(0x20)
                m.w(rec + 0xC, self.views[a['link16c']])
                m.w(actor + 0x16C, self.handle(100 + i, rec))
            else:
                m.w(actor + 0x16C, 0x00ff00ff)  # stale handle: generation mismatch
            # +0x94 -> +0x34 link (0x640B40 / 0x5E05D0)
            l94 = a.get('link94')
            if l94 is not None:
                comp = m.alloc(0x40)
                rec = m.alloc(0x20)
                m.w(rec + 0xC, self.views[l94] if l94 >= 0 else 0)
                m.w(comp + 0x34, self.handle(200 + i, rec))
                m.w(actor + 0x94, comp)
            # member organization (+0x7C / +0x80 -> +0x10)
            org = a.get('org')
            if a.get('f7c'):
                # +0x7C organization component: +0x40 -> record, +0xA4 = 0 (no
                # voiced record; 0x436600/0x436850 keep the view itself).
                comp = m.alloc(0x50)
                rec = m.alloc(0xB0)
                m.w(comp + 0x40, rec)
                m.w(actor + 0x7C, comp)
            else:
                m.w(actor + 0x7C, 0)
            if org is not None:
                comp = m.alloc(0x20)
                oactor = m.r(self.views[org] + 0x14)
                m.w(comp + 0x10, oactor)
                m.w(actor + 0x80, comp)

    def wide(self, text):
        data = text.encode('utf-16-le') + b'\0\0'
        p = self.m.alloc(len(data))
        self.m.u.mem_write(p, data)
        return p

    def cstring(self, text):
        """An MFC CStringT: header {manager, length, allocated, refs} + data."""
        data = text.encode('utf-16-le') + b'\0\0'
        h = self.m.alloc(0x10 + len(data))
        self.m.w(h, self.string_manager)
        self.m.w(h + 4, len(text))
        self.m.w(h + 8, len(text))
        self.m.w(h + 0xC, 1)
        self.m.u.mem_write(h + 0x10, data)
        obj = self.m.alloc(4)
        self.m.w(obj, h + 0x10)
        return obj

    def read_wide(self, p):
        out = []
        while True:
            c = self.m.u.mem_read(p + 2 * len(out), 2)
            c = c[0] | c[1] << 8
            if c == 0:
                return ''.join(out)
            out.append(chr(c))

    def group_ctor(self, m):
        g = m.ecx()
        name = m.r(m.arg(1))
        m.w(g, self.wide(self.read_wide(name)))
        m.w(g + 0xC, m.alloc(4))
        m.w(g + 0x10, 0)
        m.w(g + 0x14, 1)
        m.wb(g + 0x18, 0)
        m.w(g + 0x1C, 0)
        m.wb(g + 0x20, 0)
        m.ret(g, 4)

    def floater(self, m):
        self.events.append(['floater', self.view_index[m.ecx()]])
        m.ret(0, 0)

    def save_group_request(self, m):
        order = m.ecx()
        self.events.append(['request', 'save_group', self.read_wide(m.r(order + 4)), m.arg(1) == PLAYER])
        m.ret(0, 4)

    def add_group(self, spec):
        m = self.m
        members = spec['members']
        g = m.alloc(0x24)
        m.w(g, self.wide(spec['name']))
        m.wb(g + 0x18, 1 if spec.get('byte_18') else 0)
        for i in members:
            m.w(self.views[i] + 0x150, g)
        ptrs = sorted(self.views[i] for i in members)
        arr = m.alloc(4 * max(len(ptrs), 1))
        for k, ptr in enumerate(ptrs):
            m.w(arr + 4 * k, ptr)
        m.w(g + 0xC, arr)
        m.w(g + 0x10, len(ptrs))
        m.w(g + 0x14, len(ptrs))
        self.groups.append(g)
        n = len(self.groups)
        garr = m.alloc(4 * n)
        for k, gp in enumerate(self.groups):
            m.w(garr + 4 * k, gp)
        m.w(self.uiw + 0x34, garr)
        m.w(self.uiw + 0x38, n)
        m.w(self.uiw + 0x3C, n)

    # -- seams ---------------------------------------------------------
    def actor_view(self, actor):
        return self.actors.get(actor, -1) if actor else None

    def request(self, kind, pop):
        m = self.m
        order = m.ecx()
        player = m.arg(1)
        if kind == 'clear':
            self.events.append(['request', 'clear', player == PLAYER])
        else:
            self.events.append(['request', kind, self.actor_view(m.r(order + 4)), player == PLAYER])
        m.ret(0, pop)

    def voice(self, m):
        self.events.append(['sound', self.actor_view(m.arg(1))])
        m.ret(0, 8)

    def masks(self, m):
        self.events.append(['masks'])
        m.ret(0, 0)

    def find_object_at(self, m):
        # thiscall(origin*, dx, dy, dz, al, hits*) ret 0x18
        # Scripted results in call order; an exhausted script hits nothing.
        spec = self.case['find'].pop(0) if self.case['find'] else {'hits': [], 'best': None}
        hits = m.arg(6)
        self.events.append(['find', m.arg(5) & 0xff, hits != 0])
        if hits:
            head = 0
            tail = 0
            for i in spec.get('hits', []):
                node = m.alloc(8)
                m.w(node, self.views[i])
                m.w(node + 4, head)
                head = node
                if tail == 0:
                    tail = node
            m.w(hits, head)
            m.w(hits + 4, tail)
        best = spec['best']
        m.ret(self.views[best] if best is not None else 0, 0x18)

    def box_frustum(self, m):
        esp = m.u.reg_read(UC_X86_REG_ESP)
        ret = m.r(esp)
        planes = m.alloc(0x40)
        m.w(esp + 4, ret)
        m.w(esp + 8, planes)
        m.u.reg_write(UC_X86_REG_ESP, esp + 4)
        m.u.reg_write(UC_X86_REG_EIP, 0x4367A0)

    def frustum_test(self, m):
        i = self.view_index[m.ecx()]
        self.events.append(['frustum', i])
        m.ret(1 if self.case['views'][i].get('inside') else 0, 4)

    # -- state ---------------------------------------------------------
    def nodes(self, head):
        out = []
        while head:
            out.append(self.view_index.get(self.m.r(head), -1))
            head = self.m.r(head + 4)
        return out

    def snapshot(self):
        m = self.m
        u = self.uiw
        garr, gcount = m.r(u + 0x34), m.r(u + 0x38)
        groups = []
        live = []
        for k in range(gcount):
            g = m.r(garr + 4 * k)
            live.append(g)
            arr, n = m.r(g + 0xC), m.r(g + 0x10)
            groups.append({'name': self.read_wide(m.r(g)), 'members': [self.view_index.get(m.r(arr + 4 * j), -1) for j in range(n)],
                           'byte_18': m.rb(g + 0x18) != 0})
        static = m.r(u + 0x40)
        if static == 0:
            static_ref = 'none'
        elif static == self.wildcard:
            static_ref = 'wildcard'
        else:
            static_ref = live.index(static)
        links = []
        for p in self.views:
            g = m.r(p + 0x150)
            links.append(live.index(g) if g in live else (None if g == 0 else 'stale'))
        return {
            'groups': groups,
            'links': links,
            'selected': self.nodes(m.r(u + 8)),
            'tail': self.view_index.get(m.r(m.r(u + 0xC)), -1) if m.r(u + 0xC) else None,
            'hovered': self.view_index.get(m.r(u + 0x10)) if m.r(u + 0x10) else None,
            'candidates': self.nodes(m.r(u + 0x14)),
            'changed_1c': m.rb(u + 0x1C),
            'static_40': static_ref,
            'class_50': m.r(u + 0x50),
            'class_54': m.r(u + 0x54),
            'flags': [m.r(p + 0x54) for p in self.views],
            'brush_8': m.rb(self.brush + 8),
            'vworld_140_40': m.rb(self.sink140 + 0x40),
        }

    def vcall(self, slot, args, pop):
        vt_entry = self.m.r(GAME_UIWORLD_VT + slot)
        return self.m.call(vt_entry, ecx=self.uiw, args=args, pop=pop)

    def vec(self, words):
        p = self.m.alloc(12)
        for k, w in enumerate(words):
            self.m.w(p + 4 * k, w)
        return p

    def list_of(self, items):
        head = 0
        for i in reversed(items):
            node = self.m.alloc(8)
            self.m.w(node, self.views[i])
            self.m.w(node + 4, head)
            head = node
        return self.vec([head, 0, 0])

    def event(self, spec):
        p = self.m.alloc(0x30)
        for k, w in enumerate(spec['world']):
            self.m.w(p + 0xC + 4 * k, w)
        return p

    def run(self):
        self.link_actors()
        m = self.m
        out = []
        for op in self.case['ops']:
            self.events = []
            kind = op[0]
            result = None
            if kind == 'select':
                self.vcall(0x04, (self.views[op[1]], op[2]), 8)
            elif kind == 'clear':
                self.vcall(0x08, (op[1],), 4)
            elif kind == 'deselect':
                self.vcall(0x0C, (self.views[op[1]],), 4)
            elif kind == 'add':
                self.vcall(0x14, (self.views[op[1]], op[2]), 8)
            elif kind == 'click':
                self.vcall(0x3C, (self.vec(op[1]), self.vec(op[2]), op[3]), 0xC)
            elif kind == 'click_add':
                self.vcall(0x40, (self.vec(op[1]), self.vec(op[2]), op[3]), 0xC)
            elif kind == 'click_hits':
                hits = self.list_of(op[2])
                hit = self.views[op[3]] if op[3] is not None else 0
                self.vcall(0x50, (self.vec(op[1]), hits, hit), 0xC)
            elif kind == 'select_list':
                self.vcall(0x6C, (self.list_of(op[1]),), 4)
            elif kind == 'box':
                self.vcall(0x48, (self.event(op[1]), self.event(op[2])), 8)
            elif kind == 'compatible':
                result = self.vcall(0x64, (self.views[op[1]],), 4) & 0xff
            elif kind == 'pick_target':
                r = self.vcall(0x54, (self.views[op[1]],), 4)
                result = self.view_index.get(r) if r else None
            elif kind == 'select_owner':
                r = self.vcall(0x58, (self.views[op[1]],), 4)
                result = self.view_index.get(r) if r else None
            elif kind == 'selectable':
                result = self.vcall(0x10, (self.views[op[1]],), 4) & 1
            elif kind == 'class':
                result = m.call(0x6406C0, ecx=self.uiw, args=(self.views[op[1]], op[2]), pop=8)
            elif kind == 'filter':
                result = m.call(0x5C19E0, ecx=self.ui, args=(self.views[op[1]],), pop=4) & 0xff
            elif kind == 'choose':
                a = self.views[op[1]] if op[1] is not None else 0
                b = self.views[op[3]] if op[3] is not None else 0
                r = m.call(0x5C1970, ecx=self.ui, args=(a, op[2], b, op[4]), pop=0x10)
                result = self.view_index.get(r) if r else None
            elif kind == 'save_group':
                m.call(0x439E90, ecx=self.uiw, args=(self.cstring(op[1]),), pop=4)
            elif kind == 'select_group':
                m.call(0x4376C0, ecx=self.uiw, args=(self.cstring(op[1]),), pop=4)
            elif kind == 'hover':
                m.w(self.uiw + 0x10, self.views[op[1]] if op[1] is not None else 0)
            else:
                raise ValueError(kind)
            m.verify_v14()
            out.append({'op': op, 'result': result, 'events': self.events, 'state': self.snapshot()})
        assert not self.case['find'], 'unused FindObjectAt scripts'
        return out


# ---------------------------------------------------------------- cases ---
rng = random.Random(0x5e1ec7ed)

def view(field10, actor=None, flags=0, pos=None, **kw):
    v = {'class_78': False, 'class_84': False, 'class_7c': False, 'class_80': False, 'pickable': True,
         'virtual_50': True, 'virtual_54': False, 'field10': field10, 'flags': flags,
         'pos': pos or [f32(rng.uniform(-200, 200)), f32(rng.uniform(-200, 200)), f32(rng.uniform(0, 20))],
         'actor': actor}
    v.update(kw)
    return v

def actor(**kw):
    a = {'valid_b4': True, 'virtual_20': True, 'virtual_58': True, 'virtual_84': True, 'flags8': 0,
         'f70': False, 'f90': True, 'link16c': None, 'link94': None, 'org': None, 'f7c': False}
    a.update(kw)
    return a

def units(n, owned=True, same=1):
    # Organization-like actors (+0x70 set, not a member): the multi-select kind.
    return [view(same, actor(f70=True, f90=False), class_78=owned) for _ in range(n)]

CASES = []
def case(name, views, ops, **kw):
    c = {'name': name, 'views': views, 'ops': ops, 'find': kw.pop('find', [])}
    c.update(kw)
    CASES.append(c)

# Single selection, re-select, clear, deselect, sounds.
case('select-basic', units(4), [['select', 0, 1], ['select', 0, 1], ['select', 1, 0], ['clear', 0], ['select', 2, 1],
                                 ['deselect', 2], ['clear', 1]])
# Selection cap of six through vt+4.
case('cap-six', units(9), [['select', i, 0] for i in range(8)])
# Class gating: owned units (0x800) vs unowned (0x200/0x100), f90 kinds, no actor.
mixed = [view(1, actor(f90=False), class_78=True), view(2, actor(f90=False), class_78=True),
         view(3, actor(f90=True), class_78=True), view(4, actor(), class_7c=True), view(5, None, class_80=True),
         view(6, actor(virtual_20=False), class_78=True), view(7, actor(f70=True, f90=False), class_78=True),
         view(8, actor(f70=True, f90=False, flags8=1 << 29), class_84=True)]
case('class-gating', mixed, [['class', i, c] for i in range(len(mixed)) for c in (0, 0x804, 0x808, 0x820, 0x101, 0x210, 0x402, 0x840, 0x880)]
     + [['select', 0, 0], ['select', 3, 0], ['select', 1, 0], ['select', 2, 0], ['select', 4, 0], ['select', 5, 0],
        ['select', 6, 0], ['compatible', 7], ['select', 7, 0], ['clear', 0], ['select', 4, 0], ['select', 3, 0]])
# Owner mapping: member -> organization (0x4CE6C0), links 0x16C/0x94, hidden 0x4549C0.
org_views = [view(1, actor(f7c=True), class_78=True),                   # 0 organization
             view(1, actor(org=0), class_78=True),                      # 1 member of 0
             view(1, actor(org=0, link16c=0), class_78=True),           # 2 member linked to 0
             view(2, actor(link94=0), class_78=True),                   # 3 inside 0 (transport link)
             view(2, actor(link94=-1), class_78=True),                  # 4 link to a NULL view
             view(3, actor(org=6), class_78=True),                      # 5 member of hidden org
             view(3, actor(f7c=True, virtual_58=False), class_78=True), # 6 hidden organization
             view(4, actor(valid_b4=False), class_78=True),             # 7 invalid actor
             view(4, None, class_78=True, flags=0x200000)]              # 8 no actor, bit21
case('owners', org_views, [[k, i] for i in range(len(org_views)) for k in ('pick_target', 'select_owner', 'selectable', 'filter')]
     + [['select', 1, 1], ['select', 2, 1], ['select', 3, 0], ['select', 5, 0], ['clear', 0]])
# Distance chooser 0x5C1970 -> 0x5E05D0 with link94-live objects.
ch = [view(1, actor()), view(1, actor(link94=0)), view(1, None), view(1, actor(valid_b4=False))]
d = [f32(x) for x in (1.0, 2.0, 2.0, 0.5)]
case('choose', ch, [['choose', a, d[a] if a is not None else 0x7f7fffff, b, d[b]] for a in (None, 0, 1, 2, 3) for b in (0, 1, 2, 3) if a != b])
# Click selection with hits (0x437B20 cycling) and 0x640CF0 / 0x640E10.
cl = units(5)
case('click-hits', cl, [['click_hits', cl[0]['pos'], [0, 1, 2], 0], ['click_hits', [f32(0)] * 3, [2, 1, 0], 1],
                        ['click_hits', [f32(0)] * 3, [0, 1, 2, 3], 2], ['click_hits', [f32(0)] * 3, [3], 3],
                        ['click_hits', [f32(0)] * 3, [], None], ['select', 4, 0], ['click_hits', [f32(0)] * 3, [1, 2], 1]])
clicks = units(6) + [view(9, actor(), class_78=False, class_7c=True)]
origin = [f32(0), f32(0), f32(300)]
dirv = [f32(0), f32(0), f32(-1)]
case('click-paths', clicks, [['click', origin, dirv, 0], ['click', origin, dirv, 0], ['click', origin, dirv, 1],
                             ['click_add', origin, dirv, 0], ['click_add', origin, dirv, 0], ['click_add', origin, dirv, 1],
                             ['click_add', origin, dirv, 1], ['click', origin, dirv, 1], ['clear', 0], ['click', origin, dirv, 1]],
     find=[{'hits': [0, 1], 'best': 0}, {'hits': [0, 1, 2], 'best': 0}, {'hits': [3], 'best': 3},
           {'best': 1}, {'best': 1}, {'best': 0}, {'hits': [6, 5], 'best': 6}])
# Six-object cap through 0x640FA0: centroid of the selection or of the list.
many = [view(1, actor(f70=True, f90=False), class_78=True, pos=[f32(rng.uniform(-300, 300)), f32(rng.uniform(-300, 300)), f32(0)])
        for _ in range(14)]
case('select-list', many, [['select_list', list(range(0, 4))], ['select_list', list(range(4, 14))], ['clear', 0],
                           ['select_list', list(range(13, -1, -1))], ['clear', 0], ['select', 3, 0], ['select', 9, 0],
                           ['select_list', list(range(14))]])
# Box selection through the original 0x4377B0 / 0x4367A0 / 0x640A40 / 0x640FA0.
boxv = [view(1, actor(f70=True, f90=False), class_78=True, inside=(i % 3 != 2), proximity=(2 if i != 5 else 1)) for i in range(11)] + \
       [view(2, actor(), class_7c=True, inside=True), view(3, None, class_80=True, inside=True), view(1, actor(f70=True, f90=False), class_78=True, inside=True, pickable=False)]
ev = {'world': [f32(1), f32(2), f32(3)]}
case('box', boxv, [['box', ev, ev], ['clear', 0], ['hover', 12], ['box', ev, ev], ['hover', None], ['select', 11, 0], ['box', ev, ev]])
# Kinds that select once (0x804 / 0x820 / 0x120) next to multi-select kinds.
kinds = [view(1, actor(f90=True), class_78=True), view(1, actor(f90=True), class_78=True),
         view(1, actor(f90=False), class_78=True), view(1, actor(f70=True, f90=False), class_78=True),
         view(1, actor(f70=True, f90=False), class_78=True), view(1, actor(virtual_20=False), class_78=True),
         view(1, actor(virtual_20=False), class_78=True), view(1, None, class_84=True)]
case('kinds', kinds, [['select', 0, 0], ['select', 1, 0], ['select', 3, 0], ['select', 4, 0], ['select', 2, 0],
                      ['select', 5, 0], ['select', 6, 0], ['select', 3, 0], ['select', 7, 0], ['clear', 0], ['select', 7, 0],
                      ['select', 3, 0], ['select', 4, 0]])
# Static groups (0x4374D0) with the group list at +0x34.
gv = units(5)
case('static-groups', gv, [['select', 0, 0], ['select', 1, 0], ['select', 2, 0], ['deselect', 2], ['clear', 0],
                           ['select', 3, 0], ['select', 4, 0], ['deselect', 4]],
     groups=[{'name': 'group_1', 'members': [0, 1]}, {'name': 'group_2', 'members': [3]}, {'name': 'group_3', 'members': [0, 1, 2]}])
# Control groups: SaveGroup 0x439E90 and SelectGroup 0x4376C0.
cg = units(8) + [view(1, actor(f70=True, f90=False), class_78=False), view(1, None, class_78=True)]
case('control-groups', cg, [['select', 0, 0], ['select', 1, 0], ['save_group', 'group_1'], ['clear', 0], ['select_group', 'group_1'],
                            ['clear', 0], ['select', 1, 0], ['select', 2, 0], ['select', 8, 0], ['save_group', 'group_2'],
                            ['select_group', 'group_1'], ['select_group', 'group_9'], ['select', 3, 0], ['save_group', 'group_1'],
                            ['clear', 0], ['save_group', 'group_2'], ['select_group', 'group_2'], ['select', 4, 0], ['select', 5, 0],
                            ['select', 6, 0], ['select', 7, 0], ['save_group', 'group_3'], ['select_group', 'group_1'],
                            ['select', 9, 0], ['save_group', 'group_4']],
     groups=[{'name': 'group_9', 'members': [6], 'byte_18': True}])

def main():
    rows = []
    visited = {}
    for precision in PRECISIONS:
        for c in CASES:
            n = Native(precision, copy.deepcopy(c))
            result = n.run()
            visited.update(n.m.visited)
            rows.append({'precision': precision, 'case': c, 'native': result})
    doc = nm.header(__file__, 'Unicorn 2.1.4; original image executed, v14 bytes compared')
    doc['rows'] = rows
    doc['executed'] = nm.instructions(visited)
    out = HERE / 'native-selection.json'
    out.write_text(json.dumps(doc, indent=0))
    print(len(rows), 'cases', sum(len(r['native']) for r in rows), 'operations', len(visited), 'instructions')

if __name__ == '__main__':
    main()
