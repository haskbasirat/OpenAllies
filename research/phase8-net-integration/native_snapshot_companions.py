"""Original full snapshot read with real staging/copy and retained companions.

0x76D0B0 writes and 0x77AAC0 reads the session, including 0x77A750,
0x779F00, 0x76EBD0, 0x5104A0, its temporary constructor 0x50F4C0 and
0x4A4C00. No settings-copy/header/kingdom/decoder seam. Standard inherited
allocator/CString/logging/definition lookup/network status/UI event seams.
"""
from pathlib import Path
import copy, hashlib, json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_launch_assignment import Machine, complete, HEADERS, PE0, PE1, executed_windows
from native_staging_assignment import StateMachine
from native_launch import SETTINGS_STORE
from native_lobby import (setup_common, host_node, client_node, session, player, ident, U,
                          client_admin, NODE, CONN, base_data)
from native_channels import WRITER_VT, READER_VT

class SnapshotMachine(Machine):
    def __init__(self, image):
        super().__init__(False, image)
        self.u.mem_map(SETTINGS_STORE, 0x1000)
        self.w(SETTINGS_STORE + 0x104, SETTINGS_STORE + 0x400)
        self.w(SETTINGS_STORE + 0x108, SETTINGS_STORE + 0x800)
        self.w(0xb39d04, SETTINGS_STORE)
        self.seams.pop(0x77a750)
        self.seams[0x7851c0] = self.state_event
    state_event = StateMachine.state_event
    snapshot = StateMachine.snapshot

def make_input(rng, index):
    stale = complete(rng, index + 1, True)
    incoming = copy.deepcopy(stale)
    # Equal-sized replacement/reordering is the important regression: stale
    # companion values cannot be detected by checking vector lengths.
    for name in HEADERS:
        ks = incoming['wire'][name]['header']['kingdoms']
        if index % 3 == 0:
            ks.reverse()
            for k in ks: k['name'] = U('replacement/' + name) + k['name']
        elif index % 3 == 1:
            ks.clear()
        else:
            source = copy.deepcopy(complete(rng, 5, True)['wire']['game']['header']['kingdoms'])
            ks.extend(source[:2])
        incoming['retained']['kingdoms'][name] = [
            {'text_0c': U(f'host-private/{index}/{name}/{i}'), 'flag_68': True} for i in range(len(ks))]
    return stale, incoming

def main():
    images = []; combined_windows = {}; expected_constructor = None
    for image in ('original', 'v14'):
        m = SnapshotMachine(image); rng = random.Random(0x77aac0); rows = []
        data = base_data(scenarios={'scenario': 'scenario'}, random_maps={'rmc': 'rmc'})
        setup_common(m, data, client_node())
        temp = m.alloc(0x6c); m.u.mem_write(temp, b'\xcd' * 0x6c)
        m.call(0x50f4c0, [1], this=temp, callee_pop=4)
        constructor = {'text_0c': m.rs(temp + 0xc), 'flag_68': m.rb(temp + 0x68) != 0}
        assert constructor == {'text_0c': [], 'flag_68': False}, constructor
        if expected_constructor is not None: assert constructor == expected_constructor
        expected_constructor = constructor
        for index in range(12):
            stale, incoming = make_input(rng, index)
            old88 = complete(rng, index + 2, True)
            if index % 3 == 0:
                old88 = copy.deepcopy(stale)  # restore also retains the same count before replacement
            # Host snapshot: one joined human, with actual native codec fields.
            hn = host_node()
            setup_common(m, data, hn)
            outgoing = session(players=[player('Node', 1, 'Host')], created=1, state=1,
                               last_player_id=1, launch=incoming['wire'], launch_88=incoming['wire'])
            host = m.build_session(outgoing)
            m.build_complete(host + 0x54, incoming); m.build_complete(host + 0x88, incoming)
            writer = m.stream(WRITER_VT, size=0x40000)
            m.call(0x76d0b0, [writer], this=host, callee_pop=4)
            end = m.position(writer); count = (end['byte'] * 8 + end['bit'] + 7) // 8
            wire = list(m.u.mem_read(m.r(writer + 4), count))
            # Client can enter from staging (no restore) or another state
            # (Launch88 restored before the incoming values are decoded).
            cn = client_node()
            setup_common(m, data, cn)
            initial = session(state=(0, 1, 2)[(index // 3) % 3], launch=stale['wire'], launch_88=old88['wire'])
            target = m.build_session(initial)
            m.build_complete(target + 0x54, stale); m.build_complete(target + 0x88, old88)
            for address in (SETTINGS_STORE + 0x400, SETTINGS_STORE + 0x800):
                m.build_complete(address, complete(rng, index + 3, True))
            m.w(target + 0x100, 0x12345678); m.w(target + 0xf8, 0x87654321)
            m.b(target + 0x16c, True)
            before = m.snapshot(target)
            admin_before = client_admin(m)
            reader = m.stream(READER_VT, data=wire)
            m.lobby = []
            m.call(0x77aac0, [reader], this=target, callee_pop=4)
            after = m.snapshot(target)
            assert m.position(reader) == end
            # Every decoded kingdom came through the constructor temporary,
            # including same-count/reordered names and previously nonempty data.
            for slot in ('launch', 'launch_88'):
                retained = after['retained_' + slot]
                for name in HEADERS:
                    count = len(after['session'][slot][name]['header']['kingdoms'])
                    assert retained['kingdoms'][name] == [constructor] * count, (image, index, slot, name)
            rows.append({'index': index, 'data': data, 'before': before, 'admin_before': admin_before,
                         'node': {'host': cn['host']}, 'wire': wire, 'end': end,
                         'after': after, 'admin_after': client_admin(m), 'events': m.lobby})
            print(image, 'snapshot', index, 'passed', flush=True)
        if images: assert images[0] == rows, 'original/v14 snapshot differences'
        images.append(rows); combined_windows.update(m.visited)
    result = {'schema': 1, 'scope': __doc__, 'constructor': expected_constructor,
              'original_sha256': hashlib.sha256(PE0.b).hexdigest(), 'v14_sha256': hashlib.sha256(PE1.b).hexdigest(),
              'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'cases': images[0], 'executed_windows': executed_windows(combined_windows)}
    (HERE / 'native-snapshot-companions.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print('24 full snapshots and two constructor probes agree', flush=True)

if __name__ == '__main__': main()
