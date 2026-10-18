"""Original MGAI controller under Unicorn: the fiber body 945090 (players
944F90 / 963430, PlayTurn 964D30 of the current kingdom, turn-ended resets,
player deletion 944D20 path), the yield 944CE0, the command queue 944F50 and
its flush 944E60, and the execution of the flushed orders by the original
order execution 9505F0 (9525A0 / 951D70, KKC_MGCommand::Process 954570 with
move/attack 9537B0, startbattle 954440, quickbattle 954220 -> resolution
954130, endturn 952790 -> 9391F0 advance, purchases 9528B0 / 952A00,
upgrade 952D10) plus the per-tick waypoint mover 940020, on perturbed
InitCreate worlds of every shipped campaign.

The engine around the fiber is scripted here and recorded (fixture 'yields'):
- SAI scheduler 79DFF0(arg) (seam): a call with arg != 0 always switches to
  the main fiber; arg 0 switches with a scripted probability (time budget).
  On every switch the main fiber's work runs (emulation paused, CPU context
  saved, nested calls on a stack below the paused frames, context restored):
  tick advance ([B36534]+0xDC), the ORIGINAL flush 944E60 (order submission
  950770/9506D0 = seam: the command is taken over and recorded), then each
  flushed order executed by the ORIGINAL 9505F0 with the current kingdom's
  actor as issuer (the order source player [[B36558]+0x284] read by 74B3B0
  is a synthetic TGC player whose +0x28 is that actor; 765E00 runs), then scripted engine orders through 9505F0 (a pending
  battle started / quick-resolved by the current kingdom, a human kingdom's
  endturn), then 940020 for some armies, then actor handles of killed /
  replaced armies are invalidated and the kingdom actor army lists (+0x284)
  are rebuilt (surviving entries keep their order, a respawned actor takes
  its predecessor's place, new armies are appended).
  'stop' ends the run inside 79DFF0 (the fiber is never resumed).
- MGState replacement: rarely a switch sets [B49FA0] to a dummy object after
  the main-fiber work and restores it at the next 945090 pass (an identical
  state 'replaced'), which drives PlayTurn aborts and player deletion.
- 76B960 (TGC player of a kingdom actor) = seam: per kingdom no player
  (computer), a computer player (+0x0C = 1) or a human player (+0x0C = 0).
- 4F5290 (world query, result unused by 944CE0 / 945090) = seam.

Seams of command processing / resolution (as native_process.py and
native_resolve.py): 40BE60 message post, 4B9F40 localisation, 95A8D0
kingdom name, 4F8F00 world kingdom lookup, 4D5EA0 actor hand-over, 4F6760
actor destroy, army vtable +0x24 / +0x134 (owner == issuer) / +0x160 (set
owner), 40FCA0 cheat gate (off), 4D7960 kill, 953A20 battle messages,
4E9720 kingdom victory, 50D370 game over, 9626D0 / 962880 move visuals,
9527CB..95288D end-turn message block, 938DD8 (938DA0's walk of the
kingdom actor list: built at call time from live armies by owner).
The MGAI seams of native_mgai_world.py stay (busy, 96A5D0 block, app RNG).
"""
import json, hashlib, random, sys, time, struct
from pathlib import Path
import mg_machine
from native_mgai_world import MgaiWorld, BLOCKS, DATA, DIFFS
from native_mgai import ai_values, Case, STAGES, CW, DATA_SHA256
from mg_machine import P, V, HERE
from mg_world import VT_ARMY, native_wstring, read_wstring
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import (UC_X86_REG_ECX, UC_X86_REG_ESI, UC_X86_REG_EIP, UC_X86_REG_ESP, UC_X86_REG_EAX,
                               UC_X86_REG_EBP, UC_X86_REG_FPCW, UC_X86_REG_FPSW, UC_X86_REG_FPTAG)

BEHAVIOURS = ['None', 'First', 'Weakest', 'Strongest', 'LeastExpensive', 'MostExpensive', 'Random', 'All']


def s32(v):
    return v - ((v >> 31) << 32)


class ControllerWorld(MgaiWorld):
    def __init__(self, ci, cw, rng):
        super().__init__(ci, cw=cw)
        m, w, st = self.m, self.w, self.st
        self.rng = rng
        self.case = Case(self)                  # record labels / dumps (its own observers stay idle)
        self.camp_obj = m.r32(st + 8)
        # quick battle settings (95F270/95F370 via B39AB0 +208/+20C)
        settings = m.alloc(0x300)
        bonus = m.alloc(4); m.w32(bonus, DATA['settings']['technology_bonus_bits'])
        prob = m.alloc(4); m.w32(prob, DATA['settings']['attack_win_probability_bits'])
        m.w32(settings + 0x208, bonus); m.w32(settings + 0x20c, prob)
        m.w32(0xb39ab0, settings)
        c = DATA['campaigns'][ci]
        m.w32(self.camp_obj + 0x78, BEHAVIOURS.index(c['attacking_destroy_behavior']))
        m.w32(self.camp_obj + 0x7c, BEHAVIOURS.index(c['defending_destroy_behavior']))
        for a in self.created:                  # waypoint list {head, tail} empty
            comp = m.r32(a['actor'] + 0xcc)
            m.w32(comp + 0x30, 0); m.w32(comp + 0x34, 0)
        self.killed, self.destroyed = set(), set()
        self.replaced_by = {}                   # old army index -> new army index (respawn)
        self.engine_log = []
        self.messages = []
        self._install_processing_seams()
        # controller (944D60), SAI, net node, order source
        self.controller = m.alloc(0x40)
        m.call(0x944d60, ecx=self.controller)
        self.sai = m.alloc(0x80)
        m.w32(0xb36550, m.alloc(0x100))
        self.order_node = m.alloc(0x400)       # [B36558]: 74B3B0 returns its +0x284 (order source player)
        m.w32(0xb36558, self.order_node)
        self.source_player = m.alloc(0x40)
        m.w32(self.order_node + 0x284, self.source_player)
        self.tgc = {}                           # kingdom index -> 'none' | 'computer' | 'human'
        self.tgc_obj = {}
        m.seams[0x79dff0] = self._sai_yield
        # The original yield 944CE0 runs here (it calls the 79DFF0 seam); the
        # scripted round-2 yield seam of MgaiWorld would never advance the
        # tick, so DoMovement 963030 would wait forever.
        del m.seams[0x944ce0]
        # Likewise the original queue append 944F50 runs (MgaiWorld's capture
        # seam would keep the commands out of the controller queue).
        del m.seams[0x944f50]
        m.seams[0x4f5290] = lambda mm: mm.ret(0, 0)
        m.seams[0x76b960] = self._player_of
        m.seams[0x950770] = self._submit
        self.dummy = m.alloc(0x100)
        self.dummy_active = False
        m.u.hook_add(UC_HOOK_CODE, self._after_delete, begin=0x94519f, end=0x94519f)
        for addr, name in STAGES.items():
            m.u.hook_add(UC_HOOK_CODE, self._observe(name), begin=addr, end=addr)
        self.stage_log = []
        self.yields = []
        self.n_loops = 0
        self.n_yields = 0
        self.pause = None
        self.captured = []

    # ---------------------------------------------------------------- seams: processing / resolution
    def _install_processing_seams(self):
        m, w = self.m, self.w
        def vf134(mm):
            mm.ret(4, int(w.army_owner[mm.u.reg_read(UC_X86_REG_ECX)] == mm.arg(0)))
        w.virtual(VT_ARMY, 0x134, vf134)
        def name(mm):
            out = mm.arg(0); mm.w32(out, native_wstring(mm, '')); mm.ret(4, out)
        w.virtual(VT_ARMY, 0x24, name)
        def set_owner(mm):
            this = mm.u.reg_read(UC_X86_REG_ECX)
            owner = mm.arg(0)
            w.army_owner[this] = owner
            self.engine_log.append(['set_owner', w.kind[this][1], self.actors.index(owner)])
            mm.ret(4, 1)
        w.virtual(VT_ARMY, 0x160, set_owner)
        m.seams[0x40be60] = lambda mm: (self.messages.append(mm.arg(1)), mm.ret(0))
        def localize(mm):
            out = mm.arg(0); mm.w32(out, native_wstring(mm, '')); mm.ret(8, out)
        m.seams[0x4b9f40] = localize
        m.seams[0x95a8d0] = lambda mm: (mm.w32(mm.arg(0), native_wstring(mm, '')), mm.ret(4, mm.arg(0)))
        ids_actor = {k['ids']: self.actors[i] for i, k in enumerate(self.camp['kingdoms'])}
        m.seams[0x4f8f00] = lambda mm: mm.ret(4, ids_actor.get(read_wstring(mm, mm.r32(mm.arg(0))), 0))
        m.seams[0x4d5ea0] = lambda mm: mm.ret(4)
        def destroy(mm):
            a = mm.arg(0)
            self.destroyed.add(w.kind[a][1])
            mm.w32(a + 8, mm.r32(a + 8) | 1)
            mm.ret(8)
        m.seams[0x4f6760] = destroy
        m.seams[0x40fca0] = lambda mm: mm.ret(8, 0)
        def kill(mm):
            this = mm.u.reg_read(UC_X86_REG_ECX)
            a = w.kind[this][1]
            assert a not in self.killed, 'army killed twice'
            self.killed.add(a)
            self.engine_log.append(['kill', a])
            mm.ret(0, 1)
        m.seams[0x4d7960] = kill
        def k_index(p):
            return None if p == 0 else self.K.index(p)
        def messages(mm):
            att, dfd, t, won = mm.arg(0), mm.arg(1), mm.arg(2), mm.arg(3) & 0xff
            assert att != 0, '953A20 dereferences the attacker kingdom'
            self.engine_log.append(['messages', k_index(att), k_index(dfd), self.T.index(t), won])
            mm.ret(0)
        m.seams[0x953a20] = messages
        def victory(mm):
            this = mm.u.reg_read(UC_X86_REG_ECX)
            assert (mm.arg(0), mm.arg(1)) == (1, 1)
            self.engine_log.append(['victory', self.actors.index(this)])
            mm.ret(8)
        m.seams[0x4e9720] = victory
        m.seams[0x50d370] = lambda mm: (self.engine_log.append(['game_over']), mm.ret(0))
        m.seams[0x9626d0] = lambda mm: mm.ret(8, 0)
        def visual(mm):
            actor, cur, nxt, flag = mm.arg(0), mm.arg(1), mm.arg(2), mm.arg(3)
            assert nxt != 0 and flag == 0
            self.engine_log.append(['move_visual', w.kind[actor][1], None if cur == 0 else self.T.index(cur), self.T.index(nxt)])
            mm.ret(16, 1)
        m.seams[0x962880] = visual
        def end_turn_message(mm):
            actor = mm.r32(mm.esp() + 0x20)
            k = next((i for i, kp in enumerate(self.K) if w.resolve(mm.r32(kp + 0xc)) == actor), None)
            assert k is not None, '95A8D0 on a null MG kingdom'
            self.engine_log.append(['end_turn_message', k])
            mm.u.reg_write(UC_X86_REG_EIP, 0x95288d)
        m.seams[0x9527cb] = end_turn_message
        def owned_list(mm):
            actor = mm.u.reg_read(UC_X86_REG_ECX)
            head = 0
            for i in reversed(range(len(self.created))):
                if not self.alive(i) or w.army_owner[self.created[i]['actor']] != actor:
                    continue
                node = mm.alloc(8)
                mm.w32(node, self.created[i]['actor']); mm.w32(node + 4, head)
                head = node
            mm.u.reg_write(UC_X86_REG_ESI, head)
            mm.u.reg_write(UC_X86_REG_EIP, 0x938dde)
        m.seams[0x938dd8] = owned_list

    def alive(self, i):
        return i not in self.killed and i not in self.destroyed

    # ---------------------------------------------------------------- seams: controller engine
    def _player_of(self, mm):
        actor = mm.arg(0)
        k = self.actors.index(actor) if actor in self.actors else None
        kind = self.tgc.get(k, 'none')
        if kind == 'none':
            mm.ret(4, 0)
            return
        if k not in self.tgc_obj:
            o = mm.alloc(0x40)
            mm.w32(o + 0x28, actor)
            self.tgc_obj[k] = o
        o = self.tgc_obj[k]
        mm.u.mem_write(o + 0xc, bytes([1 if kind == 'computer' else 0]))
        mm.ret(4, o)

    def _submit(self, mm):
        # 950770 -> 9506D0 (network order): take the command over, record it
        wrapper = mm.u.reg_read(UC_X86_REG_ECX)
        cmd = mm.r32(wrapper + 4)
        mm.w32(wrapper + 4, 0)
        self.captured.append(cmd)
        mm.ret(4)

    def _sai_yield(self, mm):
        arg = mm.arg(0) & 0xff
        site = mm.r32(mm.esp())
        self.n_yields += 1
        if PROGRESS:
            print(f'  yield {self.n_yields} site {site:#x} arg {arg} loops {self.n_loops} t={time.time():.0f}', flush=True)
        loop = site == 0x945161
        if loop:
            self.n_loops += 1
        rec = {'arg': arg, 'loop': int(loop)}
        mm.ret(4)
        if (loop and self.n_loops > self.max_loops) or self.n_yields > self.max_yields:
            rec['stop'] = 1
            self.yields.append(rec)
            self.pause = ('stop', rec)
            mm.u.emu_stop()
            return
        sc = self.script
        switched = arg != 0 or sc.random() < self.p_switch0
        rec['switched'] = int(switched)
        restore = loop and self.dummy_active
        set_dummy = switched and not restore and sc.random() < self.p_replace
        rec['replaced'] = int(restore or set_dummy)
        rec['set_dummy'] = int(set_dummy)
        self.yields.append(rec)
        if switched:
            self.pause = ('switch', rec)
            mm.u.emu_stop()

    def _after_delete(self, u, pc, size, _):
        if self.dummy_active:
            self.m.w32(0xb49fa0, self.st)
            self.dummy_active = False

    def _observe(self, name):
        def hook(u, pc, size, _):
            player = u.reg_read(UC_X86_REG_ESI)
            players = self.w.read_vector(self.controller + 0x14)
            k = players.index(player)
            self.stage_log.append({'yield': len(self.yields), 'kingdom': k, 'stage': name,
                                   'analysis': self.case.analysis(player) if name == 'analysis' else None,
                                   'goals': self.case.goals(player),
                                   'flags': [self.m.u.mem_read(player + o, 1)[0] for o in (0x28, 0x29)]})
        return hook

    # ---------------------------------------------------------------- nested calls on the paused fiber
    def ncall(self, target, args=(), ecx=0, count=200_000_000):
        m = self.m
        start, end = self._stub(target)
        sp = (m.u.reg_read(UC_X86_REG_ESP) - 0x40000) & ~0xf
        for i, v in enumerate(args):
            m.w32(sp + 4 * i, v)
        m.u.reg_write(UC_X86_REG_ESP, sp)
        m.u.reg_write(UC_X86_REG_EBP, 0)
        m.u.reg_write(UC_X86_REG_ECX, ecx)
        m.u.reg_write(UC_X86_REG_FPCW, m.cw)
        m.u.reg_write(UC_X86_REG_FPSW, 0)
        m.u.reg_write(UC_X86_REG_FPTAG, 0xffff)
        self.pause = None
        m.u.emu_start(start, end, count=count)
        m.check_null()
        assert m.u.reg_read(UC_X86_REG_EIP) == end, hex(m.u.reg_read(UC_X86_REG_EIP))
        assert self.pause is None
        return m.u.reg_read(UC_X86_REG_EAX)

    def make_command(self, index):
        cmd = self.m.alloc(0x28)
        self.ncall(0x9510f0, ecx=cmd)
        self.ncall(0x950cb0, args=(self.cmd_data[index],), ecx=cmd)
        return cmd

    def execute(self, cmd, k, source):
        """9505F0 on an order holding `cmd`, issuer = kingdom k's actor."""
        m = self.m
        actor = self.actors[k]
        rec = {'source': source, 'issuer': k, 'command': self.read_command(cmd)}
        rec['code'] = self.ncall(0x951d70, args=(actor,), ecx=cmd)
        m.w32(self.source_player + 0x28, actor)
        order = m.alloc(8)
        m.w32(order, 0xacac20); m.w32(order + 4, cmd)
        self.messages = []
        self.engine_log = []
        n_before = len(self.created)
        destroyed_before = set(self.destroyed)
        eax = self.ncall(0x9505f0, ecx=order)
        if rec['code'] == 0:
            rec['result'] = eax & 0xff
        new = list(range(n_before, len(self.created)))
        for i in new:
            self.add_body(self.created[i]['actor'])
            comp = m.r32(self.created[i]['actor'] + 0xcc)
            assert m.r32(comp + 0x30) == 0 and m.r32(comp + 0x34) == 0
        gone = sorted(self.destroyed - destroyed_before)
        assert len(gone) <= 1 and len(gone) <= len(new)
        if gone:
            self.replaced_by[gone[0]] = new[-1]
            rec['respawned'] = [gone[0], new[-1]]
            new = new[:-1]
        rec['created'] = new
        rec['messages'] = list(self.messages)
        rec['engine'] = list(self.engine_log)
        return rec

    def pending_battle(self):
        return self.ncall(0x93a320, ecx=self.st) != 0

    def engine_switch(self, rec):
        m, st, sc = self.m, self.st, self.script
        tick = (m.r32(self.world + 0xdc) + sc.randint(0, self.dt_max)) & 0xffffffff
        m.w32(self.world + 0xdc, tick)
        rec['tick'] = tick
        before = (set(self.killed), set(self.destroyed))
        cur = self.K.index(m.r32(st + 0x10))
        self.captured = []
        self.ncall(0x944e60, ecx=self.controller)
        orders = []
        for cmd in self.captured:
            orders.append(self.execute(cmd, cur, 'ai'))
        # engine orders
        cur = self.K.index(m.r32(st + 0x10))
        flags = lambda: [m.u.mem_read(st + o, 1)[0] for o in (0x44, 0x46)]
        if self.pending_battle() and not flags()[0] and sc.random() < self.p_start:
            orders.append(self.execute(self.make_command(2), cur, 'engine'))
        if self.pending_battle() and flags()[0] and not flags()[1] and sc.random() < self.p_quick:
            orders.append(self.execute(self.make_command(5), cur, 'engine'))
        cur = self.K.index(m.r32(st + 0x10))
        if self.tgc.get(cur) == 'human' and sc.random() < self.p_human_end:
            orders.append(self.execute(self.make_command(6), cur, 'engine'))
        rec['orders'] = orders
        # mover
        steps = []
        for i in range(len(self.created)):
            if not self.alive(i) or sc.random() >= self.p_step:
                continue
            comp = m.r32(self.created[i]['actor'] + 0xcc)
            if m.r32(comp + 0x30) == 0:
                continue
            self.engine_log = []
            self.ncall(0x940020, args=(0, 0), ecx=comp + 8, count=5_000_000)
            steps.append([i, list(self.engine_log)])
        rec['steps'] = steps
        # handles of killed / replaced actors die; kingdom lists rebuilt
        for i in (self.killed - before[0]) | (self.destroyed - before[1]):
            h = m.r32(self.created[i]['actor'] + 0x14)
            m.w16(self.w.table + 4 + (h & 0xffff) * 2, 0)
        self.rebuild_lists()
        rec['lists'] = [self.lists[k] for k in range(len(self.actors))]
        if orders or steps:
            rec['digest'] = self.digest()

    def rebuild_lists(self):
        w = self.w
        orders = {}
        for k, actor in enumerate(self.actors):
            out = []
            for a in self.lists.get(k, []):
                while a in self.replaced_by:
                    a = self.replaced_by[a]
                if self.alive(a) and w.army_owner[self.created[a]['actor']] == actor and a not in out:
                    out.append(a)
            for a in range(len(self.created)):
                if self.alive(a) and w.army_owner[self.created[a]['actor']] == actor and a not in out:
                    out.append(a)
            orders[k] = out
        self.build_lists(orders)

    def digest(self):
        m, w, st = self.m, self.w, self.st
        cur = m.r32(st + 0x10)
        out = {'current': None if cur == 0 else self.K.index(cur), 'round': m.r32(st + 0x14), 'rng': m.r32(st + 0x18),
               'flags': [m.u.mem_read(st + o, 1)[0] for o in (0x44, 0x45, 0x46, 0x50)],
               'kingdoms': [[s32(m.r32(kp + 0x38)), s32(m.r32(kp + 0x3c)), [self.T.index(t) for t in w.read_vector(kp + 0x14)],
                             [self.tech_ids[t] for t in w.read_vector(kp + 0x20)]] for kp in self.K],
               'territories': [], 'armies': []}
        for tp in self.T:
            owner = m.r32(tp + 0x24)
            refs = [[w.kind[w.resolve(m.r32(r + 4)) or 0][1] if w.resolve(m.r32(r + 4)) else None, m.r32(r + 8)]
                    for r in w.read_vector(tp + 0x10)]
            out['territories'].append([None if owner == 0 else self.K.index(owner), s32(m.r32(tp + 0x1c)), refs])
        for i, a in enumerate(self.created):
            if not self.alive(i):
                out['armies'].append(None)
                continue
            comp = m.r32(a['actor'] + 0xcc)
            wp, node = [], m.r32(comp + 0x30)
            while node:
                wp.append(self.T.index(m.r32(node))); node = m.r32(node + 4)
            t = m.r32(comp + 0x20)
            out['armies'].append([self.actors.index(w.army_owner[a['actor']]), None if t == 0 else self.T.index(t),
                                  s32(m.r32(comp + 0x10)), [self.T.index(x) for x in w.read_vector(comp + 0x24)], wp,
                                  [self.divisions.index(d) for d in w.read_vector(comp + 0x14)],
                                  self.thing_ids[m.r32(a['actor'] + 4)]])
        return out

    # ---------------------------------------------------------------- the fiber
    def run_fiber(self):
        m = self.m
        start, end = self._stub(0x945090)
        sp = mg_machine.STACK_TOP - 0x1000
        m.w32(sp, self.sai)
        m.u.reg_write(UC_X86_REG_ESP, sp)
        m.u.reg_write(UC_X86_REG_EBP, 0)
        m.u.reg_write(UC_X86_REG_ECX, self.controller)
        m.u.reg_write(UC_X86_REG_FPCW, m.cw)
        m.u.reg_write(UC_X86_REG_FPSW, 0)
        m.u.reg_write(UC_X86_REG_FPTAG, 0xffff)
        pc = start
        while True:
            self.pause = None
            m.u.emu_start(pc, end, count=4_000_000_000)
            m.check_null()
            pc = m.u.reg_read(UC_X86_REG_EIP)
            assert pc != end and self.pause is not None, hex(pc)
            kind, rec = self.pause
            if kind == 'stop':
                return
            ctx = m.u.context_save()
            if self.dummy_active:
                m.w32(0xb49fa0, self.st)           # the engine works on the (identical) live state
            self.engine_switch(rec)
            if rec['set_dummy']:
                self.dummy_active = True
            elif rec['replaced']:                  # restore at a 945090 pass
                self.dummy_active = False
            m.w32(0xb49fa0, self.dummy if self.dummy_active else self.st)
            m.u.context_restore(ctx)
            assert m.u.reg_read(UC_X86_REG_EIP) == pc


def perturb(W, rng, ops):
    """Initial world perturbation (recorded ops, replayed by the Rust test)."""
    m, w, st = W.m, W.w, W.st
    nk, n_t = len(W.K), len(W.T)
    for _ in range(rng.randint(0, 5)):
        t = rng.randrange(n_t); k = rng.randrange(nk)
        m.call(0x955620, args=(W.K[k],), ecx=W.T[t])
        ops.append(['owner', t, k])
    for _ in range(rng.randint(2, 10)):
        a = rng.randrange(len(W.created))
        comp = m.r32(W.created[a]['actor'] + 0xcc)
        cur = W.T.index(m.r32(comp + 0x20))
        links = w.read_vector(m.r32(W.T[cur] + 0xc) + 0x68)
        if not links:
            continue
        tdef = m.r32(rng.choice(links) + 8)
        t = next(i for i, tp in enumerate(W.T) if m.r32(tp + 0xc) == tdef)
        owner = m.r32(W.T[t] + 0x24)
        if owner and w.relations['enemy_140'][W.K.index(owner)][W.actors.index(w.army_owner[W.created[a]['actor']])]:
            continue                       # no battles at the start
        m.call(0x940100, args=(W.T[t],), ecx=comp)
        ops.append(['move', a, t])
    for a in range(len(W.created)):
        comp = m.r32(W.created[a]['actor'] + 0xcc)
        m.w32(comp + 0x10, 0)
        w.vector(comp + 0x24, [m.r32(comp + 0x20)], 4)
        ops.append(['turn_start', a])
    for k, kp in enumerate(W.K):
        r = rng.choice([0, 25, 75, 150, 300, 600])
        m.w32(kp + 0x38, r); m.w32(kp + 0x3c, 0)
        ops.append(['resources', k, r]); ops.append(['battles', k, 0])
    cur = rng.randrange(nk)
    m.w32(st + 0x10, W.K[cur]); ops.append(['current', cur])
    f51 = rng.choice([0, 1])
    m.u.mem_write(st + 0x51, bytes([f51])); ops.append(['flag51', f51])
    orders = {}
    for k, ka in enumerate(W.actors):
        own = [i for i, c in enumerate(W.created) if w.army_owner[c['actor']] == ka]
        if rng.random() < 0.5:
            rng.shuffle(own)
        orders[k] = own
    W.build_lists(orders)
    ops.append(['lists', [orders[k] for k in range(len(W.actors))]])


PROGRESS = bool(__import__('os').environ.get('MGAI_CTRL_PROGRESS'))


def main():
    ci = int(sys.argv[1])
    worlds_n = int(sys.argv[2]) if len(sys.argv) > 2 else 2
    max_loops = int(sys.argv[3]) if len(sys.argv) > 3 else 12
    seed = 0x945090 + ci
    rng = random.Random(seed)
    worlds = []
    t0 = time.time()
    for wi in range(worlds_n):
        pc = 'pc53' if wi % 3 != 2 else rng.choice(['pc24', 'pc64'])
        W = ControllerWorld(ci, CW[pc], rng)
        if wi > 0:
            W.m.stop_coverage()
        m, w, st = W.m, W.w, W.st
        camp = W.camp
        n_map = len(DATA['maps'][camp['map']]['territories'])
        ai = {}
        for k in camp['kingdoms']:
            i = k['mgai']
            if i not in ai:
                ai[i] = ai_values(i, rng, wi > 0 and rng.random() < 0.5, n_map)
                W.case.label_records(i, W.build_mgai(i, ai[i]))
        W.attach_mgai()
        ops = [['relations', w.relations['enemy_140'], w.relations['relation_138']]]
        perturb(W, rng, ops)
        nk = len(W.K)
        for k in range(nk):
            W.tgc[k] = rng.choice(['none', 'none', 'computer', 'human'] if wi > 0 else ['none', 'computer'])
        W.max_loops = max_loops
        W.max_yields = 600
        W.script = random.Random(rng.getrandbits(32))
        W.p_switch0 = rng.choice([0.1, 0.5, 1.0])
        W.p_replace = rng.choice([0, 0, 0.01, 0.03])
        W.p_start = rng.choice([0.5, 1.0])
        W.p_quick = rng.choice([0.3, 0.7, 1.0])
        W.p_human_end = rng.choice([0.3, 1.0])
        W.p_step = rng.choice([0.2, 0.5, 0.9])
        W.dt_max = rng.choice([60, 200, 500])
        rng_word = rng.getrandbits(32)
        m.w32(W.rng_word, rng_word)
        brng = random.Random(rng.getrandbits(32))
        busy_p = rng.choice([0, 0.2])
        W.busy_script = lambda idx, r=brng, b=busy_p: int(r.random() < b)
        W.uninit_rng = random.Random(rng.getrandbits(32))
        W.arrive_script = None
        W.issued = []
        W.busy_log.clear(); W.uninit_log.clear()
        tick_in = m.r32(W.world + 0xdc)
        W.run_fiber()
        players = w.read_vector(W.controller + 0x14)
        world = {'campaign': ci, 'cw': pc, 'ai': {str(k): v for k, v in ai.items()}, 'ops': ops,
                 'tgc': [W.tgc[k] for k in range(nk)], 'rng_in': rng_word, 'tick_in': tick_in,
                 'yields': W.yields, 'stages': W.stage_log, 'busy': list(W.busy_log), 'uninit': list(W.uninit_log),
                 'rng_out': m.r32(W.rng_word), 'final': W.digest(),
                 'players': [[m.u.mem_read(p + o, 1)[0] for o in (0x28, 0x29)] for p in players],
                 'queue': len(w.read_vector(W.controller + 8))}
        worlds.append(world)
        n_sw = sum(1 for y in W.yields if y.get('switched'))
        n_orders = sum(len(y.get('orders', [])) for y in W.yields)
        print(f'ci {ci} world {wi} {pc} loops {W.n_loops} yields {len(W.yields)} switches {n_sw} orders {n_orders} '
              f'stages {len(W.stage_log)} round {m.r32(st + 0x14)} t={time.time() - t0:.0f}s', flush=True)
    assert hashlib.sha256((HERE / 'metagame-data.json').read_bytes()).hexdigest() == DATA_SHA256, 'metagame-data.json changed during the run'
    report = {
        'schema': 1, 'scope': __doc__, 'seed': seed, 'campaign': ci,
        'original_sha256': hashlib.sha256(P.b).hexdigest(), 'v14_sha256': hashlib.sha256(V.b).hexdigest(),
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'world_sha256': hashlib.sha256((HERE / 'native_mgai_world.py').read_bytes()).hexdigest(),
        'mgai_sha256': hashlib.sha256((HERE / 'native_mgai.py').read_bytes()).hexdigest(),
        'data_sha256': DATA_SHA256,
        'original_v14_difference_ranges': [[hex(a), n] for a, n in DIFFS],
        'worlds': worlds,
        'blocks': [{'va': hex(a), 'bytes': b} for a, b in sorted(BLOCKS.items())],
    }
    out = HERE / f'native-mgai-controller-c{ci}.json'
    out.write_text(json.dumps(report) + '\n')
    print(json.dumps({'worlds': len(worlds), 'blocks': len(BLOCKS), 'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
