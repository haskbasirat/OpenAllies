"""Fresh executable render smoke tests, not full game or visual parity acceptance."""
from pathlib import Path
import argparse
import hashlib
import json
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--exe', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    exe = args.exe.resolve()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    original_sha = sha(exe)
    cases = [
        ('live-jog-start-vulkan', ['--live-preview', 'usa_infantry', '--clip', 'jog', '--time', '0', '--backend', 'vulkan']),
        ('live-jog-half-vulkan', ['--live-preview', 'usa_infantry', '--clip', 'jog', '--time', '0.5', '--backend', 'vulkan']),
        ('live-jog-half-dx12', ['--live-preview', 'usa_infantry', '--clip', 'jog', '--time', '0.5', '--backend', 'dx12']),
        ('crete-vulkan', ['--map', 'Campaign/axis01_crete', '--backend', 'vulkan']),
        ('corps-dx12', ['--model', 'Buildings/GBR/GBRCorps/GBRCorps.nif', '--backend', 'dx12']),
        ('animation-vulkan', ['--animated', 'USAInfantry', '--clip-index', '1', '--time', '0.5', '--backend', 'vulkan']),
    ]
    rows = []
    for name, options in cases:
        image = output / f'{name}.png'
        command = [str(exe), *options, '--render-check', str(image)]
        start = time.monotonic()
        process = subprocess.run(command, cwd=ROOT.parent.parent, capture_output=True, text=True,
                                 encoding='utf-8', errors='replace', timeout=180)
        (output / f'{name}.stdout.txt').write_text(process.stdout, encoding='utf-8')
        (output / f'{name}.stderr.txt').write_text(process.stderr, encoding='utf-8')
        assert process.returncode == 0, f'{name}: {process.stderr}'
        result = json.loads(process.stdout)
        assert result['draw_calls'] > 0 and result['triangles'] > 0
        assert image.read_bytes().startswith(b'\x89PNG\r\n\x1a\n')
        rows.append({'name': name, 'arguments': options, 'exit_code': process.returncode,
                     'elapsed_seconds': time.monotonic() - start, 'image_sha256': sha(image), 'result': result})
    assert rows[0]['image_sha256'] != rows[1]['image_sha256'], 'Jog time did not change image'
    rejected = []
    for options in [['--live-preview'], ['--live-preview', 'not_an_actor'],
                    ['--live-preview', 'usa_infantry', '--clip', 'not_a_clip'],
                    ['--live-preview', 'usa_infantry', '--map', 'Campaign/tutorial01']]:
        process = subprocess.run([str(exe), *options, '--render-check', str(output / 'invalid.png')],
                                 cwd=ROOT.parent.parent, capture_output=True, text=True,
                                 encoding='utf-8', errors='replace', timeout=60)
        assert process.returncode != 0 and not (output / 'invalid.png').exists(), options
        rejected.append({'arguments': options, 'exit_code': process.returncode,
                         'diagnostic': process.stderr.strip()})
    assert original_sha == sha(exe)
    report = {'passed': True, 'executable': str(exe), 'executable_sha256': original_sha,
              'scope': 'Packaged executable static render smoke; not full gameplay or frame-rate validation',
              'renders': rows, 'invalid_arguments_rejected': rejected}
    (output / 'report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps({'passed': True, 'renders': len(rows), 'invalid_inputs': len(rejected),
                      'executable_sha256': original_sha}))
