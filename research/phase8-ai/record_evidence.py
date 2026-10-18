"""Regenerate every phase8 AI oracle fixture and the inventory, then pin hashes.

Read-only with respect to the game: each producer runs original AA.exe code in
Unicorn. Writes the fixture JSON files, inventory.json, ego-fields.json and
findings.json in this directory. Rust tests are run separately (README.txt).
The SAIData oracles read answers recorded from the shipped data by the Rust
example first (README.txt): cargo run -p aa-mind --release --locked --offline
--example dump_sai_inputs -- research/phase8-ai/sai-shipped-inputs.json
research/phase8-ai/sai-shipped-egos.json
"""
from pathlib import Path
import hashlib, importlib, json, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()

PRODUCERS = ['native_sai_tables', 'native_goal_priority', 'native_rout_score', 'native_rout_select',
             'native_recover_select', 'native_mind_primitives', 'native_rout_think', 'native_state_thinks', 'native_goal_qsort',
             'native_goal_engine', 'native_cai_engage', 'native_cai_reactions', 'native_uai_kill',
             'native_goal_assign', 'native_goal_base', 'native_rout_enter', 'native_sleep_think',
             'native_engage_think', 'native_guard_think', 'native_saidata', 'native_saidata_ego', 'native_travel_time',
             'native_aai_attack', 'native_aai_escort', 'native_aai_guard', 'native_aai_join', 'native_aai_landing', 'native_aai_queued', 'native_aai_recon', 'native_aai_takeoff', 'native_aai_transport', 'native_cai_bombard', 'native_cai_construct', 'native_cai_explore', 'native_cai_join', 'native_cai_kill', 'native_cai_pack', 'native_cai_rampage', 'native_cai_reform', 'native_cai_repair', 'native_cai_siege', 'native_cai_unpack', 'native_uai_bombard', 'native_uai_engage', 'native_uai_join', 'native_uai_reattach', 'native_uai_repair', 'native_uai_sleep', 'native_uai_unload', 'native_uai_unpack', 'native_member_added', 'native_map_analysis', 'native_actor_notify',
             'ego_fields',
             'build_inventory']
RUST = ['crates/aa-mind/src/lib.rs', 'crates/aa-mind/src/x87.rs', 'crates/aa-mind/src/rout_score.rs',
        'crates/aa-mind/src/rout_select.rs', 'crates/aa-mind/src/recover_select.rs', 'crates/aa-mind/src/cai_rout.rs', 'crates/aa-mind/src/cai_states.rs',
        'crates/aa-mind/src/sai/mod.rs', 'crates/aa-mind/src/sai/goal_priority.rs', 'crates/aa-mind/src/sai/tables.rs',
        'crates/aa-mind/tests/sai_goal_priority_native.rs', 'crates/aa-mind/tests/rout_score_native.rs',
        'crates/aa-mind/tests/rout_select_native.rs', 'crates/aa-mind/tests/recover_select_native.rs',
        'crates/aa-mind/tests/mind_primitives_native.rs', 'crates/aa-mind/tests/cai_rout_native.rs',
        'crates/aa-mind/tests/state_thinks_native.rs', 'crates/aa-mind/src/sai/crt_qsort.rs',
        'crates/aa-mind/src/sai/engine.rs', 'crates/aa-mind/src/cai_engage.rs', 'crates/aa-mind/src/cai_reactions.rs',
        'crates/aa-mind/src/uai_kill.rs', 'crates/aa-mind/tests/sai_qsort_native.rs',
        'crates/aa-mind/tests/sai_engine_native.rs', 'crates/aa-mind/tests/cai_engage_native.rs',
        'crates/aa-mind/tests/cai_reactions_native.rs', 'crates/aa-mind/tests/uai_kill_native.rs',
        'crates/aa-mind/src/sai/assign.rs', 'crates/aa-mind/src/sai/goal_base.rs', 'crates/aa-mind/src/cai_sleep.rs',
        'crates/aa-mind/src/cai_engage_think.rs', 'crates/aa-mind/src/cai_guard.rs',
        'crates/aa-mind/tests/sai_assign_native.rs', 'crates/aa-mind/tests/sai_goal_base_native.rs',
        'crates/aa-mind/tests/cai_rout_enter_native.rs', 'crates/aa-mind/tests/cai_sleep_native.rs',
        'crates/aa-mind/tests/cai_engage_think_native.rs', 'crates/aa-mind/tests/cai_guard_native.rs',
        'crates/aa-mind/src/sai/data.rs', 'crates/aa-mind/src/sai/profile.rs', 'crates/aa-mind/src/sai/rules_data.rs',
        'crates/aa-mind/src/sai/bridge.rs', 'crates/aa-mind/src/sai/ego.rs', 'crates/aa-mind/src/sai/ego_lists.rs',
        'crates/aa-mind/src/sai/native_hash.rs', 'crates/aa-mind/examples/dump_sai_inputs.rs',
        'crates/aa-mind/tests/common/mod.rs', 'crates/aa-mind/tests/common/data_host.rs',
        'crates/aa-mind/tests/sai_profile_native.rs', 'crates/aa-mind/tests/sai_ego_native.rs',
        'crates/aa-mind/tests/sai_bridge.rs', 'crates/aa-mind/src/sai/travel.rs',
        'crates/aa-mind/tests/sai_travel_native.rs',
        'crates/aa-mind/src/cai_member_added.rs',
        'crates/aa-mind/src/cai_thinks.rs',
        'crates/aa-mind/src/cai_thinks/bombard.rs',
        'crates/aa-mind/src/cai_thinks/construct.rs',
        'crates/aa-mind/src/cai_thinks/explore.rs',
        'crates/aa-mind/src/cai_thinks/join.rs',
        'crates/aa-mind/src/cai_thinks/kill.rs',
        'crates/aa-mind/src/cai_thinks/pack.rs',
        'crates/aa-mind/src/cai_thinks/rampage.rs',
        'crates/aa-mind/src/cai_thinks/reform.rs',
        'crates/aa-mind/src/cai_thinks/repair.rs',
        'crates/aa-mind/src/cai_thinks/siege.rs',
        'crates/aa-mind/src/cai_thinks/siege_target.rs',
        'crates/aa-mind/src/cai_thinks/stack_action.rs',
        'crates/aa-mind/src/cai_thinks/unit_order.rs',
        'crates/aa-mind/src/cai_thinks/unpack.rs',
        'crates/aa-mind/src/element_move_continuation.rs',
        'crates/aa-mind/src/sai/actor_notify.rs',
        'crates/aa-mind/src/sai/map_analysis.rs',
        'crates/aa-mind/src/uai_thinks.rs',
        'crates/aa-mind/src/uai_thinks/aai_attack.rs',
        'crates/aa-mind/src/uai_thinks/aai_escort.rs',
        'crates/aa-mind/src/uai_thinks/aai_guard.rs',
        'crates/aa-mind/src/uai_thinks/aai_join.rs',
        'crates/aa-mind/src/uai_thinks/aai_landing.rs',
        'crates/aa-mind/src/uai_thinks/aai_math.rs',
        'crates/aa-mind/src/uai_thinks/aai_queued.rs',
        'crates/aa-mind/src/uai_thinks/aai_recon.rs',
        'crates/aa-mind/src/uai_thinks/aai_takeoff.rs',
        'crates/aa-mind/src/uai_thinks/aai_transport.rs',
        'crates/aa-mind/src/uai_thinks/uai_bombard.rs',
        'crates/aa-mind/src/uai_thinks/uai_engage.rs',
        'crates/aa-mind/src/uai_thinks/uai_join.rs',
        'crates/aa-mind/src/uai_thinks/uai_reattach.rs',
        'crates/aa-mind/src/uai_thinks/uai_repair.rs',
        'crates/aa-mind/src/uai_thinks/uai_sleep.rs',
        'crates/aa-mind/src/uai_thinks/uai_unload.rs',
        'crates/aa-mind/src/uai_thinks/uai_unpack.rs',
        'crates/aa-mind/tests/aai_attack_native.rs',
        'crates/aa-mind/tests/aai_escort_native.rs',
        'crates/aa-mind/tests/aai_guard_native.rs',
        'crates/aa-mind/tests/aai_join_native.rs',
        'crates/aa-mind/tests/aai_landing_native.rs',
        'crates/aa-mind/tests/aai_queued_native.rs',
        'crates/aa-mind/tests/aai_recon_native.rs',
        'crates/aa-mind/tests/aai_takeoff_native.rs',
        'crates/aa-mind/tests/aai_transport_native.rs',
        'crates/aa-mind/tests/cai_bombard_native.rs',
        'crates/aa-mind/tests/cai_construct_native.rs',
        'crates/aa-mind/tests/cai_explore_native.rs',
        'crates/aa-mind/tests/cai_join_native.rs',
        'crates/aa-mind/tests/cai_kill_native.rs',
        'crates/aa-mind/tests/cai_member_added_native.rs',
        'crates/aa-mind/tests/cai_pack_native.rs',
        'crates/aa-mind/tests/cai_rampage_native.rs',
        'crates/aa-mind/tests/cai_reform_native.rs',
        'crates/aa-mind/tests/cai_repair_native.rs',
        'crates/aa-mind/tests/cai_siege_native.rs',
        'crates/aa-mind/tests/cai_unpack_native.rs',
        'crates/aa-mind/tests/sai_actor_notify_native.rs',
        'crates/aa-mind/tests/sai_map_analysis_native.rs',
        'crates/aa-mind/tests/uai_bombard_native.rs',
        'crates/aa-mind/tests/uai_engage_native.rs',
        'crates/aa-mind/tests/uai_join_native.rs',
        'crates/aa-mind/tests/uai_reattach_native.rs',
        'crates/aa-mind/tests/uai_repair_native.rs',
        'crates/aa-mind/tests/uai_sleep_native.rs',
        'crates/aa-mind/tests/uai_unload_native.rs',
        'crates/aa-mind/tests/uai_unpack_native.rs']
INPUTS = ['sai-shipped-inputs.json', 'sai-shipped-egos.json']
FIXTURES = {
    'native-sai-tables.json': ('7BDDD0/7BDB70, 7BDCF0, 7BDEA0', 'aa_mind::sai::tables', 'sai_goal_priority_native'),
    'native-goal-priority.json': ('7A5200, 7A3860 (+45D400/85D840)', 'aa_mind::sai::goal_priority', 'sai_goal_priority_native'),
    'native-rout-score.json': ('52A930 (+445400 natively, captured)', 'aa_mind::rout_score::score', 'rout_score_native'),
    'native-rout-select.json': ('52B530 (+52A930, 85DAA0/85DA80)', 'aa_mind::rout_select::select', 'rout_select_native'),
    'native-recover-select.json': ('52BB40 (+58AC70, 52A930)', 'aa_mind::recover_select::select', 'recover_select_native'),
    'native-mind-primitives.json': ('52A340, 529E60, 52A000, 52A1F0', 'aa_mind::{pop,think_base,is_current,within_arrival}', 'mind_primitives_native'),
    'native-rout-think.json': ('521240 (+529E60, 52A000, 4F02B0, 529F50, 52A1F0)', 'aa_mind::cai_rout::think', 'cai_rout_native'),
    'native-state-thinks.json': ('51E110, 52DF30, 51DE10, 51E260, 523930 (+52A340)', 'aa_mind::cai_states', 'state_thinks_native'),
    'native-goal-qsort.json': ('96CE20/96CDB0 CRT qsort + 7A30B0', 'aa_mind::sai::crt_qsort, engine::base_priority_order', 'sai_qsort_native'),
    'native-goal-engine.json': ('7A7700, 7A93B0', 'aa_mind::sai::engine', 'sai_engine_native'),
    'native-cai-engage.json': ('521060, 523DD0, 524C10, 51EFB0, 51F040, 523EA0, 51FE60', 'aa_mind::cai_engage', 'cai_engage_native'),
    'native-cai-reactions.json': ('51F9F0, 51F920 (+52A810), 5206A0, 52E830, 527240', 'aa_mind::cai_reactions', 'cai_reactions_native'),
    'native-uai-kill.json': ('5340C0, 535240', 'aa_mind::uai_kill', 'uai_kill_native'),
    'native-goal-assign.json': ('7A9150, 7A7E30, 7A35D0, 7A3740, 7A7EB0, 7A6C70, 7A3E10, 7A3F30, 7A50E0, 7A33E0, 7AFA40, 56A1F0, 7A6480',
                                'aa_mind::sai::assign', 'sai_assign_native'),
    'native-goal-base.json': ('7A3B00, 4A7130 (+96D890 SSE2), 7A4FB0, 7A5020, 7A5080, 7A5600, 7A3D30, 7AFB00, 7A6930, 7A6A90, 7A67B0, 7A0F80',
                              'aa_mind::sai::goal_base', 'sai_goal_base_native'),
    'native-rout-enter.json': ('51F190 (+51F040, 52C0D0)', 'aa_mind::cai_rout::enter', 'cai_rout_enter_native'),
    'native-sleep-think.json': ('524D70 (+52A810)', 'aa_mind::cai_sleep::think', 'cai_sleep_native'),
    'native-engage-think.json': ('526A70 (+524C10, 4364C0)', 'aa_mind::cai_engage_think::think', 'cai_engage_think_native'),
    'native-guard-think.json': ('5294F0 (+529C20, 52A810)', 'aa_mind::cai_guard::think', 'cai_guard_native'),
    'native-saidata-profile.json': ('4AA430, 4AE3E0 (+47D960, 473AD0, 497220)', 'aa_mind::sai::profile',
                                    'sai_profile_native'),
    'native-saidata-ego.json': ('4AB9A0, 4A8E00, 4A5C60, 4B0560 (+47D960, 4AD980, 4AF500, 4A5D80, 4AF230, 4AAD80, '
                                '4AF8E0, 4A66E0, 4ADF50, 4AB090, 4AAEB0, 4A7C70, 4AFD50, 4AFB60, 4AA1D0, 4A8700, '
                                '4AEDC0, 4AF0C0, 4A86D0, 7BA3C0, 606750, 7A6430, 96CE20/4A5C20, 473AD0)',
                                'aa_mind::sai::ego, ego_lists, native_hash', 'sai_ego_native'),
    'native-travel-time.json': ('7A1C10, 7A1B30 (route block 53F320/53E5B0/5C2CD0 = aa_navigation::route_cache::route_cost)',
                                'aa_mind::sai::travel', 'sai_travel_native'),
    'native-aai-escort.json': ('51CC10 (helper harness native_uai_harness.py)', 'aa_mind::uai_thinks::aai_escort', 'aai_escort_native'),
    'native-aai-guard.json': ('51C900 (helper harness native_uai_harness.py)', 'aa_mind::uai_thinks::aai_guard', 'aai_guard_native'),
    'native-aai-join.json': ('518D60 (helper harness native_uai_harness.py)', 'aa_mind::uai_thinks::aai_join', 'aai_join_native'),
    'native-aai-landing.json': ('519750 (helper harness native_uai_harness.py)', 'aa_mind::uai_thinks::aai_landing', 'aai_landing_native'),
    'native-aai-queued.json': ('519F70 (helper harness native_uai_harness.py)', 'aa_mind::uai_thinks::aai_queued', 'aai_queued_native'),
    'native-aai-recon.json': ('5180B0 (helper harness native_uai_harness.py)', 'aa_mind::uai_thinks::aai_recon', 'aai_recon_native'),
    'native-aai-takeoff.json': ('519150 (helper harness native_uai_harness.py)', 'aa_mind::uai_thinks::aai_takeoff', 'aai_takeoff_native'),
    'native-aai-transport.json': ('5185D0 (helper harness native_uai_harness.py)', 'aa_mind::uai_thinks::aai_transport', 'aai_transport_native'),
    'native-cai-bombard.json': ('5207F0 (helper harness native_cai_regiment.py)', 'aa_mind::cai_thinks::bombard', 'cai_bombard_native'),
    'native-cai-construct.json': ('51DB80 (helper harness native_cai_regiment.py)', 'aa_mind::cai_thinks::construct', 'cai_construct_native'),
    'native-cai-explore.json': ('525D70 (helper harness native_cai_regiment.py)', 'aa_mind::cai_thinks::explore', 'cai_explore_native'),
    'native-cai-join.json': ('521850 (helper harness native_cai_regiment.py)', 'aa_mind::cai_thinks::join', 'cai_join_native'),
    'native-cai-kill.json': ('520730 (helper harness native_cai_regiment.py)', 'aa_mind::cai_thinks::kill', 'cai_kill_native'),
    'native-cai-pack.json': ('527090 (helper harness native_cai_regiment.py)', 'aa_mind::cai_thinks::pack', 'cai_pack_native'),
    'native-cai-rampage.json': ('5267E0 (helper harness native_cai_regiment.py)', 'aa_mind::cai_thinks::rampage', 'cai_rampage_native'),
    'native-cai-reform.json': ('51DE70 (helper harness native_cai_regiment.py)', 'aa_mind::cai_thinks::reform', 'cai_reform_native'),
    'native-cai-repair.json': ('5239A0 (helper harness native_cai_regiment.py)', 'aa_mind::cai_thinks::repair', 'cai_repair_native'),
    'native-cai-siege.json': ('525930 (helper harness native_cai_regiment.py)', 'aa_mind::cai_thinks::siege', 'cai_siege_native'),
    'native-cai-unpack.json': ('526E60 (helper harness native_cai_regiment.py)', 'aa_mind::cai_thinks::unpack', 'cai_unpack_native'),
    'native-uai-bombard.json': ('5353E0 (helper harness native_uai_harness.py)', 'aa_mind::uai_thinks::uai_bombard', 'uai_bombard_native'),
    'native-uai-engage.json': ('536E10 (helper harness native_uai_harness.py)', 'aa_mind::uai_thinks::uai_engage', 'uai_engage_native'),
    'native-uai-join.json': ('531AD0 (helper harness native_uai_harness.py)', 'aa_mind::uai_thinks::uai_join', 'uai_join_native'),
    'native-uai-reattach.json': ('534000 (helper harness native_uai_harness.py)', 'aa_mind::uai_thinks::uai_reattach', 'uai_reattach_native'),
    'native-uai-repair.json': ('5359D0 (helper harness native_uai_harness.py)', 'aa_mind::uai_thinks::uai_repair', 'uai_repair_native'),
    'native-uai-sleep.json': ('52F950 (helper harness native_uai_harness.py)', 'aa_mind::uai_thinks::uai_sleep', 'uai_sleep_native'),
    'native-uai-unload.json': ('52F440 (helper harness native_uai_harness.py)', 'aa_mind::uai_thinks::uai_unload', 'uai_unload_native'),
    'native-uai-unpack.json': ('5372B0 (helper harness native_uai_harness.py)', 'aa_mind::uai_thinks::uai_unpack', 'uai_unpack_native'),
    'native-aai-attack.json': ('51B8C0 (+401060, 401290, 445250, 45E790, 46B000, 516720; harness native_uai_harness.py)', 'aa_mind::uai_thinks::aai_attack', 'aai_attack_native'),
    'native-actor-notify.json': ('7A0C20', 'aa_mind::sai::actor_notify::notify', 'sai_actor_notify_native'),
    'native-member-added.json': ('524B80 (+4D0900)', 'aa_mind::cai_member_added::member_added', 'cai_member_added_native'),
    'native-map-analysis.json': ('7AC0D0 (+7AA5B0, 7A9C60/7A9C80, 7B9180, 7BB480, 7BAC00, 7AE350, 7AD890, 7ADDA0, 4A5E40, 96D890 SSE2, 7ABA70, 7AB940, 7AA620, 45D720, 7B9050, 7BB780, 7BB820, 5C37E0)', 'aa_mind::sai::map_analysis::dynamic_map_analysis', 'sai_map_analysis_native'),
}


def main():
    for name in PRODUCERS:
        importlib.import_module(name).main()
    fixtures = {}
    for f, (addresses, rust, test) in FIXTURES.items():
        d = json.loads((HERE / f).read_text(encoding='utf-8'))
        fixtures[f] = {'native': addresses, 'rust': rust, 'test': f'cargo test -p aa-mind --test {test}',
                       'cases': len(d.get('cases', [])) or sum(len(t['entries']) for t in d.get('tables', {}).values()),
                       'original_instructions_pinned': len(d['instructions']), 'sha256': sha(HERE / f)}
    inv = json.loads((HERE / 'inventory.json').read_text(encoding='utf-8'))
    findings = {
        'schema': 1,
        'scope': 'Phase8 AI: native AI inventory; SAI goal priority/affordability; SAI enum tables; CAI rout/recover destination '
                 'scoring and selection; CAI rout Think; retreat/deploy/regroup/exhausted/chasing Think dispatchers; '
                 'Mind pop/think-base/is-current/arrival primitives; SAI goal engine passes, actor assignment and base '
                 'goal virtuals; combat-facing CAI target list, reactions and UAI kill; rout OnEnter; sleep, engage and '
                 'guard Thinks; SAIData profile and ego loaders on the shipped data; SAI actor travel time; SAI dynamic '
                 'map analysis; the remaining regiment, element and aircraft Mind-state Thinks; regiment Mind '
                 'member-added.',
        'fixtures': fixtures,
        'inventory_summary': inv['summary'],
        'producers': {p.name: sha(p) for p in sorted(HERE.glob('*.py'))},
        'rust_pins': {r: sha(ROOT / r) for r in RUST},
        'inputs': {f: sha(HERE / f) for f in INPUTS},
        'inventory_sha256': sha(HERE / 'inventory.json'),
        'ego_fields_sha256': sha(HERE / 'ego-fields.json'),
    }
    (HERE / 'findings.json').write_text(json.dumps(findings, indent=1) + '\n', encoding='utf-8')
    print(json.dumps({k: v['cases'] for k, v in fixtures.items()}))


if __name__ == '__main__':
    main()
