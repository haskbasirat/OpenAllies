"""Original Mind/TAIState primitives used by the state Think functions.

- 52A340 Mind pop (finish the current state): leave +1C, unlink and free the
  head node, schedule Updatable(+8) virtual +4 (0, 0), then resume the new
  head (status 1, enter +18) or restore the default (+1A0), and finally the
  state-changed virtual +1A4. Callbacks may mutate statuses and stacks; native
  rereads are preserved.
- 529E60 KKC_TAIState::Think base: status forced to 2 (the one-time wrong
  activity log behind latch B3D248 is a diagnostic seam).
- 52A000 state-is-current predicate (Mind+10 head node's state).
- 52A1F0 Mind actor within 0.5 world units of a point (squared < 0.25).

All four execute unchanged; seams are the virtual stubs, the allocator free
6F7430 and the diagnostic logger 404ED0.
"""
from pathlib import Path
import copy, json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, PRECISIONS, f32, header, instructions, sha


class MindMachine(Machine):
    def __init__(self, c):
        super().__init__(c.get('precision', 'Extended64'))
        self.c = copy.deepcopy(c)
        self.mutations = self.c.get('mutations', [])
        self.mvt = self.alloc(0x200); self.svt = self.alloc(0x40); self.uvt = self.alloc(0x10)
        self.w(self.mvt + 0x1a0, self.stub(lambda m: self.v_mind('default')))
        self.w(self.mvt + 0x1a4, self.stub(lambda m: self.v_mind('changed')))
        self.w(self.svt + 0x18, self.stub(lambda m: self.v_state('enter')))
        self.w(self.svt + 0x1c, self.stub(lambda m: self.v_state('leave')))
        self.w(self.uvt + 4, self.stub(self.v_schedule))
        self.actor = self.alloc(0x40)
        defn = self.alloc(0x20); self.w(defn + 8, self.alloc(4)); self.w(self.actor + 4, defn); self.w(self.actor + 0x14, 0x10001)
        self.w(self.actor + 0x20, c.get('actor_position', [0, 0])[0]); self.w(self.actor + 0x24, c.get('actor_position', [0, 0])[1])
        self.mind = self.alloc(0x40); self.w(self.mind, self.mvt); self.w(self.mind + 4, self.actor); self.w(self.mind + 8, self.uvt)
        self.states = {}
        for key in c['states']:
            p = self.alloc(0x20); meta = self.alloc(0x10); name = self.alloc(4)
            self.w(p, self.svt); self.w(p + 4, self.mind); self.w(p + 8, meta); self.w(meta, name)
            self.states[key] = p
        self.freed = []
        self.write()
        self.seam(0x6f7430, self.s_free)
        self.seam(0x404ed0, self.s_log)

    def sp(self, key):
        return 0 if key is None else self.states[key]

    def sid(self, p):
        if p == 0:
            return None
        return next(k for k, v in self.states.items() if v == p)

    def write(self):
        for key, st in self.c['states'].items():
            self.w(self.states[key] + 0xc, st['status'])
        head = tail = 0
        for ident in reversed(self.c['stack']):
            node = self.alloc(8); self.w(node, self.sp(ident)); self.w(node + 4, head); head = node
            if not tail:
                tail = node
        self.w(self.mind + 0x10, head); self.w(self.mind + 0x14, tail)

    def snapshot(self):
        states = {k: {'status': self.r(p + 0xc)} for k, p in self.states.items()}
        stack, node = [], self.r(self.mind + 0x10)
        while node:
            stack.append(self.sid(self.r(node))); node = self.r(node + 4)
        tail = self.r(self.mind + 0x14)
        return {'states': states, 'stack': stack, 'tail_state': None if tail == 0 else self.sid(self.r(tail)) if self.r(tail) else 'null'}

    def event(self, name, extra=None):
        snap = self.snapshot()
        self.events.append({'event': name, **({} if extra is None else extra), 'arena': snap})
        if self.mutations and self.mutations[0]['event'] == name:
            m = self.mutations.pop(0)
            self.c['states'] = copy.deepcopy(snap['states'])
            for k, v in m.get('states', {}).items():
                self.c['states'][k].update(v)
            self.c['stack'] = m.get('stack', snap['stack'])
            self.write()

    def v_mind(self, name):
        assert self.ecx() == self.mind
        self.event(name); self.ret(0)

    def v_state(self, name):
        self.event(name, {'state': self.sid(self.ecx())}); self.ret(0)

    def v_schedule(self, m):
        assert self.ecx() == self.mind + 8
        self.event('schedule', {'delay': self.arg(1), 'data': self.arg(2)}); self.ret(0, 8)

    def s_free(self, m):
        self.freed.append(self.arg(1)); self.ret(0)

    def s_log(self, m):
        self.events.append({'event': 'diagnostic_log'}); self.ret(0)

    def run(self):
        e = self.c['entry']
        result = None
        if e == 'pop':
            try:
                self.call(0x52a340, ecx=self.mind)
            except AssertionError:
                assert self.null_accesses, self.c['name']
                return {'fault': 'null state dereference', 'events': self.events}
        elif e == 'think_base':
            self.wb(0xb3d248, int(self.c['latch']))
            self.call(0x529e60, ecx=self.sp(self.c['state']))
            result = self.rb(0xb3d248)
        elif e == 'is_current':
            result = self.call(0x52a000, ecx=self.sp(self.c['state'])) & 0xff
        elif e == 'within':
            result = self.call(0x52a1f0, ecx=self.mind, args=self.c['point']) & 0xff
        assert not self.mutations, self.c['name']
        return {'result': result, 'arena': self.snapshot(), 'events': self.events, 'freed_nodes': len(self.freed)}


def cases():
    states = {str(i): {'status': s} for i, s in enumerate([0, 1, 2, 3, 2], 1)}
    rows = []

    def add(**c):
        c.setdefault('states', copy.deepcopy(states)); rows.append(c)

    for stack in [['1'], ['1', '2'], ['2', '1', '3'], ['1', None], ['3', '3']]:
        add(name=f'pop_{stack}', entry='pop', stack=stack)
    add(name='pop_leave_clears_stack', entry='pop', stack=['1', '2'], mutations=[{'event': 'leave', 'stack': []}])
    add(name='pop_leave_pushes', entry='pop', stack=['1', '2'], mutations=[{'event': 'leave', 'stack': ['4', '1', '2']}])
    add(name='pop_leave_changes_status', entry='pop', stack=['1', '2'], mutations=[{'event': 'leave', 'states': {'2': {'status': 7}}}])
    add(name='pop_schedule_changes_stack', entry='pop', stack=['1', '2', '3'], mutations=[{'event': 'schedule', 'stack': ['5']}])
    add(name='pop_schedule_empties', entry='pop', stack=['1', '2'], mutations=[{'event': 'schedule', 'stack': []}])
    add(name='pop_enter_mutates', entry='pop', stack=['1', '2'], mutations=[{'event': 'enter', 'stack': ['3'], 'states': {'2': {'status': 9}}}])
    add(name='pop_default_mutates', entry='pop', stack=['1'], mutations=[{'event': 'default', 'stack': ['4']}])
    for status in [0, 1, 2, 3, 0xffffffff]:
        for latch in [0, 1]:
            add(name=f'think_base_{status:x}_{latch}', entry='think_base', state='1', latch=latch,
                stack=['1'], states={**copy.deepcopy(states), '1': {'status': status}})
    for stack, state in [([], '1'), (['1'], '1'), (['2', '1'], '1'), ([None, '1'], '1'), (['1'], '2')]:
        add(name=f'is_current_{stack}_{state}', entry='is_current', stack=stack, state=state)
    rnd = random.Random(0x52a1f0)
    points = [([0, 0], [0, 0]), ([10, 10], [10.5, 10]), ([10, 10], [10.49, 10]), ([10, 10], [10.3, 10.4]),
              ([10, 10], [10.3, 10.39]), ([-3, 7], [-3, 7.5]), ([1e6, 1e6], [1e6 + 0.25, 1e6 + 0.25])]
    points += [([rnd.uniform(-100, 100), rnd.uniform(-100, 100)], None) for _ in range(20)]
    for i, (pos, pt) in enumerate(points):
        if pt is None:
            pt = [pos[0] + rnd.uniform(-0.6, 0.6), pos[1] + rnd.uniform(-0.6, 0.6)]
        for p in PRECISIONS:
            add(name=f'within_{i}_{p}', entry='within', stack=[], precision=p,
                actor_position=[f32(pos[0]), f32(pos[1])], point=[f32(pt[0]), f32(pt[1])])
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        try:
            m = MindMachine(c); result = m.run()
        except Exception:
            print(c['name']); raise
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-mind-primitives.json'
    p.write_text(json.dumps(d, indent=1) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
