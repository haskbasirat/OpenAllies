"""Original KKC_SAICommandData::Send 7AF220 executing unchanged under
Unicorn, with 530F10, 4D9E20, 4D9B10, 60C8A0, 61E4B0 and 4D03C0 executed
natively.

Seams (recorded in call order): the order-request emitters 60DE10
(request 51 KKC_FactionUseAbilityOrderRequest), 7AD6D0 (331
SelectionSetOrderRequest), 436AD0 (12 SelectionAddOrderRequest) and the
TellSelectedCommand order object's virtual +4 61EEF0 (-> 61E960, request
84), the 404ED0 assertion log, the command destructor 4D9CC0 (vtable A28450
slot 0), the ability object's destructor, allocator 6F7810/6F7430.

Memory: the command data {+4 list object (+8 GTID chain), +8 player (+C
AI byte), +10 command (+4 refs, +8 definition from [[B36530]+298], +C
flags, +10 target GTID, +30 byte), +14 ability order (+4 sub-object, +8
refs)}, the GTID registry, actors (+8 flags, +14 GTID, +154 SAI record
with +20 pending byte), the command registry [[B36530]+298] (null slots
included, ordinal 0x25 defined or not).
Outputs: events (requests with the command fields), the data's +10/+14
after the call, every record's +20 byte.
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from sai_world import World, f32, header, sha, PRECISIONS
from unicorn import x86_const as X

ORDINALS = 0x30


class SendWorld(World):
    def __init__(self, c):
        super().__init__(c)
        objs = {int(k): v for k, v in c['objects'].items()}
        for ident, o in objs.items():
            self.place(ident, {'actor': 0x200, 'record': 0x40, 'player': 0x40, 'ability': 0x40}[o['kind']])
        for ident, o in objs.items():
            p = self.a(ident)
            if o['kind'] == 'actor':
                self.w(p + 8, o['flags']); self.w(p + 0x14, o['gtid']); self.w(p + 0x154, self.a(o['record']))
            elif o['kind'] == 'record':
                self.w(p + 4, 1000); self.wb(p + 0x20, o['pending'])
            elif o['kind'] == 'player':
                self.wb(p + 0xc, o['ai'])
            elif o['kind'] == 'ability':
                sub_vt = self.vtable({0: self.ability_dtor})
                self.w(p + 4, sub_vt); self.w(p + 8, 100)
        self.registry(c['registry'])
        # Command registry [[B36530]+298].
        root = self.alloc(0x400); self.w(0xb36530, root)
        table = self.alloc(4 * ORDINALS); self.w(root + 0x298, table)
        self.defs = {}
        for k in c['defined']:
            d = self.alloc(0x20); self.w(table + 4 * k, d); self.defs[d] = k
        # Command data.
        self.data = self.alloc(0x20)
        lst = self.alloc(0x10); self.w(lst + 4, 100)
        head = 0
        for g in reversed(c['actors']):
            node = self.alloc(8); self.w(node, g); self.w(node + 4, head); head = node
        self.w(lst + 8, head)
        self.w(self.data + 4, lst); self.w(self.data + 8, self.a(c['player']))
        self.commands = {}
        if c['command'] is not None:
            cm = c['command']
            p = self.alloc(0x40); self.w(p, 0xa28450); self.w(p + 4, 100)
            self.w(p + 8, self.def_of(cm['ordinal'])); self.w(p + 0xc, cm['flags']); self.w(p + 0x10, cm['target'])
            self.wb(p + 0x30, cm['byte30'])
            self.commands[p] = cm['handle']
            self.w(self.data + 0x10, p)
        self.w(self.data + 0x14, self.a(c['ability']))
        for addr, fn in [(0x60de10, self.s_ability), (0x7ad6d0, self.s_set), (0x436ad0, self.s_add),
                         (0x61eef0, self.s_tell), (0x404ed0, self.s_log), (0x4d9cc0, self.s_cmd_dtor)]:
            self.seam(addr, fn)

    def def_of(self, ordinal):
        for d, k in self.defs.items():
            if k == ordinal:
                return d
        return 0

    def ability_dtor(self, m):
        self.ret(0, 4)

    def s_ability(self, m):
        self.ev('request', descriptor=51, player=self.id_of(self.arg(1)), ability=self.id_of(self.ecx()))
        self.ret(0, 4)

    def s_set(self, m):
        assert self.r(self.ecx()) == 0xa70e3c
        self.ev('request', descriptor=331, player=self.id_of(self.arg(1)), actor=self.id_of(self.r(self.ecx() + 4)))
        self.ret(0, 4)

    def s_add(self, m):
        assert self.r(self.ecx()) == 0xa70e3c
        self.ev('request', descriptor=12, player=self.id_of(self.arg(1)), actor=self.id_of(self.r(self.ecx() + 4)))
        self.ret(0, 4)

    def s_tell(self, m):
        order = self.ecx()
        assert self.r(order) == 0xa574e4
        cmd = self.r(order + 4)
        d = self.r(cmd + 8)
        self.ev('request', descriptor=84, player=self.id_of(self.arg(1)),
                command={'handle': self.commands.get(cmd), 'definition': self.defs.get(d) if d else None,
                         'flags': self.r(cmd + 0xc), 'target': self.r(cmd + 0x10), 'byte30': self.rb(cmd + 0x30)})
        self.ret(0, 4)

    def s_log(self, m):
        self.ev('log', line=self.arg(3)); self.ret(0)

    def s_cmd_dtor(self, m):
        self.ret(0, 4)

    def run(self):
        out = {}
        try:
            self.call(0x7af220, ecx=self.data, args=[], count=1_000_000)
            out['fault'] = False
        except AssertionError as e:
            out['fault'] = True; out['fault_detail'] = str(e)[:200]; self.null_accesses.clear()
        except Exception as e:
            out['fault'] = True; out['fault_detail'] = '%s at %x' % (e, self.u.reg_read(X.UC_X86_REG_EIP))
        out['events'] = self.events
        out['command_kept'] = self.r(self.data + 0x10) != 0
        out['ability_kept'] = self.r(self.data + 0x14) != 0
        out['pending'] = {k: self.rb(self.a(int(k)) + 0x20) for k, o in self.c['objects'].items() if o['kind'] == 'record'}
        return out


def make_case(rnd, seed):
    ids = iter(range(1, 10000))
    objects, registry, used = {}, {}, set()

    def gtid_for(obj, live=True):
        while True:
            idx = rnd.randrange(1, 0x400)
            if idx not in used:
                break
        used.add(idx)
        serial = rnd.randrange(1, 0x8000)
        registry[idx] = [serial, obj if live else 0]
        return (serial << 16) | idx

    actors = []
    for i in range(rnd.randrange(0, 7)):
        a = next(ids); actors.append(a)
        rec = next(ids) if rnd.random() < 0.8 else 0
        objects[a] = {'kind': 'actor', 'flags': rnd.choice([0, 0, 0, 1, 0x20, 0x21, 0x10]), 'gtid': 0, 'record': rec}
        objects[a]['gtid'] = gtid_for(a, rnd.random() < 0.85)
        if rec:
            objects[rec] = {'kind': 'record', 'pending': rnd.choice([0, 1, 1])}
    player = next(ids)
    objects[player] = {'kind': 'player', 'ai': rnd.choice([1, 1, 1, 1, 0])}
    ability = 0
    if rnd.random() < 0.1:
        ability = next(ids); objects[ability] = {'kind': 'ability'}
    defined = sorted(rnd.sample(range(ORDINALS), rnd.randrange(ORDINALS // 2, ORDINALS)))
    command = None
    if rnd.random() < 0.92:
        ordinal = rnd.randrange(ORDINALS)
        flags = rnd.choice([0, 1, 1, 0x200, 0x201]) | (0x40 if ordinal in defined else 0)
        target = rnd.choice([objects[a]['gtid'] for a in actors] + [gtid_for(0, False)]) if actors else gtid_for(0, False)
        command = {'handle': next(ids), 'ordinal': ordinal, 'flags': flags, 'target': target, 'byte30': rnd.randrange(3)}
    lst = [objects[a]['gtid'] for a in actors if rnd.random() < 0.9]
    rnd.shuffle(lst)
    return {'seed': seed, 'precision': 'Double53', 'objects': {str(k): v for k, v in objects.items()},
            'registry': {str(k): v for k, v in registry.items()}, 'defined': defined, 'actors': lst,
            'player': player if rnd.random() < 0.97 else 0, 'command': command, 'ability': ability}


def main():
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 600
    rnd = random.Random(0x7af220)
    out, visited, faults = [], {}, 0
    for i in range(n):
        c = make_case(rnd, rnd.getrandbits(32))
        m = SendWorld(c)
        r = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': r}); faults += int(r['fault'])
    d = header(__file__, __doc__)
    d.update(cases=out)
    p = HERE / 'native-send.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    from fn_extent import extent
    body = [x.address for x in extent(0x7af220)]
    print(json.dumps({'cases': len(out), 'faults': faults,
                      'coverage_7AF220': '%d/%d' % (len([a for a in body if a in visited]), len(body)),
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
