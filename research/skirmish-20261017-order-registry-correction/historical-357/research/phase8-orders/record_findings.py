"""Write findings.json for research/phase8-orders and research/phase8-net:
hash pins of the original images, every producer script, fixture and Rust
source involved, fixture case counts, and the proof scope."""
from pathlib import Path
import hashlib, json
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[1]; GAME = ROOT.parent
NET = ROOT / 'research/phase8-net'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
def pin(p): return {'path': Path(p).relative_to(ROOT).as_posix() if str(p).startswith(str(ROOT)) else str(p), 'sha256': sha(p)}

def main():
    traces = json.loads((HERE / 'order-traces.json').read_text())
    flow = json.loads((HERE / 'order-flow.json').read_text())
    transport = json.loads((NET / 'native-transport.json').read_text())
    sync = json.loads((NET / 'native-sync.json').read_text())
    runs = [r for d in traces['descriptors'] for r in d['runs']]
    cases = [c for d in flow['descriptors'] for c in d['cases']]
    src = sorted((ROOT / 'crates/aa-orders/src').rglob('*.rs')) + sorted((ROOT / 'crates/aa-orders/tests').glob('*.rs'))
    common = {'original_image': {'path': str(GAME / 'AA.exe'), 'sha256': traces['original_sha256']},
              'v14_image': {'sha256': traces['v14_sha256']},
              'rust_sources': [pin(p) for p in src]}
    orders = {'schema': 1, **common,
              'producers': [pin(HERE / f) for f in ['native_order_traces.py', 'native_order_flow.py', 'static_calls.py', 'tokens.py',
                                                    'gen_schemas.py', 'gen_flow_table.py', 'gen_inventory.py', 'natdis.py',
                                                    'native_actor_command.py']],
              'fixtures': [pin(HERE / f) for f in ['order-traces.json', 'order-flow.json', 'static-handler-calls.json',
                                                   'descriptor-write-sites.json', 'inventory.json', 'native-actor-command.json']],
              'counts': {'descriptors': len(traces['descriptors']), 'trace_runs': len(runs),
                         'trace_runs_decoded': sum(r['status'] == 'stopped_before_process' for r in runs),
                         'trace_runs_native_fault': sum(r['status'] == 'native_null_dereference' for r in runs),
                         'flow_cases': len(cases), 'flow_cases_returned': sum(c['status'] == 'returned' for c in cases),
                         'flow_cases_native_fault': sum(c['status'] == 'native_null_dereference' for c in cases),
                         'actor_command_cases': len(json.loads((HERE / 'native-actor-command.json').read_text())['cases'])},
              'proven': ['all 357 payload encodings (reads, end position, byte re-encode)',
                         'request->decree re-encoding (writes and bytes) except trigger regrouping 293/296 (refused)',
                         'ProcessRequest/decree control flow, sync-log entries, mirror, rejections for all 357 handlers',
                         'KKC_GActorCommand::Process 0x4D9B60 actor command delivery walk (acyclic graphs; a cycle loops forever natively, typed error here)'],
              'not_proven': ['order process functions (gameplay/lobby effects) other than phase4/5 ports',
                             'validity rules forced true in the oracle (0x5092F0) and cheat permission rule',
                             'branches requiring game data beyond the declared synthetic environment'],
              'tests': ['crates/aa-orders/tests/order_traces.rs', 'crates/aa-orders/tests/order_flow.rs',
                        'crates/aa-orders/tests/actor_command.rs']}
    load = lambda f: json.loads((NET / f).read_text())
    channels, packets, misc, connect = load('native-channels.json'), load('native-packets.json'), load('native-packets-misc.json'), load('native-connect.json')
    gate, speed, files, heart = load('native-send-gate.json'), load('native-speed.json'), load('native-file-transfer.json'), load('native-heartbeat.json')
    lobby = load('native-lobby.json')
    resume, selector, launch, leave = load('native-join-resume.json'), load('native-send-selector.json'), load('native-launch.json'), load('native-leave.json')
    netsrc = sorted((ROOT / 'crates/aa-netcode/src').rglob('*.rs')) + sorted((ROOT / 'crates/aa-netcode/tests').glob('*.rs'))
    net = {'schema': 1, **common,
           'rust_sources_netcode': [pin(p) for p in netsrc],
           'producers': [pin(NET / f) for f in ['native_transport.py', 'native_sync.py', 'native_channels.py', 'native_packets.py',
                                                'native_packets_misc.py', 'native_connect.py', 'native_send_gate.py', 'native_speed.py',
                                                'native_file_transfer.py', 'native_heartbeat.py', 'native_lobby.py', 'null_guard.py',
                                                'native_join_resume.py', 'native_send_selector.py', 'native_launch.py', 'native_leave.py']],
           'fixtures': [pin(NET / f) for f in ['native-transport.json', 'native-sync.json', 'native-channels.json', 'native-packets.json',
                                               'native-packets-misc.json', 'native-connect.json', 'native-send-gate.json', 'native-speed.json',
                                               'native-file-transfer.json', 'native-heartbeat.json', 'native-lobby.json',
                                               'native-join-resume.json', 'native-send-selector.json', 'native-launch.json', 'native-leave.json']],
           'counts': {'transport_cases': len(transport['cases']),
                      'transport_reads': sum(c['kind'] == 'read' for c in transport['cases']),
                      'sync_sequences': len(sync['sequences']), 'sync_operations': sum(len(s) for s in sync['sequences']),
                      'channel_steps': sum(len(s['steps']) for s in channels['scenarios']),
                      'packet_steps': {k: sum(len(s['steps']) for s in v) for k, v in packets['images'].items()},
                      'misc_packet_cases': {k: len(misc[k]) for k in ['reconnect', 'client_to_backup', 'backup_to_client', 'empty_bodies', 'disconnect_decree']},
                      'connect_cases': len(connect['cases']),
                      'send_gate_cases': {k: len(v) for k, v in gate['images'].items()},
                      'speed_cases': len(speed['cases']),
                      'lobby_cases': {k: len(lobby[k]) for k in ['join_messages', 'player_join', 'addresses']},
                      'join_resume_cases': len(resume['cases']),
                      'send_selector_cases': len(selector['cases']),
                      'launch_cases': len(launch['cases']),
                      'client_leave_cases': len(leave['cases']),
                      'client_leave_native_faults': sum(c['status'] != 'ok' for c in leave['cases'])},
           'proven': ['datagram header writer/reader 0x764DA0/0x764CA0/0x764BB0/0x764600/0x764550/0x764AF0 at PC24 and PC53',
                      'TGC_Synchronizer 0x77CCB0/0x77DA70/0x77D930/0x77D9D0/0x77DB10/0x77D900/0x77D050',
                      'message-history channels over TGC_BlockSender/Receiver',
                      'admin packet types 2..11 bodies and reader gates incl. bindings 0x752300/0x7523A0/0x752490/0x7525B0, ack dispatcher 0x74B4F0, '
                      'status 0x74C6D0/0x74C7B0, file generation 0x74C8B0, backlog 0x74BCB0 + v14 caves 0x9EFF40/0x9EFEC0',
                      'Connect 0x750330/0x75BF90 and PlayerJoin request payload 0x77B680',
                      'send gate 0x74BDE0 with 0x74B600/0x74BD80 at PC24/PC53, both images',
                      'SetGameSpeed decree process 0x75D810 bound to Timeline::set_game_speed',
                      'file transfer (see file-transfer-notes.txt)', 'heartbeat/update/lag/migration (see heartbeat-notes.txt)',
                      'lobby join path: private messages, session snapshot, host completion, PlayerJoin process (see lobby-notes.txt)',
                      'synchronizer resume: AdminNode+0x200/+0x204 = decree stream position after the PlayerJoin decree; host 0x74D710 + client 0x74FD10 '
                      'replay bit-exact, resumed client aligned, packet-start client not (native-join-resume.json)',
                      'send selector 0x763EE0',
                      'staging: readiness 0x765FC0/0x76C960, countdown 0x774A50/0x772350/0x76CFA0/0x7723F0 (+0x76DA60, 0x76CBF0), Launch 0x77A950, '
                      'ChangeToState 0x77A750/0x77A6E0, AutoAssignKingdomToPlayer 0x778D10/0x7664D0',
                      'ClientLeave 0x755840 with 0x74F160/0x74D380/0x74D320/0x74EF90/0x751F70/0x74DA50/0x74DAA0/0x4F1430 and the 0x7737E0 player loop'],
           'composition_only': ['net::admin and net::seams (aa-orders tests/net_lockstep.rs)',
                                'NetworkSeams adapter (aa-netcode src/seams.rs)',
                                'heartbeat channel effects onto packets.rs (tests/channel_effects.rs)',
                                'LOOPBACK ONLY: host+client threads over 127.0.0.1 UDP, direct and through a lossy/reordering relay, '
                                'join through lobby.rs (PlayerJoin process, snapshot, admin-state resume) after the host has ticked, '
                                'decrees replayed identically and every client synchronizer value validated by the host (tests/loopback.rs)'],
           'documented_not_ported': ['PlayerJoin duplicate-name cases without native evidence (3)',
                                     'KKC_LaunchSettings::operator= 0x779F00, kingdom setters 0x777F00/0x777CD0/0x777610, not-ready report 0x7725A0, '
                                     'session task handlers 0x775BF0/0x773AE0/0x76EE10 (traits/effects)',
                                     'NetNode bandwidth buckets (intentionally not ported)', 'SimulatedNetEffects',
                                     'GameSpy / NAT negotiation (out of scope)'],
           'removed_caps_in_code': ['sent-time and received-flag windows', 'order/decree histories', 'synchronizer ring', 'bitstreams',
                                    '64-socket select 0x783D8F', '32 receives per update 0x783DC7', '4096-byte receive buffer 0x783BE5 -> 65535',
                                    '4096-byte send buffers 0x76526D/0x765133', 'bandwidth token buckets 0x764FA0/0x763E30 (not ported)',
                                    'channel/file/history initial capacities 0x100/0x400'],
           'kept_rules': ['per-pass send bursts 8/9 (0x74BED7/0x74BF16)', 'packet bit targets 0x403/0x603/0x803/0xC03 (v14 file 0x2403), 0x400',
                          'file BlocksPerFrame 20 / BlockSize 64 / MaxPendingBlocks 256', 'reorder window 4, ack bits 32, retention 200/1000/100/32'],
           'emulator_caveats_rechecked': {'null_guard': 'every phase8-net/phase8-orders harness',
                                          'trace_runs_masked_by_zero_page': 96, 'flow_cases_masked_by_zero_page': 4,
                                          'results_changed': 0, 'native_faults_confirmed_genuine': '17 trace + 6 flow'},
           'tests': ['crates/aa-orders/tests/net_transport.rs', 'crates/aa-orders/tests/net_sync.rs', 'crates/aa-orders/tests/net_lockstep.rs'] +
                    [Path(p).relative_to(ROOT).as_posix() for p in sorted((ROOT / 'crates/aa-netcode/tests').glob('*.rs'))],
           'untested': ['real network sessions between machines']}
    (HERE / 'findings.json').write_text(json.dumps(orders, indent=1) + '\n')
    (NET / 'findings.json').write_text(json.dumps(net, indent=1) + '\n')
    print(orders['counts'], net['counts'])

if __name__ == '__main__':
    main()
