"""Reproduce native generic-command fixtures and hash their Rust validation."""
from pathlib import Path
import hashlib
import json
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).parent

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    commands = []
    for name, command in [
        ('native-oracle.log', [sys.executable, str(OUT / 'native_commands.py')]),
        ('client-samples.log', [sys.executable, str(OUT / 'select_client_samples.py')]),
        ('rust-tests.log', ['cargo', 'test', '-p', 'aa-orders']),
        ('clippy.log', ['cargo', 'clippy', '-p', 'aa-orders', '--all-targets', '--', '-D', 'warnings']),
    ]:
        run = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (OUT / name).write_bytes(run.stdout)
        commands.append({'command': command, 'exit_code': run.returncode, 'log': name,
                         'sha256': sha(OUT / name)})
        assert run.returncode == 0, run.stdout.decode(errors='replace')
    native = json.loads((OUT / 'commands-native-oracle.json').read_text())
    files = [ROOT / 'crates/aa-orders' / name for name in [
        'Cargo.toml', 'README.md', 'src/lib.rs', 'src/commands.rs',
        'src/native.rs', 'src/native_registry.rs', 'tests/generic_commands.rs']]
    files += [OUT / name for name in [
        'README.txt', 'native_commands.py', 'commands-native-oracle.json',
        'record_evidence.py', 'select_client_samples.py', 'client_samples.rs', 'client-samples.json']]
    dependencies = [ROOT / name for name in [
        'research/phase4-orders/recover_orders.py',
        'research/phase3-orders/original_orders.py',
        'research/engine-baseline/audit_baseline.py']]
    report = {
        'schema': 1, 'passed': True,
        'component': 'aa-orders generic command serialization and guarded delivery boundaries',
        'original_sha256': native['original_sha256'], 'v14_sha256': native['v14_sha256'],
        'native_cases': {name: len(native[name]) for name in [
            'payloads', 'slots', 'actor_process', 'selected_process', 'player_process', 'packets']},
        'rust_tests': {'phase5': 7, 'all_orders': 19},
        'visited_byte_windows': len(native['executed_windows']),
        'byte_window_scope': native['byte_window_scope'],
        'source_sha256': {path.relative_to(ROOT).as_posix(): sha(path) for path in files},
        'dependency_sha256': {path.relative_to(ROOT).as_posix(): sha(path) for path in dependencies},
        'commands': commands, 'seams': native['seams'], 'limits': native['limits'],
        'peer_review': 'Separate independent-review.json; not recursively included in this ledger.',
    }
    (OUT / 'findings.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'passed': True, 'native_cases': report['native_cases'],
                      'findings_sha256': sha(OUT / 'findings.json')}))

if __name__ == '__main__':
    main()
