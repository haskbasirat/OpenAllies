"""Independent replay and review binding for the four Phase6 routing modules.

Does not overwrite any native fixture or owner evidence ledger. Region prepass
and cost/search implementation peer review are separate scopes.
"""
from pathlib import Path
import hashlib
import json
import os
import re
import struct
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
import endpoint_oracle as endpoint
import materialize_oracle as materialize
import result_plan_oracle as result_plan
import solve_oracle as solve


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


original = endpoint.Pe(ROOT.parent / 'AA.exe', endpoint.ORIGINAL_SHA)
accepted = endpoint.Pe(ROOT.parent / 'reconstruction/native-game/runtime-expanded-v14/AA.exe', endpoint.ACCEPTED_SHA)
provenance = json.loads((HERE / 'native-provenance.json').read_text())
for row in provenance['functions'].values():
    address = int(row['start'], 16)
    code = original.at(address, row['size'])
    assert code == accepted.at(address, row['size'])
    assert hashlib.sha256(code).hexdigest() == row['sha256']
for address, row in provenance['constants'].items():
    raw = original.at(int(address, 16), 4)
    assert raw == accepted.at(int(address, 16), 4)
    assert struct.unpack('<I', raw)[0] == row['bits']
for row in provenance['tables'].values():
    raw = original.at(int(row['address'], 16), len(row['words']) * 4)
    assert raw == accepted.at(int(row['address'], 16), len(raw))
    assert hashlib.sha256(raw).hexdigest() == row['sha256']
    assert list(struct.unpack('<' + 'I' * len(row['words']), raw)) == row['words']

counts = {'endpoint': 258, 'materialize': 159, 'result-plan': 249, 'solve': 72}
files = [HERE / 'native-provenance.json', Path(__file__)]
for name, count in counts.items():
    module = {'endpoint': endpoint, 'materialize': materialize, 'result-plan': result_plan, 'solve': solve}[name]
    fixture = HERE / (name + '-fixtures.json')
    ranges = HERE / (name + '-ranges.json')
    doc = json.loads(fixture.read_text())
    assert doc['case_count'] == len(doc['fixtures']) == count
    assert doc['oracle_sha256'] == sha(Path(module.__file__))
    assert doc['original_sha256'] == endpoint.ORIGINAL_SHA
    assert doc['v14_sha256'] == endpoint.ACCEPTED_SHA
    for row in json.loads(ranges.read_text()):
        if 'bytes' in row:
            address, raw = int(row['address'], 16), bytes.fromhex(row['bytes'])
            assert original.at(address, len(raw)) == accepted.at(address, len(raw)) == raw
        else:
            address, end = int(row['start'], 16), int(row['end_exclusive'], 16)
            raw = original.at(address, end - address)
            assert raw == accepted.at(address, end - address)
            assert hashlib.sha256(raw).hexdigest() == row['sha256']
    for row in doc['fixtures']:
        if name == 'solve':
            machine = solve.SolveMachine({'Single24': 0x7f, 'Double53': 0x27f, 'Extended64': 0x37f}[row['precision']])
            result = machine.run(row['input'])
        else:
            result = module.Machine(original, row['input']).run()
        assert result == row['output'], (name, row['input']['name'])
    files += [fixture, ranges, Path(module.__file__)]
    print(f'{name}: independently replayed {count} original cases', flush=True)

cargo = Path(os.environ['USERPROFILE']) / '.cargo/bin/cargo.exe'
commands = []
test_count = 0
for args, log_name in [(['test', '-p', 'aa-navigation', '--', '--nocapture'], 'independent-tests.log'),
                       (['clippy', '-p', 'aa-navigation', '--all-targets', '--', '-D', 'warnings'], 'independent-clippy.log')]:
    run = subprocess.run([str(cargo), *args], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    log = HERE / log_name
    log.write_text(run.stdout)
    assert run.returncode == 0, run.stdout
    commands.append({'command': [str(cargo), *args], 'exit_code': run.returncode, 'log': log_name, 'sha256': sha(log)})
    if log_name == 'independent-tests.log':
        test_count = sum(map(int, re.findall(r'test result: ok\. (\d+) passed', run.stdout)))
    files.append(log)

files += [ROOT / 'crates/aa-navigation' / p for p in ['src/endpoint.rs', 'src/materialize.rs', 'src/solver.rs', 'src/result_plan.rs',
          'tests/endpoint_fixtures.rs', 'tests/materialize_fixtures.rs', 'tests/solve_fixtures.rs', 'tests/result_plan_fixtures.rs']]
dependencies = ['crates/aa-navigation/src/lib.rs', 'crates/aa-navigation/src/cost.rs', 'crates/aa-navigation/src/search.rs',
                'crates/aa-spatial/src/tile_walk.rs', 'crates/aa-numeric/src/lib.rs',
                'research/phase6-pathsearch/native_search.py', 'research/phase5-navigation/native_oracle.py',
                'research/phase3-terrain/terrain_uv_oracle.py']
report = {
    'schema': 1, 'passed': True,
    'component': 'Phase6 endpoint adjustment, predecessor materialization, composed Solve and ResultPlan',
    'findings': [], 'original_sha256': endpoint.ORIGINAL_SHA, 'v14_sha256': endpoint.ACCEPTED_SHA,
    'native_cases_independently_reexecuted': counts, 'total_native_cases': sum(counts.values()),
    'entire_crate_tests_passed': test_count, 'commands': commands,
    'source_sha256': {p.relative_to(ROOT).as_posix(): sha(p) for p in files},
    'dependency_sha256': {p: sha(ROOT / p) for p in dependencies},
    'checks': [
        'Reexecuted all 738 source-pinned original fixtures in memory without replacing fixture files, and verified original/v14 function ranges, constants, direction tables and visited instruction-byte pins.',
        'Reviewed endpoint stepping/clamping and asymmetric x87 stores, including final Manhattan threshold and false-return endpoint mutation.',
        'Reviewed predecessor direction reduction, lazy collision short circuit, saved descriptor restoration, cached initial direction, half-cell f32 stores, existing tail retention and final unlink/reset behavior.',
        'Reviewed Solve branch order, strategy writes and whole native static-grid composition through concrete cost/heap/search/ray/materialization, with only node allocation/free hooks in the composed oracle.',
        'Reviewed ResultPlan retry ordering, turning-circle asymmetric stores, generation lookup/replacement, repeated virtual property calls, mutable callbacks and distinct continuation/ActionGo/failure behavior.',
        'Reviewed explicit nonfinite, invalid grid, nonprogressing endpoint, cyclic predecessor and missing actor errors; they do not silently replace native decisions with success.',
        'Visited byte pins include intercepted seam-entry bytes and are not evidence that replaced callbacks executed from native code.'
    ],
    'limits': [
        'The separate region.rs and region_search.rs implementations are not included in this review; their crate tests may be included in the whole-crate test count.',
        'Cost/search and TileWalk are dependency implementations. Their use is exercised in 72 whole-Solve cases; this report does not replace their separate full source review gates.',
        'Whole-Solve cases disable the region pather and use static collision masks with no actor objects. Region-enabled orchestration is source-reviewed only; dynamic actor collisions and actual-map construction remain separate.',
        'Endpoint, materialization and ResultPlan fixtures replace world/allocator/controller leaves explicitly. Required callbacks are not complete world integration, and arbitrary component replacement/reentrant mutation is outside fixture coverage.',
        'Round-to-nearest precision is explicit per operation; no global game-thread FPU mode or exception-flag lifecycle is inferred.',
        'World errors can leave prior state changes; automatic retry and native invalid-pointer/undefined behavior parity are not claimed.',
        'No native game executable, gameplay assets, saves, runtime or release file was changed.'
    ]
}
path = HERE / 'independent-review.json'
path.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'passed': True, 'cases': sum(counts.values()), 'crate_tests': test_count, 'report_sha256': sha(path)}))
