"""Bind final movement source, original instruction fixtures and focused checks."""
from pathlib import Path
import hashlib, json, os, subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).parent
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()


def main():
    cargo = Path(os.environ.get('USERPROFILE', 'C:/Users/haskb')) / '.cargo/bin/cargo.exe'
    commands = [([str(cargo), 'test', '-p', 'aa-movement', '--', '--nocapture'], 'rust-tests.log'),
                ([str(cargo), 'clippy', '-p', 'aa-movement', '--all-targets', '--', '-D', 'warnings'], 'clippy.log')]
    results = []
    for args, name in commands:
        run = subprocess.run(args, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        log = OUT / name; log.write_text(run.stdout)
        results.append({'command': args, 'exit_code': run.returncode, 'log': name, 'sha256': sha(log)})
        if run.returncode: raise RuntimeError(run.stdout)
    base = json.loads((OUT / 'native-movement-fixtures.json').read_text())
    turns = json.loads((OUT / 'native-movement-turning-fixtures.json').read_text())
    assert base['passed'] and turns['passed']
    pc53 = [r for r in turns['fixtures'] if r['input']['context']['precision'] == 'Double53' and r['input']['operation'] == 'step']
    unique = {r['input']['name'] for r in base['fixtures']} | {r['input']['name'] for r in pc53}
    sources = [ROOT / 'crates/aa-movement' / p for p in ['Cargo.toml', 'README.md', 'src/lib.rs', 'tests/native_fixtures.rs']]
    sources += [OUT / p for p in ['README.md', 'inspect_native.py', 'native-ranges.json', 'native_oracle.py',
                'native-movement-fixtures.json', 'native-movement-turning-fixtures.json',
                'record_actiongo_chain.py', 'actiongo-chain.json', 'record_evidence.py']]
    dependencies = [ROOT / p for p in ['crates/aa-numeric/src/lib.rs', 'crates/aa-trig/src/lib.rs', 'crates/aa-trig/src/angle.rs',
                    'research/phase4-trig/lookup-tables.bin', 'research/phase4-trig/native-trig-vectors.json',
                    'research/phase4-orders/numeric-independent-review.json',
                    'research/phase4-trig/lookup-independent-review.json',
                    'research/phase4-trig-angle/independent-review.json']]
    report = {'schema': 1, 'passed': True, 'component': 'Original bounded Mover order and ground-step operations',
              'full_game_movement': False, 'concrete_turning_precision': 'Double53 only; no native default inferred',
              'original_sha256': base['original_sha256'], 'v14_sha256': base['accepted_v14_sha256'],
              'rust_test_count': 4, 'base_comparisons': len(base['fixtures']),
              'concrete_pc53_ground_step_comparisons': len(pc53), 'total_rust_comparisons': len(base['fixtures']) + len(pc53),
              'unique_native_cases_compared': len(unique), 'extended_native_fixture_executions': turns['case_count'],
              'commands': results,
              'source_sha256': {p.relative_to(ROOT).as_posix(): sha(p) for p in sources},
              'dependency_sha256': {p.relative_to(ROOT).as_posix(): sha(p) for p in dependencies},
              'supported': ['complete begin_ground_motion5773c0 and stop5767c0 operations with explicit activation/scheduler boundaries',
                            'complete ground_step5774a0 control flow with required world/collision/Mind/Character callbacks',
                            'exact dyadic arithmetic/store boundaries, explicit PC and separate acceleration/world deltas',
                            'concrete original table/CRT turning only under explicitly supplied PC53'],
              'limits': ['World callbacks are required interfaces, not implemented pathfinding/collision/terrain/spatial engine parity.',
                         'Actor command5 must not bypass UAI/path production by invoking the Mover directly.',
                         'Outer scheduling, falling, aircraft and path/formation planning remain separate.',
                         'Global world/SVar values are stable operation inputs; callback mutation parity applies to mover/actor fields.',
                         'Unicorn precision-exception traces do not establish real-CPU sticky status; PC24/64 concrete turning rejects.',
                         'Nonfinite/overflow/undefined table-index cases are rejected; arbitrary invalid native memory is outside the supported domain.',
                         'Original/release/runtime/profile files were not changed.']}
    path = OUT / 'movement-evidence.json'; path.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'passed': True, 'sha256': sha(path), 'tests': report['rust_test_count'],
                      'comparisons': report['total_rust_comparisons'], 'unique_cases': len(unique)}))


if __name__ == '__main__': main()
