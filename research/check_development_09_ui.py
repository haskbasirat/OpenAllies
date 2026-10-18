"""Bounded app-owned input-script smoke runner; screenshots require inspection.

Does not automate Windows or assert gameplay/DPI/audio/network correctness.
Each case has an isolated preference directory and an unrelated working folder.
"""
from pathlib import Path
import argparse
import hashlib
import json
import os
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--exe', required=True, type=Path)
    parser.add_argument('--script', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    exe, script, output = args.exe.resolve(), args.script.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    steps = json.loads(script.read_text(encoding='utf-8-sig'))
    assert steps and steps[-1].get('quit') is True, 'A bounded quit step is required'
    shots = [Path(row['shot']).resolve() for row in steps if 'shot' in row]
    assert shots and all(p.parent == output and not p.exists() for p in shots)
    profile = output / 'profile'
    (profile / 'roaming').mkdir(parents=True)
    (profile / 'local').mkdir()
    env = dict(os.environ, APPDATA=str(profile / 'roaming'), LOCALAPPDATA=str(profile / 'local'))
    initial_hash = sha(exe)
    start = time.monotonic()
    command = [str(exe), '--assets', str(ROOT.parent / 'reconstruction'), '--ui-script', str(script)]
    result = subprocess.run(command, cwd=ROOT.parent.parent, env=env,
                            capture_output=True, text=True, encoding='utf-8',
                            errors='replace', timeout=150)
    (output / 'stdout.txt').write_text(result.stdout, encoding='utf-8')
    (output / 'stderr.txt').write_text(result.stderr, encoding='utf-8')
    assert result.returncode == 0, result.stderr
    rows = []
    for path in shots:
        data = path.read_bytes()
        assert data[:8] == b'\x89PNG\r\n\x1a\n'
        rows.append({'path': str(path), 'sha256': sha(path),
                     'width': int.from_bytes(data[16:20], 'big'),
                     'height': int.from_bytes(data[20:24], 'big')})
    assert sha(exe) == initial_hash
    report = {'passed': True, 'scope': 'App-owned scripted input and screenshot production; manual visual inspection recorded separately',
              'executable_sha256': initial_hash, 'script_sha256': sha(script),
              'command': command, 'exit_code': result.returncode,
              'elapsed_seconds': time.monotonic() - start,
              'isolated_profile': str(profile), 'screenshots': rows}
    (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(report))
