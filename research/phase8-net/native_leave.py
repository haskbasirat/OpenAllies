"""Original TGC_AdminNode::ClientLeave 0x755840(peer, reason, message, check
votes) under Unicorn on native AdminNode / AdminConn objects (built as in
native_heartbeat.py), with its node-level helpers executed natively:

  0x74F160 vote-kick sweep (0x74D380 majority, 0x74D320 votes per peer,
           recursive ClientLeave(peer, 4, 1, 1)),
  0x74EF90 vote update (vote 0: the leaving connection is removed from every
           remaining peer's +0x64 list, 0x4F1430),
  0x751F70 / 0x74DA50 / 0x74DAA0 task completion check and ClearAllTasks,
  0x4F1430 peer-list removal (+0x2AC), the AdminNode/AdminConn downcast
           virtual +4 (0x4C9FA0 / 0x8D6180), 0x8569A0, the SetDisconnecting
           stores, KKC_Session::DestroyPlayer 0x7737E0 (with the list erase).

Recording seams: logs 0x404F60, name helpers 0x766380 / 0x7667D0 and the
UI message 0x40BC50 (as in native_heartbeat.py), the launch countdown abort
0x76CFA0 (ported and proven by native-launch.json), AdminConn::Reset
0x752110 (proven by native-heartbeat.json), the timeline byte 0x76AA80, the
GameSpy report 0x788290, the in-game leave 0x76EE10, the task history
0x750940 / 0x750990 on AdminNode+0x238, the session task completion
0x776560, the rated-flag update 0x770280, the SessionPlayer deleting
destructor (vtable slot 0) and the engine free 0x6F7430 (a no-op: the
engine pool's free 0x85C2B0 only clears a bitmap bit, so a freed list node
keeps its links, which 0x74F160 reads after the recursive ClientLeave).
"""
from pathlib import Path
import copy, json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_heartbeat import HbMachine, base, conn, env, node, f2b, s32, CONN0, CONN_STRIDE
from native_channels import PE0, PE1, sha
from unicorn.x86_const import *

PLAYER_VT = 0x7700400; DELETE_STUB = 0x7700410; TIMELINE = 0x7700600; GAMESPY_STATS = 0x7700700

class LeaveMachine(HbMachine):
    def __init__(self):
        super().__init__()
        self.u.mem_write(DELETE_STUB, b'\xc3'); self.w(PLAYER_VT, DELETE_STUB)
        self.stubs.update({
            0x6f7430: lambda: self.ret(),
            0x76cfa0: lambda: (self.ev('abort_countdown'), self.ret()),
            0x752110: lambda: (self.ev('reset_conn', conn=self.peer_name(self.ecx())), self.ret()),
            0x76aa80: lambda: (self.ev('timeline_28', arg=self.arg(0) & 0xff), self.ret(4)),
            0x788290: lambda: (self.ev('gamespy_player_left', player=self.arg(0)), self.ret(4)),
            0x76ee10: lambda: (self.ev('in_game_leave'), self.ret()),
            0x750940: lambda: (self.ev('task_history_clear', this_is_238=self.ecx() == self.admin + 0x238), self.ret()),
            0x750990: lambda: (self.ev('task_history_first', arg=self.arg(0)), self.ret(4)),
            0x776560: lambda: (self.ev('tasks_complete', flag=self.arg(0) & 0xff), self.ret(4)),
            0x770280: lambda: (self.ev('update_rated'), self.ret()),
            DELETE_STUB: lambda: (self.ev('delete_player', player=self.pid(self.ecx()), arg=self.arg(0)), self.ret(4)),
            0x7667d0: self.display_name,
        })
    def display_name(self):
        # 0x7667D0 on a null player: every path dereferences it (0x766819 reads
        # +0x28, 0x766380 reads +8), reported as the native null read.
        if self.ecx() == 0:
            self.null_accesses.append({'pc': '0x7667d0', 'address': 0x28, 'size': 4, 'write': False})
            self.u.emu_stop(); return
        self.name_stub('D')()

def build(m, st):
    m.setup(st)
    L = st['leave']; a = m.admin; S = m.session
    m.w(0xb36550, S); m.w(0xb36554, TIMELINE); m.w(0xb40c24, GAMESPY_STATS)
    m.u.mem_write(TIMELINE, bytes(0x40)); m.b(TIMELINE + 0x28, 1); m.b(TIMELINE + 0x29, L['timeline_29'])
    m.b(S + 0xf4, L['multiplayer']); m.w(S + 0xe4, L['session_state'])
    # Session players: {id, link ref, field_28}; vtable slot 0 = deleting destructor stub.
    prev = 0
    for pl in reversed(L['players']):
        p = m.player(pl['id']); m.w(p, PLAYER_VT); m.w(p + 4, m.ref(pl['link'])); m.w(p + 0x28, pl['field_28'])
        n = m.alloc(8); m.w(n, p); m.w(n + 4, prev); prev = n
    m.w(S + 0xbc, prev)
    # Vote lists (+0x64/+0x68) hold peer pointers.
    for i, r in enumerate(['node'] + list(range(len(st['conns'])))):
        p = m.ref(r); prev = 0; tail = 0
        for v in reversed(L['votes'].get(str(r), [])):
            n = m.alloc(8); m.w(n, m.ref(v)); m.w(n + 4, prev)
            if prev == 0: tail = n
            prev = n
        m.w(p + 0x64, prev); m.w(p + 0x68, tail)
        m.w(p + 0x34, L['tasks'].get(str(r), 0) & 0xffffffff)
    m.w(a + 0x2a0, m.ref(L['field_2a0'])); m.w(a + 0x2a4, m.ref(L['field_2a4']))
    m.b(a + 0x2b4, 0); m.b(a + 0x8c, 1)

def peer_list(m, head):
    out = []; n = head
    while n:
        out.append(m.unref(m.r(n))); n = m.r(n + 4)
    return out

def state_after(m, st):
    a = m.admin; S = m.session
    refs = ['node'] + list(range(len(st['conns'])))
    peers = {}
    for r in refs:
        p = m.ref(r)
        peers[str(r)] = {'player': m.pid(m.r(p + 8)), 'ordering_player': m.pid(m.r(p + 0x2c)), 'task': s32(m.r(p + 0x34)),
                         'votes': peer_list(m, m.r(p + 0x64)), 'vote_tail': m.unref(m.r(p + 0x68))}
        if r != 'node':
            peers[str(r)].update({'connected': m.rb(p + 0x88), 'disconnecting': m.rb(p + 0x89),
                                  'disconnect_reason': m.r(p + 0x8c), 'disconnect_time': m.r(p + 0x90)})
    players = []; n = m.r(S + 0xbc)
    while n:
        players.append(m.pid(m.r(n))); n = m.r(n + 4)
    return {'list': peer_list(m, m.r(a + 0x2ac)), 'peers': peers, 'field_2a0': m.unref(m.r(a + 0x2a0)),
            'field_2a4': m.unref(m.r(a + 0x2a4)), 'peers_changed': m.rb(a + 0x2b4), 'node_8c': m.rb(a + 0x8c),
            'timeline_28': m.rb(TIMELINE + 0x28), 'session_players': players}

def make_case(rng):
    nconn = rng.randrange(0, 5)
    conns = []
    for i in range(nconn):
        pid = i + 2 if rng.random() < 0.85 else None
        c = conn(rng); c['base'] = base(rng, player=pid, ordering_player=pid if rng.random() < 0.9 else None)
        conns.append(c)
    order = list(range(nconn)); rng.shuffle(order)
    listed = [i for i in order if rng.random() < 0.85]
    host = 'node' if rng.random() < 0.65 else (rng.choice(range(nconn)) if nconn else 'node')
    st = {'node': node(rng, host=host, server=host), 'conns': conns, 'list': ['node'] + listed,
          'env': env(now=f2b(rng.choice([3.5, 10.0, 77.25])), session_state=rng.choice([1, 2]), gamespy=int(rng.random() < 0.4))}
    refs = ['node'] + listed
    players = [{'id': 1, 'link': 'node', 'field_28': rng.choice([0, 1])}]
    for i in range(nconn):
        pid = conns[i]['base']['player']
        if pid is not None and rng.random() < 0.9:
            players.append({'id': pid, 'link': i, 'field_28': rng.choice([0, 1, 1])})
        if rng.random() < 0.15:
            players.append({'id': 20 + i, 'link': i, 'field_28': 1})
    if rng.random() < 0.3:
        players.append({'id': 30, 'link': 'node', 'field_28': 1})
    rng.shuffle(players)
    votes = {}
    for r in refs:
        votes[str(r)] = [v for v in refs if v != r and rng.random() < 0.45]
    target = 'node' if rng.random() < 0.2 or not nconn else rng.choice(range(nconn))
    st['leave'] = {
        'target': target, 'reason': rng.choice([4, 5, 0xb, 0, 2, 13]), 'message': int(rng.random() < 0.6),
        'check_votes': int(rng.random() < 0.6), 'timeline_29': int(rng.random() < 0.5),
        'multiplayer': int(rng.random() < 0.8), 'session_state': st['env']['session_state'],
        'players': players, 'votes': votes,
        'tasks': {str(r): rng.choice([0, 1, 2, 3, 3, 4, 5]) for r in ['node'] + list(range(nconn))},
        'field_2a0': rng.choice([None, 'node'] + list(range(nconn))), 'field_2a4': rng.choice([None] + list(range(nconn))),
    }
    # Session players of a peer drive the 0x74D380 player +0x28 read: keep every listed peer's
    # player pointer valid in most cases (a null one is the native null read).
    return st

def main():
    m = LeaveMachine(); rng = random.Random(0x755840); rows = []
    for k in range(400):
        st = make_case(rng)
        L = st['leave']
        build(m, st)
        status = 'ok'
        try:
            m.call(0x755840, [m.ref(L['target']), L['reason'], L['message'], L['check_votes']], this=m.admin, callee_pop=0x10)
        except Exception as e:
            status = 'error: ' + type(e).__name__ + ' ' + str(e)[:160]
        rows.append({'before': st, 'status': status, 'events': m.hb_events,
                     'after': state_after(m, st) if status == 'ok' else None})
    ranges = []
    for a, size in sorted(m.visited.items()):
        if not PE0.base <= a < PE0.base + PE0.image_size: continue
        if ranges and ranges[-1][1] >= a: ranges[-1][1] = max(ranges[-1][1], a + size)
        else: ranges.append([a, a + size])
    windows = [{'start': hex(a), 'end': hex(b), 'sha256': sha(PE0.range(a, b - a)),
                'v14_identical': PE0.range(a, b - a) == PE1.range(a, b - a)} for a, b in ranges]
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
              'producer_sha256': sha(Path(__file__).read_bytes()), 'cases': rows, 'executed_windows': windows}
    (HERE / 'native-leave.json').write_text(json.dumps(report) + '\n')
    import collections
    print(collections.Counter(r['status'][:60] for r in rows))
    print(collections.Counter(len([e for e in r['events'] if e['kind'] == 'reset_conn' or e['kind'] == 'ui_message']) for r in rows))
    print('v14 differs at', [w['start'] for w in windows if not w['v14_identical']])

if __name__ == '__main__':
    main()
