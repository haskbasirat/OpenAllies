"""Unicorn world for the original metagame AI (KKC_MGAIPlayer) oracles.

Builds, on top of mg_campaign's InitCreate world, the objects the MGAI reads
that InitCreate does not create, at their native offsets, and the seams the
MGAI calls into the engine. Nothing here claims native behaviour; it builds
inputs and reads outputs.

Objects built here (layouts read from the original code):
- MGAI data (960510 layout), one per AI definition, referenced from the
  campaign kingdom data +0x14: ints +0x24 additional_attack_occupied_importance,
  +0x28 additional_attack_unoccupied_importance, +0x2C additional_defense_importance,
  +0x30 production_importance_multiplier, +0x34 no_attack_distance,
  +0x38 max_influence_depth, +0x4C additional_border_importance,
  +0x50 additional_border_reduction_importance; binary32 +0x3C
  actor_distance_influence_multiplier, +0x40 actor_distance_strength_multiplier,
  +0x44 goal_cutoff_ratio, +0x48 attack_defendability_factor,
  +0x54 purchase_army_probability, +0x58 purchase_tech_probability;
  +0x18/+0x1C map records (960120 layout: +0x04 map object (campaign +0x18),
  +0x08/+0x0C territory records, +0x14 DefaultTerritory, +0x18 Capital,
  +0x1C FriendlyCapital, +0x20 EnemyCapital); territory record (95FD70):
  +0x04 map territory definition (95FF10 compares its +0x04 IDS with wcscmp),
  +0x08 additional_importance, +0x0C additional_neighbor_proximity_importance_depth,
  +0x10 ..._max, +0x14 required_attack_strength_ratio (binary32),
  +0x18 desired_defense_threat_ratio (binary32).
- Kingdom actor army list (actor +0x284): singly linked nodes {army actor,
  next}; engine data, built here in an explicit (recorded) order.
- Command data registry read by the command constructor 9511E0:
  [[[B36530]+0x820]+0x10]+0x24 -> array of command data objects (+0x0C
  index, +0x24 flags, +0x2C hot), the built-in values of 93BAB0.
- App/SAI RNG: [[B36528]+0x2C] -> state word (draws 85D840).
- World tick [B36534]+0xDC.
- Army actor +0x70 body object whose vtable +0x64 ("busy") is a seam.

Speed-up (no semantic change): the per-instruction Python trace hook of
mg_machine.Machine is replaced by per-address hooks for seams only. The
original/v14 identity proof is kept exactly by guard hooks over every byte
range where the original and the v14 images differ (77 ranges, 442 bytes):
any executed instruction overlapping one aborts the oracle. Executed basic
blocks are collected (block hook) for the report.
"""
import struct, random
import mg_machine
from mg_machine import P, V, HERE, Machine, STUB
from unicorn import UC_HOOK_CODE, UC_HOOK_BLOCK, UC_HOOK_MEM_UNMAPPED
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_EIP, UC_X86_REG_ESP, UC_X86_REG_EAX, UC_X86_REG_EDX

BLOCKS = {}          # executed block start -> bytes hex (coverage report)


def _diff_ranges():
    out = []
    for vs, r, size, off in P.sections:
        a = P.base + r
        pb, vb = P.range(a, size), V.range(a, size)
        i, n = 0, len(pb)
        while i < n:
            if pb[i] != vb[i]:
                j = i
                while j < n and pb[j] != vb[j]:
                    j += 1
                out.append((a + i, j - i))
                i = j
            else:
                i += 1
    return out


DIFFS = _diff_ranges()


class SeamDict(dict):
    def __init__(self, m):
        super().__init__()
        self.m = m

    def __setitem__(self, k, v):
        if k not in self:
            self.m.u.hook_add(UC_HOOK_CODE, self.m._seam_hook, begin=k, end=k)
        super().__setitem__(k, v)


class FastMachine(Machine):
    def __init__(self, control_word):
        from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
        self.u = Uc(UC_ARCH_X86, UC_MODE_32)
        self.u.mem_map(0, 0x1000)
        self.u.mem_map(P.base, (P.image_size + 4095) & ~4095)
        for vs, r, size, off in P.sections:
            self.u.mem_write(P.base + r, P.b[off:off + size])
        self.u.mem_map(mg_machine.HEAP, mg_machine.HEAP_SIZE)
        self.next = mg_machine.HEAP + 0x1000
        self.cw = control_word
        self.seams = SeamDict(self)
        self.stubs = {}
        self.seam_log = []
        self.u.hook_add(UC_HOOK_MEM_UNMAPPED, self._unmapped)
        self.fault = None
        self.last = []
        for a, n in DIFFS:
            self.u.hook_add(UC_HOOK_CODE, self._guard(a, n), begin=a - 15, end=a + n - 1)
        self.cover = self.u.hook_add(UC_HOOK_BLOCK, self._block)
        # page-0 / allocator / import guards (mg_machine.install_guards)
        mg_machine.install_guards(self)

    def _guard(self, a, n):
        def hook(u, pc, size, _):
            if pc < a + n and pc + size > a:
                raise AssertionError(f'executed instruction {pc:#x} overlaps original/v14 difference {a:#x}+{n}')
        return hook

    def _block(self, u, pc, size, _):
        if P.base <= pc < P.base + P.image_size and pc not in self.seams and pc not in BLOCKS:
            BLOCKS[pc] = P.range(pc, size).hex()

    def stop_coverage(self):
        if self.cover is not None:
            self.u.hook_del(self.cover)
            self.cover = None

    def _seam_hook(self, u, pc, size, _):
        seam = self.seams.get(pc)
        if seam is not None:
            seam(self)


import mg_campaign  # noqa: E402
mg_campaign.Machine = FastMachine
from mg_campaign import DATA, run  # noqa: E402
from mg_world import VT_ARMY, native_wstring  # noqa: E402

FLAGS = [0x1000840, 0x1000820, 0, 0, 0, 0, 0x1000001, 0x1010000, 0x1001000, 0x1100800, 0x1000800, 0x20]
HOT = [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
VT_BODY = 0x2f84000
SEAM_BUSY = 0x2f85000


def f32(x):
    return struct.unpack('<I', struct.pack('<f', x))[0]


class MgaiWorld:
    """InitCreate world of campaign ci plus the MGAI inputs and seams."""

    def __init__(self, ci, cw=0x027f):
        self.snap, self.ctx = run(ci, cw=cw, keep=True)
        c = self.ctx
        self.ci = ci
        self.m, self.w, self.st = c['m'], c['w'], c['st']
        m, w = self.m, self.w
        self.T = w.read_vector(self.st + 0x28)
        self.K = w.read_vector(self.st + 0x1c)
        self.actors = c['actors']
        self.created = c['created']
        self.defs = c['defs']
        self.kdatas = c['kdatas']
        self.camp = c['c']
        self.thing_ids = {v: k for k, v in c['things'].items()}
        self.tech_ids = {v: k for k, v in c['techs'].items()}
        self.divisions = c['divisions']
        # command data registry (9511E0)
        objs = []
        for i in range(12):
            d = m.alloc(0x30)
            m.w32(d + 0xc, i)
            m.w32(d + 0x24, FLAGS[i])
            m.u.mem_write(d + 0x2c, bytes([HOT[i]]))
            objs.append(d)
        self.cmd_data = objs
        arr = m.array(objs)
        o3 = m.alloc(0x40); m.w32(o3 + 0x24, arr); m.w32(o3 + 0x28, 12)
        o2 = m.alloc(0x20); m.w32(o2 + 0x10, o3)
        reg = m.alloc(0x900); m.w32(reg + 0x820, o2)
        m.w32(0xb36530, reg)
        # app RNG (85D840 on [[B36528]+0x2C])
        app = m.alloc(0x40)
        self.rng_word = m.alloc(4)
        m.w32(app + 0x2c, self.rng_word)
        m.w32(0xb36528, app)
        self.world = m.r32(0xb36534)
        m.w32(self.world + 0xdc, 0)
        # army bodies (actor +0x70, vtable +0x64 busy seam)
        for off in range(0, 0x100, 4):
            m.w32(VT_BODY + off, 0x2f82200 + off)        # unexpected slots (seamed by mg_world)
        m.w32(VT_BODY + 0x64, SEAM_BUSY)
        m.u.mem_write(SEAM_BUSY, b'\xc3')
        self.body_of = {}
        for a in self.created:
            self.add_body(a['actor'])
        self.busy_log = []
        self.busy_script = None
        m.seams[SEAM_BUSY] = self._busy
        # seams: timer 59ABA0 (QueryPerformanceCounter based, result unused)
        m.seams[0x59aba0] = lambda mm: (mm.u.reg_write(UC_X86_REG_EDX, 0), mm.ret(0, 0))
        # yield 944CE0 / issue 944F50 on the controller
        self.controller = m.alloc(0x40)
        self.yield_log = []
        self.yield_script = None
        self.arrive_script = None
        self.applied = set()
        self.pending_arrival = None
        self.issued = []
        m.seams[0x944ce0] = self._yield
        m.seams[0x944f50] = self._issue
        # CRT/engine heap with reuse (the oracle runs hundreds of millions of
        # instructions; the bump allocator of mg_world never frees). Blocks
        # are zeroed on allocation as before (mg_machine.alloc); freed blocks
        # are reused by rounded size. No MGAI code reads uninitialised heap
        # except the 96A5D0 block below, which is filled explicitly.
        self.free_lists = {}
        self.block_size = {}
        def heap_alloc(mm, n):
            size = max((n + 15) & ~15, 16)
            lst = self.free_lists.get(size)
            if lst:
                p = lst.pop()
                mm.u.mem_write(p, bytes(size))
            else:
                p = mm.alloc(size)
            self.block_size[p] = size
            return p
        def malloc(mm):
            mm.ret(0, heap_alloc(mm, mm.arg(0)))
        def free(mm):
            p = mm.arg(0)
            size = self.block_size.pop(p, None)
            if size is not None:
                self.free_lists.setdefault(size, []).append(p)
            mm.ret(0)
        m.seams[0x96e19f] = malloc
        m.seams[0x96d331] = free
        m.seams[0x6f7430] = free
        # uninitialised heap buffer of 96A5D0 (operator new returning to 96A689)
        self.uninit_log = []
        self.uninit_rng = None
        def new(mm):
            ret_addr = mm.r32(mm.esp())
            n = mm.arg(0)
            p = heap_alloc(mm, max(n, 4))
            if ret_addr == 0x96a689:
                r = self.uninit_rng
                raw = bytes(0 if r.random() < 0.6 else r.randrange(1, 256) for _ in range(n)) if r else bytes(n)
                mm.u.mem_write(p, raw)
                self.uninit_log.append([n, sum(1 for b in raw if b)])
            mm.ret(0, p)
        m.seams[0x6f7810] = new
        # _wcsicmp 96CAD6: same C-locale semantics as mg_world.install_wcsicmp,
        # reading strings in 64-byte chunks (speed only)
        def wread(mm, p):
            out = []
            while True:
                try:
                    chunk = bytes(mm.u.mem_read(p, 64))
                except Exception:
                    chunk = bytes(mm.u.mem_read(p, 2))
                for i in range(0, len(chunk), 2):
                    ch = chunk[i] | chunk[i + 1] << 8
                    if ch == 0:
                        return out
                    out.append(ch + 0x20 if 0x41 <= ch <= 0x5a else ch)
                p += len(chunk)
        def wcsicmp(mm):
            a, b = wread(mm, mm.arg(0)), wread(mm, mm.arg(1))
            for x, y in zip(a + [0], b + [0]):
                if x != y:
                    mm.ret(0, (x - y) & 0xffffffff)
                    return
            mm.ret(0, 0)
        m.seams[0x96cad6] = wcsicmp
        self.mgai = {}         # ai index -> object
        self.ai_values = {}    # ai index -> dict (values written)
        self.lists = {}

    # ---------------------------------------------------------------- inputs
    def add_body(self, actor):
        m = self.m
        b = m.alloc(0x20)
        m.w32(b, VT_BODY)
        m.w32(actor + 0x70, b)
        self.body_of[b] = actor

    def _busy(self, mm):
        body = mm.u.reg_read(UC_X86_REG_ECX)
        a = self.body_of[body]
        idx = self.w.kind[a][1]
        val = self.busy_script(idx) if self.busy_script else 0
        self.busy_log.append([idx, val])
        mm.ret(0, val)

    def _yield(self, mm):
        arg = mm.arg(0) & 0xff
        st = self.st
        ret, set46, dt = self.yield_script(arg) if self.yield_script else (1, 0, 0)
        tick = (mm.r32(self.world + 0xdc) + dt) & 0xffffffff
        mm.w32(self.world + 0xdc, tick)
        if set46:
            mm.u.mem_write(st + 0x46, b'\x01')
        # scripted engine progress during the yield: the first issued, not yet
        # applied move/attack order's army is relocated by the original 940100
        # to the MG territory of the order's (alias-resolved) map territory.
        arrival = None
        if self.arrive_script is not None and self.arrive_script():
            for i, c in enumerate(self.issued):
                if c['index'] in (0, 1) and c.get('army') is not None and i not in self.applied:
                    self.applied.add(i)
                    t = next(j for j, tp in enumerate(self.T) if mm.r32(tp + 0xc) == self.defs[c['territory_def']])
                    arrival = [c['army'], t]
                    break
        self.yield_log.append([arg, ret, set46, tick, arrival])
        mm.ret(4, ret)
        if arrival is not None:
            self.pending_arrival = arrival
            mm.u.emu_stop()

    def _stub(self, target):
        m = self.m
        k = (target, False)
        if k not in m.stubs:
            at = STUB + 16 * len(m.stubs)
            assert at + 16 <= STUB + 0x1000
            code = bytes([0xe8]) + struct.pack('<i', target - (at + 5))
            m.u.mem_write(at, code)
            m.stubs[k] = (at, at + len(code))
        return m.stubs[k]

    def run_resumable(self, target, ecx, count=2_000_000_000):
        """Machine.call for a function whose yields may pause it (emu_stop in
        the yield seam): a pending arrival is applied by running the original
        940100 on a stack below the paused frames; then the saved CPU context
        (general and x87 registers) is restored and emulation resumes."""
        from unicorn.x86_const import UC_X86_REG_EBP, UC_X86_REG_FPCW, UC_X86_REG_FPSW, UC_X86_REG_FPTAG
        m = self.m
        start, end = self._stub(target)
        m.u.reg_write(UC_X86_REG_ESP, mg_machine.STACK_TOP - 0x1000)
        m.u.reg_write(UC_X86_REG_EBP, 0)
        m.u.reg_write(UC_X86_REG_ECX, ecx)
        m.u.reg_write(UC_X86_REG_FPCW, m.cw)
        m.u.reg_write(UC_X86_REG_FPSW, 0)
        m.u.reg_write(UC_X86_REG_FPTAG, 0xffff)
        pc = start
        while True:
            self.pending_arrival = None
            m.u.emu_start(pc, end, count=count)
            m.check_null()
            pc = m.u.reg_read(UC_X86_REG_EIP)
            if pc == end:
                return m.u.reg_read(UC_X86_REG_EAX)
            assert self.pending_arrival is not None, hex(pc)
            ctx = m.u.context_save()
            army, t = self.pending_arrival
            comp = m.r32(self.created[army]['actor'] + 0xcc)
            s2, e2 = self._stub(0x940100)
            nsp = (m.u.reg_read(UC_X86_REG_ESP) - 0x10000) & ~0xf
            m.w32(nsp, self.T[t])
            m.u.reg_write(UC_X86_REG_ESP, nsp)
            m.u.reg_write(UC_X86_REG_ECX, comp)
            m.u.reg_write(UC_X86_REG_FPCW, m.cw)
            m.u.reg_write(UC_X86_REG_FPSW, 0)
            m.u.reg_write(UC_X86_REG_FPTAG, 0xffff)
            m.u.emu_start(s2, e2, count=50_000_000)
            m.check_null()
            assert m.u.reg_read(UC_X86_REG_EIP) == e2
            m.u.context_restore(ctx)
            assert m.u.reg_read(UC_X86_REG_EIP) == pc

    def _issue(self, mm):
        slot = mm.arg(0)
        cmd = mm.r32(slot)
        self.issued.append(self.read_command(cmd))
        mm.w32(slot, 0)            # ownership moved to the controller queue (944F50)
        mm.ret(4, 0)

    def read_command(self, cmd):
        m, w = self.m, self.w
        bits = m.r32(cmd + 8)
        out = {'presence': bits}
        data = m.r32(cmd + 4)
        out['index'] = m.r32(data + 0xc) if data else None
        if bits & 1:
            a = w.resolve(m.r32(cmd + 0xc))
            out['army'] = w.kind[a][1] if a else None
        if bits & 2:
            out['actor_def'] = self.thing_ids[m.r32(cmd + 0x10)]
        if bits & 4:
            out['territory_def'] = self.defs.index(m.r32(cmd + 0x14))
        if bits & 0x10:
            out['tech'] = self.tech_ids[m.r32(cmd + 0x18)]
        if bits & 0x20:
            out['divisions'] = [self.divisions.index(d) for d in w.read_vector(cmd + 0x1c)]
        return out

    def build_mgai(self, ai_index, values):
        """values: dict as recorded in the fixture (ints, float bits, records)."""
        m, w = self.m, self.w
        o = m.alloc(0x60)
        for key, off in AI_INTS:
            m.w32(o + off, values[key])
        for key, off in AI_FLOATS:
            m.w32(o + off, values[key])
        def record(r):
            p = m.alloc(0x20)
            m.w32(p + 4, 0 if r['territory'] is None else self.defs[r['territory']])
            m.w32(p + 8, r['additional_importance'])
            m.w32(p + 0xc, r['additional_neighbor_proximity_importance_depth'])
            m.w32(p + 0x10, r['additional_neighbor_proximity_importance_max'])
            m.w32(p + 0x14, r['required_attack_strength_ratio_bits'])
            m.w32(p + 0x18, r['desired_defense_threat_ratio_bits'])
            return p
        maps = []
        for mr in values['maps']:
            p = m.alloc(0x30)
            m.w32(p + 4, m.r32(self.ctx_campaign_map()))
            w.vector(p + 8, [record(r) for r in mr['territories']])
            for key, off in (('default_territory', 0x14), ('capital', 0x18), ('friendly_capital', 0x1c), ('enemy_capital', 0x20)):
                m.w32(p + off, record(mr[key]))
            maps.append(p)
        w.vector(o + 0x18, maps)
        self.mgai[ai_index] = o
        self.ai_values[ai_index] = values
        return o

    def ctx_campaign_map(self):
        # 938970: MGState +0x08 (campaign) -> +0x18 map object; return its address
        return self.m.r32(self.st + 8) + 0x18

    def attach_mgai(self):
        for k, kd in zip(self.camp['kingdoms'], self.kdatas):
            self.m.w32(kd + 0x14, self.mgai[k['mgai']])

    def build_lists(self, orders):
        """orders: kingdom index -> list of army indices (native list order)."""
        m = self.m
        for k, a in enumerate(self.actors):
            head = 0
            for idx in reversed(orders[k]):
                node = m.alloc(8)
                m.w32(node, self.created[idx]['actor'])
                m.w32(node + 4, head)
                head = node
            m.w32(a + 0x284, head)
        self.lists = {k: list(v) for k, v in orders.items()}


AI_INTS = [('additional_attack_occupied_importance', 0x24), ('additional_attack_unoccupied_importance', 0x28),
           ('additional_defense_importance', 0x2c), ('production_importance_multiplier', 0x30),
           ('no_attack_distance', 0x34), ('max_influence_depth', 0x38),
           ('additional_border_importance', 0x4c), ('additional_border_reduction_importance', 0x50)]
AI_FLOATS = [('actor_distance_influence_multiplier_bits', 0x3c), ('actor_distance_strength_multiplier_bits', 0x40),
             ('goal_cutoff_ratio_bits', 0x44), ('attack_defendability_factor_bits', 0x48),
             ('purchase_army_probability_bits', 0x54), ('purchase_tech_probability_bits', 0x58)]
