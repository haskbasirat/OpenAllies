"""Original VS2003 CRT qsort 96CE20 (+ shortsort 96CDB0) with the SAI goal
comparator 7A30B0, as called by the goal selection 7A93B0.

Both execute unchanged on arrays of pointers to emulated goals whose +38 base
priority (binary32) is the sort key. Every comparator call is observed (goal
indices of both arguments and the returned value) and the final order is
recorded. Cases include duplicates, signed zeros, NaN (an inconsistent
comparator), sizes around the 8-element shortsort threshold and large arrays.
"""
from pathlib import Path
import json, math, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, f32, header, instructions, sha
from unicorn.x86_const import UC_X86_REG_EAX


class SortMachine(Machine):
    def __init__(self, keys):
        super().__init__('Extended64')
        self.goals = []
        for k in keys:
            g = self.alloc(0x40); self.w(g + 0x38, k); self.goals.append(g)
        self.arr = self.array(self.goals)
        self.calls = []
        self.index = {g: i for i, g in enumerate(self.goals)}
        self.observe(0x7a30b0, self.o_cmp)
        self.observe(0x7a30d5, self.o_ret)
        self.observe(0x7a30ed, self.o_ret)

    def o_cmp(self, m):
        a, b = self.r(self.arg(1)), self.r(self.arg(2))
        self.calls.append([self.index[a], self.index[b]])

    def o_ret(self, m):
        self.calls[-1].append(self.u.reg_read(UC_X86_REG_EAX) if self.u.reg_read(UC_X86_REG_EAX) < 0x80000000 else self.u.reg_read(UC_X86_REG_EAX) - 0x100000000)

    def run(self):
        self.call(0x96ce20, args=[self.arr, len(self.goals), 4, 0x7a30b0], pop=0)
        order = [self.index[self.r(self.arr + 4 * i)] for i in range(len(self.goals))]
        return {'order': order, 'compares': self.calls}


def cases():
    rnd = random.Random(0x96ce20)
    rows = []
    for n in [0, 1, 2, 3, 7, 8, 9, 10, 16, 17, 31, 64, 200]:
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
        n = rnd.randrange(2, 120)
        rows.append((f'random{i}', [f32(rnd.randrange(-20, 20) / 4) for _ in range(n)]))
    return rows


def main():
    out, visited = [], {}
    for name, keys in cases():
        m = SortMachine(keys); result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'name': name, 'keys': keys, **result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-goal-qsort.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'compares': sum(len(c['compares']) for c in out),
                      'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
