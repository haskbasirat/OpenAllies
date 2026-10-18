"""Original SAI enum constructors 7BDDD0 (goal types, via 7BDB70), 7BDCF0
(difficulty levels; static initializer 7BDFA0), 7BDEA0 (goal actor-proximity
enum, general power Location type) and 499940 (component types, via 497FA0;
registration 4976B0; static initializer 9CCAB0, map B3B258).

Each constructor executes unchanged with both values of its byte argument.
Seams: allocator 6F7810 (bump heap), 766DE0 (returns a scratch slot) and the
enum entry registrations 7BDA60/4976B0, whose (value, wide name, flag) arguments are
recorded in call order. These tables are what the SAIData loaders use to
translate goal_IDS / difficulty_level strings.
"""
from pathlib import Path
import json, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, header, instructions, sha

CONSTRUCTORS = {'goal_types': 0x7bddd0, 'difficulty_levels': 0x7bdcf0, 'goal_actor_proximity': 0x7bdea0,
                'component_types': 0x499940}


class TableMachine(Machine):
    def __init__(self):
        super().__init__('Extended64')
        self.entries = []
        self.seam(0x6f7810, lambda m: self.ret(self.alloc(self.arg(1) + 16)))
        self.seam(0x766de0, lambda m: self.ret(self.alloc(16)))
        self.seam(0x7bda60, self.register)
        self.seam(0x4976b0, self.register)

    def wide(self, p):
        out = []
        while True:
            ch = self.u.mem_read(p + 2 * len(out), 2)
            v = ch[0] | ch[1] << 8
            if v == 0:
                return ''.join(map(chr, out))
            out.append(v)

    def register(self, m):
        self.entries.append({'value': self.arg(1), 'name': self.wide(self.arg(2)), 'flag': self.arg(3)})
        self.ret(0, 12)


def main():
    rows = {}
    visited = {}
    for name, entry in CONSTRUCTORS.items():
        results = []
        for flag in [0, 1]:
            m = TableMachine()
            this = m.alloc(0x40)
            m.call(entry, ecx=this, args=[flag])
            m.verify_v14(); visited.update(m.visited)
            results.append(m.entries)
        assert results[0] == results[1]
        rows[name] = {'constructor': hex(entry), 'entries': results[0]}
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), tables=rows)
    p = HERE / 'native-sai-tables.json'
    p.write_text(json.dumps(d, indent=1) + '\n', encoding='utf-8')
    print(json.dumps({k: len(v['entries']) for k, v in rows.items()} | {'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
