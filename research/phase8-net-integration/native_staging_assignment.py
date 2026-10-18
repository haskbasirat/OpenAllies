"""Original ChangeToState 0x77A750 with 0x779F00 running, not a copy seam.

This composes previously isolated native session-state and value-assignment
ports. Only event publication is intercepted; its complete value is captured.
"""
from pathlib import Path
import copy, hashlib, json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_launch_assignment import Machine, complete, PE0, PE1, executed_windows
from native_launch import LaunchMachine, SETTINGS_STORE
from native_lobby import setup_common, NODE, CONN
from unicorn.x86_const import UC_X86_REG_ESP

class StateMachine(LaunchMachine, Machine):
    def __init__(self):
        super().__init__()
        self.seams.pop(0x779f00)

    def state_event(self):
        o = self.u.reg_read(UC_X86_REG_ESP) + 4
        p = o + 0xc
        st = {'kind': self.r(p), 'flag_04': self.rb(p + 4) != 0, 'text': self.rs(p + 8),
              'field_0c': self.r(p + 0xc), 'field_10': self.r(p + 0x10),
              'flag_14': self.rb(p + 0x14) != 0, 'flag_15': self.rb(p + 0x15) != 0,
              'flag_16': self.rb(p + 0x16) != 0, 'field_18': self.r(p + 0x18),
              'field_1c': self.r(p + 0x1c), 'flag_20': self.rb(p + 0x20) != 0,
              'field_24': self.r(p + 0x24)}
        self.ev('state_event', text=self.rs(o + 4), state=self.r(o + 8), launch_state=st, flag=self.rb(o + 0x34) != 0)
        self.ret(0x3c)

    def snapshot(self, s):
        result = self.read_session(s)
        result['launch'] = self.read_complete(s + 0x54)['wire']
        result['launch_88'] = self.read_complete(s + 0x88)['wire']
        for name, off in [('field_f8', 0xf8), ('field_100', 0x100)]: result[name] = self.r(s + off)
        result['launch_countdown'] = 0; result['host_countdown'] = 0
        for name, off in [('flag_169', 0x169), ('flag_16a', 0x16a), ('flag_16b', 0x16b), ('flag_16c', 0x16c)]: result[name] = self.rb(s + off) != 0
        return {'session': result, 'retained_launch': self.read_complete(s + 0x54)['retained'],
                'retained_launch_88': self.read_complete(s + 0x88)['retained'],
                'single': self.read_complete(SETTINGS_STORE + 0x400), 'multi': self.read_complete(SETTINGS_STORE + 0x800)}

def main():
    source = json.loads((HERE.parent / 'phase8-net/native-lobby.json').read_text())
    base = next(c for c in source['player_join'] if c['name'] == 'host_join')
    m = StateMachine(); rng = random.Random(0x77a750); rows = []
    for old in (0, 1, 2, 5):
        for new in (1, 2, 4):
            for role in range(4):
                i = len(rows)
                node = copy.deepcopy(base['node'])
                if role == 1: node['host'] = {'Conn': node['conn_address']}
                setup_common(m, base['data'], node)
                m.w(0xb39d04, SETTINGS_STORE)
                m.w(NODE + 0x2e0, 1 if role == 2 else 0)
                session = copy.deepcopy(base['session']); session['players'] = []; session['created'] = None
                session['state'] = old; session['multiplayer'] = bool(i & 1)
                s = m.build_session(session)
                m.w(s + 0x100, 0x12345678); m.w(s + 0xf8, 0x87654321)
                m.b(s + 0x169, role != 3); m.b(s + 0x16c, True)
                for addr, offset in [(s + 0x54, 0), (s + 0x88, 1), (SETTINGS_STORE + 0x400, 2), (SETTINGS_STORE + 0x800, 3)]:
                    value = complete(rng, i + offset, (i + offset) % 3 != 0)
                    m.build_complete(addr, value)
                before = m.snapshot(s)
                m.lobby = []
                m.call(0x77a750, [new, i & 1], this=s, callee_pop=8)
                rows.append({'before': before, 'new_state': new, 'flag': bool(i & 1),
                             'node': {'view': {'own_address': node['own_address'], 'host': node['host']},
                                      'took_over': 1 if role == 2 else 0, 'ordering_player': None, 'local_player': None, 'internet': False},
                             'after': m.snapshot(s), 'events': m.lobby})
    windows = executed_windows(m.visited)
    assert all(w['v14_identical'] for w in windows), 'relevant original/v14 instructions differ'
    result = {'schema': 1, 'scope': __doc__, 'original_sha256': hashlib.sha256(PE0.b).hexdigest(),
              'v14_sha256': hashlib.sha256(PE1.b).hexdigest(), 'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'cases': rows, 'executed_windows': windows}
    (HERE / 'native-staging-assignment.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(len(rows), 'native ChangeToState cases with real settings copies passed', flush=True)

if __name__ == '__main__': main()
