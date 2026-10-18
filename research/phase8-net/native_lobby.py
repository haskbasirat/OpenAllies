"""Original lobby join path of AA.exe under Unicorn (v14 target).

Host node: TGC_AdminConn host join completion 0x753580 (connected, 0x752300,
0x752490, session message 0x74C270 -> KKC_Session::SerializeState(O)
0x76D0B0 with its sub-codecs 0x76B800 / 0x59EB30 / 0x512CB0 family /
0x511BB0 / 0x50E200 / 0x50DC60 / 0x505DE0 / 0x784360 / 0x77B5A0 / 0x79C720 /
0x76A7C0 / 0x74EBC0 / 0x74B8D0 / 0x77DCF0, admin-state message 0x74C130,
finalizer 0x74D0A0, private history append) and the PlayerJoin decree
process 0x75DA80 -> 0x758FD0 (0x76CCA0 / 0x76C850, 0x773710 / 0x7661D0 /
0x770280, 0x766560, 0x74CA80 request emission, 0x45F240 / 0x74DC10).
Client node: private history dispatcher 0x7531A0 -> admin-state handler
0x74DF50 (0x765F70, 0x77D900, client clock 0x75FAC0 / 0x75FB40) and
KKC_Session::SerializeState(I) 0x77AAC0 (0x76EBD0 / 0x5A1E90 / 0x513130
family / 0x512070 / 0x510D90 / 0x50FF20 / 0x506560 / 0x7847B0 / 0x77B9D0 /
0x79CC00 / 0x76A810 + address interning 0x783700 / 0x74B430 / 0x74BC50 /
0x74AFD0 / 0x74DC10 / 0x7661D0), plus the PlayerJoin decree on clients.

Every executed native instruction is recorded; each case runs on the original
image and on the v14 image (.text/.rdata overlaid, native_packets.NodeMachine)
and the results must agree; executed windows record v14 identity.

Seams (recorded as events, never invented): allocator / CString manager /
logging (Phase3/4 machines); WinSock byte order ntohl/ntohs/htonl/htons (IAT
0x9F0374/0x9F0348/0x9F034C/0x9F0344); _getptd (C locale, native_connect);
NetNode connection lookup 0x764970 (fixture conn per address, event
'connect'); KKC_Session::ChangeToState 0x77A750 (event); SendUpdateMapStatus
0x770B70 / SendUpdateRMCMapStatus 0x7770B0 (event, fixture bool); session
info object 0x781DD0 / 0x781250 (events); scenario lookup 0x59F570 and
random-map lookup 0x41EEC0 (fixture name table; the native empty-key test of
0x41EEC0 runs); kingdom info 0x77E2B0 (fixture +5 byte); AutoAssignKingdom
0x778D10 (event); text table 0x4B9F40 (fixture format per key); CString copy
0x856400 (value copy); UI message 0x40BC50 (event); request streams
0x766CC0/0x766CD0/0x76AC60/0x766C10/0x766C90 (fixture writer; descriptor
0x766D00 and payload 0x765F10 run natively). Definition registries are
synthetic managers ([reg]+0x24 array, +0x28 count) with definitions carrying
+8 name, +0xC index, +0x20 kind and a virtual +0x38 (accept / reject stub).
Session, launch-settings and AdminConn objects of other peers are synthetic
memory with the native layouts the codecs touch; SessionPlayers are built by
the original constructor 0x7661D0, address objects by the original 0x783700,
the host connection by the original AdminConn ctor 0x756BC0, the client clock
by 0x75FAA0 and synchronizer A by 0x77CCB0.
"""
from pathlib import Path
import copy, json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_packets import NodeMachine, NODE, CONN, NETCONN, SESSION_, NETNODE
from native_channels import READER_VT, WRITER_VT, PE0, PE1, sha
from unicorn.x86_const import *

STUB = 0x7700000; PTD = 0x202a000; DATA = 0x2026000; FLAGS = 0x2027000
PC = {'Single': 0x007f, 'Double': 0x027f, 'Extended': 0x037f}
ACCEPT = STUB + 0x200; REJECT = STUB + 0x210; LOCK_RET = STUB + 0x220
LOCK_VT = STUB + 0x300; DEF_VT_ACCEPT = STUB + 0x400; DEF_VT_REJECT = STUB + 0x500
HTONS = STUB + 0x30; HTONL = STUB + 0x40
SYNC_A = 0x2027800
BIG_HEAP = 0x10000000; BIG_HEAP_SIZE = 0x10000000
REQ_STREAM = 0x2028800
REGISTRIES = {'MAP_DEF_44': 0xb3aa24, 'MAP_DEF_48': 0xb3aa34, 'CONDITIONS': 0xb3aa70, 'WORLD_RULES': 0xb3aa88,
              'NATIONS': 0xb3aa44, 'FACTIONS': 0xb3aa4c, 'COLORS': 0xb3aa60, 'AI_HANDICAPS': 0xb3aa68,
              'START_PROPERTIES': 0xb3aa2c, 'OBJECT_DEFINITIONS': 0xb39f58}
NODE_VT = 0xa913b0; CONN_VT = 0xa8d964; WC_VT = 0xa31990

def units(s):
    b = s.encode('utf-16-le', errors='surrogatepass')
    return list(struct.unpack('<%dH' % (len(b) // 2), b))
def f2b(v): return struct.unpack('<I', struct.pack('<f', v))[0]
def s32(v): return struct.unpack('<i', struct.pack('<I', v & 0xffffffff))[0]


class LobbyMachine(NodeMachine):
    def __init__(self, host, image):
        super().__init__(host, image)
        # As native_connect.ConnectMachine: WinSock byte order and _getptd stubs.
        self.u.mem_map(STUB, 0x1000)
        self.w(0x9f0374, STUB); self.w(0x9f0348, STUB + 0x10)
        self.w(0xb36530, DATA); self.w(0xb3654c, FLAGS)
        self.w(PTD + 0x64, self.r(0xb35444))
        code = {ACCEPT: b'\xb8\x01\x00\x00\x00\xc3', REJECT: b'\x31\xc0\xc3', LOCK_RET: b'\xc3'}
        for a, c in code.items(): self.u.mem_write(a, c)
        self.w(LOCK_VT + 4, LOCK_RET); self.w(LOCK_VT + 8, LOCK_RET)
        self.w(DEF_VT_ACCEPT + 0x38, ACCEPT); self.w(DEF_VT_REJECT + 0x38, REJECT)
        self.w(0x9f0344, HTONS); self.w(0x9f034c, HTONL)
        self.w(0xb3655c, SYNC_A)
        # Static empty CStrings created by the CRT initializers (0xB43B40 is set by NodeMachine).
        for g in (0xb43b48, 0xb43b50):
            if self.r(g) == 0: self.w(g, self.string(''))
        self.w(self.order_manager + 4, 357)
        self.w(0xb409a4 + 4, 307); self.w(0xb409a4 + 8, self.string('TGC_PostPlayerJoinOrderRequest'))
        # Large private heap: the heap is never rewound (see reset_node).
        self.u.mem_map(BIG_HEAP, BIG_HEAP_SIZE)
        self.heap = BIG_HEAP
        self.mark = self.heap
        self.lobby = []; self.req = 0
        self.build_seams()
    def alloc(self, n):
        p = self.heap; self.heap += (max(n, 1) + 15) & ~15
        if self.heap >= BIG_HEAP + BIG_HEAP_SIZE: raise AssertionError('harness heap exhausted')
        self.u.mem_write(p, bytes(max(n, 1))); return p
    # -- seams -------------------------------------------------------------
    def ecx(self): return self.u.reg_read(UC_X86_REG_ECX)
    def ev(self, _event, **kw):
        d = {'event': _event}; d.update(kw); self.lobby.append(d)
    def hook(self, u, pc, size, user):
        if self.mode == 'channels':
            if pc in (STUB, HTONL):
                self.visited[pc] = size
                v = self.arg(); self.ret(4, struct.unpack('<I', struct.pack('>I', v))[0]); return
            if pc in (STUB + 0x10, HTONS):
                self.visited[pc] = size
                v = self.arg() & 0xffff; self.ret(4, ((v & 0xff) << 8) | (v >> 8)); return
            if pc == 0x9738a9:
                self.visited[pc] = size
                self.ret(0, PTD); return
            if pc == 0x753288 and getattr(self, 'dispatch_watch', False):
                # 0x7531A0 after the kind handler: its reader object is at esp+0x14.
                rd = self.u.reg_read(UC_X86_REG_ESP) + 0x14
                self.ends.append({'byte': self.r(rd + 8), 'bit': self.r(rd + 0xc)})
            f = self.seams.get(pc) if hasattr(self, 'seams') else None
            if f is not None:
                self.visited[pc] = size
                if f() is not False: return
        super().hook(u, pc, size, user)
    def admin_log(self):
        if getattr(self, 'watch_logs', False) and self.text(self.arg(1)).endswith('AdminLayer.cpp'):
            self.decree_logs.append(self.arg(2))
        self.ret()
    def build_seams(self):
        self.seams = {
            0x404f60: self.admin_log,
            0x764970: self.net_conn,
            0x77a750: lambda: (self.ev('change_to_state', state=self.arg(0), arg=self.arg(1)), self.ret(8)),
            0x770b70: lambda: self.map_status(False),
            0x7770b0: lambda: self.map_status(True),
            0x781dd0: lambda: (self.ev('reset_session_info'), self.ret()),
            0x781250: lambda: (self.ev('apply_session_info', text=units(self.text(self.arg(0)))), self.ret(4)),
            0x59f570: lambda: self.lookup('scenario', self.text(self.r(self.arg(0))), 0x38, 0),
            0x41eec0: self.rmc_lookup,
            0x77e2b0: self.kingdom_info,
            0x778d10: lambda: (self.ev('auto_assign_kingdom', player=self.r(self.arg(0) + 0x20)), self.ret(4, 0)),
            0x4b9f40: self.localize,
            0x856400: self.copy_string,
            0x40bc50: self.ui_message,
            0x766cc0: lambda: self.ret(0, self.lock),
            0x766cd0: lambda: (self.ev('request_gate'), self.ret(0, 1)),
            0x76ac60: lambda: self.ret(0, 0),
            0x766c10: lambda: (self.ev('request_player', player=self.arg(0)), self.ret()),
            0x766c90: lambda: self.ret(0, self.req),
        }
    def net_conn(self):
        a = self.arg(0)
        addr = {'ip': struct.unpack('>I', bytes(self.u.mem_read(a + 8, 4)))[0], 'port': struct.unpack('>H', bytes(self.u.mem_read(a + 6, 2)))[0]}
        self.ev('connect', address=addr)
        nc = self.netconn_for(addr, a)  # no nested emulation inside a seam: reuse the interned object
        self.w(a, nc)
        self.ret(4, nc)
    def map_status(self, rmc):
        self.ev('map_status', rmc=rmc, file=units(self.text(self.r(self.arg(0)))), crc=self.arg(1), source_check=self.arg(2) & 0xff)
        self.ret(0xc, int(self.map_status_result.pop(0)) if self.map_status_result else 1)
    def lookup(self, kind, key, name_off, pop):
        self.ev(kind, key=units(key))
        name = self.names[kind].get(key)
        if name is None:
            self.ret(pop, 0); return
        o = self.alloc(0x60); self.w(o + name_off, self.string(name)); self.ret(pop, o)
    def rmc_lookup(self):
        p = self.arg(0)
        if self.u.mem_read(p, 2) == b'\0\0':
            return False  # the ORIGINAL 0x41EEC0 returns null for an empty key
        self.lookup('random_map', self.text(p), 8, 4)
    def kingdom_info(self):
        name = self.text(self.r(self.arg(0)))
        self.ev('kingdom_info', kingdom=units(name))
        flag = self.kingdom_open.get(name)
        if flag is None:
            self.ret(4, 0); return
        o = self.alloc(0x10); self.u.mem_write(o + 5, bytes([int(flag)])); self.ret(4, o)
    def localize(self):
        # 0x4B9F40(&key CString, &default 0xB43B48) on the text table [0xB36544]; returns the
        # localized CString. Keys without a fixture format come back unchanged (identity).
        key = self.text(self.r(self.arg(0)))
        if key not in self.formats:
            self.ret(8, self.arg(0)); return
        self.ev('localize', key=key)
        out = self.alloc(4); self.w(out, self.string(self.formats[key])); self.ret(8, out)
    def copy_string(self):
        out = self.arg(0); self.w(out, self.string(self.text(self.arg(1)))); self.ret(0, out)
    def ui_message(self):
        self.ev('ui_message', kind=self.arg(1), text=units(self.text(self.arg(2)))); self.ret()
    # -- memory helpers ------------------------------------------------------
    def b(self, p, v): self.u.mem_write(p, bytes([int(v) & 0xff]))
    def rb(self, p): return self.u.mem_read(p, 1)[0]
    def cs(self, p, text): self.w(p, self.string(text) if isinstance(text, str) else self.ustring(text))
    def ustring(self, us):
        b = struct.pack('<%dH' % len(us), *us); p = self.alloc(len(b) + 18)
        self.w(p, self.manager); self.w(p + 4, len(us)); self.w(p + 8, len(us)); self.w(p + 12, 1)
        self.u.mem_write(p + 16, b + b'\0\0'); return p + 16
    def rs(self, p):
        """CString at *p as code units (length from the header)."""
        d = self.r(p); n = self.r(d - 12)
        return list(struct.unpack('<%dH' % n, bytes(self.u.mem_read(d, 2 * n)))) if n else []
    def vec(self, base, elems, size, build):
        n = len(elems)
        data = self.alloc(max(n, 1) * size) if n else 0
        self.w(base, data); self.w(base + 4, n); self.w(base + 8, n)
        for i, e in enumerate(elems): build(data + i * size, e)
    def rvec(self, base, size, read):
        data, n = self.r(base), s32(self.r(base + 4))
        return [read(data + i * size) for i in range(max(n, 0))]
    def fpu(self, precision): self.u.reg_write(UC_X86_REG_FPCW, PC[precision])

    # -- game data -------------------------------------------------------------
    def setup_data(self, data):
        self.defs = {}
        for name, reg in REGISTRIES.items():
            cfg = data['registries'][name]
            mgr = self.alloc(0x30); arr = self.alloc(max(cfg['count'], 1) * 4)
            self.w(mgr + 0x24, arr); self.w(mgr + 0x28, cfg['count']); self.w(reg, mgr)
            for i in range(cfg['count']):
                if i in cfg.get('null', []):
                    self.w(arr + 4 * i, 0); continue
                d = self.alloc(0x40)
                self.w(d, DEF_VT_REJECT if i in cfg.get('reject', []) else DEF_VT_ACCEPT)
                self.cs(d + 8, cfg.get('names', {}).get(str(i), f'{name.lower()}_{i}'))
                self.w(d + 0xc, i); self.w(d + 0x20, cfg.get('kinds', {}).get(str(i), 0))
                self.w(arr + 4 * i, d); self.defs[d] = i
        self.names = {'scenario': data['scenarios'], 'random_map': data['random_maps']}
        self.kingdom_open = data.get('kingdom_open', {})
        self.formats = data.get('formats', {})
        self.map_status_result = list(data.get('map_status_results', []))
    def def_ptr(self, reg, index):
        if index is None: return 0
        arr = self.r(self.r(REGISTRIES[reg]) + 0x24); return self.r(arr + 4 * index)
    def def_index(self, p): return None if p == 0 else self.defs[p]

    # -- addresses / peers -----------------------------------------------------
    def address(self, a):
        if a is None: return 0
        return self.call(0x783700, [a['ip'], a['port']], this=self.r(0xb40b80), callee_pop=8)
    def raddress(self, p):
        if p == 0: return None
        return {'ip': struct.unpack('>I', bytes(self.u.mem_read(p + 8, 4)))[0], 'port': struct.unpack('>H', bytes(self.u.mem_read(p + 6, 2)))[0]}
    def netconn_for(self, addr, obj=None):
        key = (addr['ip'], addr['port'])
        if key not in self.netconns:
            nc = self.alloc(0xa4); c = self.alloc(0x424)
            self.w(c, CONN_VT); self.w(c + 0x414, nc); self.cs(c + 0x74, '')
            self.w(nc + 0x9c, obj if obj is not None else self.address(addr)); self.w(nc + 0xa0, c)
            self.netconns[key] = nc
        return self.netconns[key]
    def link_ptr(self, link):
        if link == 'Node': return NODE
        if link == 'Null': return 0
        return self.r(self.netconn_for(link['Conn']) + 0xa0)
    def link_of(self, p):
        if p == NODE: return 'Node'
        if p == 0: return 'Null'
        for key, nc in self.netconns.items():
            if self.r(nc + 0xa0) == p: return {'Conn': {'ip': key[0], 'port': key[1]}}
        raise AssertionError(hex(p))
    def set_identity(self, peer, ident):
        self.w(peer + 0x3c, ident['field_3c']); self.w(peer + 0x70, ident['join_order'])
        pid = ident['player_id']
        self.cs(peer + 0x74, pid['name']); self.w(peer + 0x78, pid['serial'])
        self.w(peer + 0x7c, pid['a']); self.w(peer + 0x80, pid['b']); self.w(peer + 0x84, pid['c'])
    def identity(self, peer):
        return {'field_3c': self.r(peer + 0x3c), 'join_order': self.r(peer + 0x70),
                'player_id': {'name': self.rs(peer + 0x74), 'serial': self.r(peer + 0x78), 'a': self.r(peer + 0x7c),
                              'b': self.r(peer + 0x80), 'c': self.r(peer + 0x84)}}

    # -- launch settings -------------------------------------------------------
    def pair(self, p, v): self.cs(p, v['first']); self.cs(p + 4, v['second'])
    def rpair(self, p): return {'first': self.rs(p), 'second': self.rs(p + 4)}
    def build_wc(self, p, wc):
        self.w(p, WC_VT); self.w(p + 4, wc['kind']); self.w(p + 8, self.def_ptr('CONDITIONS', wc['condition']))
        def rule(q, r):
            self.w(q, self.def_ptr('WORLD_RULES', r['rule'])); self.w(q + 4, int(r['enabled'])); self.w(q + 8, r['value'])
        self.vec(p + 0xc, wc['rules'], 0xc, rule); self.w(p + 0x18, wc['changes'])
    def read_wc(self, p):
        return {'kind': self.r(p + 4), 'condition': self.def_index(self.r(p + 8)),
                'rules': self.rvec(p + 0xc, 0xc, lambda q: {'rule': self.def_index(self.r(q)), 'enabled': self.rb(q + 4) != 0,
                                                          'value': self.r(q + 8)}),
                'changes': self.r(p + 0x18)}
    def build_kingdom(self, p, k):
        self.cs(p, k['name']); self.pair(p + 4, k['pair']); self.b(p + 0x10, k['flag_10']); self.cs(p + 0x14, k['text_14'])
        for off, reg, key in [(0x18, 'NATIONS', 'nation'), (0x1c, 'FACTIONS', 'faction'), (0x20, 'COLORS', 'color'), (0x24, 'AI_HANDICAPS', 'ai_handicap')]:
            self.w(p + off, self.def_ptr(reg, k[key]))
        self.b(p + 0x28, k['flag_28']); self.b(p + 0x29, k['flag_29']); self.cs(p + 0x2c, k['text_2c'])
        self.b(p + 0x30, k['flag_30']); self.b(p + 0x31, k['flag_31']); self.b(p + 0x32, k['flag_32'])
        self.vec(p + 0x34, k['words'], 4, lambda q, v: self.w(q, v))
        self.vec(p + 0x40, k['pairs'], 8, self.pair)
        self.vec(p + 0x4c, k['start_properties'], 4, lambda q, v: self.w(q, self.def_ptr('START_PROPERTIES', v)))
        self.cs(p + 0x58, k['text_58'])
        self.vec(p + 0x5c, k['additional_actors'], 4, lambda q, v: self.w(q, self.def_ptr('OBJECT_DEFINITIONS', v)))
    def read_kingdom(self, p):
        return {'name': self.rs(p), 'pair': self.rpair(p + 4), 'flag_10': self.rb(p + 0x10) != 0, 'text_14': self.rs(p + 0x14),
                'nation': self.def_index(self.r(p + 0x18)), 'faction': self.def_index(self.r(p + 0x1c)),
                'color': self.def_index(self.r(p + 0x20)), 'ai_handicap': self.def_index(self.r(p + 0x24)),
                'flag_28': self.rb(p + 0x28) != 0, 'flag_29': self.rb(p + 0x29) != 0, 'text_2c': self.rs(p + 0x2c),
                'flag_30': self.rb(p + 0x30) != 0, 'pairs': self.rvec(p + 0x40, 8, self.rpair),
                'start_properties': self.rvec(p + 0x4c, 4, lambda q: self.def_index(self.r(q))), 'text_58': self.rs(p + 0x58),
                'additional_actors': self.rvec(p + 0x5c, 4, lambda q: self.def_index(self.r(q))),
                'flag_31': self.rb(p + 0x31) != 0, 'flag_32': self.rb(p + 0x32) != 0, 'words': self.rvec(p + 0x34, 4, self.r)}
    def build_header(self, p, h):
        def named(q, e): self.cs(q, e['name']); self.pair(q + 4, e['pair'])
        self.vec(p + 4, h['names'], 0xc, named)
        self.vec(p + 0x10, h['kingdoms'], 0x6c, self.build_kingdom)
    def read_header(self, p):
        return {'names': self.rvec(p + 4, 0xc, lambda q: {'name': self.rs(q), 'pair': self.rpair(q + 4)}),
                'kingdoms': self.rvec(p + 0x10, 0x6c, self.read_kingdom)}
    def build_game(self, g):
        p = self.alloc(0x68); wc = self.alloc(0x1c); h = self.alloc(0x1c)
        self.w(p + 4, wc); self.w(p + 8, h); self.build_wc(wc, g['conditions']); self.build_header(h, g['header'])
        self.w(p + 0xc, g['version']); self.w(p + 0x10, g['field_10']); self.w(p + 0x18, g['time_low']); self.w(p + 0x1c, g['time_high'])
        self.pair(p + 0x20, g['pair_20']); self.pair(p + 0x28, g['pair_28']); self.w(p + 0x30, g['field_30']); self.w(p + 0x34, g['field_34'])
        self.cs(p + 0x38, g['text_38']); self.cs(p + 0x3c, g['text_3c']); self.w(p + 0x40, g['field_40'])
        def entry(q, e):
            self.cs(q, e['text_0']); self.b(q + 4, e['flag_4']); self.b(q + 5, e['flag_5']); self.cs(q + 8, e['text_8'])
        self.vec(p + 0x44, g['entries'], 0xc, entry)
        return p
    def read_game(self, p):
        return {'pair_20': self.rpair(p + 0x20), 'pair_28': self.rpair(p + 0x28), 'field_10': self.r(p + 0x10), 'version': self.r(p + 0xc),
                'header': self.read_header(self.r(p + 8)), 'conditions': self.read_wc(self.r(p + 4)),
                'field_30': self.r(p + 0x30), 'field_34': self.r(p + 0x34), 'text_3c': self.rs(p + 0x3c), 'field_40': self.r(p + 0x40),
                'text_38': self.rs(p + 0x38), 'time_low': self.r(p + 0x18), 'time_high': self.r(p + 0x1c),
                'entries': self.rvec(p + 0x44, 0xc, lambda q: {'flag_4': self.rb(q + 4) != 0, 'text_8': self.rs(q + 8),
                                                              'text_0': self.rs(q), 'flag_5': self.rb(q + 5) != 0})}
    def build_map(self, m, size):
        p = self.alloc(size)
        self.build_header(p + 4, m['header']); self.build_wc(p + 0x20, m['conditions'])
        self.w(p + 0x3c, m['size_x']); self.w(p + 0x40, m['size_y'])
        self.w(p + 0x44, self.def_ptr('MAP_DEF_44', m['def_44'])); self.w(p + 0x48, self.def_ptr('MAP_DEF_48', m['def_48']))
        self.w(p + 0x4c, m['field_4c'])
        return p
    def read_map(self, p):
        return {'header': self.read_header(p + 4), 'conditions': self.read_wc(p + 0x20), 'size_x': self.r(p + 0x3c),
                'size_y': self.r(p + 0x40), 'def_44': self.def_index(self.r(p + 0x44)), 'def_48': self.def_index(self.r(p + 0x48)),
                'field_4c': self.r(p + 0x4c)}
    def named_obj(self, name, off):
        if name is None: return 0
        o = self.alloc(0x60); self.cs(o + off, name); return o
    def build_launch(self, base, L):
        self.w(base, L['game_type']); self.w(base + 4, L['mode']); self.cs(base + 8, L['file']); self.w(base + 0xc, L['field_0c'])
        self.cs(base + 0x10, ''); self.cs(base + 0x30, '')
        self.w(base + 0x14, self.build_game(L['game']))
        md = self.build_map(L['map_default'], 0x54); self.b(md + 0x50, L['map_default_flag']); self.w(base + 0x18, md)
        ms = self.build_map(L['map_scenario'], 0x54); self.w(ms + 0x50, self.named_obj(L['scenario'], 0x38)); self.w(base + 0x1c, ms)
        mr = self.build_map(L['map_random'], 0x78); self.w(mr + 0x50, self.named_obj(L['random_map'], 8)); self.w(base + 0x20, mr)
        r = L['rmc_settings']; self.w(mr + 0x54, r['field_0'])
        def nw(q, e): self.cs(q, e['name']); self.w(q + 4, e['value'])
        self.vec(mr + 0x58, r['list_4'], 8, nw); self.vec(mr + 0x64, r['list_10'], 8, nw); self.w(mr + 0x70, r['field_1c'])
        self.w(base + 0x24, self.build_map(L['map_24'], 0x54))
        self.w(base + 0x28, self.build_map(L['map_metagame'], 0x5c))
        st = self.alloc(0x28); s = L['state']; self.w(base + 0x2c, st)
        self.w(st, s['kind']); self.b(st + 4, s['flag_04']); self.cs(st + 8, s['text']); self.w(st + 0xc, s['field_0c'])
        self.w(st + 0x10, s['field_10']); self.b(st + 0x14, s['flag_14']); self.b(st + 0x15, s['flag_15']); self.b(st + 0x16, s['flag_16'])
        self.w(st + 0x18, s['field_18']); self.w(st + 0x1c, s['field_1c']); self.b(st + 0x20, s['flag_20']); self.w(st + 0x24, s['field_24'])
    def read_launch(self, base):
        mr = self.r(base + 0x20); st = self.r(base + 0x2c)
        nw = lambda q: {'name': self.rs(q), 'value': self.r(q + 4)}
        scen = self.r(self.r(base + 0x1c) + 0x50); rmc = self.r(mr + 0x50)
        return {'game_type': self.r(base), 'mode': self.r(base + 4), 'file': self.rs(base + 8), 'field_0c': self.r(base + 0xc),
                'game': self.read_game(self.r(base + 0x14)),
                'map_default': self.read_map(self.r(base + 0x18)), 'map_default_flag': self.rb(self.r(base + 0x18) + 0x50) != 0,
                'map_scenario': self.read_map(self.r(base + 0x1c)), 'scenario': None if scen == 0 else self.rs(scen + 0x38),
                'map_random': self.read_map(mr), 'random_map': None if rmc == 0 else self.rs(rmc + 8),
                'rmc_settings': {'field_0': self.r(mr + 0x54), 'list_4': self.rvec(mr + 0x58, 8, nw), 'list_10': self.rvec(mr + 0x64, 8, nw),
                                 'field_1c': self.r(mr + 0x70)},
                'map_24': self.read_map(self.r(base + 0x24)), 'map_metagame': self.read_map(self.r(base + 0x28)),
                'state': {'kind': self.r(st), 'flag_04': self.rb(st + 4) != 0, 'text': self.rs(st + 8), 'field_0c': self.r(st + 0xc),
                          'field_10': self.r(st + 0x10), 'flag_14': self.rb(st + 0x14) != 0, 'flag_15': self.rb(st + 0x15) != 0,
                          'flag_16': self.rb(st + 0x16) != 0, 'field_18': self.r(st + 0x18), 'field_1c': self.r(st + 0x1c),
                          'flag_20': self.rb(st + 0x20) != 0, 'field_24': self.r(st + 0x24)}}

    # -- session ---------------------------------------------------------------
    def build_player(self, pl):
        p = self.alloc(0x54)
        self.call(0x7661d0, [self.link_ptr(pl['link']), pl['id'], int(pl['ai']), int(pl['implicit']), int(pl['film'])], this=p, callee_pop=0x14)
        self.cs(p + 8, pl['name']); self.w(p + 0x10, pl['field_10']); self.cs(p + 0x14, pl['text_14']); self.w(p + 0x18, pl['status'])
        self.b(p + 0x1c, pl['flag_1c']); self.cs(p + 0x24, pl['kingdom']); self.w(p + 0x28, pl['field_28']); self.b(p + 0x2c, pl['flag_2c'])
        self.b(p + 0x2d, pl['flag_2d']); self.cs(p + 0x30, pl['text_30']); self.cs(p + 0x34, pl['text_34']); self.b(p + 0x38, pl['flag_38'])
        self.w(p + 0x3c, pl['field_3c']); self.w(p + 0x40, pl['field_40'])
        return p
    def read_player(self, p):
        return {'link': self.link_of(self.r(p + 4)), 'name': self.rs(p + 8), 'ai': self.rb(p + 0xc) != 0, 'implicit': self.rb(p + 0xd) != 0,
                'film': self.rb(p + 0xe) != 0, 'field_10': self.r(p + 0x10), 'text_14': self.rs(p + 0x14), 'status': self.r(p + 0x18),
                'flag_1c': self.rb(p + 0x1c) != 0, 'id': self.r(p + 0x20), 'kingdom': self.rs(p + 0x24), 'field_28': self.r(p + 0x28),
                'flag_2c': self.rb(p + 0x2c) != 0, 'flag_2d': self.rb(p + 0x2d) != 0, 'text_30': self.rs(p + 0x30),
                'text_34': self.rs(p + 0x34), 'flag_38': self.rb(p + 0x38) != 0, 'field_3c': self.r(p + 0x3c), 'field_40': self.r(p + 0x40)}
    def build_session(self, s):
        S = self.alloc(0x200); self.w(0xb36550, S); self.session_ptr = S
        info = self.alloc(0x58); self.cs(info + 0x28, s['info_text']); self.w(S + 0x14, info)
        self.build_launch(S + 0x54, s['launch']); self.build_launch(S + 0x88, s['launch_88'])
        prev = 0; ptrs = []
        for pl in s['players']: ptrs.append(self.build_player(pl))
        nodes = [self.alloc(8) for _ in ptrs]
        for i, (n, p) in enumerate(zip(nodes, ptrs)):
            self.w(n, p); self.w(n + 4, nodes[i + 1] if i + 1 < len(nodes) else 0)
        self.w(S + 0xbc, nodes[0] if nodes else 0); self.w(S + 0xc0, nodes[-1] if nodes else 0)
        created = 0
        if s['created'] is not None:
            created = next(p for p in ptrs if self.r(p + 0x20) == s['created'])
        self.w(S + 0xc4, created)
        self.b(S + 0xc8, s['changed']); self.b(S + 0xc9, s['changed_c9']); self.b(S + 0xca, s['changed_ca'])
        o = s['options']; self.cs(S + 0xcc, o['name']); self.cs(S + 0xd0, o['password']); self.w(S + 0xd4, o['max_players'])
        self.w(S + 0xd8, o['field_d8']); self.b(S + 0xdc, o['allow_observers'])
        self.w(S + 0xe0, s['counter_e0']); self.w(S + 0xe4, s['state']); self.w(S + 0xe8, s['last_player_id']); self.b(S + 0xf4, s['multiplayer'])
        self.w(S + 0xfc, s['map_crc']); self.w(S + 0x104, s['load_crc']); self.cs(S + 0x108, s['load_file']); self.b(S + 0x10c, s['load_rmc'])
        self.w(S + 0x128, s['field_128']); self.w(S + 0x154, s['field_154']); self.w(S + 0x158, s['field_158']); self.w(S + 0x15c, s['field_15c'])
        self.cs(S + 0x160, s['text_160']); self.w(S + 0x164, s['field_164'])
        t = s['properties']
        if t is None: self.w(S + 0x170, 0)
        else:
            tp = self.alloc(0x14); self.w(S + 0x170, tp); self.b(tp + 0x10, t['flag']); n = len(t['entries'])
            arr = self.alloc(max(n, 1) * 4); self.w(tp + 4, arr); self.w(tp + 8, t['count']); self.w(tp + 0xc, max(n, 1))
            for i, e in enumerate(t['entries']):
                q = self.alloc(8); self.cs(q, e['name']); self.w(q + 4, e['value']); self.w(arr + 4 * i, q)
        return S
    def read_session(self, S):
        players = []; n = self.r(S + 0xbc)
        while n:
            players.append(self.read_player(self.r(n))); n = self.r(n + 4)
        c = self.r(S + 0xc4); tp = self.r(S + 0x170)
        table = None
        if tp:
            cnt = s32(self.r(tp + 8)); arr = self.r(tp + 4)
            table = {'flag': self.rb(tp + 0x10) != 0, 'count': cnt,
                     'entries': [{'name': self.rs(self.r(arr + 4 * i)), 'value': self.r(self.r(arr + 4 * i) + 4)} for i in range(max(cnt, 0))]}
        return {'info_text': self.rs(self.r(S + 0x14) + 0x28), 'launch': self.read_launch(S + 0x54), 'launch_88': self.read_launch(S + 0x88),
                'players': players, 'created': None if c == 0 else self.r(c + 0x20),
                'changed': self.rb(S + 0xc8) != 0, 'changed_c9': self.rb(S + 0xc9) != 0, 'changed_ca': self.rb(S + 0xca) != 0,
                'options': {'name': self.rs(S + 0xcc), 'password': self.rs(S + 0xd0), 'max_players': s32(self.r(S + 0xd4)),
                            'field_d8': self.r(S + 0xd8), 'allow_observers': self.rb(S + 0xdc) != 0},
                'counter_e0': self.r(S + 0xe0), 'state': self.r(S + 0xe4), 'last_player_id': self.r(S + 0xe8),
                'multiplayer': self.rb(S + 0xf4) != 0, 'map_crc': self.r(S + 0xfc), 'load_crc': self.r(S + 0x104),
                'load_file': self.rs(S + 0x108), 'load_rmc': self.rb(S + 0x10c) != 0, 'field_128': self.r(S + 0x128),
                'field_154': self.r(S + 0x154), 'field_158': self.r(S + 0x158), 'field_15c': self.r(S + 0x15c),
                'text_160': self.rs(S + 0x160), 'field_164': self.r(S + 0x164), 'properties': table}

    # -- node ------------------------------------------------------------------
    def reset_node(self, node):
        """Per-case node state: AdminNode identity / address / host link, peer list, conns."""
        # The heap is never rewound: native channel statics (0xB40AA4, 0xB40AB4, ...) keep heap
        # pointers across calls (native_channels.py). The host connection is rebuilt per case.
        self.lobby = []; self.netconns = {}
        self.u.mem_write(CONN, bytes(0x424)); self.call(0x756bc0, [NETCONN], this=CONN, callee_pop=4)
        self.b(CONN + 0x88, 1)
        # TGC_WinSock ctor 0x783FE0 ([0xB40B80]); its maps intern the address objects (0x783700).
        self.u.mem_write(0xb40b80, bytes(0x20))
        ws = self.alloc(0x40); self.call(0x783fe0, [], this=ws)
        self.netconns[(node['conn_address']['ip'], node['conn_address']['port'])] = NETCONN
        self.w(NETCONN + 0x9c, self.address(node['conn_address']))
        self.set_identity(NODE, node['identity']); self.set_identity(CONN, node['conn_identity'])
        self.w(NODE, NODE_VT)
        self.w(NODE + 0x29c, self.address(node['own_address']))
        self.w(NODE + 0x2ac, 0); self.w(NODE + 0x2b0, 0); self.b(NODE + 0x2b4, 0)
        self.w(NODE + 0x2c4, NODE if node['host'] == 'Node' else self.link_ptr(node['host']))
        self.w(CONN + 8, 0); self.w(CONN + 0x2c, 0)
        self.lock = self.alloc(4); self.w(self.lock, LOCK_VT)
    def peer_list(self):
        out = []; n = self.r(NODE + 0x2ac)
        while n:
            out.append(self.link_of(self.r(n))); n = self.r(n + 4)
        return out
    def peers_state(self):
        """Identity, +8/+0x2C players of the node and of every known connection."""
        out = []
        for link in ['Node'] + [{'Conn': {'ip': k[0], 'port': k[1]}} for k in sorted(self.netconns)]:
            p = self.link_ptr(link)
            pl = lambda q: None if self.r(q) == 0 else self.r(self.r(q) + 0x20)
            out.append({'link': link, 'identity': self.identity(p), 'joined_player': pl(p + 8), 'ordering_player': pl(p + 0x2c)})
        return out


# --------------------------------------------------------------------------------
# Fixture builders (Python dicts in the serde shape of crates/aa-netcode/src/lobby.rs)
# --------------------------------------------------------------------------------

def U(s): return units(s)
def pair(a='', b=''): return {'first': U(a), 'second': U(b)}
def wc(kind=0, condition=None, rules=(), changes=0):
    return {'kind': kind, 'condition': condition, 'rules': [dict(r) for r in rules], 'changes': changes}
def kingdom(name, **kw):
    k = {'name': U(name), 'pair': pair('kp1', 'kp2'), 'flag_10': False, 'text_14': U(''), 'nation': None, 'faction': None, 'color': None,
         'ai_handicap': None, 'flag_28': True, 'flag_29': False, 'text_2c': U(''), 'flag_30': False, 'pairs': [], 'start_properties': [],
         'text_58': U(''), 'additional_actors': [], 'flag_31': False, 'flag_32': False, 'words': []}
    k.update(kw); return k
def header(names=(), kingdoms=()): return {'names': list(names), 'kingdoms': list(kingdoms)}
def game(version=1, **kw):
    g = {'pair_20': pair(), 'pair_28': pair(), 'field_10': 0, 'version': version, 'header': header(), 'conditions': wc(),
         'field_30': 0, 'field_34': 0, 'text_3c': U(''), 'field_40': 0, 'text_38': U(''), 'time_low': 0, 'time_high': 0, 'entries': []}
    g.update(kw); return g
def mapset(**kw):
    m = {'header': header(), 'conditions': wc(), 'size_x': 0, 'size_y': 0, 'def_44': None, 'def_48': None, 'field_4c': 0}
    m.update(kw); return m
def state(**kw):
    s = {'kind': 0, 'flag_04': False, 'text': U(''), 'field_0c': 0xffffffff, 'field_10': 1, 'flag_14': False, 'flag_15': False,
         'flag_16': False, 'field_18': 4, 'field_1c': 1, 'flag_20': False, 'field_24': 0}
    s.update(kw); return s
def launch(**kw):
    L = {'game_type': 1, 'mode': 1, 'file': U(''), 'field_0c': 0, 'game': game(), 'map_default': mapset(), 'map_default_flag': False,
         'map_scenario': mapset(), 'scenario': None, 'map_random': mapset(), 'random_map': None,
         'rmc_settings': {'field_0': 0, 'list_4': [], 'list_10': [], 'field_1c': 0}, 'map_24': mapset(), 'map_metagame': mapset(),
         'state': state()}
    L.update(kw); return L
def pid(name='', serial=0, a=0, b=0, c=0): return {'name': U(name), 'serial': serial, 'a': a, 'b': b, 'c': c}
def ident(field_3c=0, join_order=0, **p): return {'field_3c': field_3c, 'join_order': join_order, 'player_id': pid(**p)}
def player(link, id, name, ai=False, implicit=False, film=False, **kw):
    p = {'link': link, 'name': U(name), 'ai': ai, 'implicit': implicit, 'film': film, 'field_10': 0xffffffff, 'text_14': U(''),
         'status': 2 if ai else 0, 'flag_1c': False, 'id': id, 'kingdom': U(''), 'field_28': 0, 'flag_2c': True, 'flag_2d': ai,
         'text_30': U(''), 'text_34': U(''), 'flag_38': True, 'field_3c': 0xffffffff, 'field_40': 0xffffffff}
    p.update(kw); return p
def session(**kw):
    s = {'info_text': U(''), 'launch': launch(), 'launch_88': launch(), 'players': [], 'created': None, 'changed': False,
         'changed_c9': False, 'changed_ca': False,
         'options': {'name': U('Game'), 'password': U(''), 'max_players': 16, 'field_d8': 0, 'allow_observers': False},
         'counter_e0': 0, 'state': 1, 'last_player_id': 0, 'multiplayer': True, 'map_crc': 0, 'load_crc': 0, 'load_file': U(''),
         'load_rmc': False, 'field_128': 0, 'field_154': 0, 'field_158': 0, 'field_15c': 0, 'text_160': U(''), 'field_164': 0,
         'properties': {'flag': False, 'count': 0, 'entries': []}}
    s.update(kw); return s
def addr(ip, port): return {'ip': ip, 'port': port}
def conn(a): return {'Conn': a}

def base_data(**kw):
    regs = {name: {'count': 4} for name in REGISTRIES}
    regs['WORLD_RULES'] = {'count': 5, 'kinds': {'0': 0, '1': 1, '2': 2, '3': 3, '4': 7}, 'names': {'0': 'RATED', '1': 'Score', '2': 'Speed', '3': 'Limit', '4': 'Odd'}}
    d = {'registries': regs, 'scenarios': {}, 'random_maps': {}, 'kingdom_open': {}, 'zero_width_list_budget': 64,
         'formats': {'networking_player_joined|s': '%s', 'networking_duplicate_name_formate|sd': '%s (%d)'}, 'map_status_results': []}
    d.update(kw); return d


# --------------------------------------------------------------------------------
# Scenario runners
# --------------------------------------------------------------------------------

def setup_common(m, data, node):
    m.mode = 'channels'
    m.setup_data_after_reset = None
    m.reset_node(node)
    m.setup_data(data)
    for extra in node.get('peers', []):
        m.set_identity(m.link_ptr(conn(extra['address'])), extra['identity'])
    m.w(0xb408f8, 0)  # static build stream of 0x753580 re-created per case
    # Synchronizer A (ctor 0x77CCB0): 16 retained entries instead of 0x10000 (retention is not
    # observable here; the 0x10000-entry ring costs millions of emulated instructions per case).
    m.u.mem_write(SYNC_A, bytes(0x40)); m.call(0x77ccb0, [16], this=SYNC_A, callee_pop=4)
    m.u.mem_write(NODE + 0x16c, bytes(0x8c)); m.call(0x75faa0, [], this=NODE + 0x16c)

def sync_state(m):
    return {'enabled': m.rb(SYNC_A) != 0, 'file_dump': m.rb(SYNC_A + 1) != 0, 'hash': s32(m.r(SYNC_A + 8)),
            'begin': s32(m.r(SYNC_A + 0x10)), 'end': s32(m.r(SYNC_A + 0x14))}

def history(m, base):
    begin, end, cap, arr = s32(m.r(base)), s32(m.r(base + 4)), m.r(base + 8), m.r(base + 12)
    out = []
    for i in range(begin, end):
        p = m.r(arr + ((i + 0x80000000) & 0xffffffff) % cap * 4)
        out.append(None if p == 0 else list(m.u.mem_read(p, m.r(p - 4))))
    return {'begin': begin, 'end': end, 'packets': out}

def set_history(m, base, begin, packets):
    cap = max(len(packets), 4); arr = m.alloc(cap * 4)
    m.w(base, begin & 0xffffffff); m.w(base + 4, (begin + len(packets)) & 0xffffffff); m.w(base + 8, cap); m.w(base + 12, arr)
    for i, data in enumerate(packets):
        m.w(arr + ((begin + i + 0x80000000) & 0xffffffff) % cap * 4, 0 if data is None else m.packet(list(data), refs=1))

def client_admin(m):
    c = NODE + 0x16c
    pl = m.r(NODE + 0x250)
    return {'own_address': m.raddress(m.r(NODE + 0x29c)), 'resume_byte': m.r(NODE + 0x248), 'resume_bit': m.r(NODE + 0x24c),
            'resume_player': None if pl == 0 else m.r(pl + 0x20), 'admin_time': m.r(NODE + 0x28c),
            'decree_next': s32(m.r(NODE + 0x20c)), 'decree_end': s32(m.r(NODE + 0x218)),
            'clock_selected': m.r(NODE + 0x158) == c,
            'clock': {'period': m.r(c + 4), 'offset': m.r(c + 8), 'samples': [m.r(c + 0xc + 4 * i) for i in range(32)]}}

def set_client_admin(m, a):
    m.w(NODE + 0x248, a['resume_byte']); m.w(NODE + 0x24c, a['resume_bit']); m.w(NODE + 0x250, 0)
    m.w(NODE + 0x28c, a['admin_time']); m.w(NODE + 0x20c, a['decree_next'] & 0xffffffff); m.w(NODE + 0x218, a['decree_end'] & 0xffffffff)
    m.w(NODE + 0x158, 0)
    c = NODE + 0x16c; m.w(c + 4, a['clock']['period']); m.w(c + 8, a['clock']['offset'])
    for i, v in enumerate(a['clock']['samples']): m.w(c + 0xc + 4 * i, v)

def host_join_messages(m, case):
    """0x753580 on the host connection: returns the conn's private history and flags."""
    h = case['host']
    setup_common(m, case['data'], h['node'])
    S = m.build_session(h['session'])
    # As after 0x758FD0: the connection's +8 / +0x2C point at the joined player (+0xC4).
    m.w(CONN + 8, m.r(S + 0xc4)); m.w(CONN + 0x2c, m.r(S + 0xc4))
    a = h['admin']
    m.w(NODE + 0x200, a['field_200']); m.w(NODE + 0x204, a['field_204']); m.w(NODE + 0x28c, a['admin_time'])
    m.w(NODE + 0x218, a['decree_end'] & 0xffffffff)
    pl = 0
    if a['player'] is not None:
        pl = m.alloc(0x54); m.w(pl + 0x20, a['player'])
    m.w(NODE + 0x208, pl)
    m.call(0x77d900, [h['sync']['index'] & 0xffffffff, h['sync']['hash'] & 0xffffffff], this=SYNC_A, callee_pop=8)
    m.b(FLAGS + 0xbc, h['internet'])
    m.u.mem_write(CONN + 0x88, b'\0')
    m.w(NODE + 0x224, h['backup_decrees']['begin'] & 0xffffffff); m.w(NODE + 0x228, h['backup_decrees']['end'] & 0xffffffff)
    backup_before = s32(m.r(CONN + 0xa0))
    status = 'ok'
    try:
        m.call(0x753580, [], this=CONN)
    except Exception as e:  # internet branch: GameSpy objects are not modelled
        status = 'stopped: ' + type(e).__name__ + ' ' + str(e)[:200] + ' at ' + hex(m.u.reg_read(UC_X86_REG_EIP))
    return {'status': status, 'private': history(m, CONN + 0xac), 'connected': m.rb(CONN + 0x88) != 0,
            'backup_index_before': backup_before, 'backup_index': s32(m.r(CONN + 0xa0))}

class EndRecorder:
    pass

def client_dispatch(m, case, packets):
    c = case['client']
    setup_common(m, case['data'], c['node'])
    S = m.build_session(c['session'])
    set_client_admin(m, c['admin'])
    m.w(NODE + 0x29c, m.address(c['node']['own_address']))
    m.call(0x77d900, [c['sync']['index'] & 0xffffffff, c['sync']['hash'] & 0xffffffff], this=SYNC_A, callee_pop=8)
    m.w(NETNODE + 0x58, c['now'])
    set_history(m, CONN + 0xac, c['private_begin'], packets)
    m.ends = []
    m.fpu(c['precision'])
    m.dispatch_watch = True
    status = 'ok'
    try:
        m.call(0x7531a0, [], this=CONN)
    except Exception as e:
        status = 'error: ' + type(e).__name__ + ' ' + str(e)[:120]
    m.dispatch_watch = False
    return {'status': status, 'ends': m.ends, 'session': m.read_session(S), 'admin': client_admin(m), 'sync': sync_state(m),
            'events': m.lobby, 'peers': m.peers_state(), 'peer_list': m.peer_list(), 'peers_changed': m.rb(NODE + 0x2b4) != 0,
            'private': {'begin': s32(m.r(CONN + 0xac)), 'end': s32(m.r(CONN + 0xb0))}}


# --------------------------------------------------------------------------------
# Cases
# --------------------------------------------------------------------------------

A = addr(0xc0a80105, 6112)       # joining client as the host sees it
H = addr(0xc0a80101, 7290)       # host as the client sees it
C3 = addr(0x0a000003, 50000)     # third players
C4 = addr(0x0a000004, 50001)
C5 = addr(0xac100007, 2302)

def host_node(**kw):
    n = {'conn_address': A, 'own_address': None, 'identity': ident(1, 1, name='HOSTKEY', serial=0x11, a=1, b=2, c=3),
         'conn_identity': ident(2, 2, name='CLIENTKEY', serial=0x22, a=4, b=5, c=6), 'host': 'Node', 'peers': []}
    n.update(kw); return n
def client_node(**kw):
    n = {'conn_address': H, 'own_address': None, 'identity': ident(), 'conn_identity': ident(), 'host': conn(H), 'peers': []}
    n.update(kw); return n
def client_admin_state(**kw):
    a = {'resume_byte': 0, 'resume_bit': 0, 'admin_time': 0, 'decree_next': 0, 'decree_end': 0,
         'clock': {'period': 0, 'offset': 0, 'samples': [0] * 32}}
    a.update(kw); return a
def host_admin(**kw):
    a = {'conn_address': A, 'field_200': 0, 'field_204': 3, 'player': 2, 'admin_time': f2b(10.0), 'decree_end': 10}
    a.update(kw); return a

def rule(r, enabled, value): return {'rule': r, 'enabled': enabled, 'value': value}

def rich_launch(rng, mode=1, field_0c=0, version=2, scenario=None, random_map=None, file='maps\\europe.map'):
    k1 = kingdom('Allies', flag_10=False, text_14=U('t14'), nation=1, faction=2, color=3, ai_handicap=0, flag_29=True,
                 text_2c=U('Ä/2c'), flag_30=True, pairs=[pair('a', 'b'), pair('', 'ccc')], start_properties=[0, None, 3],
                 text_58=U('t58'), additional_actors=[1, 2, None], flag_31=True, flag_32=False, words=[1, 0xffffffff, 7])
    k2 = kingdom('Axis', flag_28=False, nation=None, faction=None, color=None, ai_handicap=3)
    k3 = kingdom('Neutral', flag_10=True)
    cond = wc(5, 2, [rule(1, True, 0x12345678), rule(2, False, f2b(-1.5)), rule(0, True, 1), rule(3, True, 0xfffffffe), rule(4, False, 0)])
    hdr = header([{'name': U('n1'), 'pair': pair('p', 'q')}, {'name': U(''), 'pair': pair()}], [k1, k2, k3])
    g = game(version=version, pair_20=pair('Title', 'Desc'), pair_28=pair('Author', ''), field_10=0x10, header=hdr,
             conditions=wc(1, None, [rule(0, False, 0)]), field_30=0x30, field_34=0x34, text_3c=U('3c'), field_40=0x40,
             text_38=U('38'), time_low=0x5e6f7a8b, time_high=0x01c4a2b3,
             entries=[{'flag_4': True, 'text_8': U('e8'), 'text_0': U('e0'), 'flag_5': False},
                      {'flag_4': False, 'text_8': U(''), 'text_0': U('x' * 40), 'flag_5': True}])
    m = mapset(header=hdr, conditions=cond, size_x=512, size_y=768, def_44=1, def_48=None, field_4c=0xdeadbeef)
    return launch(game_type=rng.choice([1, 2]), mode=mode, file=U(file), field_0c=field_0c, game=g, map_default=m,
                  map_default_flag=True, map_scenario=mapset(conditions=wc(2, 0, [rule(0, True, 1)]), size_x=320, size_y=320),
                  scenario=scenario, map_random=mapset(header=header([], [k1]), size_x=1024, size_y=1024, def_48=2),
                  random_map=random_map,
                  rmc_settings={'field_0': f2b(0.5), 'list_4': [{'name': U('Water'), 'value': 3}, {'name': U('Hills'), 'value': 0xffffffff}],
                                'list_10': [{'name': U('Res'), 'value': 9}], 'field_1c': 2},
                  map_24=mapset(field_4c=24), map_metagame=mapset(conditions=wc(6, 3, []), def_44=0),
                  state=state(kind=9, flag_04=True, text=U('state'), field_10=7, flag_14=True, flag_15=True, flag_16=True,
                              field_18=15, field_1c=0x1c, flag_20=True, field_24=6))

NAMES = ['Host', 'Jörg', '日本語', 'Ölaf\U0001F600', '', 'N' * 300, 'tab\there', 'nul\0inside', 'A' * 17, 'Zed',
         'lone\ud800surrogate']

def roster(n, rng):
    """Host session players: the host, AI players (node), connected humans."""
    out = [player('Node', 1, 'Host', kingdom=U('Allies'), status=2)]
    addrs = [A, C3, C4, C5]
    humans = 0
    for i in range(2, n + 1):
        if rng.random() < 0.3:
            p = player('Node', i, 'AI ' + str(i), ai=True, implicit=rng.random() < 0.5, kingdom=U('Axis'))
        else:
            a = addrs[humans] if humans < len(addrs) else addr(0x0a010000 + i, 40000 + i)
            humans += 1
            p = player(conn(a), i, NAMES[i % len(NAMES)], status=rng.choice([0, 1, 2]))
        p.update(field_10=rng.choice([0xffffffff, 0, 5]), text_14=U(rng.choice(['', 'x14'])), flag_1c=rng.random() < 0.5,
                 text_30=U(rng.choice(['', '30'])), text_34=U(rng.choice(['', 'Unit34'])), flag_38=rng.random() < 0.5,
                 field_3c=rng.choice([0xffffffff, 3]), field_40=rng.choice([0xffffffff, 4]), flag_2d=rng.random() < 0.5)
        out.append(p)
    return out

def extra_peers(players):
    seen = []
    for p in players:
        if isinstance(p['link'], dict) and p['link']['Conn'] != A and p['link']['Conn'] not in [s['address'] for s in seen]:
            k = len(seen) + 3
            seen.append({'address': p['link']['Conn'], 'identity': ident(k % 4, 10 + k, name=f'KEY{k}', serial=k, a=k, b=k + 1, c=k + 2)})
    return seen

def join_message_cases():
    rng = random.Random(1214)
    cases = []
    def add(name, host=None, client=None, data=None):
        h = {'node': host_node(), 'session': session(players=[player('Node', 1, 'Host'), player(conn(A), 2, 'Client')], created=2,
                                                      last_player_id=2),
             'admin': host_admin(), 'sync': {'index': 17, 'hash': 1234}, 'internet': False, 'backup_decrees': {'begin': 0, 'end': 0}}
        c = {'node': client_node(), 'session': session(), 'admin': client_admin_state(decree_end=12), 'sync': {'index': -1, 'hash': -1},
             'now': f2b(100.25), 'precision': 'Double', 'private_begin': 0}
        for k, v in (host or {}).items(): h[k] = v
        for k, v in (client or {}).items(): c[k] = v
        h['node']['peers'] = extra_peers(h['session']['players'])
        cases.append({'name': name, 'data': data or base_data(), 'host': h, 'client': c})
    add('two_players')
    for n in (1, 3, 8, 12, 16):
        pl = roster(n, rng)
        created = rng.choice([p['id'] for p in pl if not p['ai']])
        add(f'roster_{n}', host={'session': session(players=pl, created=created, last_player_id=n, state=rng.choice([0, 1, 2, 3, 4, 5]))})
    for st in range(6):
        add(f'state_{st}', host={'session': session(players=[player('Node', 1, 'Host'), player(conn(A), 2, 'Client')], created=2, state=st,
                                                     last_player_id=0xfffffffe)})
    rich = lambda **kw: session(**dict(dict(players=[player('Node', 1, 'Host', kingdom=U('Allies')), player(conn(A), 5, 'Client', kingdom=U('Axis')),
                                                     player('Node', 6, 'AI', ai=True)], created=5, last_player_id=6), **kw))
    data = base_data(scenarios={'Europe': 'Europe (1944)'}, random_maps={'Continents': 'Continents', 'Isles': 'ISLES'})
    add('launch_rich_default_mode', host={'session': rich(launch=rich_launch(rng), launch_88=rich_launch(rng, version=1))}, data=data)
    add('launch_scenario_mode_found', host={'session': rich(launch=rich_launch(rng, mode=2, scenario=U('Europe')), map_crc=0xabcdef01)}, data=data)
    add('launch_scenario_mode_missing', host={'session': rich(launch=rich_launch(rng, mode=2, scenario=U('Africa')))}, data=data)
    add('launch_random_mode', host={'session': rich(launch=rich_launch(rng, mode=0, random_map=U('Isles')), map_crc=77)}, data=data)
    add('launch_random_empty_name', host={'session': rich(launch=rich_launch(rng, mode=0, random_map=None))}, data=data)
    add('launch_random_nul_name', host={'session': rich(launch=rich_launch(rng, mode=0, random_map=U('\0Isles')))}, data=data)
    add('load_check_rmc', host={'session': rich(launch=rich_launch(rng, mode=0, field_0c=5, random_map=U('Continents')),
                                                load_crc=5, load_file=U('save1'), load_rmc=True)}, data=data)
    add('load_check_map_refused', host={'session': rich(launch=rich_launch(rng, mode=2, field_0c=5), load_crc=6, load_file=U('save2'),
                                                        load_rmc=False)},
        data=dict(data, map_status_results=[False]))
    add('load_check_metagame', host={'session': rich(launch=rich_launch(rng, mode=5, field_0c=5), load_file=U('mg'))}, data=data)
    zero = base_data()
    zero['registries']['START_PROPERTIES'] = {'count': 0}
    zero['registries']['OBJECT_DEFINITIONS'] = {'count': 3, 'reject': [1]}
    zl = rich_launch(rng)
    zl['map_default']['header']['kingdoms'][0]['start_properties'] = [None] * 9
    add('zero_width_and_rejected_defs', host={'session': rich(launch=zl)}, data=zero)
    nulls = base_data(); nulls['registries']['NATIONS'] = {'count': 4, 'null': [2]}
    nl = rich_launch(rng); nl['map_default']['header']['kingdoms'][0]['nation'] = None
    add('null_definition_entry', host={'session': rich(launch=nl)}, data=nulls)
    add('properties_and_options', host={'session': rich(info_text=U('rated=1;foo'), field_154=154, map_crc=0xfc, load_crc=0x104,
                                                        load_file=U('file108'), load_rmc=True, field_128=0x128, field_158=0x158,
                                                        field_15c=0x15c, text_160=U('text160'), field_164=0x164,
                                                        options={'name': U('Grand Game ☃'), 'password': U('pw'), 'max_players': -3,
                                                                 'field_d8': 0xd8, 'allow_observers': True},
                                                        properties={'flag': True, 'count': 3, 'entries': [
                                                            {'name': U('RATED'), 'value': 1}, {'name': U('map'), 'value': 0xffffffff},
                                                            {'name': U(''), 'value': 0}]})})
    add('properties_negative_count', host={'session': rich(properties={'flag': False, 'count': -3, 'entries': []})})
    prior = session(players=[player(conn(C3), 9, 'Stale', status=1)], last_player_id=99, counter_e0=41, state=3,
                    launch=rich_launch(rng), launch_88=rich_launch(rng),
                    properties={'flag': True, 'count': 2, 'entries': [{'name': U('old'), 'value': 1}, {'name': U('old2'), 'value': 2}]})
    add('client_prior_state', host={'session': rich(launch=launch(game=game(version=1)))},
        client={'session': prior, 'node': client_node(own_address=C4)})
    add('host_own_address_set', host={'node': host_node(own_address=addr(0x01020304, 1)), 'session': rich()})
    for i, (prec, now, at, dn, de, start) in enumerate([
            ('Single', 100.25, 10.0, 10, 12, 0), ('Double', 3.5, 3.0, 40, 45, 7), ('Extended', 1000.0, 999.9375, 0, 1, 0),
            ('Single', 0.1, 1.0e7, -5, 0x7ffffff0, 0), ('Double', 0.1, 1.0e7, -5, 0x7ffffff0, 3), ('Single', 5.0, 0.0, 100, 0, 0),
            ('Double', 1e-3, -2.5, 2, -0x80000000, 0), ('Single', 16777217.0, 3.0, 0x7fffffff, -2, 0)]):
        add(f'admin_state_{i}', host={'admin': host_admin(field_200=i * 3, field_204=0xffffffff - i, admin_time=f2b(at), decree_end=dn,
                                                          player=rng.choice([None, 1, 2, 7]))},
            client={'precision': prec, 'now': f2b(now), 'admin': client_admin_state(decree_end=de, decree_next=7, admin_time=f2b(1.0),
                    clock={'period': f2b(0.0625), 'offset': f2b(2.0), 'samples': [f2b(-1.0e30 if i == 5 else 1.25 * i)] + [f2b(9.0)] * 31}),
                    'sync': {'index': 3, 'hash': 99}, 'private_begin': start})
    add('admin_player_film', host={'admin': host_admin(player=2)},
        client={'session': session(players=[player('Node', 2, 'Film', film=True), player('Node', 2, 'Live')])})
    add('internet_host', host={'internet': True})
    add('writer_null_player_connection', host={'session': session(players=[player('Node', 1, 'Host'), player('Null', 2, 'Ghost')], created=1)})
    add('writer_null_created', host={'session': session(players=[player('Node', 1, 'Host')], created=None)})
    add('writer_null_properties', host={'session': session(players=[player('Node', 1, 'Host')], created=1, properties=None)})
    add('reader_state_kind_assert', host={'session': rich(launch=launch(state=state(field_24=7)))})
    add('reader_mode_assert', host={'session': rich(launch=launch(mode=6))})
    add('reader_session_state_assert', host={'session': rich(state=7)})
    add('reader_null_world_rule', host={'session': rich(launch=launch(map_24=mapset(conditions=wc(0, None, [rule(None, False, 0)]))))})
    return cases


def run_join_message_case(machines, case):
    host, client = machines
    r = host_join_messages(host, case)
    packets = list(r['private']['packets'])
    c = client_dispatch(client, case, packets) if packets else None
    return {'host': r, 'client': c}


def executed_windows(visited):
    ranges = []
    for a, size in sorted(visited.items()):
        if not PE0.base <= a < PE0.base + PE0.image_size: continue
        if ranges and ranges[-1][1] >= a: ranges[-1][1] = max(ranges[-1][1], a + size)
        else: ranges.append([a, a + size])
    out = []
    for a, b in ranges:
        raw = PE0.range(a, b - a)
        out.append({'start': hex(a), 'end': hex(b), 'sha256': sha(raw), 'v14_identical': raw == PE1.range(a, b - a)})
    return out


def address_cases(m):
    """Address interning 0x783700 (TGC_WinSock maps): object identity and stored port."""
    m.reset_node(host_node())
    rows = []; objs = {}
    for ip, port in [(0xc0a80105, 6112), (0xc0a80105, 6112), (0xc0a80105, 0x11a0 | 0x10000), (0xc0a80105, 0x11a0),
                     (1, 0), (0, 1), (0xffffffff, 0xffff), (0x0a000001, 0xffff0001), (0x0a000001, 1)]:
        o = m.call(0x783700, [ip, port], this=m.r(0xb40b80), callee_pop=8)
        objs.setdefault(o, len(objs))
        rows.append({'ip': ip, 'port': port, 'object': objs[o], 'stored': m.raddress(o)})
    return rows


def worker(job):
    """One process: an image and a slice of the cases (machines are not picklable)."""
    image, kind, indices = job
    machines = (LobbyMachine(True, image), LobbyMachine(False, image))
    out = []
    if kind == 'messages':
        cases = join_message_cases()
        out = [(i, run_join_message_case(machines, cases[i])) for i in indices]
    elif kind == 'decrees':
        cases = decree_cases()
        out = [(i, run_decree(machines[0] if cases[i]['host_side'] else machines[1], cases[i])) for i in indices]
    else:
        out = [(0, address_cases(machines[0]))]
    visited = {}
    for m in machines: visited.update(m.visited)
    return image, kind, out, visited


def main():
    import multiprocessing
    cases = join_message_cases(); dcases = decree_cases()
    jobs = []
    for image in ('original', 'v14'):
        for k in range(6):
            jobs.append((image, 'messages', list(range(k, len(cases), 6))))
        for k in range(3):
            jobs.append((image, 'decrees', list(range(k, len(dcases), 3))))
        jobs.append((image, 'addresses', [0]))
    results = {(im, kd): {} for im in ('original', 'v14') for kd in ('messages', 'decrees', 'addresses')}
    visited = {}
    with multiprocessing.Pool(int(sys.argv[1]) if len(sys.argv) > 1 else 6) as pool:
        for image, kind, out, v in pool.imap_unordered(worker, jobs):
            for i, r in out: results[(image, kind)][i] = r
            visited.update(v)
            print(image, kind, 'done', len(out), flush=True)
    def ordered(image, kind, n): return [results[(image, kind)][i] for i in range(n)]
    msgs = {im: ordered(im, 'messages', len(cases)) for im in ('original', 'v14')}
    decs = {im: ordered(im, 'decrees', len(dcases)) for im in ('original', 'v14')}
    adds = {im: ordered(im, 'addresses', 1)[0] for im in ('original', 'v14')}
    differ = [c['name'] for c, a, b in zip(cases, msgs['original'], msgs['v14']) if a != b]
    differ += [c['name'] for c, a, b in zip(dcases, decs['original'], decs['v14']) if a != b]
    if adds['original'] != adds['v14']: differ.append('addresses')
    rows = [dict(c, result=r) for c, r in zip(cases, msgs['original'])]
    drows = [dict(c, result=r) for c, r in zip(dcases, decs['original'])]
    windows = executed_windows(visited)
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
              'producer_sha256': sha(Path(__file__).read_bytes()),
              'harness_sha256': {n: sha((HERE / n).read_bytes()) for n in ('native_packets.py', 'native_channels.py', 'null_guard.py')},
              'admin_log_lines': {hex(k): v for k, v in ADMIN_LOG_LINES.items()},
              'v14_result_differences': differ, 'join_messages': rows, 'player_join': drows,
              'addresses': adds['original'], 'executed_windows': windows}
    (HERE / 'native-lobby.json').write_text(json.dumps(report) + chr(10))
    print(len(rows), 'join-message cases,', len(drows), 'decree cases; v14 result differences:', differ,
          '; v14 window differences:', [w['start'] for w in windows if not w['v14_identical']])
    for r in rows:
        h = r['result']['host']; c = r['result']['client']
        print(f"{r['name']:36s} host={h['status'][:50]:50s} client={(c or {}).get('status', '-')[:60]}")
    for r in drows:
        x = r['result']
        print(f"{r['name']:36s} {x['status'][:60]:60s} logs={[ADMIN_LOG_LINES.get(l, hex(l)) for l in x['logs']]}")


# --------------------------------------------------------------------------------
# PlayerJoin decree 0x75DA80 -> 0x758FD0
# --------------------------------------------------------------------------------

ADMIN_LOG_LINES = {0xb9e: 'full', 0xba1: 'full_host', 0xba8: 'full_already_joined', 0xbb2: 'rejected_self',
                   0xbbb: 'rejected_duplicate', 0x102f: 'set_disconnecting'}

def join_info(i):
    return {'name': U(i['name']), 'password': U(i.get('password', 'pw')), 'version': U(i.get('version', '1.0')),
            'build': U(i.get('build', 'AA')), 'data_checksum': i.get('checksum', 0x1234), 'address': i['address'],
            'player_id': pid(i.get('key', 'KEY'), i.get('serial', 7), 1, 2, 3)}

def build_decree(m, info):
    d = m.alloc(0x40); p = d + 4
    m.cs(p, info['name']); m.cs(p + 4, info['password']); m.cs(p + 8, info['version']); m.cs(p + 0xc, info['build'])
    m.w(p + 0x10, info['data_checksum']); m.w(p + 0x14, m.address(info['address']))
    k = info['player_id']; m.cs(p + 0x18, k['name']); m.w(p + 0x1c, k['serial'])
    m.w(p + 0x20, k['a']); m.w(p + 0x24, k['b']); m.w(p + 0x28, k['c'])
    return d

def conn_state(m, c):
    pl = lambda q: None if m.r(q) == 0 else m.r(m.r(q) + 0x20)
    return {'accepted': m.rb(c + 0x420) != 0, 'joined_player': pl(c + 8), 'ordering_player': pl(c + 0x2c),
            'disconnecting': m.rb(c + 0x89) != 0, 'disconnect_reason': m.r(c + 0x8c), 'disconnect_started': m.r(c + 0x90),
            'connected': m.rb(c + 0x88) != 0, 'identity': m.identity(c)}

def run_decree(m, case):
    """case: data, node (as join-message cases plus 'joined' / 'now' / 'internet'), session, info, admin, sync."""
    n = case['node']
    setup_common(m, case['data'], n)
    S = m.build_session(case['session'])
    m.b(FLAGS + 0xbc, case['internet'])
    m.w(NETNODE + 0x58, n['now'])
    m.w(0xb40930, 0)
    # Connections that already have a player (+8), and the accepted flag +0x420 of every known conn.
    for link, ident_id in n.get('joined', []):
        c = m.link_ptr(link)
        p = m.alloc(0x54); m.w(p + 0x20, ident_id); m.w(c + 8, p); m.w(c + 0x2c, p)
    for key, nc in m.netconns.items():
        c = m.r(nc + 0xa0); m.b(c + 0x420, 1); m.b(c + 0x89, 0); m.w(c + 0x8c, 0xd); m.w(c + 0x90, 0)
    a = case['admin']
    m.w(NODE + 0x200, a['field_200']); m.w(NODE + 0x204, a['field_204']); m.w(NODE + 0x28c, a['admin_time'])
    m.w(NODE + 0x218, a['decree_end'] & 0xffffffff); m.w(NODE + 0x208, 0)
    m.call(0x77d900, [case['sync']['index'] & 0xffffffff, case['sync']['hash'] & 0xffffffff], this=SYNC_A, callee_pop=8)
    m.u.mem_write(CONN + 0x88, b'\0')
    backup_before = s32(m.r(CONN + 0xa0))
    m.req = m.stream(WRITER_VT, size=0x100)
    m.decree_logs = []
    m.watch_logs = True
    d = build_decree(m, case['info'])
    status = 'ok'
    try:
        m.call(0x75da80, [], this=d)
    except Exception as e:
        status = 'error: ' + type(e).__name__ + ' ' + str(e)[:160]
    m.watch_logs = False
    end = m.position(m.req); bits = end['byte'] * 8 + end['bit']
    conns = {}
    for key in sorted(m.netconns):
        conns[f'{key[0]}:{key[1]}'] = conn_state(m, m.r(m.netconns[key] + 0xa0))
    return {'status': status, 'logs': m.decree_logs, 'session': m.read_session(S), 'events': m.lobby,
            'conns': conns, 'peer_list': m.peer_list(), 'peers_changed': m.rb(NODE + 0x2b4) != 0,
            'request': {'bytes': list(m.u.mem_read(m.r(m.req + 4), (bits + 7) // 8)), 'end': end},
            'private': history(m, CONN + 0xac), 'backup_index_before': backup_before, 'backup_index': s32(m.r(CONN + 0xa0)),
            'own_address': m.raddress(m.r(NODE + 0x29c))}

OPEN = launch(map_default=mapset(header=header([], [kingdom('Allies'), kingdom('Axis'), kingdom('Russia')])))
OPEN_DATA = {'Allies': True, 'Axis': True, 'Russia': True}

def decree_cases():
    rng = random.Random(317)
    cases = []
    def add(name, host=True, info=None, sess=None, node=None, data=None, internet=False, admin=None):
        nd = host_node() if host else client_node(own_address=addr(0x0a0000fe, 7777))
        nd.update(now=f2b(42.5), joined=[])
        nd.update(node or {})
        s = sess or session(players=[player('Node', 1, 'Host', kingdom=U('Allies'))], created=1, last_player_id=1, launch=OPEN)
        nd['peers'] = extra_peers(s['players'])
        cases.append({'name': name, 'host_side': host, 'data': data or base_data(kingdom_open=OPEN_DATA), 'node': nd, 'session': s,
                      'info': join_info(info or {'name': 'Bob', 'address': A}), 'internet': internet,
                      'admin': admin or host_admin(player=None, decree_end=25), 'sync': {'index': 40, 'hash': 777}})
    add('host_join')
    add('host_join_unicode_key', info={'name': 'Ölaf\U0001F600', 'address': A, 'key': 'Kéy', 'serial': 0xffffffff})
    add('host_join_long_name', info={'name': 'L' * 400, 'address': A})
    bobs = session(players=[player('Node', 1, 'Bob'), player('Node', 3, 'Bob (1)', ai=True), player('Node', 4, 'Bob (3)', ai=True)],
                   created=1, last_player_id=4, launch=OPEN)
    add('host_join_duplicate_names', sess=bobs)
    add('host_join_empty_name', sess=session(players=[player('Node', 1, '')], created=1, last_player_id=1, launch=OPEN), info={'name': '', 'address': A})
    add('host_join_nul_name', sess=session(players=[player('Node', 1, 'Bo')], created=1, last_player_id=1, launch=OPEN), info={'name': 'Bo\0b', 'address': A})
    add('host_already_joined', node={'joined': [(conn(A), 9)]})
    add('host_self_null_address', info={'name': 'Me', 'address': None})
    add('host_full_max_players', sess=session(players=[player('Node', 1, 'Host')], created=1, last_player_id=1,
                                              options={'name': U('G'), 'password': U(''), 'max_players': 1, 'field_d8': 0, 'allow_observers': False}))
    add('host_full_already_joined', node={'joined': [(conn(A), 9)]},
        sess=session(players=[player('Node', 1, 'Host')], created=1, last_player_id=1,
                     options={'name': U('G'), 'password': U(''), 'max_players': 1, 'field_d8': 0, 'allow_observers': False}))
    add('host_full_ai_and_film_do_not_count', sess=session(players=[player('Node', 1, 'Host'), player('Node', 2, 'AI', ai=True),
                                                                  player('Node', 3, 'Film', film=True)], created=1, last_player_id=3,
                                                         options={'name': U('G'), 'password': U(''), 'max_players': 2, 'field_d8': 0,
                                                                  'allow_observers': False}))
    kd = base_data(kingdom_open={'Allies': True, 'Axis': True, 'Neutral': True, 'Closed': False})
    kl = launch(map_default=mapset(header=header([], [kingdom('Allies'), kingdom('Axis', flag_28=False), kingdom('Neutral', flag_10=True),
                                                       kingdom('Closed')])))
    add('host_full_no_free_kingdom', sess=session(players=[player('Node', 1, 'Host', kingdom=U('Allies'))], created=1, last_player_id=1,
                                                  launch=kl), data=kd)
    kl2 = launch(map_default=mapset(header=header([], [kingdom('Allies'), kingdom('Free')])))
    add('host_free_kingdom', sess=session(players=[player('Node', 1, 'Host', kingdom=U('Allies'))], created=1, last_player_id=1,
                                          launch=kl2), data=base_data(kingdom_open={'Allies': True, 'Free': True}))
    add('host_observers_skip_kingdom_rule', sess=session(players=[player('Node', 1, 'Host', kingdom=U('Allies'))], created=1, last_player_id=1,
                                                         launch=kl,
                                                         options={'name': U('G'), 'password': U(''), 'max_players': 8, 'field_d8': 0,
                                                                  'allow_observers': True}), data=kd)
    add('host_metagame_launch_skips_kingdom_rule', sess=session(players=[player('Node', 1, 'Host', kingdom=U('Allies'))], created=1,
                                                                last_player_id=1, launch=dict(kl, field_0c=5)), data=kd)
    add('not_multiplayer_assert', sess=session(players=[player('Node', 1, 'Host')], created=1, last_player_id=1, multiplayer=False))
    rated = launch(game_type=1, mode=1, map_default=mapset(header=OPEN['map_default']['header'],
                                                         conditions=wc(0, None, [rule(1, True, 5), rule(0, False, 0x101)])))
    add('host_rated_rule_cleared', sess=session(players=[player('Node', 1, 'Host')], created=1, last_player_id=1, launch=rated))
    rated_net = launch(game_type=2, mode=2, file=U('Maps\\Europe.map'),
                       map_scenario=mapset(header=OPEN['map_default']['header'], conditions=wc(0, None, [rule(0, False, 0)])))
    add('internet_rated_set', internet=True,
        sess=session(players=[player('Node', 1, 'Host')], created=1, last_player_id=1, launch=rated_net, map_crc=0xfffffff6,
                     properties={'flag': True, 'count': 2, 'entries': [{'name': U('other'), 'value': 10},
                                                                         {'name': U('maps\\europe.MAP'), 'value': 10}]}))
    add('internet_not_rated_single_player_after_join', internet=True,
        sess=session(players=[player('Node', 1, 'Host', ai=True)], created=1, last_player_id=1, launch=rated_net, map_crc=10,
                     properties={'flag': True, 'count': 1, 'entries': [{'name': U('Maps\\Europe.map'), 'value': 10}]}))
    add('host_id_wrap', sess=session(players=[player('Node', 1, 'Host')], created=1, last_player_id=0xffffffff, launch=OPEN))
    # client side
    add('client_join_third_player', host=False, info={'name': 'Carl', 'address': C3})
    add('client_join_null_address_host_conn', host=False, info={'name': 'Carl', 'address': None})
    add('client_join_host_conn_has_player', host=False, info={'name': 'Carl', 'address': None}, node={'joined': [(conn(H), 1)]})
    add('client_self', host=False, info={'name': 'Me', 'address': addr(0x0a0000fe, 7777)})
    add('client_full_ignored', host=False, sess=session(players=[player(conn(H), 1, 'Host')], created=1, last_player_id=1,
                                                       options={'name': U('G'), 'password': U(''), 'max_players': 1, 'field_d8': 0,
                                                                'allow_observers': False}))
    add('client_join_known_address', host=False, info={'name': 'Dora', 'address': C4},
        sess=session(players=[player(conn(H), 1, 'Host'), player(conn(C4), 2, 'Old')], created=1, last_player_id=2, launch=OPEN))
    add('client_join_port_above_16_bits', host=False, info={'name': 'Eve', 'address': addr(0x0a000005, 0x1234)})
    return cases


if __name__ == '__main__':
    main()
