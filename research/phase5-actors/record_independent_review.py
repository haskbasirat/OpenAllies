"""Independent bounded base-Thing construction and saved identity review."""
from pathlib import Path
import hashlib
import json
import struct
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
import original_creation as oracle

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

doc = json.loads((HERE / 'creation-native-oracle.json').read_text())
assert doc['producer_sha256'] == sha(HERE / 'original_creation.py')
assert doc['registry_machine_sha256'] == sha(ROOT / 'research/phase4-actors/trace_registry.py')
assert oracle.H(oracle.P.b) == doc['original_sha256']
assert oracle.H(oracle.V.b) == doc['v14_sha256']
assert sha(HERE / 'direction-tables.bin') == doc['direction_table_sha256']
for row in doc['native_windows']:
    start, end = int(row['start'], 16), int(row['end'], 16)
    raw = oracle.P.range(start, end - start)
    assert raw == oracle.V.range(start, end - start)
    assert hashlib.sha256(raw).hexdigest() == row['sha256']
constants = {}
for address in [0x9f32e0, 0x9f32dc, 0xa2a5a0, 0x9ffe7c]:
    raw = oracle.P.range(address, 4)
    assert raw == oracle.V.range(address, 4)
    constants[hex(address)] = raw.hex()
control = {'Single24': 0x7f, 'Double53': 0x27f, 'Extended64': 0x37f}
tables = {}
for row in doc['tables']:
    machine = oracle.Machine(control[row['precision']])
    value = machine.table()
    assert value == row['values'] and len(value) == 2048
    tables[row['precision']] = value
binary = b''.join(struct.pack('<2I', *xy) for row in doc['tables'] for xy in row['values'])
assert binary == (HERE / 'direction-tables.bin').read_bytes()
assert len(doc['cases']) == 282
for row in doc['cases']:
    case = row['input']
    machine = oracle.Machine(control[case['precision']])
    machine.use_table(tables[case['startup_precision']])
    assert machine.constructor(case) == row
log = (HERE / 'independent-tests.log').read_text(encoding='utf-8-sig')
assert log.count('test result: ok. 2 passed; 0 failed') == 3
assert 'test result: ok. 3 passed; 0 failed' in log
files = ['crates/aa-actors/src/creation.rs', 'crates/aa-actors/src/registry.rs',
         'crates/aa-actors/tests/native_creation.rs', 'research/phase5-actors/original_creation.py',
         'research/phase5-actors/creation-native-oracle.json', 'research/phase5-actors/direction-tables.bin',
         'research/phase5-actors/independent-tests.log',
         'crates/aa-numeric/src/lib.rs', 'crates/aa-simulation/src/arithmetic.rs']
report = {
    'schema': 1, 'passed': True, 'component': 'aa-actors base Thing construction and valid-record identity',
    'files': {p: sha(ROOT / p) for p in files},
    'original_sha256': doc['original_sha256'], 'v14_sha256': doc['v14_sha256'],
    'constant_original_v14_bytes': constants, 'findings': [],
    'independent_tests': {'command': 'cargo test -p aa-actors -- --nocapture',
                          'passed': 9, 'failed': 0, 'phase5_tests': 3,
                          'reexecuted_constructor_identity_cases': 282,
                          'reexecuted_startup_tables': 3, 'vectors_per_table': 2048},
    'checks': [
        'Independently verified original/v14 file pins, every recorded native window and the used one/zero/normalization tolerance constants; re-executed all282 native cases and all three2048-vector startup tables.',
        'Flag mask retains bits8..25 and29 with initial bit31; initial scalar words/ID, raw position/time bits and UTF16 including embedded NUL/unpaired units match native results.',
        'Axis bit17 takes precedence over random bit15 and avoids a draw. Random branch consumes one supplied world-stream LCG draw modulo2048; constructor never chooses a seed.',
        'Supplied direction is checked using strict length error threshold and mutates the source record on rejection. Subsequent2D squared-length normalization and bit12 three-dimensional normalization remain distinct.',
        'Arithmetic uses explicit precision and native binary32 stores. Startup table precision is a separate input from current constructor precision; no host trigonometric substitute appears in production.',
        'Saved ID generation seeding precedes saved registration, ordinary zero ID uses the existing native allocator gate, and the Rust registry owns the sole installed ID.',
        'String/pool allocation and base visual notification representation are excluded explicitly. Spatial bit26 is absent from constructor mask and native visual link starts null, so no unimplemented spatial/publication callback is invented.',
        'Safe invalid saved slot/occupied slot behavior is delegated to the previously reviewed registry; nonfinite directions reject only when consumed. Full actor classification remains a required derived-class input.'
    ],
    'limits': [
        'This is base Thing construction, not full derived GActor creation, typed/inherited definition loading, components, ownership, ability initialization, spatial placement, destruction or world publication.',
        'Recorded native_windows pins include visited intercepted seam entry addresses; pinning is not proof of execution behind mocked allocation/debug leaves.',
        'Startup vectors were independently re-executed in the original emulated x86 routine; this review does not add a hardware cross-check of the complete direction table initialization.',
        'Fixture constructor cases use matching startup/runtime precision modes; separate table identity is verified, but all mixed startup/runtime combinations are not separately fixture-executed.',
        'Full-registry native case supplies count32768 directly; Rust tests use a valid populated registry. Neither constitutes a whole-world capacity test.',
        'The separate native definition-getter research is evidence only and is not counted as implemented Rust behavior. No implementation/native/runtime/asset/save/release file was changed by review.'
    ]
}
(HERE / 'independent-review.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'passed': True, 'report_sha256': sha(HERE / 'independent-review.json')}))
