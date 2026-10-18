"""Original SAI manager lifecycle routines executing unchanged under Unicorn:

  ctor   KKC_SAI 7A0AA0 (with 85D8A0 RNG seeding executed natively, the
         96D18D vector constructor and 79E130); seams 857D60 (critical
         section), 59ABA0 (millisecond timer = the seed), 7A5BF0 (static
         init), allocator.
  rng    85D840 draws after 85D8A0 seeding.
  think  KKC_SAI::Think 7A0080 with _ftol2 96CC4C executed natively; seams
         76AC90 (session mode), the critical section virtuals +4/+8,
         59ABA0 (timer answers), 79DF00/79DF20/79DF40 (fiber deletes),
         79EF60/79F660/79E830 (fiber creates), the IAT entries
         ConvertThreadToFiber [9F0298], SwitchToFiber [9F0290],
         GetLastError [9F0200], the queue pop 79F9F0 (+7AEE50/7ADC60),
         7AF220 send, 944E60, 404EA0 log.
  reset  79FF10 with seams 7B1760 (player reset), DeleteFiber [9F028C],
         79F9B0 (queue clear), 473300 (general-power release).
  fog    the zone hook 7A9A60 with 7B9300, 7B8FF0 and the white-fog timer
         7B8E70 executed natively; seam 4B2E60 (SVar
         ResetWhiteFogTimerPecent).

Memory per case as described in each builder. Three precisions.
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from sai_world import World, f32, header, sha, PRECISIONS, SAI, WORLD, MAP
from unicorn import x86_const as X

SESSION, CHEAT, REVISION, PAUSE = 0xb36550, 0xb39940, 0xb3cc94, 0xb36554


class ManagerWorld(World):
    def run(self):
        out = {}
        try:
            getattr(self, 'run_' + self.c['entry'])(out)
            out['fault'] = False
        except AssertionError as e:
            out['fault'] = True; out['fault_detail'] = str(e)[:200]; self.null_accesses.clear()
        except Exception as e:
            out['fault'] = True; out['fault_detail'] = '%s at %x' % (e, self.u.reg_read(X.UC_X86_REG_EIP))
        out['events'] = self.events
        return out

    # ctor ------------------------------------------------------------------
    def run_ctor(self, out):
        c = self.c
        self.seam(0x857d60, lambda m: self.ret(0))
        self.seam(0x59aba0, lambda m: self.ret(c['ticks']))
        self.seam(0x7a5bf0, lambda m: self.ret(0))
        sai = self.alloc(0xa0)
        for i in range(0, 0xa0, 4):
            self.w(sai + i, 0xcdcdcdcd)
        r = self.call(0x7a0aa0, ecx=sai, args=[])
        assert r == sai
        words = {('%x' % off): self.r(sai + off) for off in (0x24, 0x28, 0x30, 0x34, 0x38, 0x3c, 0x40, 0x44, 0x48,
                                                            0x4c, 0x50, 0x54, 0x58, 0x64, 0x68, 0x6c, 0x70, 0x78, 0x7c,
                                                            0x80, 0x84, 0x88, 0x8c, 0x94, 0x98)}
        bytes_ = {('%x' % off): self.rb(sai + off) for off in (0x20, 0x5c, 0x5d, 0x5e, 0x74, 0x75)}
        out.update(words=words, bytes=bytes_, rng=self.r(self.r(sai + 0x2c)),
                   vector60=self.r(sai + 0x60) != 0, vector90=self.r(sai + 0x90) != 0,
                   singleton=self.r(SAI) == sai and self.r(0xb40d4c) == sai and self.r(0xb40d48) == SAI)

    # rng ---------------------------------------------------------------------
    def run_rng(self, out):
        st = self.alloc(4)
        self.call(0x85d8a0, ecx=st, args=[self.c['ticks']])
        draws = []
        for _ in range(self.c['draws']):
            code = self.alloc(16)
            # push st; call 85D840; add esp,4; ret
            draws.append(self.call(0x85d840, args=[st], pop=0))
        out.update(state=self.r(st), draws=draws)

    # think -------------------------------------------------------------------
    def run_think(self, out):
        c = self.c
        g = c['globals']
        sai = self.alloc(0xa0); self.sai = sai
        # +4 is the critical-section object itself (its vtable at +4).
        self.w(sai + 4, self.vtable({4: self.s_lock, 8: self.s_unlock}))
        s = c['sai']
        self.wb(sai + 0x20, s['ready']); self.w(sai + 0x24, s['revision'])
        mg = 0
        if s['metagame']:
            mg = self.alloc(0x10); self.w(mg, self.vtable({0: lambda m: self.ret(0, 4)}))
        self.w(sai + 0x28, mg)
        for off, key in ((0x30, 'goal_fiber'), (0x34, 'rai_fiber'), (0x38, 'metagame_fiber'), (0x3c, 'main_fiber')):
            self.w(sai + off, 0x1000 + off if s[key] else 0)
        self.w(sai + 0x40, s['deadlines'][0]); self.w(sai + 0x44, s['deadlines'][1]); self.w(sai + 0x48, s['deadlines'][2])
        self.w(sai + 0x64, s['players'])
        self.queue = list(s['queue'])
        self.queue_nodes()
        sess = self.alloc(0x100); self.w(SESSION, sess); self.w(sess + 0xe4, g['session_e4'])
        cheat = self.alloc(0x300); flag = self.alloc(4); self.wb(flag, g['mode7_enabled']); self.w(cheat + 0x284, flag)
        self.w(CHEAT, cheat)
        if g['world'] is not None:
            world = self.alloc(0x300); self.w(world + 0xdc, g['world'][0]); self.wb(world + 0x275, g['world'][1])
            self.w(WORLD, world)
        if g['map'] is not None:
            mp = self.alloc(0x40); self.wb(mp + 4, g['map'][0]); self.wb(mp + 5, g['map'][1]); self.w(MAP, mp)
        self.w(REVISION, g['revision'])
        pz = self.alloc(0x40); self.wb(pz + 0x29, g['paused']); self.w(PAUSE, pz)
        self.modes = list(c['modes']); self.clock = list(c['clock'])
        for slot, fn in ((0x9f0298, self.s_convert), (0x9f0290, self.s_switch), (0x9f0200, lambda m: self.ret(5)),
                         (0x9f028c, lambda m: self.ret(1, 4))):
            self.w(slot, self.stub(fn))
        for addr, fn in [(0x76ac90, self.s_mode), (0x59aba0, self.s_now),
                         (0x79df00, lambda m: self.s_delete(0x30, 'goal_engine')),
                         (0x79df20, lambda m: self.s_delete(0x34, 'rai')),
                         (0x79df40, lambda m: self.s_delete(0x38, 'metagame')),
                         (0x79f660, lambda m: self.s_create(0x30, 'goal_engine')),
                         (0x79ef60, lambda m: self.s_create(0x34, 'rai')),
                         (0x79e830, lambda m: self.s_create(0x38, 'metagame')),
                         (0x79f9f0, self.s_pop), (0x7aee50, lambda m: self.ret(self.ecx())),
                         (0x7af220, self.s_send), (0x7adc60, lambda m: self.ret(0)),
                         (0x944e60, self.s_mg_post), (0x404ea0, self.s_log)]:
            self.seam(addr, fn)
        self.call(0x7a0080, ecx=sai, args=[c['budget']], count=1_000_000)
        out['sai'] = {'revision': self.r(sai + 0x24), 'metagame': self.r(sai + 0x28) != 0,
                      'fibers': [self.r(sai + off) != 0 for off in (0x30, 0x34, 0x38, 0x3c)],
                      'deadlines': [self.r(sai + 0x40), self.r(sai + 0x44), self.r(sai + 0x48)],
                      'queue': self.queue}

    def queue_nodes(self):
        sai = self.sai
        if self.queue:
            node = self.alloc(0x20); self.w(node + 0xc, self.queue[0])
            self.w(sai + 0x78, 1); self.w(sai + 0x7c, node)
        else:
            self.w(sai + 0x78, 0); self.w(sai + 0x7c, 0)

    def s_lock(self, m):
        self.ev('lock'); self.ret(0)

    def s_unlock(self, m):
        self.ev('unlock'); self.ret(0)

    def s_mode(self, m):
        r = self.modes.pop(0); self.ev('session_mode', result=r); self.ret(r)

    def s_now(self, m):
        r = self.clock.pop(0); self.ev('now', result=r); self.ret(r)

    def s_delete(self, off, name):
        assert self.ecx() == self.sai
        self.ev('delete_fiber', fiber=name)
        self.w(self.sai + off, 0)
        if off == 0x38:
            self.w(self.sai + 0x28, 0)
        self.ret(0)

    def s_create(self, off, name):
        assert self.ecx() == self.sai
        self.ev('create_fiber', fiber=name); self.w(self.sai + off, 0x2000 + off); self.ret(0)

    def s_convert(self, m):
        r = self.c['convert_ok']; self.ev('convert_thread', result=r); self.ret(0x3000 if r else 0, 4)

    def s_switch(self, m):
        h = self.arg(1)
        name = {0x30: 'goal_engine', 0x34: 'rai', 0x38: 'metagame'}[[off for off in (0x30, 0x34, 0x38)
                                                                    if self.r(self.sai + off) == h][0]]
        dl = {'goal_engine': 0x40, 'rai': 0x44, 'metagame': 0x48}[name]
        self.ev('run', fiber=name, deadline=self.r(self.sai + dl)); self.ret(0, 4)

    def s_pop(self, m):
        assert self.ecx() == self.sai + 0x78
        due = self.queue.pop(0)
        self.queue_nodes()
        self.ret(1, 4)

    def s_send(self, m):
        self.ev('send'); self.ret(0)

    def s_mg_post(self, m):
        self.ev('metagame_post'); self.ret(0)

    def s_log(self, m):
        self.ev('log'); self.ret(0)

    # reset -------------------------------------------------------------------
    def run_reset(self, out):
        c = self.c
        sai = self.alloc(0xa0); self.sai = sai
        players = c['players']
        arr = self.alloc(4 * max(len(players), 1))
        for i, p in enumerate(players):
            self.w(arr + 4 * i, 0x50000 + p if p else 0)
        self.w(sai + 0x60, arr); self.w(sai + 0x64, len(players))
        self.wb(sai + 0x20, 1)
        self.w(sai + 0x30, 0x1030 if c['goal_fiber'] else 0); self.w(sai + 0x34, 0x1034 if c['rai_fiber'] else 0)
        self.w(sai + 0x38, 0x1038)
        head = 0
        for g in reversed(c['general_powers']):
            node = self.alloc(8); self.w(node, g); self.w(node + 4, head); head = node
        self.w(sai + 0x80, head); self.w(sai + 0x84, 0)
        self.w(0x9f028c, self.stub(lambda m: (self.ev('delete_fiber', handle=self.arg(1)), self.ret(1, 4))))
        self.seam(0x7b1760, lambda m: (self.ev('reset_player', player=self.ecx() - 0x50000), self.ret(0)))
        self.seam(0x79f9b0, lambda m: (self.ev('clear_queue'), self.ret(0)))
        self.seam(0x473300, lambda m: (self.ev('release', gp=self.r(self.ecx())), self.ret(0)))
        self.call(0x79ff10, ecx=sai, args=[])
        out['after'] = {'ready': self.rb(sai + 0x20), 'goal_fiber': self.r(sai + 0x30), 'rai_fiber': self.r(sai + 0x34),
                        'metagame_fiber': self.r(sai + 0x38), 'gp_head': self.r(sai + 0x80)}

    # white fog ---------------------------------------------------------------
    def run_fog(self, out):
        c = self.c
        svar = self.alloc(4); self.w(svar, c['percent'])
        self.seam(0x4b2e60, lambda m: self.ret(svar, 8))
        world = self.alloc(0x300); self.w(WORLD, world); self.w(world + 0xe0, c['time'])
        mp = self.alloc(0x40); self.w(MAP, mp)
        regions = c['regions']
        arr = self.alloc(4 * max(len(regions), 1)); self.w(mp + 0x14, arr); self.w(mp + 0x18, len(regions))
        self.records = {}
        for i, rg in enumerate(regions):
            r = self.alloc(0x80); self.w(arr + 4 * i, r); self.w(r, i); self.w(r + 0xc, rg['area'])
            recs = self.alloc(4 * max(len(rg['records']), 1))
            for k, rec in enumerate(rg['records']):
                if rec is None:
                    continue
                p = self.alloc(0x20); self.w(p, r); self.w(p + 0x10, rec['visible']); self.w(p + 0x14, rec['timer'])
                self.w(recs + 4 * k, p); self.records[(i, k)] = p
            self.w(r + 0x10, recs); self.w(r + 0x14, len(rg['records']))
        tile = self.alloc(0x20)
        if c['tile_region'] is not None:
            rect = self.alloc(0x20); self.w(rect + 4, c['tile_region'] & 0xffffffff); self.w(tile + 0x10, rect)
        kingdom = self.alloc(0x200); self.w(kingdom + 0x18c, c['kingdom'] & 0xffffffff)
        self.call(0x7a9a60, args=[tile, kingdom])
        out['records'] = {'%d:%d' % key: [self.r(p + 0x10), self.r(p + 0x14)] for key, p in self.records.items()}


def rf(rnd, lo, hi):
    return f32(rnd.uniform(lo, hi))


def make_case(rnd, entry, precision, seed):
    c = {'entry': entry, 'precision': precision, 'seed': seed}
    if entry in ('ctor', 'rng'):
        c['ticks'] = rnd.choice([0, 1, 0x8e4b, 0x12a58e4b & 0xffff, rnd.getrandbits(32), rnd.getrandbits(16),
                                 0x8e4b12a5, 0x12a5 | (0x8e4b << 16)])
        c['draws'] = rnd.randrange(1, 9)
    elif entry == 'think':
        c['budget'] = rnd.choice([rf(rnd, 0, 0.2), f32(0.05), f32(0.0), rf(rnd, 0, 3), f32(1e-4), f32(-0.01)])
        c['globals'] = {'mode7_enabled': rnd.random() < 0.5,
                        'world': rnd.choice([None, [1, 0], [5, 0], [0, 0], [3, 1]] + [[1, 0]] * 8),
                        'map': rnd.choice([None, [1, 0], [0, 1]] + [[1, 1]] * 9),
                        'session_e4': rnd.choice([2, 2, 2, 5, 5, 4, 3]), 'revision': rnd.choice([7, 7, 8]),
                        'paused': rnd.random() < 0.1}
        c['sai'] = {'ready': int(rnd.random() < 0.9), 'revision': 7, 'metagame': rnd.random() < 0.5,
                    'goal_fiber': rnd.random() < 0.6, 'rai_fiber': rnd.random() < 0.6,
                    'metagame_fiber': rnd.random() < 0.5, 'main_fiber': rnd.random() < 0.6,
                    'deadlines': [rnd.getrandbits(20)] * 3, 'players': rnd.choice([0, 1, 3]),
                    'queue': sorted(rnd.getrandbits(14) for _ in range(rnd.randrange(0, 5)))}
        if rnd.random() < 0.75:
            c['modes'] = [rnd.choice([7, 4, 14, 4, 14])] * 4
        else:
            c['modes'] = [rnd.choice([7, 4, 14, 2, 7, 4]) for _ in range(4)]
        base = rnd.getrandbits(14)
        c['clock'] = [base + rnd.randrange(0, 300) * k for k in range(20)]
        c['convert_ok'] = rnd.random() < 0.9
    elif entry == 'reset':
        c['players'] = [rnd.choice([0, rnd.randrange(1, 100)]) for _ in range(rnd.randrange(0, 6))]
        c['goal_fiber'] = rnd.random() < 0.6; c['rai_fiber'] = rnd.random() < 0.6
        c['general_powers'] = [rnd.randrange(1, 1000) for _ in range(rnd.randrange(0, 4))]
    elif entry == 'fog':
        regions = []
        for i in range(rnd.randrange(1, 4)):
            recs = [rnd.choice([None, {'visible': rnd.choice([0, 1, 2, 5, 40, 0xffffffff]), 'timer': rf(rnd, 0, 50)}])
                    for _ in range(rnd.randrange(0, 4))]
            regions.append({'area': rnd.choice([0, 1, 4, 16, 64, 100, -3 & 0xffffffff]), 'records': recs})
        c['regions'] = regions
        c['tile_region'] = rnd.choice([None, -1] + list(range(len(regions))) * 3)
        c['kingdom'] = rnd.randrange(0, 4)
        c['percent'] = rnd.choice([rf(rnd, 0, 1), f32(0.5), f32(0.0), f32(1.0), f32(-0.25)])
        c['time'] = rf(rnd, 0, 900)
    return c


def main():
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 300
    rnd = random.Random(0x7a0080)
    out, visited, stats = [], {}, {}
    precisions = list(PRECISIONS)
    for entry in ('ctor', 'rng', 'think', 'reset', 'fog'):
        for i in range(n if entry in ('think', 'fog') else n // 3):
            c = make_case(rnd, entry, precisions[i % 3], rnd.getrandbits(32))
            m = ManagerWorld(c)
            r = m.run()
            m.verify_v14(); visited.update(m.visited)
            out.append({'input': c, 'output': r})
            s = stats.setdefault(entry, {'cases': 0, 'faults': 0}); s['cases'] += 1; s['faults'] += int(r['fault'])
    d = header(__file__, __doc__)
    d.update(cases=out, stats=stats)
    p = HERE / 'native-manager.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    from fn_extent import extent
    cover = {}
    for name, start in [('7A0AA0', 0x7a0aa0), ('85D8A0', 0x85d8a0), ('85D840', 0x85d840), ('7A0080', 0x7a0080),
                        ('79FF10', 0x79ff10), ('7A9A60', 0x7a9a60), ('7B9300', 0x7b9300), ('7B8FF0', 0x7b8ff0),
                        ('7B8E70', 0x7b8e70)]:
        body = [x.address for x in extent(start)]
        cover[name] = '%d/%d' % (len([a for a in body if a in visited]), len(body))
    print(json.dumps({'cases': len(out), 'stats': stats, 'coverage': cover, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
