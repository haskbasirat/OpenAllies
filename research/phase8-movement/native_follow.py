"""Original UAIStateMove formation-following speed control.

Whole unchanged routines execute: 0x52FF00 (ActionGo's mode-3 formation
gate) with slot lag 0x52E400, path-length difference 0x52E300, path length
0x52CE70, vector inequality 0x46B000, squared distance 0x45D880, catch-up
0x577090 and Mover speed clamp 0x576D20.

Seams (recorded): Mind virtual +0x44 (returns the organization/leader path
collection supplied by the case), leader moving predicate 0x4CEDE0, stop
0x52D660, UAI timer virtual (UAI+8)+4 and state finish 0x52A3C0.
"""
import copy, json, math, random, struct, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, NullDereference, write_document, f2b, PRECISION  # noqa: E402


class FollowMachine(Machine):
    def __init__(self, case):
        super().__init__(case['precision'])
        self.c = case
        s = case['state']
        self.state = self.alloc(0x100)
        uai = self.alloc(0x40)
        self.actor = self.alloc(0x100)
        self.mover = self.alloc(0x100)
        timer_vt = self.alloc(0x20)
        self.w32(self.state + 4, uai)
        self.w32(uai + 4, self.actor)
        self.w32(uai + 8, timer_vt)
        self.w32(timer_vt + 4, self.stub(lambda m: (m.event('Schedule', m.arg(0), m.arg(1)), m.ret(8))))
        self.uai = uai
        self.w32(self.state + 0x24, s['requested_speed'])
        self.w32(self.state + 0x30, s['mode'])
        self.path(self.state + 0x58, s['path'])
        a = case['actor']
        self.w32(self.actor + 0x20, *a['position'])
        self.w32(self.actor + 0x6c, self.mover)
        self.w32(self.mover + 0x24, a['mover_requested_speed'])
        self.w32(self.mover + 0x28, a['mover_maximum_speed'])
        self.w32(self.mover + 0x50, a['mover_speed_factor'])
        cache = self.alloc(0x100)
        self.w32(0xb39ab0, cache)
        self.w32(cache + 0x48, cache + 0x80)
        self.w32(cache + 0x80, case['catchup_factor'])
        e = case['element']
        if e is not None:
            element = self.alloc(0x40)
            self.w32(self.actor + 0x80, element)
            self.w32(element + 0x18, e['slot'] & 0xffffffff)
            o = e['organization']
            if o is not None:
                org = self.alloc(0x100)
                comp = self.alloc(0x100)
                self.w32(element + 0x10, org)
                self.w32(org + 0x20, *o['position'])
                self.w32(org + 0x2c, *o['facing'])
                self.w32(org + 0x70, self.mind(o['path'], 'OrganizationPath'))
                self.w32(org + 0x7c, comp)
                self.w32(comp + 4, org)
                slots = self.alloc(8 * max(1, len(o['slot_positions'])))
                for i, p in enumerate(o['slot_positions']):
                    self.w32(slots + 8 * i, *p)
                self.w32(comp + 0x78, slots)
                l = o['leader']
                if l is not None:
                    leader = self.alloc(0x100)
                    self.w32(leader + 0x20, *l['position'])
                    self.w32(leader + 0x70, self.mind(l['path'], 'LeaderPath'))
                    self.w32(comp + 0x40, leader)
        self.responses = list(case.get('responses', []))
        self.seams[0x4cede0] = self.leader_moving
        self.seams[0x52d660] = lambda m: (m.event('StopAction'), m.ret(0))
        self.seams[0x52a3c0] = self.finish

    def stub(self, handler):
        address = self.stub_next
        self.stub_next += 16
        self.seams[address] = handler
        return address

    def path(self, collection, p):
        nodes = []
        for words in p['nodes']:
            n = self.alloc(0x28)
            self.w32(n, *words)
            nodes.append(n)
        for i, n in enumerate(nodes):
            self.w32(n + 0x20, nodes[i - 1] if i else 0)
            self.w32(n + 0x24, nodes[i + 1] if i + 1 < len(nodes) else 0)
        self.w32(collection + 4, nodes[0] if nodes else 0, nodes[-1] if nodes else 0,
                 p['kind1_count'] & 0xffffffff, p['kind2_count'] & 0xffffffff)

    def mind(self, path, name):
        mind = self.alloc(0x40)
        vt = self.alloc(0x100)
        collection = self.alloc(0x20)
        self.path(collection, path)
        self.w32(mind, vt)
        self.w32(vt + 0x44, self.stub(lambda m: (m.event(name), m.ret(0, collection))))
        return mind

    def leader_moving(self, m):
        m.event('LeaderMoving')
        r = self.responses.pop(0)
        assert r['name'] == 'LeaderMoving'
        m.ret(0, int(r['value']))

    def finish(self, m):
        assert m.ecx() == self.uai
        m.event('FinishState')
        m.ret(0)

    def run(self):
        eax = self.call(0x52ff00, self.state)
        assert self.final_esp == 0x2f08004
        assert self.fpu_top() == 0
        assert not self.responses
        return {'return_al': eax & 255, 'state_mode': self.r32(self.state + 0x30),
                'state_requested_speed': self.r32(self.state + 0x24),
                'mover_requested_speed': self.r32(self.mover + 0x24),
                'mover_speed_factor': self.r32(self.mover + 0x50), 'events': self.events}


def node(x, y, dx, dy, kind):
    return [f2b(x), f2b(y), f2b(dx), f2b(dy), kind, 0, f2b(x), f2b(y)]


def unit(a):
    return math.cos(a), math.sin(a)


def random_path(rng, start, count, kinds=(1,), stop_kind2=False):
    nodes = []
    x, y = start
    for i in range(count):
        a = rng.uniform(-math.pi, math.pi)
        length = rng.uniform(0.5, 9)
        nx, ny = x + math.cos(a) * length, y + math.sin(a) * length
        dx, dy = (nx - x) / length, (ny - y) / length
        nodes.append(node(nx, ny, dx, dy, rng.choice(kinds)))
        x, y = nx, ny
    if stop_kind2 and nodes:
        nodes[rng.randrange(len(nodes))][4] = 2
    return {'nodes': nodes, 'kind1_count': sum(1 for n in nodes if n[4] == 1),
            'kind2_count': sum(1 for n in nodes if n[4] == 2)}


def base(rng):
    org_pos = [rng.uniform(20, 80), rng.uniform(20, 80)]
    a = rng.uniform(-math.pi, math.pi)
    slots = [[f2b(rng.uniform(-8, 8)), f2b(rng.uniform(-8, 8))] for _ in range(6)]
    return {
        'name': 'base', 'precision': 'Double53', 'catchup_factor': f2b(1.75),
        'state': {'mode': 3, 'requested_speed': f2b(rng.uniform(0.5, 4.0)),
                  'path': random_path(rng, org_pos, 3)},
        'actor': {'position': [f2b(org_pos[0] + rng.uniform(-6, 6)), f2b(org_pos[1] + rng.uniform(-6, 6))],
                  'mover_requested_speed': f2b(2.0), 'mover_maximum_speed': f2b(rng.uniform(2.0, 6.0)),
                  'mover_speed_factor': f2b(1.0)},
        'element': {'slot': rng.randrange(6), 'organization': {
            'position': [f2b(org_pos[0]), f2b(org_pos[1])], 'facing': [f2b(math.cos(a)), f2b(math.sin(a))],
            'slot_positions': slots, 'path': random_path(rng, org_pos, 3),
            'leader': {'position': [f2b(org_pos[0] + 1), f2b(org_pos[1] - 1)],
                       'path': random_path(rng, org_pos, 3)}}},
        'responses': [],
    }


def cases():
    rng = random.Random(0x52ff00)
    rows = []
    def add(name, c):
        c['name'] = name
        rows.append(c)
    c = base(rng); c['state']['mode'] = 0
    add('mode_0', c)
    c = base(rng); c['state']['mode'] = 2
    add('mode_2', c)
    c = base(rng); c['element'] = None
    add('no_element', c)
    c = base(rng); c['element']['organization'] = None
    add('detached_element', c)
    c = base(rng); c['element']['organization'] = None; c['actor']['mover_maximum_speed'] = f2b(1.0 / 3.0)
    c['state']['requested_speed'] = f2b(5.0)
    add('detached_third', c)
    # Geometric (equal kind1 counts) and path-count lag branches; then speed gate.
    for i in range(160):
        c = base(rng)
        c['precision'] = 'Double53'
        st = c['state']
        org = c['element']['organization']
        kinds = rng.choice([(1,), (1, 0), (0,), (1, 2)])
        st['path'] = random_path(rng, [struct.unpack('<f', struct.pack('<I', c['actor']['position'][k]))[0]
                                        for k in range(2)], rng.randint(1, 4), kinds=kinds, stop_kind2=rng.random() < 0.2)
        mode = rng.random()
        if mode < 0.45:
            org['path']['kind1_count'] = st['path']['kind1_count']
        if rng.random() < 0.25 and len(st['path']['nodes']) > 1:
            # Collinear first two nodes exercise 0x46B000 equality.
            st['path']['nodes'][1][2:4] = st['path']['nodes'][0][2:4]
        if rng.random() < 0.3:
            # Near the first node: the lateral corner term applies (< 36).
            n0 = st['path']['nodes'][0]
            c['actor']['position'] = [f2b(struct.unpack('<f', struct.pack('<I', n0[0]))[0] + rng.uniform(-3, 3)),
                                      f2b(struct.unpack('<f', struct.pack('<I', n0[1]))[0] + rng.uniform(-3, 3))]
        if rng.random() < 0.2:
            c['state']['requested_speed'] = f2b(rng.uniform(0.0, 0.4))
        c['catchup_factor'] = f2b(rng.choice([1.75, 1.0, 0.1, 2.5]))
        c['responses'] = [{'name': 'LeaderMoving', 'value': rng.random() < 0.5}]
        add(f'generated_{i}', c)
    c = base(rng); c['state']['path'] = {'nodes': [], 'kind1_count': 0, 'kind2_count': 0}
    add('empty_path_null_read', c)
    return rows


def main():
    out = []
    for t in cases():
        for precision in PRECISION:
            c = copy.deepcopy(t)
            c['precision'] = precision
            m = FollowMachine(c)
            try:
                c['output'] = m.run()
                # Drop the unused optional response so the replay is exact.
            except NullDereference as error:
                c['output'] = {'null_dereference': str(error), 'events': m.events}
            except AssertionError:
                if m.responses and m.responses[0]['name'] == 'LeaderMoving':
                    m2 = FollowMachine({**c, 'responses': []})
                    c['responses'] = []
                    c['output'] = m2.run()
                else:
                    raise
            out.append(c)
    write_document('native-follow.json', __doc__, out)


if __name__ == '__main__':
    main()
