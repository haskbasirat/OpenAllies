"""Original SAI goal-class base priorities (goal vtable slot +78) executing
unchanged under Unicorn, for the classes ported in
crates/aa-mind/src/sai/base_priority.rs:

  ATTACK_REGION 7C5AE0, ATTACK_ACTOR 7C7460, DEFEND_REGION 7C5F80,
  DEFEND_ACTOR 7C62D0, AIR_ATTACK 7C7E90, RECOVER 7C6640, REPAIR 7A4230,
  BOMBARD 7C8750

with their callees executed natively: the general-power predicates 572040,
5720C0, 572120, 572940, 572810, 571C60 (+4A6650), 4CF010, 4CFE90, 4D09D0,
4CF1B0, 7A1120 (+96D710), 7A10F0, 432E40, 7A2FC0, 7A2FD0, 529C30, 7B9050
and the CRT _CIpow 96D890 (SSE2 path, [B4B340] = 1, MXCSR 0x1F80).

Seams (answered from the case RNG and recorded in call order): goal
virtuals +14/+18/+20; game-object virtuals +58/+70/+100/+108/+138/+140;
SAI actor record virtual +C; 7AA3D0 region at point; 53F320 route search
(status, node list) with 5C2CD0 release; 79F390 unit check; 4EA430 seen
record; 4D0230 health; the SAI player valuations 7AD9E0, 7AD340, 7AD3E0;
the region threat 7BD360 (entry, &threat, 0); allocator 6F7810/6F7430.

Memory: GTID registry [B364C0] (live, stale-serial and empty-slot ids),
[B36528]+80 and SAI player +9C general-power lists (instance +8 caster,
+10 side, +14->+8->+C kind, +18 bonus, +1C target, +20 targets actor,
+24 owner, +2C/+30 point, +34 has point, +3D additive), goal +4 -> +4 SAI
player (+0 slot, +8 player object, +1C ego, +9C list), ego fields
(+3F4, +420..+454, +6CC..+6E0, +890), objects (+4 definition +2F4, +8,
+14, +20/+24, +7C component +5C/+60, +80 component +10, +100, +104,
+118), SAI actor records (+8, +18/+1C, +30), regions (+0 index, +1C entry
array, entry +2C -> +8 state), [B3652C]+5, [B36534]+E0/+234.

Cases: random worlds per class at PC24/PC53/PC64; faults (native null
dereferences) are recorded with the events before them.
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from sai_world import World, x87_of, x87_ext, f32, header, instructions, sha, PRECISIONS, SAI, WORLD, MAP

CLASSES = {
    'ATTACK_REGION': 0x7c5ae0, 'ATTACK_ACTOR': 0x7c7460, 'DEFEND_REGION': 0x7c5f80, 'DEFEND_ACTOR': 0x7c62d0,
    'AIR_ATTACK': 0x7c7e90, 'RECOVER': 0x7c6640, 'REPAIR': 0x7a4230, 'BOMBARD': 0x7c8750,
    'PACK': 0x7cb640, 'RECRUIT': 0x4b2830, 'GIVE_RESOURCES': 0x7c10b0, 'GIVE_ACTOR': 0x7c1220,
    'RESEARCH': 0x7d0680, 'ATTACH': 0x7be260,
    'EXPLORE': 0x7c29f0, 'BUILD': 0x7bfa30, 'UPGRADE': 0x7c0870, 'DESTROY': 0x7c0bd0, 'UNPACK': 0x7cbc40,
}
EGO_FLOATS = [0x420, 0x424, 0x428, 0x42c, 0x430, 0x434, 0x438, 0x43c, 0x440, 0x44c, 0x450, 0x454,
              0x6cc, 0x6d0, 0x6d4, 0x6d8, 0x6dc, 0x6e0, 0x890]


class BasePriorityWorld(World):
    def __init__(self, c):
        super().__init__(c)
        objs = {int(k): v for k, v in c['objects'].items()}
        sizes = {'actor': 0x400, 'component': 0x100, 'record': 0x40, 'region': 0x80, 'key': 0x10, 'entry': 0x20}
        for ident, o in objs.items():
            self.place(ident, sizes[o['kind']])
            if o['kind'] == 'actor':
                self.w(self.a(ident) + 0xd8, 0)
        actor_vt = self.vtable({0x58: self.v58, 0x70: self.v70, 0x100: self.v100, 0x108: self.v108,
                                0x138: self.v138, 0x140: self.v140, 0x34: self.v34})
        record_vt = self.vtable({0xc: self.record_dead, 0x8: self.record_region})
        for ident, o in objs.items():
            p = self.a(ident)
            k = o['kind']
            if k == 'actor':
                self.w(p, actor_vt)
                d = self.place(o['definition'], 0x600); self.wb(d + 0x2f4, int(o['def2f4']))
                self.wb(d + 0x348, int(o['def348'])); self.w(d + 0x548, o['def548']); self.w(p + 4, d)
                self.w(p + 0x84, o['w84']); self.w(p + 0x128, o['w128'])
                if o['org'] is not None:
                    c98 = self.place(100000 + ident, 0x20); self.w(c98 + 0x14, self.a(o['org'])); self.w(p + 0x98, c98)
                self.w(p + 0x9c, self.place(200000 + ident, 0x20))
                self.w(d + 0x534, o['def534'])
                self.w(p + 0x18c, 0)
                if o.get('economy') is not None:
                    have, reserved, econ = o['economy']
                    for off, arr, cnt in ((0x1a0, have, 0x1a4), (0x1b8, reserved, None), (0x1c4, econ, 0x1c8)):
                        a_ = self.alloc(4 * max(len(arr), 1))
                        for i, v in enumerate(arr):
                            self.w(a_ + 4 * i, v)
                        self.w(p + off, a_)
                        if cnt:
                            self.w(p + cnt, len(arr))
                self.w(p + 0xd8, self.a(o['cd8'])); self.w(p + 0x154, self.a(o['record']))
                hq = self.alloc(4 * max(len(o['hqs']), 1))
                for i, h in enumerate(o['hqs']):
                    self.w(hq + 4 * i, self.a(h))
                self.w(p + 0x2dc, hq); self.w(p + 0x2e0, len(o['hqs']))
                self.w(p + 8, o['flags']); self.w(p + 0x14, o['gtid'])
                self.w(p + 0x20, o['pos'][0]); self.w(p + 0x24, o['pos'][1])
                self.w(p + 0x7c, self.a(o['c7c']))
                if o['c80'] is not None:
                    comp = self.alloc(0x20); self.w(comp + 0x10, self.a(o['c80'])); self.w(p + 0x80, comp)
                self.w(p + 0x100, o['f100']); self.w(p + 0x104, o['f104']); self.w(p + 0x118, o['w118'])
            elif k == 'component':
                self.w(p + 0x5c, o['c5c']); self.w(p + 0x60, o['c60'])
                self.w(p + 0x40, self.a(o['c40'])); self.w(p + 0xd0, o['cd0'])
            elif k == 'key':
                self.w(p, o['key'])
            elif k == 'record':
                self.w(p, record_vt); self.w(p + 4, 1000); self.w(p + 8, o['object'])
                self.w(p + 0x18, o['mask'] & 0xffffffff); self.w(p + 0x1c, o['mask'] >> 32)
                if o['route'] is not None:
                    r = self.alloc(8); self.w(r, o['route']); self.w(p + 0x30, r)
                if o['table'] is not None:
                    t = self.place(300000 + ident, 0x60); self.w(t + 0x48, self.hash_buckets(o['table'])); self.w(t + 0x4c, o['table']['n'])
                    self.w(p + 0x24, t)
                self.wb(p + 0x28, int(o['b28']))
            elif k == 'region':
                self.w(p, o['index']); self.w(p + 0xc, o['area'])
                rect = self.alloc(0x10); self.w(rect + 8, o['island']); node = self.alloc(8); self.w(node, rect)
                self.w(p + 4, node)
                recs = self.alloc(8)
                if o['explored'] is not None:
                    rr = self.alloc(0x20); self.w(rr, p); self.w(rr + 8, o['explored']); self.w(recs, rr)
                self.w(p + 0x10, recs); self.w(p + 0x14, 1)
            elif k == 'entry':
                self.w(p + 0x14, o['w14']); self.w(p + 0x18, o['w18'])
        self.registry(c['registry'])
        # SAI player, ego, controller, goal.
        pl = c['player']
        self.player = self.place(pl['id'], 0x200)
        self.w(self.player, pl['slot']); self.w(self.player + 8, self.a(pl['actor']))
        ego = self.alloc(0x1000)
        for off, v in c['ego'].items():
            self.w(ego + int(off), v)
        self.w(self.player + 0x1c, ego)
        self.w(ego + 0x830, self.hash_buckets(c['ego_table_830'])); self.w(ego + 0x834, c['ego_table_830']['n'])
        self.wb(ego + 0x7a4, c['ego_7a4'])
        self.w(self.player + 0x9c, self.gp_list(c['player_gps']))
        for ident, o in objs.items():
            if o['kind'] == 'region':
                ents = self.alloc(4 * (pl['slot'] + 1))
                e = self.alloc(0x40); self.w(e, self.player)
                rec = self.alloc(0x10); self.w(rec + 8, o['entry_state']); self.w(e + 0x2c, rec)
                self.w(ents + 4 * pl['slot'], e)
                self.w(self.a(ident) + 0x1c, ents); self.w(self.a(ident) + 0x20, pl['slot'] + 1)
        ctl = self.alloc(0x40); self.w(ctl + 4, self.player)
        g = c['goal']
        self.goal = self.alloc(0x100)
        self.w(self.goal, self.vtable({0x14: self.goal_expired, 0x18: self.goal_v18, 0x20: self.goal_location}))
        self.w(self.goal + 4, ctl); self.w(self.goal + 0x38, g['priority'])
        for i, v in enumerate(g['words']):
            off = 0x3c + 4 * i
            self.w(self.goal + off, self.a(v) if off in g['pointer_words'] else v)
        # Globals.
        sai = self.alloc(0x100); self.w(SAI, sai); self.w(sai + 0x80, self.gp_list(c['sai_gps']))
        world = self.alloc(0x400); self.w(WORLD, world); self.w(world + 0xe0, c['time'])
        self.solver = self.alloc(0x40); self.w(world + 0x234, self.solver)
        mp = self.alloc(0x40); self.w(MAP, mp); self.wb(mp + 5, int(c['map_ready']))
        self.mapobj = mp
        gmap = self.alloc(0x40); self.w(world + 0x30, gmap)
        self.w(gmap + 0xc, c['extent'][0]); self.w(gmap + 0x10, c['extent'][1])
        self.gmap = gmap
        self.sai = sai
        rng = self.alloc(4); self.w(rng, c['rng']); self.w(sai + 0x2c, rng); self.rng = rng
        islands = self.alloc(4 * len(c['islands']))
        self.island_ptrs = []
        for i, isl in enumerate(c['islands']):
            ip = self.alloc(0x10); regs = self.alloc(4 * max(len(isl), 1))
            for k, r in enumerate(isl):
                self.w(regs + 4 * k, self.a(r))
            self.w(ip + 4, regs); self.w(ip + 8, len(isl)); self.w(islands + 4 * i, ip); self.island_ptrs.append(ip)
        self.w(mp + 8, islands); self.w(mp + 0x2c, self.island_ptrs[0])
        setting = self.alloc(4); self.w(setting, c['explore_setting']); cfg = self.alloc(0x200); self.w(cfg + 0x108, setting)
        self.w(0xb39ab0, cfg)
        if c['statics'] is not None:
            diffs, econ = c['statics']
            for base, arr in ((0xb28aa4, diffs), (0xb28a98, econ)):
                a_ = self.alloc(4 * max(len(arr), 1))
                for i, v in enumerate(arr):
                    self.w(a_ + 4 * i, v)
                self.w(base, a_); self.w(base + 4, len(arr)); self.w(base + 8, len(arr))
            self.wb(0xb40ec0, 1)
        for addr, fn in [(0x7aa3d0, self.s_region_at), (0x53f320, self.s_route), (0x5c2cd0, self.s_release),
                         (0x79f390, self.s_unit_check), (0x4ea430, self.s_seen), (0x4d0230, self.s_health),
                         (0x7ad9e0, self.s_attack_value), (0x7ad340, self.s_threat_value),
                         (0x7ad3e0, self.s_repair_value), (0x7bd360, self.s_region_threat),
                         (0x42a530, self.s_tile), (0x54f8b0, self.s_zone), (0x79e590, self.s_79e590),
                         (0x574a10, self.s_hq_accepts), (0x4d01e0, self.s_4d01e0), (0x4a6fa0, self.s_give_resources),
                         (0x4ace30, self.s_give_actor), (0x4a9db0, self.s_definition_value),
                         (0x587b60, self.s_can_build), (0x557190, self.s_build_site), (0x557010, self.s_unpack_site),
                         (0x566110, self.s_can_upgrade), (0x7bb1c0, self.s_regions_within), (0x7ae4c0, self.s_build_value),
                         (0x7ada20, self.s_upgrade_value), (0x4a6db0, self.s_template_value),
                         (0x4a6d40, self.s_template_missing), (0x4a6ce0, self.s_template_has),
                         (0x4ac970, self.s_destroy_value), (0x4a63a0, self.s_destroy_record_value),
                         (0x4a9ae0, self.s_unpack_value)]:
            self.seam(addr, fn)

    def hash_buckets(self, t):
        b = self.alloc(4 * max(t['n'], 1))
        for i, chain in enumerate(t['buckets']):
            head = 0
            for key, value in reversed(chain):
                node = self.alloc(12); self.w(node, key); self.w(node + 4, value); self.w(node + 8, head); head = node
            self.w(b + 4 * i, head)
        return b

    def gp_list(self, gps):
        head = 0
        for gp in reversed(gps):
            o = self.alloc(0x40)
            d2 = self.alloc(0x10); self.w(d2 + 0xc, gp['kind'])
            d1 = self.alloc(0x10); self.w(d1 + 8, d2)
            self.w(o + 8, gp['caster']); self.w(o + 0x10, self.a(gp['side'])); self.w(o + 0x14, d1)
            self.w(o + 0x18, gp['bonus']); self.w(o + 0x1c, gp['target']); self.wb(o + 0x20, int(gp['targets_actor']))
            self.w(o + 0x24, gp['owner']); self.w(o + 0x2c, gp['point'][0]); self.w(o + 0x30, gp['point'][1])
            self.wb(o + 0x34, int(gp['has_point'])); self.wb(o + 0x3d, int(gp['additive']))
            node = self.alloc(8); self.w(node, o); self.w(node + 4, head); head = node
        return head

    # answers -----------------------------------------------------------
    def bool_answer(self, p_true=0.7):
        return self.rnd.random() < p_true

    def obj_answer(self, pool):
        return self.pick(self.c['pools'][pool])

    def this(self):
        return self.id_of(self.ecx())

    def v58(self, m):
        r = self.bool_answer(); self.ev('virtual_58', obj=self.this(), other=self.id_of(self.arg(1)), flag=self.arg(2), result=r)
        self.ret(int(r), 8)

    def v70(self, m):
        r = self.bool_answer(0.6); self.ev('virtual_70', obj=self.this(), other=self.id_of(self.arg(1)), flag=self.arg(2), result=r)
        self.ret(int(r), 8)

    def v100(self, m):
        r = self.obj_answer('v100'); self.ev('virtual_100', obj=self.this(), result=r); self.ret(self.a(r))

    def v108(self, m):
        r = self.obj_answer('v108'); self.ev('virtual_108', obj=self.this(), result=r); self.ret(self.a(r))

    def v138(self, m):
        r = self.bool_answer(); self.ev('virtual_138', obj=self.this(), other=self.id_of(self.arg(1)), result=r)
        self.ret(int(r), 4)

    def v140(self, m):
        r = self.bool_answer(0.8); self.ev('virtual_140', obj=self.this(), other=self.id_of(self.arg(1)), result=r)
        self.ret(int(r), 4)

    def record_dead(self, m):
        r = self.bool_answer(0.1); self.ev('record_dead', record=self.this(), result=r); self.ret(int(r))

    def goal_expired(self, m):
        assert self.ecx() == self.goal
        r = self.bool_answer(0.1); self.ev('goal_expired', result=r); self.ret(int(r))

    def goal_v18(self, m):
        assert self.ecx() == self.goal
        self.ev('goal_virtual_18', flag=self.arg(1) & 0xff); self.ret(0, 4)

    def goal_location(self, m):
        assert self.ecx() == self.goal
        r = self.obj_answer('location'); self.ev('goal_location', result=r); self.ret(self.a(r))

    def s_region_at(self, m):
        assert self.ecx() == self.mapobj
        r = self.obj_answer('region_at')
        self.ev('region_at', point=[self.arg(1), self.arg(2)], result=r); self.ret(self.a(r), 8)

    def s_route(self, m):
        assert self.ecx() == self.solver and self.arg(4) == 1
        lst, start, goal = self.arg(1), self.arg(2), self.arg(3)
        if self.bool_answer(0.75):
            hops = self.rnd.randrange(0, 7)
            head = 0
            for _ in range(hops):
                node = self.alloc(0x10); self.w(node + 8, head); head = node
            self.w(lst, head)
            self.ev('route_hops', start=start, goal=goal, result=hops); self.ret(1, 0x10)
        else:
            self.ev('route_hops', start=start, goal=goal, result=None); self.ret(self.pick([0, 2]), 0x10)

    def s_release(self, m):
        self.ret(0)

    def s_unit_check(self, m):
        r = self.bool_answer(0.6); self.ev('unit_check', obj=self.id_of(self.arg(1)), result=r); self.ret(int(r))

    def s_seen(self, m):
        r = self.obj_answer('seen')
        self.ev('seen_record', obj=self.id_of(self.ecx()), kingdom=self.id_of(self.arg(1)), result=r)
        self.ret(self.a(r) if r else 0, 4)

    def s_health(self, m):
        img = self.pick([x87_ext(self.rnd, 0.0, 1.3), x87_of(self.rnd.choice([0.0, 0.25, 0.5, 1.0])),
                         x87_of(from32(self.c['ego'].get(str(0x424), 0))), x87_of(from32(self.c['ego'].get(str(0x438), 0)))])
        self.ev('health', obj=self.this(), flag=self.arg(1) & 0xff, result=img); self.x87_return(img, 4)

    def value(self):
        return self.pick([x87_ext(self.rnd, -20, 400), x87_ext(self.rnd, 0, 50), x87_of(0.0)])

    def s_attack_value(self, m):
        assert self.ecx() == self.player
        img = self.value(); self.ev('attack_value', target=self.id_of(self.arg(1)), result=img); self.x87_return(img, 4)

    def s_threat_value(self, m):
        assert self.ecx() == self.player
        img = self.value()
        self.ev('threat_value', actor=self.id_of(self.arg(1)), flag=self.arg(2) & 0xff, result=img); self.x87_return(img, 8)

    def s_repair_value(self, m):
        assert self.ecx() == self.player
        img = self.value(); self.ev('repair_value', actor=self.id_of(self.arg(1)), result=img); self.x87_return(img, 4)

    def s_region_threat(self, m):
        entry = self.ecx()
        assert self.arg(2) == 0
        region = None
        for ident, o in self.c['objects'].items():
            if o['kind'] == 'region':
                ents = self.r(self.a(int(ident)) + 0x1c)
                if self.r(ents + 4 * self.c['player']['slot']) == entry:
                    region = int(ident)
        out = self.obj_answer('threat')
        self.w(self.arg(1), self.a(out))
        img = self.value()
        self.ev('region_threat', region=region, result=[img, out]); self.x87_return(img, 8)

    def s_tile(self, m):
        assert self.ecx() == self.gmap
        tile = self.alloc(8); self.w(tile, self.arg(1)); self.w(tile + 4, self.arg(2))
        self.ret(tile, 8)

    def s_zone(self, m):
        tile = self.ecx()
        r = self.bool_answer(0.3)
        self.ev('tile_zone', point=[self.r(tile), self.r(tile + 4)], kingdom=self.id_of(self.arg(1)), flag=self.arg(2), result=r)
        self.ret(int(r), 8)

    def s_79e590(self, m):
        assert self.ecx() == self.sai
        r = self.bool_answer(0.7); self.ev('sai_query_79e590', kingdom=self.id_of(self.arg(1)), result=r); self.ret(int(r), 4)

    def s_hq_accepts(self, m):
        assert self.arg(2) == 0 and self.arg(3) == 0
        r = self.bool_answer(0.4)
        self.ev('hq_accepts', component=self.this(), actor=self.id_of(self.arg(1)), result=r); self.ret(int(r), 12)

    def s_4d01e0(self, m):
        img = self.value(); self.ev('value_4d01e0', obj=self.this(), flag=self.arg(1) & 0xff, result=img); self.x87_return(img, 4)

    def s_give_resources(self, m):
        img = self.value()
        self.ev('give_resources_value', kingdom=self.id_of(self.arg(1)), actor=self.id_of(self.arg(2)), amount=self.arg(3), result=img)
        self.x87_return(img, 12)

    def s_give_actor(self, m):
        img = self.value()
        self.ev('give_actor_value', kingdom=self.id_of(self.arg(1)), recipient=self.id_of(self.arg(2)),
                given=self.id_of(self.arg(3)), player=self.id_of(self.arg(4)), result=img)
        self.x87_return(img, 16)

    def s_definition_value(self, m):
        img = self.value()
        self.ev('definition_value', definition=self.id_of(self.arg(1)), player=self.id_of(self.arg(2)), flag=self.arg(3) & 0xff, result=img)
        self.x87_return(img, 12)

    def v34(self, m):
        r = self.bool_answer(0.45); self.ev('virtual_34', obj=self.this(), kind=self.arg(1), result=r); self.ret(int(r), 4)

    def record_region(self, m):
        r = self.obj_answer('location') or self.c['regions'][0]
        self.ev('record_region', record=self.this(), result=r); self.ret(self.a(r))

    def s_can_build(self, m):
        r = self.bool_answer(0.8)
        self.ev('can_build', component=self.this(), definition=self.arg(1), point=[self.arg(2), self.arg(3)],
                rest=[self.arg(4), self.arg(5), self.arg(6)], result=r)
        self.ret(int(r), 0x18)

    def site(self, out):
        if self.bool_answer(0.8):
            pt = [rf(self.rnd, 0, 700), rf(self.rnd, 0, 700)]
            self.w(out, pt[0]); self.w(out + 4, pt[1]); return pt
        return None

    def s_build_site(self, m):
        assert self.ecx() == self.gmap and self.arg(5) == self.rng
        r = self.site(self.arg(1))
        self.ev('build_site', definition=self.arg(2), kingdom=self.id_of(self.arg(3)), builder=self.id_of(self.arg(4)), result=r)
        self.ret(int(r is not None), 0x14)

    def s_unpack_site(self, m):
        assert self.ecx() == self.gmap and self.arg(5) == self.rng and self.arg(6) == 1 and self.arg(7) == 0
        r = self.site(self.arg(1))
        self.ev('unpack_site', layout=self.arg(2), kingdom=self.id_of(self.arg(3)), region=self.id_of(self.arg(4)), result=r)
        self.ret(int(r is not None), 0x1c)

    def s_can_upgrade(self, m):
        assert self.arg(2) == 0 and self.arg(3) == 0
        r = self.bool_answer(0.8); self.ev('can_upgrade', component=self.this(), definition=self.arg(1), result=r)
        self.ret(int(r), 0xc)

    def s_regions_within(self, m):
        lst, depth = self.arg(1), self.arg(2)
        assert self.arg(3) == 0
        regions = [r for r in self.c['regions'] if self.rnd.random() < 0.5]
        head = tail = 0
        for r in regions:  # appended in order
            node = self.alloc(8); self.w(node, self.a(r)); self.w(node + 4, 0)
            if tail:
                self.w(tail + 4, node)
            else:
                head = node
            tail = node
        self.w(lst, head); self.w(lst + 4, tail); self.w(lst + 8, len(regions))
        self.ev('regions_within', region=self.this(), depth=depth, result=regions); self.ret(0, 0xc)

    def s_build_value(self, m):
        assert self.ecx() == self.player and self.arg(4) == 1 and self.arg(5) == 1
        img = self.value(); self.ev('build_value', definition=self.arg(1), point=[self.arg(2), self.arg(3)], result=img)
        self.x87_return(img, 0x14)

    def s_upgrade_value(self, m):
        assert self.ecx() == self.player
        img = self.value(); self.ev('upgrade_value', actor=self.id_of(self.arg(1)), definition=self.arg(2), result=img)
        self.x87_return(img, 8)

    def s_template_value(self, m):
        img = self.value()
        self.ev('template_value', table=self.this(), record=self.id_of(self.arg(1)), definition=self.arg(2), result=img)
        self.x87_return(img, 8)

    def s_template_missing(self, m):
        r = self.obj_answer('entries')
        self.ev('template_missing', table=self.this(), record=self.id_of(self.arg(1)), result=r); self.ret(self.a(r), 4)

    def s_template_has(self, m):
        r = self.bool_answer(0.4)
        self.ev('template_has', table=self.this(), definition=self.id_of(self.arg(1)), result=r); self.ret(int(r), 4)

    def vec(self, base):
        data, n = self.r(base), self.r(base + 4)
        return [self.r(data + 4 * i) for i in range(n)]

    def s_destroy_value(self, m):
        assert self.ecx() == self.r(self.player + 0x1c) and self.arg(1) == self.player
        img = self.value()
        self.ev('destroy_value', target=self.id_of(self.arg(2)), differences=self.vec(self.arg(5)),
                economy=self.vec(self.arg(6)), result=img)
        self.x87_return(img, 0x18)

    def s_destroy_record_value(self, m):
        assert self.ecx() == self.r(self.player + 0x1c) and self.arg(1) == self.player
        img = self.value(); self.ev('destroy_record_value', record=self.id_of(self.arg(2)), result=img)
        self.x87_return(img, 8)

    def s_unpack_value(self, m):
        assert self.ecx() == self.r(self.player + 0x1c) and self.arg(3) == self.player and self.arg(7) == self.rng
        img = self.value()
        self.ev('unpack_value', record=self.id_of(self.arg(1)), region=self.id_of(self.arg(2)),
                point=[self.arg(4), self.arg(5)], same=self.arg(6) & 0xff, result=img)
        self.x87_return(img, 0x1c)

    def run(self):
        entry = CLASSES[self.c['class']]
        out = {}
        try:
            self.call(entry, ecx=self.goal, args=[], count=500000)
            out['fault'] = False
        except AssertionError as e:
            out['fault'] = True
            out['fault_detail'] = str(e)[:200]
            self.null_accesses.clear()
        except Exception as e:  # UcError: invalid memory
            out['fault'] = True
            from unicorn.x86_const import UC_X86_REG_EIP
            out['fault_detail'] = '%s at %x' % (e, self.u.reg_read(UC_X86_REG_EIP))
        out['events'] = self.events
        g = self.c['goal']
        out['priority'] = self.r(self.goal + 0x38)
        out['words'] = [self.id_of(self.r(self.goal + 0x3c + 4 * i)) if 0x3c + 4 * i in g['pointer_words']
                        else self.r(self.goal + 0x3c + 4 * i) for i in range(len(g['words']))]
        out['rng'] = self.r(self.rng)
        out['statics'] = [self.vec(0xb28aa4), self.vec(0xb28a98)] if self.rb(0xb40ec0) else None
        return out


def from32(bits):
    return struct.unpack('<f', struct.pack('<I', bits & 0xffffffff))[0]


def rf(rnd, lo, hi):
    return f32(rnd.uniform(lo, hi))


def hash_table(rnd, keys):
    n = rnd.choice([1, 3, 7, 11])
    buckets = [[] for _ in range(n)]
    for k in rnd.sample(keys, rnd.randrange(0, len(keys) + 1)):
        buckets[k % n].insert(0, [k, rf(rnd, -10, 80)])
    if rnd.random() < 0.3 and keys:  # duplicate key deeper in a chain
        k = rnd.choice(keys); buckets[k % n].append([k, rf(rnd, -10, 80)])
    return {'n': n, 'buckets': buckets}


def make_case(rnd, cls, precision, seed):
    ids = iter(range(1, 1000))
    objects, registry = {}, {}
    used = set()

    def new_gtid(live_obj):
        while True:
            idx = rnd.randrange(1, 0x400)
            if idx not in used:
                break
        used.add(idx)
        serial = rnd.randrange(1, 0x8000)
        registry[idx] = [serial, live_obj]
        return (serial << 16) | idx

    def stale_gtid():
        mode = rnd.randrange(3)
        if mode == 0 and registry:
            idx = rnd.choice(sorted(registry))
            return ((registry[idx][0] ^ 0x55) << 16) | idx  # serial mismatch
        if mode == 1:
            g = new_gtid(0)  # empty slot
            return g
        while True:
            idx = rnd.randrange(0x400, 0x800)
            if idx not in used:
                used.add(idx)
                return (rnd.randrange(1, 0x8000) << 16) | idx  # never registered: serial 0 != serial

    keys = [rnd.randrange(1, 1 << 20) for _ in range(4)]
    regions = []
    for i in range(rnd.randrange(2, 5)):
        rid = next(ids)
        objects[rid] = {'kind': 'region', 'index': rnd.randrange(0, 60), 'entry_state': rnd.choice([0, 1, 2, 2, 3]),
                        'area': rnd.choice([1, 4, 9, 16, 30, 0]), 'island': rnd.choice([0, 0, 1]),
                        'explored': rnd.choice([None, 0, 1, 3, 8, 16, 30])}
        regions.append(rid)
    comps = []
    for i in range(2):
        cid = next(ids)
        objects[cid] = {'kind': 'component', 'c5c': rf(rnd, 0, 100), 'c60': rnd.choice([0, f32(-1.0), rf(rnd, 1, 120)])}
        comps.append(cid)
    actors = []
    for i in range(rnd.randrange(3, 7)):
        aid = next(ids)
        actors.append(aid)
    for aid in actors:
        objects[aid] = {'kind': 'actor', 'def2f4': rnd.random() < 0.5,
                        'flags': rnd.choice([0, 0, 1, 1 << 29, (1 << 29) | 0x10, 0x10]),
                        'gtid': 0, 'pos': [rf(rnd, 0, 500), rf(rnd, 0, 500)],
                        'c7c': rnd.choice([0] + comps * 3), 'c80': rnd.choice([None, 0, rnd.choice(actors)]),
                        'f100': rnd.choice([0, 0x80000, 0x10]), 'f104': rnd.choice([0, 0, 0x80, 0x40]),
                        'w118': rnd.choice([0, 0, 1]), 'definition': next(ids), 'def348': rnd.random() < 0.8,
                        'def548': rnd.choice([0, 1, 7]), 'w84': rnd.choice([0, 3, 3]), 'org': None,
                        'def534': rnd.choice([0, 0, 0x77, 0x99]), 'economy': None,
                        'w128': rnd.choice([0, 0, 0, 9]), 'cd8': rnd.choice(comps), 'record': 0,
                        'hqs': [rnd.choice([0] + actors) for _ in range(rnd.randrange(0, 4))]}
        objects[aid]['gtid'] = new_gtid(aid) if rnd.random() < 0.9 else stale_gtid()
    for aid in actors:
        objects[aid]['org'] = rnd.choice([None, None, 0, rnd.choice(actors)])
    for cid in comps:
        objects[cid]['c40'] = rnd.choice([0] + actors)
        objects[cid]['cd0'] = rnd.choice([stale_gtid(), stale_gtid(), objects[rnd.choice(actors)]['gtid']])
    records = []
    for i in range(2):
        rid = next(ids)
        live_actors = [a for a in actors if registry.get(objects[a]['gtid'] & 0xffff, [None, 0])[1] == a]
        objects[rid] = {'kind': 'record', 'object': objects[rnd.choice(live_actors or actors)]['gtid'] if rnd.random() < 0.85 else stale_gtid(),
                        'mask': rnd.choice([1 << 4, (1 << 4) | (1 << 33), 1 << 2, 0x10010]),
                        'route': rnd.choice([None, 0, 2, 2, 1]), 'table': rnd.choice([None, hash_table(rnd, keys)]), 'b28': rnd.random() < 0.6}
        records.append(rid)
    for aid in actors:
        objects[aid]['record'] = rnd.choice([0] + records + records)
    player_actor = rnd.choice(actors) if rnd.random() < 0.95 else 0
    if player_actor:
        n_res = rnd.randrange(1, 9)
        objects[player_actor]['economy'] = [[rf(rnd, 0, 900) for _ in range(n_res)],
                                            [rf(rnd, 0, 300) for _ in range(n_res)],
                                            [rf(rnd, 0, 900) for _ in range(rnd.randrange(1, 9))]]
    entries = []
    for i in range(2):
        eid = next(ids); objects[eid] = {'kind': 'entry', 'w14': rf(rnd, 0, 20), 'w18': rf(rnd, 0, 20)}
        entries.append(eid)
    pid = next(ids)

    def gtid_choice():
        r = rnd.random()
        if r < 0.75:
            return objects[rnd.choice(actors + ([player_actor] if player_actor else []))]['gtid']
        return stale_gtid()

    class_kind = {'ATTACK_REGION': 0, 'ATTACK_ACTOR': 0, 'AIR_ATTACK': 0, 'BOMBARD': 0, 'DEFEND_REGION': 2,
                  'DEFEND_ACTOR': 2, 'RECOVER': 3, 'REPAIR': 3, 'EXPLORE': 9, 'BUILD': 4,
                  'UPGRADE': 4}.get(cls, 0)
    focus = []

    def gp():
        target = gtid_choice()
        if focus and rnd.random() < 0.5:
            target = rnd.choice(focus)
        return {'caster': gtid_choice(), 'side': rnd.choice([0] + actors),
                'kind': class_kind if rnd.random() < 0.6 else rnd.choice([0, 2, 3, 1]),
                'bonus': rnd.choice([0, 0, rf(rnd, -5, 60), rf(rnd, 0, 30), f32(-0.0), rf(rnd, 0, 30)]),
                'target': target, 'targets_actor': rnd.random() < 0.5,
                'owner': objects[player_actor]['gtid'] if player_actor and rnd.random() < 0.65 else gtid_choice(),
                'point': [rf(rnd, 0, 500), rf(rnd, 0, 500)], 'has_point': rnd.random() < 0.6,
                'additive': rnd.random() < 0.3}

    ego = {str(off): rf(rnd, 0.1, 3.0) for off in EGO_FLOATS}
    ego[str(0x890)] = rnd.choice([rf(rnd, 0, 40), 0])
    ego[str(0x3f4)] = rnd.randrange(-1, 6)
    for off in (0x424, 0x428, 0x438):
        ego[str(off)] = rf(rnd, 0.0, 1.1)
    ego[str(0x6d4)] = rf(rnd, 0.2, 1.1); ego[str(0x6e0)] = rf(rnd, 0.2, 1.1)
    ego[str(0x450)] = rnd.choice([0, f32(-1.0), rf(rnd, 1, 120)])
    time = rf(rnd, 0, 600)
    def target_choice():
        live = [a for a in actors if registry.get(objects[a]['gtid'] & 0xffff, [None, 0])[1] == a]
        if live and rnd.random() < 0.85:
            return objects[rnd.choice(live)]['gtid']
        return gtid_choice()
    words = [0] * 8
    pointer_words = []
    priority = rnd.choice([0, f32(-3.0), rf(rnd, 0.5, 200), rf(rnd, 0.5, 200), rf(rnd, -50, 300)])
    if cls in ('ATTACK_ACTOR', 'DEFEND_ACTOR'):
        words[(0x4c - 0x3c) // 4] = target_choice()
    elif cls in ('AIR_ATTACK', 'BOMBARD', 'REPAIR'):
        words[0] = target_choice()
        words[1] = rnd.choice([f32(-1.0), 0, rf(rnd, 0, 600), time])
    elif cls == 'RECOVER':
        words[0] = rnd.choice(records)
        pointer_words = [0x3c]
    elif cls == 'DEFEND_REGION':
        words[(0x4c - 0x3c) // 4] = target_choice()
    elif cls == 'PACK':
        words[0] = rnd.choice(records)
        pointer_words = [0x3c]
    elif cls == 'GIVE_RESOURCES':
        words[0] = target_choice(); words[1] = rnd.choice([rf(rnd, 0, 500), rnd.randrange(0, 1000)])
    elif cls in ('GIVE_ACTOR',):
        words[0] = target_choice(); words[1] = target_choice()
    elif cls == 'RESEARCH':
        kid = next(ids)
        objects[kid] = {'kind': 'key', 'key': rnd.choice(keys + [rnd.randrange(1, 1 << 20)])}
        words[0] = target_choice(); words[1] = kid
        pointer_words = [0x40]
    elif cls == 'ATTACH':
        words[0] = target_choice(); words[1] = rnd.choice([0, gtid_choice()])
    elif cls == 'EXPLORE':
        words[0] = rnd.choice([0, 1, 2, 3]); words[1] = rnd.choice([0, 1, 2, 5, -1 & 0xffffffff])
    elif cls in ('BUILD', 'UPGRADE'):
        words[0] = target_choice(); words[1] = rnd.choice([0x30000040, 0x30000080])
    elif cls == 'DESTROY':
        words[0] = target_choice()
    elif cls == 'UNPACK':
        words[0] = rnd.choice(records); words[1] = rnd.choice([0, rnd.choice(regions)])
        pointer_words = [0x3c, 0x40]
    if cls in ('ATTACK_ACTOR', 'DEFEND_ACTOR'):
        focus.append(words[(0x4c - 0x3c) // 4])
    elif cls in ('AIR_ATTACK', 'BOMBARD', 'REPAIR'):
        focus.append(words[0])
    elif cls == 'RECOVER':
        focus.append(objects[words[0]]['object'])
    sai_gps = [gp() for _ in range(rnd.randrange(0, 6))]
    player_gps = [gp() for _ in range(rnd.randrange(0, 5))]
    pools = {'v100': [0, 0] + actors, 'v108': [0] + actors + ([player_actor] if player_actor else []),
             'location': regions + [regions[0]] * 4 + ([0] if cls == 'ATTACK_REGION' or rnd.random() < 0.05 else []),
             'region_at': regions + [regions[0]] * 3 + [0], 'seen': [0] + actors[:2], 'threat': [0] + actors}
    ego[str(0x368)] = rf(rnd, 0, 90); ego[str(0x7a8)] = rf(rnd, 0, 3); ego[str(0x82c)] = rf(rnd, -5, 50)
    for base in (0x464, 0x528, 0x5ec):
        ego[str(base + 4)] = rf(rnd, 0, 100); ego[str(base + 8)] = rf(rnd, 0, 30)
        ego[str(base + 0xc)] = rf(rnd, -10, 60); ego[str(base + 0x10)] = rf(rnd, 0, 1.1)
    ego[str(0x7ac)] = rf(rnd, 0, 50); ego[str(0x7b0)] = rf(rnd, 0, 20)
    ego[str(0x334)] = rnd.randrange(0, 4); ego[str(0x338)] = rnd.choice([-1, 0, 1, 3, 5])
    islands = [[r for r in regions if objects[r]['island'] == 0] + ([0] if rnd.random() < 0.2 else []),
               [r for r in regions if objects[r]['island'] == 1]]
    if not islands[0] or rnd.random() < 0.1:
        islands[0] = islands[0]  # an empty primary island is allowed (native 0/0)
    pools['entries'] = [0] + entries
    return {'class': cls, 'precision': precision, 'ego_table_830': hash_table(rnd, keys), 'ego_7a4': rnd.choice([0, 1]),
            'regions': regions, 'islands': islands, 'rng': rnd.getrandbits(32),
            'explore_setting': rnd.choice([0, 1, 1, 1]),
            'statics': rnd.choice([None, None, [[rf(rnd, 0, 9)], [rf(rnd, 0, 9), rf(rnd, 0, 9)]]]),
            'extent': [rf(rnd, 100, 600), rf(rnd, 100, 600)], 'seed': seed, 'objects': {str(k): v for k, v in objects.items()},
            'registry': {str(k): v for k, v in registry.items()},
            'player': {'id': pid, 'slot': rnd.randrange(0, 4), 'actor': player_actor},
            'ego': ego, 'sai_gps': sai_gps, 'player_gps': player_gps,
            'time': time, 'map_ready': rnd.random() < 0.9, 'pools': pools,
            'goal': {'priority': priority, 'words': words, 'pointer_words': pointer_words}}


def main():
    per_class = int(sys.argv[1]) if len(sys.argv) > 1 else 300
    out, visited, stats = [], {}, {}
    precisions = list(PRECISIONS)
    for ci, cls in enumerate(CLASSES):
        rnd = random.Random(0x78000 + ci)
        for i in range(per_class):
            c = make_case(rnd, cls, precisions[i % 3], rnd.getrandbits(32))
            m = BasePriorityWorld(c)
            r = m.run()
            m.verify_v14(); visited.update(m.visited)
            out.append({'input': c, 'output': r})
            s = stats.setdefault(cls, {'cases': 0, 'faults': 0})
            s['cases'] += 1; s['faults'] += int(r['fault'])
    d = header(__file__, __doc__)
    d.update(instructions=[], cases=out, stats=stats)
    p = HERE / 'native-base-priority.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    from fn_extent import extent
    cover = {}
    for cls, start in CLASSES.items():
        body = [i.address for i in extent(start)]
        cover[cls] = '%d/%d' % (len([a for a in body if a in visited]), len(body))
    for name, start in [('572040', 0x572040), ('5720C0', 0x5720c0), ('572120', 0x572120), ('572940', 0x572940),
                        ('572810', 0x572810), ('571C60', 0x571c60), ('4CF010', 0x4cf010)]:
        body = [i.address for i in extent(start)]
        cover[name] = '%d/%d' % (len([a for a in body if a in visited]), len(body))
    print(json.dumps({'cases': len(out), 'stats': stats, 'body_instructions': cover, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
