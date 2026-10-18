"""Independent rerun of command fixtures and bounded wire/process review."""
from pathlib import Path
import hashlib
import json
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
import native_commands as oracle

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

doc = json.loads((HERE / 'commands-native-oracle.json').read_text())
assert doc['producer_sha256'] == sha(HERE / 'native_commands.py')
assert oracle.sha(oracle.PE0.b) == doc['original_sha256']
assert oracle.sha(oracle.PE1.b) == doc['v14_sha256']
for window in doc['executed_windows']:
    start, end = int(window['start'], 16), int(window['end'], 16)
    raw = oracle.PE0.range(start, end - start)
    assert raw == oracle.PE1.range(start, end - start)
    assert hashlib.sha256(raw).hexdigest() == window['sha256']
machine = oracle.Machine()
actual = {'payloads': oracle.payloads(machine) + oracle.boundary_payloads(machine),
          'slots': oracle.slots(machine), 'actor_process': oracle.actor_process(machine),
          'selected_process': oracle.selected_process(machine),
          'player_process': oracle.player_process(machine), 'packets': oracle.packet_cases(machine)}
counts = {'payloads': 309, 'slots': 18, 'actor_process': 5, 'selected_process': 3,
          'player_process': 7, 'packets': 3}
for key, count in counts.items():
    assert len(actual[key]) == count and actual[key] == doc[key], key
log = (HERE / 'independent-tests.log').read_text(encoding='utf-8-sig')
assert log.count('test result: ok. 6 passed; 0 failed') == 2
assert 'test result: ok. 7 passed; 0 failed' in log
files = ['crates/aa-orders/src/commands.rs', 'crates/aa-orders/src/lib.rs',
         'crates/aa-orders/tests/generic_commands.rs', 'research/phase5-orders/native_commands.py',
         'research/phase5-orders/commands-native-oracle.json',
         'research/phase5-orders/independent-tests.log',
         'research/phase4-orders/recover_orders.py', 'research/phase3-orders/original_orders.py']
report = {
    'schema': 1, 'passed': True, 'component': 'aa-orders generic command wire and accepted process boundaries',
    'files': {p: sha(ROOT / p) for p in files},
    'original_sha256': doc['original_sha256'], 'v14_sha256': doc['v14_sha256'],
    'findings': [],
    'resolved_findings': ['The historical executed_windows field includes visited hook-entry bytes; producer metadata and README now explicitly distinguish byte pinning from execution behind intercepted seams.'],
    'independent_tests': {'command': 'cargo test -p aa-orders -- --nocapture',
                          'passed': 19, 'failed': 0, 'phase5_tests': 7,
                          'reexecuted_original_case_counts': counts},
    'checks': [
        'Independently checked original/v14 image hashes and every recorded byte window, then regenerated all345 Phase5 fixture records in memory and matched their stored outputs.',
        'Eleven-bit flags retain bit10 without inventing payload; conditional fields follow native order, preserve raw float-looking words, count sentinels, parameter-set null discriminator, signed count and final trailing bit.',
        'All start bit offsets0..8 match native serializer/reader; writer retains unusual whole-byte/fragment ordering and explicit cursor position.',
        'Registry counts and references remain caller inputs, actor suffix is a low16 slot without generation, and object/actor virtual type eligibility remains required resolution work.',
        'Request/decree descriptor variants remain distinct. Actor permission may replace command before linked-group preparation/delivery; delivery return is deliberately not equated with acceptance.',
        'Selected command processing delegates the original whole selection operation, with a separate diagnostic/default inspection boundary rather than an invented actor broadcast.',
        'Current-player lookup/authorization order, live client binding, missing film-stop callback mutation and final abort store match native fixture behavior.',
        'Malformed counts/references/truncation are explicit errors, and parameter lists require caller budget even for zero-bit references. Mixed packets establish player before generic command delivery.'
    ],
    'limits': [
        'The serializer is not a native packet finalizer, network protocol session, authority scheduler or implemented gameplay command graph.',
        'Runtime registries, pointer/type resolution, selection formation/default ownership handling, group preparation, permission, final gameplay delivery and film-stop internals are explicit synthetic leaves.',
        'Visited byte windows include intercepted seam entry bytes; no execution or implementation parity is inferred for bytes behind those seams.',
        'Only accepted decrees may reach process APIs after external authorization. Methods do not enforce a network trust boundary by themselves.',
        'Errors are nontransactional; safe memory/count domains intentionally differ from native corrupt-pointer behavior. No implementation/native/runtime/save/release file was changed by review.'
    ]
}
(HERE / 'independent-review.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'passed': True, 'report_sha256': sha(HERE / 'independent-review.json')}))
