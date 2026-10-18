"""Original Denizen component (slot 10) with NONEMPTY Inventories: constructor
56B720 (ActorData +35C list, 568880 flags with 4C1D60/4C1CF0 over +34C
organization layouts) then the complete saved Load 56C320 on the same object,
under Unicorn (native_machine.py).

Seams (recorded): 45EEF0 (ActorData has Property), allocation, the 404F30
log; 50BCD0 runs on a GWorld+284 table. A probe at 56C45B records the stream
position after the Inventory occupant section. Real cases: the 168 shipped
Denizen payloads whose definition has a nonempty Inventory, with the slot
lists of aa-rules ActorCatalog::native_denizen_inventory (real-payloads.json);
their +35C lists are rebuilt so 56B720 itself produces those lists.
"""
from pathlib import Path
import sys, json, struct, random, copy
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, P, V, VISITED, H, f32, Throw
from unicorn.x86_const import UC_X86_REG_ECX

W = lambda *v: struct.pack('<' + 'I' * len(v), *[x & 0xffffffff for x in v])

# synthetic targets: name -> (+174 flags, +284 health bits)
TARGETS = {'fighter': (0x41, f32(100.0)), 'bomber': (0x41, f32(37.5)), 'boat': (0x01, f32(250.0)),
           'tank': (0x40, f32(80.0)), 'odd': (0x41, 0x3eaaaaab)}
# organization ActorData for +34C: layout entries [(property, required, locked)], params
ORGS = {'wing': ([('prop_air', 0, 0)], ['fighter']),
        'guard': ([('prop_guard', 0, 1), ('prop_air', 1, 0)], ['bomber', None])}
HAS = {('fighter', 'prop_air'): True, ('bomber', 'prop_air'): False, ('odd', 'prop_air'): True,
       ('bomber', 'prop_guard'): True, ('odd', 'prop_guard'): True}


class DenizenMachine(Machine):
    def __init__(self, case):
        super().__init__(bytes.fromhex(case['payload']), case['precision'])
        self.case = case
        world = self.alloc(0x400)
        self.w(0xb36534, world)
        table = self.alloc(16)
        size = 0x10000  # saved Updatable IDs of shipped maps exceed 0x500
        slots = self.alloc(4 * (size + 2))
        self.w(table, size)
        self.w(table + 4, slots)
        self.w(world + 0x284, table)
        self.defs = {}
        for name, (flags, health) in case['targets'].items():
            d = self.alloc(0x600)
            self.w(d + 8, self.cstring(name))
            self.w(d + 0x174, flags)
            self.w(d + 0x284, health)
            self.defs[name] = d
            self.labels[d] = name
        self.props = {}
        for prop in ('prop_air', 'prop_guard'):
            pr = self.alloc(0x20)
            self.w(pr + 8, self.cstring(prop))
            self.props[prop] = pr
            self.labels[pr] = prop
        owner_def = self.alloc(0x600)
        self.w(owner_def + 8, self.cstring('denizen_owner'))
        head = 0
        for target, count in reversed(case['inventory']):
            n = self.alloc(0x10)
            self.w(n, self.defs[target])
            self.w(n + 8, count)
            self.w(n + 0xc, head)
            head = n
        self.w(owner_def + 0x35c, head)
        head = 0
        for org, behavior in reversed(case['organizations']):
            entries, params = ORGS[org]
            od = self.alloc(0x400)
            lay = self.alloc(0x40)
            ld = self.alloc(0x40)
            e = self.alloc(16 * len(entries))
            for i, (prop, req, locked) in enumerate(entries):
                self.w(e + 16 * i + 4, self.props[prop])
                self.u.mem_write(e + 16 * i + 0xc, bytes([req, locked, 0, 0]))
            self.w(ld + 0x1c, e)
            self.w(ld + 0x20, len(entries))
            pa = self.alloc(4 * len(params))
            for i, x in enumerate(params):
                self.w(pa + 4 * i, self.defs[x] if x else 0)
            self.w(lay + 8, ld)
            self.w(lay + 0xc, pa)
            self.w(od + 0x2f0, lay)
            n = self.alloc(0x10)
            self.w(n, od)
            self.w(n + 8, behavior)
            self.w(n + 0xc, head)
            head = n
        self.w(owner_def + 0x34c, head)
        self.owner = self.alloc(0x200)
        self.w(self.owner + 4, owner_def)

        def has_property(m):  # 45EEF0(Property) thiscall on ActorData
            actor, prop = m.labels[m.u.reg_read(UC_X86_REG_ECX)], m.labels[m.arg(0)]
            r = HAS.get((actor, prop), False)
            m.record('45EEF0', actor=actor, property=prop, result=r)
            m.ret(4, int(r))
        self.seam(0x45eef0, has_property)

        def log(m):
            fmt = m.read_wide(m.arg(4))
            m.record('404F30', line=m.arg(2), format=fmt, args=m.fmt_args(fmt, 5))
            m.ret(0)
        self.seam(0x404f30, log)
        self.occupant_end = None

        def probe(m):
            m.occupant_end = m.at
        self.probes[0x56c45b] = probe

    def slots(self, d):
        out, node = [], self.r(d + 0x18)
        while node:
            out.append({'word_00': self.r(node), 'target': self.labels.get(self.r(node + 4)),
                        'value_08': self.r(node + 8), 'worker': self.b(node + 0xc),
                        'militia_or_air_defensive': self.b(node + 0xd)})
            last = node
            node = self.r(node + 0x10)
        tail = self.r(d + 0x1c)
        assert tail == (last if out else 0)
        return out

    def state(self, d):
        handles, node = [], self.r(d + 0x2c)
        while node:
            handles.append(self.r(node))
            node = self.r(node + 4)
        return {'inventory': self.slots(d),
                'words': {'%#x' % o: self.r(d + o) for o in (0x10, 0x14, 0x24, 0x28, 0x34, 0x38, 0x3c, 0x8c, 0x90)},
                'bytes': [self.b(d + 0x20), self.b(d + 0x21)], 'handles': handles,
                'commands_head': self.r(d + 0x40)}

    def run(self):
        d = self.alloc(0x94, 0xcd)
        self.call(0x56b720, d, [self.owner])
        after_ctor = self.state(d)
        ctor_calls = list(self.calls)
        self.calls.clear()
        self.call(0x56c320, d, [self.ctx])
        out = {'after_ctor': after_ctor, 'ctor_calls': ctor_calls, 'consumed': self.at, 'reads': self.reads,
               'load_calls': self.calls, 'outcome': self.outcome, 'occupant_end': self.occupant_end}
        if self.outcome is None or self.outcome['kind'] != 'NullDereference':
            out['state'] = self.state(d)
        return out


def payload(version, occupants=None, count=None, word34=0x3434, handles=(), b20=1, b21=0xff, uid=None):
    p = W(1, 9 if uid is None else uid) + W(version)
    if version >= 1:
        p += bytes([b20])
    if version >= 2:
        p += bytes([b21])
    if version >= 3:
        occ = occupants or []
        p += W(len(occ) if count is None else count) + b''.join(W(a, b) for a, b in occ)
        if version >= 4:
            p += W(word34)
        p += W(len(handles), *handles) + W(0)
    return p


def cases():
    rng = random.Random(0x56c320)
    rows = []

    def add(name, inventory, organizations, data, precision='Double53', targets=None):
        rows.append({'name': name, 'precision': precision, 'inventory': inventory,
                     'organizations': organizations, 'targets': targets or TARGETS, 'payload': data.hex()})

    def occ(n):
        return [(rng.getrandbits(32), f32(rng.choice([1.0, 0.5, 0.25, 1 / 3, 0.9]))) for _ in range(n)]

    inventories = {'empty': [], 'three': [('fighter', 3)], 'mixed': [('fighter', 2), ('bomber', 1), ('boat', 2)],
                   'odd': [('odd', 1), ('tank', 0), ('fighter', 1)], 'negative': [('fighter', -1), ('bomber', 2)]}
    orgs = {'none': [], 'wing': [('wing', 2)], 'both': [('wing', 4), ('guard', 0x28)], 'guard': [('guard', 0x2c)],
            'nomatch': [('wing', 0x80)]}
    for iname, inv in inventories.items():
        for oname, org in orgs.items():
            add('ctor_%s_%s' % (iname, oname), inv, org, payload(4, occ(2)))
    for version in range(5):
        for n in (0, 2, 5, 9):
            add('load_v%d_n%d' % (version, n), inventories['mixed'], orgs['both'], payload(version, occ(n), handles=(7, 8)))
    add('load_negative_count', inventories['mixed'], [], payload(4, [], count=-1))
    add('load_count_beyond_payload', inventories['three'], [], payload(4, occ(2), count=3))
    for pc in ('Single24', 'Double53', 'Extended64'):
        for mult in (0x0060057a, 0x00601c60, 0x80603346, f32(3e38), f32(1e-30)):
            add('precision_%08x/%s' % (mult, pc), [('odd', 2)], [], payload(4, [(0x11, mult), (0x22, mult)]), pc)
    full = payload(4, occ(4), handles=(5,))
    for cut in range(0, len(full), 3):
        add('truncated_%d' % cut, inventories['mixed'], [], full[:cut])
    # real payloads
    real = json.loads((HERE / 'real-payloads.json').read_text())
    for d in real['denizens']:
        slots = real['denizen_inventories'][d['definition']]['slots']
        targets = {s['target']: (0x41, s['health_284_bits']) for s in slots}
        runs = []
        for s in slots:
            if runs and runs[-1][0] == s['target']:
                runs[-1][1] += 1
            else:
                runs.append([s['target'], 1])
        inventory = [tuple(r) for r in reversed(runs)]
        rows.append({'name': 'real %s#%d %s' % (d['map'], d['ordinal'], d['definition']), 'precision': 'Double53',
                     'inventory': inventory, 'organizations': [], 'targets': targets, 'payload': d['payload'],
                     'producer_slots': slots})
    return rows


def main():
    out = []
    for c in cases():
        m = DenizenMachine(c)
        native = m.run()
        out.append({'input': c, 'native': native})
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'machine_sha256': H((HERE / 'native_machine.py').read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__,
           'visited_instructions': len(VISITED), 'cases': out}
    path = HERE / 'native-denizen-inventory.json'
    path.write_text(json.dumps(doc, indent=1))
    kinds = {}
    for row in out:
        k = (row['native']['outcome'] or {}).get('kind', 'ok')
        kinds[k] = kinds.get(k, 0) + 1
    print(json.dumps({'cases': len(out), 'outcomes': kinds, 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
