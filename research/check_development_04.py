"""Bounded executable integration and unchanged-render regression checks.

No GUI is launched. The prior executable and native game are read-only.
"""
from pathlib import Path
import argparse
import hashlib
import json
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--exe', type=Path, default=ROOT / 'target/x86_64-pc-windows-msvc/release/AxisAlliesRust.exe')
parser.add_argument('--output', type=Path, default=ROOT / 'qa-output/phase4/executable-check')
args = parser.parse_args()
exe = args.exe.resolve()
previous = ROOT / 'release/Development-03/AxisAlliesRust.exe'
out = args.output.resolve()
out.mkdir(parents=True, exist_ok=False)
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
report = {
    'executable_sha256': sha(exe), 'previous_executable_sha256': sha(previous),
    'scope': 'Headless mechanics integration and bounded unchanged-render regression; no gameplay or live UI acceptance.',
    'checks': [], 'render_pairs': [],
}


def save():
    (out / 'report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')


def run(binary, arguments):
    # Deliberately unrelated CWD, so the package cannot rely on the workspace CWD.
    result = subprocess.run([str(binary), *arguments], cwd=ROOT.parent.parent,
                            capture_output=True, text=True, encoding='utf-8', timeout=90)
    if result.returncode:
        save()
        raise RuntimeError(f'{binary.name} {arguments}: {result.returncode}\n{result.stdout}\n{result.stderr}')
    return {'result': json.loads(result.stdout), 'stderr': result.stderr}


for name, arguments in [
    ('mechanics', ['--mechanics-check']),
    ('asset_catalog', ['--verify-assets']),
    ('authored_actor', ['--actor-check', 'usa_infantry']),
    ('economy', ['--economy-check']),
]:
    check = {'name': name, **run(exe, arguments)}
    report['checks'].append(check)
    save()
    print(f'{name}: passed', flush=True)

# Exact source comparison proves that broad renderer tests need not be repeated.
with zipfile.ZipFile(ROOT / 'backups/development-03-source-before-phase4.zip') as backup:
    paths = [name for name in backup.namelist()
             if name.startswith('crates/aa-client/src/') and not name.endswith('main.rs')]
    for name in paths:
        assert backup.read(name) == (ROOT / name).read_bytes(), f'renderer source changed: {name}'
    report['unchanged_client_modules'] = paths

cases = [
    ('corps', ['--model', 'Buildings/GBR/GBRCorps/GBRCorps.nif'], 'vulkan'),
    ('corps_pack', ['--model', 'Buildings/GBR/GBRCorps/GBRCorpsPack.nif'], 'vulkan'),
    ('depot_oil', ['--model', 'Buildings/GBR/GBRDepotOil/GBRDepotOil.nif'], 'vulkan'),
    ('tutorial', ['--map', 'Campaign/tutorial01'], 'vulkan'),
    ('bunker_clip', ['--animated', 'Vehicle_USABunkerArtillery', '--clip-index', '1', '--time', '0.2'], 'vulkan'),
    ('corps_dx12', ['--model', 'Buildings/GBR/GBRCorps/GBRCorps.nif'], 'dx12'),
]
for name, scene, backend in cases:
    row = {'case': name, 'backend': backend}
    for label, binary in [('previous', previous), ('current', exe)]:
        png = out / f'{name}-{label}.png'
        row[label] = run(binary, ['--backend', backend, '--workers', '4', *scene, '--render-check', str(png)])
        row[label]['image_sha256'] = sha(png)
    row['pixel_file_identical'] = row['previous']['image_sha256'] == row['current']['image_sha256']
    report['render_pairs'].append(row)
    save()
    assert row['pixel_file_identical'], f'render changed in {name}'
    print(f'{name}: identical to Development-03', flush=True)
report['passed'] = True
save()
print(json.dumps({'passed': True, 'render_pairs': len(cases), 'report': str(out / 'report.json')}))
