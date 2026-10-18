"""Original GKingdom actor membership and recruit item table in isolated x86.

Executed unchanged: 4E6180 AddActor, 4E4470 counter/statistics part, 4E44E0
RemoveActor, 4E1F00/4E38B0 supply list, 4E1ED0 oldest flare, 4D1050,
5031E0 origin resolve, 58E870 score add, 766DE0 vector append, 4E2A60 /
4E2CB0 / 4E4C30 recruit item table (hash insert 7ADDA0 removal, 606750 rehash).
Seams (recorded): 5704C0 / 56F4D0 economy, 4D1350 refresh, 4E1540 counted,
58F010 / 58EA80 / 58EFE0 statistics tracking, 4E3C70 ownership tracking,
4CEF10 / 4CEF70 property effects, 45ECA0 structure research items, 4E3AD0 /
4E23D0 kingdom property add/remove, structure virtual +0x100, 4F6760 destroy,
79F500 / 7A47C0 / 7A48A0 SAI listeners, allocator.
"""
from pathlib import Path
import json, random, shutil, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, MODES, bits, pin, ret_pop, ROOT  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RANGES = {'add_actor_4E6180': (0x4E6180, 0x4E64AE), 'counters_4E4470': (0x4E4470, 0x4E44D5),
          'remove_actor_4E44E0': (0x4E44E0, 0x4E48A6), 'supply_add_4E1F00': (0x4E1F00, 0x4E1F56),
          'supply_remove_4E38B0': (0x4E38B0, 0x4E3937), 'oldest_flare_4E1ED0': (0x4E1ED0, 0x4E1F00),
          'transient_4D1050': (0x4D1050, 0x4D1079), 'origin_5031E0': (0x5031E0, 0x503206),
          'score_58E870': (0x58E870, 0x58E883), 'find_single_4E2A60': (0x4E2A60, 0x4E2AB3),
          'remove_item_4E2CB0': (0x4E2CB0, 0x4E2DF8), 'add_item_4E4C30': (0x4E4C30, 0x4E4DC0)}
K, GT, STATS, WORLD, SAI = 0x2000000, 0x2100000, 0x2001000, 0x2002000, 0x2003000
LISTENERS = 0x2004000
ACTORS, ITEMS, DEFS, AVT = 0x2200000, 0x2300000, 0x2310000, 0x2320000
STUB_EXCLUDED = 0x3000100
RT = K + 0x1FC
MAXFLARES = 0x2005000


def A(i): return ACTORS + i * 0x1000
def AD(i): return A(i) + 0x800
def CL(i): return A(i) + 0x900
def DEF(d): return DEFS + d * 0x40
def ITEM(j): return ITEMS + j * 0x40


class RegistryMachine(Machine):
    def __init__(self, cw, case):
        super().__init__(cw)
        self.case = case
        w = case['world']
        self.put(0xB364C0, GT)
        for slot in range(1, 9):
            self.put16(GT + 4 + 2 * slot, 5); self.put(GT + 0x20004 + 4 * slot, A(slot - 1))
        self.put(0xB36534, WORLD)
        self.put(K + 0x190, w['word_190']); self.put(K + 0x32C, STATS if w['statistics'] else 0)
        for vec in (0x2C4, 0x2D0, 0x2DC, 0x2E8, 0x318):
            self.put(K + vec, self.alloc(64)); self.put(K + vec + 4, 0); self.put(K + vec + 8, 16)
        records = w['records']
        data = self.read(K + 0x318)
        for i, r in enumerate(records):
            rec = self.alloc(0x18)
            self.put(rec + 8, r['source']); self.put(rec + 0xC, DEF(r['definition'])); self.put(rec + 0x14, r['origin'])
            self.put(data + 4 * i, rec)
        self.put(K + 0x31C, len(records))
        self.put(0xB3CBD8, 1); self.put(0xB3CBD4, MAXFLARES); self.put(MAXFLARES, w['max_flares'])
        self.put(0xB36528, SAI); self.put8(SAI + 0x20, int(w['ai']))
        self.put(AVT + 0x100, STUB_EXCLUDED)
        for i, a in enumerate(case['actors']):
            self.write_actor(i, a)
        # recruit table: 5 buckets, rehash enabled
        self.put(RT + 4, K); self.put(RT + 8, self.alloc(4 * 5)); self.put(RT + 0xC, 5); self.put(RT + 0x10, 0)
        self.put8(RT + 0x14, 1)
        self.put(RT + 0x18, self.alloc(64)); self.put(RT + 0x1C, 0); self.put(RT + 0x20, 16); self.put(RT + 0x24, 0xFFFFFFFF)
        self.trace = []
        self.seam(0x6F7810, lambda m, esp: (self.alloc(self.read(esp + 4)), 0))
        self.seam(0x6F7430, ret_pop(0))
        self.seam(0x5704C0, self._rec('economy_add', lambda: self.comp_actor(self.ecx(), 0x84), 0))
        self.seam(0x56F4D0, self._rec('economy_remove', lambda: self.comp_actor(self.ecx(), 0x84), 0))
        self.seam(0x4D1350, self._rec('refresh_organization', lambda: self.actor_of(self.ecx()), 0))
        self.seam(0x4E1540, self._counted)
        self.seam(0x58F010, self._track(True))
        self.seam(0x58EFE0, self._track(False))
        self.seam(0x58EA80, self._classified)
        self.seam(0x4E3C70, self._ownership)
        self.seam(0x4CEF10, self._apply)
        self.seam(0x4CEF70, self._remove_effect)
        self.seam(0x45ECA0, self._research_items)
        self.seam(0x4E3AD0, self._add_property)
        self.seam(0x4E23D0, self._remove_property)
        self.seam(STUB_EXCLUDED, self._excluded)
        self.seam(0x4F6760, self._destroy)
        self.seam(0x79F500, self._listeners)
        self.seam(0x7A47C0, self._notify(True))
        self.seam(0x7A48A0, self._notify(False))
        self.items = []
        for j, item in enumerate(case['items']):
            self.make_item(j, item)

    def ecx(self):
        return self.uc.reg_read(UC_X86_REG_ECX)

    def actor_of(self, p):
        i = (p - ACTORS) // 0x1000
        assert A(i) == p, hex(p)
        return i

    def comp_actor(self, comp, offset):
        return (comp - ACTORS) // 0x1000

    def write_actor(self, i, a):
        p = A(i)
        self.put(p, AVT); self.put(p + 4, AD(i)); self.put(AD(i) + 0x18, CL(i))
        self.put(CL(i) + 0xC, a['class_kind']); self.put(CL(i) + 0x1C, a['class_index'])
        self.put(p + 8, a['flags8']); self.put(p + 0x100, a['states'] & 0xFFFFFFFF); self.put(p + 0x104, a['states'] >> 32)
        self.put(p + 0x14, (5 << 16) | (i + 1)); self.put(p + 0x1C, a['flare_time'])
        for key, off in [('properties', 0x78), ('organization', 0x7C), ('economy', 0x84), ('supply', 0x8C),
                         ('structure', 0x94), ('settlement', 0x98), ('component_ac', 0xAC), ('flare', 0xC8), ('hq', 0xD8)]:
            self.put(p + off, p + 0x400 + off if a[key] else 0)

    def make_item(self, j, item):
        p = ITEM(j)
        if item['single']:
            self.put(p, 0xA29694); self.put(p + 4, DEF(item['definition'])); self.put8(p + 8, int(item['available']))
            self.put(p + 0xC, item['actor']); self.put(p + 0x10, item['organization']); self.put(p + 0x1C, item['progress'])
        else:
            self.put(p, 0xA29664); self.put(p + 4, DEF(item['definition'])); self.put8(p + 8, 1); self.put(p + 0xC, 1)

    # seams
    def _rec(self, name, actor, pop):
        def h(m, esp):
            self.trace.append({'kind': name, 'actor': actor()})
            return None, pop
        return h

    def _counted(self, m, esp):
        i = self.actor_of(self.read(esp + 4))
        self.trace.append({'kind': 'counted', 'actor': i})
        return int(self.case['actors'][i]['counted']), 4

    def _track(self, add):
        def h(m, esp):
            assert self.ecx() == STATS
            self.trace.append({'kind': 'statistics_track', 'actor': self.actor_of(self.read(esp + 4)), 'add': add})
            return None, 4
        return h

    def _classified(self, m, esp):
        i = self.actor_of(self.read(esp + 4))
        self.trace.append({'kind': 'statistics_classified', 'actor': i})
        return int(self.case['actors'][i]['classified']), 0

    def _ownership(self, m, esp):
        self.trace.append({'kind': 'ownership_track', 'actor': self.actor_of(self.read(esp + 4)), 'add': bool(self.read(esp + 8) & 0xFF)})
        return None, 8

    def _apply(self, m, esp):
        origin = self.read(esp + 12)
        self.trace.append({'kind': 'apply_property', 'actor': self.actor_of(self.ecx()), 'definition': (self.read(esp + 4) - DEFS) // 0x40,
                           'source': self.read(esp + 8), 'origin': self.actor_of(origin) if origin else None})
        return None, 12

    def _remove_effect(self, m, esp):
        origin = self.read(esp + 8)
        self.trace.append({'kind': 'remove_property_effect', 'actor': self.actor_of(self.ecx()),
                           'definition': (self.read(esp + 4) - DEFS) // 0x40, 'origin': self.actor_of(origin) if origin else None})
        return None, 8

    def _research_items(self, m, esp):
        i = (self.ecx() - ACTORS) // 0x1000
        out = self.read(esp + 4)
        nodes = [self.alloc(8) for _ in self.case['actors'][i]['research']]
        for k, (node, d) in enumerate(zip(nodes, self.case['actors'][i]['research'])):
            self.put(node, DEF(d)); self.put(node + 4, nodes[k + 1] if k + 1 < len(nodes) else 0)
        self.put(out, 0); self.put(out + 4, nodes[0] if nodes else 0)
        return out, 4

    def _add_property(self, m, esp):
        assert self.ecx() == K and self.read(esp + 8) == 0xA and self.read(esp + 16) == 1
        self.trace.append({'kind': 'add_kingdom_property', 'definition': (self.read(esp + 4) - DEFS) // 0x40,
                           'actor': self.actor_of(self.read(esp + 12))})
        return None, 16

    def _remove_property(self, m, esp):
        assert self.ecx() == K
        self.trace.append({'kind': 'remove_kingdom_property', 'definition': (self.read(esp + 4) - DEFS) // 0x40,
                           'actor': self.actor_of(self.read(esp + 8))})
        return None, 8

    def _excluded(self, m, esp):
        i = self.actor_of(self.ecx())
        self.trace.append({'kind': 'structure_excluded', 'actor': i})
        return int(self.case['actors'][i]['excluded']), 0

    def _destroy(self, m, esp):
        self.trace.append({'kind': 'destroy_actor', 'actor': self.actor_of(self.read(esp + 4))})
        return None, 8

    def _listeners(self, m, esp):
        assert self.read(esp + 4) == K
        out = self.read(esp + 8)
        n = self.case['world']['listeners']
        data = self.alloc(4 * max(1, n))
        for k in range(n):
            obj = LISTENERS + k * 0x40
            self.put(obj + 0xC, obj + 0x20)
            self.put(data + 4 * k, obj)
        self.free_later = data
        self.put(out, data); self.put(out + 4, n); self.put(out + 8, n)
        return None, 8

    def _notify(self, added):
        def h(m, esp):
            listener = (self.ecx() - 0x20 - LISTENERS) // 0x40
            item = (self.read(esp + 4) - ITEMS) // 0x40
            self.trace.append({'kind': 'notify_ai', 'listener': listener, 'item': item, 'added': added})
            return None, 4
        return h

    # snapshots
    def plist(self, off):
        out, node = [], self.read(K + off)
        while node:
            out.append(self.actor_of(self.read(node)))
            node = self.read(node + 4)
        return out

    def vec(self, off):
        return [self.actor_of(x) for x in self.words(self.read(K + off), self.read(K + off + 4))]

    def registry(self):
        classes = {}
        for c in range(8):
            v = self.plist(0x26C + 8 * c)
            if v:
                classes[str(c)] = v
        cap = self.read(K + 0x1F8)
        return {'class_lists': classes, 'supply': self.plist(0x264), 'structures': self.vec(0x2C4),
                'settlements': self.vec(0x2D0), 'hqs': self.vec(0x2DC), 'organizations': self.vec(0x2E8),
                'counted': self.read(K + 0x2F4) - (1 << 32) if self.read(K + 0x2F4) >> 31 else self.read(K + 0x2F4),
                'flares': self.plist(0x2F8), 'capital': self.actor_of(cap) if cap else None,
                'word_190': self.read(K + 0x190),
                'flags8': [self.read(A(i) + 8) for i in range(len(self.case['actors']))],
                'score14': self.read(STATS + 0x28 + 14 * 4)}

    def table(self):
        entries = []
        buckets, nb = self.read(RT + 8), self.read(RT + 0xC)
        for b in range(nb):
            node = self.read(buckets + 4 * b)
            while node:
                item = self.read(node + 4)
                entries.append([(self.read(node) - DEFS) // 0x40, self.read(item) == 0xA29694])
                node = self.read(node + 8)
        singles = []
        for k, item in enumerate(self.words(self.read(RT + 0x18), self.read(RT + 0x1C))):
            assert self.read(item + 0x20) == k
            singles.append({'definition': (self.read(item + 4) - DEFS) // 0x40, 'available': bool(self.read8(item + 8)),
                            'actor': self.read(item + 0xC), 'organization': self.read(item + 0x10),
                            'progress': self.read(item + 0x1C), 'item': (item - ITEMS) // 0x40})
        return {'entries': sorted(entries), 'singles': singles, 'changes': self.read(RT + 0x28)}

    def run(self, op):
        self.trace = []
        kind = op['kind']
        result = None
        if kind == 'add_actor':
            self.call(0x4E6180, [A(op['actor'])], K)
        elif kind == 'remove_actor':
            self.call(0x4E44E0, [A(op['actor'])], K)
        elif kind == 'add_item':
            self.call(0x4E4C30, [ITEM(op['item'])], RT)
        elif kind == 'remove_item':
            self.call(0x4E2CB0, [ITEM(op['item']), int(op['force'])], RT)
        elif kind == 'find_single':
            eax, _ = self.call(0x4E2A60, [DEF(op['definition']), int(op['available_only'])], RT)
            result = (eax - ITEMS) // 0x40 if eax else None
        return {'registry': self.registry(), 'table': self.table(), 'events': self.trace, 'result': result}


R = random.Random(0x4E6180)


def make_case(i):
    n = R.randint(3, 7)
    actors = []
    for k in range(n):
        actors.append({'class_index': R.randrange(4), 'class_kind': R.choice([0x1D, 3, 3, 7]),
                       'flags8': R.choice([0, 0, 1 << 27]), 'states': R.choice([0, 0, 1 << 21, 1 << 26, 1 << 36]),
                       'economy': R.random() < 0.5, 'structure': R.random() < 0.4, 'settlement': R.random() < 0.3,
                       'hq': R.random() < 0.2, 'organization': R.random() < 0.4, 'supply': R.random() < 0.4,
                       'properties': R.random() < 0.6, 'component_ac': R.random() < 0.4, 'flare': R.random() < 0.3,
                       'counted': R.random() < 0.6, 'classified': R.random() < 0.5, 'excluded': R.random() < 0.3,
                       'flare_time': bits(R.choice([1.0, 2.0, 5.5, R.uniform(0, 100)])),
                       'research': R.sample(range(1, 10), R.randint(0, 2))})
    items = []
    used = R.sample(range(1, 15), 8)
    for j in range(R.randint(2, 7)):
        items.append({'single': R.random() < 0.75, 'definition': used[j], 'available': R.random() < 0.5,
                      'actor': R.choice([0, 0, (5 << 16) | 1]), 'organization': R.choice([0, 0, 7]),
                      'progress': bits(R.choice([0.0, 0.5, 1.0]))})
    world = {'word_190': R.choice([0, 0, 0, 1]), 'statistics': R.random() < 0.7, 'max_flares': R.choice([1, 2, 3]),
             'ai': R.random() < 0.5, 'listeners': R.randint(0, 3),
             'records': [{'source': R.choice([2, 10]), 'definition': R.randrange(1, 10),
                          'origin': R.choice([0, (5 << 16) | R.randint(1, n)])} for _ in range(R.randint(0, 3))]}
    ops = []
    added_actors, added_items = set(), set()
    for _ in range(R.randint(6, 14)):
        r = R.random()
        if r < 0.35:
            a = R.randrange(n)
            if a not in added_actors:
                ops.append({'kind': 'add_actor', 'actor': a}); added_actors.add(a)
        elif r < 0.55:
            if added_actors:
                a = R.choice(sorted(added_actors))
                ops.append({'kind': 'remove_actor', 'actor': a}); added_actors.discard(a)
        elif r < 0.75:
            j = R.randrange(len(items))
            if j not in added_items:
                ops.append({'kind': 'add_item', 'item': j}); added_items.add(j)
        elif r < 0.88:
            if added_items:
                j = R.choice(sorted(added_items))
                if items[j]['single']:
                    force = R.random() < 0.5
                    it = items[j]
                    removed = force or it['available'] or (it['actor'] == 0 and it['organization'] == 0)
                    ops.append({'kind': 'remove_item', 'item': j, 'force': force})
                    if removed:
                        added_items.discard(j)
        else:
            ops.append({'kind': 'find_single', 'definition': R.choice(used), 'available_only': R.random() < 0.5})
    return {'name': f'random-{i}', 'world': world, 'actors': actors, 'items': items, 'ops': ops}


def main():
    cases = [make_case(i) for i in range(120)]
    fixtures, executions = [], 0
    for case in cases:
        for mode, cw in MODES:
            m = RegistryMachine(cw, case)
            initial = {'registry': m.registry(), 'table': m.table()}
            steps = [{'op': op, 'expected': m.run(op)} for op in case['ops']]
            executions += len(steps)
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'initial': initial, 'steps': steps})
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-registry-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    shutil.copyfile(out, ROOT / 'crates/aa-economy/tests/fixtures/native-registry-fixtures.json')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
