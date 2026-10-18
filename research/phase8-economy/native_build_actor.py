"""Original BuilderComponent::BuildActor 0x563160 in isolated x86.

Executed unchanged: 563160 BuildActor, 6939B0 placement adjustment, 562FD0
construct affordability, 4E20A0 stock check, 4E1690 debit, the creation
parameter construction and the footprint iteration skeleton. Seams (recorded;
bodies not executed): definition virtual +0x30, 46EE00 grid snap, actor virtual
+0x108 owner, 4E1910 restrictions, 554DD0/554330 footprint iteration over
fixture tiles, 54EFC0 enemy tile test, 555830 site lookup, 404F60 diagnostic,
4F9C10 actor creation, 45F4D0 cost quotes, 4CF110, messages, CString helpers,
allocator (shared with native_construction.py).
"""
from pathlib import Path
import json, random, shutil, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import MODES, ROOT, bits, pin  # noqa: E402
from native_construction import ConstructionMachine, ACTOR, UDEF, K  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RANGES = {
    'build_actor_563160': (0x563160, 0x563612), 'placement_6939B0': (0x6939B0, 0x693A08),
    'construct_afford_562FD0': (0x562FD0, 0x563153), 'can_afford_4E20A0': (0x4E20A0, 0x4E20F7),
    'debit_4E1690': (0x4E1690, 0x4E16C5),
}
DVT, ALLOC, AVT2, FAKEMAP, FAKETILES = 0x2038000, 0x2039000, 0x2039100, 0x203A000, 0x203B000
SITES, CREATED = 0x203C000, 0x203D000
STUB_SNAPS, STUB_NIL, STUB_FREE = 0x3000120, 0x3000130, 0x3000140
MESSAGES = {0x56345C: 'notify_enemies_nearby', 0x5635A1: 'notify_creation_failed'}


class BuildActorMachine(ConstructionMachine):
    def __init__(self, cw, case):
        super().__init__(cw, case)
        w = case['world']
        for d, flags in w['placement_flags'].items():
            base = UDEF + int(d) * 0x400
            self.put(base, DVT); self.put(base + 0x1C, flags)
            self.put(base + 0x430, int(w['check_enemies'][d])); self.put(base + 0x4E0, w['site_class'][d])
        self.put(DVT + 0x30, STUB_SNAPS)
        self.put(ACTOR + 0x28, w['builder_field_28'])
        self.put(ALLOC, AVT2); self.put(AVT2 + 0xC, STUB_NIL); self.put(AVT2 + 4, STUB_FREE)
        self.put(FAKEMAP + 0x1C, 1); self.put(FAKEMAP + 0x28, FAKETILES)
        self.created = 0
        self.seam(STUB_SNAPS, self._snaps)
        self.seam(0x46EE00, self._snap)
        self.seam(0x856370, lambda m, esp: (ALLOC, 0))
        self.seam(STUB_NIL, lambda m, esp: (self._string() - 0x10, 0))
        self.seam(STUB_FREE, lambda m, esp: (None, 4))
        self.seam(0x554DD0, self._iterate)
        self.seam(0x554330, self._advance)
        self.seam(0x54EFC0, self._enemy)
        self.seam(0x555830, self._site)
        self.seam(0x404F60, self._log)
        self.seam(0x4F9C10, self._create)

    def _snaps(self, m, esp):
        d = self.ident(self.uc.reg_read(UC_X86_REG_ECX))
        self.trace.append({'kind': 'snaps_to_grid', 'definition': d})
        return int(self.op['snaps']), 0

    def _snap(self, m, esp):
        d = self.ident(self.uc.reg_read(UC_X86_REG_ECX) - 0x38)
        loc = self.read(esp + 4)
        before = self.words(loc, 2)
        self.put_words(loc, self.op['snapped'])
        self.trace.append({'kind': 'snap_location', 'definition': d, 'location': before})
        return None, 8

    def _iterate(self, m, esp):
        it = self.uc.reg_read(UC_X86_REG_ECX)
        shape = self.read(esp + 8)
        d = self.ident(self.read(shape) - 0x38)
        self.trace.append({'kind': 'enemies_nearby', 'definition': d})
        n = len(self.op['tiles'])
        self.put(it, FAKEMAP); self.put(it + 0x10, n - 1); self.put(it + 0x24, 0); self.put(it + 0x28, 0)
        return it, 16

    def _advance(self, m, esp):
        it = self.uc.reg_read(UC_X86_REG_ECX)
        self.put(it + 0x28, self.read(it + 0x28) + 1)
        return None, 4

    def _enemy(self, m, esp):
        tile = (self.uc.reg_read(UC_X86_REG_ECX) - FAKETILES) // 48
        assert self.read(esp + 8) == 5
        assert self.read(esp + 4) == (K if self.case['world']['kingdom'] else 0)
        return int(self.op['tiles'][tile]), 8

    def _site(self, m, esp):
        d = self.ident(self.read(esp + 4))
        loc = self.words(esp + 8, 2)
        self.trace.append({'kind': 'find_site', 'definition': d, 'location': loc})
        site = self.op['site']
        if site is None:
            return 0, 12
        p = SITES + 0x40 * (site & 0xFF)
        self.put(p + 0x14, site)
        return p, 12

    def _log(self, m, esp):
        self.trace.append({'kind': 'log_missing_site'})
        return None, 0

    def _create(self, m, esp):
        p = self.read(esp + 4)
        r = self.read
        params = {'definition': self.ident(r(p + 4)), 'location': self.words(p + 8, 2), 'builder_field_28': r(p + 0x10),
                  'facing': self.words(p + 0x14, 2), 'owner': r(p + 0x28) != 0, 'tag': r(p + 0x34),
                  'site': r(p + 0x3C) or None, 'flag_38': r(p + 0x38), 'vtable': r(p)}
        assert params.pop('flag_38') == 1 and params.pop('vtable') == 0xA25B28
        self.trace.append({'kind': 'create_actor', 'params': params})
        if not self.op['created']:
            return 0, 4
        self.created += 1
        return CREATED + 0x100 * self.created, 4

    def _message(self, m, esp):
        caller = self.read(esp)
        if caller not in MESSAGES:
            return ConstructionMachine._message(self, m, esp)
        assert self.read(esp + 8) == 0x11 and self.read(esp + 12) == ACTOR
        self.trace.append({'kind': MESSAGES[caller], 'kingdom': self.read(esp + 16) != 0})
        return None, 0

    def run(self, op):
        self.op = op
        self.trace = []
        self.case['world'].update(op.get('world', {}))
        d = UDEF + op['definition'] * 0x400
        loc, facing = self.alloc(8), self.alloc(8)
        self.put_words(loc, op['location']); self.put_words(facing, op['facing'])
        before = self.created
        eax, _ = self.call(0x563160, [d, loc, facing, op['tag']], 0x2030000)
        assert self.words(loc, 2) == op['location'] and self.words(facing, 2) == op['facing']
        result = None
        if eax:
            assert eax == CREATED + 0x100 * self.created and self.created == before + 1
            result = self.created
        return {'stock': self.words(0x2010000, self.n), 'events': self.trace, 'result': result}


R = random.Random(0x563160)


def make_case(i):
    n = R.randint(1, 4)
    kinds = [R.choice(['Stockpiled', 'Stockpiled', 'Upkeep']) for _ in range(n)]

    def cost():
        return [bits(R.choice([0.0, 10.0, 25.0, 60.0, R.uniform(0, 120)])) for _ in range(n)]
    defs = list(range(1, 7))
    world = {'kingdom': R.random() < 0.85, 'kingdom_allows': R.random() < 0.85, 'state_active': True,
             'has_structure': False, 'actor_flags': 0, 'destroyed': False, 'upgrade_list': [],
             'definition_flags': {str(d): 0 for d in defs},
             'placement_flags': {str(d): R.choice([0, 1 << 16, 1 << 17, 3 << 16, 0x5 | (1 << 16)]) for d in defs},
             'check_enemies': {str(d): R.random() < 0.5 for d in defs},
             'site_class': {str(d): R.choice([0, 0, 0x1d, 7]) for d in defs},
             'builder_field_28': bits(R.uniform(0, 50))}
    ops = []
    for _ in range(R.randint(3, 8)):
        op = {'definition': R.choice(defs), 'location': [bits(R.uniform(0, 900)), bits(R.uniform(0, 900))],
              'facing': [bits(R.uniform(-1, 1)), bits(R.uniform(-1, 1))], 'tag': R.choice([0, 1, 0x50007]),
              'snaps': R.random() < 0.6, 'snapped': [bits(float(R.randrange(0, 900, 4))), bits(float(R.randrange(0, 900, 4)))],
              'tiles': [R.random() < 0.15 for _ in range(R.randint(0, 4))],
              'site': R.choice([None, 0x50000 | R.randint(1, 200)]), 'created': R.random() < 0.85}
        if R.random() < 0.2:
            op['world'] = {R.choice(['kingdom', 'kingdom_allows']): R.random() < 0.5}
        ops.append(op)
    return {'name': f'random-{i}', 'catalog': {'resources': [{'id': f'r{j}', 'kind': kk, 'limited_provider': False,
                                                              'purchase_cost': [0] * n} for j, kk in enumerate(kinds)]},
            'limits': {'provided_maximum': 100, 'stockpiled_maximum': bits(10000.0)},
            'stock': [bits(R.choice([30.0, 150.0, 500.0, R.uniform(0, 300)])) for _ in range(n)],
            'quotes': {str(d): [cost() for _ in range(R.randint(1, 3))] for d in defs},
            'payments': {'paid': [], 'upgrade_paid': [], 'upgrade': 0},
            'world': world, 'ops': ops}


def main():
    cases = [make_case(i) for i in range(100)]
    fixtures, executions = [], 0
    for case in cases:
        for mode, cw in MODES:
            m = BuildActorMachine(cw, json.loads(json.dumps(case)))
            steps = []
            for op in case['ops']:
                steps.append({'op': op, 'expected': m.run(op)})
                executions += 1
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'steps': steps})
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-build-actor-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
