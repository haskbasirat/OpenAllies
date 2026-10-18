"""Record independent review evidence without changing implementation/fixtures."""
from pathlib import Path
import hashlib
import json
import struct

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

fixture = json.loads((HERE / 'sqrt-vectors.json').read_text())
assert fixture['producer_sha256'] == sha(HERE / 'sqrt_oracle.py')
assert len(fixture['cases']) == 3267
assert {p: sum(c['precision'] == p for c in fixture['cases'])
        for p in ('Single24', 'Double53', 'Extended64')} == dict.fromkeys(
            ('Single24', 'Double53', 'Extended64'), 1089)
raw_remainder = json.loads((HERE / 'remainder-vectors.json').read_text())
remainder = json.loads((HERE / 'remainder-hardware-vectors.json').read_text())
assert raw_remainder['producer_sha256'] == sha(HERE / 'remainder_oracle.py')
assert remainder['producer_sha256'] == raw_remainder['producer_sha256']
for key, path in [('raw_unicorn_fixture_sha256', 'remainder-vectors.json'),
                  ('capture_producer_sha256', 'capture_host_remainder.py'),
                  ('host_source_sha256', 'host_remainder.rs'),
                  ('host_executable_sha256', 'host_remainder.exe'),
                  ('input_sha256', 'host-remainder-input.bin'),
                  ('output_sha256', 'host-remainder-output.bin')]:
    assert remainder[key] == sha(HERE / path)
assert len(remainder['cases']) == 3696
assert {p: sum(c['precision'] == p for c in remainder['cases'])
        for p in ('Single24', 'Double53', 'Extended64')} == dict.fromkeys(
            ('Single24', 'Double53', 'Extended64'), 1232)
inputs = (HERE / 'host-remainder-input.bin').read_bytes()
outputs = (HERE / 'host-remainder-output.bin').read_bytes()
assert len(inputs) == 3696 * 24 and len(outputs) == 3696 * 14
discrepancies = []
for index, (row, raw) in enumerate(zip(remainder['cases'], raw_remainder['cases'])):
    assert (row['precision'], row['kind'], row['inputs']) == (raw['precision'], raw['kind'], raw['inputs'])
    assert row['unicorn_result80'] == raw['result80']
    assert row['unicorn_result32'] == raw['result32']
    expected_input = struct.pack('<IB3xQQ', {'Single24': 0x7f, 'Double53': 0x27f, 'Extended64': 0x37f}[row['precision']], row['kind'] == 'f64', *row['inputs'])
    assert inputs[index*24:(index+1)*24] == expected_input
    expected_output = bytes.fromhex(row['result80']) + struct.pack('<I', row['result32'])
    assert outputs[index*14:(index+1)*14] == expected_output
    if row['result80'] != raw['result80'] or row['result32'] != raw['result32']:
        discrepancies.append({'index': index, **row})
inventory = json.loads((HERE / 'remainder-unicorn-discrepancies.json').read_text())
assert discrepancies == inventory['cases']
assert len(discrepancies) == inventory['count'] == remainder['unicorn_mismatches'] == 1188
log = (HERE / 'independent-tests.log').read_text(encoding='utf-8-sig')
assert log.count('test result: ok. 3 passed; 0 failed') == 3
files = ['crates/aa-numeric/src/lib.rs', 'crates/aa-numeric/tests/square_root.rs',
         'crates/aa-numeric/tests/remainder.rs',
         'crates/aa-numeric/tests/native_parity.rs',
         'research/phase5-numeric/sqrt_oracle.py',
         'research/phase5-numeric/sqrt-vectors.json',
         'research/phase5-numeric/remainder_oracle.py',
         'research/phase5-numeric/remainder-vectors.json',
         'research/phase5-numeric/remainder-hardware-vectors.json',
         'research/phase5-numeric/remainder-unicorn-discrepancies.json',
         'research/phase5-numeric/capture_host_remainder.py',
         'research/phase5-numeric/host_remainder.rs',
         'research/phase5-numeric/host_remainder.exe',
         'research/phase5-numeric/host-remainder-input.bin',
         'research/phase5-numeric/host-remainder-output.bin',
         'research/phase5-numeric/independent-tests-unicorn-failure.log',
         'research/phase5-numeric/independent-tests.log']
report = {
    'schema': 1, 'passed': True, 'component': 'aa-numeric finite square root and exact remainder extensions',
    'files': {p: sha(ROOT / p) for p in files},
    'findings': [],
    'resolved_findings': ['Unicorn FPREM returns incorrect results for 1188 of 3696 wide-exponent cases. Independent tests caught the discrepancy; raw emulator observations and the failing test log are preserved. Acceptance uses independently captured and re-executed real-host FPREM results plus exact integer arithmetic. Production remainder was not changed to match the emulator error.'],
    'independent_tests': {'command': 'cargo test -p aa-numeric -- --nocapture',
                          'passed': 9, 'failed': 0, 'sqrt_cases': 3267,
                          'remainder_cases': 3696,
                          'unicorn_remainder_discrepancies': 1188,
                          'actual_host_x87_cases': 6963,
                          'prior_synthetic_arithmetic_cases': 234},
    'checks': [
        'Reviewed scale selection k=floor(log2(x)/2)-(p-1), including odd negative exponents and binade carry.',
        'Integer root of floor(N/D) gives the correct lower root; exact comparison 4N versus D*(2q+1)^2 implements nearest-even with no host-float approximation.',
        'Negative zero is preserved, negative nonzero input is rejected, and existing checked normal extended exponent limits remain in force.',
        'Fixtures retain the 80-bit result before any binary32 store; tests compare that value separately from the final binary32 bits and explicit overflow error.',
        'Fixture generator hash and case counts matched; all 3267 extended fixture values independently passed against actual host FSQRT at PC24/53/64.',
        'Exact remainder aligns both dyadic operands to a common exponent, applies unsigned magnitude modulo and restores the dividend sign. Divisor sign does not change the result; zero dividend/remainder preserves its sign.',
        'FPREM is iterated until C2 clears in capture and host test; all 3696 hardware-captured extended remainders match Rust before binary32 store and actual host re-execution at all three precision settings.',
        'Independently verified capture source/executable/input/output hashes, every binary input/output record, unchanged raw emulator observations and all 1188 mismatch-inventory records.',
        'Remainder does not round by PC, matching complete FPREM rather than substituting rounded division/multiplication. Zero divisor and unsupported extended result domain are explicit errors.',
        'Host test/capture assembly saves/restores aligned 512-byte FXSAVE64 state, initializes a private x87 environment, restores on every path, and changes no stack pointer or unrelated memory. EAX/flags clobber is declared in the FPREM loop.',
        'Original add/subtract/multiply/divide behavior is unchanged; the earlier 234 primitive vectors also independently pass.'
    ],
    'limits': [
        'These are synthetic arithmetic instructions, not original game control-flow evidence; original/v14 executable pinning is not applicable to this primitive oracle and is checked by each native caller fixture.',
        'Only finite operands, nearest-even, and the normal extended exponent domain are supported. Exception flags, nonfinite arithmetic, alternate rounding and extended subnormal results remain outside the API.',
        'The x86_64 host cross-check ran on this machine, not every supported CPU or platform.',
        'Unicorn remainder results must not be extrapolated beyond hardware-checked domains. Original game callers using CRT fmod need their own operation/store and domain evidence.',
        'No implementation, native executable, runtime, asset, save, or release file was changed by this review.'
    ]
}
(HERE / 'independent-review.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'passed': True, 'report_sha256': sha(HERE / 'independent-review.json')}))
