"""Original KKC_CAIStateRout::Think 521240 (regiment rout state, slot +C).

Executes unchanged together with 529E60 (TAIState::Think base), 52A000 (is
current), 4F02B0 (world RNG accessor) and 52A1F0 (arrival test). Seams, all
recorded in order: profiler/diagnostic 77DC50 and 77DA70, Mind virtual +178
(routing time elapsed), +144 (destination reached), destination selection
52B530 (proven separately by native_rout_select.py; here it returns case
values and must receive the world RNG word), move activation 520440 on the
embedded move state Mind+68, embedded move state virtual +C (its Think) and
the Mind Updatable (+8) virtual +4 scheduler. 529F50 executes natively and
its write into the current command (+C |= 4, +18/+1C) is snapshotted.
"""
from pathlib import Path
import copy, json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, PRECISIONS, f32, header, instructions, sha


class RoutThinkMachine(Machine):
    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = c
        world = self.world = self.alloc(0x200); self.w(0xb36534, world)
        self.w(world + 0xe0, c['world_time']); self.w(world + 0xd4, 0x1234)
        svars = self.alloc(0x100); rt = self.alloc(4); self.w(rt, c['routing_time']); self.w(svars + 0xb0, rt)
        self.w(0xb39ab0, svars)
        self.w(0xb3655c, self.alloc(0x40))
        self.seam(0x77dc50, lambda m: self.ret(0, 8))
        self.seam(0x77da70, lambda m: self.ret(0, 12))
        actor = self.actor = self.alloc(0x40)
        defn = self.alloc(0x20); self.w(defn + 8, self.alloc(4)); self.w(actor + 4, defn); self.w(actor + 0x14, 0x20002)
        self.w(actor + 0x20, c['actor_position'][0]); self.w(actor + 0x24, c['actor_position'][1])
        mind = self.mind = self.alloc(0x200)
        mvt = self.alloc(0x200); self.w(mind, mvt)
        self.w(mvt + 0x144, self.stub(lambda m: self.v_mind('reached')))
        self.w(mvt + 0x178, self.stub(lambda m: self.v_mind('end_rout')))
        uvt = self.alloc(0x10); self.w(uvt + 4, self.stub(self.v_schedule)); self.w(mind + 8, uvt)
        self.w(mind + 4, actor)
        state = self.state = self.alloc(0x40)
        meta = self.alloc(0x10); self.w(meta, self.alloc(4))
        self.w(state + 4, mind); self.w(state + 8, meta); self.w(state + 0xc, c['status']); self.w(state + 0x10, c['entered'])
        other = self.alloc(0x40)
        head = 0
        if c['current'] is not None:
            head = self.alloc(8); self.w(head, state if c['current'] else other)
        self.w(mind + 0x10, head)
        self.command = 0
        if c['command'] is not None:
            self.command = self.alloc(0x40); self.w(self.command + 0xc, c['command']['flags'])
            self.w(self.command + 0x18, 0x11111111); self.w(self.command + 0x1c, 0x22222222)
        self.w(mind + 0x20, self.command)
        move = self.move = mind + 0x68
        svt = self.alloc(0x40); self.w(svt + 0xc, self.stub(self.v_move_think)); self.w(move, svt)
        self.w(move + 4, mind); self.w(move + 0xc, c['move_status'])
        self.w(move + 0x14, c['move_destination'][0]); self.w(move + 0x18, c['move_destination'][1])
        self.w(0xb279c0, c['zero_vector'][0]); self.w(0xb279c4, c['zero_vector'][1])
        self.seam(0x52b530, self.s_select)
        self.seam(0x520440, self.s_activate)
        self.seam(0x404ed0, self.s_log)

    def snap(self):
        return {'status': self.r(self.state + 0xc),
                'command': None if not self.command else {'flags': self.r(self.command + 0xc),
                                                          'point': [self.r(self.command + 0x18), self.r(self.command + 0x1c)]}}

    def v_mind(self, name):
        assert self.ecx() == self.mind
        self.events.append({'event': name, **self.snap()}); self.ret(0)

    def v_schedule(self, m):
        assert self.ecx() == self.mind + 8
        self.events.append({'event': 'schedule', 'delay': self.arg(1), 'data': self.arg(2), **self.snap()}); self.ret(0, 8)

    def v_move_think(self, m):
        assert self.ecx() == self.move
        self.events.append({'event': 'move_think', **self.snap()}); self.ret(0)

    def s_select(self, m):
        assert self.ecx() == self.mind and self.arg(1) == self.world + 0xd4
        out = self.arg(2)
        assert [self.r(out), self.r(out + 4)] == [0, 0]
        dest = self.c['selected']
        self.events.append({'event': 'select', 'result': dest, **self.snap()})
        if dest is not None:
            self.w(out, dest[0]); self.w(out + 4, dest[1])
        self.ret(int(dest is not None), 8)

    def s_activate(self, m):
        assert self.ecx() == self.move
        args = [self.arg(i) for i in range(1, 7)]
        self.events.append({'event': 'activate_move', 'args': args, **self.snap()}); self.ret(0, 0x18)

    def s_log(self, m):
        self.events.append({'event': 'diagnostic_log'}); self.ret(0)

    def run(self):
        self.call(0x521240, ecx=self.state)
        return {'events': self.events, **self.snap()}


def cases():
    base = {'name': 'base', 'precision': 'Single24', 'world_time': f32(100), 'routing_time': f32(25), 'entered': f32(90),
            'status': 2, 'current': True, 'move_status': 0, 'command': None, 'selected': [f32(321.5), f32(-7.25)],
            'actor_position': [f32(50), f32(60)], 'move_destination': [f32(50.3), f32(60.4)], 'zero_vector': [0, 0]}
    rows = []

    def add(name, **k):
        c = copy.deepcopy(base); c.update(k); c['name'] = name; rows.append(c)

    for status in [0, 1, 2, 3]:
        add(f'status{status}', status=status)
    for cur in [None, False, True]:
        for t, e in [(100, 90), (100, 75), (100, 74.99), (115, 90), (0, 0), (25, 0)]:
            add(f'timer_{cur}_{t}_{e}', current=cur, world_time=f32(t), entered=f32(e))
    for ms in [0, 1, 2, 3]:
        for sel in [None, [f32(1), f32(2)]]:
            add(f'move{ms}_sel{sel is not None}', move_status=ms, selected=sel)
    add('zero_vector_nonzero', zero_vector=[f32(0.5), f32(-0.5)])
    for cmd in [None, {'flags': 0}, {'flags': 0xfffffffb}]:
        for dest in [[50.3, 60.4], [50.3, 60.39], [50.5, 60], [50.49, 60], [10, 10]]:
            add(f'moving_cmd{cmd}_dest{dest}', move_status=2, command=cmd, move_destination=[f32(dest[0]), f32(dest[1])])
    rnd = random.Random(0x521240)
    for i in range(30):
        add(f'random{i}', status=rnd.choice([0, 1, 2]), current=rnd.choice([None, False, True, True]),
            world_time=f32(rnd.uniform(0, 200)), entered=f32(rnd.uniform(0, 200)), routing_time=f32(rnd.uniform(0, 60)),
            move_status=rnd.choice([0, 1, 2, 2]), command=rnd.choice([None, {'flags': rnd.getrandbits(32)}]),
            selected=rnd.choice([None, [f32(rnd.uniform(-500, 500)), f32(rnd.uniform(-500, 500))]]),
            actor_position=[f32(rnd.uniform(-100, 100)), f32(rnd.uniform(-100, 100))])
        rows[-1]['move_destination'] = [f32(struct.unpack('<f', struct.pack('<I', rows[-1]['actor_position'][0]))[0] + rnd.uniform(-0.6, 0.6)),
                                        f32(struct.unpack('<f', struct.pack('<I', rows[-1]['actor_position'][1]))[0] + rnd.uniform(-0.6, 0.6))]
    for p in PRECISIONS:
        for c in rows:
            yield {**copy.deepcopy(c), 'name': c['name'] + '_' + p, 'precision': p}


def main():
    out, visited = [], {}
    for c in cases():
        try:
            m = RoutThinkMachine(c); result = m.run()
        except Exception:
            print(c['name']); raise
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-rout-think.json'
    p.write_text(json.dumps(d, indent=1) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
