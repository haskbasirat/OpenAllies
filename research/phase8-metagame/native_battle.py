"""Original metagame quick-battle and destroy-selection code under Unicorn.

Executes unchanged original x86 (checked byte-identical in the v14 image):
  85D840 metagame LCG draw
  93EB70/93EBD0 army quick-battle strength (division damage sum + army damage)
  954FD0 defender strength, 955EB0 defending kingdom, 95F270 technology bonus,
  95F2F0 odds clamp, 95F370 attack odds, 95F480 KKC_QuickBattleSolver::SolveBattle
  954D80 via 954EB0/954ED0/954EF0/954F10 (weakest/strongest/most/least expensive)
    with comparators 93EC40/93ECA0/93F110/93F080, cost 93EA90, shop finds 961810/961840
  9564F0 random destroy selection (955420/9554F0 nth-army filters)
  95C7B0 destroy-behaviour enum registration
under x87 precision control 24, 53 and 64 bits.

Territory +24 is the owning (controlling) kingdom; +1C counts the armies whose
kingdom the owner regards as an enemy (kingdom actor vtable+140, cf. 954B30
'enemy territory required'). Battle sides: byte 0 = those enemies (attackers),
byte 1 = the others (defenders).

Explicit seams (synthetic inputs, never claimed as native behaviour):
  army actor vtable+108 (owning kingdom actor), kingdom actor vtable+140
  (enemy relation) and vtable+138 (second relation, ally-like; used by 955EB0),
  404F60 debug log (arguments captured),
  6F7810/6F7430 allocator, 4849F0 vector storage, 4D7960 actor kill and 956280
  territory removal (selection recorded, state untouched), 95C6A0 enum insert.
"""
import json, random, struct, hashlib, sys
from pathlib import Path
from mg_machine import Machine, P, V, VISITED, ext_to_json, STACK_TOP, HERE
from unicorn.x86_const import *

PCS = {'single24': 0x007f, 'double53': 0x027f, 'extended64': 0x037f}
VT_ARMY = 0x2f80000      # fake vtables live in the heap mapping
VT_KINGDOM = 0x2f81000
S108 = 0x2f82000         # seam entry points (never executed: hooked)
S138 = 0x2f82010
S140 = 0x2f82020
H_GEN0 = 0x4100

class World:
    def __init__(self, m, case):
        self.m = m
        self.case = case
        m.u.mem_write(S108, b'\xc3' * 0x40)
        for vt in (VT_ARMY, VT_KINGDOM):
            for off in range(0, 0x200, 4):
                m.w32(vt + off, 0)
        m.w32(VT_ARMY + 0x108, S108)
        m.w32(VT_KINGDOM + 0x138, S138)
        m.w32(VT_KINGDOM + 0x140, S140)
        self.table = m.alloc(0x20004 + 4 * 0x400)
        m.w32(0xb364c0, self.table)
        self.next_handle = 1
        # kingdom actors
        self.kingdom_actor = []
        self.actor_index = {}
        for i in range(case['kingdom_actor_count']):
            a = m.alloc(0x40)
            m.w32(a, VT_KINGDOM)
            self.kingdom_actor.append(a)
            self.actor_index[a] = ('kingdom_actor', i)
        # technologies and divisions are identity objects
        self.tech = [m.alloc(0x10) for _ in range(case['technology_count'])]
        self.division = []
        for bits in case['division_damage_bits']:
            d = m.alloc(0x40)
            m.w32(d + 0x34, bits)
            self.division.append(d)
        # MG kingdoms (metagame state kingdom list)
        self.mg = []
        for k in case['mg_kingdoms']:
            shop = m.alloc(0x40)
            entries = []
            for tech, cost in k['shop_technologies']:
                e = m.alloc(0x10); m.w32(e + 4, cost); m.w32(e + 8, self.tech[tech]); entries.append(e)
            m.w32(shop + 0x24, m.array(entries)); m.w32(shop + 0x28, len(entries))
            entries = []
            for div, cost in k['shop_divisions']:
                e = m.alloc(0x10); m.w32(e + 4, cost); m.w32(e + 8, self.division[div]); entries.append(e)
            m.w32(shop + 0x18, m.array(entries)); m.w32(shop + 0x1c, len(entries))
            data = m.alloc(0x80); m.w32(data + 0x60, shop)
            mgk = m.alloc(0x40)
            m.w32(mgk + 8, data)
            m.w32(mgk + 0xc, self.handle(self.kingdom_actor[k['actor']]))
            owned = [self.tech[t] for t in k['owned_technologies']]
            m.w32(mgk + 0x20, m.array(owned)); m.w32(mgk + 0x24, len(owned))
            self.mg.append(mgk)
        state = m.alloc(0x40)
        m.w32(state + 0x18, case['rng_state'])
        m.w32(state + 0x1c, m.array(self.mg)); m.w32(state + 0x20, len(self.mg))
        m.w32(0xb49fa0, state)
        self.state = state
        settings = m.alloc(0x300)
        bonus = m.alloc(4); m.w32(bonus, case['technology_bonus_bits'])
        prob = m.alloc(4); m.w32(prob, case['attack_win_probability_bits'])
        m.w32(settings + 0x208, bonus); m.w32(settings + 0x20c, prob)
        m.w32(0xb39ab0, settings)
        # armies
        self.army = []
        self.component = []
        refs = []
        for a in case['armies']:
            g = m.alloc(0x40); m.w32(g + 0x14, a['army_damage_bits'])
            definition = m.alloc(0x600); m.w32(definition + 0x5b8, g)
            actor = m.alloc(0x100)
            m.w32(actor, VT_ARMY); m.w32(actor + 4, definition)
            comp = m.alloc(0x40)
            m.w32(comp + 4, actor)
            divs = [self.division[d] for d in a['divisions']]
            m.w32(comp + 0x14, m.array(divs)); m.w32(comp + 0x18, len(divs))
            m.w32(actor + 0xcc, comp)
            ref = m.alloc(0x10); m.w32(ref + 4, self.handle(actor))
            refs.append(ref)
            self.actor_index[actor] = ('army', len(self.army))
            self.army.append(actor)
            self.component.append(comp)
        territory = m.alloc(0x40)
        m.w32(territory + 0x10, m.array(refs)); m.w32(territory + 0x14, len(refs))
        m.w32(territory + 0x1c, case['enemy_army_count'])
        m.w32(territory + 0x24, 0 if case['owner'] is None else self.mg[case['owner']])
        self.territory = territory
        self.solver = m.alloc(0x20); m.w32(self.solver + 4, territory)
        self.log = []
        self.destroyed = []
        m.seams[S108] = self.vf108
        m.seams[S138] = lambda mm: self.relation(mm, 'relation_138')
        m.seams[S140] = lambda mm: self.relation(mm, 'enemy_140')
        m.seams[0x404f60] = self.debug_log
        m.seams[0x6f7810] = lambda mm: mm.ret(0, mm.alloc(mm.arg(0) or 4))
        m.seams[0x6f7430] = lambda mm: mm.ret(0)
        m.seams[0x4849f0] = self.vector_storage
        m.seams[0x4d7960] = lambda mm: mm.ret(0)
        m.seams[0x956280] = self.remove_army
    def handle(self, ptr):
        idx = self.next_handle; self.next_handle += 1
        gen = H_GEN0 + idx
        self.m.w16(self.table + 4 + idx * 2, gen)
        self.m.w32(self.table + 0x20004 + idx * 4, ptr)
        return (gen << 16) | idx
    def vf108(self, m):
        kind, i = self.actor_index[m.u.reg_read(UC_X86_REG_ECX)]
        assert kind == 'army'
        m.ret(0, self.kingdom_actor[self.case['armies'][i]['kingdom_actor']])
    def relation(self, m, name):
        kind, i = self.actor_index[m.u.reg_read(UC_X86_REG_ECX)]
        other = self.actor_index[m.arg(0)]
        assert kind == 'kingdom_actor' and other[0] == 'kingdom_actor'
        m.ret(4, int(self.case[name][i][other[1]]))
    def debug_log(self, m):
        sp = m.esp()
        doubles = [struct.unpack('<Q', bytes(m.u.mem_read(sp + 0x18 + 8 * k, 8)))[0] for k in range(2)]
        self.log.append({'line': m.r32(sp + 0xc), 'double_bits': [f'{d:016x}' for d in doubles]})
        m.ret(0)
    def vector_storage(self, m):
        ecx = m.u.reg_read(UC_X86_REG_ECX); n = m.arg(0)
        m.w32(ecx, m.alloc(max(4, 4 * n))); m.w32(ecx + 8, n)
        m.ret(4)
    def remove_army(self, m):
        self.destroyed.append(self.army_of(m.arg(0)))
        m.ret(4, 1)
    def army_of(self, ptr):
        if ptr == 0:
            return None
        kind, i = self.actor_index[ptr]
        assert kind == 'army'
        return i
    def mg_of(self, ptr):
        return None if ptr == 0 else self.mg.index(ptr)

def f32(x): return struct.unpack('<I', struct.pack('<f', x))[0]

def gen_case(rng, index):
    n_actor = rng.randint(1, 6)
    n_mg = rng.randint(1, n_actor)
    mg_actors = rng.sample(range(n_actor), n_mg)
    n_tech = rng.randint(0, 8)
    n_div = rng.randint(1, 8)
    damage_pool = [0.0, 0.05, 0.13, 0.15, 0.18, 0.19, 0.23, 0.25, 0.28, 0.1, 0.3333333, 1e-3]
    def dmg():
        return f32(rng.choice(damage_pool)) if rng.random() < 0.7 else f32(rng.uniform(0, 0.5))
    case = {
        'index': index,
        'kingdom_actor_count': n_actor,
        'technology_count': n_tech,
        'division_damage_bits': [dmg() for _ in range(n_div)],
        'mg_kingdoms': [],
        'armies': [],
        'enemy_140': None,
        'relation_138': None,
        'rng_state': rng.getrandbits(32),
        'technology_bonus_bits': f32(0.05) if rng.random() < 0.6 else f32(rng.uniform(0, 0.2)),
        'attack_win_probability_bits': f32(0.47) if rng.random() < 0.6 else f32(rng.uniform(0, 1.2)),
    }
    # mostly team-shaped relations (enemy = other team, 138 = same team), some noise
    team = [rng.randrange(3) for _ in range(n_actor)]
    noisy = rng.random() < 0.25
    case['enemy_140'] = [[(team[a] != team[b]) != (noisy and rng.random() < 0.3) for b in range(n_actor)] for a in range(n_actor)]
    case['relation_138'] = [[(team[a] == team[b]) != (noisy and rng.random() < 0.3) for b in range(n_actor)] for a in range(n_actor)]
    costs = [25, 40, 50, 75, 100, 125, 150, 0, 7, 33]
    for a in mg_actors:
        case['mg_kingdoms'].append({
            'actor': a,
            'owned_technologies': rng.sample(range(n_tech), rng.randint(0, n_tech)),
            'shop_technologies': [(t, rng.choice(costs)) for t in rng.sample(range(n_tech), rng.randint(0, n_tech))],
            'shop_divisions': [(d, rng.choice(costs)) for d in rng.sample(range(n_div), rng.randint(0, n_div))],
        })
    for _ in range(rng.randint(0, 9)):
        case['armies'].append({
            'kingdom_actor': rng.randrange(n_actor),
            'army_damage_bits': dmg(),
            'divisions': [rng.randrange(n_div) for _ in range(rng.randint(0, 9))],
        })
    owner = rng.randrange(n_mg) if rng.random() < 0.9 else None
    case['owner'] = owner
    if owner is None:
        count = len(case['armies']) if rng.random() < 0.5 else rng.randint(0, len(case['armies']))
    else:
        actor = case['mg_kingdoms'][owner]['actor']
        count = sum(case['enemy_140'][actor][a['kingdom_actor']] for a in case['armies'])
        if rng.random() < 0.1:
            count = rng.randint(0, len(case['armies']))   # stored count is an input word
    case['enemy_army_count'] = count
    return case

def run_case(case, pc_name):
    m = Machine(PCS[pc_name])
    w = World(m, case)
    out = {'precision': pc_name}
    out['army_strength'] = [ext_to_json(m.call(0x93ebd0, ecx=c, store_st0=True)[1]) for c in w.component]
    out['defender_strength'] = ext_to_json(m.call(0x954fd0, ecx=w.territory, store_st0=True)[1])
    out['defending_kingdom'] = w.mg_of(m.call(0x955eb0, ecx=w.territory)[0])
    out['technology_bonus'] = [ext_to_json(m.call(0x95f270, args=(k,), ecx=w.solver, store_st0=True)[1]) for k in w.mg + [0]]
    out['attack_odds'] = ext_to_json(m.call(0x95f370, ecx=w.solver, store_st0=True)[1])
    selections = {}
    for name, fn in (('weakest', 0x954eb0), ('strongest', 0x954ed0), ('most_expensive', 0x954ef0), ('least_expensive', 0x954f10)):
        for side in (0, 1):
            selections[f'{name}_{side}'] = w.army_of(m.call(fn, args=(side,), ecx=w.territory)[0])
    out['selection'] = selections
    # SolveBattle consumes one draw from the metagame state RNG (+18)
    w.log.clear()
    eax, _ = m.call(0x95f480, ecx=w.solver)
    out['solve'] = {'result': eax, 'log': list(w.log), 'rng_state_after': m.r32(w.state + 0x18)}
    random_out = {}
    for side in (0, 1):
        m.w32(w.state + 0x18, case['rng_state'])
        w.destroyed.clear()
        eax, _ = m.call(0x9564f0, args=(side,), ecx=w.territory)
        random_out[str(side)] = {'returned_low_byte': eax & 0xff, 'removed': list(w.destroyed), 'rng_state_after': m.r32(w.state + 0x18)}
    out['random_destroy'] = random_out
    return out

def clamp_cases(rng):
    special = [0.0, -0.0, 1.0, 0.5, 0.47, 2.0, -1.0, 1e-8, 0.999999, 1.000001]
    rows = []
    for _ in range(400):
        d = rng.choice(special) if rng.random() < 0.3 else rng.uniform(-0.5, 2.5)
        a = rng.choice(special) if rng.random() < 0.3 else rng.uniform(-0.5, 2.5)
        b = rng.choice([0.47, 0.0, 1.0, 0.5]) if rng.random() < 0.5 else rng.uniform(-0.5, 1.5)
        rows.append((f32(d), f32(a), f32(b)))
    return rows

def run_clamp(rows, pc_name):
    m = Machine(PCS[pc_name])
    settings = m.alloc(0x300); prob = m.alloc(4)
    m.w32(settings + 0x20c, prob); m.w32(0xb39ab0, settings)
    out = []
    for d, a, b in rows:
        m.w32(prob, b)
        st0 = m.call(0x95f2f0, args=(d, a), store_st0=True)[1]
        out.append({'defender_bits': d, 'attacker_bits': a, 'base_bits': b, 'result': ext_to_json(st0)})
    return out

def run_rng(rng):
    m = Machine(PCS['extended64'])
    cell = m.alloc(4)
    rows = []
    seeds = [0, 1, 0xffffffff, 0x7fffffff, 0x80000000, 12345] + [rng.getrandbits(32) for _ in range(200)]
    for s in seeds:
        m.w32(cell, s)
        eax, _ = m.call(0x85d840, args=(cell,))
        rows.append({'state': s, 'returned': eax, 'state_after': m.r32(cell)})
    return rows

def run_enum():
    m = Machine(PCS['extended64'])
    rows = []
    def insert(mm):
        value, name = mm.arg(0), mm.arg(1)
        raw = bytearray(); a = name
        while True:
            ch = bytes(mm.u.mem_read(a, 2)); a += 2
            if ch == b'\0\0': break
            raw += ch
        rows.append({'value': value, 'name': raw.decode('utf-16le'), 'third_argument': mm.arg(2)})
        mm.ret(12)
    m.seams[0x95c6a0] = insert
    m.call(0x95c7b0, ecx=m.alloc(0x40))
    return rows

def main():
    rng = random.Random(0x95f480)
    cases = [gen_case(rng, i) for i in range(160)]
    results = []
    for case in cases:
        per = {pc: run_case(case, pc) for pc in PCS}
        results.append({'input': case, 'native': per})
    clamp_inputs = clamp_cases(random.Random(0x95f2f0))
    clamp = {pc: run_clamp(clamp_inputs, pc) for pc in PCS}
    report = {
        'schema': 1,
        'scope': __doc__,
        'original_sha256': hashlib.sha256(P.b).hexdigest(),
        'v14_sha256': hashlib.sha256(V.b).hexdigest(),
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'control_words': PCS,
        'rng': run_rng(random.Random(0x85d840)),
        'destroy_behaviour_enum': run_enum(),
        'odds_clamp': clamp,
        'battles': results,
        'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())],
    }
    out = HERE / 'native-battle.json'
    out.write_text(json.dumps(report, indent=1) + '\n')
    print(json.dumps({'battles': len(results), 'clamp': len(clamp_inputs), 'visited': len(VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))

if __name__ == '__main__':
    main()
