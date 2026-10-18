"""Original regiment Mind member-added 524B80 (with 4D0900) executing
unchanged under Unicorn. Seams: the member Mind's virtuals +3C and +40,
522890, operator new; the GTID registry [B364C0], [[B36550]+E4] and the
owner Mind words are memory.

Cases: random owner Minds (+24 word, +28 GTID live / stale / zero, arrival
list +2BC/+2C0 empty or holding nodes), members and game modes (3 or not).
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, header, instructions, sha


class MemberMachine(Machine):
    def __init__(self, c):
        super().__init__('Double53')
        self.c = c
        self.seam(0x6f7810, lambda m: self.ret(self.alloc(max(self.arg(1), 4))))
        table = self.alloc(0x40000)
        self.w(0xb364c0, table)
        self.objects = {}
        for name, (index, serial, live) in c['gtids'].items():
            obj = self.alloc(0x20)
            self.objects[obj] = name
            self.u.mem_write(table + 4 + index * 2, (serial & 0xffff).to_bytes(2, 'little'))
            if live:
                self.w(table + 0x20004 + index * 4, obj)
        gm = self.alloc(0x100); self.w(0xb36550, gm); self.w(gm + 0xe4, c['game_mode'])
        self.mind = self.alloc(0x300)
        self.w(self.mind + 0x24, c['word24']); self.w(self.mind + 0x28, c['home'])
        prev = 0
        for g in reversed(c['list']):
            node = self.alloc(8); self.w(node, g); self.w(node + 4, prev)
            if prev == 0:
                self.w(self.mind + 0x2c0, node)
            prev = node
        self.w(self.mind + 0x2bc, prev)
        vt = self.alloc(0x80)
        self.w(vt + 0x3c, self.stub(self.s_3c)); self.w(vt + 0x40, self.stub(self.s_40))
        self.member_mind = self.alloc(0x40); self.w(self.member_mind, vt)
        self.member = self.alloc(0x100)
        self.w(self.member + 0x14, c['member_gtid']); self.w(self.member + 0x70, self.member_mind)
        self.seam(0x522890, self.s_522890)

    def s_3c(self, m):
        assert self.ecx() == self.member_mind
        self.events.append({'op': 'mind_3c', 'value': self.arg(1)}); self.ret(0, 4)

    def s_40(self, m):
        assert self.ecx() == self.member_mind
        p = self.arg(1)
        self.events.append({'op': 'mind_40', 'object': self.objects.get(p) if p else None}); self.ret(0, 4)

    def s_522890(self, m):
        assert self.ecx() == self.mind and self.arg(1) == self.member
        self.events.append({'op': '522890', 'flag': self.arg(2)}); self.ret(0, 8)

    def run(self):
        self.call(0x524b80, ecx=self.mind, args=[self.member], pop=4)
        out, p, last = [], self.r(self.mind + 0x2bc), 0
        while p:
            out.append(self.r(p)); last = p; p = self.r(p + 4)
        assert self.r(self.mind + 0x2c0) == last
        return {'events': self.events, 'list': out}


def main():
    rnd = random.Random(0x524b80)
    out, visited = [], {}
    for i in range(200):
        gtids = {}
        for k in range(rnd.randrange(1, 4)):
            gtids['obj%d' % k] = [rnd.randrange(1, 300), rnd.randrange(1, 0x7fff), rnd.random() < 0.8]
        names = sorted(gtids)
        def gtid_of(name, stale=False):
            index, serial, live = gtids[name]
            return ((serial + (1 if stale else 0)) & 0xffff) << 16 | index
        home = rnd.choice([0] + [gtid_of(n) for n in names] + [gtid_of(names[0], True)])
        c = {'gtids': gtids, 'game_mode': rnd.choice([0, 1, 2, 3, 3, 4]), 'word24': rnd.getrandbits(32),
             'home': home, 'list': [rnd.getrandbits(32) for _ in range(rnd.choice([0, 0, 1, 3]))],
             'member_gtid': rnd.getrandbits(32)}
        m = MemberMachine(c)
        r = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': r})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-member-added.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
