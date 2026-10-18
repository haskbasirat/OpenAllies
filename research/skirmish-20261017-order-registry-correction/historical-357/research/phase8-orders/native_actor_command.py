"""Original KKC_GActorCommand::Process 0x4D9B60 (actor command delivery, used
by the TellActorCommand decree process, combat special operations/summons,
triggers, minds and navigation) under Unicorn, on synthetic actor graphs.

0x4D9B60(this = command, actor): slot k = [[command+8]+0x18]; handler =
actor+0x60+4k. With a handler: actor virtual +0x38(command type) decides;
true -> return handler virtual +0x24(command) (low byte), false -> 0. Without
one: actor virtual +0x158() true -> actor = virtual +0x15C() (a null result
ends the walk with the "ignoring command to null actor" log); false -> actor
= [[actor+0x7C]+0x44] (a null container or member ends without log). A null
actor logs (0x404F30, GActorCommands.cpp line 0xF7) and returns 0.

Every virtual is a recording seam at its own fresh stub address (returning
the case's value); the walk, slot indexing and return values are original.
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase4-orders'))
from recover_orders import OrdersMachine, PE0, PE1, sha
sys.path.insert(0, str(ROOT / 'research/phase8-net'))
import null_guard
from unicorn.x86_const import *

ACTORS = 0x2060000; CMD = 0x2068000; CTYPE = 0x2068100; STUBS = 0x7700000
SLOTS = 6

class CommandMachine(OrdersMachine):
    def __init__(self):
        super().__init__()
        self.mode = 'command'; self.stub_map = {}; self.next_stub = STUBS; self.trace = []
        self.u.mem_map(STUBS, 0x20000)
        null_guard.install(self)
    def stub(self, kind, owner, pop):
        a = self.next_stub; self.next_stub += 0x10
        self.u.mem_write(a, b'\xc3')
        self.stub_map[a] = (kind, owner, pop)
        return a
    def hook(self, u, pc, size, user):
        if self.mode == 'command':
            if pc in self.stub_map:
                kind, owner, pop = self.stub_map[pc]
                spec = self.spec[owner]
                if kind == 'accepts':
                    self.trace.append({'call': 'accepts', 'actor': owner, 'type_ok': self.arg() == CTYPE})
                    self.ret(4, int(spec['accepts']))
                elif kind == 'handle':
                    self.trace.append({'call': 'handle', 'actor': owner, 'command_ok': self.arg() == CMD})
                    self.ret(4, spec['result'])
                elif kind == 'has_redirect':
                    self.trace.append({'call': 'has_redirect', 'actor': owner})
                    self.ret(0, int(spec['redirect'] is not False))
                elif kind == 'redirect':
                    self.trace.append({'call': 'redirect', 'actor': owner})
                    r = spec['redirect']
                    self.ret(0, 0 if r is None else ACTORS + r * 0x200)
                return
            if pc == 0x404f30:
                self.trace.append({'call': 'log', 'line': self.arg(2)}); self.ret(); return
            self.visited[pc] = size
        super().hook(u, pc, size, user)
    def build(self, actors, slot):
        self.spec = {}
        m = self
        m.w(CMD + 8, CTYPE); m.w(CTYPE + 0x18, slot)
        for i, a in enumerate(actors):
            base = ACTORS + i * 0x200
            m.u.mem_write(base, b'\0' * 0x200)
            vt = m.alloc(0x200)
            m.w(vt + 0x38, m.stub('accepts', i, 4)); m.w(vt + 0x158, m.stub('has_redirect', i, 0)); m.w(vt + 0x15c, m.stub('redirect', i, 0))
            m.w(base, vt)
            for k, h in a['handlers'].items():
                hobj = m.alloc(0x20); hvt = m.alloc(0x40); m.w(hvt + 0x24, m.stub('handle', i, 4)); m.w(hobj, hvt)
                m.w(base + 0x60 + 4 * int(k), hobj)
            if a['container'] is not None:
                cont = m.alloc(0x80); m.w(base + 0x7c, cont)
                m.w(cont + 0x44, 0 if a['container'] == 'null' else ACTORS + a['container'] * 0x200)
            self.spec[i] = a

def random_graph(rng):
    n = rng.randrange(1, 6)
    actors = []
    for i in range(n):
        handlers = {str(k): True for k in range(SLOTS) if rng.random() < 0.3}
        kind = rng.random()
        redirect = False; container = None
        # Targets only later actors: a cycle makes the original loop forever.
        if kind < 0.3: redirect = rng.choice([None] + list(range(i + 1, n)))
        elif kind < 0.7: container = rng.choice(['null'] + list(range(i + 1, n)))
        actors.append({'handlers': handlers, 'accepts': rng.random() < 0.7, 'result': rng.choice([0, 1, 0x101, 0xff00]),
                       'redirect': redirect, 'container': container})
    return actors

def main():
    m = CommandMachine(); rng = random.Random(4096); rows = []
    for k in range(300):
        actors = random_graph(rng)
        slot = rng.randrange(SLOTS)
        start = None if k % 25 == 0 else rng.randrange(len(actors))
        m.build(actors, slot)
        m.trace = []
        try:
            r = m.call(0x4d9b60, [0 if start is None else ACTORS + start * 0x200], this=CMD, callee_pop=4)
            status = 'returned'
        except Exception as e:
            r = None; status = 'error: ' + type(e).__name__
        rows.append({'actors': actors, 'slot': slot, 'start': start, 'status': status,
                     'result': None if r is None else r & 0xff, 'trace': m.trace})
    ranges = []
    for a, size in sorted(m.visited.items()):
        if not PE0.base <= a < PE0.base + PE0.image_size: continue
        if ranges and ranges[-1][1] >= a: ranges[-1][1] = max(ranges[-1][1], a + size)
        else: ranges.append([a, a + size])
    windows = [{'start': hex(a), 'end': hex(b), 'sha256': sha(PE0.range(a, b - a)),
                'v14_identical': PE0.range(a, b - a) == PE1.range(a, b - a)} for a, b in ranges]
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
              'producer_sha256': sha(Path(__file__).read_bytes()), 'cases': rows, 'executed_windows': windows}
    (HERE / 'native-actor-command.json').write_text(json.dumps(report) + '\n')
    import collections
    print(len(rows), collections.Counter(r['status'] for r in rows), 'v14 differs at', [w['start'] for w in windows if not w['v14_identical']])

if __name__ == '__main__':
    main()
