"""Original Connect packet (type 2) under Unicorn: writer
TGC_AdminConn::SerializeConnect(O) 0x750330 on a client node and reader /
validation SerializeConnect(I) 0x75BF90 on a host node (both with native
TGC_AdminConn objects from ctor 0x756BC0), including the player-id codecs
0x74EBC0/0x752660 (+0x74CCC0/0x74CD30), the connect-info object 0x74EDE0,
the disconnect setter 0x74B620 and, on acceptance, the PlayerJoin request
object 0x7589C0 whose info is serialized by the ORIGINAL 0x77B680 (the
TGC_PlayerJoinOrderRequest payload, descriptor 306).

Seams (recorded): allocator, CString manager, logging; 0x5AF810 depot
checksum string and 0x5AF970 mods string (return configured strings);
the ban-list lookup 0x54C240 inside the original 0x74D2F0, and the
banned-player prompt 0x75B4D0 (UI, internet only); request emission 0x74C9C0 (the request
object is captured and its info serialized natively); WinSock ntohl/ntohs
(IAT 0x9F0374/0x9F0348) byte swaps; _getptd 0x9738A9 returns a thread
block on the process C locale (the original _wcsicmp 0x96CAD6 then runs).
"""
from pathlib import Path
import json, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_packets import NodeMachine, NODE, CONN, NETCONN, SESSION_, NETNODE
from native_channels import READER_VT, WRITER_VT, PE0, PE1, sha
from unicorn.x86_const import *

STUB = 0x7700000; PTD = 0x202a000; DATA = 0x2026000; FLAGS = 0x2027000; ADDR = 0x2028000; DEPOT = 0x2029000

class ConnectMachine(NodeMachine):
    def __init__(self, host):
        super().__init__(host)
        self.u.mem_map(STUB, 0x1000)
        self.w(0x9f0374, STUB); self.w(0x9f0348, STUB + 0x10)
        self.w(0xb36530, DATA); self.w(0xb36540, DEPOT); self.w(0xb3654c, FLAGS)
        self.w(SESSION_ + 0xd0, self.string(''))
        self.w(PTD + 0x64, self.r(0xb35444))
        self.events = []; self.depot = ''; self.mods = ''; self.banned = False
    def hook(self, u, pc, size, user):
        if self.mode == 'channels':
            if pc == STUB:
                v = self.arg(); self.ret(4, struct.unpack('<I', struct.pack('>I', v))[0]); return
            if pc == STUB + 0x10:
                v = self.arg() & 0xffff; self.ret(4, ((v & 0xff) << 8) | (v >> 8)); return
            if pc == 0x9738a9:
                # _getptd: a thread whose locale info is the process one (__initiallocinfo
                # 0xB353F0, LC_CTYPE handle 0 = "C"; AA only ever sets LC_NUMERIC, 0x7F1965/0x7F1A39),
                # so the ORIGINAL _wcsicmp 0x96CAD6 runs its C-locale ASCII path.
                self.ret(0, PTD); return
            if pc == 0x5af810:
                out = self.arg(); self.w(out, self.string(self.depot)); self.ret(4, out); return
            if pc == 0x5af970:
                out = self.arg(); self.w(out, self.string(self.mods)); self.ret(4, out); return
            if pc == 0x54c240:
                # Ban-list lookup used by the ORIGINAL 0x74D2F0 (index, or >= 0x80000000 when absent).
                p = self.arg(); self.events.append({'event': 'ban_lookup', 'key': self.text(self.r(p))})
                self.ret(4, 0 if self.banned else 0xffffffff); return
            if pc == 0x75b4d0:
                self.events.append({'event': 'banned_prompt'}); self.ret(8); return
            if pc == 0x74c9c0:
                # Capture the request's info (0x2C bytes at request+4); its CStrings live on the
                # never-freed harness heap. 0x77B680 serializes it after the reader returns.
                req = u.reg_read(UC_X86_REG_ECX); copy = self.alloc(0x30)
                self.u.mem_write(copy, bytes(self.u.mem_read(req + 4, 0x2c)))
                self.events.append({'event': 'player_join_request', 'vtable': hex(self.r(req)), 'player_arg': self.arg()})
                self.captured = copy
                self.ret(4); return
        super().hook(u, pc, size, user)

def text(m, units):
    return m.string(units)

def configure_client(m, c):
    m.w(NODE + 0xec, m.string(c['name'])); m.w(NODE + 0xf0, m.string(c['password']))
    m.w(0xb3888c, m.string(c['version'])); m.w(0xb388a0, m.string(c['build']))
    m.w(DATA + 8, c['checksum']); m.depot = c['depot']; m.mods = c['mods']
    pid = NODE + 0x104
    m.w(pid, m.string(c['player_id']['name']))
    for k, off in [('serial', 4), ('a', 8), ('b', 0xc), ('c', 0x10)]: m.w(pid + off, c['player_id'][k])
    m.u.mem_write(FLAGS + 0xbc, bytes([int(c['internet'])]))

def configure_host(m, h):
    m.w(NODE + 0x2c4, NODE if h['we_are_host'] else CONN)
    m.w(NODE + 0x2e0, int(h['migrated_host']))
    m.u.mem_write(FLAGS + 0xbc, bytes([int(h['internet'])]))
    m.w(SESSION_ + 0xe4, h['session_mode']); m.w(SESSION_ + 0x118, h['session_players_in_game'])
    m.w(SESSION_ + 0xd0, m.string(h['password']))
    m.w(0xb3888c, m.string(h['version'])); m.w(0xb388a0, m.string(h['build']))
    m.w(DATA + 8, h['checksum']); m.depot = h['depot']; m.mods = h['mods']; m.banned = h['banned']
    m.u.mem_write(CONN + 0x420, bytes([int(h['conn_accepted'])]))
    m.u.mem_write(CONN + 0x88, bytes([int(h['conn_connected'])]))
    m.u.mem_write(CONN + 0x89, bytes([int(h['conn_disconnecting'])]))
    m.w(CONN + 0x8c, 0xd); m.w(CONN + 0x90, 0)
    m.w(NETNODE + 0x58, struct.unpack('<I', struct.pack('<f', h['now']))[0])
    m.w(NETCONN + 0x9c, ADDR)
    m.u.mem_write(ADDR + 6, struct.pack('>H', h['peer_port'])); m.u.mem_write(ADDR + 8, struct.pack('>I', h['peer_ip']))

BASE_CLIENT = {'name': 'Player One', 'password': 'secret', 'version': '1.0.0.2', 'build': 'AA2004', 'checksum': 0x1234abcd,
               'depot': 'DepotSums', 'mods': '', 'internet': False,
               'player_id': {'name': 'cdkeyhash', 'serial': 7, 'a': 0x11111111, 'b': 0x22222222, 'c': 0x33333333}}
BASE_HOST = {'we_are_host': True, 'migrated_host': False, 'internet': False, 'session_mode': 1, 'session_players_in_game': 0,
             'password': 'secret', 'version': '1.0.0.2', 'build': 'AA2004', 'checksum': 0x1234abcd, 'depot': 'DepotSums',
             'mods': '', 'banned': False, 'conn_accepted': False, 'conn_connected': False, 'conn_disconnecting': False,
             'now': 12.5, 'peer_ip': 0xc0a80105, 'peer_port': 6112}

def cases():
    out = [('accepted', {}, {})]
    out += [('not_host', {}, {'we_are_host': False}), ('already_accepted', {}, {'conn_accepted': True}),
            ('conn_connected', {}, {'conn_connected': True}), ('conn_disconnecting', {}, {'conn_disconnecting': True}),
            ('migrated_host', {}, {'migrated_host': True}), ('internet_mismatch', {'internet': True}, {}),
            ('mode_2', {}, {'session_mode': 2}), ('mode_4', {}, {'session_mode': 4}), ('mode_0', {}, {'session_mode': 0}),
            ('in_game', {}, {'session_players_in_game': 3}), ('version', {'version': '1.0.0.1'}, {}),
            ('version_case', {'version': '1.0.0.2'.upper() + 'x'}, {}), ('build', {'build': 'aa2004'}, {}),
            ('password_wrong', {'password': 'Secret'}, {}), ('no_host_password', {'password': 'anything'}, {'password': ''}),
            ('internet_ok', {'internet': True}, {'internet': True}),
            ('internet_banned', {'internet': True}, {'internet': True, 'banned': True}),
            ('internet_empty_id', {'internet': True, 'player_id': dict(BASE_CLIENT['player_id'], name='')}, {'internet': True, 'banned': True}),
            ('depot_mismatch', {'depot': 'Other'}, {}), ('depot_case_only', {'depot': 'DEPOTSUMS'}, {}),
            ('mods_mismatch', {'mods': 'ModA'}, {}), ('mods_case_only', {'mods': 'moda'}, {'mods': 'MODA'}),
            ('checksum', {'checksum': 0x1234abce}, {}),
            ('version_and_password', {'version': 'x', 'password': 'y'}, {}),
            ('password_and_depot', {'password': 'y', 'depot': 'z'}, {}),
            ('depot_mods_checksum', {'depot': 'z', 'mods': 'm', 'checksum': 1}, {}),
            ('mode2_and_version', {'version': 'x'}, {'session_mode': 2}),
            ('unicode_name', {'name': 'Jörg 日本', 'player_id': dict(BASE_CLIENT['player_id'], name='é')}, {}),
            ('empty_strings', {'name': '', 'password': '', 'player_id': dict(BASE_CLIENT['player_id'], name='')}, {'password': ''}),
            ('long_name', {'name': 'N' * 300}, {})]
    return out

def main():
    client = ConnectMachine(False); host = ConnectMachine(True); rows = []
    for name, cd, hd in cases():
        c = dict(BASE_CLIENT, **cd); h = dict(BASE_HOST, **hd)
        configure_client(client, c)
        W = client.stream(WRITER_VT, size=0x2000)
        client.call(0x750330, [W, 0], this=CONN, callee_pop=8)
        end = client.position(W); bits = end['byte'] * 8 + end['bit']
        data = list(client.u.mem_read(client.r(W + 4), (bits + 7) // 8))
        configure_host(host, h); host.events = []
        host.w(NETCONN + 0x34, 0)
        R = host.stream(READER_VT, data)
        host.captured = None
        host.call(0x75bf90, [R, 0], this=CONN, callee_pop=8)
        read_end = host.position(R)
        if host.captured is not None:
            W2 = host.stream(WRITER_VT, size=0x2000)
            host.call(0x77b680, [W2, host.captured], this=0, callee_pop=0)
            pend = host.position(W2); pbits = pend['byte'] * 8 + pend['bit']
            host.events[-1]['payload'] = list(host.u.mem_read(host.r(W2 + 4), (pbits + 7) // 8))
            host.events[-1]['payload_end'] = pend
        rows.append({'name': name, 'client': c, 'host': h, 'bytes': data, 'end': end, 'read_end': read_end,
                     'events': host.events,
                     'conn': {'accepted': host.u.mem_read(CONN + 0x420, 1)[0], 'disconnecting': host.u.mem_read(CONN + 0x89, 1)[0],
                              'reason': host.r(CONN + 0x8c), 'disconnect_time_bits': host.r(CONN + 0x90)}})
    visited = {**client.visited, **host.visited}
    ranges = []
    for a, size in sorted(visited.items()):
        if not PE0.base <= a < PE0.base + PE0.image_size: continue
        if ranges and ranges[-1][1] >= a: ranges[-1][1] = max(ranges[-1][1], a + size)
        else: ranges.append([a, a + size])
    windows = []
    for a, b in ranges:
        raw = PE0.range(a, b - a)
        windows.append({'start': hex(a), 'end': hex(b), 'sha256': sha(raw), 'v14_identical': raw == PE1.range(a, b - a)})
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
              'producer_sha256': sha(Path(__file__).read_bytes()), 'cases': rows, 'executed_windows': windows}
    (HERE / 'native-connect.json').write_text(json.dumps(report) + '\n')
    print(len(rows), 'cases;', 'v14 differs at', [w['start'] for w in windows if not w['v14_identical']])
    for r in rows: print(r['name'], r['conn'], [e['event'] for e in r['events']])

if __name__ == '__main__':
    main()
