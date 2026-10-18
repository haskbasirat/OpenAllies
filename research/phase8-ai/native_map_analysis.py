"""Original KKC_SAIMap::DynamicMapAnalysis 7AC0D0 executing unchanged under
Unicorn with its callees 7AA5B0, 7A9C60/7A9C80, 7B9180, 7BB480, 7BAC00,
7AE350 (7AD890, 7ADDA0, 4A5E40, CRT pow 96D890 SSE2 path: [B4B340] = 1,
MXCSR 0x1F80), 7ABA70, 7AB940, 7AA620, 45D720, 7B9050, 7BB780, 7BB820,
4A8700, 45F180, 4849F0, 5C37E0.

Seams (world services): 4D0180 combat value and the actor virtuals +104
(speed) and +108 (owner), 4CEDA0 organization actor, 7B9320 region terrain
(first rectangle), 79DF70 fiber yield, 79EFF0 reschedule, 4B2E60 SVar
lookup, operator new/delete. Memory: the SAI map, regions (+0 index, +4
rectangle node, +1C/+20 player entries, +28/+34 actor lists, +5C neighbour
hash), SAI players (+0 slot, +1C ego, +170 learning table), objects (placed
AT their handle, so the learning table hashes the same words; +4 the
definition whose +8 name the speed-cap log reads), the GTID
registry [B364C0], [B36534]+E0 game time and [B28A88].

Cases: random maps of 2..6 regions with neighbour hashes, land/water,
1..2 SAI players with egos and learning tables (records pruned by age, lists
emptied and erased, scaled or plain sums, pow below 1 and above the
ceiling), static and mobile actor lists with stale GTIDs, deleting objects,
non-positive combat values, organization remaps, settlement ranges from +B0
or the +148 slot, speeds above the cap; analysed or not; three precisions.
Outputs: the event sequence, every region entry's three influence vectors
and every player's learning table.
"""
from pathlib import Path
import json, math, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, PRECISIONS, f32, header, instructions, sha
from unicorn import x86_const as X

OBJECT_BASE = 0x40000000
SVAR_NAMES = {'DynamicMapAnalysisSpeedCap': 'speed_cap',
              'settlement_cv_range_fudge_factor': 'settlement_cv_range_fudge_factor',
              'min_cv_propogation_factor': 'min_cv_propogation_factor',
              'dynamic_map_analysis_think_frequency': 'think_frequency'}
EGO_FIELDS = {'cv_influence_exponent': 0x28, 'cv_influence_max_travel_time': 0x2c, 'border_influence_range': 0x30,
              'learning_factor': 0x93c, 'recruit_duration': 0x940, 'cv_exponent': 0x958, 'cv_ceiling': 0x95c}


def x87_of(v):
    if v == 0:
        return {'significand': 0, 'sign_exponent': 0}
    m, e = math.frexp(abs(v))
    return {'significand': int(m * (1 << 64)) & ((1 << 64) - 1), 'sign_exponent': ((v < 0) << 15) | (e - 1 + 16383)}


class MapMachine(Machine):
    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = c
        u = self.u
        u.reg_write(X.UC_X86_REG_MXCSR, 0x1f80)
        self.w(0xb4b340, 1)
        self.seam(0x6f7810, lambda m: self.ret(self.alloc(max(self.arg(1), 4))))
        self.seam(0x6f7430, lambda m: self.ret(0))
        # SVars.
        self.svar_ptr = {}
        for name, key in SVAR_NAMES.items():
            p = self.alloc(4); self.w(p, c['svars'][key]); self.svar_ptr[name] = p
        self.seam(0x4b2e60, self.s_svar)
        # Globals.
        world = self.alloc(0x400); self.w(0xb36534, world); self.w(world + 0xe0, c['game_time'])
        self.sai = self.alloc(0x100); self.w(0xb36528, self.sai)
        self.wb(0xb28a88, int(c['immediate']))
        self.seam(0x79df70, self.s_yield); self.seam(0x79eff0, self.s_schedule)
        self.seam(0x4d0180, self.s_cv); self.seam(0x4ceda0, self.s_org); self.seam(0x7b9320, self.s_terrain)
        # Objects at their handles and the GTID registry.
        table = self.alloc(0x40000); self.w(0xb364c0, table)
        vt = self.alloc(0x200)
        self.w(vt + 0x104, self.stub(self.s_speed)); self.w(vt + 0x108, self.stub(self.s_owner))
        self.owners = {}
        for k in range(4):
            o = self.alloc(0x200); self.w(o + 0x188, k); self.owners[o] = k
        self.owner_ptr = {k: o for o, k in self.owners.items()}
        self.objects = {}
        for hs, o in c['objects'].items():
            h = int(hs); self.objects[h] = o
            self.map_at(h, 0x200)
            definition = 0
            if o['definition']:
                definition = self.alloc(0x20)
                name = self.alloc(0x20); self.u.mem_write(name, 'unit'.encode('utf-16-le') + bytes(2))
                self.w(definition + 8, name)
            self.w(h, vt); self.w(h + 4, definition); self.w(h + 8, o['flags']); self.w(h + 0x14, o['gtid'])
            self.w(h + 0x98, 1 if o['org'] is not None else 0)
            if o['b0'] is not None:
                comp = self.alloc(0x40); self.w(comp + 0x2c, o['b0']); self.w(h + 0xb0, comp)
            self.w(h + 0x14c, o['slot_count'] & 0xffffffff)
            slots = self.alloc(0x40); self.w(h + 0x148, slots)
            if o['slot5'] is not None:
                s5 = self.alloc(0x20); self.w(s5 + 8, o['slot5']); self.w(slots + 0x14, s5)
            index, serial = o['gtid'] & 0xffff, o['gtid'] >> 16
            u.mem_write(table + 4 + index * 2, serial.to_bytes(2, 'little'))
            if o['live']:
                self.w(table + 0x20004 + index * 4, h)
        # Players and egos.
        self.players = []
        for pl in c['players']:
            p = self.alloc(0x200); self.w(p, pl['slot'])
            ego = self.alloc(0x98c)
            for name, off in EGO_FIELDS.items():
                self.w(ego + off, pl['ego'][name])
            self.wb(ego + 0x960, pl['ego']['cv_scale_over_duration'])
            self.w(p + 0x1c, ego)
            lt = pl['learning']
            buckets = self.alloc(4 * lt['n'])
            for b, chain in enumerate(lt['buckets']):
                head = 0
                for key, records in reversed(chain):
                    node = self.alloc(12); self.w(node, key); self.w(node + 8, head); head = node
                    if records is not None:
                        hdr = self.alloc(8); nxt = 0; last = 0
                        for r in reversed(records):
                            rec = self.alloc(0x10); self.w(rec, r[0]); self.w(rec + 4, r[1]); self.w(rec + 8, r[2])
                            self.w(rec + 0xc, nxt)
                            if nxt == 0:
                                last = rec
                            nxt = rec
                        self.w(hdr, nxt); self.w(hdr + 4, last)
                        self.w(node + 4, hdr)
                self.w(buckets + 4 * b, head)
            self.w(p + 0x170, buckets); self.w(p + 0x174, lt['n']); self.w(p + 0x178, lt['count']); self.wb(p + 0x17c, 1)
            self.players.append(p)
        # Regions.
        self.map = self.alloc(0x40); self.wb(self.map + 4, int(c['analysed']))
        n = len(c['regions'])
        arr = self.alloc(4 * n); self.w(self.map + 0x14, arr); self.w(self.map + 0x18, n)
        self.rects = {}
        self.regions = []
        for i, rg in enumerate(c['regions']):
            r = self.alloc(0x80); self.w(arr + 4 * i, r); self.regions.append(r)
            self.w(r, rg['index'])
            rect = self.alloc(0x20); node = self.alloc(8); self.w(node, rect); self.w(r + 4, node)
            self.rects[rect] = i
            ents = self.alloc(4 * max(len(rg['entries']), 1)); self.w(r + 0x1c, ents); self.w(r + 0x20, len(rg['entries']))
            for s, e in enumerate(rg['entries']):
                if e is None:
                    continue
                ep = self.alloc(0x30); self.w(ep, self.players[e['player']])
                for off in (8, 0x14, 0x20):
                    stale = e['stale']
                    data = self.alloc(4 * max(len(stale), 1))
                    for k, v in enumerate(stale):
                        self.w(data + 4 * k, v)
                    self.w(ep + off, data); self.w(ep + off + 4, len(stale)); self.w(ep + off + 8, len(stale))
                self.w(ents + 4 * s, ep)
            for off, key in ((0x28, 'mobile'), (0x34, 'static')):
                lists = rg[key]
                la = self.alloc(4 * max(len(lists), 1))
                for k, gl in enumerate(lists):
                    lst = self.alloc(4); head = 0
                    for g in reversed(gl):
                        nd = self.alloc(8); self.w(nd, g); self.w(nd + 4, head); head = nd
                    self.w(lst, head); self.w(la + 4 * k, lst)
                self.w(r + off, la); self.w(r + off + 4, len(lists))
            nb = rg['neighbors']
            bk = self.alloc(4 * nb['n'])
            for b, chain in enumerate(nb['buckets']):
                head = 0
                for key, dist in reversed(chain):
                    nd = self.alloc(12); self.w(nd, key); self.w(nd + 4, dist); self.w(nd + 8, head); head = nd
                self.w(bk + 4 * b, head)
            self.w(r + 0x5c, bk); self.w(r + 0x60, nb['n']); self.w(r + 0x64, sum(len(ch) for ch in nb['buckets']))

    def map_at(self, address, size):
        page, end = address & ~0xfff, (address + size + 0xfff) & ~0xfff
        if not hasattr(self, 'mapped'):
            self.mapped = set()
        for q in range(page, end, 0x1000):
            if q not in self.mapped:
                self.u.mem_map(q, 0x1000); self.mapped.add(q)

    def x87_return(self, v, pop=0):
        scratch = self.alloc(16)
        self.u.mem_write(scratch, struct.pack('<QH', v['significand'], v['sign_exponent']))
        stub = self.alloc(16)
        code = b'\xdb\x2d' + struct.pack('<I', scratch) + (b'\xc2' + struct.pack('<H', pop) if pop else b'\xc3')
        self.u.mem_write(stub, code)
        self.u.reg_write(X.UC_X86_REG_EIP, stub)

    def s_svar(self, m):
        assert self.ecx() == 0xb39aa8 and self.arg(2) == 2
        self.ret(self.svar_ptr[self.wstr(self.arg(1))], 8)

    def wstr(self, p):
        out = bytearray()
        while True:
            ch = bytes(self.u.mem_read(p, 2))
            if ch == b'\0\0':
                return out.decode('utf-16-le')
            out += ch; p += 2

    def s_yield(self, m):
        assert self.ecx() == self.sai and self.arg(1) == 0
        self.events.append({'op': 'yield'}); self.ret(0, 4)

    def s_schedule(self, m):
        assert self.ecx() == self.sai and self.arg(1) == 0xffffffff and self.arg(4) == 0
        self.events.append({'op': 'schedule', 'delay': self.arg(2), 'immediate': self.arg(3) & 0xff}); self.ret(0, 0x10)

    def s_cv(self, m):
        h = self.ecx()
        self.events.append({'op': 'cv', 'object': h}); self.x87_return(self.objects[h]['cv'])

    def s_org(self, m):
        h = self.ecx()
        r = self.objects[h]['org']
        self.events.append({'op': 'organization_actor', 'object': h, 'result': r}); self.ret(r)

    def s_terrain(self, m):
        i = self.rects[self.ecx()]
        t = self.c['regions'][i]['terrain']
        self.events.append({'op': 'terrain', 'region': i, 'result': t}); self.ret(t)

    def s_speed(self, m):
        h = self.ecx()
        self.events.append({'op': 'speed', 'object': h}); self.x87_return(self.objects[h]['speed'])

    def s_owner(self, m):
        h = self.ecx()
        k = self.objects[h]['owner']
        self.events.append({'op': 'owner', 'object': h, 'result': k}); self.ret(self.owner_ptr[k])

    def run(self):
        from unicorn import UcError
        fault = None
        try:
            self.call(0x7ac0d0, ecx=self.map, args=[], pop=0, count=20_000_000)
        except (AssertionError, UcError):
            if not self.null_accesses:
                raise
            fault = 'null dereference'
        out = {'events': self.events}
        if fault:
            out['fault'] = fault
            return out
        assert self.rb(0xb28a88) == 0
        regions = []
        for r in self.regions:
            ents = []
            for s in range(self.r(r + 0x20)):
                ep = self.r(self.r(r + 0x1c) + 4 * s)
                if not ep:
                    ents.append(None); continue
                vec = lambda off: [self.r(self.r(ep + off) + 4 * k) for k in range(self.r(ep + off + 4))]
                ents.append({'mobile': vec(8), 'settlement': vec(0x14), 'border': vec(0x20)})
            regions.append(ents)
        players = []
        for p in self.players:
            bk, n = self.r(p + 0x170), self.r(p + 0x174)
            chains = []
            for b in range(n):
                chain, e = [], self.r(bk + 4 * b)
                while e:
                    hdr = self.r(e + 4)
                    if hdr:
                        recs, q, last = [], self.r(hdr), 0
                        while q:
                            recs.append([self.r(q), self.r(q + 4), self.r(q + 8)]); last = q; q = self.r(q + 0xc)
                        assert self.r(hdr + 4) == last
                    else:
                        recs = None
                    chain.append([self.r(e), recs]); e = self.r(e + 8)
                chains.append(chain)
            players.append({'count': self.r(p + 0x178), 'buckets': chains})
        out.update(regions=regions, players=players)
        return out


def hash_chains(keys, n, value):
    """Chains of a native hash built by head insertion in `keys` order."""
    chains = [[] for _ in range(n)]
    for k in keys:
        chains[k % n].insert(0, [k, value(k)])
    return chains


def case(rnd, i):
    n = rnd.randrange(2, 7)
    nplayers = rnd.choice([1, 1, 2])
    tf = rnd.uniform(200, 900)
    t = f32(tf)
    objects = {}
    used = set()
    def new_object(mobile):
        h = OBJECT_BASE + 0x400 * len(objects)
        index = rnd.randrange(1, 2000)
        while index in used:
            index = rnd.randrange(1, 2000)
        used.add(index)
        gtid = (rnd.randrange(1, 0x7fff) << 16) | index
        cvv = rnd.choice([rnd.uniform(0.5, 40), rnd.uniform(0.5, 40), 0.0, -2.0])
        o = {'gtid': gtid, 'live': rnd.random() < 0.9, 'flags': rnd.choice([0, 0, 0, 1, 4]),
             'definition': rnd.random() < 0.97, 'owner': rnd.randrange(4), 'cv': x87_of(cvv),
             'org': None, 'b0': rnd.choice([None, f32(rnd.uniform(5, 60))]),
             'slot_count': rnd.choice([0, 5, 6, 9, -1]), 'slot5': rnd.choice([None, f32(rnd.uniform(5, 60))]),
             'speed': x87_of(rnd.choice([rnd.uniform(1, 30), 15.0, 40.0]))}
        objects[str(h)] = o
        return h
    regions = []
    for r in range(n):
        lists = {}
        for key in ('static', 'mobile'):
            ls = []
            for _ in range(rnd.choice([0, 1, 1, 2])):
                gl = []
                for _ in range(rnd.choice([0, 1, 2, 3])):
                    h = new_object(key == 'mobile')
                    g = objects[str(h)]['gtid']
                    gl.append(g if rnd.random() < 0.9 else (g + 0x10000) & 0xffffffff)
                ls.append(gl)
            lists[key] = ls
        nbn = rnd.choice([3, 7])
        keys = rnd.sample([k for k in range(n) if k != r], rnd.randrange(0, n))
        dist = {k: f32(rnd.uniform(2, 45)) for k in keys}
        entries = []
        for s in range(nplayers):
            if rnd.random() < 0.03:
                entries.append(None)
            else:
                entries.append({'player': s, 'stale': [f32(rnd.uniform(0, 5)) for _ in range(rnd.choice([0, 2]))]})
        regions.append({'index': r, 'terrain': rnd.choice([0, 0, 0, 1]),
                        'neighbors': {'n': nbn, 'buckets': hash_chains(keys, nbn, lambda k: dist[k])},
                        'entries': entries, 'static': lists['static'], 'mobile': lists['mobile']})
    # Organization remaps point at other objects.
    handles = [int(h) for h in objects]
    for h in handles:
        if rnd.random() < 0.15:
            objects[str(h)]['org'] = rnd.choice(handles)
    players = []
    for s in range(nplayers):
        keys = rnd.sample(handles, min(len(handles), rnd.randrange(0, 6)))
        def records(k):
            if rnd.random() < 0.15:
                return None
            return [[f32(rnd.uniform(-1, 3)), f32(tf - rnd.uniform(-20, 300)), rnd.getrandbits(32)]
                    for _ in range(rnd.randrange(1, 4))]
        lt_n = rnd.choice([2, 3, 7])
        players.append({'slot': s, 'ego': {
            'cv_influence_exponent': f32(rnd.uniform(0.5, 2)), 'cv_influence_max_travel_time': f32(rnd.uniform(1, 6)),
            'border_influence_range': f32(rnd.uniform(5, 60)), 'learning_factor': f32(rnd.uniform(0, 1)),
            'recruit_duration': f32(rnd.uniform(50, 250)), 'cv_exponent': f32(rnd.uniform(0.2, 2)),
            'cv_ceiling': f32(rnd.uniform(1, 4)), 'cv_scale_over_duration': rnd.choice([0, 1])},
            'learning': {'n': lt_n, 'count': len(keys), 'buckets': hash_chains(keys, lt_n, records)}})
    return {'precision': list(PRECISIONS)[i % 3], 'analysed': rnd.random() < 0.95, 'immediate': rnd.random() < 0.3,
            'game_time': t, 'svars': {'speed_cap': f32(rnd.choice([15.0, 25.0])),
                                      'settlement_cv_range_fudge_factor': f32(rnd.uniform(0.5, 2)),
                                      'min_cv_propogation_factor': f32(rnd.uniform(0.05, 0.5)),
                                      'think_frequency': f32(rnd.uniform(1, 10))},
            'regions': regions, 'players': players, 'objects': objects}


def main():
    rnd = random.Random(0x7ac0d0)
    out, visited, faults = [], {}, 0
    for i in range(400):
        c = case(rnd, i)
        m = MapMachine(c)
        r = m.run()
        m.verify_v14(); visited.update(m.visited)
        faults += 'fault' in r
        out.append({'input': c, 'output': r})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-map-analysis.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': faults, 'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
