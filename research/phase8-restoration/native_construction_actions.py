"""Original Construction action-mask contributor 565140 (with 565090), called by
4D1350 for every actor whose Construction component (actor+9C) is present.

Executes the unmodified x86: state group lookup 4D14C0/4D0820 against the
registered masks (477FC0 runs first), settlement walk 4CEDA0, the actual
GActor virtuals +34 (4D1F90), +100 (4CED60), +158 (4CE980 -> 588990),
+15C (4CE9A0), Wall 58CB30, ActorData access 529C30, Properties 4CF110 and
action writes 517FA0. Seams: the SVar lookup 4B2E60 ("AllowSellUnderSiege")
returns a supplied Bool cell, and the settlement upgrade readiness 585F00 is a
supplied answer (reachable only with ActorData+4C0 != 0, which the shipped
source census excludes).
"""
from pathlib import Path
import sys, json, random, struct
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase7-actors'))
from actor_states import Machine as Base, P, V, H
from unicorn.x86_const import *

VTABLE = {0x34: 0x4d1f90, 0x100: 0x4ced60, 0x158: 0x4ce980, 0x15c: 0x4ce9a0}


class Machine(Base):
    def __init__(self, case):
        super().__init__()
        self.register()
        self.case = case
        self.bump = 0x20d0000
        self.events = []
        self.vt = self.alloc(0x400)
        for off, target in VTABLE.items():
            self.w(self.vt + off, target)
        self.actors = {}
        for name, spec in case['actors'].items():
            self.actors[name] = self.alloc(0x200)
        self.sell_cell = self.alloc(16)
        self.u.mem_write(self.sell_cell, bytes([int(case['allow_sell_under_siege'])]))
        for name, spec in case['actors'].items():
            a = self.actors[name]
            definition = self.alloc(0x600)
            self.w(definition + 0x4c0, spec.get('upgrade_count', 0) & 0xffffffff)
            self.u.mem_write(definition + 0x4e4, bytes([int(spec.get('can_sell', False)), int(spec.get('can_raze', False))]))
            self.w(a, self.vt)
            self.w(a + 4, definition)
            self.w(a + 8, 0)
            self.w(a + 0x100, spec['state'] & 0xffffffff)
            self.w(a + 0x104, spec['state'] >> 32)
            self.w(a + 0xf0, spec.get('actions', 0) & 0xffffffff)
            self.w(a + 0xf4, spec.get('actions', 0) >> 32)
            if spec.get('properties') is not None:
                p = self.alloc(0x40)
                self.w(p + 0x1c, spec['properties'])
                self.w(a + 0x78, p)
            if 'wall' in spec:
                wall = self.alloc(0x40)
                self.w(wall + 4, a)
                self.w(wall + 0x10, self.ref(spec['wall']))
                self.w(a + 0x90, wall)
            if 'structure' in spec:
                s = self.alloc(0x40)
                self.w(s + 4, a)
                self.w(s + 0x10, self.ref(spec['structure']))
                self.w(a + 0x94, s)
            if 'settlement' in spec:
                s = self.alloc(0x40)
                self.w(s + 4, a)
                self.w(s + 0x14, self.ref(spec['settlement']))
                self.w(a + 0x98, s)
        self.component = self.alloc(0x40)
        self.w(self.component + 4, self.actors['self'])
        sys.path.insert(0, str(HERE))
        import null_guard
        null_guard.install(self)

    def alloc(self, n):
        p = self.bump
        self.bump += (n + 15) & ~15
        assert self.bump < 0x2100000
        return p

    def ref(self, name):
        return 0 if name is None else self.actors[name]

    def hook(self, u, pc, size, user):
        if pc == 0x4b2e60:
            self.events.append(['svar', self.text(self.arg()), self.arg(1)])
            self.ret(self.sell_cell, 8)
        elif pc == 0x585f00:
            owner = self.r(u.reg_read(UC_X86_REG_ECX) + 4)
            name = next(k for k, v in self.actors.items() if v == owner)
            self.events.append(['upgrades_ready', name])
            self.ret(int(self.case['upgrades_ready']))
        else:
            super().hook(u, pc, size, user)

    def run(self):
        self.invoke(0x565140, self.component)
        assert not self.null_accesses, self.null_accesses
        a = self.actors['self']
        return {'actions': self.r(a + 0xf0) | (self.r(a + 0xf4) << 32), 'events': self.events}


def cases():
    rng = random.Random(0x565140)
    rows = []
    group13 = [None, 25, 26, 27, 28, 29, 30, 31]
    for index in range(640):
        g = group13[index % 8]
        state = rng.getrandbits(42) & ~(0xfe << 24) & ~(3 << 20)
        if g is not None:
            state |= 1 << g
        state |= ((index >> 3) % 4) << 20  # bits 20/21
        topology = (index >> 5) % 10
        actors = {'self': {'state': state, 'actions': rng.getrandbits(42), 'can_sell': bool(index & 1),
                           'can_raze': bool((index >> 1) & 1) and not (index & 1) if index % 7 else bool((index >> 1) & 1),
                           'upgrade_count': 0 if index % 9 else (index % 3) + 1,
                           'properties': None if index % 5 == 0 else rng.choice([0, 0x800, 0xffffffff, 0x7ff, 0x1800])}}
        razing = (1 << 30) if index % 3 == 0 else 0
        if topology == 1:
            actors['self']['settlement'] = 'city'
            actors['city'] = {'state': razing}
        elif topology == 2:
            actors['self']['settlement'] = None
        elif topology == 3:  # attached settlement center: parent's settlement points back
            actors['self']['structure'] = 'parent'
            actors['parent'] = {'state': razing, 'settlement': 'self'}
        elif topology == 4:  # structure in a city, not its center
            actors['self']['structure'] = 'parent'
            actors['parent'] = {'state': 0, 'settlement': 'center'}
            actors['center'] = {'state': razing}
        elif topology == 5:  # null structure parent, wall reference
            actors['self']['structure'] = None
            actors['self']['wall'] = 'gate'
            actors['gate'] = {'state': razing, 'settlement': 'gate'}
        elif topology == 6:
            actors['self']['wall'] = None
        elif topology == 7:  # two-step walk through a structure parent without settlement match
            actors['self']['structure'] = 'parent'
            actors['parent'] = {'state': 0, 'settlement': None}
        elif topology == 8:
            actors['self']['wall'] = 'outer'
            actors['outer'] = {'state': razing, 'structure': 'top'}
            actors['top'] = {'state': razing, 'settlement': 'top'}
        elif topology == 9:
            actors['self']['structure'] = 'parent'
            actors['parent'] = {'state': razing, 'settlement': 'self'}
            actors['self']['wall'] = 'parent'
        rows.append({'name': f'case{index}', 'actors': actors, 'allow_sell_under_siege': bool((index >> 2) & 1),
                     'upgrades_ready': bool((index >> 4) & 1)})
    return rows


def main():
    rows = []
    observed = {}
    for case in cases():
        m = Machine(case)
        try:
            out = m.run()
        except Exception as e:
            raise RuntimeError((hex(m.u.reg_read(UC_X86_REG_EIP)), case)) from e
        rows.append({'input': case, 'output': out})
        observed.update(m.observed)
    for a, b in observed.items():
        assert b == P.range(a, len(b)) == V.range(a, len(b))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b),
           'scope': '565140/565090 with native 4D14C0/4D0820 (registered masks), 4CEDA0, GActor virtuals 4D1F90/4CED60/4CE980/4CE9A0, 588990, 58CB30, 529C30, 4CF110, 517FA0. Seams: SVar 4B2E60 Bool cell, 585F00 answer.',
           'instructions': [{'va': hex(a), 'bytes': b.hex()} for a, b in sorted(observed.items())],
           'cases': rows}
    path = HERE / 'native-construction-actions.json'
    path.write_text(json.dumps(doc, indent=1))
    print(json.dumps({'cases': len(rows), 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
