"""Original TGC_AdminNode async heartbeat and TGC_AdminConn::Update under Unicorn.

Executes the ORIGINAL code of:
  0x75B820 heartbeat (AdminNode, 16 Hz)        0x75BD70 AdminConn::Update
  0x7593E0 ServerDispatchBackupRequestsAndBuildDecree
  0x753940 backup chat request -> chat decree  0x74FA20 chat decree writer
  0x74F890 backup vote request                 0x75A2C0 DisconnectLostPlayers rule
  0x74DE10 / 0x74DED0 / 0x74DD50 / 0x74DD20 / 0x74DD80 vote helpers
  0x759530 drop lagging players (0x74E970 TaskStatus request 304,
           0x750CB0 PlayerLeave decree 309, 0x7501D0 reverse decree transfer)
  0x74F1A0 lag refresh with every peer vtable slot (node 0xA913B0:
           0x5AA2D0 0x74B270 0x74B2A0 0x74B2B0 0x74B2D0 0x74B2F0 0x74B300;
           conn 0xA8D964: 0x74B260 0x74B280(+0x7642D0) 0x74D490 0x74B2C0
           0x74B2E0 0x74B2F0 0x74D5B0)
  0x752C60 / 0x74E530 / 0x74C450 backup status decree build
  0x75A0C0 ProcessBackupDecrees, 0x74E5C0 / 0x74C590 status, 0x756E00 chat
  0x74F310 UpdateFileTransferStatus, 0x74EA20 / 0x74CC20 UpdateMapStatus 305
  0x75A250 Timeout, 0x759790 SetServer, 0x757010 SetHost, 0x752110 / 0x7514F0
  connection reset, 0x752300 / 0x752490 channel (re)binding sequences,
  0x74F950 / 0x74C3D0 client backup request writers,
  history helpers 0x7507D0 0x750880 0x74EC30 0x74CDB0, packet/stream helpers
  0x766D00 0x765F10 0x765F70 0x76B8F0 0x40D420 0x40D5D0 0x411FA0 0x74D0A0.

Seams (recorded as events, never invented): logging; CString name getters
0x766380 / 0x7667D0 ("P<id>" / "D<id>"); text table 0x4B9F40 (identity);
UI message 0x40BC50 (format + string arguments); request gate 0x766CD0
(fixture flag), local player 0x76AC60, SetPlayer 0x766C10, request/decree
streams 0x766C90 / 0x766CA0 (fixture writers); sync log 0x77DA70 / 0x40F8D0;
PlayerLeave process 0x75DAA0; channel objects 0x763600 / 0x763080 /
0x762B20 / 0x763130 / 0x761400 / 0x761500 (recorded with the channel offset);
peer sub-object reset 0x77B8D0; channel acknowledgement query 0x7626A0
(fixture value per channel); file progress 0x7612D0 / size 0x7A6610
(fixture values); clock restart 0x75FBF0 and the old clock virtual +8
(fixture float); ResetOrderVersion 0x74DBC0; backup-mode decision 0x759680
(ported in aa-runtime); private decrees 0x7531A0; GameSpy 0x791840; timer
0x59ABA0 (log only). FPU control word explicit per case (PC24 / PC53 / PC64).
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase4-orders'))
from recover_orders import OrdersMachine, PE0, PE1, sha
sys.path.insert(0, str(HERE))
import null_guard
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import *

PC = {'Single': 0x007f, 'Double': 0x027f, 'Extended': 0x037f}
STUB = 0x7700000
NETNODE = 0x2010000
CONN0 = 0x2020000; CONN_STRIDE = 0x800
NETCONN0 = 0x2028000; NETCONN_STRIDE = 0x100
REQ = 0x2030000; DEC = 0x2030100; SCRATCH = 0x2030200
SYNC_A = 0x2031000
GAMESPY_A = 0x2032000; GAMESPY_B = 0x2032100
AVARS = 0x2033000; AVARS2 = 0x2034000
NODE_VT = 0xa913b0; CONN_VT = 0xa8d964
HIST = {0xc: 'requests', 0x1c: 'backup_requests', 0xac: 'private_decrees', 0xbc: 'reverse_decrees'}
NODE_HIST = {0x1c: 'backup_requests', 0x214: 'decrees', 0x224: 'backup_decrees'}
CHANNELS = {0xcc: 'broadcast_decree_sender', 0x114: 'broadcast_decree_receiver', 0x144: 'reverse_decree_sender',
            0x18c: 'reverse_decree_receiver', 0x1bc: 'broadcast_request_sender', 0x204: 'broadcast_request_receiver',
            0x234: 'private_decree_sender', 0x27c: 'private_decree_receiver', 0x2ac: 'backup_decree_sender',
            0x2f4: 'backup_decree_receiver', 0x324: 'backup_request_sender', 0x36c: 'backup_request_receiver',
            0x39c: 'file_sender', 0x3e8: 'file_receiver'}
STUB_RET = STUB; STUB_FLD = STUB + 0x10; LOCK_VT = STUB + 0x100; CLOCK_VT = STUB + 0x200; FLD_VALUE = STUB + 0x800


def f2b(v): return struct.unpack('<I', struct.pack('<f', v))[0]
def b2f(b): return struct.unpack('<f', struct.pack('<I', b & 0xffffffff))[0]
def s32(v): return struct.unpack('<i', struct.pack('<I', v & 0xffffffff))[0]


class HbMachine(OrdersMachine):
    def __init__(self):
        super().__init__()
        null_guard.install(self)  # emulator caveat: page-0 accesses raise NullDereference
        self.mode = 'heartbeat'
        self.u.mem_map(STUB, 0x1000)
        code = {STUB_RET: b'\xc3', STUB_FLD: b'\xd9\x05' + struct.pack('<I', FLD_VALUE) + b'\xc2\x04\x00'}
        for a, c in code.items(): self.u.mem_write(a, c)
        self.w(LOCK_VT + 4, STUB_RET); self.w(LOCK_VT + 8, STUB_RET); self.w(CLOCK_VT + 8, STUB_FLD)
        self.w(0xb36558, self.admin); self.w(0xb36524, NETNODE); self.w(0xb3655c, SYNC_A)
        self.w(self.order_manager + 4, 357)
        for desc, ident, name in [(0xb40980, 304, 'TGC_UpdateTaskStatusOrderRequest'),
                                  (0xb4098c, 305, 'TGC_UpdateMapStatusOrderRequest'),
                                  (0xb409bc, 309, 'TGC_PlayerLeaveOrderDecree')]:
            self.w(desc + 4, ident); self.w(desc + 8, self.string(name))
        self.w(0xb39990, AVARS); self.w(0xb39ab0, AVARS2)
        self.w(0xb3654c, GAMESPY_A); self.w(0xb40c9c, GAMESPY_B)
        self.mark = self.heap
        self.ack = {}; self.file = {}
        self.stubs = {
            0x404f60: lambda: self.ret(), 0x404f90: lambda: self.ret(),
            0x766380: self.name_stub('P'), 0x7667d0: self.name_stub('D'),
            0x4b9f40: lambda: self.ret(8, self.arg(0)),
            0x40bc50: self.ui_message,
            0x766cd0: lambda: (self.ev('request_gate', active=self.active), self.ret(0, int(self.active))),
            0x76ac60: lambda: self.ret(0, self.local_player),
            0x766c10: lambda: (self.ev('request_player', player=self.pid(self.arg(0))), self.ret()),
            0x766c90: lambda: self.ret(0, REQ), 0x766ca0: lambda: self.ret(0, DEC),
            0x77da70: lambda: (self.ev('sync_int', log=self.log_name(), label=self.text(self.arg(0)), value=self.arg(1)), self.ret(12)),
            0x40f8d0: lambda: (self.ev('sync_string', log='A', label=self.text(self.arg(0)), value=self.text(self.arg(1))), self.ret()),
            0x75daa0: self.process_stub,
            0x763600: lambda: self.chan('sender_bind', 8, history=self.hist_name(self.arg(0)), start=s32(self.arg(1))),
            0x763080: lambda: self.chan('receiver_reset', 0),
            0x762b20: lambda: self.chan('sender_reset', 0),
            0x763130: lambda: self.chan('receiver_bind', 4, history=self.hist_name(self.arg(0))),
            0x761400: lambda: self.chan('file_sender_reset', 0),
            0x761500: lambda: self.chan('file_receiver_reset', 0),
            0x77b8d0: lambda: (self.ev('peer_subobject_reset', peer=self.peer_name(self.ecx() - 0x74)), self.ret()),
            0x7626a0: lambda: self.ret(0, self.ack[self.ecx()] & 0xffffffff),
            0x7612d0: lambda: self.ret(0, self.file[self.ecx()][0] & 0xffffffff),
            0x7a6610: lambda: self.ret(0, self.file[self.ecx()][1] & 0xffffffff),
            0x75fbf0: lambda: (self.ev('clock_restart', clock=hex(self.ecx() - self.admin), period=self.arg(0), start=self.arg(1)), self.ret(8)),
            0x74dbc0: lambda: (self.ev('reset_order_version', arg=self.arg(0)), self.ret(4)),
            0x759680: lambda: (self.ev('backup_mode_decision'), self.ret()),
            0x7531a0: lambda: (self.ev('private_decrees', conn=self.peer_name(self.ecx())), self.ret()),
            0x791840: lambda: (self.ev('gamespy_verify_end'), self.ret()),
            0x59aba0: lambda: self.ret(0, 0),
        }
        self.events = []
    # --- seam helpers -------------------------------------------------
    def ecx(self): return self.u.reg_read(UC_X86_REG_ECX)
    def ev(self, kind, **kw):
        d = {'kind': kind}; d.update(kw); self.hb_events.append(d)
    def log_name(self): return 'A' if self.ecx() == SYNC_A else hex(self.ecx())
    def pid(self, p): return None if p == 0 else self.r(p + 0x20)
    def name_stub(self, prefix):
        def f():
            ecx = self.ecx(); out = self.arg(0)
            self.w(out, self.string(prefix + ('null' if ecx == 0 else str(self.r(ecx + 0x20)))))
            self.ret(8, out)
        return f
    def ui_message(self):
        fmt = self.text(self.r(self.arg(2)) if False else self.arg(2))
        n = fmt.split('|')[-1].count('s') if '|' in fmt else 0
        self.ev('ui_message', format=fmt, args=[self.text(self.arg(3 + i)) for i in range(n)])
        self.ret()
    def process_stub(self):
        o = self.ecx()
        self.ev('process', descriptor=309, player=self.pid(self.r(o + 4)), reason=self.r(o + 8)); self.ret()
    def peer_name(self, p):
        if p == self.admin: return 'node'
        for i, c in enumerate(self.conns):
            if p == c: return i
        return hex(p)
    def hist_name(self, p):
        for off, n in NODE_HIST.items():
            if p == self.admin + off: return 'node.' + n
        for i, c in enumerate(self.conns):
            for off, n in HIST.items():
                if p == c + off: return f'conn{i}.' + n
        return hex(p)
    def chan(self, op, pop, **kw):
        this = self.ecx(); who = None
        for i, c in enumerate(self.conns):
            if c <= this < c + 0x424: who = (i, CHANNELS.get(this - c, hex(this - c)))
        self.ev('channel', conn=who[0], channel=who[1], op=op, **kw); self.ret(pop)
    def hook(self, u, pc, size, user):
        self.visited[pc] = size
        f = self.stubs.get(pc)
        if f is not None:
            f(); return
        super().hook(u, pc, size, user)
    # --- memory helpers ------------------------------------------------
    def f(self, p, v): self.u.mem_write(p, struct.pack('<f', v))
    def fb(self, p, bits): self.w(p, bits)
    def b(self, p, v): self.u.mem_write(p, bytes([int(v) & 0xff]))
    def rb(self, p): return self.u.mem_read(p, 1)[0]
    def fpu(self, precision): self.u.reg_write(UC_X86_REG_FPCW, PC[precision])
    def history(self, base, h):
        begin = h['begin']; packets = h['packets']; cap = max(h.get('capacity', 4), 1, len(packets))
        alloc = self.alloc(4 + cap * 4); self.w(alloc, cap); arr = alloc + 4
        self.w(base, begin); self.w(base + 4, begin + len(packets)); self.w(base + 8, cap); self.w(base + 12, arr)
        for i, data in enumerate(packets):
            if data is not None:
                self.w(arr + (((begin + i) + 0x80000000) & 0xffffffff) % cap * 4, self.packet(data))
    def read_history(self, base):
        begin = s32(self.r(base)); end = s32(self.r(base + 4)); cap = self.r(base + 8); arr = self.r(base + 12)
        out = []
        for i in range(begin, end):
            p = self.r(arr + ((i + 0x80000000) & 0xffffffff) % cap * 4)
            out.append(None if p == 0 else list(self.u.mem_read(p, self.r(p - 4))))
        return {'begin': begin, 'packets': out}
    def player(self, ident, film=0):
        if ident is None: return 0
        if ident not in self.players:
            p = self.alloc(0x40); self.w(p + 0x20, ident); self.b(p + 0xe, film); self.players[ident] = p
        return self.players[ident]
    # --- state ---------------------------------------------------------
    def setup(self, st):
        """st: fixture input (see native-heartbeat.json 'before')."""
        self.heap = self.mark; self.players = {}; self.hb_events = []; self.ack = {}; self.file = {}
        self.u.mem_write(self.admin, b'\0' * 0x1000); self.u.mem_write(NETNODE, b'\0' * 0x100)
        for flag in (0xb408e0, 0xb408e8, 0xb408f0, 0xb40900): self.w(flag, 0)  # static build streams re-created per case
        self.u.mem_write(self.session, b'\0' * 0x100)
        env = st['env']
        self.f(NETNODE + 0x58, b2f(env['now'])); self.b(NETNODE + 0x64, env['falling_behind']); self.w(NETNODE + 8, LOCK_VT)
        self.w(self.session + 0xe4, env['session_state'])
        known = env.get('known_players', [])
        prev = 0
        for ident in reversed(known):
            n = self.alloc(8); self.w(n, self.player(ident)); self.w(n + 4, prev); prev = n
        self.w(self.session + 0xbc, prev)
        for ident in env.get('film_players', []):
            self.b(self.player(ident) + 0xe, 1)
        gs = env['gamespy']
        self.b(GAMESPY_A + 0xbc, gs); self.w(GAMESPY_B + 0xb8, 1 if gs else 0)
        cfg = env['config']
        table = self.alloc(0x200 * 4)
        for i in range(0x200): self.w(table + 4 * i, 0)
        def var(idx, bits):
            p = self.alloc(4); self.w(p, bits); self.w(AVARS + 4 * idx, p)
        self.u.mem_write(AVARS, b'\0' * 0x800)
        var(0x47, cfg['async_ticks_per_second']); var(0x4b, cfg['broadcast_decree_behind'] & 0xffffffff)
        var(0x4f, cfg['backup_decree_behind'] & 0xffffffff)
        p = self.alloc(4); self.w(p, cfg['admin_ticks_per_second']); self.w(AVARS2 + 0x20, p)
        self.f(FLD_VALUE, 0.0); self.w(FLD_VALUE, env['old_clock_time'])
        self.active = env['request_active']; self.local_player = self.player(env['local_player'])
        self.w(0xb40934, env['heartbeat_counter'] & 0xffffffff)
        # streams
        for s in (REQ, DEC):
            buf = self.packet([0] * 4096); self.w(s, 0xa39da8); self.w(s + 4, buf); self.w(s + 8, 0); self.w(s + 12, 3)
        node = st['node']; a = self.admin
        self.conns = [CONN0 + i * CONN_STRIDE for i in range(len(st['conns']))]
        self.w(a, NODE_VT); self.w(a + 0x88, 0xa913ac)
        self.peer_base(a, node['base'])
        self.w(a + 0x90, node['task_status'])
        self.w(a + 0x158, (a + 0x15c) if node['has_admin_clock'] else 0)
        self.w(a + 0x16c, CLOCK_VT)
        self.history(a + 0x214, node['decrees']); self.history(a + 0x224, node['backup_decrees'])
        self.w(a + 0x234, node['next_backup_decree'] & 0xffffffff)
        self.w(a + 0x2b8, node['vote_count'] & 0xffffffff); self.fb(a + 0x2bc, node['max_patience'])
        self.w(a + 0x2c4, self.ref(node['host'])); self.w(a + 0x2c8, self.ref(node['server']))
        self.w(a + 0x2e0, node['took_over'] & 0xffffffff)
        for i, c in enumerate(st['conns']):
            p = self.conns[i]; nc = NETCONN0 + i * NETCONN_STRIDE
            self.u.mem_write(p, b'\0' * 0x424); self.u.mem_write(nc, b'\0' * 0xa4)
            self.w(p, CONN_VT); self.peer_base(p, c['base'])
            self.b(p + 0x88, c['connected']); self.b(p + 0x89, c['disconnecting']); self.w(p + 0x8c, c['disconnect_reason'])
            self.fb(p + 0x90, c['disconnect_time']); self.w(p + 0x94, c['field_94']); self.w(p + 0x98, c['field_98'])
            self.b(p + 0x9c, c['disconnect_requested']); self.w(p + 0xa0, c['backup_index'] & 0xffffffff)
            self.w(p + 0xa4, c['reverse_acked'] & 0xffffffff); self.b(p + 0xa8, c['request_backlog']); self.b(p + 0xa9, c['falling_behind'])
            self.b(p + 0xaa, c['field_aa'])
            self.history(p + 0xac, c['private_decrees']); self.history(p + 0xbc, c['reverse_decrees'])
            self.ack[p + 0xcc] = c['broadcast_decree_acked']; self.ack[p + 0x2ac] = c['backup_decree_acked']
            self.file[p + 0x39c] = (c['file_sent'], c['file_size'])
            self.fb(p + 0x410, c['last_update']); self.w(p + 0x414, nc)
            self.w(p + 0x418, c['file_generation'] & 0xffffffff); self.b(p + 0x420, c['field_420'])
            n = c['net']
            self.fb(nc + 0x2c, n['last_advance']); self.fb(nc + 0x38, n['rtt'])
            flags = n['received_flags']; cap = max(1, len(flags))
            data = self.alloc(cap + 4)
            self.w(nc + 0x1c, n['received_begin'] & 0xffffffff); self.w(nc + 0x20, (n['received_begin'] + len(flags)) & 0xffffffff)
            self.w(nc + 0x24, cap); self.w(nc + 0x28, data)
            for k, fl in enumerate(flags):
                self.b(data + ((n['received_begin'] + k + 0x80000000) & 0xffffffff) % cap, fl)
        # peer list (+0x2AC)
        prev = 0
        for ref in reversed(st['list']):
            n = self.alloc(8); self.w(n, self.ref(ref)); self.w(n + 4, prev); prev = n
        self.w(a + 0x2ac, prev)
    def conns_for(self, st): return [CONN0 + i * CONN_STRIDE for i in range(len(st['conns']))]
    def ref_for(self, st, r): return self.admin if r == 'node' else self.conns_for(st)[r]
    def ref(self, r):
        if r is None: return 0
        if r == 'node': return self.admin
        return self.conns[r]
    def peer_base(self, p, b):
        self.w(p + 4, b['field_04']); self.w(p + 8, self.player(b['player']))
        self.history(p + 0xc, b['requests']); self.history(p + 0x1c, b['backup_requests'])
        self.w(p + 0x2c, self.player(b['ordering_player']))
        for off, k in [(0x30, 'flag_30'), (0x31, 'flag_31'), (0x32, 'field_32'), (0x48, 'flag_48'), (0x49, 'flag_49'),
                       (0x4a, 'flag_4a'), (0x4b, 'flag_4b'), (0x54, 'field_54'), (0x55, 'vote'), (0x56, 'field_56'), (0x6c, 'field_6c')]:
            self.b(p + off, b[k])
        for off, k in [(0x34, 'field_34'), (0x38, 'transfer_status'), (0x3c, 'field_3c'), (0x40, 'field_40'),
                       (0x44, 'patience'), (0x4c, 'rtt'), (0x50, 'received_fraction'), (0x58, 'field_58'), (0x5c, 'field_5c'),
                       (0x60, 'frame_ratio'), (0x70, 'join_order')]:
            self.w(p + off, b[k] & 0xffffffff)
        prev = 0; tail = 0
        for ident in reversed(b.get('vote_to_kick', [])):
            n = self.alloc(8); self.w(n, self.player(ident)); self.w(n + 4, prev)
            if prev == 0: tail = n
            prev = n
        self.w(p + 0x64, prev); self.w(p + 0x68, tail)
    def read_base(self, p):
        b = {'field_04': self.r(p + 4), 'player': self.pid(self.r(p + 8)), 'requests': self.read_history(p + 0xc),
             'backup_requests': self.read_history(p + 0x1c), 'ordering_player': self.pid(self.r(p + 0x2c))}
        for off, k in [(0x30, 'flag_30'), (0x31, 'flag_31'), (0x32, 'field_32'), (0x48, 'flag_48'), (0x49, 'flag_49'),
                       (0x4a, 'flag_4a'), (0x4b, 'flag_4b'), (0x54, 'field_54'), (0x55, 'vote'), (0x56, 'field_56'), (0x6c, 'field_6c')]:
            b[k] = self.rb(p + off)
        for off, k in [(0x34, 'field_34'), (0x38, 'transfer_status'), (0x3c, 'field_3c'), (0x40, 'field_40'),
                       (0x44, 'patience'), (0x4c, 'rtt'), (0x50, 'received_fraction'), (0x58, 'field_58'), (0x5c, 'field_5c'),
                       (0x60, 'frame_ratio'), (0x70, 'join_order')]:
            v = self.r(p + off)
            b[k] = s32(v) if k in ('field_34', 'transfer_status', 'field_3c', 'field_40', 'field_5c', 'join_order') else v
        kick = []; n = self.r(p + 0x64)
        while n:
            kick.append(self.pid(self.r(n))); n = self.r(n + 4)
        b['vote_to_kick'] = kick
        return b
    def unref(self, p):
        if p == 0: return None
        if p == self.admin: return 'node'
        return self.conns.index(p) if p in self.conns else hex(p)
    def snapshot(self):
        a = self.admin
        node = {'base': self.read_base(a), 'task_status': self.r(a + 0x90),
                'has_admin_clock': self.r(a + 0x158) != 0, 'admin_clock_own': self.r(a + 0x158) == a + 0x15c,
                'decrees': self.read_history(a + 0x214), 'backup_decrees': self.read_history(a + 0x224),
                'next_backup_decree': s32(self.r(a + 0x234)), 'vote_count': s32(self.r(a + 0x2b8)),
                'max_patience': self.r(a + 0x2bc), 'host': self.unref(self.r(a + 0x2c4)),
                'server': self.unref(self.r(a + 0x2c8)), 'took_over': s32(self.r(a + 0x2e0))}
        conns = []
        for p in self.conns:
            c = {'base': self.read_base(p), 'connected': self.rb(p + 0x88), 'disconnecting': self.rb(p + 0x89),
                 'disconnect_reason': self.r(p + 0x8c), 'disconnect_time': self.r(p + 0x90), 'field_94': self.r(p + 0x94),
                 'field_98': self.r(p + 0x98), 'disconnect_requested': self.rb(p + 0x9c), 'backup_index': s32(self.r(p + 0xa0)),
                 'reverse_acked': s32(self.r(p + 0xa4)), 'request_backlog': self.rb(p + 0xa8), 'falling_behind': self.rb(p + 0xa9),
                 'field_aa': self.rb(p + 0xaa), 'private_decrees': self.read_history(p + 0xac),
                 'reverse_decrees': self.read_history(p + 0xbc), 'last_update': self.r(p + 0x410),
                 'file_generation': s32(self.r(p + 0x418)), 'field_420': self.rb(p + 0x420)}
            conns.append(c)
        def stream(s):
            pos = self.position(s); n = (pos['byte'] * 8 + pos['bit'] + 7) // 8
            return {'bytes': list(self.u.mem_read(self.r(s + 4), n)), 'end': pos}
        return {'node': node, 'conns': conns, 'heartbeat_counter': s32(self.r(0xb40934)),
                'request_stream': stream(REQ), 'decree_stream': stream(DEC)}


# ---------------------------------------------------------------------------
# Fixture state generators
# ---------------------------------------------------------------------------
def hist(begin=0, packets=(), capacity=4):
    packets = [None if p is None else list(p) for p in packets]
    return {'begin': begin, 'packets': packets, 'capacity': max(capacity, len(packets), 1)}

def base(rng=None, **kw):
    b = {'field_04': 0, 'player': None, 'requests': hist(), 'backup_requests': hist(), 'ordering_player': None,
         'flag_30': 0, 'flag_31': 0, 'field_32': 0, 'flag_48': 0, 'flag_49': 0, 'flag_4a': 0, 'flag_4b': 0,
         'field_54': 0, 'vote': 0, 'field_56': 0, 'field_6c': 1, 'field_34': 0, 'transfer_status': 0, 'field_3c': 0,
         'field_40': 0, 'patience': f2b(15.0), 'rtt': 0, 'received_fraction': 0, 'field_58': f2b(-10000.0),
         'field_5c': 0, 'frame_ratio': f2b(0.0001), 'join_order': 0}
    if rng is not None:
        for k in ['flag_30', 'flag_31', 'flag_48', 'flag_49', 'flag_4a', 'flag_4b', 'vote', 'field_32', 'field_54', 'field_56']:
            b[k] = int(rng.random() < 0.3)
        b['field_04'] = rng.getrandbits(32); b['field_34'] = rng.randrange(-5, 50); b['field_3c'] = rng.randrange(0, 9)
        b['field_40'] = rng.randrange(0, 9); b['field_5c'] = rng.randrange(0, 9); b['join_order'] = rng.randrange(0, 6)
        b['rtt'] = f2b(rng.choice([0.0, 0.05, 0.123, 1.5])); b['received_fraction'] = f2b(rng.random())
        b['frame_ratio'] = f2b(rng.choice([0.0001, 0.5, 1.25, 3.0]))
        b['patience'] = f2b(rng.choice([15.0, 14.95, 0.1, 0.05, 0.0, 7.3, 0.10000001]))
        b['field_6c'] = rng.randrange(2); b['transfer_status'] = rng.choice([0, 1, 2, 3])
        b['field_58'] = f2b(rng.choice([-10000.0, 1.0]))
    b.update(kw)
    return b

def conn(rng=None, **kw):
    c = {'base': base(rng), 'connected': 1, 'disconnecting': 0, 'disconnect_reason': 13, 'disconnect_time': 0,
         'field_94': 0, 'field_98': 0, 'disconnect_requested': 0, 'backup_index': 0, 'reverse_acked': -1,
         'request_backlog': 0, 'falling_behind': 0, 'field_aa': 0, 'private_decrees': hist(), 'reverse_decrees': hist(),
         'broadcast_decree_acked': 0, 'backup_decree_acked': 0, 'file_sent': 0, 'file_size': 0,
         'last_update': 0, 'file_generation': 0, 'field_420': 0,
         'net': {'last_advance': 0, 'rtt': 0, 'received_begin': 0, 'received_flags': []}}
    if rng is not None:
        for k in ['connected', 'disconnect_requested', 'request_backlog', 'falling_behind', 'field_aa', 'field_420']:
            c[k] = int(rng.random() < (0.8 if k == 'connected' else 0.3))
        c['backup_index'] = rng.randrange(0, 4); c['reverse_acked'] = rng.randrange(-1, 5)
        c['broadcast_decree_acked'] = rng.randrange(-3, 60); c['backup_decree_acked'] = rng.randrange(-3, 90)
        c['field_94'] = rng.randrange(0, 5); c['field_98'] = rng.randrange(0, 5); c['file_generation'] = rng.randrange(0, 4)
        n = rng.choice([0, 1, 5, 33, 100]); rb = rng.randrange(0, 50)
        c['net'] = {'last_advance': f2b(rng.choice([0.0, 1.0, 9.5])), 'rtt': f2b(rng.choice([0.0, 0.0625, 0.3, 3.0])),
                    'received_begin': rb, 'received_flags': [int(rng.random() < 0.75) for _ in range(n)]}
    c.update(kw)
    return c

def env(**kw):
    e = {'now': f2b(10.0), 'falling_behind': 0, 'session_state': 2, 'known_players': [], 'film_players': [],
         'gamespy': 0, 'request_active': 1, 'local_player': 1, 'heartbeat_counter': 0, 'old_clock_time': f2b(0.0),
         'config': {'async_ticks_per_second': f2b(16.0), 'broadcast_decree_behind': 32, 'backup_decree_behind': 64,
                    'admin_ticks_per_second': f2b(16.0)}}
    e.update(kw)
    return e

def node(rng=None, **kw):
    n = {'base': base(rng, player=1), 'task_status': 3, 'has_admin_clock': 1, 'decrees': hist(),
         'backup_decrees': hist(), 'next_backup_decree': -1, 'vote_count': 0, 'max_patience': 0,
         'host': 'node', 'server': 'node', 'took_over': 0}
    n.update(kw)
    return n


# ---------------------------------------------------------------------------
# Case runners
# ---------------------------------------------------------------------------
def run(m, kind, st, precision, fn, this, args=(), pop=0, label=''):
    m.setup(st); m.fpu(precision)
    m.call(fn, list(args), this=this, callee_pop=pop)
    return {'kind': kind, 'label': label, 'precision': precision, 'before': st, 'events': m.hb_events, 'after': m.snapshot()}

def native_packet(m, fields):
    """Input packets are written with the ORIGINAL 0x40D420 writer into a fresh
    stream at byte 0 bit 3 and finalized like 0x74D0A0. fields: ('u', value,
    width) or ('text', units)."""
    buf = m.packet([0] * 4096); s = SCRATCH
    m.w(s, 0xa39da8); m.w(s + 4, buf); m.w(s + 8, 0); m.w(s + 12, 3)
    for f in fields:
        if f[0] == 'u':
            v = m.alloc(4); m.w(v, f[1] & 0xffffffff); m.call(0x40d420, [s, v, f[2]])
        else:
            units = f[1]; v = m.alloc(4); m.w(v, len(units)); m.call(0x40d420, [s, v, 16])
            if units:
                d = m.alloc(2 * len(units) + 2); m.u.mem_write(d, b''.join(struct.pack('<H', u) for u in units))
                m.call(0x40d420, [s, d, 16 * len(units)])
    pos = m.position(s); bits = pos['byte'] * 8 + pos['bit']; n = (bits + 7) // 8
    data = bytearray(m.u.mem_read(m.r(s + 4), n)); data[0] |= (8 - bits % 8) & 7
    return list(data)

def units(text): return [ord(c) for c in text]

def chat_request(m, text): return native_packet(m, [('u', 0, 1), ('text', units(text))])
def vote_request(m, v): return native_packet(m, [('u', 1, 1), ('u', v, 1)])
def chat_decree(m, text, player): return native_packet(m, [('u', 1, 2), ('text', units(text)), ('u', player, 32)])
def odd_decree(m): return native_packet(m, [('u', 2, 2), ('u', 0x1234, 16)])
def status_decree(m, rng, records):
    f = [('u', 0, 2), ('u', rng.randrange(0, 5), 32), ('u', f2b(rng.choice([0.0, 0.1, 3.5, 15.0])), 32)]
    for _ in range(records):
        f += [('u', rng.randrange(2), 1) for _ in range(4)]
        f += [('u', f2b(rng.choice([0.0, 0.0625, 0.25])), 32), ('u', f2b(rng.random()), 32)]
        f += [('u', rng.randrange(2), 1), ('u', rng.randrange(2), 1), ('u', rng.getrandbits(32), 32)]
    return native_packet(m, f)

def refs(nconn): return ['node'] + list(range(nconn))

def hb_state(m, rng, i):
    nconn = rng.randrange(0, 5)
    lst = refs(nconn)
    if nconn and rng.random() < 0.15: lst = list(range(nconn)) + ['node']
    server = rng.choice(['node'] * 3 + list(range(nconn)))
    host = rng.choice(['node'] * 3 + list(range(nconn)) + [None])
    known = [1] + [10 + k for k in range(nconn)] + [77]
    known = [p for p in known if rng.random() < 0.85]
    film = [p for p in known if rng.random() < 0.1]
    is_server = server == 'node'
    def requests(rng):
        out = []
        for _ in range(rng.choice([0, 0, 1, 2, 3])):
            if rng.random() < 0.5: out.append(chat_request(m, rng.choice(['', 'hi', 'lag!', 'x' * 20])))
            else: out.append(vote_request(m, rng.randrange(2)))
        return hist(rng.randrange(-3, 10), out, capacity=rng.choice([1, 2, 4, 8]))
    conns = []
    for k in range(nconn):
        c = conn(rng, base=base(rng, player=None if rng.random() < 0.15 else 10 + k, backup_requests=requests(rng),
                                ordering_player=rng.choice([None, 10 + k])))
        c['base']['vote_to_kick'] = rng.choice([[], [1], [10, 11]])
        conns.append(c)
    nd_base = base(rng, player=None if rng.random() < 0.1 else 1, backup_requests=requests(rng))
    dbegin = rng.randrange(0, 30); dpk = [[0x08, k] for k in range(rng.randrange(0, 4))]
    for c in conns:
        rev = rng.randrange(0, 4)
        rb = rng.randrange(0, dbegin + len(dpk) + 1)
        span = (dbegin + len(dpk)) - rb + rev
        c['reverse_decrees'] = hist(rb, [[0x18, rb + j] if rng.random() < 0.9 else None for j in range(max(span, 0))], capacity=8)
        c['reverse_acked'] = rng.randrange(rb - 2, rb + span + 2)
        c['broadcast_decree_acked'] = rng.randrange(dbegin - 40, dbegin + len(dpk) + 2)
        if rng.random() < 0.4:
            c['base']['transfer_status'] = 2; c['file_size'] = rng.choice([0, 64, 1000, 12345, 7])
            c['file_sent'] = rng.choice([c['file_size'], 0, -128, c['file_size'] // 3, 64])
    records = len(lst)
    bd = []
    for _ in range(rng.choice([0, 0, 1, 2, 3])):
        k = rng.random()
        if k < 0.45 and not is_server: bd.append(status_decree(m, rng, records))
        elif k < 0.45: bd.append(status_decree(m, rng, rng.randrange(0, 3)))
        elif k < 0.9: bd.append(chat_decree(m, rng.choice(['gg', '', 'wait']), rng.choice([1, 10, 11, 77, 0, 99])))
        else: bd.append(odd_decree(m))
    bbegin = rng.randrange(0, 250)
    nxt = rng.choice([-1, -1, bbegin, bbegin + len(bd), bbegin + min(1, len(bd))])
    nd = node(rng, base=nd_base, host=host, server=server, task_status=rng.randrange(0, 5),
              has_admin_clock=int(rng.random() < 0.85), backup_decrees=hist(bbegin, bd, capacity=rng.choice([2, 4, 8])),
              next_backup_decree=nxt, decrees=hist(dbegin, dpk, capacity=8), vote_count=rng.randrange(0, 3),
              max_patience=f2b(rng.choice([0.0, 2.5])), took_over=0)
    e = env(now=f2b(rng.choice([10.0, 123.5, 3.25])), falling_behind=rng.choice([0, 1, 2]),
            session_state=rng.choice([1, 2, 2]), known_players=known, film_players=film,
            gamespy=int(rng.random() < 0.4), request_active=int(rng.random() < 0.9), local_player=rng.choice([1, 50]),
            heartbeat_counter=rng.choice([0, 3, 15, 16, 16, 17, 40, -2]), old_clock_time=f2b(rng.choice([0.0, 12.375, 99.0625])),
            config={'async_ticks_per_second': f2b(16.0), 'broadcast_decree_behind': rng.choice([32, 2]),
                    'backup_decree_behind': rng.choice([64, 3]), 'admin_ticks_per_second': f2b(rng.choice([16.0, 10.0, 3.0]))})
    return {'env': e, 'node': nd, 'conns': conns, 'list': lst}

def lost_player_state(m, rng, variant):
    """Targeted DisconnectLostPlayers scenarios (server node)."""
    nconn = 3
    conns = [conn(None, base=base(None, player=10 + k)) for k in range(nconn)]
    nd = node(None, host='node', server='node', has_admin_clock=1)
    e = env(session_state=2, known_players=[1, 10, 11, 12], heartbeat_counter=3, gamespy=1)
    st = {'env': e, 'node': nd, 'conns': conns, 'list': refs(nconn)}
    lag = conns[1]['base']; lag['flag_49'] = 1; lag['patience'] = f2b(0.05)
    if variant == 'quit_requested':
        conns[1]['disconnect_requested'] = 1
    elif variant == 'votes_enough':
        nd['base']['vote'] = 1; conns[0]['base']['vote'] = 1; conns[2]['base']['vote'] = 1
    elif variant == 'votes_short':
        nd['base']['vote'] = 1
    elif variant == 'patience_high':
        nd['base']['vote'] = 1; conns[0]['base']['vote'] = 1; conns[2]['base']['vote'] = 1; lag['patience'] = f2b(0.10000001)
    elif variant == 'patience_equal':
        nd['base']['vote'] = 1; conns[0]['base']['vote'] = 1; conns[2]['base']['vote'] = 1; lag['patience'] = f2b(0.1)
    elif variant == 'staging_vote':
        e['session_state'] = 1; nd['base']['vote'] = 1
    elif variant == 'staging_no_vote':
        e['session_state'] = 1; conns[0]['base']['vote'] = 1
    elif variant == 'votes_via_requests':
        conns[0]['base']['backup_requests'] = hist(0, [vote_request(m, 1)]); nd['base']['backup_requests'] = hist(5, [vote_request(m, 1), chat_request(m, 'kick')])
        conns[2]['base']['backup_requests'] = hist(2, [vote_request(m, 1), vote_request(m, 0)])
    elif variant == 'backup_server_takeover':
        nd['host'] = 0; conns[0]['base']['flag_4a'] = 1; conns[0]['connected'] = 0
        conns[1]['disconnect_requested'] = 1; nd['task_status'] = 2
        nd['decrees'] = hist(4, [[8, 1], [8, 2]], capacity=8)
        conns[2]['reverse_decrees'] = hist(3, [[0x18, 3], [0x18, 4], [0x18, 5], [0x18, 6]], capacity=8); conns[2]['reverse_acked'] = 9
        conns[1]['reverse_decrees'] = hist(0, [[0x18, 0]] * 8, capacity=8); conns[1]['connected'] = 1
    elif variant == 'backup_server_not_current':
        nd['host'] = 0; conns[2]['reverse_decrees'] = hist(0, [[1, 1]], capacity=4); conns[2]['reverse_acked'] = 1
    elif variant == 'nobody_lagging':
        lag['flag_49'] = 0; nd['base']['vote'] = 1; conns[2]['base']['vote'] = 1
    elif variant == 'no_admin_clock':
        nd['has_admin_clock'] = 0
    return st

def heartbeat_cases(m, rng):
    rows = []
    variants = ['quit_requested', 'votes_enough', 'votes_short', 'patience_high', 'patience_equal', 'staging_vote',
                'staging_no_vote', 'votes_via_requests', 'backup_server_takeover', 'backup_server_not_current',
                'nobody_lagging', 'no_admin_clock']
    for v in variants:
        st = lost_player_state(m, rng, v)
        for precision in ['Single', 'Double']:
            rows.append(run(m, 'heartbeat', st, precision, 0x75b820, m.admin, label=v))
    for i in range(160):
        st = hb_state(m, rng, i)
        precision = rng.choice(['Single', 'Double', 'Extended'])
        rows.append(run(m, 'heartbeat', st, precision, 0x75b820, m.admin, label=f'random{i}'))
    return rows

def update_state(rng, label):
    nconn = rng.randrange(1, 4)
    conns = [conn(rng, base=base(rng, player=10 + k)) for k in range(nconn)]
    i = rng.randrange(nconn)
    server = rng.choice(['node', 'node', i] + list(range(nconn)))
    host = rng.choice(['node', i, None] + list(range(nconn)))
    nd = node(rng, host=host, server=server, has_admin_clock=int(rng.random() < 0.85),
              base=base(rng, player=None if rng.random() < 0.1 else 1, join_order=rng.randrange(0, 4)),
              backup_decrees=hist(rng.randrange(0, 20), [[0], [1]], capacity=4), next_backup_decree=rng.choice([-1, 3]),
              decrees=hist(rng.randrange(0, 20), [[2]], capacity=4))
    now = rng.choice([10.0, 2.0000002384185791, 100.25, 1e-30, 3.0])
    c = conns[i]
    c['last_update'] = f2b(rng.choice([now - 0.0625, now - 1.0, now - 1.0000001, now - 5.0, now + 0.5, now, now - 0.3333]))
    c['disconnecting'] = int(rng.random() < 0.35)
    c['disconnect_time'] = f2b(rng.choice([now - 2.0, now - 2.0000005, now - 1.9, now - 7.0, 2.384185791015625e-07 - 9.313225746154785e-10]))
    c['net']['last_advance'] = f2b(rng.choice([now - 2.0, now - 3.0, now - 2.5, now - 3.0000002, now - 0.1,
                                                2.384185791015625e-07 - 9.313225746154785e-10, now - 10.0]))
    c['base']['patience'] = f2b(rng.choice([15.0, 14.99, 14.9999, 0.03, 0.0, 7.123456, 1e-8, 100.0]))
    for cc in conns:
        cc['base']['join_order'] = rng.randrange(0, 4)
        cc['base']['backup_requests'] = hist(rng.randrange(0, 5), [[0x10]] if rng.random() < 0.5 else [], capacity=2)
        cc['private_decrees'] = hist(rng.randrange(0, 5), [[0x20]] if rng.random() < 0.5 else [], capacity=2)
        cc['base']['requests'] = hist(rng.randrange(0, 5), [[0x30]] if rng.random() < 0.5 else [], capacity=2)
        cc['base']['vote_to_kick'] = rng.choice([[], [10]])
    e = env(now=f2b(now), falling_behind=rng.choice([0, 1]))
    return {'env': e, 'node': nd, 'conns': conns, 'list': refs(nconn)}, i

def update_cases(m, rng):
    rows = []
    for k in range(150):
        st, i = update_state(rng, k)
        for precision in ['Single', 'Double', 'Extended']:
            rows.append(dict(run(m, 'update', st, precision, 0x75bd70, m.conns_for(st)[i], label=f'random{k}'), conn=i))
    # precision edges: difference 2 + 2^-30 (PC24 rounds to 2.0, PC53/PC64 do not).
    for which in ['disconnect', 'liveness', 'server_liveness']:
        st, i = update_state(random.Random(5), 'edge')
        c = st['conns'][i]; nowv = 2.0000002384185791
        st['env']['now'] = f2b(nowv); c['last_update'] = f2b(nowv)
        st['node']['has_admin_clock'] = 1; st['node']['base']['player'] = 1; c['connected'] = 1
        c['disconnecting'] = 0; c['net']['last_advance'] = f2b(nowv - 0.5)
        tiny = 2.384185791015625e-07 - 9.313225746154785e-10
        if which == 'disconnect':
            c['disconnecting'] = 1; c['disconnect_time'] = f2b(tiny)
        elif which == 'liveness':
            st['node']['server'] = 'node'; c['net']['last_advance'] = f2b(tiny)
        else:
            nowv = 3.0000002384185791; st['env']['now'] = f2b(nowv); c['last_update'] = f2b(nowv)
            st['node']['server'] = i; c['net']['last_advance'] = f2b(tiny)
        for precision in ['Single', 'Double', 'Extended']:
            rows.append(dict(run(m, 'update', st, precision, 0x75bd70, m.conns_for(st)[i], label='edge_' + which), conn=i))
    # patience rounding: many dt/patience pairs under every precision.
    prng = random.Random(44)
    for k in range(80):
        st, i = update_state(prng, 'patience')
        c = st['conns'][i]; st['node']['server'] = 'node'
        for f in ['flag_30', 'flag_49', 'flag_4a', 'flag_4b']: c['base'][f] = 0
        nowv = prng.uniform(1, 1000); st['env']['now'] = f2b(nowv)
        c['last_update'] = f2b(nowv - prng.uniform(0, 1.2)); c['base']['patience'] = f2b(prng.uniform(0, 15))
        c['disconnecting'] = 0
        for precision in ['Single', 'Double', 'Extended']:
            rows.append(dict(run(m, 'update', st, precision, 0x75bd70, m.conns_for(st)[i], label='patience'), conn=i))
    return rows

RATE_BITS = 0xCCCCCD  # 0.1f significand (0xA8C0BC = 0x3DCCCCCD)

def precision_patience_inputs():
    """(dt, patience) pairs whose patience increase differs between PC24 and
    PC53. Three were found by exact random search; the rest are constructed
    with 0.1f = (2^26+1)/(5*2^27): dt = 80q*2^-24 (q odd) gives the product
    q*2^-21 + q*2^-47, so patience + product lies 2^-47*q above a binary32
    midpoint: PC24 rounds the product to q*2^-21 first (exact tie -> even),
    PC53/PC64 keep the excess (-> upper neighbour). A PC53/PC64 difference
    needs the exact sum within 2^-53 (relative) of a midpoint but not on it;
    exhaustive and constructive searches with the 0.1f factor found none
    (heartbeat-precision.txt)."""
    pairs = [(0.00035762786865234375, 8.823973655700684), (0.014595985412597656, 14.878662109375),
             (0.013308525085449219, 13.42258358001709)]
    rng = random.Random(9)
    while len(pairs) < 8:
        q = rng.randrange(104858, 209715) | 1
        P = rng.randrange(8 << 20, 14 << 20)
        if (P + (q - 1) // 2) % 2: P += 1        # ties-to-even must pick the lower neighbour
        pairs.append((80 * q * 2.0 ** -24, P * 2.0 ** -20))
    return pairs

def precision_update_cases(m):
    rows = []
    for k, (dt, p) in enumerate(precision_patience_inputs()):
        st, i = update_state(random.Random(100 + k), 'precision')
        c = st['conns'][i]; st['node']['server'] = 'node'
        for f in ['flag_30', 'flag_49', 'flag_4a', 'flag_4b']: c['base'][f] = 0
        st['env']['now'] = f2b(dt); c['last_update'] = f2b(0.0); c['base']['patience'] = f2b(p)
        c['disconnecting'] = 0; c['net']['last_advance'] = f2b(dt)
        for precision in ['Single', 'Double', 'Extended']:
            rows.append(dict(run(m, 'update', st, precision, 0x75bd70, m.conns_for(st)[i], label=f'patience_precision{k}'), conn=i))
    # PC64 comparison edges: now - t = c + 2^-60 rounds to c at PC24/PC53 only.
    for which, nowv in [('disconnect', 2.0), ('liveness', 2.0), ('server_liveness', 3.0)]:
        st, i = update_state(random.Random(6), 'edge64')
        c = st['conns'][i]
        st['env']['now'] = f2b(nowv); c['last_update'] = f2b(nowv)
        st['node']['has_admin_clock'] = 1; st['node']['base']['player'] = 1; c['connected'] = 1
        c['disconnecting'] = 0; c['net']['last_advance'] = f2b(nowv - 0.5)
        if which == 'disconnect':
            c['disconnecting'] = 1; c['disconnect_time'] = f2b(-2.0 ** -60)
        elif which == 'liveness':
            st['node']['server'] = 'node'; c['net']['last_advance'] = f2b(-2.0 ** -60)
        else:
            st['node']['server'] = i; c['net']['last_advance'] = f2b(-2.0 ** -60)
        for precision in ['Single', 'Double', 'Extended']:
            rows.append(dict(run(m, 'update', st, precision, 0x75bd70, m.conns_for(st)[i], label='edge64_' + which), conn=i))
    return rows

def sethost_cases(m, rng):
    rows = []
    for k in range(40):
        st, i = update_state(rng, k)
        st['node']['host'] = rng.choice(['node', None, 0])
        st['env']['old_clock_time'] = f2b(rng.choice([0.0, 5.5, 1234.0625]))
        st['env']['config']['admin_ticks_per_second'] = f2b(rng.choice([16.0, 3.0, 10.0, 0.0]))
        for c in st['conns']: c['reverse_acked'] = rng.randrange(-1, 9)
        target = rng.choice(['node', 'node', i])
        st['target'] = target
        rows.append(run(m, 'set_host', st, 'Double', 0x757010, m.admin, [m.ref_for(st, target)], 4, label=f'random{k}'))
    return rows

def setserver_cases(m, rng):
    rows = []
    for k in range(40):
        st, i = update_state(rng, k)
        target = rng.choice(['node', 'node', i])
        st['target'] = target
        rows.append(run(m, 'set_server', st, rng.choice(['Single', 'Double']), 0x759790, m.admin, [m.ref_for(st, target)], 4, label=f'random{k}'))
    return rows

def sender_cases(m, rng):
    rows = []
    for k, text in enumerate(['', 'hello', 'x' * 40, 'é中!']):
        st = {'env': env(), 'node': node(None, base=base(None, player=1, backup_requests=hist(rng.choice([0, 990, 1000, 1500]), [[8]] * rng.randrange(0, 3), capacity=4))),
              'conns': [], 'list': ['node']}
        st['text'] = units(text)
        m.setup(st); m.fpu('Double')
        m.call(0x753410, [m.string(text)], this=m.admin, callee_pop=4)
        rows.append({'kind': 'send_chat', 'label': f'chat{k}', 'precision': 'Double', 'before': st, 'events': m.hb_events, 'after': m.snapshot()})
    for k, (vote, begin) in enumerate([(0, 0), (1, 0), (1, 999), (0, 1000), (1, 5000)]):
        st = {'env': env(), 'node': node(None, base=base(None, player=1, backup_requests=hist(begin, [[8]] * 2, capacity=2))),
              'conns': [], 'list': ['node']}
        st['vote'] = vote
        rows.append(run(m, 'send_vote', st, 'Double', 0x7532d0, m.admin, [vote], 4, label=f'vote{k}'))
    return rows

def bare_fraction(begin, n, pattern, precision):
    """0x74B280 + 0x7642D0 on a large received window, without hooks (speed)."""
    u = Uc(UC_ARCH_X86, UC_MODE_32)
    u.mem_map(PE0.base, (PE0.image_size + 4095) & ~4095)
    for vs, r, size, off in PE0.sections: u.mem_write(PE0.base + r, PE0.b[off:off + size])
    u.mem_map(0x2000000, 0x10000); u.mem_map(0x4000000, 0x10000)
    conn, net, data, stack, stop = 0x2000000, 0x2001000, 0x2002000, 0x4008000, 0x400f000
    u.mem_write(data, bytes(pattern)); u.mem_write(conn + 0x414, struct.pack('<I', net))
    u.mem_write(net + 0x1c, struct.pack('<iiII', begin, begin + n, len(pattern), data))
    u.mem_write(stack, struct.pack('<I', stop)); u.reg_write(UC_X86_REG_ESP, stack); u.reg_write(UC_X86_REG_ECX, conn)
    u.reg_write(UC_X86_REG_FPCW, PC[precision])
    u.emu_start(0x74b280, stop)
    assert u.reg_read(UC_X86_REG_EIP) == stop
    return struct.unpack('<I', bytes(u.mem_read(conn + 0x50, 4)))[0]

def fraction_cases():
    rows = []
    for begin, n, pattern in [(0, (1 << 24) + 3, [1, 1, 0]), (7, (1 << 24) + 1, [1, 0]), (-5, (1 << 25) + 7, [1, 1, 1, 0, 1]),
                              (100, 3 * (1 << 23) + 1, [1, 1, 0, 1, 0, 1, 1]), (0, 1000003, [1, 1, 0])]:
        count = sum(pattern[((begin + k + 0x80000000) & 0xffffffff) % len(pattern)] for k in range(n)) if n < 2000000 else None
        for precision in ['Single', 'Double', 'Extended']:
            rows.append({'kind': 'fraction', 'precision': precision, 'begin': begin, 'n': n, 'pattern': pattern,
                         'result': bare_fraction(begin, n, pattern, precision)})
    return rows

def refresh_cases(m, rng):
    rows = []
    for i in range(60):
        nconn = rng.randrange(0, 4)
        conns = [conn(rng, base=base(rng, player=10 + k)) for k in range(nconn)]
        lst = ['node'] + list(range(nconn))
        if nconn and rng.random() < 0.3: lst = list(range(nconn)) + ['node']
        host = rng.choice(['node'] + list(range(nconn))); server = rng.choice(['node', host])
        nd = node(rng, host=host, server=server, decrees=hist(rng.randrange(0, 40), [[0]] * rng.randrange(0, 3)),
                  backup_decrees=hist(rng.randrange(0, 90), [[0]] * rng.randrange(0, 3)))
        e = env(falling_behind=int(rng.random() < 0.4),
                config={'async_ticks_per_second': f2b(16.0), 'broadcast_decree_behind': rng.choice([32, 0, 5, -1]),
                        'backup_decree_behind': rng.choice([64, 0, 10]), 'admin_ticks_per_second': f2b(16.0)})
        st = {'env': e, 'node': nd, 'conns': conns, 'list': lst}
        for precision in ['Single', 'Double']:
            rows.append(run(m, 'refresh', st, precision, 0x74f1a0, m.admin, label=f'random{i}'))
    return rows

def main():
    m = HbMachine(); rng = random.Random(0x75b820)
    rows = []
    rows += refresh_cases(m, rng)
    rows += update_cases(m, rng)
    rows += precision_update_cases(m)
    rows += sethost_cases(m, rng)
    rows += setserver_cases(m, rng)
    rows += heartbeat_cases(m, rng)
    rows += sender_cases(m, rng)
    rows += fraction_cases()
    ranges = []
    for a, size in sorted(m.visited.items()):
        if not PE0.base <= a < PE0.base + PE0.image_size: continue
        if ranges and ranges[-1][1] >= a: ranges[-1][1] = max(ranges[-1][1], a + size)
        else: ranges.append([a, a + size])
    windows = []
    for a, b in ranges:
        raw = PE0.range(a, b - a); assert raw == PE1.range(a, b - a), hex(a)
        windows.append({'start': hex(a), 'end': hex(b), 'sha256': sha(raw)})
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
              'producer_sha256': sha(Path(__file__).read_bytes()), 'cases': rows, 'executed_windows': windows}
    (HERE / 'native-heartbeat.json').write_text(json.dumps(report, separators=(',', ':')) + '\n')
    kinds = {}
    for r in rows: kinds[r['kind']] = kinds.get(r['kind'], 0) + 1
    print(len(rows), 'cases', kinds)

if __name__ == '__main__':
    main()
