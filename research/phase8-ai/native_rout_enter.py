"""Original KKC_CAIStateRout::OnEnter 51F190 executing unchanged on the
emulated regiment Mind of native_cai_engage.py (rout state at Mind+20C):
status +C = 2, entry time +10 = [B36534]+E0, organization halt 4D3FA0 (seam),
engage target clear 51F040 (with its callees as in native_cai_engage.py) and
the pending command release 52C0D0 (Mind+18/+1C list of refcounted command
objects; their destructors are recorded).
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import header, instructions, sha
from native_cai_engage import EngageMachine
import native_cai_engage


class RoutEnterMachine(EngageMachine):
    def __init__(self, c):
        super().__init__(c)
        world = self.alloc(0x100); self.w(0xb36534, world); self.w(world + 0xe0, c['time'])
        self.rout = self.mind + 0x20c
        self.w(self.rout + 4, self.mind)
        self.w(self.rout + 0xc, c['status']); self.w(self.rout + 0x10, c['entered'])
        self.seam(0x4d3fa0, self.s_halt)
        cvt = self.alloc(0x40)
        self.w(cvt, self.stub(self.c_destroy))
        self.commands = {}
        head = tail = 0
        for i, cmd in enumerate(reversed(c['commands'])):
            p = 0
            if cmd is not None:
                p = self.alloc(0x40); self.w(p, cvt); self.w(p + 4, cmd['refs'])
                self.commands[p] = cmd['id']
            node = self.alloc(8); self.w(node, p); self.w(node + 4, head); head = node
            if not tail:
                tail = node
        self.w(self.mind + 0x18, head); self.w(self.mind + 0x1c, tail)

    def s_halt(self, m):
        assert self.ecx() == self.org
        self.events.append({'event': 'organization_halt'}); self.ret(0)

    def c_destroy(self, m):
        assert self.arg(1) == 1
        self.events.append({'event': 'command_destroyed', 'command': self.commands[self.ecx()]}); self.ret(0, 4)

    def run(self):
        from unicorn import UcError
        try:
            self.call(0x51f190, ecx=self.rout)
        except (AssertionError, UcError):
            if not self.null_accesses:
                raise
            return {'fault': 'null dereference', 'events': self.events}
        refs = {str(cid): self.r(p + 4) for p, cid in sorted(self.commands.items(), key=lambda x: x[1])}
        return {'events': self.events, 'list': self.read_list(), 'status': self.r(self.rout + 0xc),
                'entered': self.r(self.rout + 0x10), 'pending_head': self.r(self.mind + 0x18),
                'pending_tail': self.r(self.mind + 0x1c), 'command_refs': refs}


def cases():
    rnd = random.Random(0x51f190)
    base = native_cai_engage.cases()
    rows = []
    for i, c in enumerate(base[:80]):
        c = dict(c)
        c['name'] = 'rout_enter%d' % i
        c['entry'] = 'rout_enter'
        c['time'] = rnd.getrandbits(32) & 0x7f7fffff
        c['status'] = rnd.choice([0, 1, 2])
        c['entered'] = rnd.getrandbits(32) & 0x7f7fffff
        cmds = []
        for k in range(rnd.choice([0, 0, 1, 2, 4])):
            cmds.append(None if rnd.random() < 0.1 else {'id': 100 + k, 'refs': rnd.choice([1, 1, 2, 3])})
        c['commands'] = cmds
        rows.append(c)
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = RoutEnterMachine(c)
        result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-rout-enter.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
