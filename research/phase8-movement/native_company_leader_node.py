"""Original regiment Mind virtual +14C 51F7A0 (the leader reached a node)
under Unicorn, at Single24, Double53 and Extended64, on the phase8-ai
machine (research/phase8-ai/native_machine.py).

Runs unchanged: 51F7A0, 51E8D0 (leader path node count), 52CDB0 (path head
removal), 51D7D0 and 52A1F0 (arrival within 0.5 of the move goal). Seams
(recorded in order): 52A000 on the states at Mind+1CC and Mind+34 (scripted
current-state answers), 57BF30 on the organization component (facing point),
Mind virtual +144 (scripted result). 85C2B0 (node free) is a seam that
checks the pool and node but is not recorded. Mind virtual +44 returns the
regiment path, the leader's +70 virtual +44 its own path; every other
virtual slot aborts the run.

Writes native-company-leader-node.json next to this script.
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-ai'))
from native_machine import Machine, PRECISIONS, f32, header, instructions, sha  # noqa: E402
from native_uai_harness import body_instructions  # noqa: E402

ENTRY, END = 0x51f7a0, 0x51f886
COUNT, COUNT_END = 0x51e8d0, 0x51e8e9
POP, POP_END = 0x52cdb0, 0x52ce00
ARRIVE, ARRIVE_END = 0x52a1f0, 0x52a237


class LeaderMachine(Machine):
    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = c
        self.mind = self.alloc(0x400)
        self.mvt = self.table(0x200); self.w(self.mind, self.mvt)
        org = self.org = self.alloc(0x100); self.w(self.mind + 4, org)
        self.w(org + 0x20, c['org_pos'][0]); self.w(org + 0x24, c['org_pos'][1])
        comp = self.comp = self.alloc(0x100); self.w(org + 0x7c, comp)
        self.leader = self.alloc(0x100); self.other = self.alloc(0x100)
        self.w(comp + 0x40, self.leader)
        lmind = self.alloc(0x100); lvt = self.table(0x100); self.w(lmind, lvt); self.w(self.leader + 0x70, lmind)
        self.leader_path = self.path(c['leader_nodes'])
        self.w(lvt + 0x44, self.stub(lambda m: self.ret(self.leader_path)))
        self.reg_path = self.path(c['nodes'], c['kind1'], c['kind2'], c['word18'])
        self.w(self.mvt + 0x44, self.stub(self.v_path))
        self.w(self.mvt + 0x144, self.stub(self.v_144))
        self.w(self.mind + 0x6c, self.mind)
        self.w(self.mind + 0x74, c['move_status'])
        self.w(self.mind + 0x7c, c['goal'][0]); self.w(self.mind + 0x80, c['goal'][1])
        self.pool = self.alloc(0x40); self.w(0xb27ae0, self.pool)
        self.seam(0x52a000, self.s_current)
        self.seam(0x57bf30, self.s_face)
        self.seam(0x85c2b0, self.s_free)

    def table(self, size):
        t = self.alloc(size)
        for slot in range(0, size, 4):
            self.w(t + slot, self.stub(self.unexpected(slot)))
        return t

    def unexpected(self, slot):
        def fn(m):
            raise AssertionError('unexpected virtual +%x' % slot)
        return fn

    def path(self, nodes, kind1=0, kind2=0, word18=0):
        """A path object: +4 head, +8 tail, +C/+10 kind counts, +14 dirty
        byte, +18 word; 0x28-byte nodes linked by +20 prev and +24 next."""
        p = self.alloc(0x20)
        ptrs = []
        for words in nodes:
            n = self.alloc(0x28)
            for k, v in enumerate(words):
                self.w(n + 4 * k, v)
            ptrs.append(n)
        for i, n in enumerate(ptrs):
            self.w(n + 0x20, ptrs[i - 1] if i else 0); self.w(n + 0x24, ptrs[i + 1] if i + 1 < len(ptrs) else 0)
        self.w(p + 4, ptrs[0] if ptrs else 0); self.w(p + 8, ptrs[-1] if ptrs else 0)
        self.w(p + 0xc, kind1 & 0xffffffff); self.w(p + 0x10, kind2 & 0xffffffff); self.wb(p + 0x14, 0); self.w(p + 0x18, word18)
        return p

    def ev(self, name, **k):
        self.events.append({'event': name, **k})

    def v_path(self, m):
        assert self.ecx() == self.mind
        self.ret(self.reg_path)

    def v_144(self, m):
        assert self.ecx() == self.mind
        r = self.c['v144']; self.ev('mind_virtual_144', result=r); self.ret(int(r))

    def s_current(self, m):
        off = self.ecx() - self.mind
        r = self.c['current'][str(off)]
        self.ev('current', offset=off, result=r); self.ret(int(r))

    def s_face(self, m):
        assert self.ecx() == self.comp
        self.ev('face', point=[self.arg(1), self.arg(2)]); self.ret(0, 8)

    def s_free(self, m):
        assert self.ecx() == self.pool and self.arg(1)
        self.ret(0, 4)

    def snapshot(self):
        p = self.reg_path
        nodes, n = [], self.r(p + 4)
        while n:
            nodes.append([self.r(n + 4 * k) for k in range(8)]); n = self.r(n + 0x24)
        tail = self.r(p + 8)
        return {'nodes': nodes, 'kind1': struct.unpack('<i', struct.pack('<I', self.r(p + 0xc)))[0],
                'kind2': struct.unpack('<i', struct.pack('<I', self.r(p + 0x10)))[0], 'dirty': self.rb(p + 0x14),
                'word18': self.r(p + 0x18), 'tail_is_last': (tail == 0) == (not nodes)}

    def run(self):
        leader = self.leader if self.c['arg_is_leader'] else self.other
        r = self.call(ENTRY, ecx=self.mind, args=[leader])
        return {'result': r & 0xff, 'events': self.events, **self.snapshot()}


def node(rnd, kind=None):
    return [rnd.getrandbits(32), rnd.getrandbits(32), f32(rnd.uniform(-50, 50)), f32(rnd.uniform(-50, 50)),
            rnd.choice([0, 1, 1, 1, 2]) if kind is None else kind, rnd.getrandbits(32), 0, rnd.getrandbits(32)]


def cases():
    rnd = random.Random(0x51f7a0)
    rows = []
    precisions = list(PRECISIONS)
    for i in range(720):
        nodes = [node(rnd) for _ in range(rnd.choice([0, 1, 1, 1, 2, 3, 5]))]
        kind1 = sum(1 for n in nodes if n[4] == 1) + rnd.choice([0, 0, 0, 1, -1, 3])
        kind2 = sum(1 for n in nodes if n[4] == 2) + rnd.choice([0, 0, 1])
        goal = [f32(rnd.uniform(-20, 20)), f32(rnd.uniform(-20, 20))]
        if rnd.random() < 0.5:
            near = rnd.choice([0.0, 0.3, 0.49, 0.5, 0.5000001, 0.51, 0.3535533, 0.35355338, 2.0])
            org = [f32(struct.unpack('<f', struct.pack('<I', goal[0]))[0] + near * rnd.choice([1, -1])),
                   f32(struct.unpack('<f', struct.pack('<I', goal[1]))[0] + rnd.choice([0.0, near, -near]))]
        else:
            org = [f32(rnd.uniform(-20, 20)), f32(rnd.uniform(-20, 20))]
        rows.append({'name': 'leader%d' % i, 'precision': precisions[i % 3], 'nodes': nodes, 'kind1': kind1, 'kind2': kind2,
                     'word18': rnd.getrandbits(32), 'leader_nodes': [node(rnd, 0) for _ in range(rnd.choice([0, 1, 1, 2, 3, 4]))],
                     'arg_is_leader': rnd.random() < 0.9,
                     'current': {str(0x1cc): rnd.random() < 0.15, str(0x34): rnd.random() < 0.15},
                     'move_status': rnd.choice([0, 1, 2, 2, 2]), 'goal': goal, 'org_pos': org, 'v144': rnd.random() < 0.6})
    # The last node: arrival distances around 0.5 at every precision.
    for i in range(360):
        goal = [f32(rnd.uniform(-2000, 2000)), f32(rnd.uniform(-2000, 2000))]
        gx, gy = (struct.unpack('<f', struct.pack('<I', g))[0] for g in goal)
        near = rnd.choice([0.0, 0.3, 0.49, 0.4999999, 0.5, 0.5000001, 0.51, 0.3535533, 0.35355338, 0.3535534])
        org = [f32(gx + near * rnd.choice([1, -1])), f32(gy + rnd.choice([0.0, near, -near]))]
        n = node(rnd, 1)
        rows.append({'name': 'arrive%d' % i, 'precision': precisions[i % 3], 'nodes': [n], 'kind1': rnd.choice([1, 2]),
                     'kind2': 0, 'word18': rnd.getrandbits(32), 'leader_nodes': [node(rnd, 0) for _ in range(rnd.choice([0, 1]))],
                     'arg_is_leader': True, 'current': {str(0x1cc): False, str(0x34): False}, 'move_status': 2,
                     'goal': goal, 'org_pos': org, 'v144': rnd.random() < 0.6})
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        try:
            m = LeaderMachine(c); result = m.run()
        except Exception:
            print(c['name']); raise
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    ranges = {'51F7A0': (ENTRY, END), '51E8D0': (COUNT, COUNT_END), '52CDB0': (POP, POP_END), '52A1F0': (ARRIVE, ARRIVE_END)}
    cover = {k: [len([a for a in visited if lo <= a < hi]), body_instructions(lo, hi)] for k, (lo, hi) in ranges.items()}
    d.update(instructions=instructions(visited), coverage=cover, cases=out)
    p = HERE / 'native-company-leader-node.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'coverage': cover, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
