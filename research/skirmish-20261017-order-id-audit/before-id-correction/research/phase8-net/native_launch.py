"""Original launch / session-state processes under Unicorn on native
KKC_Session objects (built as in native_lobby.py):

  TGC_StartLaunchCountdownOrder (96/97)     process 0x75DB90 -> 0x774A50
  TGC_StopLaunchCountdownOrder (101/105)    process 0x75DBA0 -> 0x772350 (-> 0x76CFA0)
  TGC_LaunchCountdownProgressOrder (325/329) process 0x75DBB0 -> 0x7723F0(value)
  TGC_LaunchOrder (0/5)                     process 0x75D8D0 -> 0x77A950
  KKC_Session::ChangeToState 0x77A750(state, flag) (also called by Launch)
  KKC_Session::AutoAssignKingdomToPlayer 0x778D10(player)

with the readiness tests 0x76C960 (all players) / 0x765FC0 (one player), the
ordering-client test 0x74B3C0 + 0x74B900, the migrated-host test 0x74BB40,
the rejection 0x76DA60, the LaunchCountdownProgress request 0x76CBF0 (its
bytes from the request stream), ChangeToState's leave-state step 0x77A6E0,
its event construction (0x7844F0, 0x4145C0, 0x76DCB0, 0x7519C0), the kingdom
record walk of 0x778D10 and SessionPlayer::SetKingdom 0x7664D0, all original.

Seams (recorded): AVar lookup 0x41D430 (LaunchCountdownTime), the session
timer start 0x59AB60 (+0x110), the not-ready reporter 0x7725A0; inside
KKC_Session::Launch the world/session loading steps 0x775DB0, 0x770500,
0x776B20, 0x775EE0 (returns the case's byte), the launch-settings sync dump
0x76D6C0, timeline 0x76DD90 and ResetOrderVersion 0x74DBC0 (game/world
loading, not network logic); inside ChangeToState the event post 0x7851C0
(the posted 0x3C-byte event is recorded) and the launch-settings copy
0x779F00 (KKC_LaunchSettings::operator=, recorded as destination/source);
inside 0x778D10 the kingdom info lookup 0x77E430 (record name -> case
table), the three kingdom setters 0x777F00 / 0x777CD0 / 0x777610 (scenario
world data) and the log-name helper 0x766380. UI text formatting is the
lobby seam (0x40BC50).
"""
from pathlib import Path
import copy, json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_lobby import (LobbyMachine, setup_common, NODE, CONN, FLAGS, U, units, conn, addr)
from native_channels import WRITER_VT, PE0, PE1, sha
from unicorn.x86_const import *

SETTINGS_STORE = 0x6f00000

class LaunchMachine(LobbyMachine):
    def __init__(self):
        super().__init__(True, 'original')
        self.countdown_time = 5; self.launch_byte = 1
        self.u.mem_map(SETTINGS_STORE, 0x1000)
        self.w(SETTINGS_STORE + 0x104, SETTINGS_STORE + 0x400); self.w(SETTINGS_STORE + 0x108, SETTINGS_STORE + 0x800)
        # Descriptor object of TGC_LaunchCountdownProgressOrderRequest (0x766D00 writes its +4 id).
        self.w(0xb40b2c + 4, 325); self.w(0xb40b2c + 8, self.string('TGC_LaunchCountdownProgressOrderRequest'))
        self.seams[0x40bc50] = self.ui_message_args
        self.seams.pop(0x77a750)   # ChangeToState runs natively here
        self.seams.pop(0x778d10)   # AutoAssignKingdomToPlayer runs natively here
        self.seams.update({
            0x41d430: lambda: (self.ev('avar', name=self.text(self.arg(0))), self.ret(4, self.countdown_time)),
            0x59ab60: lambda: (self.ev('start_timer', this_is_110=self.ecx() == self.session_ptr + 0x110), self.ret()),
            0x7725a0: lambda: (self.ev('not_ready'), self.ret()),
            0x775db0: lambda: (self.ev('launch_step', fn='0x775db0', arg=self.arg(0)), self.ret(4)),
            0x770500: lambda: (self.ev('launch_step', fn='0x770500'), self.ret()),
            0x776b20: lambda: (self.ev('launch_step', fn='0x776b20'), self.ret()),
            0x775ee0: lambda: (self.ev('launch_step', fn='0x775ee0'), self.ret(0, self.launch_byte)),
            0x76d6c0: lambda: (self.ev('dump_settings', label=self.text(self.arg(0)), settings=self.settings_name(self.arg(1))), self.ret()),
            0x76dd90: lambda: (self.ev('timeline_freeze', arg=self.arg(0) & 0xff), self.ret(4)),
            0x74dbc0: lambda: (self.ev('reset_order_version', arg=self.arg(0)), self.ret(4)),
            0x7851c0: self.state_event,
            0x779f00: lambda: (self.ev('copy_settings', dst=self.settings_name(self.ecx()), src=self.settings_name(self.arg(0))), self.ret(4)),
            0x77e430: self.kingdom_record_info,
            0x777f00: lambda: (self.ev('set_kingdom', player=self.r(self.arg(0) + 0x20), kingdom=units(self.text(self.arg(1))),
                                       report=self.arg(2) & 0xff), self.ret(0xc, 1)),
            0x777cd0: lambda: (self.ev('set_kingdom_30', record=self.record_index(self.arg(0)), player=self.field_owner(self.arg(1), 0x30)), self.ret(8)),
            0x777610: lambda: (self.ev('set_kingdom_34', record=self.record_index(self.arg(0)), player=self.field_owner(self.arg(1), 0x34)), self.ret(8)),
            0x766380: self.log_name,
        })
    def ui_message_args(self):
        fmt = self.text(self.arg(2))
        args = [self.arg(3)] if fmt.endswith('|d') else []
        self.ev('ui_message', kind=self.arg(1), text=units(fmt), args=args); self.ret()
    def settings_name(self, p):
        S = self.session_ptr
        return {S + 0x54: 'launch', S + 0x88: 'launch_88', SETTINGS_STORE + 0x400: 'store_single',
                SETTINGS_STORE + 0x800: 'store_multi'}.get(p, hex(p))
    def state_event(self):
        o = self.u.reg_read(UC_X86_REG_ESP) + 4
        self.ev('state_event', field_0=self.r(o), text=units(self.text(self.r(o + 4))), state=self.r(o + 8),
                launch_kind=self.r(o + 0xc), launch_field_18=self.r(o + 0xc + 0x18), field_34=self.rb(o + 0x34),
                flag=self.r(o + 0x38))
        self.ret(0x3c)
    def kingdom_record_info(self):
        rec = self.arg(0); name = self.text(self.r(rec))
        self.ev('kingdom_info', kingdom=units(name))
        flag = self.kingdom_open.get(name)
        if flag is None:
            self.ret(4, 0); return
        o = self.alloc(0x10); self.u.mem_write(o + 5, bytes([int(flag)])); self.ret(4, o)
    def record_index(self, p):
        base = self.r(self.r(self.session_ptr + 0x54 + 0x18 + self.map_slot() * 4) + 4 + 0x10)
        return (p - base) // 0x6c if base and (p - base) % 0x6c == 0 else hex(p)
    def map_slot(self):
        mode = self.r(self.session_ptr + 0x58)
        return {0: 2, 2: 1, 5: 4}.get(mode, 0)
    def field_owner(self, p, off):
        q = p - off
        return self.r(q + 0x20) if q > 0 else hex(p)
    def log_name(self):
        ecx = self.ecx(); out = self.arg(0)
        self.w(out, self.string('P' + str(self.r(ecx + 0x20)))); self.ret(8, out)

def player_ptr(m, S, pid):
    n = m.r(S + 0xbc)
    while n:
        p = m.r(n)
        if m.r(p + 0x20) == pid: return p
        n = m.r(n + 4)
    return 0

def player(link, pid, name, status=0, flag_2d=False, ai=False, kingdom=''):
    return {'link': link, 'id': pid, 'name': U(name), 'ai': ai, 'implicit': False, 'film': False, 'field_10': 0,
            'text_14': [], 'status': status, 'flag_1c': False, 'kingdom': U(kingdom), 'field_28': 0, 'flag_2c': False,
            'flag_2d': flag_2d, 'text_30': [], 'text_34': [], 'flag_38': False, 'field_3c': 0, 'field_40': 0}

PEERS = [addr(0x0a000002, 50002), addr(0x0a000003, 50003), addr(0x0a000004, 50004)]
KINGDOMS = ['Allies', 'Axis', 'Japan', 'UK', 'USSR']

def kingdom_record(template, name, rng):
    k = copy.deepcopy(template); k['name'] = U(name)
    k['flag_10'] = rng.random() < 0.15; k['flag_28'] = rng.random() < 0.85
    return k

def make_case(rng, base, kind):
    case = copy.deepcopy(base)
    host_link = 'Node' if rng.random() < 0.7 else conn(base['node']['conn_address'])
    node = case['node']; node['host'] = host_link
    node['peers'] = [{'address': a, 'identity': {'field_3c': rng.choice([0, 1, 2]), 'join_order': k + 2,
                                                  'player_id': {'name': [], 'serial': 0, 'a': 0, 'b': 0, 'c': 0}}}
                     for k, a in enumerate(PEERS)]
    s = case['session']
    L = s['launch']
    template = L['map_default']['header']['kingdoms'][0]
    names = rng.sample(KINGDOMS, rng.randrange(0, 5))
    records = [kingdom_record(template, n, rng) for n in names]
    for mp in ['map_default', 'map_scenario', 'map_random', 'map_24', 'map_metagame']:
        L[mp]['header']['kingdoms'] = copy.deepcopy(records)
    taken = lambda: rng.choice([''] + names) if names and rng.random() < 0.5 else ''
    players = [player('Node', 1, 'Host', status=rng.choice([0, 1, 2]), flag_2d=rng.random() < 0.5, kingdom=taken())]
    for k in range(rng.randrange(0, 4)):
        players.append(player(conn(PEERS[k]), k + 2, f'P{k}', status=rng.choice([0, 1, 2, 3]), flag_2d=rng.random() < 0.6,
                              ai=rng.random() < 0.2, kingdom=taken()))
    s['players'] = players; s['last_player_id'] = len(players)
    L['mode'] = rng.choice([0, 1, 2, 2, 3, 5]); s['multiplayer'] = rng.random() < 0.9
    L['game_type'] = rng.choice([0, 1, 2, 2, 3, 4, 5]); L['field_0c'] = rng.choice([0, 0, 0, 5, 4294967295])
    st = L['state']; st['kind'] = rng.choice([0xc, 0xc, 6, 0, 3]); st['field_10'] = rng.choice([0, 1, 1])
    st['flag_14'] = rng.random() < 0.3; st['field_18'] = rng.choice([0, 4, 7])
    s['state'] = rng.choice([0, 1, 1, 2, 4, 5]); s['field_164'] = rng.choice([0, 3])
    ids = [p['id'] for p in players]
    case['launch'] = {
        'kind': kind,
        'ordering': rng.choice(ids + [None]),
        'local': rng.choice(ids + [None]),
        'migrated': rng.choice([0, 0, 1, 2]),
        'internet': rng.random() < 0.3,
        'countdown_118': rng.choice([-1, 0, 1, 3, 5]),
        'countdown_11c': rng.choice([-1, 0, 0, 2]),
        'value': rng.choice([-1, 0, 1, 2, 3, 5]),
        'countdown_time': rng.choice([5, 3, 0]),
        'launch_byte': rng.choice([0, 1]),
        'flag_169': rng.random() < 0.4,
        'new_state': rng.choice([0, 1, 1, 2, 2, 3, 4, 5]), 'new_flag': rng.choice([0, 1]),
        'assign_player': rng.choice(ids),
        'kingdom_open': {n: rng.choice([None, True, True, False]) for n in names},
    }
    return case

def text_at(m, p): return units(m.text(m.r(p)))

def run_case(m, case):
    L = case['launch']
    data = copy.deepcopy(case['data']); data['kingdom_open'] = L['kingdom_open']
    setup_common(m, data, case['node'])
    S = m.build_session(case['session'])
    m.b(FLAGS + 0xbc, L['internet'])
    m.w(NODE + 0x284, player_ptr(m, S, L['ordering']) if L['ordering'] else 0)
    m.w(NODE + 8, player_ptr(m, S, L['local']) if L['local'] else 0)
    m.w(NODE + 0x2e0, L['migrated'])
    m.w(S + 0x118, L['countdown_118'] & 0xffffffff); m.w(S + 0x11c, L['countdown_11c'] & 0xffffffff)
    m.b(S + 0x169, L['flag_169']); m.b(S + 0x16a, 0); m.b(S + 0x16b, 1); m.b(S + 0x16c, 1)
    m.w(S + 0xf8, 0x55); m.w(S + 0x100, 0x77)
    m.cs(S + 0x84, 'pending text')
    m.w(0xb39d04, SETTINGS_STORE)
    m.countdown_time = L['countdown_time']; m.launch_byte = L['launch_byte']
    m.req = m.stream(WRITER_VT, size=0x100)
    m.lobby = []
    status = 'ok'; result = None
    try:
        if L['kind'] == 'start': m.call(0x774a50, [], this=S, callee_pop=0)
        elif L['kind'] == 'stop': m.call(0x772350, [], this=S, callee_pop=0)
        elif L['kind'] == 'progress': m.call(0x7723f0, [L['value'] & 0xffffffff], this=S, callee_pop=4)
        elif L['kind'] == 'launch': m.call(0x77a950, [], this=S, callee_pop=0)
        elif L['kind'] == 'change_state': m.call(0x77a750, [L['new_state'], L['new_flag']], this=S, callee_pop=8)
        elif L['kind'] == 'auto_assign':
            result = m.call(0x778d10, [player_ptr(m, S, L['assign_player'])], this=S, callee_pop=4) & 0xff
        elif L['kind'] == 'all_ready': result = m.call(0x76c960, [], this=S, callee_pop=0) & 0xff
        elif L['kind'] == 'ready':
            result = [m.call(0x765fc0, [], this=player_ptr(m, S, p['id']), callee_pop=0) & 0xff for p in case['session']['players']]
    except Exception as e:
        status = 'error: ' + type(e).__name__ + ' ' + str(e)[:120]
    end = m.position(m.req); bits = end['byte'] * 8 + end['bit']
    players = []
    n = m.r(S + 0xbc)
    while n:
        p = m.r(n); players.append({'id': m.r(p + 0x20), 'kingdom': text_at(m, p + 0x24), 'flag_2c': m.rb(p + 0x2c)}); n = m.r(n + 4)
    return {'status': status, 'result': result, 'events': m.lobby,
            'request': {'bytes': list(m.u.mem_read(m.r(m.req + 4), (bits + 7) // 8)), 'end': end},
            'session': {'countdown_118': struct.unpack('<i', m.u.mem_read(S + 0x118, 4))[0],
                        'countdown_11c': struct.unpack('<i', m.u.mem_read(S + 0x11c, 4))[0],
                        'field_164': m.r(S + 0x164), 'flag_169': m.rb(S + 0x169), 'flag_16a': m.rb(S + 0x16a),
                        'flag_16b': m.rb(S + 0x16b), 'flag_16c': m.rb(S + 0x16c), 'state': m.r(S + 0xe4),
                        'game_type': m.r(S + 0x54), 'field_f8': m.r(S + 0xf8), 'field_100': m.r(S + 0x100),
                        'changed': m.rb(S + 0xc8), 'changed_c9': m.rb(S + 0xc9), 'text_84': text_at(m, S + 0x84),
                        'players': players}}

def main():
    base_all = json.loads((HERE / 'native-lobby.json').read_text())
    base = [c for c in base_all['player_join'] if c['name'] == 'host_join'][0]
    base = {'data': base['data'], 'node': base['node'], 'session': base['session']}
    m = LaunchMachine(); rng = random.Random(9697); rows = []
    for kind, n in [('ready', 60), ('all_ready', 60), ('start', 120), ('stop', 60), ('progress', 80), ('launch', 80),
                    ('change_state', 120), ('auto_assign', 120)]:
        for k in range(n):
            case = make_case(rng, base, kind)
            r = run_case(m, case)
            rows.append({'case': {'node': case['node'], 'session': case['session'], 'launch': case['launch']}, 'result': r})
        print(kind, n, flush=True)
    ranges = []
    for a, size in sorted(m.visited.items()):
        if not PE0.base <= a < PE0.base + PE0.image_size: continue
        if ranges and ranges[-1][1] >= a: ranges[-1][1] = max(ranges[-1][1], a + size)
        else: ranges.append([a, a + size])
    windows = [{'start': hex(a), 'end': hex(b), 'sha256': sha(PE0.range(a, b - a)),
                'v14_identical': PE0.range(a, b - a) == PE1.range(a, b - a)} for a, b in ranges]
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
              'producer_sha256': sha(Path(__file__).read_bytes()), 'data': base['data'], 'cases': rows, 'executed_windows': windows}
    (HERE / 'native-launch.json').write_text(json.dumps(report) + '\n')
    import collections
    print(collections.Counter((r['case']['launch']['kind'], r['result']['status'][:40]) for r in rows))
    print('v14 differs at', [w['start'] for w in windows if not w['v14_identical']])

if __name__ == '__main__':
    main()
