"""Original metagame battle resolution on perturbed InitCreate worlds.

Runs the ORIGINAL code of KKC_MGCommand::ResolveBattle 954130 (953A20
messages = seam, KKC_MGBattleResolver 966FF0 / Process 967530 with 93E8F0,
955420, 938B70, ProcessAttackerWin 967390 -> 967290 (95BEB0 / 938BC0
liberation, 954F30 capture + 954CF0, 956380 immobile cleanup, 9670D0
destroy behaviours 966F40..966FD0 / 954D80 / 9564F0 / 'all' loop, 9671A0
retreat-or-destroy with 967040 retreat target (954BE0/954B70, 954FD0) and
93F550 orders, 955620 owner change), ProcessDefenderWin 9673F0 (93EB50,
93F550), game-over check 938E10 (95A6B0), 938980 flag +0x46, 95EFD0 record
appended by 939410), ProcessQuickBattle 954220 (93A320, 95F220, SolveBattle
95F480), the order setter 93F550 and the deferred per-tick waypoint mover
940020 on the InitCreate worlds of every shipped campaign (mg_campaign).

Perturbations are original code (940100 relocation, 955620 owner change,
45F240 waypoint list push, 954CF0 recount) or plain field writes (army fought
C+10, kingdom battles K+3C / resources K+38, current kingdom S+10, RNG S+18,
game-over byte S+50, campaign destroy behaviours C+78 / C+7C, kingdom actor
+19C), all recorded as ops for the Rust replay. Relations (kingdom actor
vtable+140 / +138) are team based or random matrices (enemy diagonal false).

Seams (engine, not metagame): army vtable+160 (set owner: the army's
vtable+108 then returns the new owner), 4D7960 actor kill (recorded; the army
is treated as dead and no longer in its kingdom actor's +284 list),
953A20 battle messages, 4E9720 kingdom victory, 50D370 world game over,
army actor +70 object vtable+64 (busy test of 940020), 9626D0 / 962880 move
visuals, kingdom actor +284 owned-actor list (field read at 938DD8, built
from live armies by owner), 952790 message block 9527CB..95288D (skipped),
CRT heap and the InitCreate seams of mg_campaign. The component waypoint list
(+30 head / +34 tail of {value, next} nodes) is reset to empty after
InitCreate (mg_campaign lays a vector there).
"""
import json, hashlib, random, struct
from pathlib import Path
from mg_machine import P, V, VISITED, HERE
from mg_campaign import DATA, run
from mg_world import VT_ARMY, SEAM_BASE
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESI, UC_X86_REG_EIP, UC_X86_REG_ESP

BEHAVIOURS = ['None', 'First', 'Weakest', 'Strongest', 'LeastExpensive', 'MostExpensive', 'Random', 'All']
PCS = {'single24': 0x007f, 'double53': 0x027f, 'extended64': 0x037f}
BUSY_SEAM = SEAM_BASE + 0x1f0
# Branch observation points (counted, never altered) reported as coverage.
TRACE_POINTS = {
    0x9670f7: '9670D0 first', 0x967102: '9670D0 weakest', 0x96710d: '9670D0 strongest',
    0x967118: '9670D0 least_expensive', 0x967123: '9670D0 most_expensive', 0x96712e: '9670D0 random',
    0x96713b: '9670D0 single destroy succeeded', 0x96717b: '9670D0 all: destroyed one',
    0x9671b6: '9671A0 no retreat target', 0x9671ed: '9671A0 retreat target found',
    0x967241: '9671A0 retreat order', 0x9671cd: '9671A0 destroy defender',
    0x9672cb: '967290 liberation record', 0x96732f: '967290 liberated to another kingdom',
    0x967386: '967290 no winner', 0x954fa5: '954F30 capture', 0x95648c: '956380 immobile kill',
    0x956512: '9564F0 draw defender side', 0x956530: '9564F0 draw attacker side',
    0x967622: '967530 attacker win', 0x96762d: '967530 defender win', 0x9675ee: '967530 no attacker',
    0x967616: '967530 battles counter', 0x9674a0: '9673F0 retreat order',
    0x938e57: '938E10 winner', 0x938eba: '938E10 game over', 0x938e19: '938E10 entry',
    0x938e01: '938DA0 false', 0x938daf: '938DA0 capital lost', 0x938d06: '938C90 new round',
    0x955708: '9556D0 auto-capture', 0x9557b2: '93FE10 army turn start', 0x939217: '9391F0 extra advance',
    0x94004c: '940020 not busy', 0x940046: '940020 entry', 0x940075: '940020 moves',
    0x9542a8: '954220 solved', 0x95f4d0: '95F480 roll',
    0x95adff: '95ADD0 probability roll', 0x95ae2d: '95ADD0 inactive', 0x95ae44: '95ADD0 active',
    0x95ae4e: '95ADD0 random kingdom', 0x95be37: '95BE20 fire',
    0x95bccb: '95BC80 add_actor', 0x95bcd5: '95BC80 add_tech', 0x95bcdf: '95BC80 remove_actor',
    0x95bce9: '95BC80 remove_tech', 0x95bcf3: '95BC80 change_territory',
    0x95b29b: '95B220 created', 0x95aea3: '95AE80 random type', 0x95af09: '95AED0 random territory',
    0x95b490: '95B400 removed', 0x95afdd: '95AF40 random tech', 0x95b69e: '95B630 added',
    0x95b85e: '95B7F0 removed', 0x95bab3: '95B9B0 random territory', 0x95baf2: '95B9B0 captured',
}
TRACE = {}
# Localized message blocks of the event code (UI only): start -> (resume, what).
# Each starts right after the native "message length != 0" test.
EVENT_MESSAGE_BLOCKS = {
    0x95bd23: (0x95bdf4, 'event'),             # 95BC80 event message
    0x95b2ce: (0x95b3c2, 'add_actor'),         # 95B220
    0x95b4e8: (0x95b5e0, 'remove_actor'),      # 95B400 (before the kill 956350)
    0x95b6cf: (0x95b7ab, 'add_tech'),          # 95B630
    0x95b88f: (0x95b96b, 'remove_tech'),       # 95B7F0
    0x95bb3c: (0x95bc33, 'change_territory'),  # 95B9B0
}


class Harness:
    def __init__(self, ci, cw, rng, relation_mode):
        snap, ctx = run(ci, cw=cw, keep=True)
        self.ci = ci
        self.ctx = ctx
        m, w, st = ctx['m'], ctx['w'], ctx['st']
        self.m, self.w, self.st = m, w, st
        self.campaign = m.r32(st + 8)
        self.kingdoms = w.read_vector(st + 0x1c)
        self.territories = w.read_vector(st + 0x28)
        self.tindex = {p: i for i, p in enumerate(self.territories)}
        self.kindex = {p: i for i, p in enumerate(self.kingdoms)}
        self.actors = ctx['actors']
        self.aindex = {p: i for i, p in enumerate(self.actors)}
        self.created = ctx['created']
        self.army_index = {a['actor']: i for i, a in enumerate(self.created)}
        self.comps = [m.r32(a['actor'] + 0xcc) for a in self.created]
        self.killed = set()
        self.log = []
        self.ops = []
        self.busy = {}
        n = len(self.actors)
        if relation_mode == 'random':
            enemy = [[a != b and rng.random() < 0.55 for b in range(n)] for a in range(n)]
            r138 = [[(a == b) or rng.random() < 0.45 for b in range(n)] for a in range(n)]
            if rng.random() < 0.3:
                for a in range(n):
                    r138[a][a] = rng.random() < 0.7
            w.relations['enemy_140'] = enemy
            w.relations['relation_138'] = r138
        self.enemy = w.relations['enemy_140']
        self.r138 = w.relations['relation_138']
        # component waypoint list: {head, tail} of {value, next} nodes (93F550/940020/45F240)
        for comp in self.comps:
            m.w32(comp + 0x30, 0)
            m.w32(comp + 0x34, 0)
        # quick battle settings (95F270/95F370 via B39AB0 +208/+20C)
        settings = m.alloc(0x300)
        bonus = m.alloc(4); m.w32(bonus, DATA['settings']['technology_bonus_bits'])
        prob = m.alloc(4); m.w32(prob, DATA['settings']['attack_win_probability_bits'])
        m.w32(settings + 0x208, bonus); m.w32(settings + 0x20c, prob)
        m.w32(0xb39ab0, settings)
        # campaign destroy behaviours: start from the shipped values
        c = DATA['campaigns'][ci]
        m.w32(self.campaign + 0x78, BEHAVIOURS.index(c['attacking_destroy_behavior']))
        m.w32(self.campaign + 0x7c, BEHAVIOURS.index(c['defending_destroy_behavior']))
        # seams
        w.virtual(VT_ARMY, 0x160, self._set_owner)
        m.seams[0x4d7960] = self._kill
        m.seams[0x953a20] = self._messages
        m.seams[0x4e9720] = self._victory
        m.seams[0x50d370] = self._game_over
        m.seams[0x9626d0] = lambda mm: mm.ret(8, 0)
        m.seams[0x962880] = self._visual
        m.seams[0x938dd8] = self._owned_list
        m.seams[0x9527cb] = self._end_turn_message
        vt_busy = m.alloc(0x100)
        m.w32(vt_busy + 0x64, BUSY_SEAM)
        m.seams[BUSY_SEAM] = self._busy
        self.vt_busy = vt_busy
        self.busy_obj = {}
        for i, a in enumerate(self.created):
            o = m.alloc(0x10)
            m.w32(o, vt_busy)
            m.w32(a['actor'] + 0x70, o)
            self.busy_obj[o] = i
        self.cell = m.alloc(4)
        from unicorn import UC_HOOK_CODE
        for a in TRACE_POINTS:
            m.u.hook_add(UC_HOOK_CODE, self._trace, begin=a, end=a)
        self.thing_ids = {v: k for k, v in ctx['things'].items()}
        self.tech_ids = {v: k for k, v in ctx['techs'].items()}
        # campaign events (synthetic, see native_turn.py): event objects and
        # their effect objects; MGState +0x54 holds the event states.
        self.event_objs = []
        self.effect_objs = []
        self.cur_event_state = None
        self.cur_effect = None
        self.crashed = None
        self.events_json = []
        # 955B70 -> 4FA030 army actor creation (mg_campaign seam), logged
        self._orig_create = m.seams[0x4fa030]
        m.seams[0x4fa030] = self._create_army
        # kingdom actor +0x284 list reads in 95A3C0 / 95A410 (remove_actor)
        m.seams[0x95a3e6] = lambda mm: self._owned_list_into(mm, 0x95a3ec)
        m.seams[0x95a436] = lambda mm: self._owned_list_into(mm, 0x95a43c)
        # localized event / effect message blocks (UI): skipped, logged
        for start, (end, what) in EVENT_MESSAGE_BLOCKS.items():
            m.seams[start] = self._message_seam(end, what)
        # observation only: 95BC80 event state (esi) and dispatched effect (ecx)
        m.u.hook_add(UC_HOOK_CODE, self._observe_event, begin=0x95bc9d, end=0x95bc9d)
        m.u.hook_add(UC_HOOK_CODE, self._observe_effect, begin=0x95bcb9, end=0x95bcb9)

    # ---------------------------------------------------------------- seams
    def _trace(self, u, pc, size, _):
        name = TRACE_POINTS.get(pc)
        if name:
            TRACE[name] = TRACE.get(name, 0) + 1

    def _set_owner(self, mm):
        this = mm.u.reg_read(UC_X86_REG_ECX)
        owner = mm.arg(0)
        self.w.army_owner[this] = owner
        self.log.append(['set_owner', self.army_index[this], self.aindex[owner]])
        mm.ret(4, 1)

    def _kill(self, mm):
        this = mm.u.reg_read(UC_X86_REG_ECX)
        a = self.army_index[this]
        assert a not in self.killed, 'army killed twice'
        self.killed.add(a)
        self.log.append(['kill', a])
        mm.ret(0, 1)

    def _k(self, p):
        return None if p == 0 else self.kindex[p]

    def _kd(self, p):
        return None if p == 0 else self.ctx['kdatas'].index(p)

    def _messages(self, mm):
        att, dfd, t, won = mm.arg(0), mm.arg(1), mm.arg(2), mm.arg(3) & 0xff
        assert att != 0, '953A20 dereferences the attacker kingdom'
        self.log.append(['messages', self._k(att), self._k(dfd), self.tindex[t], won])
        mm.ret(0)

    def _victory(self, mm):
        this = mm.u.reg_read(UC_X86_REG_ECX)
        assert (mm.arg(0), mm.arg(1)) == (1, 1)
        self.log.append(['victory', self.aindex[this]])
        mm.ret(8)

    def _game_over(self, mm):
        self.log.append(['game_over'])
        mm.ret(0)

    def _visual(self, mm):
        actor, cur, nxt, flag = mm.arg(0), mm.arg(1), mm.arg(2), mm.arg(3)
        assert nxt != 0 and flag == 0
        self.log.append(['move_visual', self.army_index[actor], None if cur == 0 else self.tindex[cur], self.tindex[nxt]])
        mm.ret(16, 1)

    def _busy(self, mm):
        a = self.busy_obj[mm.u.reg_read(UC_X86_REG_ECX)]
        mm.ret(0, int(self.busy.get(a, 0)))

    def _owned_head(self, mm, actor):
        """Engine list of a kingdom actor's actors (+0x284, nodes {actor,
        next}): ASSUMPTION live armies owned by the actor, creation order."""
        head = 0
        for i in reversed(range(len(self.created))):
            a = self.created[i]['actor']
            if i in self.killed or self.w.army_owner[a] != actor:
                continue
            node = mm.alloc(8)
            mm.w32(node, a)
            mm.w32(node + 4, head)
            head = node
        return head

    def _owned_list(self, mm):
        # 938DD8: mov esi, [ecx+0x284] -> engine list of the kingdom actor's actors
        actor = mm.u.reg_read(UC_X86_REG_ECX)
        mm.u.reg_write(UC_X86_REG_ESI, self._owned_head(mm, actor))
        mm.u.reg_write(UC_X86_REG_EIP, 0x938dde)

    def _owned_list_into(self, mm, resume):
        # 95A3E6 / 95A436: mov ecx, [ecx+0x284]
        actor = mm.u.reg_read(UC_X86_REG_ECX)
        mm.u.reg_write(UC_X86_REG_ECX, self._owned_head(mm, actor))
        mm.u.reg_write(UC_X86_REG_EIP, resume)

    def _create_army(self, mm):
        desc = mm.arg(0)
        data = mm.r32(desc + 4)
        self._orig_create(mm)
        self.sync_created()
        a = len(self.created) - 1
        self.log.append(['create_army', a, self.thing_ids[data],
                         self.aindex[self.w.army_owner[self.created[a]['actor']]]])

    def sync_created(self):
        for i in range(len(self.comps), len(self.created)):
            a = self.created[i]['actor']
            self.comps.append(self.m.r32(a + 0xcc))
            self.army_index[a] = i
            o = self.m.alloc(0x10)
            self.m.w32(o, self.vt_busy)
            self.m.w32(a + 0x70, o)
            self.busy_obj[o] = i

    def _observe_event(self, u, pc, size, _):
        self.cur_event_state = u.reg_read(UC_X86_REG_ESI)

    def _observe_effect(self, u, pc, size, _):
        self.cur_effect = u.reg_read(UC_X86_REG_ECX)

    def _message_seam(self, end, what):
        """Skip a localized message block, recording its formatting arguments
        (the objects the block reads, as indices / IDS)."""
        def seam(mm):
            from unicorn.x86_const import UC_X86_REG_EBX, UC_X86_REG_EDI, UC_X86_REG_EBP
            reg = mm.u.reg_read
            e = self.event_objs.index(self.m.r32(self.cur_event_state + 8))
            mk = lambda p: None if p == 0 else self.kindex[p]
            if what == 'event':
                # 938B70(current actor) -> 95A8D0: current kingdom name
                self.log.append(['event_message', e, mk(self.m.r32(self.st + 0x10))])
            else:
                i = self.effect_objs[e].index(self.cur_effect)
                if what == 'add_actor':
                    args = [self.army_index[reg(UC_X86_REG_ESI)], mk(reg(UC_X86_REG_EBX)), self.tindex[reg(UC_X86_REG_EDI)]]
                elif what == 'remove_actor':
                    t = self.m.r32(mm.esp() + 0x10)
                    args = [self.army_index[reg(UC_X86_REG_ESI)], mk(reg(UC_X86_REG_EBP)), self.tindex[t]]
                elif what in ('add_tech', 'remove_tech'):
                    args = [mk(reg(UC_X86_REG_EDI)), self.tech_ids[reg(UC_X86_REG_ESI)]]
                else:  # change_territory: target, territory, source (95A8D0(source))
                    args = [mk(reg(UC_X86_REG_EDI)), self.tindex[reg(UC_X86_REG_ESI)], mk(reg(UC_X86_REG_EBX))]
                    if reg(UC_X86_REG_EBX) == 0:
                        # 95A8D0(NULL) dereferences NULL: a native crash
                        self.crashed = '95B9B0 message with no source kingdom: 95A8D0(NULL)'
                self.log.append(['effect_message', e, i, what] + args)
            mm.u.reg_write(UC_X86_REG_EIP, end)
        return seam

    def _end_turn_message(self, mm):
        # 952790 block 9527CB..95288D: localized end-turn message (938B70 + 95A8D0 + 40BE60)
        actor = mm.r32(mm.esp() + 0x20)
        k = next((i for i, kp in enumerate(self.kingdoms) if self.w.resolve(mm.r32(kp + 0xc)) == actor), None)
        assert k is not None, '95A8D0 on a null MG kingdom'
        self.log.append(['end_turn_message', k])
        mm.u.reg_write(UC_X86_REG_EIP, 0x95288d)

    # ---------------------------------------------------------------- state
    def waypoints(self, a):
        out = []
        node = self.m.r32(self.comps[a] + 0x30)
        while node:
            v = self.m.r32(node)
            out.append(None if v == 0 else self.tindex[v])
            node = self.m.r32(node + 4)
        return out

    def army_territory(self, a):
        p = self.m.r32(self.comps[a] + 0x20)
        return None if p == 0 else self.tindex[p]

    def path(self, a):
        return [self.tindex[t] for t in self.w.read_vector(self.comps[a] + 0x24)]

    def owner(self, a):
        return self.aindex[self.w.army_owner[self.created[a]['actor']]]

    def dump(self):
        m, w, st = self.m, self.w, self.st
        cur = m.r32(st + 0x10)
        out = {
            'current': None if cur == 0 else self.kindex[cur],
            'round': m.r32(st + 0x14),
            'rng': m.r32(st + 0x18),
            'flags': [m.u.mem_read(st + o, 1)[0] for o in (0x44, 0x45, 0x46, 0x50)],
            'results': [[self.tindex[m.r32(e + 4)], self._k(m.r32(e + 8)), self._k(m.r32(e + 0xc)), m.u.mem_read(e + 0x10, 1)[0]]
                        for e in w.read_vector(st + 0x60)],
            'kingdoms': [[struct.unpack('<i', struct.pack('<I', m.r32(kp + 0x38)))[0],
                          struct.unpack('<i', struct.pack('<I', m.r32(kp + 0x3c)))[0],
                          [self.tindex[t] for t in w.read_vector(kp + 0x14)],
                          [self.tech_ids[t] for t in w.read_vector(kp + 0x20)]] for kp in self.kingdoms],
            # MGState +0x54 event states (95AD60): active byte +0xC, kingdom data +0x10
            'events': [[m.u.mem_read(es + 0xc, 1)[0], self._kd(m.r32(es + 0x10))]
                       for es in w.read_vector(st + 0x54)],
            'territories': [],
            'armies': [],
        }
        for tp in self.territories:
            refs = []
            for r in w.read_vector(tp + 0x10):
                refs.append([self.army_index[w.resolve(m.r32(r + 4))], m.r32(r + 8)])
            owner = m.r32(tp + 0x24)
            out['territories'].append([None if owner == 0 else self.kindex[owner],
                                       struct.unpack('<i', struct.pack('<I', m.r32(tp + 0x1c)))[0], refs])
        for a in range(len(self.created)):
            if a in self.killed:
                out['armies'].append(None)
                continue
            out['armies'].append([self.owner(a), self.army_territory(a),
                                  struct.unpack('<i', struct.pack('<I', m.r32(self.comps[a] + 0x10)))[0],
                                  self.path(a), self.waypoints(a)])
        return out

    # ---------------------------------------------------------------- perturbations (recorded)
    def op(self, *o):
        self.ops.append(list(o))

    def move(self, a, t):
        self.m.call(0x940100, args=(self.territories[t],), ecx=self.comps[a])
        self.op('move', a, t)

    def set_territory_owner(self, t, k):
        tp = self.territories[t]
        if k is None:
            # 955620(NULL) dereferences NULL in 95A850 (a native crash; it
            # passed silently while page 0 was readable). Synthetic setup of
            # the unowned state InitCreate leaves: 959F20 removal from the old
            # owner, owner NULL, 954CF0 recount (Rust MgState::clear_owner).
            old = self.m.r32(tp + 0x24)
            if old:
                self.m.call(0x959f20, args=(tp,), ecx=old)
            self.m.w32(tp + 0x24, 0)
            self.m.call(0x954cf0, ecx=tp)
        else:
            self.m.call(0x955620, args=(self.kingdoms[k],), ecx=tp)
        self.op('owner', t, k)

    def set_fought(self, a, v):
        self.m.w32(self.comps[a] + 0x10, v); self.op('fought', a, v)

    def set_battles(self, k, v):
        self.m.w32(self.kingdoms[k] + 0x3c, v); self.op('battles', k, v)

    def set_resources(self, k, v):
        self.m.w32(self.kingdoms[k] + 0x38, v); self.op('resources', k, v)

    def set_current(self, k):
        self.m.w32(self.st + 0x10, self.kingdoms[k]); self.op('current', k)

    def set_rng(self, v):
        self.m.w32(self.st + 0x18, v); self.op('rng', v)

    def set_game_over(self, v):
        self.m.u.mem_write(self.st + 0x50, bytes([v])); self.op('game_over', v)

    def set_behaviours(self, att, dfd):
        self.m.w32(self.campaign + 0x78, att); self.m.w32(self.campaign + 0x7c, dfd)
        self.op('behaviours', BEHAVIOURS[att], BEHAVIOURS[dfd])

    def set_field19c(self, k, v):
        self.m.w32(self.actors[k] + 0x19c, v); self.op('field19c', k, v)

    def set_busy(self, a, v):
        self.busy[a] = v; self.op('busy', a, v)

    def push_waypoint(self, a, t):
        self.m.w32(self.cell, self.territories[t])
        self.m.call(0x45f240, args=(self.cell,), ecx=self.comps[a] + 0x30)
        self.op('push_waypoint', a, t)

    def set_technologies(self, k, ids):
        # MG kingdom +0x20 owned technologies (shared technology objects by IDS)
        self.w.vector(self.kingdoms[k] + 0x20, [self.ctx['techs'][i] for i in ids])
        self.op('technologies', k, list(ids))

    def set_army_owner(self, a, k):
        # engine-side owner change (army vtable+160 seam), then the original recount 954CF0
        self.w.army_owner[self.created[a]['actor']] = self.actors[k]
        t = self.army_territory(a)
        self.m.call(0x954cf0, ecx=self.territories[t])
        self.op('army_owner', a, k)

    # ---------------------------------------------------------------- operations under test
    def record(self, kind, args, result, **extra):
        case = {'op': [kind] + list(args), 'result': result, 'log': self.log, 'state': self.dump()}
        case.update(extra)
        self.log = []
        self.ops.append(case)
        return case

    def resolve(self, att, dfd, t, won):
        self.log = []
        eax, _ = self.m.call(0x954130, args=(0 if att is None else self.kingdoms[att],
                                             0 if dfd is None else self.kingdoms[dfd],
                                             self.territories[t], won), count=50_000_000)
        return self.record('resolve', (att, dfd, t, won), eax & 0xff)

    def quick(self):
        self.log = []
        eax, _ = self.m.call(0x954220, args=(0,), count=50_000_000)
        return self.record('quick', (), eax & 0xff)

    def step(self, a):
        self.log = []
        self.m.call(0x940020, args=(0, 0), ecx=self.comps[a] + 8, count=5_000_000)
        return self.record('step', (a,), None)

    def order(self, a, t):
        self.log = []
        eax, _ = self.m.call(0x93f550, args=(self.territories[t],), ecx=self.comps[a])
        return self.record('order', (a, t), eax & 0xff)

    def end_turn(self, k):
        self.log = []
        eax, _ = self.m.call(0x952790, args=(self.actors[k],), count=200_000_000)
        return self.record('end_turn', (k,), eax & 0xff)

    def advance(self):
        self.log = []
        self.m.call(0x938c90, ecx=self.st, count=200_000_000)
        return self.record('advance', (), None)

    def advance_playable(self):
        self.log = []
        self.m.call(0x9391f0, ecx=self.st, count=200_000_000)
        return self.record('advance_playable', (), None)

    # ---------------------------------------------------------------- queries (not recorded)
    def live(self):
        return [a for a in range(len(self.created)) if a not in self.killed]

    def kingdom_of_army(self, a):
        actor = self.w.army_owner[self.created[a]['actor']]
        return next((i for i, kp in enumerate(self.kingdoms) if self.w.resolve(self.m.r32(kp + 0xc)) == actor), None)

    def tmeta(self, t):
        tp = self.territories[t]
        owner = self.m.r32(tp + 0x24)
        refs = [self.army_index[self.w.resolve(self.m.r32(r + 4))] for r in self.w.read_vector(tp + 0x10)]
        return (None if owner == 0 else self.kindex[owner]), refs, self.m.r32(tp + 0x1c)

    def attackers(self, t):
        owner, refs, _ = self.tmeta(t)
        if owner is None:
            return refs
        return [a for a in refs if self.enemy[owner][self.owner(a)]]

    def links(self, t):
        d = self.m.r32(self.territories[t] + 0xc)
        defs = self.ctx['defs']
        out = []
        for lo in self.w.read_vector(d + 0x68):
            target = self.m.r32(lo + 8)
            out.append(next(i for i, tp in enumerate(self.territories) if self.m.r32(tp + 0xc) == target))
        return out

    def movement(self, a):
        ids = self.thing_ids[self.created[a]['data']]
        return next(x['movement'] for x in DATA['armies'] if x['ids'] == ids)


def liberation_territories(ci):
    c = DATA['campaigns'][ci]
    out = []
    for k in c['kingdoms']:
        for r in k['territories']:
            if r['liberates_to'] is not None:
                out.append(r['territory'])
    return out


def setup_battle(h, rng, ci):
    """Move armies of an attacking kingdom into a target territory."""
    n = len(h.kingdoms)
    lib_defs = liberation_territories(ci)
    tdefs = [h.m.r32(tp + 0xc) for tp in h.territories]
    defs = h.ctx['defs']
    roll = rng.random()
    if roll < 0.25 and lib_defs:
        d = rng.choice(lib_defs)
        t = next(i for i, x in enumerate(tdefs) if x == defs[d])
    elif roll < 0.6:
        # a territory holding armies (contested candidates)
        cands = [t for t in range(len(h.territories)) if h.tmeta(t)[1]]
        t = rng.choice(cands) if cands else rng.randrange(len(h.territories))
    else:
        t = rng.randrange(len(h.territories))
    owner, refs, _ = h.tmeta(t)
    # optionally hand the territory to another kingdom first
    if rng.random() < 0.3:
        h.set_territory_owner(t, rng.randrange(n) if rng.random() < 0.9 else None)
        owner = h.tmeta(t)[0]
    # attacker kingdom: an enemy of the owner when possible
    if owner is not None and rng.random() < 0.85:
        enemies = [k for k in range(n) if h.enemy[owner][k]]
        ak = rng.choice(enemies) if enemies else rng.randrange(n)
    else:
        ak = rng.randrange(n)
    live = h.live()
    own = [a for a in live if h.owner(a) == ak and h.army_territory(a) != t]
    mobile = [a for a in own if h.movement(a) != 0]
    pool = mobile if (mobile and rng.random() < 0.8) else own
    for a in rng.sample(pool, min(len(pool), rng.choice([0, 1, 1, 2, 2, 3]))):
        # come from a neighbour so the path holds a previous territory
        if rng.random() < 0.5:
            links = h.links(t)
            if links:
                h.move(a, rng.choice(links))
        h.move(a, t)
    # extra defenders sometimes
    if owner is not None and rng.random() < 0.3:
        friends = [a for a in live if not h.enemy[owner][h.owner(a)] and h.army_territory(a) != t]
        for a in rng.sample(friends, min(len(friends), rng.randint(1, 3))):
            h.move(a, t)
    return t, ak


def perturb(h, rng):
    n = len(h.kingdoms)
    live = h.live()
    for _ in range(rng.randint(0, 3)):
        a = rng.choice(live)
        if rng.random() < 0.5:
            links = h.links(h.army_territory(a))
            t = rng.choice(links) if links else rng.randrange(len(h.territories))
        else:
            t = rng.randrange(len(h.territories))
        h.move(a, t)
    if rng.random() < 0.3:
        h.set_territory_owner(rng.randrange(len(h.territories)), rng.randrange(n))
    for _ in range(rng.randint(0, 2)):
        h.set_fought(rng.choice(live), rng.choice([0, 1, 2]))
    if rng.random() < 0.3:
        h.set_battles(rng.randrange(n), rng.choice([0, 1, 5]))
    if rng.random() < 0.2:
        h.set_rng(rng.getrandbits(32))
    if rng.random() < 0.2:
        h.set_game_over(0)
    if rng.random() < 0.1:
        h.set_field19c(rng.randrange(n), rng.choice([0, 1, 2]))
    if rng.random() < 0.08 and h.ctx['techs']:
        names = sorted(h.ctx['techs'])
        h.set_technologies(rng.randrange(n), rng.sample(names, min(len(names), rng.randint(0, 4))))


def run_world(ci, pc, rng, relation_mode, n_ops):
    h = Harness(ci, PCS[pc], rng, relation_mode)
    n = len(h.kingdoms)
    stats = {}
    for step in range(n_ops):
        perturb(h, rng)
        kind = rng.choices(['resolve', 'quick', 'step', 'order'], [0.6, 0.15, 0.17, 0.08])[0]
        if kind == 'quick':
            pending, _ = h.m.call(0x93a320, ecx=h.st)
            if pending == 0:
                kind = 'resolve'
        if kind in ('resolve', 'quick'):
            if kind == 'resolve':
                t, ak = setup_battle(h, rng, ci)
            else:
                # quick battles also get random destroy behaviours
                pass
            h.set_behaviours(rng.randrange(8), rng.randrange(8))
            if kind == 'quick':
                t = h.tindex[pending]
                if rng.random() < 0.97:
                    give_previous_territory(h, rng, t)
                    pending, _ = h.m.call(0x93a320, ecx=h.st)
                    if pending == 0:
                        continue
                case = h.quick()
            else:
                owner, refs, _ = h.tmeta(t)
                atts = h.attackers(t)
                att = h.kingdom_of_army(atts[0]) if atts else None
                if att is None or rng.random() < 0.1:
                    att = ak
                won = 1 if rng.random() < 0.55 else 0
                # ProcessDefenderWin pushes 93EB50 = 0 (null waypoint) for attackers with path <= 1
                null_order = not won and any(len(h.path(a)) <= 1 for a in atts)
                if null_order and rng.random() < 0.85:
                    give_previous_territory(h, rng, t)
                case = h.resolve(att, owner, t, won)
        elif kind == 'step':
            live = h.live()
            with_wp = [a for a in live if h.waypoints(a)]
            if not with_wp or rng.random() < 0.3:
                a = rng.choice(live)
                for _ in range(rng.randint(1, 3)):
                    t = rng.choice(h.links(h.army_territory(a)) or [rng.randrange(len(h.territories))])
                    h.push_waypoint(a, t)
                with_wp = [a]
            a = rng.choice(with_wp)
            if None in h.waypoints(a):
                continue
            if rng.random() < 0.1:
                h.set_busy(a, 1)
            elif h.busy.get(a):
                h.set_busy(a, 0)
            h.step(a)
        else:
            a = rng.choice(h.live())
            h.order(a, rng.randrange(len(h.territories)))
        last = h.ops[-1]
        if isinstance(last, dict):
            stats[last['op'][0]] = stats.get(last['op'][0], 0) + 1
            if end_on_null_order(h, last):
                stats['null_order'] = stats.get('null_order', 0) + 1
                break
    return h, stats


def give_previous_territory(h, rng, t):
    """Attackers of t whose path has at most one entry (path [t]: walking out
    and back in truncates it again, 93F9F0) leave t for a linked territory,
    so ProcessDefenderWin (93EB50) finds a previous territory for the rest."""
    links = h.links(t)
    if not links:
        return
    for a in h.attackers(t):
        if len(h.path(a)) <= 1:
            h.move(a, rng.choice(links))


def end_on_null_order(h, case):
    """9673F0 pushes 93EB50 = 0 (null waypoint) for an attacker with a path of
    at most one entry; 940020 would dereference it. MgArmy cannot hold it, so
    the Rust port reports Unsupported: the world ends with that case."""
    if any(None in h.waypoints(a) for a in h.live()):
        case['null_order'] = True
        return True
    return False


def world_record(h, pc, relation_mode):
    return {'campaign': h.ci, 'precision': pc, 'relations': relation_mode,
            'enemy_140': h.enemy, 'relation_138': h.r138, 'events': h.events_json, 'ops': h.ops}


def main(small=False):
    rng = random.Random(0x954130)
    worlds = []
    totals = {}
    plan = []
    for ci in range(len(DATA['campaigns'])):
        for v in range(6):
            plan.append((ci, 'double53', 'random' if v % 2 else 'teams', 45))
    # other precisions (subset)
    for ci in range(len(DATA['campaigns'])):
        plan.append((ci, 'single24', 'random' if ci % 2 else 'teams', 30))
        plan.append((ci, 'extended64', 'teams' if ci % 2 else 'random', 20))
    if small:
        plan = [(0, 'double53', 'teams', 25), (1, 'single24', 'random', 25)]
    for ci, pc, mode, n_ops in plan:
        h, stats = run_world(ci, pc, rng, mode, n_ops)
        for k, v in stats.items():
            totals[k] = totals.get(k, 0) + v
        worlds.append(world_record(h, pc, mode))
    report = {
        'schema': 1, 'scope': __doc__,
        'original_sha256': hashlib.sha256(P.b).hexdigest(), 'v14_sha256': hashlib.sha256(V.b).hexdigest(),
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'data_sha256': hashlib.sha256((HERE / 'metagame-data.json').read_bytes()).hexdigest(),
        'worlds': worlds,
        'branch_coverage': dict(sorted(TRACE.items())),
        'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())],
    }
    out = HERE / 'native-resolve.json'
    out.write_text(json.dumps(report, separators=(',', ':')) + '\n')
    print(json.dumps({'worlds': len(worlds), 'cases': totals, 'visited': len(VISITED), 'trace': dict(sorted(TRACE.items())),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    import sys
    main(small='small' in sys.argv[1:])
