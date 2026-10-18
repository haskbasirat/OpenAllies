"""Original DataRef pending pass 475230 over SAI ego references executing
unchanged under Unicorn: each queued reference's virtual (A17038 4B0480 ->
4AEA00, A1703C 4B04E0 -> 4AEA90, A17040 4B0510 -> 4AEBD0, A17044 4B0520 ->
4AEC30, A17048 4AEC90 (+4A7300), A1704C 4B0530 -> 4AED00 (+4A72A0)) with
45F240, 7BA3C0 and 606750 executed natively.

Seams (recorded): 4AE200 Statistic find, 48D500 on the CompanyTemplate
(+570) / SettlementTemplate (+56C) managers, 463FA0 Property, 427BD0 Thing
with the Thing virtual +38, 410C40 data error (throws: the run stops),
9454B0 reference destructor, allocator.

Memory: [B36530] root (+C, +464, +56C, +570, +574), the +824 pending list
(head = last queued), CStringW ids (length at -0xC), egos (+8 IDS, +1C
filter list, +79C recruit list, +830 tech hash, +97C / +984 template lists,
combined-arms lists at owner+4).
Outputs: events, every container in native order, the tech hash buckets.
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from sai_world import World, f32, header, sha
from native_machine import original
from unicorn import x86_const as X

ROOT = 0xb36530
FORMATS = {}


def wide(va):
    out = []
    data = original.range(va, 400)
    for i in range(0, 400, 2):
        ch = data[i] | (data[i + 1] << 8)
        if ch == 0:
            break
        out.append(chr(ch))
    return ''.join(out)


class RefWorld(World):
    def __init__(self, c):
        super().__init__(c)
        self.errored = False
        self.thing_objs = {}
        root = self.alloc(0x900); self.w(ROOT, root); self.root = root
        self.mgr = {}
        for off, name in ((0xc, 'thing'), (0x464, 'property'), (0x56c, 'settlement'), (0x570, 'company'), (0x574, 'statistic')):
            m = self.alloc(0x40); self.w(root + off, m); self.mgr[m] = name
        self.strings = {}
        self.egos = []
        for e in c['egos']:
            p = self.alloc(0xa00)
            self.w(p + 8, self.cstring(e['ids']))
            buckets = self.alloc(4 * 7); self.w(p + 0x830, buckets); self.w(p + 0x834, 7); self.wb(p + 0x83c, 1)
            self.egos.append(p)
        head = 0
        for q in c['queue']:  # creation order: each pushed at the head
            r = self.alloc(0x30)
            self.w(r, q['vtable']); self.w(r + 4, self.cstring(q['ids'])); self.w(r + 8, head)
            self.w(r + 0xc, self.egos[q['ego']] + q['offset'])
            for i, v in enumerate(q['words']):
                self.w(r + 0x10 + 4 * i, v)
            if q['vtable'] == 0xa1704c:
                self.wb(r + 0x24, q['flag'])
            head = r
        self.w(root + 0x824, head)
        thing_vt = self.vtable({0x38: self.s_thing_v38})
        self.thing_vt = thing_vt
        for addr, fn in [(0x4ae200, self.s_statistic), (0x48d500, self.s_find), (0x463fa0, self.s_property),
                         (0x427bd0, self.s_thing), (0x410c40, self.s_error), (0x9454b0, lambda m: self.ret(0))]:
            self.seam(addr, fn)

    def cstring(self, s):
        data = s.encode('utf-16-le') + b'\0\0'
        p = self.alloc(0x10 + len(data)) + 0x10
        self.w(p - 0x10, 0); self.w(p - 0xc, len(s)); self.w(p - 8, len(s)); self.w(p - 4, 1)
        self.u.mem_write(p, data)
        self.strings[p] = s
        return p

    def text(self, p):
        out = bytearray()
        while True:
            ch = bytes(self.u.mem_read(p, 2))
            if ch == b'\0\0':
                return out.decode('utf-16-le')
            out += ch; p += 2

    def s_statistic(self, m):
        assert self.mgr[self.ecx()] == 'statistic'
        ids = self.text(self.arg(1)); r = self.c['statistic'].get(ids.lower())
        self.ev('statistic', ids=ids, result=r)
        if r is None:
            self.ret(0, 4)
        else:
            o = self.alloc(0x20); self.w(o + 0xc, r); self.ret(o, 4)

    def s_find(self, m):
        kind = self.mgr[self.ecx()]
        assert kind in ('company', 'settlement')
        ids = self.text(self.arg(1)); r = self.c[kind].get(ids.lower())
        self.ev(kind, ids=ids, result=r)
        if r is None:
            self.ret(0, 4)
        else:
            o = self.alloc(0x20); self.w(o + 0xc, r); self.ret(o, 4)

    def s_property(self, m):
        assert self.mgr[self.ecx()] == 'property'
        ids = self.text(self.arg(1)); r = self.c['property'].get(ids.lower())
        self.ev('property', ids=ids, result=r); self.ret(r or 0, 4)

    def s_thing(self, m):
        assert self.mgr[self.ecx()] == 'thing'
        ids = self.text(self.arg(1)); t = self.c['thing'].get(ids.lower())
        self.ev('thing', ids=ids, result=t[0] if t else None)
        if t is None:
            self.ret(0, 4)
        else:
            o = self.alloc(0x20); self.w(o, self.thing_vt); self.w(o + 4, t[0]); self.w(o + 8, int(t[1]))
            self.thing_objs[o] = t[0]
            self.ret(o, 4)

    def s_thing_v38(self, m):
        r = self.r(self.ecx() + 8); self.ev('thing_actor', result=bool(r)); self.ret(r)

    def s_error(self, m):
        fmt = self.arg(1)
        text = FORMATS.setdefault(fmt, wide(fmt))
        n = text.count('%s')
        args = [self.strings.get(self.arg(2 + i), self.text(self.arg(2 + i))) for i in range(n)]
        self.ev('data_error', format=text, args=args)
        self.errored = True
        self.u.emu_stop()

    def run(self):
        out = {}
        try:
            self.call(0x475230, ecx=self.root + 0x824, args=[], count=2_000_000)
            out['outcome'] = 'ok'
        except AssertionError as e:
            if self.errored:
                out['outcome'] = 'data_error'
            else:
                out['outcome'] = 'fault'; out['fault_detail'] = str(e)[:200]
            self.null_accesses.clear()
        out['events'] = self.events
        egos = []
        for qi, p in enumerate(self.egos):
            e = {'filters': [], 'company': self.int_list(p + 0x984), 'settlement': self.int_list(p + 0x97c),
                 'recruits': [], 'combined': {}, 'tech': []}
            node = self.r(p + 0x20)
            while node:
                e['filters'].append([self.r(node) - (1 << 32) if self.r(node) >> 31 else self.r(node),
                                     self.r(node + 4), self.r(node + 8)])
                node = self.r(node + 0xc)
            node = self.r(p + 0x79c)
            while node:
                rec = [self.r(node + 4 * i) for i in range(8)]
                rec[0] = self.thing_objs.get(rec[0], rec[0])  # the stored Thing object -> its handle
                e['recruits'].append(rec); node = self.r(node + 0x20)
            for off in self.c['owners']:
                lst, node = [], self.r(p + off + 4)
                while node:
                    lst.append([self.r(node), self.r(node + 4), self.r(node + 8)]); node = self.r(node + 0xc)
                e['combined'][str(off)] = lst
            n = self.r(p + 0x834); buckets = self.r(p + 0x830)
            for b in range(n):
                chain, node = [], self.r(buckets + 4 * b)
                while node:
                    chain.append([self.r(node), self.r(node + 4)]); node = self.r(node + 8)
                e['tech'].append(chain)
            egos.append(e)
        out['egos'] = egos
        return out

    def int_list(self, header):
        out, node = [], self.r(header)
        while node:
            v = self.r(node); out.append(v - (1 << 32) if v >> 31 else v); node = self.r(node + 4)
        return out


def rf(rnd, lo, hi):
    return f32(rnd.uniform(lo, hi))


def make_case(rnd, seed):
    names = ['alpha', 'beta', 'gamma', 'delta', 'eps', 'Zeta', 'eta', 'theta', 'iota', 'kappa']
    pick = lambda d, p: {k.lower(): v for k, v in d.items() if rnd.random() < p}
    statistic = pick({n: i for i, n in enumerate(names)}, 0.8)
    company = pick({n: 10 + i for i, n in enumerate(names)}, 0.85)
    settlement = pick({n: 20 + i for i, n in enumerate(names)}, 0.85)
    prop = pick({n: 0x34000000 + 0x40 * i for i, n in enumerate(names)}, 0.85)
    thing = {k: [0x30000000 + 0x40 * i, rnd.random() < 0.85] for i, k in enumerate(n.lower() for n in names) if rnd.random() < 0.85}
    egos = [{'ids': 'ego_%d' % i} for i in range(rnd.randrange(1, 3))]
    owners = [0x400, 0x500]
    queue = []
    for _ in range(rnd.randrange(1, 9)):
        vt = rnd.choice([0xa17038, 0xa1703c, 0xa17040, 0xa17044, 0xa17048, 0xa1704c])
        ids = rnd.choice(names + [''] if rnd.random() < 0.05 else names)
        q = {'vtable': vt, 'ids': ids, 'ego': rnd.randrange(len(egos)), 'offset': 0, 'words': [0] * 7, 'flag': 0}
        if vt == 0xa17038:
            q['offset'] = 0x1c; q['words'][:2] = [rf(rnd, 0, 5), rf(rnd, 5, 50)]
        elif vt == 0xa1703c:
            q['words'][:2] = [0, rf(rnd, -3, 30)]
        elif vt == 0xa17048:
            q['offset'] = rnd.choice(owners); q['words'][:2] = [rf(rnd, 0, 1), rf(rnd, 0, 3)]
        elif vt == 0xa1704c:
            q['words'][:6] = [rnd.randrange(0, 9), rf(rnd, 0, 50), rf(rnd, 0, 9), rf(rnd, 0, 9), rf(rnd, 0, 9), 0]
            q['words'][6] = rf(rnd, 0, 9)  # +28 (word index 6 -> +28)
            q['words'][5] = 0  # +24 holds the flag byte
            q['flag'] = rnd.choice([0, 1])
        queue.append(q)
    return {'seed': seed, 'precision': 'Double53', 'statistic': statistic, 'company': company,
            'settlement': settlement, 'property': prop, 'thing': thing, 'egos': egos, 'owners': owners, 'queue': queue}


def main():
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 600
    rnd = random.Random(0x475230)
    out, visited, outcomes = [], {}, {}
    for i in range(n):
        c = make_case(rnd, rnd.getrandbits(32))
        m = RefWorld(c)
        r = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': r})
        outcomes[r['outcome']] = outcomes.get(r['outcome'], 0) + 1
    d = header(__file__, __doc__)
    d.update(cases=out)
    p = HERE / 'native-data-refs.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    from fn_extent import extent
    cover = {}
    for name, start in [('475230', 0x475230), ('4AEA00', 0x4aea00), ('4AEA90', 0x4aea90), ('4AEBD0', 0x4aebd0),
                        ('4AEC30', 0x4aec30), ('4AEC90', 0x4aec90), ('4AED00', 0x4aed00), ('4B04E0', 0x4b04e0)]:
        body = [x.address for x in extent(start)]
        cover[name] = '%d/%d' % (len([a for a in body if a in visited]), len(body))
    print(json.dumps({'cases': len(out), 'outcomes': outcomes, 'coverage': cover, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
