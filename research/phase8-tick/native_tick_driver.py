"""Execute the ORIGINAL frame/admin/world tick driver under Unicorn.

Executed original code (unchanged AA.exe bytes):
  0x75CA70  TGC_AdminNode per-frame update (world tick vs admin tick choice)
  0x755290  admin tick body (decree seams hooked, admin-time increment native)
  0x76B780  KKC_Sim admin-tick-end timeline update ("m_admin_tick_end_*")
  0x76AF90  GetNextTickEventAdminTime  (x87 return in ST0)
  0x76AF30  game time per admin tick
  0x76AEE0, 0x76B020, 0x76B030, 0x8581C0, 0x6A71F0 gate helpers
  0x76DCE0  KKC_Sim::ResetAdminTime (case setup and dedicated rows)
  0x76F790  KKC_Sim world-tick wrapper (perf timers stubbed)
  0x4F9A07..0x4F9A3B + 0x4F9A43 the GWorld clock store fragment of 0x4F9970,
            copied byte-for-byte into a trampoline (the rest of 0x4F9970 is the
            already-proven world dispatch, research/phase2-world-order).
Seams (hooked, recorded as events, no behaviour invented):
  network pump 0x776540, command processing 0x75C9A0, sync-log writers
  0x77D930/0x77D9D0/0x77DA70 (values recorded), host decree execution 0x752E60,
  host history record 0x752EA0, admin history 0x752DA0, host rate 0x75FD80,
  client history replay 0x74FD10 (seam performs ONLY its final +0x20C += 1),
  sync-log flushes 0x74E8B0/0x74E670, heartbeat 0x75B820, 0x759680,
  timing smoothing 0x41C180, end-condition check 0x76F600, perf timers.
  0x752AE0 (wall-clock tick statistics around 0x76F790) is replaced by a
  direct jump into the original 0x76F790; its +0x60 average is telemetry.
The real-time admin clock (AdminNode+0x158 vfunc 8) and the AVI frame pump
(KKC_Sim+4 vfunc 4) are native-code stubs returning scripted values.
No live process; the executable is never modified.
"""
from pathlib import Path
import hashlib, json, struct, sys, random

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GAME = ROOT.parent
sys.path.insert(0, str(ROOT / 'research/engine-baseline'))
from audit_baseline import PE
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import *  # noqa

NATIVE = PE(GAME / 'AA.exe')
ACCEPTED = PE(GAME / 'reconstruction/native-game/runtime-expanded-v14/AA.exe')
assert hashlib.sha256(NATIVE.b).hexdigest() == '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
assert hashlib.sha256(ACCEPTED.b).hexdigest() == '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'

EXECUTED = {
    'admin_frame_75CA70': (0x75CA70, 0x75CD16), 'admin_tick_755290': (0x755290, 0x7554AD),
    'admin_tick_end_76B780': (0x76B780, 0x76B7F7), 'next_tick_event_76AF90': (0x76AF90, 0x76B01B),
    'game_per_admin_tick_76AF30': (0x76AF30, 0x76AF82), 'lockstep_gate_76AEE0': (0x76AEE0, 0x76AF2F),
    'avi_mode_76B020': (0x76B020, 0x76B02E), 'avi_pump_76B030': (0x76B030, 0x76B080),
    'avi_count_8581C0': (0x8581C0, 0x8581C4), 'always_false_6A71F0': (0x6A71F0, 0x6A71F3),
    'reset_admin_time_76DCE0': (0x76DCE0, 0x76DD83), 'sim_world_tick_76F790': (0x76F790, 0x76F884),
    'world_clock_fragment_4F9A07': (0x4F9A07, 0x4F9A3B),
}
CLOCK_FRAGMENT = NATIVE.range(0x4F9A07, 0x4F9A3B - 0x4F9A07)
assert NATIVE.range(0x4F9A43, 2) == b'\xdd\xd8'

PC = {24: 0x007F, 53: 0x027F, 64: 0x037F}
NO_EVENT_BITS = 0x5368D4A5  # 0xA92F84 binary32 1e12


def bits(f):
    return struct.unpack('<I', struct.pack('<f', f))[0]


def fl(b):
    return struct.unpack('<f', struct.pack('<I', b))[0]


def digest(b):
    return hashlib.sha256(b).hexdigest()


class Machine:
    HEAP, STACK = 0x2000000, 0x3000000

    def __init__(self, pc=64):
        u = self.u = Uc(UC_ARCH_X86, UC_MODE_32)
        u.mem_map(0, 0x1000)
        u.mem_map(NATIVE.base, (NATIVE.image_size + 0xFFF) & ~0xFFF)
        for vs, r, size, off in NATIVE.sections:
            u.mem_write(NATIVE.base + r, NATIVE.b[off:off + size])
        u.mem_map(self.HEAP, 0x400000)
        u.mem_map(self.STACK, 0x20000)
        self.next = self.HEAP
        self.stack = self.STACK + 0x10000
        self.stop = self.STACK + 0x1F000
        self.events = []
        self.pc = pc
        # Native stubs: scripted x87 return, scripted int return, plain returns.
        self.cell_real = self.alloc(4)
        self.cell_pump = self.alloc(4)
        self.stub_real = self.code(b'\xd9\x05' + struct.pack('<I', self.cell_real) + b'\xc2\x04\x00')  # fld [cell]; ret 4
        self.stub_pump = self.code(b'\xa1' + struct.pack('<I', self.cell_pump) + b'\xc2\x08\x00')      # mov eax,[cell]; ret 8
        self.stub_ret = self.code(b'\xc3')
        # mov esi,[0xB36534] after saving; scratch for the fragment's [esp+0x10].
        self.tramp_clock = self.code(b'\x56\x83\xec\x20\x8b\x35' + struct.pack('<I', 0xB36534) + CLOCK_FRAGMENT
                                     + b'\xdd\xd8\x83\xc4\x20\x5e\xc3')
        self.res80 = self.alloc(16)
        self.tramp_next = self.code(b'\xe8' + b'\0' * 4 + b'\xdb\x3d' + struct.pack('<I', self.res80) + b'\xc3')
        rel = 0x76AF90 - (self.tramp_next + 5)
        self.u.mem_write(self.tramp_next + 1, struct.pack('<i', rel))
        self.tramp_rate = self.code(b'\xe8' + b'\0' * 4 + b'\xdb\x3d' + struct.pack('<I', self.res80) + b'\xc3')
        rel = 0x76AF30 - (self.tramp_rate + 5)
        self.u.mem_write(self.tramp_rate + 1, struct.pack('<i', rel))
        self.vt_timer = self.table({8: self.stub_real})
        self.vt_pump = self.table({4: self.stub_pump})
        self.vt_perf = self.table({4: self.stub_ret, 8: self.stub_ret})
        # Globals.
        self.world = self.alloc(0x300)
        self.sim = self.alloc(0x60)
        self.session = self.alloc(0x200)
        self.netnode = self.alloc(0x100)
        self.admin = self.alloc(0x300)
        # 0xB36558 is the local TGC_AdminNode itself (ctor 0x75CE80); its +0x2C4
        # is the host node (SetHost 0x757010) and +0x2C8 the server node
        # (SetServer 0x759790).
        self.netmgr = self.admin
        self.other_admin = self.alloc(0x300)
        self.synclog = self.alloc(0x40)
        self.synclog2 = self.alloc(0x40)
        for g, v in [(0xB36534, self.world), (0xB36554, self.sim), (0xB36550, self.session),
                     (0xB36558, self.netmgr), (0xB36524, self.netnode), (0xB3655C, self.synclog),
                     (0xB36560, self.synclog2), (0xB40908, 0), (0xB40904, 0), (0xB396C4, 0)]:
            self.w32(g, v)
        self.uvars = self.alloc(0x400)
        self.gvars = self.alloc(0x400)
        self.vvars = self.alloc(0x400)
        self.w32(0xB39940, self.uvars)
        self.w32(0xB39AB0, self.gvars)
        self.w32(0xB39990, self.vvars)
        self.uvar_avi = self.var(self.uvars, 0x1C8)        # RenderDumpAVI (bool)
        self.uvar_avi_frames = self.var(self.uvars, 0x1CC)  # RenderAVIFramesPerTick (int)
        self.uvar_autopause = self.var(self.uvars, 0x350)  # SinglePlayerAutoPause (bool)
        self.gvar_fog = self.var(self.gvars, 0x1BC)
        self.vvar_async = self.var(self.vvars, 0x11C)      # NetworkAdminAsyncTicksPerSecond
        self.w32(self.vvar_async, bits(10.0))
        self.u.mem_write(self.netnode + 8, struct.pack('<I', self.vt_perf))
        self.u.mem_write(self.netnode + 0x24, struct.pack('<I', self.vt_perf))
        timer = self.alloc(8); self.w32(timer, self.vt_timer)
        self.w32(self.admin + 0x158, timer)
        self.w8(self.admin + 0x2C0, 1)
        self.w32(self.sim + 4, self.vt_pump)
        self.w32(self.netmgr + 0x2C4, self.admin)
        self.w32(self.netmgr + 0x2C8, 0)
        self.u.reg_write(UC_X86_REG_FPCW, PC[pc])
        self.u.reg_write(UC_X86_REG_MXCSR, 0x1F80)  # Windows thread default
        self.u.hook_add(UC_HOOK_CODE, self.hook)
        self.seams = {
            0x776540: ('net_pump', 0), 0x75C9A0: ('process_commands', 0), 0x751770: ('admin_disconnect_reset', 0),
            0x7642A0: ('net_7642A0', 0), 0x75B820: ('heartbeat', 0), 0x759680: ('tail_759680', 0),
            0x752E60: ('host_execute_requests', 0), 0x752EA0: ('host_record_history', 0),
            0x752DA0: ('admin_history_752DA0', 0), 0x75FD80: ('host_rate_75FD80', 4),
            0x74E8B0: ('sync_flush_client', 4), 0x74E670: ('sync_flush_host', 4),
            0x41C180: ('timing_smoothing', 0), 0x76F600: ('end_condition_check', 0),
            0x41C0E0: (None, 4), 0x41C110: (None, 4),
        }

    # ---- memory helpers
    def alloc(self, n):
        p = self.next
        self.next += (n + 15) & ~15
        self.u.mem_write(p, b'\0' * n)
        return p

    def code(self, b):
        p = self.alloc(len(b) + 1)
        self.u.mem_write(p, b)
        return p

    def table(self, slots):
        t = self.alloc(0x40)
        for off, target in slots.items():
            self.w32(t + off, target)
        return t

    def var(self, table, off):
        p = self.alloc(4)
        self.w32(table + off, p)
        return p

    def w32(self, a, v):
        self.u.mem_write(a, struct.pack('<I', v & 0xFFFFFFFF))

    def w8(self, a, v):
        self.u.mem_write(a, bytes([v & 0xFF]))

    def r32(self, a):
        return struct.unpack('<I', self.u.mem_read(a, 4))[0]

    def r8(self, a):
        return self.u.mem_read(a, 1)[0]

    def wstr(self, a):
        out = ''
        while True:
            c = struct.unpack('<H', self.u.mem_read(a, 2))[0]
            if not c:
                return out
            out += chr(c)
            a += 2

    def ret(self, n=0, value=None):
        sp = self.u.reg_read(UC_X86_REG_ESP)
        if value is not None:
            self.u.reg_write(UC_X86_REG_EAX, value)
        self.u.reg_write(UC_X86_REG_EIP, self.r32(sp))
        self.u.reg_write(UC_X86_REG_ESP, sp + 4 + n)

    def arg(self, i):
        return self.r32(self.u.reg_read(UC_X86_REG_ESP) + 4 + 4 * i)

    # ---- backup-mode clients (AdminNode+0x2AC list of {client, next})
    def add_client(self, **flags):
        c = self.alloc(0x80)
        node = self.alloc(8)
        self.w32(node, c)
        tail = self.r32(self.admin + 0x2B0)
        if tail:
            self.w32(tail + 4, node)
        else:
            self.w32(self.admin + 0x2AC, node)
        self.w32(self.admin + 0x2B0, node)
        self.set_client(c, **flags)
        return c

    def set_client(self, c, f30=0, f48=0, f49=0, f4a=0, f4b=0):
        for off, v in ((0x30, f30), (0x48, f48), (0x49, f49), (0x4A, f4a), (0x4B, f4b)):
            self.w8(c + off, v)

    def clients(self):
        out, p = [], self.r32(self.admin + 0x2AC)
        while p:
            c = self.r32(p)
            out.append({k: self.r8(c + off) for k, off in (('f30', 0x30), ('f48', 0x48), ('f49', 0x49),
                                                           ('f4a', 0x4A), ('f4b', 0x4B))})
            p = self.r32(p + 4)
        return out

    # ---- hooks
    def hook(self, u, pc, size, _):
        if pc == 0x759680 and getattr(self, 'native_backup', False):
            self.events.append({'event': 'server_backup_check'})
            return
        if pc == 0x757250:
            self.events.append({'event': 'set_backup_mode', 'value': self.arg(0) & 0xFF})
            return
        if pc == 0x757290:  # entering-backup diagnostics loop; resume after it
            self.events.append({'event': 'backup_mode_entered'})
            sp = self.u.reg_read(UC_X86_REG_ESP) - 4
            self.w32(sp, self.u.reg_read(UC_X86_REG_EDI))
            self.u.reg_write(UC_X86_REG_ESP, sp)
            self.u.reg_write(UC_X86_REG_EIP, 0x7573B3)
            return
        if pc == 0x791800:
            self.events.append({'event': 'gamespy_backup_report'})
            self.ret(0)
            return
        if pc in self.seams:
            name, n = self.seams[pc]
            if name:
                self.events.append({'event': name})
            self.ret(n)
            return
        if pc in (0x77D930, 0x77D9D0, 0x77DA70):
            name = self.wstr(self.arg(0))
            value = self.arg(1)
            if pc == 0x77D9D0:
                value &= 0xFF
            self.events.append({'event': 'log', 'name': name, 'value': value})
            self.ret(12)
            return
        if pc in (0x404F60, 0x855D30):
            self.events.append({'event': 'diagnostic', 'at': hex(pc)})
            self.ret(0, value=0)
            return
        if pc == 0x74FD10:
            self.events.append({'event': 'client_replay_history', 'index': self.r32(self.admin + 0x20C)})
            self.w32(self.admin + 0x20C, self.r32(self.admin + 0x20C) + 1)
            self.ret(0)
            return
        if pc == 0x755290:
            self.events.append({'event': 'admin_tick', 'admin_time_bits': self.r32(self.admin + 0x28C)})
            return
        if pc == 0x752AE0:
            self.events.append({'event': 'world_tick'})
            self.u.reg_write(UC_X86_REG_ECX, self.r32(0xB36554))
            self.u.reg_write(UC_X86_REG_EIP, 0x76F790)
            return
        if pc == 0x4F9970:
            self.events.append({'event': 'world_update', 'tick_before': self.r32(self.world + 0xDC)})
            self.u.reg_write(UC_X86_REG_EIP, self.tramp_clock)
            return
        if (pc == 0x76DD90 or pc == 0x74B180) and not getattr(self, 'allow_freeze_calls', False):
            raise AssertionError(f'unexpected call {pc:#x}')

    # ---- execution
    def call(self, va, ecx, args=()):
        self.w32(self.stack, self.stop)
        for i, a in enumerate(args):
            self.w32(self.stack + 4 + 4 * i, a)
        self.u.reg_write(UC_X86_REG_ESP, self.stack)
        self.u.reg_write(UC_X86_REG_ECX, ecx)
        saved = {UC_X86_REG_EBX: 0xABC00001, UC_X86_REG_ESI: 0xABC00002, UC_X86_REG_EDI: 0xABC00003,
                 UC_X86_REG_EBP: 0xABC00004}
        for r, v in saved.items():
            self.u.reg_write(r, v)
        self.u.emu_start(va, self.stop, count=2_000_000)
        for r, v in saved.items():
            assert self.u.reg_read(r) == v, hex(va)
        return self.u.reg_read(UC_X86_REG_ESP) - self.stack

    def f80(self):
        return self.u.mem_read(self.res80, 10).hex()

    # ---- state
    def sim_state(self):
        s = self.sim
        return {'begin_game': self.r32(s + 0x10), 'begin_admin': self.r32(s + 0x14), 'end_game': self.r32(s + 0x18),
                'end_admin': self.r32(s + 0x1C), 'speed': self.r32(s + 0x20), 'max_speed': self.r32(s + 0x24),
                'paused': self.r8(s + 0x28), 'frozen': self.r8(s + 0x29), 'game_per_admin_tick': self.r32(s + 0x2C),
                'avi_frames': self.r32(s + 0x38)}

    def state(self):
        return {'backup_mode': self.r8(self.admin + 0x2D4), 'backup_since': self.r32(self.admin + 0x2D8),
                'admin_time': self.r32(self.admin + 0x28C), 'processed': self.r32(self.admin + 0x20C),
                'received': self.r32(self.admin + 0x218), 'heartbeat_at': self.r32(self.admin + 0x2DC),
                'client_sync_counter': self.r32(0xB40908), 'host_sync_counter': self.r32(0xB40904),
                'sim': self.sim_state(),
                'world': {'tick': self.r32(self.world + 0xDC), 'begin': self.r32(self.world + 0xE0),
                          'end': self.r32(self.world + 0xE4), 'step': self.r32(self.world + 0xE8)}}

    def set_gates(self, g):
        self.w8(self.session + 0xF4, g.get('session_f4', 0))
        self.w32(self.session + 0xE4, g.get('session_mode', 0))
        self.w32(0xB396C4, g.get('flags_b396c4', 0))
        self.w8(self.uvar_autopause, g.get('single_player_auto_pause', 0))
        self.w8(self.uvar_avi, g.get('avi', 0))
        self.w32(self.uvar_avi_frames, g.get('avi_frames_per_tick', 0))
        self.w8(self.admin + 0x2D4, g.get('playback', 0))

    def set_sim(self, s):
        o = self.sim
        for off, key in [(0x10, 'begin_game'), (0x14, 'begin_admin'), (0x18, 'end_game'), (0x1C, 'end_admin'),
                         (0x20, 'speed'), (0x24, 'max_speed'), (0x2C, 'game_per_admin_tick'), (0x38, 'avi_frames')]:
            if key in s:
                self.w32(o + off, s[key])
        for off, key in [(0x28, 'paused'), (0x29, 'frozen')]:
            if key in s:
                self.w8(o + off, s[key])

    def set_world(self, tick, begin, end, step):
        self.w32(self.world + 0xDC, tick)
        self.w32(self.world + 0xE0, begin)
        self.w32(self.world + 0xE4, end)
        self.w32(self.world + 0xE8, step)


# --------------------------------------------------------------------------
GATES_SP = {'session_f4': 0, 'session_mode': 0, 'flags_b396c4': 0, 'single_player_auto_pause': 0}


def driver_case(name, pc, rate_bits, step_bits, frames, role='host', admin_time=0.0, gates=None, received=0,
                frame_script=None, real_script=None, now_script=None, pump_script=None, sim_overrides=None,
                world=(0, 0, 0), server=False, setup=None):
    m = Machine(pc)
    if server:
        m.w32(m.admin + 0x2C8, m.admin)
        m.native_backup = True
    if setup:
        setup(m)
    m.set_gates(gates or GATES_SP)
    m.set_world(world[0], world[1], world[2], step_bits)
    m.set_sim({'paused': 0, 'frozen': 0, 'game_per_admin_tick': rate_bits, 'speed': 0})
    if role == 'client':
        m.w32(m.netmgr + 0x2C4, m.other_admin)
        m.w32(m.admin + 0x218, received)
    t0 = bits(admin_time)
    m.w32(m.admin + 0x28C, t0)
    m.call(0x76DCE0, m.sim, (t0,))  # ResetAdminTime(t0) as KKC_Sim::SetFrozen(false) does
    if sim_overrides:
        m.set_sim(sim_overrides)
    initial = m.state()
    rows = []
    for f in range(frames):
        real = (real_script or (lambda i: 1.0e6))(f)
        now = (now_script or (lambda i: 0.25 * i))(f)
        pump = (pump_script or (lambda i: 1))(f)
        if frame_script:
            frame_script(m, f)
        m.w32(m.cell_real, bits(real))
        m.w32(m.netnode + 0x58, bits(now))
        m.w32(m.cell_pump, pump)
        clients_before = m.clients()
        inputs = {'received': m.r32(m.admin + 0x218), 'paused': m.r8(m.sim + 0x28),
                  'frozen': m.r8(m.sim + 0x29), 'avi': m.r8(m.uvar_avi),
                  'avi_frames_per_tick': m.r32(m.uvar_avi_frames), 'playback': m.r8(m.admin + 0x2D4)}
        m.events = []
        m.call(0x75CA70, m.admin)
        rows.append({'frame': f, 'real_admin_clock': bits(real), 'now': bits(now), 'pump': pump,
                     'inputs': inputs, 'clients': clients_before, 'events': m.events, 'state': m.state()})
    return {'name': name, 'kind': 'driver', 'precision': pc, 'role': role, 'server': server,
            'gates': gates or GATES_SP,
            'rate_bits': rate_bits, 'step_bits': step_bits, 'reset_admin_time_bits': t0, 'initial': initial,
            'frames': rows}


def actions(row):
    return [e['event'] for e in row['events'] if e['event'] in ('world_tick', 'admin_tick')]


def driver_cases():
    out = []
    s16 = bits(1 / 16)
    for pc in (64, 24, 53):
        out.append(driver_case(f'host_speed1_pc{pc}', pc, bits(1.0 / 16), s16, 24))
        out.append(driver_case(f'host_speed2_pc{pc}', pc, bits(2.0 / 16), s16, 24))
        out.append(driver_case(f'host_speed4_pc{pc}', pc, bits(4.0 / 16), s16, 24))
        out.append(driver_case(f'host_quarter_speed_pc{pc}', pc, bits(0.25 / 16), s16, 24))
        # Non-dyadic rates and steps (GameSpeed fractional, compiled default 10 ticks/s).
        out.append(driver_case(f'host_sqrt2_rate_step_tenth_pc{pc}', pc, bits(2 ** 0.5 / 16), bits(0.1), 40,
                               admin_time=3.4375))
        out.append(driver_case(f'host_third_rate_pc{pc}', pc, bits(1 / 3), bits(1 / 16), 40, admin_time=7.1))
    # Real-time gating: only frames whose clock reaches min(next event, admin time) act.
    out.append(driver_case('host_realtime_gating', 64, bits(1 / 16), s16, 20,
                           real_script=lambda i: [0.0, 0.0, 0.0625, 0.0625, 0.03, 0.0625, 0.1, 0.125, 0.125, 0.2,
                                                  0.2, 0.2, 0.25, 0.3, 0.3125, 0.3125, 1.0, 1.0, 1.0, 1.0][i]))
    # Pause: zero game time per admin tick and no world events; frozen: timeline untouched.
    def pause_script(m, f):
        if f == 6:
            m.w8(m.sim + 0x28, 1)
        if f == 14:
            m.w8(m.sim + 0x28, 0)
    out.append(driver_case('host_pause_resume', 64, bits(1 / 16), s16, 24, frame_script=pause_script))
    def freeze_script(m, f):
        if f == 5:
            m.w8(m.sim + 0x29, 1)
        if f == 11:
            m.w8(m.sim + 0x29, 0)
    out.append(driver_case('host_freeze_unfreeze_without_reset', 64, bits(1 / 16), s16, 20, frame_script=freeze_script))
    # Client: admin ticks only while processed < received.
    def client_script(m, f):
        if f == 12:
            m.w32(m.admin + 0x218, 6)
    out.append(driver_case('client_received_window', 64, bits(1 / 16), s16, 24, role='client', received=3,
                           frame_script=client_script))
    # Playback / replay gate: host never runs admin ticks while (session f4, mode, playback).
    out.append(driver_case('host_playback_gate_blocks_admin', 64, bits(1 / 16), s16, 8,
                           gates={**GATES_SP, 'session_f4': 1, 'session_mode': 2}, sim_overrides=None,
                           frame_script=lambda m, f: m.w8(m.admin + 0x2D4, 1)))
    # Multiplayer host (session f4) with sync-log counter cadence.
    out.append(driver_case('mp_host_mode2_sync_cadence', 64, bits(1 / 16), s16, 44,
                           gates={**GATES_SP, 'session_f4': 1, 'session_mode': 2}))
    # Single-player autopause UVar disables the timeline (gate false) unless flags.
    out.append(driver_case('sp_autopause_gate_disables_timeline', 64, bits(1 / 16), s16, 10,
                           gates={**GATES_SP, 'single_player_auto_pause': 1}))
    out.append(driver_case('sp_autopause_overridden_by_flag', 64, bits(1 / 16), s16, 10,
                           gates={**GATES_SP, 'single_player_auto_pause': 1, 'flags_b396c4': 2}))
    # AVI capture mode: frame pump accumulates into KKC_Sim+0x38 up to N per frame.
    out.append(driver_case('avi_mode_frames_per_tick3', 64, bits(1 / 16), s16, 24,
                           gates={**GATES_SP, 'avi': 1, 'avi_frames_per_tick': 3},
                           pump_script=lambda i: [1, 1, 0, 1, 1, 1, 2, 0][i % 8]))
    # Server node: 0x759680 backup-mode decision at the frame tail gates the
    # host's admin ticks (0x75CC13: session f4 && mode && +0x2D4).
    def backup_setup(m):
        m.cA = m.add_client()
        m.cB = m.add_client()
    def backup_script(m, f):
        if f == 6:
            m.set_client(m.cA, f49=1)
        if f == 12:
            m.set_client(m.cA, f48=1)
        if f == 16:
            m.set_client(m.cA)
        if f == 20:
            m.set_client(m.cB, f30=1, f4a=1)
        if f == 23:
            m.set_client(m.cB)
    out.append(driver_case('server_backup_mode_cycle', 64, bits(1 / 16), s16, 28, server=True,
                           gates={**GATES_SP, 'session_f4': 1, 'session_mode': 2},
                           setup=backup_setup, frame_script=backup_script))
    def gamespy_setup(m):
        backup_setup(m)
        gs = m.alloc(0x100); m.w8(gs + 0xBC, 1); m.w32(0xB3654C, gs)
        gp = m.alloc(0x100); m.w32(gp + 0xB8, 1); m.w32(0xB40C9C, gp)
    out.append(driver_case('server_backup_with_gamespy_report', 64, bits(1 / 16), s16, 10, server=True,
                           gates={**GATES_SP, 'session_f4': 1, 'session_mode': 2},
                           setup=gamespy_setup, frame_script=lambda m, f: f == 3 and m.set_client(m.cB, f4b=1)))
    # Server without a networked session: backup entered but never gates admin ticks.
    out.append(driver_case('server_backup_without_session_gate', 64, bits(1 / 16), s16, 10, server=True,
                           setup=backup_setup, frame_script=lambda m, f: f == 2 and m.set_client(m.cA, f49=1)))
    # Nonzero starting world clock (mid-game reset after unfreeze).
    out.append(driver_case('host_reset_midgame', 64, bits(1 / 16), s16, 16, admin_time=12.5,
                           world=(37, bits(36 / 16), bits(37 / 16))))
    return out


def pure_cases(seed=0x5EED8):
    rng = random.Random(seed)
    out = []
    def rnd():
        choice = rng.random()
        if choice < 0.3:
            return bits(rng.choice([0.0, 0.0625, 0.125, 1.0, 3.0, 0.1, 1 / 3]) * rng.randint(0, 64))
        return bits(rng.uniform(-2.0, 300.0))
    for pc in (24, 53, 64):
        for i in range(60):
            m = Machine(pc)
            g = {'session_f4': int(rng.random() < 0.3), 'flags_b396c4': rng.choice([0, 0, 2, 1]),
                 'single_player_auto_pause': int(rng.random() < 0.3), 'session_mode': 0}
            m.set_gates(g)
            s = {'begin_game': rnd(), 'begin_admin': rnd(), 'end_game': rnd(), 'end_admin': rnd(),
                 'paused': int(rng.random() < 0.1), 'frozen': int(rng.random() < 0.1), 'game_per_admin_tick': rnd()}
            if i % 7 == 0:
                s['end_game'] = s['begin_game']
            if i % 11 == 0:
                s['end_admin'] = s['begin_admin']
            m.set_sim(s)
            world_end = rnd()
            m.set_world(0, 0, world_end, bits(1 / 16))
            if i % 13 == 0:
                m.w32(0xB36534, 0)
            try:
                m.call(m.tramp_next, m.sim)
                nxt = m.f80()
            except Exception as e:  # pragma: no cover - recorded, never expected
                nxt = 'error:' + str(e)
            m.w32(0xB36534, m.world)
            m.call(m.tramp_rate, m.sim)
            rate = m.f80()
            admin = rnd()
            before = m.sim_state()
            m.events = []
            m.call(0x76B780, m.sim, (admin,))
            after = m.sim_state()
            t = rnd()
            m.set_world(0, rnd(), world_end, bits(1 / 16))
            wbegin = m.r32(m.world + 0xE0)
            m2 = Machine(pc)
            m2.set_world(0, wbegin, world_end, bits(1 / 16))
            m2.call(0x76DCE0, m2.sim, (t,))
            reset = m2.sim_state()
            # Admin increment inside 0x755290 (host, all decree seams hooked).
            m3 = Machine(pc)
            m3.set_gates(g)
            m3.set_sim(s)
            a0 = rnd()
            m3.w32(m3.admin + 0x28C, a0)
            m3.call(0x755290, m3.admin)
            out.append({'precision': pc, 'gates': g, 'sim': s, 'world_end': world_end,
                        'world_present': i % 13 != 0, 'next_tick_event_f80': nxt, 'game_per_admin_tick_f80': rate,
                        'admin_tick_end': {'argument': admin, 'before': before, 'after': after},
                        'reset_admin_time': {'argument': t, 'world_begin': wbegin, 'after': reset},
                        'admin_increment': {'before': a0, 'after': m3.r32(m3.admin + 0x28C),
                                            'timeline': m3.sim_state()}})
    return out


def main():
    ranges = {}
    for name, (a, b) in EXECUTED.items():
        data = NATIVE.range(a, b - a)
        assert data == ACCEPTED.range(a, b - a), name
        ranges[name] = {'start': hex(a), 'end_exclusive': hex(b), 'sha256': digest(data), 'unchanged_in_v14': True}
    cases = driver_cases()
    # Structural assertions over the oracle (not used as Rust expectations).
    for c in cases:
        for row in c['frames']:
            assert len(actions(row)) <= 1, (c['name'], row['frame'])
    sp1 = next(c for c in cases if c['name'] == 'host_speed1_pc64')
    assert [actions(r) for r in sp1['frames'][:4]] == [['admin_tick'], ['world_tick'], ['admin_tick'], ['world_tick']]
    sp2 = next(c for c in cases if c['name'] == 'host_speed2_pc64')
    assert [a for r in sp2['frames'][:7] for a in actions(r)] == ['admin_tick', 'world_tick', 'world_tick', 'admin_tick',
                                                                   'world_tick', 'world_tick', 'admin_tick']
    pure = pure_cases()
    report = {
        'schema': 1, 'passed': True,
        'source_sha256': digest(NATIVE.b), 'accepted_sha256': digest(ACCEPTED.b),
        'producer_sha256': digest(Path(__file__).read_bytes()),
        'method': __doc__, 'executed_ranges': ranges,
        'no_event_admin_time_bits': NO_EVENT_BITS,
        'driver_cases': cases, 'pure_cases': pure,
    }
    target = HERE / 'native-tick-driver.json'
    target.write_text(json.dumps(report, indent=1) + '\n')
    print(json.dumps({'driver_cases': len(cases), 'frames': sum(len(c['frames']) for c in cases),
                      'pure_cases': len(pure), 'sha256': digest(target.read_bytes())}))


if __name__ == '__main__':
    main()
