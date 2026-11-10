"""Original SAI list merge sort 7A3370 (split 4C82A0, merge 638400) with the
goal comparator 7A3070, as the goal execution 7A59C0 orders its goals.

7A59C0 gathers the selected goals of the 23 buckets (bucket list +10, head
first) by pushing each at the head of a new list, then splits that list and
sorts it with exactly the code of 7A3370 (4C82A0, two 7A3370 calls, 638400).
Here 7A3370 runs unchanged on a list of 8-byte nodes {goal, next} whose goals
carry the +34 current priority (binary32) the comparator reads. Every
comparator call is observed (goal indices of both arguments and the returned
value) and the final order and tail are recorded. Cases include duplicates,
signed zeros, NaN (an inconsistent comparator), short lists and long lists.
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, f32, header, instructions, sha
from unicorn.x86_const import UC_X86_REG_EAX


class MergeMachine(Machine):
    def __init__(self, keys):
        super().__init__('Double53')
        self.goals, self.nodes = [], []
        for k in keys:
            g = self.alloc(0x40); self.w(g + 0x34, k); self.goals.append(g)
        for g in self.goals:
            n = self.alloc(8); self.w(n, g); self.nodes.append(n)
        for a, b in zip(self.nodes, self.nodes[1:]):
            self.w(a + 4, b)
        self.cells = self.alloc(8)
        self.w(self.cells, self.nodes[0] if self.nodes else 0)
        self.w(self.cells + 4, self.nodes[-1] if self.nodes else 0)
        self.index = {g: i for i, g in enumerate(self.goals)}
        self.calls = []
        self.observe(0x7a3070, self.o_cmp)
        for a in (0x7a3095, 0x7a30ad):
            self.observe(a, self.o_ret)

    def o_cmp(self, m):
        a, b = self.r(self.arg(1)), self.r(self.arg(2))
        self.calls.append([self.index[a], self.index[b]])

    def o_ret(self, m):
        v = self.u.reg_read(UC_X86_REG_EAX)
        self.calls[-1].append(v if v < 0x80000000 else v - 0x100000000)

    def run(self):
        self.call(0x7a3370, args=[self.cells, self.cells + 4, 0x7a3070])
        order, n = [], self.r(self.cells)
        while n:
            order.append(self.index[self.r(n)]); last = n; n = self.r(n + 4)
        assert self.r(self.cells + 4) == (last if order else 0), 'tail'
        return {'order': order, 'compares': self.calls}


def cases():
    rnd = random.Random(0x7a3370)
    rows = []
    for n in [0, 1, 2, 3, 4, 5, 7, 8, 9, 16, 17, 31, 64, 150]:
        rows.append((f'distinct_{n}', [f32(rnd.uniform(-500, 500)) for _ in range(n)]))
        rows.append((f'few_values_{n}', [f32(rnd.choice([0, 10, 10, 25, -3])) for _ in range(n)]))
    rows.append(('all_equal_40', [f32(7)] * 40))
    rows.append(('ascending_50', [f32(i) for i in range(50)]))
    rows.append(('descending_50', [f32(-i) for i in range(50)]))
    rows.append(('signed_zeros', [0, 0x80000000] * 10))
    nan = 0x7fc00000
    rows.append(('nan_mix_12', [f32(3), nan, f32(1), f32(9), nan, f32(2), f32(2), f32(8), nan, f32(-1), f32(0), f32(5)]))
    rows.append(('nan_mix_30', [nan if rnd.random() < 0.2 else f32(rnd.randrange(10)) for _ in range(30)]))
    for i in range(20):
        n = rnd.randrange(2, 90)
        rows.append((f'random{i}', [f32(rnd.randrange(-20, 20) / 4) for _ in range(n)]))
    return rows


def main():
    out, visited = [], {}
    for name, keys in cases():
        m = MergeMachine(keys); result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'name': name, 'keys': keys, **result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-goal-merge.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'compares': sum(len(c['compares']) for c in out),
                      'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
