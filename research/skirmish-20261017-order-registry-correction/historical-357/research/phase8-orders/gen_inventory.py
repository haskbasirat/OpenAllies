"""Build research/phase8-orders/inventory.json: every native order descriptor
and every player command type, with native encoding, handler/process
addresses, Rust port and proof status.

Inputs (all produced in this directory or earlier phases):
  order-traces.json        original reader/writer traces (native_order_traces.py)
  order-flow.json          original ProcessRequest/decree flow (native_order_flow.py)
  static-handler-calls.json, descriptor-write-sites.json (static capstone scans)
  crates/aa-orders/src/payload/*.rs, src/flow_table.rs (Rust ports)
"""
from pathlib import Path
import json, re, sys
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[1]; GAME = ROOT.parent
sys.path.insert(0, str(HERE))
from tokens import tokens

def rust_schemas():
    out = {}
    for f in ['complex', 'game', 'lobby', 'editor']:
        src = (ROOT / f'crates/aa-orders/src/payload/{f}.rs').read_text()
        body = src.split('Some(match descriptor {', 1)[1].split('_ => return None', 1)[0]
        for arm in re.findall(r'([\d |]+)=> (\w+),', body):
            for i in arm[0].split('|'):
                out[int(i.strip())] = f'aa_orders::payload::{f}::{arm[1]}' if f == 'complex' else f'aa_orders::payload ({f}.rs {arm[1]})'
    return out

def describe(seq):
    names = {'U': lambda t: f'u{t[1]}', 'Str': lambda t: 'cstring' if t[1] != '0076a9c0' else 'named_cstring(0x76A9C0)',
             'Def': lambda t: f'def[{t[1]}]', 'Bounded': lambda t: f'bounded({t[1]})', 'Player': lambda t: 'player_id32',
             'NetAddr': lambda t: 'ipv4_32+port_32'}
    parts = []
    for t in seq:
        k = t[0]
        parts.append(names[k](t) if k in names else (f'thing16({k[5:] or "any"})' if k.startswith('Thing') else str(t)))
    return ' '.join(parts) or '(empty)'

ACTOR_COMMANDS = ['kill', 'bombard', 'guard', 'repair', 'construct', 'move', 'stop', 'retreat', 'rout', 'explore', 'disband',
                  'unpack', 'pack_organization', 'build_structure', 'commission', 'recruit', 'set_rally_point', 'recruit_location',
                  'structure_cancel_queued', 'construction_cancel', 'sell', 'upgrade', 'raze', 'sally_forth', 'recall', 'air_attack',
                  'air_recon', 'escort_on', 'escort_off', 'attach', 'detach', 'hq_attach', 'hq_detach', 'set_formation', 'pack',
                  'research', 'cancel_flare', 'group_behavior_enable', 'group_behavior_disable']

def main():
    traces = json.loads((HERE / 'order-traces.json').read_text())
    managers = {c + 1: g for g, c in traces['managers'].items()}
    static = {r['id']: r for r in json.loads((HERE / 'static-handler-calls.json').read_text())}
    sites = json.loads((HERE / 'descriptor-write-sites.json').read_text())
    flow_path = HERE / 'order-flow.json'
    flow = {d['id']: d for d in json.loads(flow_path.read_text())['descriptors']} if flow_path.exists() else {}
    rust = rust_schemas()
    ported_process = {47: 'aa_orders::native::process_move (0x61E830, phase4 oracle)', 66: 'aa_orders::native::process_move (0x61E830, phase4 oracle)',
                      85: 'aa_orders::commands::process_actor (0x61E220, phase5 oracle)', 87: 'aa_orders::commands::process_actor (0x61E220, phase5 oracle)',
                      84: 'aa_orders::commands::process_selected boundary (0x61F210; 0x4ED360 unported)', 86: 'aa_orders::commands::process_selected boundary (0x61F210; 0x4ED360 unported)',
                      298: 'aa_orders::commands::process_set_player (0x75D710, phase5 oracle)', 320: 'aa_orders::commands::process_set_player (0x75D710, phase5 oracle)'}
    rows = []
    for d in traces['descriptors']:
        i = d['id']; s = static[i]
        runs = d['runs']
        seqs = {tuple(tokens(r['reads'], managers)) for r in runs if r['status'] == 'stopped_before_process'}
        decrees = {r['writes'][0]['value'] for r in runs if r.get('writes')}
        f = flow.get(i)
        flow_summary = None
        if f:
            acc = next((c for c in f['cases'] if c['config'] in ('accepted', 'executed')), None)
            flow_summary = {'cases': [c['config'] for c in f['cases']],
                            'accepted_events': [e['kind'] + (':' + e.get('log', '') if 'log' in e else '') for e in acc['events']] if acc else None,
                            'rejections': {c['config']: next((e['line'] for e in c['events'] if e['kind'] == 'rejected'), None) for c in f['cases']}}
        rows.append({
            'id': i, 'name': d['name'], 'role': 'request' if d['name'].endswith('Request') else 'decree',
            'handler': d['handler'],
            'decree_for_request': sorted(decrees)[0] if decrees else None,
            'request_emitters': sites.get(str(i)) if d['name'].endswith('Request') else None,
            'process': s.get('process'), 'validity': s.get('validity'),
            'template_request': s.get('template_request'),
            'checks': {'ordering_client': '0x766c30' in s['calls'], 'version': '0x766c40' in s['calls'], 'film': '0x766c70' in s['calls']},
            'encoding_observed': sorted({describe(q) for q in seqs}, key=len)[:6],
            'encoding_variants_observed': len(seqs),
            'native_read_functions': sorted({e['fn'] for r in runs for e in r['reads']}),
            'oracle_runs': len(runs),
            'oracle_null_dereference_runs': sum(r['status'] == 'native_null_dereference' for r in runs),
            'rust_schema': rust.get(i),
            'flow': flow_summary,
            'ported': {'encoding': i in rust, 'request_decree_flow': bool(f), 'process': ported_process.get(i, 'external OrderHost::process seam')},
            'proven': {'encoding': 'order-traces.json + tests/order_traces.rs' if i in rust else None,
                       'flow': 'order-flow.json + tests/order_flow.rs' if f else None,
                       'process': 'phase4/phase5 oracle' if i in ported_process else None},
        })
    commands = [{'type_id': n, 'ids': name, 'binding': '0x4DA2F0 (0x4DA1E0(type, IDS))',
                 'registry': '0xB3AA58 ActorCommand definitions (capacity 0x27 at 0x48A8E0->0x47F7C0)',
                 'carried_by': 'generic command wire bit6 definition (0x61E6B0/0x61EBA0) in KKC_TellSelectedCommandOrder 84/86 or KKC_TellActorCommandOrder 85/87; move also KKC_TellActorMoveOrder 47/66 (type 5), attack also KKC_TellActorAttackOrder 46/65',
                 'data': 'Data/Game/actor_commands*.tgi, Game/ActorCommands/*.tgi'} for n, name in enumerate(ACTOR_COMMANDS)]
    team = re.findall(r'IDS\s*=\s*(\w+)', (GAME / 'reconstruction/assets/original/Data/Game/team_commands.tgi').read_text(errors='replace'))
    mg = re.findall(r'IDS\s*=\s*(\w+)', (GAME / 'reconstruction/assets/original/Data/MetaGame/mg_command.tgi').read_text(errors='replace'))
    report = {'schema': 1, 'source': 'original AA.exe; see README.txt', 'descriptor_count': len(rows),
              'summary': {'encoding_ported': sum(r['ported']['encoding'] for r in rows),
                          'encoding_oracle_runs': sum(r['oracle_runs'] for r in rows),
                          'flow_ported': sum(r['ported']['request_decree_flow'] for r in rows),
                          'process_ported': sorted(ported_process)},
              'descriptors': rows,
              'actor_command_types': commands,
              'team_commands': {'ids_in_data_order': team, 'carried_by': 'KKC_TeamCommand 22/23: 32-bit index into [[0xB36530]+0x270]+0x24, flag word +0x24 selects fields (0x509090)'},
              'metagame_commands': {'ids': mg, 'carried_by': 'KKC_TellMGCommandOrder 355/356 (0x953510)'}}
    (HERE / 'inventory.json').write_text(json.dumps(report, indent=1) + '\n')
    print(report['summary'])

if __name__ == '__main__':
    main()
