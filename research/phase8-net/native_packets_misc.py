"""Original admin packet types 3, 6, 7, 8, 10 and 11 under Unicorn, on two
isolated emulated nodes with native TGC_AdminConn objects (ctor 0x756BC0):

  3  Reconnect               writer 0x74C2D0, reader 0x753820 (+0x752300)
  6  ClientToBackupServer    writer 0x74E3C0, reader 0x74E430 (status 0x74C7B0)
  7  BackupServerToClient    writer 0x74C310, reader 0x75A5C0 (+0x7523A0, 0x74B6E0)
  8  ServerDisconnect        writer 0x74B680, reader 0x75A730
  10 ClientDisconnectRequest writer 0x74B6B0, reader 0x74F7C0
  11 ClientDisconnectDecree  writer 0x74C380, reader 0x75A780 (gate, reason and
                             the session take-over; the run is stopped at
                             0x75A878, before the UI dialog and ClientLeave)

Bindings run natively (0x7525B0, 0x7523A0, 0x752300). Node-level calls are
recording seams: SetServer 0x759790, SetHost 0x757010, Timeout 0x75A250,
PlayerLeave request 0x74EAE0, admin stop 0x74B980; logging 0x404F60 records
the AdminLayer.cpp line. Gate inputs are enumerated per reader.
"""
from pathlib import Path
import itertools, json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_packets import NodeMachine, NODE, CONN, NETCONN, SESSION_, NETNODE
from native_channels import finalized, READER_VT, WRITER_VT, PE0, PE1, sha
from unicorn.x86_const import *

OTHER = 0x2029000; PLAYER = 0x202a000; CLOCK = 0x202b000
SEAMS = {0x759790: ('set_server', 4), 0x757010: ('set_host', 4), 0x75a250: ('timeout', 0),
         0x74eae0: ('player_leave_request', 4), 0x74b980: ('admin_stop', 0)}

class MiscMachine(NodeMachine):
    def __init__(self, host):
        self.events = []; self.stop_at = None; self.stopped = False
        super().__init__(host)
        self.events = []
    def hook(self, u, pc, size, user):
        if self.mode == 'channels':
            if pc == 0x404f60:
                self.events.append({'event': 'log', 'line': self.r(u.reg_read(UC_X86_REG_ESP) + 12)})
                self.ret(); return
            if pc in SEAMS:
                name, pop = SEAMS[pc]
                e = {'event': name}
                if pop: e['arg'] = self.label(self.arg())
                if name == 'player_leave_request':
                    req = u.reg_read(UC_X86_REG_ECX)
                    e['player'] = self.label(self.r(req + 4)); e['reason'] = self.r(req + 8)
                self.events.append(e); self.ret(pop); return
            if pc == self.stop_at:
                self.stopped = True; u.emu_stop(); return
        super().hook(u, pc, size, user)
    def label(self, p):
        return {NODE: 'node', CONN: 'conn', OTHER: 'other', PLAYER: 'player', 0: None}.get(p, hex(p))
    def run_until(self, pc, args, stop_at, callee_pop):
        """Call; when `stop_at` is reached the run ends there (no return)."""
        self.stop_at = stop_at; self.stopped = False
        try:
            self.call(pc, args, this=CONN, callee_pop=callee_pop)
        except AssertionError:
            if not self.stopped: raise
        self.stop_at = None
        return self.stopped

def roles(m, c):
    """Apply the gate inputs of case `c` to node/conn fields."""
    m.w(NODE + 0x2c4, NODE if c.get('we_are_host') else (CONN if c.get('conn_is_host') else OTHER))
    m.w(NODE + 0x2c8, NODE if c.get('we_are_server') else (CONN if c.get('conn_is_server') else OTHER))
    m.u.mem_write(NODE + 0x2cc, bytes([c.get('connecting', 0)]))
    m.w(NODE + 0x158, CLOCK if c.get('admin_clock') else 0)
    m.w(NODE + 8, PLAYER if c.get('local_player') else 0)
    m.w(NODE + 0x70, c.get('node_rank', 0)); m.w(CONN + 0x70, c.get('conn_rank', 0))
    m.w(CONN + 8, PLAYER if c.get('conn_player') else 0)
    m.u.mem_write(CONN + 0x88, bytes([c.get('connected', 0), c.get('disconnecting', 0)]))
    m.u.mem_write(CONN + 0x9c, bytes([0]))
    m.w(SESSION_ + 0xe4, c.get('session_state', 0))

def body(m, fn, seq):
    W = m.stream(WRITER_VT)
    m.call(fn, [W, seq], this=CONN, callee_pop=8)
    end = m.position(W); bits = end['byte'] * 8 + end['bit']
    return list(m.u.mem_read(m.r(W + 4), (bits + 7) // 8)), end

def conn_state(m):
    s = lambda p: struct.unpack('<i', m.u.mem_read(p, 4))[0]
    return {'connected': m.u.mem_read(CONN + 0x88, 1)[0], 'server_disconnected': m.u.mem_read(CONN + 0x9c, 1)[0],
            'backup_index': s(CONN + 0xa0), 'reverse_start': s(CONN + 0xa4), 'status_word': m.r(CONN + 0x60),
            'backlog': m.u.mem_read(CONN + 0xa8, 1)[0], 'falling_behind': m.u.mem_read(CONN + 0xa9, 1)[0],
            'backup_decree_next': s(NODE + 0x234)}

def gates(keys, rng, n):
    allc = [dict(zip(keys, v)) for v in itertools.product([0, 1], repeat=len(keys))]
    rng.shuffle(allc)
    return allc[:n]

def reconnect_cases(rng):
    out = []
    w = MiscMachine(False); w.w(NODE + 0x218, 0x11223344)
    data, end = body(w, 0x74c2d0, 0)
    for g in gates(['admin_clock', 'local_player', 'disconnecting', 'most_recent', 'we_are_server', 'conn_player', 'rank_ok', 'connected'], rng, 60) + \
             [dict(admin_clock=1, local_player=1, disconnecting=0, most_recent=1, we_are_server=1, conn_player=1, rank_ok=1, connected=c) for c in (0, 1)]:
        m = MiscMachine(True); m.events = []
        g = dict(g, node_rank=2, conn_rank=3 if g['rank_ok'] else 1)
        roles(m, g)
        m.w(NETCONN + 0x34, 0 if g['most_recent'] else 5)
        R = m.stream(READER_VT, data); m.call(0x753820, [R, 0], this=CONN, callee_pop=8)
        out.append({'case': g, 'bytes': data, 'end': end, 'read_end': m.position(R), 'events': m.events, 'conn': conn_state(m),
                    'conn_requests_begin_backup': struct.unpack('<i', m.u.mem_read(CONN + 0x1c, 4))[0]})
    return out

def backup_up_cases(rng):
    """Type 6: client conn bound by 0x7525B0 + 0x7523A0(reverse start), server conn by 0x752300."""
    out = []
    for k, g in enumerate(gates(['we_are_server', 'we_are_host', 'disconnecting', 'connected', 'index_ok', 'most_recent'], rng, 40)
                          + [dict(we_are_server=1, we_are_host=0, disconnecting=0, connected=1, index_ok=1, most_recent=m) for m in (0, 1)] * 2):
        c = MiscMachine(False); s = MiscMachine(True)
        c.call(0x7525b0, [], this=CONN, callee_pop=0)
        # As after the broadcast decree header arrived (0x7631D0 -> 0x750880(0)).
        c.w(NODE + 0x214, 0); c.w(NODE + 0x218, 0)
        reverse_start = [-1, 0, 2][k % 3]
        for i in range(3): c.append(0x214, finalized([rng.getrandbits(1) for _ in range(rng.randrange(0, 300))]))
        c.call(0x7523a0, [reverse_start & 0xffffffff], this=CONN, callee_pop=4)
        for i in range(2): c.append(0x1c, finalized([rng.getrandbits(1) for _ in range(rng.randrange(0, 300))]))
        c.w(CONN + 0xa0, 1 if g['index_ok'] else 7)
        c.w(CONN + 0x60, rng.getrandbits(32)); c.u.mem_write(NETNODE + 0x64, bytes([k & 1]))
        c.w(NODE + 0x2c4, OTHER)  # the backup server is not the host conn
        data, end = body(c, 0x74e3c0, 0)
        s.call(0x752300, [], this=CONN, callee_pop=0)
        s.events = []
        roles(s, g); s.w(NETCONN + 0x34, 0 if g['most_recent'] else 5)
        R = s.stream(READER_VT, data); s.call(0x74e430, [R, 0], this=CONN, callee_pop=8)
        out.append({'case': dict(g, reverse_start=reverse_start, client_status_word=c.r(CONN + 0x60), falling_behind=k & 1,
                                 client_decrees=c.history(NODE + 0x214), client_backup_requests=c.history(NODE + 0x1c)),
                    'bytes': data, 'end': end, 'read_end': s.position(R), 'events': s.events, 'conn': conn_state(s),
                    'server_backup_requests': s.history(CONN + 0x1c), 'server_reverse': s.history(CONN + 0xbc)})
    return out

def backup_down_cases(rng):
    """Type 7: server conn bound by 0x752300 (backup index 1); client conn bound by 0x7525B0."""
    out = []
    for k, g in enumerate(gates(['conn_is_server', 'conn_is_host', 'admin_clock', 'local_player', 'most_recent', 'rank_ok', 'index_same'], rng, 50)
                          + [dict(conn_is_server=1, conn_is_host=0, admin_clock=1, local_player=1, most_recent=1, rank_ok=1, index_same=x) for x in (0, 1)]
                          + [dict(conn_is_server=0, conn_is_host=0, admin_clock=1, local_player=1, most_recent=1, rank_ok=1, index_same=x) for x in (0, 1)]):
        s = MiscMachine(True); c = MiscMachine(False)
        for i in range(2): s.append(0x224, finalized([rng.getrandbits(1) for _ in range(rng.randrange(0, 300))]))
        s.call(0x752300, [], this=CONN, callee_pop=0)
        for i in range(2): s.append(0x224, finalized([rng.getrandbits(1) for _ in range(rng.randrange(0, 300))]))
        s.w(NODE + 0x218, 40 + k)
        data, end = body(s, 0x74c310, 0)
        c.call(0x7525b0, [], this=CONN, callee_pop=0)
        if g['index_same']:
            # As after an earlier NewBackupIndex: 0x7523A0(decree end) then 0x74B6E0(1).
            c.call(0x7523a0, [0], this=CONN, callee_pop=4); c.call(0x74b6e0, [1], this=CONN, callee_pop=4)
        c.events = []
        g = dict(g, node_rank=2, conn_rank=1 if g['rank_ok'] else 3)
        roles(c, g)
        c.w(NETCONN + 0x34, 0 if g['most_recent'] else 5)
        R = c.stream(READER_VT, data); c.call(0x75a5c0, [R, 0], this=CONN, callee_pop=8)
        out.append({'case': g, 'server_backup_decrees': s.history(NODE + 0x224), 'bytes': data, 'end': end,
                    'read_end': c.position(R), 'events': c.events, 'conn': conn_state(c),
                    'client_backup_decrees': c.history(NODE + 0x224), 'client_backup_requests': c.history(NODE + 0x1c)})
    return out

def empty_cases(rng):
    out = []
    for fn, rd, keys in [(0x74b680, 0x75a730, ['conn_is_server', 'we_are_server']),
                         (0x74b6b0, 0x74f7c0, ['we_are_server', 'we_are_host', 'conn_player'])]:
        for g in gates(keys, rng, 1 << len(keys)):
            m = MiscMachine(True)
            data, end = body(m, fn, 0)
            m.events = []; roles(m, g)
            R = m.stream(READER_VT, data); m.call(rd, [R, 0], this=CONN, callee_pop=8)
            out.append({'type': 8 if fn == 0x74b680 else 10, 'case': g, 'bytes': data, 'end': end, 'read_end': m.position(R),
                        'events': m.events, 'conn': conn_state(m)})
    return out

def decree_cases(rng):
    out = []
    for k, g in enumerate(gates(['we_are_server', 'conn_is_server', 'conn_is_host', 'connecting', 'session_state'], rng, 32)):
        w = MiscMachine(True); w.w(CONN + 0x8c, k % 13)
        data, end = body(w, 0x74c380, 0)
        m = MiscMachine(False); m.events = []; roles(m, g)
        R = m.stream(READER_VT, data)
        stopped = m.run_until(0x75a780, [R, 0], 0x75a878, 8)
        out.append({'case': g, 'reason_written': k % 13, 'bytes': data, 'end': end, 'read_end': m.position(R),
                    'reached_dialog': stopped, 'events': m.events,
                    'node_2e0': m.r(NODE + 0x2e0)})
    return out

def main():
    rng = random.Random(91)
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
              'producer_sha256': sha(Path(__file__).read_bytes())}
    for name, fn in [('reconnect', reconnect_cases), ('client_to_backup', backup_up_cases), ('backup_to_client', backup_down_cases),
                     ('empty_bodies', empty_cases), ('disconnect_decree', decree_cases)]:
        report[name] = fn(rng)
        print(name, len(report[name]), flush=True)
    (HERE / 'native-packets-misc.json').write_text(json.dumps(report) + '\n')

if __name__ == '__main__':
    main()
