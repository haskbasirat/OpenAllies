"""Original SAI ego load: constructor 4AB9A0 (4A8E00, 4A5C60, 96D18D with
4A5D60, 4A6B20) then KKC_SAIEgoData::LoadData 4B0560 with every sub-loader
(4AD980 Filters, 4AF500 GoalPriority, 4AF230 ResourceNeed, 4AAD80
ComponentValue, 4AF8E0 ActorPriority, 4A66E0 exploration goals, 4ADF50
RecruitPreference, 4AB090 SpecificBuildingBonus, 4AAEB0/4A7C70
SpecificRecruitRequest, 4AFD50/4AFB60/4AA1D0 general powers, 4A8700,
4AEDC0 formations, 4AF0C0 GoalFormation), the containers (40D770, 4849F0,
4A6BB0, 4A6C40, 45F180, 766DE0, hash 4A86D0/7BA3C0/606750/7A6430), CRT qsort
96CE20 with 4A5C20, 47D960 and 473AD0, all executing unchanged on seamed
data nodes (native_data.py). Seams: the node getters, 48D500 (definition
registries; found objects are mapped AT their handle, carrying the registry
index at +C and the AbilityDefinition area at +38), 4BDBA0 (Things via
[B39F58] or [[B36530]+0C]), 463FA0 (Properties), 410C40 (data error),
operator new (allocations filled with 0xCD) and delete, CRT atexit 96CAB0.

Cases: every shipped [Ego] (answers recorded from the original Data.rwd +
v14 overlays by crates/aa-mind/examples/dump_sai_inputs.rs into
sai-shipped-egos.json) at all three precisions, plus mutations of them
(answer patches) covering every validation and lookup failure of the loader
and the value-dependent paths (rounding, ms conversion, the 1/60 scale,
sorting, hash growth). Each case stores its patches, the native fault, the
number of getter/lookup events and sha256 digests of the canonical JSON
(sorted keys, compact separators, UTF-8) of the events and of the decoded
ego; successful shipped Double53 cases also store the decoded ego in full
('decoded').
Bytes the constructor leaves uninitialized (found by constructing over two
fill patterns) and pointer / container-header words are zero in the decoded
image; their contents are decoded separately.
"""
from pathlib import Path
import copy, hashlib, json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import PRECISIONS, f32, header, instructions, sha
from native_data import DataMachine
from native_saidata import registered, x87_of_f32

EGO_SIZE = 0x98C
POWER_SIZE = 0xC8
GOAL_SLOTS = 23
POISON = 0xCD
# Pointer / container-header bytes of the ego image (decoded separately).
EGO_POINTERS = [(0x8, 4), (0x10, 4)] + [(0x4c + 0x18 * t + 0x14, 4) for t in range(GOAL_SLOTS)] + [
    (0x2a4, 12), (0x2b0, 12), (0x2c0, 12), (0x374, 16), (0x384, 16), (0x3fc, 12), (0x704, 16),
    (0x73c, 16), (0x830, 16), (0x884, 12), (0x910, 12), (0x92c, 16)]
# GeneralPower: inline requirements (typed), list and vector headers, and
# the bytes 4AA1D0 leaves unwritten.
POWER_MASK = [(0x18, 0x18), (0x4b, 1), (0x4c, 8), (0x63, 1), (0x64, 8), (0xb4, 12)]
REF_LAYOUTS = {0xa17038: 'www', 0xa17048: 'www', 0xa1703c: 'www', 0xa17044: 'w', 0xa17040: 'ww',
               0xa1704c: 'wwwwwwbw'}


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False)


def digest(value):
    return hashlib.sha256(canonical(value).encode('utf-8')).hexdigest()


def signed(v):
    return v - ((v >> 31) << 32)


class EgoMachine(DataMachine):
    poison = POISON

    def __init__(self, precision, answers):
        super().__init__(precision, answers)
        self.enum_map(0xb40e88, registered('goal_types'))
        self.enum_map(0xb40eac, registered('goal_actor_proximity'))
        self.enum_map(0xb3b258, registered('component_types'))
        counts = answers.get('counts', {})
        abilities = self.alloc(0x40)
        self.w(self.root + 0x468, abilities); self.fake_managers[abilities] = 0x468
        self.w(abilities + 0x28, counts.get('Abilities', 0xdead))
        # Resource manager embedded at root+10: its registry size (+28).
        self.w(self.root + 0x38, counts.get('Resources', 0xdead))
        self.global_things = self.alloc(0x10)
        self.w(0xb39f58, self.global_things)
        self.seam(0x4bdba0, self.s_thing)
        self.seam(0x463fa0, self.s_property)
        # CRT atexit (96CAB0) registering the destructor of the static default
        # GoalPriority (B3B400, guard bit B3B418): process-exit bookkeeping.
        self.seam(0x96cab0, lambda m: self.ret(0))

    def found_object(self, kind, handle, p):
        if kind == 'Abilities':
            self.w(p + 0x38, self.answers.get('areas', {}).get(str(handle), 0xdeadbeef))

    def s_thing(self, m):
        ecx = self.ecx()
        if ecx == self.global_things:
            source = 'Global'
        elif ecx == self.r(self.root + 0xc):
            source = 'Root'
        else:
            raise AssertionError('4BDBA0 on unknown manager %#x' % ecx)
        ids, mask = self.wstr(self.arg(1)), self.arg(2)
        assert mask == 0x2b, hex(mask)
        table = self.answers.get('things', {}).get(source, {})
        if ids.lower() not in table:
            raise AssertionError('unanswered thing %s %s' % (source, ids))
        r = table[ids.lower()]
        self.ev('thing', source=source, ids=ids, result=r)
        self.ret(r or 0, 8)

    def s_property(self, m):
        assert self.ecx() == self.r(self.root + 0x464)
        ids = self.wstr(self.arg(1))
        table = self.answers.get('properties', {})
        if ids.lower() not in table:
            raise AssertionError('unanswered property %s' % ids)
        r = table[ids.lower()]
        self.ev('property', ids=ids, result=r)
        self.ret(r or 0, 4)

    def construct(self, ids, fill):
        obj = self.alloc(EGO_SIZE)
        self.u.mem_write(obj, bytes([fill]) * EGO_SIZE)
        slot = self.alloc(4); self.w(slot, self.cstring(ids))
        self.call(0x4ab9a0, ecx=obj, args=[slot], pop=4, count=1_000_000)
        return obj

    # decoding ------------------------------------------------------------
    def vector(self, base, size, fn):
        p, n = self.r(base), signed(self.r(base + 4))
        return [fn(p + size * i) for i in range(n)]

    def hash(self, base, fn):
        buckets, n = self.r(base), self.r(base + 4)
        chains = []
        for b in range(n):
            chain, e = [], self.r(buckets + 4 * b)
            while e:
                chain.append([self.r(e), fn(self.r(e + 4))]); e = self.r(e + 8)
            chains.append(chain)
        return {'count': self.r(base + 8), 'grow': self.rb(base + 0xc), 'buckets': chains}

    def list(self, header, link, fn):
        """Head-first walk of a {head, tail} list whose nodes link at +link."""
        out, p, last = [], self.r(header), 0
        while p:
            out.append(fn(p)); last = p; p = self.r(p + link)
        assert self.r(header + 4) == last, 'tail'
        return out

    def requirements(self, p):
        return {'max': self.r(p), 'flags': [self.rb(p + 4 + k) for k in range(5)],
                'properties': self.list(p + 0xc, 4, lambda n: self.r(n)), 'multiplier': self.r(p + 0x14)}

    def power(self, p):
        if not p:
            return None
        img = bytearray(self.u.mem_read(p, POWER_SIZE))
        for off, n in POWER_MASK:
            img[off:off + n] = bytes(n)
        return {'image': img.hex(), 'location': self.requirements(p + 0x18),
                'cv': self.list(p + 0x4c, 4, lambda n: self.requirements(self.r(n))),
                'value': self.list(p + 0x64, 4, lambda n: self.requirements(self.r(n))),
                'resources': self.vector(p + 0xb4, 4, self.r)}

    def decode(self, obj, mask):
        img = bytearray(self.u.mem_read(obj, EGO_SIZE))
        for off in mask:
            img[off] = 0
        for off, n in EGO_POINTERS:
            img[off:off + n] = bytes(n)
        prefs = []
        for t in range(GOAL_SLOTS):
            lp = self.r(obj + 0x4c + 0x18 * t + 0x14)
            prefs.append(None if not lp else self.list(lp, 0x18, lambda p: [
                self.r(p), self.r(p + 4), self.r(p + 8), self.rb(p + 0xc), self.r(p + 0x10),
                self.rb(p + 0x14), self.rb(p + 0x15)]))
        thing_priority = lambda p: [self.rb(p), self.r(p + 4), self.r(p + 8), signed(self.r(p + 0xc)), self.r(p + 0x10)]
        refs = []
        for ref in reversed(self.pending_refs()):
            fields = []
            for k, kind in enumerate(REF_LAYOUTS[ref['vtable']]):
                a = ref['address'] + 0xc + 4 * k
                if kind == 'b':
                    fields.append({'byte': self.rb(a)})
                else:
                    v = self.r(a)
                    fields.append({'ego': v - obj} if obj <= v < obj + EGO_SIZE else {'word': v})
            refs.append({'vtable': ref['vtable'], 'ids': ref['ids'], 'fields': fields})
        return {
            'name': self.cstr_text(self.r(obj + 0x10)),
            'image': img.hex(),
            'goal_preferences': prefs,
            'resource_needs': self.vector(obj + 0x2a4, 16, lambda p: [self.r(p + 4 * k) for k in range(4)]),
            'economic_excess': self.vector(obj + 0x2b0, 8, lambda p: [self.r(p), self.r(p + 4)]),
            'component_values': self.vector(obj + 0x2c0, 8, lambda p: [self.r(p), self.rb(p + 4)]),
            'actor_priorities': self.hash(obj + 0x374, thing_priority),
            'property_priorities': self.hash(obj + 0x384, thing_priority),
            'wall_strength': self.vector(obj + 0x3fc, 4, self.r),
            'recruit_preferences': self.hash(obj + 0x704, lambda p: self.list(p, 8, lambda n: [self.r(n), self.r(n + 4)])),
            'building_bonuses': self.hash(obj + 0x73c, lambda v: v),
            'table_830': self.hash(obj + 0x830, lambda v: v),
            'general_powers': self.vector(obj + 0x884, 4, lambda p: self.power(self.r(p))),
            'flare_priorities': self.vector(obj + 0x910, 4, self.r),
            'goal_formations': self.hash(obj + 0x92c, lambda p: [self.r(p), self.r(p + 4), self.r(p + 8)]),
            'refs': refs,
        }

    def run(self, ids, root, mask):
        obj = self.construct(ids, POISON)
        self.events.clear()
        fault = self.run_call(0x4b0560, ecx=obj, args=[self.node(root)], pop=4, count=50_000_000)
        out = {'events': self.events}
        if fault:
            out['fault'] = fault
        else:
            assert self.cstr_text(self.r(obj + 8)) == ids
            out['ego'] = self.decode(obj, mask)
        return out


def uninitialized_mask():
    """Bytes 4AB9A0 leaves unwritten: they keep the fill byte under two fills."""
    m = EgoMachine('Double53', {'nodes': {}, 'managers': {}})
    a = bytes(m.u.mem_read(m.construct('mask', 0xcd), EGO_SIZE))
    b = bytes(m.u.mem_read(m.construct('mask', 0x5a), EGO_SIZE))
    return [i for i in range(EGO_SIZE) if a[i] != b[i]]


# mutations --------------------------------------------------------------
def x87(sig, exp, sign=0):
    return {'significand': sig, 'sign_exponent': (sign << 15) | exp}


def val(v):
    """80-bit image of a binary32-representable Python float."""
    return x87_of_f32(f32(v))


ABOVE_ONE = x87((1 << 63) | (1 << 33), 16383)      # 1 + 2^-30: binary32 rounds to 1.0
BELOW_ZERO = x87(1 << 63, 16383 - 40, 1)           # -2^-40


class Patcher:
    def __init__(self, base):
        self.a = copy.deepcopy(base)
        self.patches = []

    def set(self, path, value):
        d = self.a
        for k in path[:-1]:
            d = d.setdefault(k, {})
        d[path[-1]] = copy.deepcopy(value)
        self.patches.append([list(path), value])

    def node(self, nid):
        return self.a['nodes'][str(nid)]

    def get(self, nid, getter, name):
        return self.node(nid).get(getter, {}).get(name.lower())

    def answer(self, nid, getter, name, value):
        self.set(['nodes', str(nid), getter, name.lower()], value)

    def section(self, nid, name):
        return self.get(nid, 'section', name)

    def items(self, nid, name):
        n = self.get(nid, 'count', name) or 0
        return [self.get(nid, 'item', '%s#%d' % (name, i)) for i in range(n)]

    def new_node(self, answers):
        nid = max(int(k) for k in self.a['nodes']) + 1
        self.set(['nodes', str(nid)], answers)
        return nid

    def set_items(self, nid, name, children):
        self.answer(nid, 'count', name, len(children))
        for i, c in enumerate(children):
            self.answer(nid, 'item', '%s#%d' % (name, i), c)

    def path(self, root, *names):
        n = root
        for name in names:
            n = self.section(n, name)
        return n


def mutate(rnd, case, n):
    p = Patcher(case['answers'])
    root = case['root']
    ge = p.path(root, 'GoalEngine')
    combat = p.path(ge, 'Combat')
    rai = p.path(root, 'ReactiveAI')
    names = ['missing', 'fixed', 'distance', 'teamwork', 'balancing', 'exponent', 'recover', 'military',
             'depth', 'goal_priority', 'preference', 'actor_priority', 'component', 'resource_need',
             'recruit_pref', 'building_bonus', 'recruit_request', 'wall', 'combined', 'excess', 'power',
             'requirements', 'flare', 'stopped', 'default_formation', 'goal_formation', 'templates',
             'reaction', 'solvency', 'explore']
    kind = names[n % len(names)]
    things = lambda src: p.a.setdefault('things', {}).setdefault(src, {})
    if kind == 'missing':
        nodes = [k for k in p.a['nodes'] if any(g in p.a['nodes'][k] for g in ('fixed', 'int', 'bool', 'string'))]
        k = rnd.choice(nodes)
        getter = rnd.choice([g for g in ('fixed', 'int', 'bool', 'string') if p.a['nodes'][k].get(g)])
        name = rnd.choice(sorted(p.a['nodes'][k][getter]))
        p.answer(k, getter, name, {'error': "Node doesn't contain field."})
    elif kind == 'fixed':
        for _ in range(6):
            k = rnd.choice([k for k in p.a['nodes'] if p.a['nodes'][k].get('fixed')])
            name = rnd.choice(sorted(p.a['nodes'][k]['fixed']))
            p.answer(k, 'fixed', name, rnd.choice([val(0.0), val(-2.5), val(1.0), val(0.3), ABOVE_ONE, BELOW_ZERO,
                                                   val(rnd.uniform(-5, 50))]))
    elif kind == 'distance':
        av = p.path(ge, 'ActorValue')
        p.answer(av, 'fixed', 'distance_mult_max', val(2.0))
        p.answer(av, 'fixed', 'distance_mult_min', rnd.choice([val(2.0), val(2.5), ABOVE_ONE, val(0.0), BELOW_ZERO, val(-1.0)]))
    elif kind == 'teamwork':
        node, name = rnd.choice([(p.path(combat, 'Teamwork'), 'attack_allied_influence_cv_multiplier'),
                                 (p.path(combat, 'Teamwork'), 'defend_allied_influence_cv_multiplier'),
                                 (combat, 'congruency_min_multiplier')])
        p.answer(node, 'fixed', name, rnd.choice([val(0.0), val(1.0), ABOVE_ONE, BELOW_ZERO, val(1.5), val(0.75)]))
    elif kind == 'balancing':
        p.answer(p.path(combat, 'Bombardement'), 'fixed', 'balancing_constant',
                 rnd.choice([val(0.0), val(-1.0), BELOW_ZERO, val(1e-30), val(2.0)]))
    elif kind == 'exponent':
        p.answer(p.path(ge, 'Construction'), 'fixed', 'construct_city_defense_exponent',
                 rnd.choice([val(0.0), x87(0, 0, 1), BELOW_ZERO, val(-0.5), val(3.0)]))
    elif kind == 'recover':
        name = rnd.choice(['max_hp', 'max_morale', 'critical_morale'])
        p.answer(p.path(ge, 'Recover'), 'fixed', name, rnd.choice([val(0.0), val(1.0), ABOVE_ONE, BELOW_ZERO,
                                                                    val(1.001), val(0.4)]))
    elif kind == 'military':
        p.answer(p.path(ge, 'Recruiting'), 'int', 'num_military_goals', rnd.choice([0, -3, 1, 9]))
    elif kind == 'depth':
        p.answer(p.path(root, 'Learning'), 'int', 'defend_depth', rnd.choice([-1, 0, 5]))
    elif kind == 'goal_priority':
        items = p.items(ge, 'GoalPriority')
        sub = rnd.choice(['unknown', 'duplicate', 'case', 'swap'])
        if sub == 'unknown':
            p.answer(rnd.choice(items), 'string', 'goal_ids', rnd.choice(['ATTACK', 'bombard_region', '']))
        elif sub == 'duplicate':
            p.set_items(ge, 'GoalPriority', items + [rnd.choice(items)])
        elif sub == 'case':
            it = rnd.choice(items)
            p.answer(it, 'string', 'goal_ids', p.get(it, 'string', 'goal_ids').lower())
        else:
            a, b = rnd.sample(items, 2)
            ga, gb = p.get(a, 'string', 'goal_ids'), p.get(b, 'string', 'goal_ids')
            p.answer(a, 'string', 'goal_ids', gb); p.answer(b, 'string', 'goal_ids', ga)
    elif kind == 'preference':
        prefs = [(gp, i) for gp in p.items(ge, 'GoalPriority') for i in p.items(gp, 'ActorPreference')]
        gp_item, it = rnd.choice(prefs)
        sub = rnd.choice(['both', 'neither', 'actor', 'unknown_actor', 'unknown_property'])
        if sub == 'both':
            p.answer(it, 'string', 'actor_ids', 'mutation_actor')
            p.answer(it, 'string', 'property_ids', 'mutation_property')
        elif sub == 'neither':
            p.answer(it, 'string', 'actor_ids', ''); p.answer(it, 'string', 'property_ids', '')
        elif sub in ('actor', 'unknown_actor'):
            p.answer(it, 'string', 'actor_ids', 'mutation_actor'); p.answer(it, 'string', 'property_ids', '')
            p.set(['things', 'Global', 'mutation_actor'], None if sub == 'unknown_actor' else 0x30000400)
        else:
            p.answer(it, 'string', 'actor_ids', ''); p.answer(it, 'string', 'property_ids', 'mutation_property')
            p.set(['properties', 'mutation_property'], None)
    elif kind == 'actor_priority':
        items = p.items(ge, 'ActorPriority')
        it = rnd.choice(items)
        sub = rnd.choice(['both', 'neither', 'actor', 'unknown_actor', 'unknown_property', 'duplicate', 'many'])
        if sub == 'both':
            p.answer(it, 'string', 'actor_ids', 'mutation_actor'); p.answer(it, 'string', 'property_ids', 'x')
        elif sub == 'neither':
            p.answer(it, 'string', 'actor_ids', ''); p.answer(it, 'string', 'property_ids', '')
        elif sub in ('actor', 'unknown_actor'):
            p.answer(it, 'string', 'actor_ids', 'mutation_actor'); p.answer(it, 'string', 'property_ids', '')
            p.set(['things', 'Global', 'mutation_actor'], None if sub == 'unknown_actor' else 0x30000440)
        elif sub == 'unknown_property':
            prop = p.get(it, 'string', 'property_ids') or 'mutation_property'
            p.answer(it, 'string', 'property_ids', prop); p.answer(it, 'string', 'actor_ids', '')
            p.set(['properties', prop.lower()], None)
        elif sub == 'duplicate':
            p.set_items(ge, 'ActorPriority', items + [rnd.choice(items)])
        else:
            extra = []
            for k in range(rnd.randrange(10, 40)):
                ids = 'mutation_thing_%d' % k
                extra.append(p.new_node({'string': {'property_ids': '', 'actor_ids': ids},
                                         'bool': {'count_all_property_actors': k % 2 == 0},
                                         'fixed': {'value': val(k), 'repeat_penalty': val(0.5), 'one_time_bonus': val(1.0)},
                                         'int': {'max_recruited_per_think': k - 3}}))
                p.set(['things', 'Global', ids], 0x30000000 + 0x40 * rnd.randrange(0, 4000))
            p.set_items(ge, 'ActorPriority', items + extra)
    elif kind == 'component':
        av = p.path(ge, 'ActorValue')
        items = p.items(av, 'ComponentValue')
        new = p.new_node({'string': {'component_ids': rnd.choice(['HQ', 'hq', 'Wall', 'NOT_A_COMPONENT', 'BODY'])},
                          'fixed': {'value': val(rnd.uniform(0, 3))}, 'bool': {'new_construction_only': rnd.random() < 0.5}})
        p.set_items(av, 'ComponentValue', items + [new])
    elif kind == 'resource_need':
        rm = p.path(ge, 'ResourceManagement')
        items = p.items(rm, 'ResourceNeed')
        new = p.new_node({'string': {'resource_ids': rnd.choice(['', 'mutation_resource', 'mutation_far_resource'])},
                          'fixed': {'short_need': val(1), 'enough_need': val(2), 'plenty_value': val(3), 'plenty_need': val(4)}})
        p.set(['managers', 'Resources', 'mutation_resource'], None)
        p.set(['managers', 'Resources', 'mutation_far_resource'], [0x3c000000 + 0x40 * 11, 11])
        p.set_items(rm, 'ResourceNeed', items + [new])
    elif kind == 'recruit_pref':
        rq = p.path(ge, 'Recruiting')
        prefs = []
        for k in range(rnd.randrange(1, 6)):
            matches = []
            for j in range(rnd.choice([0, 1, 2, 3]) if k else rnd.choice([1, 2, 3])):
                ids = 'mutation_match_%d_%d' % (k, j)
                p.set(['things', 'Root', ids], None if rnd.random() < 0.05 else 0x30000000 + 0x40 * rnd.randrange(4000))
                matches.append(p.new_node({'string': {'unit_ids': ids}, 'fixed': {'priority': val(rnd.uniform(0, 9))}}))
            ids = 'mutation_unit_%d' % rnd.randrange(4)
            if ids not in things('Root'):
                p.set(['things', 'Root', ids], None if rnd.random() < 0.1 else 0x30000000 + 0x40 * rnd.randrange(4000))
            pref = p.new_node({'string': {'unit_ids': ids}})
            p.set_items(pref, 'Match', matches)
            prefs.append(pref)
        p.set_items(rq, 'RecruitPreference', prefs)
    elif kind == 'building_bonus':
        rq = p.path(ge, 'Recruiting')
        bonuses = []
        for k in range(rnd.randrange(1, 6)):
            ids = rnd.choice(['', 'mutation_building_%d' % rnd.randrange(5)])
            if ids and ids not in things('Global'):
                p.set(['things', 'Global', ids], None if rnd.random() < 0.1 else 0x30000000 + 0x40 * rnd.randrange(4000))
            bonuses.append(p.new_node({'string': {'building_ids': ids}, 'fixed': {'bonus': val(rnd.uniform(0, 4))}}))
        p.set_items(rq, 'SpecificBuildingBonus', bonuses)
    elif kind == 'recruit_request':
        rq = p.path(ge, 'Recruiting')
        items = p.items(rq, 'SpecificRecruitRequest')
        it = rnd.choice(items)
        r, q = rnd.choice([('', ''), ('unit_x', 'prop_y'), ('', 'prop_y'), ('unit_x', '')])
        p.answer(it, 'string', 'recruit_ids', r); p.answer(it, 'string', 'property_ids', q)
    elif kind == 'wall':
        ws = p.path(combat, 'WallStrength')
        levels = []
        for k in range(rnd.randrange(1, 6)):
            level = rnd.choice([0, 0, 1, 2, 3, 5, -1])
            value = rnd.choice([val(0.0), val(1.5), val(0.25), x87(0, 0, 1), val(-1.0)])
            levels.append(p.new_node({'int': {'level': level}, 'fixed': {'value': value}}))
        p.set_items(ws, 'SettlementLevel', levels)
    elif kind == 'combined':
        sec = rnd.choice(['CombinedArmsGeneral', 'CombinedArmsWalls'])
        node = p.path(combat, sec)
        els = [p.new_node({'string': {'property': 'mutation_element_%d' % k},
                           'fixed': {'max_percent': val(rnd.uniform(0, 1)), 'max_multiplier': val(rnd.uniform(1, 3))}})
               for k in range(rnd.randrange(1, 4))]
        p.set_items(node, 'Element', els)
    elif kind == 'excess':
        rows = [p.new_node({'fixed': {'quantity': val(rnd.uniform(0, 100)),
                                      'bonus': val(rnd.choice([0.0, 1.0, 1.0, 2.5, -3.0, rnd.uniform(-5, 5)]))}})
                for k in range(rnd.randrange(2, 14))]
        p.set_items(ge, 'EconomicExcess', rows)
    elif kind == 'power':
        gpw = p.path(ge, 'GeneralPowers')
        items = p.items(gpw, 'Power')
        it = rnd.choice(items)
        sub = rnd.choice(['unknown', 'duplicate', 'location', 'resource', 'resource_unknown'])
        if sub == 'unknown':
            ids = p.get(it, 'string', 'ability_ids')
            p.set(['managers', 'Abilities', ids.lower()], None)
        elif sub == 'duplicate':
            p.set_items(gpw, 'Power', items + [it])
        elif sub == 'location':
            p.answer(p.path(it, 'Location'), 'string', 'type', rnd.choice(['NONE', 'FOG', 'fog', 'near_actor', 'bogus']))
        else:
            eg = p.path(it, 'Priority', 'EconomyGenerated')
            res = p.items(eg, 'Resource')
            ids = 'mutation_resource'
            p.set(['managers', 'Resources', ids], None if sub == 'resource_unknown' else [0x3c000000 + 0x40 * 99, 99])
            p.set_items(eg, 'Resource', res + [p.new_node({'string': {'ids': ids}, 'fixed': {'amount': val(5)}})])
    elif kind == 'requirements':
        gpw = p.path(ge, 'GeneralPowers')
        it = rnd.choice(p.items(gpw, 'Power'))
        cats = p.items(p.path(it, 'Priority', 'CV'), 'Category') + p.items(p.path(it, 'Priority', 'ActorValue'), 'Category')
        node = rnd.choice([p.path(it, 'Location', 'ActorRequirements')] + cats)
        sub = rnd.choice(['health', 'flags', 'bad_property', 'dup_property'])
        if sub == 'health':
            p.answer(node, 'fixed', 'max_percent_health', rnd.choice([val(0.0), val(1.0), ABOVE_ONE, val(1.5), BELOW_ZERO, val(0.5)]))
        elif sub == 'flags':
            for name in ('must_be_engaged', 'must_not_be_engaged', 'must_be_friendly', 'must_be_enemy', 'must_be_owned'):
                p.answer(node, 'bool', name, rnd.random() < 0.5)
        else:
            props = p.items(node, 'RequiredProperty')
            if sub == 'bad_property':
                new = p.new_node({'string': {'ids': 'mutation_property'}})
                p.set(['properties', 'mutation_property'], None)
                p.set_items(node, 'RequiredProperty', props + [new])
            elif props:
                p.set_items(node, 'RequiredProperty', props + [rnd.choice(props)])
            else:
                new = p.new_node({'string': {'ids': 'mutation_property'}})
                p.set(['properties', 'mutation_property'], 0x34000000 + 0x40 * 77)
                p.set_items(node, 'RequiredProperty', [new, new])
    elif kind == 'flare':
        fl = p.path(ge, 'InternalFlares')
        rows = [p.new_node({'string': {'goal': rnd.choice(['BOMBARD', 'attack_actor', 'ATTACH', 'NOT_A_GOAL'])},
                            'fixed': {'priority': val(rnd.uniform(0, 9))}}) for k in range(rnd.randrange(1, 5))]
        p.set_items(fl, 'AttackBombarderGoalPriority', rows)
    elif kind == 'stopped':
        p.answer(rai, 'string', 'stopped_formation', rnd.choice(['', 'mutation_formation', 'mutation_known_formation']))
        p.set(['managers', 'Formations', 'mutation_formation'], None)
        p.set(['managers', 'Formations', 'mutation_known_formation'], [0x3a000000 + 0x40 * 3, 3])
    elif kind == 'default_formation':
        df = p.path(rai, 'DefaultFormation')
        name = rnd.choice(['unthreatened_formation', 'threatened_formation', 'combat_formation'])
        p.answer(df, 'string', name, rnd.choice(['', 'mutation_formation']))
        p.set(['managers', 'Formations', 'mutation_formation'], None)
    elif kind == 'goal_formation':
        items = p.items(rai, 'GoalFormation')
        it = rnd.choice(items)
        sub = rnd.choice(['unknown_goal', 'duplicate', 'empty', 'partial', 'unknown_formation'])
        if sub == 'unknown_goal':
            p.answer(it, 'string', 'goal_ids', 'NOT_A_GOAL')
        elif sub == 'duplicate':
            p.set_items(rai, 'GoalFormation', items + [it])
        else:
            names = ['unthreatened_formation', 'threatened_formation', 'combat_formation']
            for k, name in enumerate(names):
                if sub == 'empty' or (sub == 'partial' and k != 1):
                    p.answer(it, 'string', name, '')
            if sub == 'unknown_formation':
                p.answer(it, 'string', rnd.choice(names), 'mutation_formation')
                p.set(['managers', 'Formations', 'mutation_formation'], None)
    elif kind == 'templates':
        for name in ('SettlementTemplate', 'CompanyTemplate'):
            rows = [p.new_node({'string': {'ids': '%s_%d' % (name.lower(), k)}}) for k in range(rnd.randrange(0, 4))]
            p.set_items(root, name, rows)
    elif kind == 'reaction':
        for node in (p.path(ge, 'GeneralSettings'), rai):
            p.answer(node, 'fixed', 'reaction_time', rnd.choice([val(0.0004), val(0.0005), val(0.0015), val(0.1),
                                                                 val(2.0), val(rnd.uniform(0, 30)), val(-0.0015)]))
    elif kind == 'solvency':
        p.answer(p.path(ge, 'Destroy'), 'fixed', 'min_seconds_solvency',
                 rnd.choice([val(3.0), val(7.0), val(13.0), val(1e-7), val(rnd.uniform(0, 600)), x87((1 << 63) | 0x5555, 16390)]))
    elif kind == 'explore':
        ex = p.path(ge, 'Exploration')
        node = p.path(ex, rnd.choice(['CloseGoals', 'DeepGoals', 'AirReconGoals']))
        for name in sorted(p.node(node).get('fixed', {}))[:12]:
            p.answer(node, 'fixed', name, val(rnd.uniform(-3, 30)))
        for name in sorted(p.node(node).get('bool', {})):
            p.answer(node, 'bool', name, rnd.random() < 0.5)
    return kind, p.patches, p.a


def cases():
    shipped = json.loads((HERE / 'sai-shipped-egos.json').read_text(encoding='utf-8'))['egos']
    base = [{'ids': e['ids'], 'root': e['root'],
             'answers': {k: e[k] for k in ('nodes', 'managers', 'things', 'properties', 'counts', 'areas')}}
            for e in shipped]
    rows = []
    for i, e in enumerate(base):
        for prec in PRECISIONS:
            rows.append(({'name': 'shipped_%s_%s' % (e['ids'], prec), 'ego': i, 'precision': prec, 'patches': []},
                         e, e['answers']))
    rnd = random.Random(0x4b0560)
    precisions = list(PRECISIONS)
    for n in range(270):
        i = rnd.randrange(len(base))
        kind, patches, answers = mutate(rnd, base[i], n)
        rows.append(({'name': 'mutated%d_%s' % (n, kind), 'ego': i, 'precision': precisions[n % 3],
                      'patches': patches}, base[i], answers))
    return rows


def run_case(job):
    case, ego, answers, mask = job
    m = EgoMachine(case['precision'], answers)
    try:
        result = m.run(ego['ids'], ego['root'], mask)
    except Exception as e:  # harness problem (e.g. an unanswered question): reported, never a result
        return dict(case, harness_error=repr(e)), {}
    m.verify_v14()
    row = dict(case)
    row.update(events=len(result['events']), events_sha256=digest(result['events']))
    if 'fault' in result:
        row['fault'] = result['fault']
        row['last_events'] = result['events'][-3:]
    else:
        row['ego_sha256'] = digest(result['ego'])
        if case['name'].startswith('shipped_') and case['precision'] == 'Double53':
            row['decoded'] = result['ego']
    return row, m.visited


def main():
    only = sys.argv[1] if len(sys.argv) > 1 else None
    mask = uninitialized_mask()
    if only:
        for case, ego, answers in cases():
            if case['name'] == only:
                m = EgoMachine(case['precision'], answers)
                result = m.run(ego['ids'], ego['root'], mask)
                if len(sys.argv) > 2:
                    Path(sys.argv[2]).write_text(json.dumps(result, indent=1), encoding='utf-8')
                print(json.dumps({'fault': result.get('fault'), 'events': len(result['events']),
                                  'last': result['events'][-3:]}, indent=1))
        return
    from multiprocessing import Pool
    jobs = [(case, ego, answers, mask) for case, ego, answers in cases()]
    out, visited = [], {}
    with Pool(4) as pool:
        for row, seen in pool.imap(run_case, jobs, chunksize=2):
            out.append(row); visited.update(seen)
    errors = [(r['name'], r['harness_error']) for r in out if 'harness_error' in r]
    if errors:
        for e in errors:
            print(e)
        raise SystemExit('%d harness errors' % len(errors))
    faults = sum('fault' in r for r in out)
    d = header(__file__, __doc__)
    d.update(inputs='sai-shipped-egos.json', inputs_sha256=sha((HERE / 'sai-shipped-egos.json').read_bytes()),
             uninitialized=mask, instructions=instructions(visited), cases=out)
    p = HERE / 'native-saidata-ego.json'
    p.write_text(json.dumps(d, separators=(',', ':')) + '\n', encoding='utf-8')
    body = [a for a in visited if 0x4b0560 <= a < 0x4b0560 + 8889]
    print(json.dumps({'cases': len(out), 'faults': faults, 'instructions': len(visited),
                      'loader_instructions': len(body), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
