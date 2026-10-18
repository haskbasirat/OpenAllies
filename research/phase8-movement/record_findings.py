"""Write findings.json: pinned sources, fixtures, counts and validation logs."""
import hashlib, json, re
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
SOURCES = [
    'crates/aa-movement/src/lib.rs', 'crates/aa-movement/src/mover.rs', 'crates/aa-movement/src/falling.rs',
    'crates/aa-movement/src/formation.rs', 'crates/aa-movement/tests/mover_native.rs',
    'crates/aa-movement/tests/formation_native.rs', 'crates/aa-navigation/src/formation_follow.rs',
    'crates/aa-navigation/tests/formation_follow_native.rs', 'crates/aa-spatial/src/overlaps.rs',
    'crates/aa-spatial/src/overlaps_tests.rs', 'crates/aa-spatial/src/query.rs',
    'crates/aa-movement/tests/native_fixtures.rs', 'crates/aa-navigation/src/sai_state.rs',
    'crates/aa-navigation/src/ai_startup.rs', 'crates/aa-navigation/tests/goal_base_fields.rs',
    'crates/aa-runtime/src/movement_world.rs',
    # Round 2
    'crates/aa-navigation/src/company_move.rs', 'crates/aa-navigation/tests/company_move_native.rs',
    'crates/aa-navigation/tests/company_think_native.rs', 'crates/aa-spatial/src/placement.rs',
    'crates/aa-spatial/src/placement_tests.rs', 'crates/aa-movement/tests/trig_constants.rs',
    'crates/aa-navigation/src/topology_builder.rs', 'crates/aa-navigation/tests/topology_builder_native.rs',
    'crates/aa-movement/src/aircraft.rs', 'crates/aa-movement/tests/aircraft_native.rs',
    'crates/aa-spatial/src/water_areas.rs', 'crates/aa-spatial/src/water_areas_tests.rs',
    'crates/aa-spatial/tests/water_areas_native.rs', 'crates/aa-heightfield/src/binding.rs',
    'crates/aa-navigation/src/low_path/mod.rs', 'crates/aa-navigation/src/low_path/window.rs',
    'crates/aa-navigation/src/low_path/goal.rs', 'crates/aa-navigation/src/low_path/search.rs',
    'crates/aa-navigation/src/low_path/continuation.rs', 'crates/aa-navigation/tests/low_path_native.rs',
    'crates/aa-navigation/src/result_plan.rs', 'crates/aa-navigation/tests/goal_base_fields.rs',
    'crates/aa-navigation/src/route_cache.rs', 'crates/aa-navigation/src/region_search.rs',
    'crates/aa-navigation/tests/route_cache_native.rs', 'crates/aa-runtime/src/spatial_world.rs',
    'crates/aa-spatial/src/gworld_placement.rs', 'crates/aa-spatial/tests/gworld_placement_native.rs',
    'crates/aa-navigation/src/uai_collision.rs', 'crates/aa-navigation/tests/uai_collision_native.rs',
]
FIXTURES = ['native-mover.json', 'native-direction-table.json', 'direction-table.bin', 'native-formation.json',
            'native-follow.json', 'native-overlaps.json', 'inventory.json',
            'native-company.json', 'native-company-think.json', 'native-placement.json', 'native-aircraft.json',
            'native-topology-builder.json', 'native-topology-builder-shipped.json',
            'native-topology-builder-findings.json', 'native-water-areas.json', 'native-water-areas-findings.json',
            'native-low-path-solve.json', 'native-low-path-continuation.json', 'native-low-path-result-plan.json',
            'native-low-path-findings.json', 'native-route-cache.json', 'native-route-cache-findings.json',
            'native-gworld-placement.json', 'native-gworld-placement-callers.json', 'native-uai-collision.json']
PRODUCERS = ['harness.py', 'native_mover.py', 'native_direction_table.py', 'native_formation.py', 'native_follow.py',
             'native_overlaps.py', 'build_inventory.py', 'native_company.py', 'native_company_think.py',
             'native_placement.py', 'native_aircraft.py', 'native_topology_builder.py', 'native_water_areas.py',
             'native_low_path.py', 'native_route_cache.py', 'record_route_cache_findings.py',
             'native_gworld_placement.py', 'native_gworld_placement_callers.py', 'native_uai_collision.py',
             'record_findings.py']


def main():
    tests = (HERE / 'rust-tests.log').read_text(encoding='utf-8', errors='ignore')
    totals = [0, 0, 0]
    for m in re.finditer(r'test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored', tests):
        for i in range(3):
            totals[i] += int(m.group(i + 1))
    inventory = json.loads((HERE / 'inventory.json').read_text())
    doc = {
        'schema': 1,
        'scope': 'Phase 8 round 1: Mover Tick/falling/speed operations, organization formation slots and '
                 'reassignment, UAI formation-following speed control, all-overlaps query and overlap-list refresh. '
                 'Round 2: CAI regiment move orders/Think/member dispatch and Element return-to-slot, free placement '
                 'search, static SAI region-topology builder, aircraft MoveTick, water-area flood fill and paint.',
        'native_executions': {
            'native-mover.json': 1251, 'native-mover.json launch_speed': 24, 'native-formation.json': 477,
            'native-follow.json': 498, 'native-overlaps.json queries': 2304, 'native-overlaps.json refreshes': 576,
            'native-direction-table.json tables': 3,
            'native-company.json': 756, 'native-company-think.json': 660,
            'native-placement.json searches': 864, 'native-placement.json distance searches': 432,
            'native-aircraft.json': 1281, 'native-topology-builder.json synthetic': 44,
            'native-topology-builder.json real-map': 12, 'native-topology-builder-shipped.json maps': 80,
            'native-water-areas.json': 417, 'native-water-areas.json original-map floods': 30,
            'native-low-path-solve.json': 321, 'native-low-path-continuation.json': 69,
            'native-low-path-result-plan.json': 21, 'native-route-cache.json cases': 84,
            'native-route-cache.json searches': 1092, 'native-route-cache.json path costs': 1080,
            'native-gworld-placement.json seamed templates x3 precisions': 1170,
            'native-gworld-placement.json composed (real 0x553BA0)': 48,
            'native-gworld-placement-callers.json 0x53CE40 rows': 1404,
            'native-gworld-placement-callers.json 0x4F46E0 rows': 1017,
            'native-uai-collision.json static/thing/tile': '441/126/69'},
        'native_null_reads_rejected_in_rust': {'native-formation.json': 3, 'native-follow.json': 3,
                                               'native-company-think.json': 39, 'native-water-areas.json': 9,
                                               'native-route-cache.json': 12},
        'helper_findings': ['native-topology-builder-findings.json', 'native-water-areas-findings.json',
                            'native-low-path-findings.json', 'native-route-cache-findings.json',
                            'native-gworld-placement.json#findings', 'native-gworld-placement-callers.json#findings',
                            'native-uai-collision.json#findings'],
        'runtime_wiring_compiled_not_oracle_run': [
            'aa-runtime spatial_world::low_path_world::SpatialLowPathWorld (LowPathWorld)',
            'aa-runtime spatial_world::low_path_world::SpatialPlacement (0x4F3C40/0x53CE40/0x4F46E0 worlds)',
            'aa-runtime spatial_world::uai_collision_world (0x46EF30, 0x46F100, 0x531170/0x5312C0, 0x4D6940, '
            '0x4CEDE0 host helpers)'],
        'removed_caps': [{'address': '0x57ba80', 'original': '32-bit pending-slot mask (assert e<32)',
                          'rust': 'exact growable per-slot set; identical below 32 slots'},
                         {'address': '0x7aa8b0', 'original': 'island flood recursion on the 0x100000-byte main stack',
                          'rust': 'explicit heap stack'}],
        'kept_limits': [{'address': '0x552cad/0x556570', 'value': 254, 'why': 'water area id is one byte (0xFF = shared)'},
                        {'address': '0x552446', 'value': '0x7FFFFFFF', 'why': 'visit-stamp restart is visible state'},
                        {'address': '0x554b30', 'value': '1..=20 rings', 'why': 'native search rule'},
                        {'address': '0x53a6aa', 'value': 'depth 32', 'why': 'low-path search rule (DEPTH_LIMIT)'},
                        {'address': '0x4f2a80', 'value': '16x16-tile window, sub shift 2',
                         'why': 'explicit WindowConfig::NATIVE; cells are a Vec'}],
        'optional_native_limits': [{'address': '0x53cdf0', 'value': '0x1F9C queue entries',
                                    'rust': 'Options::queue_limit (None = unbounded growth)'}],
        'explicit_data_inputs': {'LowPathingExaminedMax': 512, 'LowPathingIgnoreMovingDepth': 3,
                                 'LowPathingAccuracyDepth': 5, 'LowPathingWaitMinimum': '2.0',
                                 'source': 'Data/Game/SVars.tgi lines 95-98; no executable default'},
        'emulator_caveat_audit': {
            'fs_base_zero': 'page-0 hook now 0..0xFFF, only FS-prefixed [0..3] allowed (harness, topology, '
                            'placement machines)',
            'allocator_trap': 'UnseamedAllocator on 0x6F7810/0x6F7430/0x6F7700/0x6F7650/0x96E19F/0x96E173/'
                              '0x96E0F8/0x96D331; never fired',
            'changed_fixtures': {'native-aircraft.json': '6 rows: first null dereference now 0x0 (was 0x1B0/0x14); '
                                                         'events unchanged',
                                 'native-water-areas.json': '3 signalling-NaN rows marked emulator_caveat'},
            'snan_inputs': 'only the 3 water rows (neighbour height 0x7F800001)',
            'stale_code': 'all stubs at fresh addresses'},
        'precision_evidence': {'direction_table': 'built after _controlfp(_PC_53) at 0x972c6b (from 0x96bdb7 in _cinit '
                                                  '0x96ec76); PC53 == PC64 output; PC24 differs in 859 words',
                               'topology_builder': 'original builder reproduces the topology shipped in 80/80 maps at '
                                                   'PC53 and PC64, 0/80 at PC24',
                               'simulation': 'lead correction: simulation thread runs PC53 with PE set every tick'},
        'inventory': {k: inventory[k] for k in ('function_count', 'status_counts', 'proven_count')},
        'validation': {
            'tests_command': 'CARGO_TARGET_DIR=target/s-movement cargo test -p aa-movement -p aa-navigation '
                             '-p aa-spatial -p aa-heightfield --release --locked --offline -j 4',
            'tests': {'passed': totals[0], 'failed': totals[1], 'ignored': totals[2]},
            'tests_log_sha256': sha(HERE / 'rust-tests.log'),
            'clippy_command': 'CARGO_TARGET_DIR=target/s-movement cargo clippy -p aa-movement -p aa-navigation '
                              '-p aa-spatial -p aa-heightfield --release --locked --offline -j 4 --all-targets '
                              '--no-deps -- -D warnings',
            'clippy_log_sha256': sha(HERE / 'clippy.log'),
            'fixtures_regenerated_identically': True,
        },
        'source_sha256': {s: sha(ROOT / s) for s in SOURCES},
        'fixture_sha256': {f: sha(HERE / f) for f in FIXTURES},
        'producer_sha256': {f: sha(HERE / f) for f in PRODUCERS},
        'not_claimed': ['runtime integration (aa-runtime still calls the ground step directly; company move, '
                        'aircraft, topology builder and water areas have no live callers)',
                        'terrain speed producer (aa-rules owner)', 'live-game validation',
                        'x87 exception domains beyond the recorded NaN/infinity cases',
                        'callbacks: aircraft 0x55f230/0x560a70/0x55edc0, topology 0x7aa180/0x7ac9a0, water brush '
                        'stages 0x5526c0/0x559fb0/0x5528f0/0x551140/0x551b40, random-map callers 0x5972f0/0x597530',
                        '80-bit results of FPATAN/FSIN/FCOS/FYL2X (Unicorn computes them via binary64)'],
    }
    (HERE / 'findings.json').write_text(json.dumps(doc, indent=1) + '\n')
    print(json.dumps(doc['validation']['tests']))


if __name__ == '__main__':
    main()
