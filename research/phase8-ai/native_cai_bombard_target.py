"""Original 527A60(target) on the regiment bombard state (Mind+D4: target
GTID +10 and name CString +14) under Unicorn on the emulated regiment Mind
(native_cai_regiment.py), called directly. The function body runs
unchanged, including its temporary CString release (lock xadd on the
header reference count and the string manager's virtual +4 when it drops
to 0).

Seams (recorded in order): target virtual +24(&out) (writes a refcounted
fake CString holding the name), 405B80 on state+14 (CString assign; the
text recorded) and 401170 on state+14 (CString empty); the string
manager's virtual +4 (free) is recorded with the header it frees.

Writes native-cai-bombard-target.json next to this script.
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import header, instructions, sha
from native_cai_regiment import RegimentMachine, UnitOrderMixin
from native_uai_harness import body_instructions

ENTRY, END = 0x527a60, 0x527afa


class TargetMachine(UnitOrderMixin, RegimentMachine):
    def __init__(self, c):
        super().__init__(c)
        self.install_unit_orders({}, [], 0)
        mvt = self.r(self.string_mgr)
        self.w(mvt + 4, self.stub(self.v_free))
        self.target = 0
        if c['target'] is not None:
            self.target = self.add_object('target', c['target']['gtid'], True, [(0x24, self.v_name)])
        b = self.states['bombard']
        self.w(b + 0x10, c['old_gtid'])
        self.w(b + 0x14, self.cstring('old label', refs=2))
        self.seam(0x405b80, self.s_assign)
        self.seam(0x401170, self.s_empty)

    def ev(self, name, **kw):
        self.events.append({'event': name, **kw})

    def v_name(self, m):
        assert self.ecx() == self.target
        dst = self.arg(1)
        self.w(dst, self.cstring(self.c['target']['name'], refs=self.c['temp_refs']))
        self.ev('object_name'); self.ret(dst, 4)

    def s_assign(self, m):
        assert self.ecx() == self.states['bombard'] + 0x14
        src = self.r(self.arg(1)) if self.c['assign_by_ref'] else self.arg(1)
        text = self.strings.get(src)
        if text is None:
            text = self.strings[self.r(self.arg(1))]
        self.ev('assign_label', text=text); self.ret(self.ecx(), 4)

    def s_empty(self, m):
        assert self.ecx() == self.states['bombard'] + 0x14
        self.ev('empty_label'); self.ret(0)

    def v_free(self, m):
        hdr = self.arg(1)
        self.ev('free', text=self.strings.get(hdr + 0x10)); self.ret(0, 4)

    def run(self):
        self.call(ENTRY, ecx=self.states['bombard'], args=[self.target])
        return {'events': self.events, 'gtid': self.r(self.states['bombard'] + 0x10)}


def cases():
    rnd = random.Random(0x527a60)
    rows = []
    for i in range(60):
        rows.append({'name': 'target%d' % i, 'precision': 'Single24', 'states': {}, 'stack': [], 'words': [0, 0, 0, 0],
                     'target': None if i % 4 == 0 else {'gtid': rnd.choice([0x00a10a10, 0x00a20a20]),
                                                        'name': rnd.choice(['Fort', 'Tower', 'Walls'])},
                     'old_gtid': rnd.choice([0, 0x00a10a10, 0x00b10b10]), 'temp_refs': rnd.choice([1, 1, 2]),
                     'assign_by_ref': False})
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        try:
            m = TargetMachine(c); result = m.run()
        except Exception:
            print(c['name']); raise
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    cover = {'527A60': [len([a for a in visited if ENTRY <= a < END]), body_instructions(ENTRY, END)]}
    d.update(instructions=instructions(visited), coverage=cover, cases=out)
    p = HERE / 'native-cai-bombard-target.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'coverage': cover, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
