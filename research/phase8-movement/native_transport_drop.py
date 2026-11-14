"""Original TransportComponent drop 58BCB0, lead distance 58B180, loaded
mark 58B160 and exit 58B130 executing unchanged under Unicorn at Single24,
Double53 and Extended64.

Native helpers run inside the original code: 529C30, 46DA80 (offset
rotation), 4C8C20/4C8C70 (creation block defaults), 4BE740, 4CF1B0,
432E40/401060 (generation handles on a real table at [B364C0]), 409230,
577380 (fall carry; SVar cache +2C gravity) and 552640 (map clamp).

Seams, recorded in call order with their arguments (object pointers as fixture
ids): 554BF0 / 554500 / 554850 (placement; their in/out vectors are written
from the case), 4F9C10 / 4FACA0 (the whole 0x54-byte creation block is
dumped), 573D90 / 574C10, 4CD250, 4CF0F0, 578430, 4E2A60 / 4E1220, 57D860,
57D950 (fills the member slot), 4CA760 / 4D9B60 / 4CA2D0, 4F0E50, 576BB0,
5770A0, 4F53D0, 4F6760, the failure message 40C230 (with its string helpers
401B00 / 4B9F40 / 401560 and the allocator 856370 seamed), plane virtuals
+0 / +18 / +108 and Organization virtual +104 (x87 returns as binary64).

usage: native_transport_drop.py [AA.exe]
The optional argument must be the same original image the shared machine
checks (its SHA-256 is asserted).
"""
from pathlib import Path
import json, random, struct, sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-ai'))
import native_machine as nm
from native_machine import Machine, PRECISIONS, f32, header, instructions, sha
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_FPSW, UC_X86_REG_ESP

DROP, DROP_END = 0x58bcb0, 0x58c80f
LEAD, LEAD_END = 0x58b180, 0x58b218
MARK, MARK_END = 0x58b160, 0x58b180
LEAVE, LEAVE_END = 0x58b130, 0x58b142

f64 = lambda v: struct.unpack('<Q', struct.pack('<d', v))[0]
STALE = 0x00ff0000


class DropMachine(Machine):
    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = c
        self.ids = {}
        self.addr = {}
        r = c.get('responses', {})
        self.resp = r
        # Stack below the frame filled with a pattern (the 57D950 flag word
        # carries three uninitialised bytes; only its low byte is recorded).
        self.u.mem_write(nm.STACK_BASE, b'\xab' * (nm.STACK - nm.STACK_BASE))
        # globals
        table = self.alloc(0x60010)
        self.table = table
        self.w(0xb364c0, table)
        gw = self.alloc(0x100)
        self.w(0xb36534, gw)
        self.gworld = gw
        m = self.map = self.alloc(0x40)
        self.w(gw + 0x30, m)
        self.w(m + 8, c['map']['cell'])
        self.w(m + 0xc, c['map']['extent'][0])
        self.w(m + 0x10, c['map']['extent'][1])
        sv = self.alloc(0x100)
        self.w(0xb39ab0, sv)
        g = self.alloc(4); self.w(g, c['gravity']); self.w(sv + 0x2c, g)
        s3 = self.alloc(4); self.w(s3, c['svar_3c']); self.w(sv + 0x3c, s3)
        self.w(0xb36544, self.alloc(0x10))
        self.w(0xb279c0, c['zero_vector'][0])
        self.w(0xb279c4, c['zero_vector'][1])
        # strings
        self.free_stub = self.stub(lambda m: (self.ev('string_free'), self.ret(0, 4)))
        self.str_alloc = self.alloc(0x10)
        sav = self.alloc(0x20)
        self.w(self.str_alloc, sav)
        self.w(sav + 4, self.free_stub)
        self.w(sav + 0xc, self.stub(lambda m: self.ret(self.empty_header)))
        self.empty_header = self.fake_header()
        self.empty = self.empty_header + 0x10
        # definitions
        for key, d in c['defs'].items():
            p = self.obj(key, 0x400)
            self.w(p + 0x10, self.name_string(key))
            self.w(p + 0x24, d['height_24'])
            self.w(p + 0x1a4, d['field_1a4'])
            self.w(p + 0x3e4, d.get('drop_offset', [0, 0])[0])
            self.w(p + 0x3e8, d.get('drop_offset', [0, 0])[1])
            self.wb(p + 0x3ec, d.get('byte_3ec', 0))
        # plane
        pc = c['plane']
        plane = self.obj('plane', 0x200)
        vt = self.alloc(0x200)
        self.w(plane, vt)
        self.w(vt + 0, self.stub(lambda m: (self.ev('plane_virtual_0'), self.ret_st0_f64(pc['v0']))))
        self.w(vt + 0x18, self.stub(lambda m: (self.ev('plane_virtual_18'), self.ret_st0_f64(pc['v18']))))
        self.w(vt + 0x108, self.stub(lambda m: (self.ev('kingdom'), self.ret(pc['kingdom']))))
        self.w(plane + 4, self.addr[pc['def']])
        self.w(plane + 8, pc['flags'])
        for i in range(3):
            self.w(plane + 0x20 + 4 * i, pc['position'][i])
            self.w(plane + 0x2c + 4 * i, pc['facing'][i])
        self.w(plane + 0x88, pc['word_88'])
        # thing (the actor any creation or placement returns)
        tc = c['thing']
        thing = self.obj('thing', 0x100)
        self.w(thing + 4, self.addr[tc['def']])
        for i in range(3):
            self.w(thing + 0x20 + 4 * i, tc['position'][i])
            self.w(thing + 0x2c + 4 * i, tc['facing'][i])
        mover = self.alloc(0x100)
        self.w(thing + 0x6c, mover)
        self.ids[mover] = 'thing'
        # organization
        oc = c['org']
        org = self.obj('org', 0x100)
        ovt = self.alloc(0x200)
        self.w(org, ovt)
        self.w(ovt + 0x104, self.stub(lambda m: (self.ev('organization_virtual_104', self.id(self.ecx())),
                                                  self.ret_st0_f64(oc['v104']))))
        self.w(org + 0x14, oc['handle'])
        comp = self.alloc(0x80)
        self.ids[comp] = 'org'
        self.w(org + 0x7c, comp)
        self.members = self.alloc(4 * 64)
        self.w(comp + 0x28, self.members)
        self.w(comp + 0x44, self.ref(oc['leader']))
        if oc['registered']:
            self.register(oc['handle'], org)
        # attachment
        att = self.obj('attachment', 0x200)
        acomp = self.alloc(0x40)
        self.ids[acomp] = 'attachment'
        self.w(att + 0xd8, acomp)
        self.register(c['attachment_handle'], att)
        # layout
        lc = c.get('layout')
        if lc is not None:
            lay = self.obj('layout', 0x40)
            info = self.alloc(0x40)
            self.w(lay + 8, info)
            self.w(info + 0x18, lc['leader_slot'])
            self.w(info + 0x2c, lc['count'])
            defs = [self.ref(d) for d in lc['slots']]
            order = list(range(len(defs)))
            random.Random(len(defs)).shuffle(order)
            arr = [0] * len(defs)
            for slot, index in enumerate(order):
                arr[index] = defs[slot]
            self.w(lay + 0xc, self.array(arr))
            self.w(info + 0x28, self.array(order))
        # transport
        t = self.t = self.alloc(0x80)
        x = c['transport']
        self.w(t + 4, plane)
        self.w(t + 0x28, self.addr[x['cargo']])
        self.w(t + 0x2c, self.addr['layout'] if lc is not None else 0)
        self.w(t + 0x30, x['point'][0]); self.w(t + 0x34, x['point'][1])
        self.wb(t + 0x38, x['loaded'])
        self.w(t + 0x3c, x['count']); self.w(t + 0x40, x['dropped'])
        self.w(t + 0x44, x['organization']); self.w(t + 0x48, x['next_slot'])
        self.name = self.fake_header() + 0x10
        self.ids[self.name] = 'name'
        self.w(t + 0x58, self.name)
        self.w(t + 0x5c, x['attachment'])
        self.seams_install()

    # objects ------------------------------------------------------------
    def obj(self, key, size):
        p = self.alloc(size)
        self.ids[p] = key
        self.addr[key] = p
        return p

    def ref(self, key):
        return 0 if key is None else self.addr[key]

    def id(self, a):
        return self.ids.get(a, a)

    def register(self, handle, p):
        i = handle & 0xffff
        self.u.mem_write(self.table + 4 + 2 * i, struct.pack('<H', handle >> 16))
        self.w(self.table + 0x20004 + 4 * i, p)

    def fake_header(self):
        h = self.alloc(0x20)
        self.w(h, self.str_alloc)
        self.w(h + 0xc, 1000)
        return h

    def name_string(self, key):
        s = self.fake_header() + 0x10
        self.ids[s] = 'name:' + key
        return s

    def ev(self, name, *args):
        self.events.append([name] + list(args))

    def f(self, p, n):
        return [self.r(p + 4 * i) for i in range(n)]

    def params(self, p):
        assert self.r(p) == 0xa25b28, hex(self.r(p))
        w = self.f(p, 0x14)
        name = w[7]
        name = 'empty' if name == self.empty else self.id(name)
        return [self.id(w[1]), w[2], w[3], w[4], w[5], w[6], name, w[8], w[9], w[10], self.id(w[11]),
                w[12], w[13], w[14], w[15], w[16], w[17], w[18], w[19],
                self.rb(p + 0x50), self.rb(p + 0x51), self.rb(p + 0x52)]

    def out(self, key, p, n=2):
        o = self.resp[key]
        for i, v in enumerate(o.get('out', [])):
            self.w(p + 4 * i, v)
        return int(o['ok'])

    # seams --------------------------------------------------------------
    def seams_install(self):
        r, S = self.resp, self.seam
        S(0x856370, lambda m: self.ret(self.str_alloc))
        S(0x401b00, lambda m: (self.w(self.ecx(), self.fake_header() + 0x10), self.ret(self.ecx(), 4)))
        S(0x4b9f40, lambda m: (self.w(self.arg(1), self.fake_header() + 0x10), self.ret(self.arg(1), 8)))
        S(0x401560, lambda m: (self.w(self.ecx(), self.r(self.arg(1))), self.ret(self.ecx(), 4)))
        S(0x405b80, lambda m: (self.w(self.ecx(), self.r(self.arg(1))), self.ret(self.ecx(), 4)))

        def report(m):
            assert self.arg(1) == 0xb399b0 and self.arg(2) == 0x11
            self.ev('report', self.id(self.arg(3)), self.arg(4), self.id(self.arg(5)), self.id(self.arg(7)))
            self.ret(0)
        S(0x40c230, report)

        def place_org(m):
            pp, pf = self.arg(1), self.arg(2)
            self.ev('place_organization', self.f(pp, 2), self.f(pf, 2), self.id(self.arg(3)),
                    [self.arg(4), self.arg(5)], self.arg(6))
            o = r['place_organization']
            for i, v in enumerate(o.get('point', [])):
                self.w(pp + 4 * i, v)
            for i, v in enumerate(o.get('facing', [])):
                self.w(pf + 4 * i, v)
            self.ret(int(o['ok']), 0x18)
        S(0x554bf0, place_org)

        def create(m):
            self.ev('create', self.params(self.arg(1)))
            self.ret(self.ref(r['create']), 4)
        S(0x4f9c10, create)

        def create_unloaded(m):
            self.ev('create_unloaded', self.params(self.arg(1)), self.id(self.arg(2)))
            self.ret(self.ref(r['create_unloaded']), 8)
        S(0x4faca0, create_unloaded)
        S(0x573d90, lambda m: (self.ev('attachment_query', self.id(self.ecx()), self.id(self.arg(1))),
                               self.ret(r['attachment_query'], 4)))
        S(0x574c10, lambda m: (self.ev('attachment_attach', self.id(self.ecx()), self.id(self.arg(1)),
                                       self.arg(2), self.arg(3)), self.ret(0, 0xc)))
        S(0x4cd250, lambda m: (self.ev('schedule', self.ecx() - self.t), self.ret(self.ecx())))
        S(0x4cf0f0, lambda m: (self.ev('plane_loaded', self.id(self.ecx())), self.ret(0)))
        S(0x578430, lambda m: (self.ev('organization_carrier', self.id(self.ecx()), self.id(self.arg(1))),
                               self.ret(0, 4)))
        S(0x4e2a60, lambda m: (self.ev('kingdom_record', (self.ecx() - 0x1fc) & 0xffffffff,
                                       self.id(self.arg(1)), self.arg(2)), self.ret(r['kingdom_record'], 8)))
        S(0x4e1220, lambda m: (self.ev('record_recruit', self.ecx(), self.id(self.arg(1)), self.arg(2)),
                               self.ret(0, 8)))
        S(0x57d860, lambda m: (self.ev('organization_add', self.id(self.ecx()), self.arg(1),
                                       self.id(self.arg(2))), self.ret(0, 8)))

        def place_thing(m):
            pp, pf = self.arg(1), self.arg(2)
            self.ev('place_thing', self.f(pp, 2), self.f(pf, 2), self.id(self.arg(3)), self.arg(4), self.arg(5),
                    self.arg(6), self.arg(7))
            o = r['place_thing']
            for i, v in enumerate(o.get('position', [])):
                self.w(pp + 4 * i, v)
            for i, v in enumerate(o.get('facing', [])):
                self.w(pf + 4 * i, v)
            self.ret(int(o['ok']), 0x1c)
        S(0x554500, place_thing)

        def place_member(m):
            slot = self.arg(1)
            self.ev('organization_place', self.id(self.ecx()), slot, [self.arg(2), self.arg(3), self.arg(4)],
                    [self.arg(5), self.arg(6)], self.arg(7) & 0xff, self.arg(8))
            self.w(self.members + 4 * slot, self.ref(r['member']))
            self.ret(1, 0x20)
        S(0x57d950, place_member)

        def build(m):
            cmd = self.ecx()
            self.ev('command_build', [self.arg(i) for i in range(1, 6)])
            self.w(cmd + 0x18, self.arg(1)); self.w(cmd + 0x1c, self.arg(2)); self.w(cmd + 0x28, self.arg(3))
            self.w(cmd + 0x20, self.arg(4)); self.w(cmd + 0x24, self.arg(5))
            self.ret(cmd, 0x14)
        S(0x4ca760, build)
        S(0x4d9b60, lambda m: (self.ev('command', self.id(self.arg(1)), self.f(self.ecx() + 0x18, 5),
                                       self.rb(self.ecx() + 0x3c)), self.ret(0, 4)))
        S(0x4ca2d0, lambda m: (self.ev('command_destroyed'), self.ret(0)))
        S(0x4f0e50, lambda m: (self.ev('ground_height', self.f(self.arg(1), 2), self.arg(2)),
                               self.ret_st0_f64(r['ground'], 8)))
        S(0x576bb0, lambda m: (self.ev('commit_pose', self.id(self.ecx()), [self.arg(i) for i in range(1, 4)],
                                       [self.arg(i) for i in range(4, 7)], self.arg(7)), self.ret(0, 0x1c)))

        def landing(m):
            pt, pd = self.arg(1), self.arg(2)
            self.ev('landing_search', self.f(pt, 2), self.f(pd, 2), self.id(self.arg(3) - 0x38),
                    [self.arg(i) for i in range(4, 9)])
            o = r['landing']
            for i, v in enumerate(o.get('target', [])):
                self.w(pt + 4 * i, v)
            for i, v in enumerate(o.get('direction', [])):
                self.w(pd + 4 * i, v)
            self.ret(int(o['ok']), 0x20)
        S(0x554850, landing)
        S(0x4f53d0, lambda m: (self.ev('destroy', self.id(self.arg(1)), self.arg(2)), self.ret(0, 8)))
        S(0x5770a0, lambda m: (self.ev('begin_aimed_fall', self.id(self.ecx()), self.arg(1),
                                       [self.arg(2), self.arg(3)], self.arg(4)), self.ret(1, 0x10)))
        S(0x4f6760, lambda m: (self.ev('remove_actor', self.id(self.arg(1)), self.arg(2)), self.ret(0, 8)))
        S(0x570020, lambda m: (_ for _ in ()).throw(AssertionError('570020 reached')))

    def top(self):
        return (self.u.reg_read(UC_X86_REG_FPSW) >> 11) & 7

    def state(self):
        t = self.t
        return {'loaded': self.rb(t + 0x38), 'count': self.r(t + 0x3c), 'dropped': self.r(t + 0x40),
                'organization': self.r(t + 0x44), 'next_slot': self.r(t + 0x48)}

    def run(self):
        op = self.c['operation']
        if op == 'drop':
            self.call(DROP, self.t)
            assert self.top() == 0
            return {'events': self.events, 'transport': self.state()}
        if op == 'lead':
            a = self.c['lead']
            self.call(LEAD, self.t, [a['distance'], a['factor']])
            v = self.st0()
            assert self.top() == 0
            return {'events': self.events, 'f32_bits': v['f32_bits'],
                    'extended': [v['extended_significand'], v['extended_sign_exponent']]}
        if op == 'mark':
            self.call(MARK, self.t)
            return {'events': self.events, 'transport': self.state()}
        if op == 'leave':
            self.call(LEAVE, self.t)
            return {'events': self.events, 'transport': self.state()}
        raise ValueError(op)


def fr(rnd, lo, hi):
    return f32(rnd.uniform(lo, hi))


def unit(rnd):
    import math
    a = rnd.uniform(-math.pi, math.pi)
    return [f32(math.cos(a)), f32(math.sin(a)), f32(rnd.choice([0.0, 0.0, rnd.uniform(-0.2, 0.2)]))]


def base(rnd, i):
    pos = [fr(rnd, -20, 300), fr(rnd, -20, 220), fr(rnd, 5, 120)]
    defs = {
        'plane_def': {'height_24': fr(rnd, 0, 3), 'field_1a4': fr(rnd, 0, 2),
                      'drop_offset': [fr(rnd, -6, 6), fr(rnd, -6, 6)], 'byte_3ec': 0},
        'cargo': {'height_24': fr(rnd, 0, 4), 'field_1a4': fr(rnd, -1, 3)},
    }
    for k in range(4):
        defs['m%d' % k] = {'height_24': fr(rnd, 0, 4), 'field_1a4': fr(rnd, -1, 3)}
    thing_pos = [fr(rnd, -10, 270), fr(rnd, -10, 200), fr(rnd, 0, 150)]
    if rnd.random() < 0.2:
        thing_pos[0] = fr(rnd, 250, 262)
    return {
        'name': 'case%d' % i, 'precision': list(PRECISIONS)[i % 3], 'operation': 'drop',
        'map': {'cell': f32(rnd.choice([2.0, 4.0])), 'extent': [f32(256.0), f32(192.0)]},
        'gravity': f32(rnd.choice([-49.05, -49.05, -9.81, -30.5])),
        'svar_3c': fr(rnd, 0.5, 12),
        'zero_vector': [0, 0] if rnd.random() < 0.7 else [fr(rnd, -1, 1), fr(rnd, -1, 1)],
        'defs': defs,
        'plane': {'def': 'plane_def', 'flags': rnd.choice([0, 0x1000, 0x1000 | 0x40, 0x41, 0x1000 | 0x2000]),
                  'position': pos, 'facing': unit(rnd), 'word_88': rnd.choice([0, 0, 0x1234]),
                  'kingdom': rnd.choice([0x5000, 0x5000, 0, 0xfffffe04]),
                  'v0': f64(rnd.uniform(0.2, 9.0)), 'v18': f64(rnd.uniform(0.0, 40.0))},
        'thing': {'def': rnd.choice(['cargo', 'm0', 'm1']), 'position': thing_pos, 'facing': unit(rnd)},
        'org': {'handle': 0x00070012, 'registered': rnd.random() < 0.9, 'v104': f64(rnd.uniform(-5, 5)),
                'leader': rnd.choice(['thing', 'thing', 'plane', None])},
        'attachment_handle': 0x00030021,
        'transport': {'cargo': 'cargo', 'point': [fr(rnd, 0, 256), fr(rnd, 0, 192)], 'loaded': 0,
                      'count': 0, 'dropped': 0, 'organization': 0, 'next_slot': 0,
                      'attachment': 0},
        'responses': {
            'place_organization': {'ok': rnd.random() < 0.8, 'point': [fr(rnd, 0, 256), fr(rnd, 0, 192)]
                                   if rnd.random() < 0.5 else [],
                                   'facing': unit(rnd)[:2] if rnd.random() < 0.3 else []},
            'create': rnd.choice(['org', 'thing', None]),
            'create_unloaded': rnd.choice(['thing', 'thing', None]),
            'attachment_query': rnd.getrandbits(32),
            'kingdom_record': rnd.choice([0, 0x6100]),
            'place_thing': {'ok': rnd.random() < 0.75,
                            'position': [fr(rnd, 0, 256), fr(rnd, 0, 192)] if rnd.random() < 0.6 else [],
                            'facing': unit(rnd)[:2] if rnd.random() < 0.4 else []},
            'member': rnd.choice(['thing', 'thing', 'thing', None]),
            'ground': f64(rnd.uniform(-5, 40)),
            'landing': {'ok': rnd.random() < 0.8,
                        'target': [fr(rnd, 0, 256), fr(rnd, 0, 192)] if rnd.random() < 0.4 else [],
                        'direction': unit(rnd)[:2] if rnd.random() < 0.3 else []},
        },
    }


def cases():
    rnd = random.Random(0x58bcb0)
    rows = []
    for i in range(900):
        c = base(rnd, i)
        x = c['transport']
        kind = i % 6
        organization = kind in (0, 1, 2, 3)
        if organization:
            n = rnd.randrange(1, 5)
            slots = [rnd.choice(['m0', 'm1', 'm2', 'm3', None]) for _ in range(n)]
            c['layout'] = {'count': n, 'leader_slot': rnd.randrange(n),
                           'slots': slots + [rnd.choice([None, 'm2'])]}
            c['defs']['org_def'] = {'height_24': fr(rnd, 0, 1), 'field_1a4': fr(rnd, 0, 1)}
            x['cargo'] = 'org_def'
            x['count'] = rnd.choice([n, n, n + 1, 1])
            if kind == 0:
                x['dropped'] = 0
                c['responses']['create'] = rnd.choice(['org', 'org', 'org', None])
                x['attachment'] = rnd.choice([0, 0, c['attachment_handle']])
                c['org']['registered'] = True
            else:
                x['dropped'] = rnd.randrange(1, x['count'] + 1) if rnd.random() < 0.9 else 0
                if x['dropped'] == 0:
                    c['responses']['create'] = 'org'
                    c['org']['registered'] = True
                x['next_slot'] = rnd.randrange(0, n + 1)
                x['organization'] = rnd.choice([c['org']['handle']] * 6 + [0, c['org']['handle'] ^ STALE])
            if kind == 3:
                c['defs']['plane_def']['byte_3ec'] = 1
        else:
            c['layout'] = None
            del c['layout']
            x['count'] = rnd.choice([1, 2, 3])
            x['dropped'] = rnd.choice([0, 0, 1, 3])
            c['responses']['create'] = rnd.choice(['thing', 'thing', None])
            if kind == 5:
                c['defs']['plane_def']['byte_3ec'] = rnd.choice([0, 1])
        if i % 40 == 7:
            c['plane']['facing'] = [f32(0.0), f32(0.0), f32(0.0)]
        if i % 50 == 11:
            c['thing']['facing'] = [f32(1e-6), f32(-1e-6), f32(0.0)]
        rows.append(c)
    # lead distance
    for i in range(240):
        c = base(rnd, 1000 + i)
        c['operation'] = 'lead'
        c['name'] = 'lead%d' % i
        if i % 2:
            n = rnd.randrange(0, 5)
            c['layout'] = {'count': n, 'leader_slot': 0,
                           'slots': [rnd.choice(['m0', 'm1', 'm2', 'm3', None]) for _ in range(n)] + [None]}
        c['lead'] = {'distance': fr(rnd, -60, 150) if rnd.random() < 0.9 else f32(0.0),
                     'factor': fr(rnd, -5, 40)}
        rows.append(c)
    for i in range(6):
        c = base(rnd, 2000 + i)
        c['operation'] = 'mark' if i < 3 else 'leave'
        c['name'] = '%s%d' % (c['operation'], i)
        rows.append(c)
    return rows


def main():
    if len(sys.argv) > 1:
        assert sha(Path(sys.argv[1]).read_bytes()) == sha(nm.original.b), 'not the original image'
    out, visited = [], {}
    for c in cases():
        try:
            m = DropMachine(c)
            result = m.run()
        except Exception:
            print(c['name'])
            raise
        m.verify_v14()
        visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    cover = {}
    for name, (lo, hi) in {'58BCB0': (DROP, DROP_END), '58B180': (LEAD, LEAD_END),
                           '58B160': (MARK, MARK_END), '58B130': (LEAVE, LEAVE_END)}.items():
        cover[name] = len([a for a in visited if lo <= a < hi])
    d.update(instructions=instructions(visited), coverage=cover, cases=out)
    p = HERE / 'native-transport-drop.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'coverage': cover,
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
