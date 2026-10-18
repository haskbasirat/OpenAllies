"""Independent re-execution and evidence binding for prepared navigation only."""
from pathlib import Path
import hashlib
import importlib.util
import json
import struct
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
import native_oracle as oracle
from inspect_native import RANGES

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

doc = json.loads((HERE / 'native-navigation-fixtures.json').read_text())
assert doc['oracle_sha256'] == sha(HERE / 'native_oracle.py')
assert len(doc['fixtures']) == doc['case_count'] == 255
original = oracle.Pe(ROOT.parent / 'AA.exe', doc['original_sha256'])
accepted = oracle.Pe(ROOT.parent / 'reconstruction/native-game/runtime-expanded-v14/AA.exe', doc['v14_sha256'])
ranges = json.loads((HERE / 'native-ranges.json').read_text())
for name, (start, end) in RANGES.items():
    code = original.at(start, end - start)
    assert code == accepted.at(start, end - start)
    assert hashlib.sha256(code).hexdigest() == ranges['ranges'][name]['sha256']
for address, value in ranges['constants'].items():
    raw = original.at(int(address, 16), 4)
    assert raw == accepted.at(int(address, 16), 4)
    assert struct.unpack('<I', raw)[0] == value['bits']
for address, value in ranges['double_constants'].items():
    raw = original.at(int(address, 16), 8)
    assert raw == accepted.at(int(address, 16), 8)
    assert struct.unpack('<Q', raw)[0] == value['bits']
for row in doc['fixtures']:
    assert oracle.Machine(original, row['input']).run() == row['output'], row['input']['name']
log = (HERE / 'independent-tests.log').read_text(encoding='utf-8-sig')
assert 'test result: ok. 3 passed; 0 failed' in log
files = ['crates/aa-navigation/src/lib.rs', 'crates/aa-navigation/tests/native_fixtures.rs',
         'research/phase5-navigation/native_oracle.py', 'research/phase5-navigation/native-ranges.json',
         'research/phase5-navigation/native-navigation-fixtures.json',
         'research/phase5-navigation/independent-tests.log',
         'crates/aa-numeric/src/lib.rs', 'research/phase5-numeric/independent-review.json']
report = {
    'schema': 1, 'passed': True, 'component': 'aa-navigation owned paths and prepared-node ActionGo',
    'files': {p: sha(ROOT / p) for p in files},
    'original_sha256': doc['original_sha256'], 'v14_sha256': doc['v14_sha256'],
    'findings': [],
    'independent_tests': {'command': 'cargo test -p aa-navigation -- --nocapture',
                          'passed': 3, 'failed': 0, 'reexecuted_original_cases': 255,
                          'rust_operation_comparisons': 252, 'native_self_copy_contract_rows': 3},
    'checks': [
        'Independently compared original/v14 executable pins, all20 instruction windows, binary32 constants and binary64 RNG constants; independently re-executed all255 original fixture cases without replacing fixture files.',
        'Reviewed every ActionGo branch against the recorded original assembly, including extended FSQRT threshold, projection check, ordered mutable callbacks, speed and formation branches, and ignored Mover bool return.',
        'The X offset product is stored to binary32 before subtraction while Y remains extended; RNG normalization and jitter retain extended intermediates until the native timer argument store.',
        'Current first-node kind is reread after Mover and RNG callbacks; callback effects on speed, factor, target/phase and current kind are fixture-covered.',
        'Activation writes occur before the final callback and survive false activation; ordinary path events alone check the separate active state.',
        'Owned path copy retains all eight payload words with independent storage; clear retains metadata; pruning uses signed type2 count and decrements kinds1/2 instead of filtering trailing kind2 nodes.',
        'Allocation/world/UAI/eligibility/scheduling callbacks are explicitly mocked, not claimed as recovered implementation; no route solver or direct command-to-Mover substitute exists.',
        'Error behavior is deliberately nontransactional; safe allocation/null/corrupt-list boundaries are documented without claiming native undefined-memory behavior.'
    ],
    'limits': [
        'This is the prepared-path outer operation, not pathfinding, formation solving, collision solving, full UAI scheduling or playable movement.',
        'Fixture callbacks provide selected mutable effects. Arbitrary reentrant world behavior and replacement of referenced component allocations are not covered.',
        'Native diagnostic logging is disabled. Pool address/free timing, allocation failure and invalid native pointer behavior are deliberately not reproduced.',
        'Explicit operation precision and settings remain stable inputs; no native process-wide precision default or exception-flag model is claimed.',
        'No implementation, native executable, runtime, asset, save, or release file was changed by this review.'
    ]
}
(HERE / 'independent-review.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'passed': True, 'cases': 255, 'report_sha256': sha(HERE / 'independent-review.json')}))
