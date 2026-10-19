"""Original custom-game staging processes on a laboratory local-host Session:

TGC_CreateExplicitAIOrder process 0x75E030 -> AdminNode 0x754F50 (countdown
0x76ADE0, ordering player Admin+0x284 -> 0x8569A0 link, ++Session+E8,
CreatePlayer 0x773710 / SessionPlayer ctor 0x7661D0 / rated 0x770280, text
0x7664B0, name 0x7664F0 or unique name 0x766560, then
KKC_Session::SetPlayerKingdom 0x777F00(player, kingdom, 1) with 0x50DC00,
0x7664D0 and 0x77E1F0), TGC_SetKingdomNationOrder process 0x75E1C0 ->
0x777D90 (0x76C560 permission, 0x777CD0, 0x777610, 0x76B920),
TGC_SetKingdomFactionOrder process 0x75E1F0 -> 0x777700 and
TGC_SetAIHandicapOrder process 0x75E340 -> 0x771880.

Every listed function executes unchanged on the original and on the v14 image
(phase8-net LobbyMachine: Session/launch-settings/SessionPlayer layouts,
synthetic definition registries with +8 names). Seams (recorded events):
allocator/CString manager/logging of the harness chain, the text table
0x4B9F40 (fixture formats), the UI message 0x40BC50, and the definition
registry Find 0x48D500 (canonical transport: the registry entry whose +8 name
equals the key, NULL for none or an empty key) and CString::Format 0x405D60
(the proven VC7.1 %s/%d domain of the duplicate-name text).
"""
from pathlib import Path
import sys, json, hashlib, copy
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-net'))
import native_lobby as L
from native_lobby import (LobbyMachine, U, kingdom, header, mapset, launch, game, session, player, base_data, host_node,
                          setup_common, REGISTRIES, NODE, FLAGS)
from native_channels import PE0, PE1, sha

REGS = {'NATIONS': 0xb3aa44, 'FACTIONS': 0xb3aa4c, 'AI_HANDICAPS': 0xb3aa68}
NAMES = {'NATIONS': ['usa', 'germany', 'britain', 'russia'], 'FACTIONS': ['infantry', 'armor', 'airborne', 'artillery'],
         'AI_HANDICAPS': ['easy', 'normal', 'hard', 'expert']}


class Machine(LobbyMachine):
    def build_seams(self):
        super().build_seams()
        self.seams[0x48d500] = self.find_definition
        self.seams[0x405d60] = self.format_text
    def format_text(self):
        # CString::Format 0x405D60(&out, format, ...) (cdecl), as the VC7.1
        # _vswprintf domain the shipped duplicate-name text uses (%s then %d,
        # literal %%; research/skirmish-20261016-local-bootstrap/implicit-ai
        # native-name-format.json proves it over the real CRT).
        out, fmt = self.arg(0), self.text(self.arg(1))
        res, i, a = '', 0, 2
        while i < len(fmt):
            ch = fmt[i]
            if ch == '%' and i + 1 < len(fmt):
                t = fmt[i + 1]
                if t == '%': res += '%'
                elif t == 's': res += self.text(self.arg(a)); a += 1
                elif t == 'd':
                    v = self.arg(a); a += 1
                    res += str(v - (1 << 32) if v & 0x80000000 else v)
                else: raise AssertionError('format conversion ' + t)
                i += 2; continue
            res += ch; i += 1
        self.ev('format', text=res)
        self.w(out, self.string(res)); self.ret(0)
    def find_definition(self):
        # 0x48D500(key CString by value) on a registry manager (ecx): an empty
        # key returns NULL (0x48D51A); otherwise the entry with that +8 name.
        key = self.text(self.arg(0))
        # the definition managers of the game data [B36530]: +0x5C Nation,
        # +0xA8 Faction (0x777D49, 0x7776A6)
        offset = self.ecx() - self.r(0xb36530)
        reg = {0x5c: 'NATIONS', 0xa8: 'FACTIONS'}.get(offset)
        if reg is None: raise AssertionError('48D500 on game data +%#x' % offset)
        mgr = self.r(REGISTRIES[reg])
        if key == '':
            self.ev('find', registry=reg, key=''); self.ret(4, 0); return
        arr, n = self.r(mgr + 0x24), self.r(mgr + 0x28)
        found = 0
        for i in range(n):
            d = self.r(arr + 4 * i)
            if d and self.text(self.r(d + 8)) == key:
                found = d; break
        self.ev('find', registry=reg, key=key, index=None if not found else self.defs[found])
        self.ret(4, found)


    def setup_data(self, data):
        super().setup_data(data)
        # Nation +0x48/+0x4C faction pointer vector (0x777610 membership test):
        # the synthetic definitions are reallocated with room for it.
        for name, reg in REGISTRIES.items():
            mgr = self.r(reg); arr, n = self.r(mgr + 0x24), self.r(mgr + 0x28)
            for i in range(n):
                old = self.r(arr + 4 * i)
                if not old: continue
                d = self.alloc(0x80); self.u.mem_write(d, bytes(self.u.mem_read(old, 0x40)))
                self.w(arr + 4 * i, d); del self.defs[old]; self.defs[d] = i
        for nation, factions in data.get('nation_factions', {}).items():
            d = self.def_ptr('NATIONS', int(nation))
            v = self.alloc(4 * max(len(factions), 1))
            for k, f in enumerate(factions): self.w(v + 4 * k, self.def_ptr('FACTIONS', f))
            self.w(d + 0x48, v); self.w(d + 0x4c, len(factions))


def data():
    d = base_data(kingdom_open={})
    for reg, names in NAMES.items():
        d['registries'][reg] = {'count': len(names), 'names': {str(i): n for i, n in enumerate(names)}}
    d['nation_factions'] = {'0': [0, 1], '1': [1, 2, 3], '2': [], '3': [3]}
    d['formats'].update({'session_order_rejected_currently_counting_down': 'counting down',
                         'session_order_failed': 'order failed', 'staging_player_not_allowed_kingdom': 'not allowed'})
    return d


def berlin(**over):
    # (2v2)Berlin's Creator rows (research/skirmish-20261017-launch-settings): four
    # playable, not required majors on two teams and two independents.
    ks = [kingdom('kingdom_0', text_14=U('team_1')), kingdom('kingdom_1', text_14=U('team_1')),
          kingdom('independent_2', flag_10=True, flag_29=True), kingdom('kingdom_3', text_14=U('team_0')),
          kingdom('kingdom_4', text_14=U('team_0')), kingdom('independent_5', flag_10=True, flag_29=True)]
    for name, kw in over.items():
        for k in ks:
            if k['name'] == U(name): k.update(kw)
    return ks


def case(name, ops, players=None, base=None, current=None, countdown=0, ordering=1, local=1, last=None):
    players = players if players is not None else [player('Node', 1, 'Player', kingdom=U('kingdom_0'))]
    g = game(header=header([], base if base is not None else berlin()))
    lch = launch(game_type=2, mode=2, game=g, map_scenario=mapset(header=header([], current if current is not None else berlin())))
    s = session(players=players, created=players[0]['id'] if players else None, last_player_id=last if last is not None else
                max([p['id'] for p in players] + [0]), launch=lch, state=0, multiplayer=False)
    return {'name': name, 'session': s, 'ops': ops, 'countdown': countdown, 'ordering': ordering, 'local': local}


def cases():
    out = []
    out.append(case('random_ai', [('ai', 'kingdom_1', '', 'Computer'), ('nation', 'kingdom_1', None), ('faction', 'kingdom_1', None)]))
    out.append(case('three_random_ais', [op for k in ('kingdom_1', 'kingdom_3', 'kingdom_4') for op in
                                         (('ai', k, '', 'Computer'), ('nation', k, None), ('faction', k, None))]))
    out.append(case('sai_ai_nation_faction', [('ai', 'kingdom_3', 'sai_patton', 'Patton'), ('nation', 'kingdom_3', 0),
                                              ('faction', 'kingdom_3', 1), ('handicap', 'kingdom_3', 2)]))
    out.append(case('sai_duplicate_name', [('ai', 'kingdom_1', 'sai_a', 'Player'), ('ai', 'kingdom_3', 'sai_a', 'Player')]))
    out.append(case('random_duplicate_name_kept', [('ai', 'kingdom_1', '', 'Player'), ('ai', 'kingdom_3', '', 'Player')]))
    out.append(case('countdown_rejects', [('ai', 'kingdom_1', '', 'Computer'), ('nation', 'kingdom_1', 1), ('handicap', 'kingdom_1', 0),
                                          ('faction', 'kingdom_1', 0)], countdown=3))
    out.append(case('no_ordering_player', [('ai', 'kingdom_1', '', 'Computer')], ordering=None))
    out.append(case('not_playable_kingdom', [('ai', 'kingdom_1', '', 'Computer')], current=berlin(kingdom_1={'flag_28': False})))
    out.append(case('unknown_kingdom', [('ai', 'nowhere', '', 'Computer'), ('handicap', 'kingdom_1', 1)]))
    out.append(case('occupied_kingdom', [('ai', 'kingdom_0', '', 'Computer')]))
    out.append(case('handicap_values', [('ai', 'kingdom_4', '', 'Computer'), ('handicap', 'kingdom_4', 3), ('handicap', 'kingdom_4', None),
                                        ('handicap', 'kingdom_4', 0)]))
    out.append(case('nation_denied_without_player', [('nation', 'kingdom_1', 1), ('faction', 'kingdom_1', 1),
                                                     ('handicap', 'kingdom_1', 1)]))
    out.append(case('fixed_base_nation', [('ai', 'kingdom_1', '', 'Computer'), ('nation', 'kingdom_1', 2), ('faction', 'kingdom_1', 0)],
                    base=berlin(kingdom_1={'nation': 1}), current=berlin(kingdom_1={'nation': 1})))
    out.append(case('fixed_base_faction', [('ai', 'kingdom_3', '', 'Computer'), ('nation', 'kingdom_3', 0), ('faction', 'kingdom_3', 2)],
                    base=berlin(kingdom_3={'faction': 3}), current=berlin(kingdom_3={'faction': 3})))
    out.append(case('existing_handicap_cleared_by_new_ai', [('ai', 'kingdom_1', '', 'Computer')],
                    current=berlin(kingdom_1={'ai_handicap': 2})))
    out.append(case('ordering_not_local', [('ai', 'kingdom_1', '', 'Computer'), ('handicap', 'kingdom_1', 1), ('nation', 'kingdom_1', 1)],
                    local=None))
    out.append(case('faction_not_in_nation', [('ai', 'kingdom_1', 'sai_b', 'Rommel'), ('nation', 'kingdom_1', 0), ('faction', 'kingdom_1', 3),
                                              ('faction', 'kingdom_1', 0)]))
    out.append(case('nation_change_revalidates_faction', [('ai', 'kingdom_3', '', 'Computer'), ('nation', 'kingdom_3', 1), ('faction', 'kingdom_3', 2),
                                                          ('nation', 'kingdom_3', 0), ('nation', 'kingdom_3', 3), ('faction', 'kingdom_3', 3),
                                                          ('nation', 'kingdom_3', None)]))
    out.append(case('human_kingdom_settings', [('nation', 'kingdom_0', 1), ('faction', 'kingdom_0', 1), ('handicap', 'kingdom_0', 2)]))
    return out


def text_slot(m, s):
    p = m.alloc(4); m.cs(p, s); return p


def player_ptr(m, S, pid):
    n = m.r(S + 0xbc)
    while n:
        p = m.r(n)
        if m.r(p + 0x20) == pid: return p
        n = m.r(n + 4)
    return 0


def run(m, c):
    setup_common(m, data(), host_node())
    S = m.build_session(c['session'])
    m.w(S + 0x118, c['countdown'] & 0xffffffff)
    m.w(NODE + 0x284, player_ptr(m, S, c['ordering']) if c['ordering'] is not None else 0)
    m.w(NODE + 8, player_ptr(m, S, c['local']) if c['local'] is not None else 0)
    m.lobby = []
    steps = []
    for op in c['ops']:
        m.lobby = []
        status = 'ok'
        try:
            if op[0] == 'ai':
                _, k, prof, name = op
                ks, ps = text_slot(m, U(k)), text_slot(m, U(prof))
                nm = m.string(name) if isinstance(name, str) else m.ustring(name)
                m.call(0x754f50, [ks, ps, nm], this=NODE, callee_pop=12)
            else:
                kind, k, idx = op
                reg = {'nation': 'NATIONS', 'faction': 'FACTIONS', 'handicap': 'AI_HANDICAPS'}[kind]
                fn = {'nation': 0x777d90, 'faction': 0x777700, 'handicap': 0x771880}[kind]
                m.call(fn, [m.ustring(U(k)), m.def_ptr(reg, idx)], this=S, callee_pop=8)
        except Exception as e:
            status = 'error: ' + type(e).__name__ + ' ' + str(e)[:160]
        s = m.read_session(S)
        # +0x68 modified bytes of the current (scenario) Creator records
        hdr = m.r(S + 0x54 + 0x1c) + 4; vec, n = m.r(hdr + 0x10), m.r(hdr + 0x14)
        modified = [m.rb(vec + 0x6c * i + 0x68) != 0 for i in range(n)]
        steps.append({'op': list(op), 'status': status, 'events': m.lobby, 'players': s['players'], 'modified_68': modified,
                      'kingdoms': s['launch']['map_scenario']['header']['kingdoms'], 'last_player_id': s['last_player_id'],
                      'created': s['created'], 'changed': s['changed']})
    return steps


def main():
    rows = {}
    visited = {}
    for image in ('original', 'v14'):
        m = Machine(True, image)
        rows[image] = []
        for c in cases():
            rows[image].append(run(m, c)); print(image, c['name'], flush=True)
        visited.update(m.visited)
    assert rows['original'] == rows['v14'], [c['name'] for c, a, b in zip(cases(), rows['original'], rows['v14']) if a != b]
    windows = L.executed_windows(visited)
    assert all(w['v14_identical'] for w in windows), [w['start'] for w in windows if not w['v14_identical']]
    out = [dict(c, steps=r) for c, r in zip(cases(), rows['original'])]
    doc = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
           'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), 'cases': out,
           'executed_windows': windows}
    path = HERE / 'native-explicit-ai.json'
    data_ = (json.dumps(doc, indent=1) + '\n').encode()
    if '--write' in sys.argv: path.write_bytes(data_)
    else: assert path.read_bytes() == data_
    for c in out:
        print(c['name'], [s['status'] for s in c['steps']])
    print(json.dumps({'cases': len(out), 'steps': sum(len(c['steps']) for c in out), 'windows': len(windows),
                      'sha256': hashlib.sha256(data_).hexdigest()}))


if __name__ == '__main__':
    main()
