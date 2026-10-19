"""Oracle: VWorldState selected-command delivery 0x4ED360, NULL group (+20).

Runs, in both pinned images at PC24/PC53/PC64, the ORIGINAL 0x4ED360 with
the selection's group pointer +20 NULL (the only group state the Rust
binding admits) and the ORIGINAL 0x4D9A90 (SetCommandDefinition) inside it.
Seams (each recorded): 0x4D9A40 sync dump (label, command), 0x4DA570
GuessCommandData (scripted per actor), actor vt+B4 / vt+150 (scripted, raw
EAX: AL tested), 0x4D1900 / 0x4DFF30 live group preparation, 0x4D9B60
delivery (records command +08 definition and +0C flags at that moment).
Group-path leaves (0x50C410, 0x4FBE10, 0x432E40, 0x4E0940, 0x4CF1B0,
0x4E2940, 0x4DE740, 0x4DFE20, 0x4ED280) trap: they must never be reached
with a NULL group.

The question this answers: does 4ED360 treat an explicit command of any of
the 39 ActorCommand types (in particular 15 recruit) differently from
move(5)/attack(0) when the group is NULL? Rows are generated in groups that
differ ONLY in the explicit definition's type (+0C) and +3C byte.

  python native_selected_delivery_4ed360.py --write   (refuses to overwrite)
  python native_selected_delivery_4ed360.py           (replay, byte-identical)
"""
from pathlib import Path
import random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import recruit_lab as lab  # noqa: E402
nm = lab.nm

OUT = HERE / 'native-selected-delivery-4ed360.json'
FUNCTIONS = {'4ED360': (0x4ed360, 0x4ed6cb), '4D9A90': (0x4d9a90, 0x4d9ab0)}
TRAPS = [0x50C410, 0x4FBE10, 0x432E40, 0x4E0940, 0x4CF1B0, 0x4E2940, 0x4DE740, 0x4DFE20, 0x4ED280]


class Machine(lab.Lab):
    def __init__(self, precision, image):
        super().__init__(precision, image)
        self.seam(0x4D9A40, self._sync)
        self.seam(0x4DA570, self._guess)
        self.seam(0x4D1900, self._link)
        self.seam(0x4DFF30, self._prepare)
        self.seam(0x4D9B60, self._deliver)
        for t in TRAPS:
            self.seam(t, self._trap)
        self.trap = self.stub(self._trap)
        self.vt_b4 = self.stub(self._b4)
        self.vt_150 = self.stub(self._permit)
        self.table = self.alloc(0x20004 + 4 * 0x100)
        self.w(0xB364C0, self.table)
        self.mark = self.heap

    def _trap(self, m):
        raise AssertionError('group-path leaf reached with NULL group')

    def actor_index(self, p):
        return self.actors.index(p)

    def _sync(self, m):
        m.events.append(['sync', m.text(m.arg(1)), m.arg(2) == m.command])
        m.ret(0)

    def _guess(self, m):
        a = self.actor_index(m.arg(1))
        g = m.case['actors'][a]['guess']
        m.events.append(['guess', a, m.arg(2)])
        m.ret(m.defs[g] if g is not None else 0, pop=8)

    def _b4(self, m):
        a = self.actor_index(m.ecx())
        m.events.append(['b4', a])
        m.ret(m.case['actors'][a]['b4'])

    def _permit(self, m):
        a = self.actor_index(m.ecx())
        assert m.arg(1) == m.kingdom and m.r(m.arg(2)) == m.command and m.arg(3) == 1
        m.events.append(['permit', a, self.fields()])
        m.ret(m.case['actors'][a]['permit'], pop=12)

    def _link(self, m):
        a = self.actor_index(m.arg(1))
        m.events.append(['link', a])
        m.ret(m.group, pop=4)

    def _prepare(self, m):
        assert m.ecx() == m.group
        m.events.append(['prepare'])
        m.ret(0, pop=0)

    def fields(self):
        d = self.r(self.command + 8)
        return {'definition': None if d == 0 else self.def_index[d], 'flags': self.r(self.command + 0xC)}

    def _deliver(self, m):
        a = self.actor_index(m.arg(1))
        assert m.ecx() == m.command
        m.events.append(['deliver', a, self.fields()])
        m.ret(0, pop=4)

    def run(self, case):
        self.heap = self.mark
        self.case, self.events = case, []
        self.defs, self.def_index = {}, {}
        for i, (kind, b3c) in enumerate(case['definitions']):
            d = self.alloc(0x48)
            self.w(d + 0xC, kind)
            self.wb(d + 0x3C, b3c)
            self.defs[i] = d
            self.def_index[d] = i
        self.kingdom = self.alloc(0x200)
        self.group = self.alloc(0x40)
        vt = self.array([self.trap] * 0x80)
        self.w(vt + 0xB4, self.vt_b4)
        self.w(vt + 0x150, self.vt_150)
        self.actors = []
        nodes = []
        self.u.mem_write(self.table, bytes(0x20004 + 4 * 0x100))
        for i, a in enumerate(case['actors']):
            p = self.alloc(0x200)
            self.w(p, vt)
            slot = 1 + i
            self.w(p + 0x164, (a['link_generation'] << 16 | slot) if a['link'] else 0)
            if a['link']:
                self.u.mem_write(self.table + 4 + 2 * slot, (7).to_bytes(2, 'little'))
                self.w(self.table + 0x20004 + 4 * slot, self.group if a['link_live'] else 0)
            self.actors.append(p)
            n = self.alloc(8)
            self.w(n, p)
            nodes.append(n)
        for n, nxt in zip(nodes, nodes[1:] + [0]):
            self.w(n + 4, nxt)
        sel = self.alloc(0x40)
        self.w(sel, self.kingdom)
        self.w(sel + 4, nodes[0] if nodes else 0)
        self.w(sel + 8, nodes[-1] if nodes else 0)
        self.w(sel + 0x20, 0)
        self.command = self.alloc(0x40)
        self.w(self.command + 0xC, case['flags'])
        self.w(self.command + 8, self.defs[case['definition']] if case['definition'] is not None else 0)
        ref = self.alloc(4)
        self.w(ref, self.command)
        self.call(0x4ED360, ecx=sel, args=(ref,), pop=4)
        return {'events': self.events, 'final': self.fields()}


def cases(rng):
    out = []
    for group in range(48):
        n = rng.randrange(0, 4)
        actors = [{'b4': rng.choice([1, 1, 1, 0x100]), 'permit': rng.choice([1, 1, 0x100, 0]),
                   'link': rng.random() < 0.4, 'link_live': rng.random() < 0.5,
                   'link_generation': rng.choice([7, 7, 8]), 'guess': None} for _ in range(n)]
        flags = rng.choice([0x40, 0x42, 0x5C, 0x7FF, 0x440])
        b3c = rng.choice([0, 1])
        for kind in range(39):
            out.append({'group': group, 'actors': actors, 'flags': flags, 'definition': 0,
                        'definitions': [[kind, b3c]]})
    # Guessed (flag bit6 clear) commands: per-actor 4DA570 results.
    for group in range(48, 96):
        n = rng.randrange(1, 4)
        defs = [[rng.randrange(0, 39), rng.choice([0, 1])] for _ in range(3)]
        actors = [{'b4': rng.choice([1, 1, 0x100]), 'permit': rng.choice([1, 0]), 'link': rng.random() < 0.3,
                   'link_live': rng.random() < 0.5, 'link_generation': 7,
                   'guess': rng.choice([None, 0, 1, 2])} for _ in range(n)]
        out.append({'group': group, 'actors': actors, 'flags': rng.choice([0x1C, 0x02, 0]), 'definition': None,
                    'definitions': defs})
    return out


def main():
    write = '--write' in sys.argv
    rows, visited = [], {}
    for label, image in lab.IMAGES.items():
        for pc in nm.PRECISIONS:
            m = Machine(pc, image)
            for case in cases(random.Random(0x4ED360)):
                rows.append({'image': label, 'precision': pc, 'input': case, 'output': m.run(case)})
            m.verify_v14()
            visited.update(m.visited)
    doc = nm.header(__file__, 'unicorn original 4ED360 (NULL group) + 4D9A90')
    doc['scope'] = __doc__
    doc['coverage'] = lab.coverage(FUNCTIONS, visited)
    doc['rows'] = rows
    doc['instructions'] = nm.instructions(visited)
    lab.write_or_replay(OUT, doc, write)
    print({k: f"{v['executed']}/{v['instructions']}" for k, v in doc['coverage'].items()}, len(rows), 'rows')


if __name__ == '__main__':
    main()
