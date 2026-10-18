"""Original organization member creation 0x4FBF40 (GWorld, thiscall:
params, organization actor, slot) under Unicorn, every callee seamed and
recorded in call order, at PC24/PC53/PC64 (it does x87 arithmetic: squared
distances to recruit structures and the slot position).

Seams: game mode [B36550]+E4; spatial query 552EE0 / 5581F0 / 558040 /
558580 (iterator yields the case's candidates in order) / 557FD0; object
virtual +108 (owner) and owner virtual +138 (relation test); 4FACA0
(creation at a structure), 57D860 (Organization member at slot), 578290
(slot placement test, cdecl), 57D950 (Organization member creation).
"""
from pathlib import Path
import sys, json, struct, hashlib, random
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_supply import Machine as Base, P, V, VISITED
from unicorn.x86_const import *

H = lambda b: hashlib.sha256(b).hexdigest()
F = lambda v: struct.unpack('<I', struct.pack('<f', v))[0]


class Machine(Base):
    def setup(self, c):
        self.case = c
        self.calls = []
        self.stub = self.alloc(0x400)
        self.stub_owner = self.stub        # object vf108
        self.stub_ally = self.stub + 0x10  # owner vf138
        vt_obj = self.alloc(0x200)
        self.write(vt_obj + 0x108, self.stub_owner)
        vt_owner = self.alloc(0x200)
        self.write(vt_owner + 0x138, self.stub_ally)
        self.owners = {}
        def owner_object(index):
            o = self.alloc(0x40)
            self.write(o, vt_owner)
            self.owners[o] = index
            return o
        mode_obj = self.alloc(0x200)
        self.write(mode_obj + 0xe4, c['game_mode'])
        self.write(0xb36550, mode_obj)
        # params: +4 definition (+17C list), +28, +40, +50, +51
        self.params = self.alloc(0x80)
        definition = self.alloc(0x400)
        self.write(self.params + 4, definition)
        self.write(self.params + 0x28, c['params_28'])
        self.u.mem_write(self.params + 0x50, bytes([c['byte_50'], c['byte_51']]))
        defs = {}
        def definition_of(index):
            if index not in defs:
                d = self.alloc(0x400)
                self.u.mem_write(d + 0x3ec, bytes([1 if index in c['recruit_flag_defs'] else 0]))
                defs[index] = d
            return defs[index]
        head = 0
        for index in reversed(c['accepted_defs']):
            node = self.alloc(8)
            self.write(node, definition_of(index))
            self.write(node + 4, head)
            head = node
        self.write(definition + 0x17c, head)
        # organization actor and its Organization (+7C)
        self.org = self.alloc(0x200)
        self.write(self.org, vt_obj)
        self.write(self.org + 0x14, 0x30007)
        self.write(self.org + 0x20, c['actor_xy'][0])
        self.write(self.org + 0x24, c['actor_xy'][1])
        self.write(self.org + 0x2c, c['facing'][0])
        self.write(self.org + 0x30, c['facing'][1])
        self.org_owner = owner_object(-1)
        self.objects = {self.org: ('org', self.org_owner)}
        self.component = self.alloc(0x200)
        self.write(self.org + 0x7c, self.component)
        self.write(self.component + 0xac, c['radius'])
        self.write(self.component + 0xbc, c['org_xy'][0])
        self.write(self.component + 0xc0, c['org_xy'][1])
        offsets = self.alloc(8 * 16)
        for i, (x, y) in enumerate(c['offsets']):
            self.write(offsets + 8 * i, x)
            self.write(offsets + 8 * i + 4, y)
        self.write(self.component + 0x78, offsets)
        self.candidates = []
        for i, cand in enumerate(c['candidates']):
            o = self.alloc(0x40)
            self.write(o, vt_obj)
            self.write(o + 4, definition_of(cand['definition']))
            self.write(o + 0x20, cand['xy'][0])
            self.write(o + 0x24, cand['xy'][1])
            owner = owner_object(i) if cand['has_owner'] else 0
            self.objects[o] = (i, owner)
            self.candidates.append(o)
        self.iter_at = 0
        self.gworld = self.alloc(0x40)

    def hook(self, u, pc, size, arg):
        c = self.case
        ecx = u.reg_read(UC_X86_REG_ECX)
        if pc == self.stub_owner:
            name, owner = self.objects[ecx]
            self.calls.append(['owner', name])
            self.ret(value=owner)
            return
        if pc == self.stub_ally:
            index = self.owners[ecx]
            assert self.arg() == self.org_owner
            self.calls.append(['related', index])
            self.ret(4, value=int(c['candidates'][index]['related']))
            return
        if pc == 0x552ee0:
            self.calls.append(['query', self.arg(), self.arg(1), self.arg(2)])
            self.ret(0xc)
            return
        if pc in (0x5581f0, 0x557fd0):
            self.ret()
            return
        if pc == 0x558040:
            self.ret(8)
            return
        if pc == 0x558580:
            value = self.candidates[self.iter_at] if self.iter_at < len(self.candidates) else 0
            self.iter_at += 1
            self.ret(value=value)
            return
        if pc == 0x4faca0:
            assert ecx == self.gworld and self.arg() == self.params
            index = self.objects[self.arg(1)][0]
            self.calls.append(['create_at_structure', index, self.read(self.params + 0x40)])
            self.ret(8, value=0x6000 if c['structure_creation'] else 0)
            return
        if pc == 0x57d860:
            assert ecx == self.component
            self.calls.append(['set_member', self.arg(), self.arg(1)])
            self.ret(8)
            return
        if pc == 0x578290:
            assert self.arg() == self.component + 0x10
            pos = self.arg(2)
            self.calls.append(['slot_placement', self.arg(1), self.read(pos), self.read(pos + 4),
                               self.arg(3), self.arg(4)])
            self.ret(value=int(c['placement_ok']))
            return
        if pc == 0x57d950:
            assert ecx == self.component
            self.calls.append(['create_member'] + [self.arg(i) for i in range(8)])
            self.ret(0x20, value=0x7000 if c['member_creation'] else 0)
            return
        super().hook(u, pc, size, arg)

    def run_case(self, c):
        self.setup(c)
        self.call(0x4fbf40, self.gworld, [self.params, self.org, c['slot']])
        return {'result': self.u.reg_read(UC_X86_REG_EAX), 'calls': self.calls,
                'params_40': self.read(self.params + 0x40)}


def cases():
    rng = random.Random(0x4fbf40)
    rows = []
    vals = [0.0, -0.0, 1.0, 3.5, -12.25, 1e-3, 1e30, 3.4e38, 1.17549435e-38, 123456.789, -7.0]
    for i in range(240):
        n = rng.choice([0, 1, 2, 3, 5])
        cands = []
        for j in range(n):
            cands.append({'definition': rng.choice([0, 1, 2]), 'has_owner': rng.random() < 0.85,
                          'related': rng.random() < 0.8,
                          'xy': [F(rng.choice(vals) if rng.random() < 0.3 else rng.uniform(-500, 500)) for _ in range(2)]})
        if rng.random() < 0.1 and n:
            cands[rng.randrange(n)]['xy'][0] = 0x7fc00000  # NaN position
        rows.append({
            'precision': ['Single24', 'Double53', 'Extended64'][i % 3],
            'game_mode': rng.choice([0, 1, 2, 3]),
            'params_28': rng.choice([0, 1, 1, 1]),
            'byte_50': rng.choice([0, 1]), 'byte_51': rng.choice([0, 1]),
            'accepted_defs': rng.sample([0, 1, 2], rng.choice([0, 1, 2, 3])),
            'recruit_flag_defs': rng.sample([0, 1, 2], rng.choice([0, 1, 2, 3])),
            'actor_xy': [F(rng.uniform(-500, 500)) for _ in range(2)],
            'facing': [F(rng.uniform(-1, 1)) for _ in range(2)],
            'radius': F(rng.uniform(0, 300)),
            'org_xy': [F(rng.choice(vals) if rng.random() < 0.3 else rng.uniform(-500, 500)) for _ in range(2)],
            'offsets': [[F(rng.choice(vals) if rng.random() < 0.3 else rng.uniform(-20, 20)) for _ in range(2)] for _ in range(4)],
            'slot': rng.randrange(4),
            'candidates': cands,
            'structure_creation': rng.random() < 0.7,
            'placement_ok': rng.random() < 0.7,
            'member_creation': rng.random() < 0.8,
        })
    return rows


def main():
    out = []
    for c in cases():
        cw = {'Single24': 0x7f, 'Double53': 0x27f, 'Extended64': 0x37f}[c['precision']]
        m = Machine(b'', cw)
        out.append({'input': c, 'native': m.run_case(c)})
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__, 'cases': out}
    path = HERE / 'native-spawn-member.json'
    path.write_text(json.dumps(doc, indent=0))
    results = {}
    for row in out:
        k = 'structure' if row['native']['result'] == 0x6000 else 'member' if row['native']['result'] == 0x7000 else 'null'
        results[k] = results.get(k, 0) + 1
    print(json.dumps({'cases': len(out), 'results': results, 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
