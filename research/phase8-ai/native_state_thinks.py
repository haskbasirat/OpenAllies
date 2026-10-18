"""Original small Mind-state Think dispatchers (vtable slot +C).

51E110 cai_retreat, 52DF30 uai_deploy, 51DE10 cai_regroup, 51E260
cai_exhausted and 523930 cai_chasing execute unchanged with 529E60, 529C30,
52A000 and (for exhausted) 52A340 pop. `this` is placed at its real embedded
offset inside an emulated Mind (regiment: retreat 1F0, regroup 1A8,
exhausted 270, chasing 120; Element: deploy 48).

Seams (recorded in order with state statuses and the Mind stack): embedded
state Think stubs at Mind+34/+68/+7C (slot +C), state enter/leave (+18/+1C),
Mind virtuals +140/+150/+1A0/+1A4, Updatable(+8) virtual +4, Organization
status 578140 and chase predicate 520D90, diagnostic logger 404ED0. The
regroup anchor (organization component+7C -> +44 object, +8 flags) is
emulated data read natively.
"""
from pathlib import Path
import copy, json, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, header, instructions, sha

ENTRIES = {'retreat': (0x51e110, 0x1f0), 'deploy': (0x52df30, 0x48), 'regroup': (0x51de10, 0x1a8),
           'exhausted': (0x51e260, 0x270), 'chasing': (0x523930, 0x120)}
EMBEDDED = [0x34, 0x68, 0x7c]


class ThinkMachine(Machine):
    def __init__(self, c):
        super().__init__('Extended64')
        self.c = copy.deepcopy(c)
        mind = self.mind = self.alloc(0x400)
        mvt = self.alloc(0x200); self.w(mind, mvt)
        self.w(mvt + 0x140, self.stub(lambda m: self.v_mind('mind_virtual_140', [self.arg(1)], 4)))
        self.w(mvt + 0x150, self.stub(lambda m: self.v_mind('mind_virtual_150', [], 0)))
        self.w(mvt + 0x1a0, self.stub(lambda m: self.v_mind('default', [], 0)))
        self.w(mvt + 0x1a4, self.stub(lambda m: self.v_mind('changed', [], 0)))
        uvt = self.alloc(0x10); self.w(uvt + 4, self.stub(self.v_schedule)); self.w(mind + 8, uvt)
        actor = self.alloc(0x100)
        defn = self.alloc(0x20); self.w(defn + 8, self.alloc(4)); self.w(actor + 4, defn); self.w(actor + 0x14, 0x30003)
        comp = self.comp = self.alloc(0x100); self.w(actor + 0x7c, comp)
        if c['anchor'] is not None:
            anchor = self.alloc(0x20); self.wb(anchor + 8, c['anchor']); self.w(comp + 0x44, anchor)
        self.w(mind + 4, actor)
        entry, offset = ENTRIES[c['kind']]
        self.offsets = {'this': offset, 'sleep': 0x34, 'move68': 0x68, 'move7c': 0x7c, 'other': 0x300}
        svt = self.alloc(0x40)
        self.w(svt + 0x18, self.stub(lambda m: self.v_state('enter')))
        self.w(svt + 0x1c, self.stub(lambda m: self.v_state('leave')))
        for name, off in self.offsets.items():
            p = mind + off
            vt = svt
            if off in EMBEDDED:
                vt = self.alloc(0x40)
                self.w(vt + 0xc, self.stub(lambda m, off=off: self.v_embedded(off)))
                self.w(vt + 0x18, self.r(svt + 0x18)); self.w(vt + 0x1c, self.r(svt + 0x1c))
            meta = self.alloc(0x10); self.w(meta, self.alloc(4))
            self.w(p, vt); self.w(p + 4, mind); self.w(p + 8, meta)
        self.w(mind + offset + 0x10, c['target'])
        self.entry = entry
        self.write()
        self.seam(0x578140, self.s_status)
        self.seam(0x520d90, self.s_chase)
        self.seam(0x6f7430, lambda m: self.ret(0))
        self.seam(0x404ed0, lambda m: (self.events.append({'event': 'diagnostic_log'}), self.ret(0)))

    def sid(self, p):
        for name, off in self.offsets.items():
            if p == self.mind + off:
                return name
        return None if p == 0 else hex(p)

    def write(self):
        for name, st in self.c['states'].items():
            self.w(self.mind + self.offsets[name] + 0xc, st)
        head = tail = 0
        for ident in reversed(self.c['stack']):
            node = self.alloc(8); self.w(node, 0 if ident is None else self.mind + self.offsets[ident]); self.w(node + 4, head); head = node
            if not tail:
                tail = node
        self.w(self.mind + 0x10, head); self.w(self.mind + 0x14, tail)

    def snap(self):
        stack, node = [], self.r(self.mind + 0x10)
        while node:
            stack.append(self.sid(self.r(node))); node = self.r(node + 4)
        return {'states': {n: self.r(self.mind + self.offsets[n] + 0xc) for n in self.c['states']}, 'stack': stack}

    def event(self, name, extra=None):
        self.events.append({'event': name, **(extra or {}), **self.snap()})

    def v_mind(self, name, args, pop):
        assert self.ecx() == self.mind
        self.event(name, {'args': args}); self.ret(0, pop)

    def v_state(self, name):
        self.event(name, {'state': self.sid(self.ecx())}); self.ret(0)

    def v_embedded(self, off):
        assert self.ecx() == self.mind + off
        self.event('embedded_think', {'offset': off}); self.ret(0)

    def v_schedule(self, m):
        assert self.ecx() == self.mind + 8
        self.event('schedule', {'delay': self.arg(1), 'data': self.arg(2)}); self.ret(0, 8)

    def s_status(self, m):
        assert self.ecx() == self.comp
        self.event('organization_status', {'result': self.c['status']}); self.ret(self.c['status'])

    def s_chase(self, m):
        assert self.ecx() == self.mind + self.offsets['this']
        self.event('chase_continue', {'target': self.arg(1), 'result': self.c['chase']}); self.ret(int(self.c['chase']), 4)

    def run(self):
        self.call(self.entry, ecx=self.mind + self.offsets['this'])
        return {'events': self.events, **self.snap()}


def cases():
    rows = []
    states = {'this': 2, 'sleep': 0, 'move68': 0, 'move7c': 0, 'other': 2}
    base = {'states': states, 'stack': ['this'], 'anchor': None, 'status': 0, 'chase': False, 'target': 0x00050007}

    def add(name, **k):
        c = copy.deepcopy(base); c.update(k); c['name'] = name; rows.append(c)

    for kind in ENTRIES:
        for status in [0, 1, 2, 3]:
            add(f'{kind}_this_status{status}', kind=kind, states={**states, 'this': status})
    for anchor in [None, 0, 1, 2, 0xff, 0xfe]:
        add(f'regroup_anchor_{anchor}', kind='regroup', anchor=anchor)
    for st in [0, 1, 2, 3, 0xffffffff]:
        for stack in [['this'], ['other', 'this'], ['this', 'other'], ['this', 'sleep', 'other'], [], [None], ['this', None]]:
            add(f'exhausted_status{st}_stack{stack}', kind='exhausted', status=st, stack=stack)
    for chase in [False, True]:
        for target in [0, 0x00050007, 0xffffffff]:
            add(f'chasing_{chase}_{target:x}', kind='chasing', chase=chase, target=target)
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = ThinkMachine(c)
        try:
            result = m.run()
        except AssertionError:
            if not m.null_accesses:
                print(c['name']); raise
            result = {'fault': 'null state dereference', 'events': m.events}
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-state-thinks.json'
    p.write_text(json.dumps(d, indent=1) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
