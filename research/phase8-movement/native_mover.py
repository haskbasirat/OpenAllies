"""Original Mover component operations not covered by phase4/phase7.

Whole unchanged routines execute: Tick dispatch 577E50 (with 4D03D0, 577330,
5768A0, 5768F0), stop-turning 5767F0, requested-speed setter 576B20, maximum
speed refresh 576C70 with turn-rate refresh 576850, fall starts 576740/5770A0,
falling step 577A70 with pose commit 576BB0, interior test 5766A0, interior
clamp 552640 and normalization 4453A0, post-load falling fixup 576700 and the
launch-speed helper 577380.

Seams (recorded, never executed): ground step 5774A0, falling step 577A70
when reached from Tick, overlap refresh 4D67F0, aircraft virtual +38, Mind
virtuals +118/+128/+12C, activation 4CFC10 (through 4DB0E0), deactivation
4DB090, scheduler 50AC00/50B250, actor state setter 4D64F0, landing 4D64A0,
map leave/enter 4D04B0/4D7EA0, position commit 4EE5C0, actor facing virtual
+54, terrain height 4F0E50, current maximum speed 45D6F0 and organization
speed notification 57B250. The PlaneFallGravityTweak SVar guard is pre-set:
first-time registry lookup is not replayed; its resolved value is an input.
"""
import copy, json, random, struct, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, write_document, f2b, b2f, d2b, PRECISION  # noqa: E402

MOVER_WORDS = [0x10, 0x14, 0x18, 0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30, 0x34, 0x38, 0x3c, 0x40, 0x44, 0x48,
               0x50, 0x58, 0x60, 0x64, 0x68, 0x70, 0x74, 0x78, 0x7c, 0x80, 0x84]
MOVER_BYTES = [0x4c, 0x4d, 0x54, 0x55, 0x56, 0x5c, 0x6c, 0x88]


def fb(values):
    return [f2b(v) for v in values]


class MoverMachine(Machine):
    def __init__(self, case):
        super().__init__(case['precision'])
        self.c = case
        self.mover = self.alloc(0x100)
        self.actor = self.alloc(0x200)
        self.definition = self.alloc(0x500)
        self.world = self.alloc(0x100)
        self.map = self.alloc(0x40)
        self.cache = self.alloc(0x200)
        self.values = self.alloc(0x100)
        a = case['actor']
        self.w32(self.mover, 0xa3ce88)  # actual Mover vtable (+38 = 4DB0E0)
        self.w32(self.mover + 8, 0xa3ce5c)
        self.w32(self.mover + 4, self.actor)
        self.w32(self.actor + 4, self.definition)
        self.w32(self.actor + 8, a['flags'])
        self.w32(self.actor + 0x14, 0x1234)
        self.w32(self.actor + 0x20, *a['position'])
        self.w32(self.actor + 0x2c, *a['facing'])
        self.w32(self.actor + 0x6c, self.mover)
        self.w32(self.actor + 0xe4, self.mover if a['mover_active'] else self.alloc(0x10))
        vt = self.alloc(0x200)
        self.w32(self.actor, vt)
        self.w32(vt + 0x54, self.stub('SetFacing'))
        if a['has_mind']:
            mind = self.alloc(0x40)
            mvt = self.alloc(0x200)
            self.w32(mind, mvt)
            self.w32(mvt + 0x118, self.stub('MindMaxSpeedChanged'))
            self.w32(mvt + 0x128, self.stub('MindMovementFailed'))
            self.w32(mvt + 0x12c, self.stub('MindDestinationReached'))
            self.w32(self.actor + 0x70, mind)
        if a['properties_flags'] is not None:
            props = self.alloc(0x40)
            self.w8(props + 0x1c, a['properties_flags'])
            self.w32(self.actor + 0x78, props)
        self.organization = None
        if a['element_organization'] is not None:
            element = self.alloc(0x40)
            organization_actor = 0
            if a['element_organization']:
                organization_actor = self.alloc(0x100)
                self.organization = self.alloc(0x100)
                self.w32(organization_actor + 0x7c, self.organization)
            self.w32(element + 0x10, organization_actor)
            self.w32(self.actor + 0x80, element)
        if a['has_aircraft']:
            aircraft = self.alloc(0x40)
            avt = self.alloc(0x100)
            self.w32(aircraft, avt)
            self.w32(avt + 0x38, self.stub('AircraftStep'))
            self.w32(self.actor + 0xbc, aircraft)
        d = case['definition']
        self.w32(self.definition + 0x228, d['speed'])
        self.w32(self.definition + 0x230, d['acceleration'])
        self.w32(self.definition + 0x234, d['turn_radius'])
        self.w32(self.definition + 0x238, d['turn_type'])
        self.w32(self.definition + 0x40c, d['aircraft_height_offset'])
        m = case['mover']
        for off, bits in m['words']:
            self.w32(self.mover + off, bits)
        for off, value in m['bytes']:
            self.w8(self.mover + off, value)
        self.w32(self.mover + 0xc, case['updatable_word_0c'])
        s = case['settings']
        self.w32(0xb36534, self.world)
        self.w32(self.world + 0x2c, s['world_version'])
        self.w32(self.world + 0x30, self.map)
        self.w32(self.world + 0xe0, f2b(125.0))
        self.w32(self.world + 0xe8, s['world_delta'])
        self.w32(self.map + 8, s['map_cell_size'])
        self.w32(self.map + 0xc, *s['map_extent'])
        self.w32(0xb39ab0, self.cache)
        for slot, name in [(0x2c, 'gravity'), (0x48, 'catchup'), (0x54, 'precision_radius')]:
            self.w32(self.values + slot, s[name])
            self.w32(self.cache + slot, self.values + slot)
        # Pre-initialized PlaneFallGravityTweak static: [B3D698] -> value.
        self.w32(0xb3d69c, self.r32(0xb3d69c) | 1)
        self.w32(self.values + 0x80, s['plane_fall_gravity_tweak'])
        self.w32(0xb3d698, self.values + 0x80)
        self.responses = list(case.get('responses', []))
        S = self.seams
        S[0x5774a0] = lambda m: (m.event('GroundStep', m.arg()), m.ret(4, 1))
        S[0x577a70] = self.tick_fall if case['operation'] == 'tick' else None
        if S[0x577a70] is None:
            del S[0x577a70]
        S[0x4d67f0] = self.refresh
        S[0x50b250] = lambda m: (m.event('CancelScheduled'), m.ret(0))
        S[0x50ac00] = lambda m: (m.event('ScheduleImmediate', m.arg()), m.ret(4))
        S[0x4db090] = lambda m: (m.event('DeactivateMover'), m.ret(0))
        S[0x4cfc10] = self.activate
        S[0x4d64f0] = lambda m: (m.event('ActorState', m.arg(0), m.arg(1)), m.ret(8))
        S[0x4d64a0] = lambda m: (m.event('Landed'), m.ret(0))
        S[0x4d04b0] = lambda m: (m.event('LeftMap'), m.ret(0))
        S[0x4d7ea0] = lambda m: (m.event('EnteredMap'), m.ret(0))
        S[0x4ee5c0] = self.commit
        S[0x4f0e50] = self.height
        S[0x57b250] = self.organization_speed

    def stub(self, name):
        address = self.stub_next
        self.stub_next += 16
        handlers = {
            'SetFacing': self.set_facing,
            'MindMaxSpeedChanged': lambda m: (m.event(name, m.arg(0), m.arg(1)), m.ret(8)),
            'MindMovementFailed': lambda m: (m.event(name), m.ret(0)),
            'MindDestinationReached': lambda m: (m.event(name), m.ret(0, 1)),
            'AircraftStep': lambda m: (m.event(name, m.arg()), m.ret(4)),
        }
        self.seams[address] = handlers[name]
        return address

    def organization_speed(self, m):
        assert m.ecx() == self.organization and m.arg(0) == self.actor
        m.event('OrganizationSpeedChanged', m.arg(1))
        m.ret(8)

    def next_response(self, name):
        r = self.responses.pop(0)
        assert r['name'] == name, (r, name)
        return r

    def tick_fall(self, m):
        m.event('Fall', m.arg())
        m.ret(4, 1)

    def refresh(self, m):
        m.event('RefreshOverlaps')
        r = self.next_response('RefreshOverlaps')
        if 'flags' in r:
            self.w32(self.actor + 8, r['flags'])
        m.ret(0)

    def activate(self, m):
        m.event('Activate')
        m.ret(4, int(self.next_response('Activate')['value']))

    def commit(self, m):
        p = m.arg()
        xyz = self.r32(p, 3)
        m.event('CommitPosition', *xyz)
        self.w32(self.actor + 0x20, *xyz)
        m.ret(4)

    def set_facing(self, m):
        p = m.arg()
        xyz = self.r32(p, 3)
        m.event('SetFacing', *xyz)
        if self.r32(self.actor + 8) & (1 << 17) == 0:
            self.w32(self.actor + 0x2c, *xyz)
        m.ret(4)

    def height(self, m):
        p, flags = m.arg(0), m.arg(1)
        x, y = self.r32(p, 2)
        m.event('GroundHeight', x, y, flags)
        r = self.next_response('GroundHeight')
        m.ret_float(int(r['f64'], 16), 'f64', 8)

    def speed_context(self):
        multiplier = self.c.get('speed_multiplier')
        if multiplier is None:
            return 0
        context = self.alloc(0x40)
        self.w32(context + 0x38, multiplier)
        return context

    def run(self):
        c = self.c
        op = c['operation']
        entry, ecx, args = {
            'tick': (0x577e50, self.mover + 8, [c['arg']]),
            'stop_turning': (0x5767f0, self.mover, []),
            'set_speed': (0x576b20, self.mover, c['args']),
            'refresh_max_speed': (0x576c70, self.mover, [self.speed_context()]),
            'begin_fall': (0x576740, self.mover, c['args']),
            'begin_aimed_fall': (0x5770a0, self.mover, c['args']),
            'fall': (0x577a70, self.mover, [c['arg']]),
            'restore_falling': (0x576700, self.mover, []),
            'interior': (0x5766a0, self.map, c['args']),
        }[op]
        eax = self.call(entry, ecx, args)
        pops = {'tick': 8, 'stop_turning': 0, 'set_speed': 8, 'refresh_max_speed': 4, 'begin_fall': 8,
                'begin_aimed_fall': 16, 'fall': 4, 'restore_falling': 0, 'interior': 8}[op]
        assert self.final_esp == 0x2f08000 + 4 + pops, (op, hex(self.final_esp))
        assert self.fpu_top() == 0, 'unbalanced x87 stack'
        assert not self.responses, self.responses
        return {
            'return_al': eax & 255,
            'mover_words': [[o, self.r32(self.mover + o)] for o in MOVER_WORDS],
            'mover_bytes': [[o, self.r8(self.mover + o)] for o in MOVER_BYTES],
            'actor_position': self.r32(self.actor + 0x20, 3),
            'actor_facing': self.r32(self.actor + 0x2c, 3),
            'actor_flags': self.r32(self.actor + 8),
            'events': self.events,
        }


def launch_speed(precision, distance, factor, offset):
    m = Machine(precision)
    obj = m.alloc(0x200)
    cache = m.alloc(0x100)
    m.w32(0xb39ab0, cache)
    m.w32(cache + 0x2c, cache + 0x80)
    m.w32(cache + 0x80, f2b(-49.05))
    m.w32(obj + 0x1a4, offset)
    out = m.alloc(16)
    # 577380 is cdecl with an x87 return; store it through a tiny caller stub.
    stub = m.alloc(64)
    code = (b'\x68' + struct.pack('<I', factor) + b'\x68' + struct.pack('<I', distance) +
            b'\x68' + struct.pack('<I', obj) + b'\xe8' + struct.pack('<i', 0x577380 - (stub + 20)) +
            b'\x83\xc4\x0c' + b'\xdb\x3d' + struct.pack('<I', out) + b'\xc3')
    m.u.mem_write(stub, code)
    m.call(stub)
    assert m.fpu_top() == 0
    return bytes(m.u.mem_read(out, 10)).hex()


def base_case():
    return {
        'name': 'base', 'operation': 'tick', 'precision': 'Double53', 'arg': f2b(0.0625), 'args': [],
        'updatable_word_0c': 0,
        'actor': {'flags': 0, 'position': fb([1.0, 2.0, 3.0]), 'facing': fb([1.0, 0.0, 0.0]),
                  'mover_active': True, 'has_mind': True, 'properties_flags': 0, 'element_organization': None,
                  'has_aircraft': False},
        'definition': {'speed': f2b(6.0), 'acceleration': f2b(2.0), 'turn_radius': f2b(1.5), 'turn_type': 1,
                       'aircraft_height_offset': f2b(12.5)},
        'mover': {'words': [[0x10, f2b(10.0)], [0x14, f2b(2.0)], [0x18, 0], [0x20, f2b(2.0)], [0x24, f2b(3.0)],
                            [0x28, f2b(6.0)], [0x30, f2b(1.5)], [0x34, f2b(4.0)], [0x50, f2b(1.0)]],
                  'bytes': []},
        'settings': {'world_version': 28, 'world_delta': f2b(0.0625), 'map_cell_size': f2b(2.0),
                     'map_extent': fb([256.0, 192.0]), 'gravity': f2b(-49.05), 'catchup': f2b(1.75),
                     'precision_radius': f2b(0.1), 'plane_fall_gravity_tweak': 0x3ba3d70a},
        'responses': [],
    }


def make(name, op, **parts):
    c = base_case()
    c['name'] = name
    c['operation'] = op
    for k, v in parts.items():
        if k in ('actor', 'definition', 'settings'):
            c[k].update(v)
        elif k == 'words':
            d = dict(c['mover']['words'])
            d.update(v)
            c['mover']['words'] = sorted([list(x) for x in d.items()])
        elif k == 'bytes':
            d = dict(c['mover']['bytes'])
            d.update(v)
            c['mover']['bytes'] = sorted([list(x) for x in d.items()])
        else:
            c[k] = v
    return c


def tick_cases():
    rows = []
    R = lambda flags=None: [{'name': 'RefreshOverlaps', **({'flags': flags} if flags is not None else {})}]
    rows.append(make('tick_falling', 'tick', bytes={0x56: 1}))
    rows.append(make('tick_inactive', 'tick', actor={'mover_active': False}))
    rows.append(make('tick_properties_bit1', 'tick', actor={'properties_flags': 2}))
    rows.append(make('tick_properties_other_bits', 'tick', actor={'properties_flags': 0xfd}, responses=R()))
    rows.append(make('tick_properties_dead_flag0', 'tick', actor={'properties_flags': 2, 'flags': 1}, responses=R()))
    rows.append(make('tick_no_properties', 'tick', actor={'properties_flags': None}, responses=R()))
    rows.append(make('tick_ground', 'tick', responses=R()))
    rows.append(make('tick_aircraft', 'tick', actor={'flags': 1 << 12, 'has_aircraft': True}, responses=R()))
    rows.append(make('tick_refresh_sets_aircraft', 'tick', actor={'has_aircraft': True}, responses=R(1 << 12)))
    rows.append(make('tick_refresh_clears_aircraft', 'tick', actor={'flags': 1 << 12, 'has_aircraft': True},
                     responses=R(0)))
    rows.append(make('tick_speed_only_requested', 'tick', words={0x20: 0, 0x24: f2b(3.0)}, responses=R()))
    rows.append(make('tick_speed_only_current', 'tick', words={0x20: f2b(1.0), 0x24: 0}, responses=R()))
    rows.append(make('tick_negative_zero_speeds_far', 'tick', words={0x20: 0x80000000, 0x24: 0x80000000}))
    rows.append(make('tick_nan_speed', 'tick', words={0x20: 0x7fc00000, 0x24: 0}, responses=R()))
    for name, dest, mind in [('near', [1.05, 2.0], True), ('near_no_mind', [1.05, 2.0], False),
                             ('far', [10.0, 2.0], True), ('exact_radius', [1.1, 2.0], True),
                             ('just_inside', [1.0, 2.0999], True), ('same_point', [1.0, 2.0], False)]:
        rows.append(make('tick_stopped_' + name, 'tick', words={0x10: f2b(dest[0]), 0x14: f2b(dest[1]),
                                                               0x20: 0, 0x24: 0},
                         actor={'has_mind': mind}))
    rows.append(make('tick_stopped_fractional', 'tick', words={0x10: f2b(1.0 + 1 / 3), 0x14: f2b(2.0 - 1 / 7),
                                                               0x20: 0, 0x24: 0},
                     actor={'position': fb([1.3, 1.9, 0.0])}, settings={'precision_radius': f2b(0.0333333)}))
    rows.append(make('tick_stopped_with_turning_byte', 'tick', words={0x10: f2b(1.01), 0x14: f2b(2.0), 0x20: 0,
                                                                      0x24: 0, 0x1c: f2b(5.0)},
                     bytes={0x55: 1}))
    rows.append(make('tick_falling_over_inactive', 'tick', bytes={0x56: 1}, actor={'mover_active': False}))
    return rows


def small_cases():
    rows = []
    rows.append(make('stop_turning', 'stop_turning', bytes={0x55: 1}, words={0x20: f2b(2.5), 0x1c: f2b(1.0)}))
    for falling in (0, 1):
        for act in (True, False):
            rows.append(make(f'set_speed_f{falling}_a{int(act)}', 'set_speed', args=fb([4.5, 0.75]),
                             bytes={0x56: falling}, responses=[] if falling else [{'name': 'Activate', 'value': act}]))
    radius_cases = [(0, 1.5), (1, 1.5), (2, 0.75), (3, 2.0), (-1, 2.0), (1, 0.3)]
    multipliers = [None, 1.0, 1.00001, 0.99999, 1.0 / 3.0, 1.5, 1.0 + 2 ** -20, 0.0]
    for turn_type, radius in radius_cases:
        for i, multiplier in enumerate(multipliers):
            for org in (None, False, True):
                for mind in (True, False):
                    words = {0x28: f2b(6.0), 0x24: f2b(4.0), 0x34: f2b(-7.0), 0x30: f2b(-3.0)}
                    rows.append(make(f'refresh_t{turn_type}_r{radius}_m{i}_o{org}_m{mind}',
                                     'refresh_max_speed', words=words,
                                     speed_multiplier=None if multiplier is None else f2b(multiplier),
                                     definition={'turn_type': turn_type & 0xffffffff, 'turn_radius': f2b(radius),
                                                 'speed': f2b(6.0 if i % 3 else 5.5)},
                                     actor={'element_organization': org, 'has_mind': mind}))
    for name, speed, multiplier, words in [
            ('equal_requested', 4.0, 1.0, {0x28: f2b(6.0), 0x24: f2b(4.0)}),
            ('extended_above_requested', 3.0, 4.0 / 3.0, {0x28: f2b(6.0), 0x24: f2b(4.0)}),
            ('extended_below_requested', 4.0, 0.99999994, {0x28: f2b(6.0), 0x24: f2b(4.0)}),
            ('threshold_exact', 6.0001, 1.0, {0x28: f2b(6.0), 0x24: f2b(9.0)}),
            ('negative', -1.0, 1.0, {0x28: f2b(6.0), 0x24: f2b(4.0)}),
            ('third', 7.0, 1.0 / 3.0, {0x28: f2b(6.0), 0x24: f2b(4.0)})]:
        rows.append(make('refresh_' + name, 'refresh_max_speed', words=words, speed_multiplier=f2b(multiplier),
                         definition={'speed': f2b(speed)}))
    for orient in (0, 1, 0x80):
        rows.append(make(f'begin_fall_o{orient}', 'begin_fall', args=[f2b(0.25), orient],
                         actor={'facing': fb([0.6, 0.8, 0.0])}, bytes={0x6c: 1}))
    for i, (x, y, h, orient) in enumerate([(5.0, 7.0, 0.5, 0), (-3.0, 2.0, 12.0 + 2 ** -35, 1),
                                           (1.0, 2.0, 3.0, 0), (100.25, 33.125, -4.0, 1)]):
        rows.append(make(f'begin_aimed_fall_{i}', 'begin_aimed_fall', args=[f2b(0.125), f2b(x), f2b(y), orient],
                         actor={'position': fb([1.5, -2.25, 0.75])},
                         responses=[{'name': 'GroundHeight', 'f64': hex(d2b(h))}]))
    for falling in (0, 1):
        for version in (21, 22, 28):
            for word in (0, 7):
                rows.append(make(f'restore_f{falling}_v{version}_w{word}', 'restore_falling', bytes={0x56: falling},
                                 settings={'world_version': version}, updatable_word_0c=word))
    for x, y in [(2.0, 2.0), (1.9999, 50.0), (253.9999, 10.0), (254.0, 10.0), (253.99991, 10.0), (10.0, 189.9999),
                 (10.0, 190.0), (-0.0, 3.0), (3.0, 191.0)]:
        rows.append(make(f'interior_{x}_{y}', 'interior', args=fb([x, y])))
    return rows


def fall_cases(rng):
    rows = []
    def fall(name, words=None, bytes_=None, actor=None, settings=None, heights=(0.0,), arg=0.0625):
        responses = [{'name': 'GroundHeight', 'f64': hex(d2b(h))} for h in heights]
        w = {0x20: f2b(8.0), 0x58: f2b(0.5), 0x60: f2b(0.6), 0x64: f2b(0.0), 0x68: f2b(0.8)}
        w.update(words or {})
        b = {0x56: 1}
        b.update(bytes_ or {})
        a = {'position': fb([40.0, 50.0, 10.0])}
        a.update(actor or {})
        rows.append(make(name, 'fall', arg=f2b(arg), words=w, bytes=b, actor=a, settings=settings or {},
                         responses=responses))
    fall('fall_basic', heights=(-5.0,))
    fall('fall_orient', bytes_={0x5c: 1}, heights=(-5.0,))
    fall('fall_lands', heights=(50.0,))
    fall('fall_lands_equal', actor={'position': fb([1.0, 2.0, 3.0])}, words={0x20: 0, 0x68: 0, 0x60: 0},
         heights=(3.0 + (-49.05 * 0.0625) * 0.0625,))
    fall('fall_lands_orient_aircraft', bytes_={0x5c: 1}, actor={'has_aircraft': True}, heights=(-9.0,))
    fall('fall_aircraft', actor={'has_aircraft': True}, heights=(-100.0,))
    fall('fall_aircraft_lands_offset', actor={'has_aircraft': True}, heights=(-10.0,))
    fall('fall_negative_drag_product', words={0x58: f2b(40.0)}, heights=(-100.0,))
    fall('fall_zero_speed', words={0x20: 0}, heights=(-100.0,))
    fall('fall_zero_gravity_zero_speed', words={0x20: 0}, settings={'gravity': 0}, heights=(-100.0,))
    fall('fall_tiny_motion', words={0x20: f2b(1e-6)}, settings={'gravity': f2b(1e-6)}, heights=(-100.0,))
    fall('fall_outside_map_x', actor={'position': fb([255.5, 50.0, 10.0])}, heights=(-100.0,))
    fall('fall_outside_map_low', actor={'position': fb([0.5, -3.0, 10.0])}, heights=(-100.0,))
    fall('fall_outside_already_flagged', actor={'position': fb([255.5, 50.0, 10.0])}, bytes_={0x88: 1},
         heights=(-100.0,))
    fall('fall_reentered', actor={'position': fb([50.0, 50.0, 10.0])}, bytes_={0x88: 1}, heights=(-100.0,))
    fall('fall_facing_locked', actor={'flags': 1 << 17}, bytes_={0x5c: 1}, heights=(-100.0,))
    for aircraft in (False, True):
        fall(f'fall_aimed_a{int(aircraft)}', bytes_={0x6c: 1},
             words={0x70: f2b(20.0), 0x74: f2b(5.0), 0x78: f2b(-1.0), 0x7c: f2b(19.0), 0x80: f2b(3.0),
                    0x84: f2b(-4.0)}, actor={'has_aircraft': aircraft}, heights=(-50.0,))
    fall('fall_aimed_zero_delta_direction', bytes_={0x6c: 1},
         words={0x70: f2b(1.0), 0x74: f2b(2.0), 0x78: f2b(3.0), 0x7c: f2b(0.5), 0x80: f2b(0.5), 0x84: f2b(-1.0)},
         actor={'position': fb([1.0, 2.0, 3.0])}, heights=(-50.0,))
    # Generated cases cover fractional/extended paths in every branch.
    for i in range(48):
        r = lambda a, b: rng.uniform(a, b)
        aircraft = rng.random() < 0.35
        aimed = rng.random() < 0.35
        words = {0x20: f2b(r(0, 30)), 0x58: f2b(r(0, 3)), 0x60: f2b(r(-1, 1)), 0x64: f2b(r(-1, 1)),
                 0x68: f2b(r(-1, 1))}
        if aimed:
            words.update({0x70: f2b(r(5, 240)), 0x74: f2b(r(5, 180)), 0x78: f2b(r(-5, 5)), 0x7c: f2b(r(-30, 30)),
                          0x80: f2b(r(-30, 30)), 0x84: f2b(r(-20, -0.5))})
        pos = [r(-5, 260), r(-5, 195), r(-3, 40)]
        height = pos[2] + r(-8, 3) + 2 ** -37
        fall(f'fall_generated_{i}', words=words,
             bytes_={0x5c: int(rng.random() < 0.5), 0x6c: int(aimed), 0x88: int(rng.random() < 0.3)},
             actor={'position': fb(pos), 'facing': fb([r(-1, 1), r(-1, 1), r(-1, 1)]), 'has_aircraft': aircraft},
             settings={'plane_fall_gravity_tweak': f2b(r(0.001, 0.02))},
             heights=(height,), arg=r(0.01, 0.2))
    return rows


def main():
    rng = random.Random(0x8a4e5)
    cases = []
    templates = tick_cases() + small_cases() + fall_cases(rng)
    for precision in PRECISION:
        for t in templates:
            c = copy.deepcopy(t)
            c['precision'] = precision
            c['output'] = MoverMachine(c).run()
            cases.append(c)
    launch = []
    for precision in PRECISION:
        for distance, factor, offset in [(10.0, 1.0, 0.0), (3.5, 0.25, 0.1), (100.0, 2.0, 50.0), (0.0, 1.0, 0.0),
                                         (1e-3, 3.0, -2.0), (7.25, 1.5, 0.75), (2.0 / 3.0, 1.0 / 3.0, 0.0),
                                         (55.5, 1.0, 100.0)]:
            launch.append({'precision': precision, 'distance': f2b(distance), 'factor': f2b(factor),
                           'offset': f2b(offset), 'gravity': f2b(-49.05),
                           'result_f80': (launch_speed(precision, f2b(distance), f2b(factor), f2b(offset)))})
    write_document('native-mover.json', __doc__, cases, {'launch_speed': launch})


if __name__ == '__main__':
    main()
