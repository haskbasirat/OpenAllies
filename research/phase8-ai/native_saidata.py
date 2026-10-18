"""Original SAI profile load: constructor 4AA430 then KKC_SAIData::LoadData
4AE3E0 (with 47D960, 473AD0, 4012C0, 405B80 and the hash sets) executing
unchanged on seamed data nodes (native_data.py).

Cases: every shipped [SAI] profile (answers recorded from the original
Data.rwd + v14 overlays by crates/aa-mind/examples/dump_sai_inputs.rs into
sai-shipped-inputs.json), plus mutations of them (unknown difficulty or
nation or faction, duplicates, a missing field, empty lists, other values).
"""
from pathlib import Path
import copy, json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import PRECISIONS, f32, header, instructions, sha
from native_data import DataMachine

PROFILE_SIZE = 0x80


def x87_of_f32(bits):
    """80-bit image of a binary32 value (exact)."""
    v = struct.unpack('<f', struct.pack('<I', bits))[0]
    if v == 0:
        return {'significand': 0, 'sign_exponent': 0x8000 if bits >> 31 else 0}
    sign = bits >> 31
    exp = (bits >> 23) & 0xff
    frac = bits & 0x7fffff
    assert exp not in (0, 0xff)
    sig = (0x800000 | frac) << 40
    return {'significand': sig, 'sign_exponent': (sign << 15) | (exp - 127 + 16383)}


def registered(table):
    """Entries registered by the original enum constructors (native-sai-tables.json)."""
    d = json.loads((HERE / 'native-sai-tables.json').read_text(encoding='utf-8'))
    return [(e['value'], e['name']) for e in d['tables'][table]['entries']]


class ProfileMachine(DataMachine):
    def __init__(self, precision, answers):
        super().__init__(precision, answers)
        # 7BDCF0 registers the difficulty levels into the map at B40E64.
        self.enum_map(0xb40e64, registered('difficulty_levels'))

    def run(self, ids, root):
        obj = self.alloc(PROFILE_SIZE)
        slot = self.alloc(4); self.w(slot, self.cstring(ids))
        self.call(0x4aa430, ecx=obj, args=[slot], pop=4)
        self.events.clear()
        fault = self.run_call(0x4ae3e0, ecx=obj, args=[self.node(root)], pop=4)
        out = {'events': self.events}
        if fault:
            out['fault'] = fault
            return out
        def hset(base):
            buckets, n = self.r(base), self.r(base + 4)
            items = []
            for b in range(n):
                e = self.r(buckets + 4 * b)
                while e:
                    items.append(self.object_handle(self.r(e))); assert self.rb(e + 4) == 1
                    e = self.r(e + 8)
            return sorted(items), self.r(base + 8)
        nations, nn = hset(obj + 0x48)
        factions, nf = hset(obj + 0x58)
        refs = self.pending_refs()
        for r in refs:
            assert r['vtable'] == 0xa17054 and self.r(r['address'] + 0xc) == obj
        out.update({
            'name': self.cstr_text(self.r(obj + 0x10)),
            'loaded': self.rb(obj + 0x14),
            'difficulty': self.r(obj + 0x1c) - ((self.r(obj + 0x1c) >> 31) << 32),
            'public': self.rb(obj + 0x20),
            'command_max_delay_ms': self.r(obj + 0x24), 'command_max_total_delay_ms': self.r(obj + 0x28),
            'max_ge_commands': self.r(obj + 0x2c), 'max_rai_commands': self.r(obj + 0x30),
            'ego_fuzziness': self.r(obj + 0x3c),
            'ego_refs': [r['ids'] for r in reversed(refs)],
            'nations': nations, 'nation_count': nn, 'factions': factions, 'faction_count': nf,
            'first_nation': self.cstr_text(self.r(obj + 0x68)),
            'first_faction': self.cstr_text(self.r(obj + 0x6c)),
        })
        return out


def mutate(rnd, case):
    c = copy.deepcopy(case)
    nodes = c['answers']['nodes']
    root = nodes[str(c['root'])]
    kind = rnd.choice(['difficulty', 'nation', 'faction', 'dup_nation', 'missing', 'values', 'empty', 'egos'])
    c['mutation'] = kind
    if kind == 'difficulty':
        root['string']['difficulty_level'] = rnd.choice(['sai_difficulty_level_easy', 'SAI_DIFFICULTY_LEVEL_EASY', 'impossible'])
    elif kind in ('nation', 'faction'):
        mgr = 'Nations' if kind == 'nation' else 'Factions'
        table = c['answers']['managers'].setdefault(mgr, {})
        if table:
            k = rnd.choice(sorted(table)); table[k] = None
    elif kind == 'dup_nation':
        items = root.get('item', {})
        keys = [k for k in items if k.startswith('faction#')]
        if len(keys) >= 2:
            items[keys[1]] = items[keys[0]]
        elif keys:
            root['count']['faction'] = 2
            items['faction#1'] = items[keys[0]]
    elif kind == 'missing':
        getter = rnd.choice([g for g in ('fixed', 'int', 'bool', 'string') if root.get(g)])
        name = rnd.choice(sorted(root[getter]))
        root[getter][name] = {'error': "Node doesn't contain field."}
    elif kind == 'values':
        for name in root.get('fixed', {}):
            root['fixed'][name] = x87_of_f32(f32(rnd.choice([0.0, 0.0004, 0.0005, 0.0015, 1.25, 2.5e3, rnd.uniform(0, 30)])))
        for name in root.get('int', {}):
            root['int'][name] = rnd.choice([-1, 0, 7, 100000])
        for name in root.get('bool', {}):
            root['bool'][name] = rnd.random() < 0.5
    elif kind == 'empty':
        for name in ('ego', 'nation', 'faction'):
            if rnd.random() < 0.6:
                root['count'][name] = 0
    elif kind == 'egos':
        n = rnd.randrange(3, 6)
        root['count']['ego'] = n
        base = max(int(k) for k in nodes) + 1
        for i in range(n):
            nodes[str(base + i)] = {'string': {'ids': 'ego_%d' % rnd.randrange(4)}}
            root['item']['ego#%d' % i] = base + i
    return c


def cases():
    shipped = json.loads((HERE / 'sai-shipped-inputs.json').read_text(encoding='utf-8'))
    rnd = random.Random(0x4ae3e0)
    precisions = list(PRECISIONS)
    rows = []
    for i, p in enumerate(shipped['profiles']):
        rows.append({'name': 'shipped_' + p['ids'], 'ids': p['ids'], 'root': p['root'], 'precision': 'Double53',
                     'answers': {'nodes': p['nodes'], 'managers': p['managers']}})
    base = list(rows)
    for i in range(150):
        c = mutate(rnd, rnd.choice(base))
        c['name'] = 'mutated%d_%s' % (i, c['mutation'])
        c['precision'] = precisions[i % 3]
        rows.append(c)
    return rows


def main():
    out, visited, faults = [], {}, 0
    for c in cases():
        m = ProfileMachine(c['precision'], c['answers'])
        result = m.run(c['ids'], c['root'])
        m.verify_v14(); visited.update(m.visited)
        faults += 'fault' in result
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-saidata-profile.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    body = [a for a in visited if 0x4ae3e0 <= a < 0x4ae3e0 + 1550]
    print(json.dumps({'cases': len(out), 'faults': faults, 'instructions': len(visited),
                      'loader_instructions': len(body), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
