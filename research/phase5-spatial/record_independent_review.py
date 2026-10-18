"""Independent original re-execution and review of bounded spatial operations."""
from pathlib import Path
import hashlib
import json
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
import original_spatial as oracle

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

doc = json.loads((HERE / 'native-spatial-fixtures.json').read_text())
original = oracle.Pe(ROOT.parent / 'AA.exe', doc['original_sha256'])
accepted = oracle.Pe(ROOT.parent / 'reconstruction/native-game/runtime-expanded-v14/AA.exe', doc['v14_sha256'])
assert doc['case_count'] == len(doc['fixtures']) == 504
ranges = json.loads((HERE / 'native-ranges.json').read_text())
executed = json.loads((HERE / 'native-executed-ranges.json').read_text())
for row in ranges['ranges'] + ranges['data'] + executed['ranges']:
    start, end = int(row['start'], 16), int(row['end'], 16)
    raw = original.at(start, end - start)
    assert raw == accepted.at(start, end - start)
    assert hashlib.sha256(raw).hexdigest() == row['sha256']
for row in doc['fixtures']:
    assert oracle.Machine(original, row['input']).run() == row['output'], row['input']['name']
log = (HERE / 'independent-tests.log').read_text(encoding='utf-8-sig')
assert 'test result: ok. 4 passed; 0 failed' in log
files = ['crates/aa-spatial/src/lib.rs', 'crates/aa-spatial/tests/native_parity.rs',
         'research/phase5-spatial/original_spatial.py', 'research/phase5-spatial/native-ranges.json',
         'research/phase5-spatial/native-executed-ranges.json',
         'research/phase5-spatial/native-spatial-fixtures.json',
         'research/phase5-spatial/independent-tests.log', 'crates/aa-numeric/src/lib.rs']
report = {
    'schema': 1, 'passed': True, 'component': 'aa-spatial ground query, half-cell snap and position commit',
    'files': {p: sha(ROOT / p) for p in files},
    'original_sha256': doc['original_sha256'], 'v14_sha256': doc['v14_sha256'],
    'findings': [],
    'independent_tests': {'command': 'cargo test -p aa-spatial -- --nocapture',
                          'passed': 4, 'failed': 0, 'reexecuted_original_cases': 504},
    'checks': [
        'Independently checked original/v14 file pins, all recorded function/data windows and531 executed instruction addresses, then re-executed all504 original cases without replacing fixture files.',
        'Reviewed full terrain-query, move and commit assembly against Rust; query uses old actor XY and rereads callback-mutated position/flags at native boundaries.',
        'Base height stores to binary32 before use, overlay and offset arithmetic retain explicit precision, total height stores before the final unrounded base-minus-half floor comparison.',
        'Signed cell-index truncation, x86 shift masking, grid clamp, ordered neighbor search and wrapping signed remainder sequence match native instructions.',
        'Half-cell snap deliberately uses an extended X reciprocal and stored binary32 Y reciprocal; the binary64 floor argument and checked i32 floor domain are documented.',
        'NewA/oldA/newB/oldB shape descriptors capture position/facing before each getter; tile callback receives oldB/newB/oldA/newA. Candidate commit precedes position notification and current world presentation gates.',
        'Callback mutation scenarios cover actor fields, world phase and presentation target. Explicit callback errors stop later work without rolling back prior position writes.',
        'Height sampling, tile ownership, collision and presentation leaves are explicit requirements. No bounding-radius guess, route solver, inferred grid constructor or direct command-to-Mover bypass is introduced.'
    ],
    'limits': [
        'Height callbacks use synthetic native FADD return values; actual terrain interpolation and tile overlap/ownership are not implemented by this component.',
        'Grid/settings are stable per-operation snapshots, candidate buffers do not alias actor state, and callback replacement of referenced world allocations is outside the contract.',
        'Bit12 without bit21 has uninitialized native Z and is rejected; nonfinite input, invalid grid, bad half-cell and out-of-i32-floor domains return explicit errors.',
        'Query fixture results have a final caller binary32 store; preservation of unstored intermediate behavior also relies on instruction/source review and numeric primitive evidence.',
        'This does not prove live match, rendering, collision or performance parity. No implementation, native/runtime/asset/save/release file was changed by review.'
    ]
}
(HERE / 'independent-review.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'passed': True, 'cases': 504, 'report_sha256': sha(HERE / 'independent-review.json')}))
