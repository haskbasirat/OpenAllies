"""Headless, read-only source-asset render checks. Does not launch or control a GUI."""
from pathlib import Path
import argparse
import hashlib
import json
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser()
parser.add_argument('--exe', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--backend', default='vulkan', choices=['vulkan', 'dx12'])
args = parser.parse_args()
exe = args.exe.resolve()
output = args.output.resolve()
output.mkdir(parents=True, exist_ok=False)
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
cases = [
    ('crete', ['--map', 'Campaign/axis01_crete']),
    ('tutorial', ['--map', 'Campaign/tutorial01']),
    ('road', ['--model', 'Roads/DirtRoad/DirtRoadStraight.nif']),
    ('corps', ['--model', 'Buildings/GBR/GBRCorps/GBRCorps.nif']),
    ('corps_pack', ['--model', 'Buildings/GBR/GBRCorps/GBRCorpsPack.nif']),
    ('depot_oil', ['--model', 'Buildings/GBR/GBRDepotOil/GBRDepotOil.nif']),
    ('gbr_infantry', ['--model', 'Units/GBR/GBRInfantry/GBRInfantry.nif']),
]
report = {'schema': 1, 'executable': str(exe), 'sha256': sha(exe),
          'method': 'Bounded source-asset loading and headless GPU render; no native gameplay or live UI acceptance.',
          'backend': args.backend, 'cases': []}
for name, selector in cases:
    png = output / f'{name}.png'
    command = [str(exe), '--assets', str(ROOT.parent/'reconstruction'), '--workers', '4',
               '--backend', args.backend, *selector, '--render-check', str(png)]
    start = time.monotonic()
    result = subprocess.run(command, cwd=ROOT.parent.parent, capture_output=True, text=True,
                            encoding='utf-8', timeout=90)
    row = {'name': name, 'selector': selector, 'exit_code': result.returncode,
           'wall_seconds': time.monotonic()-start, 'stderr': result.stderr}
    if result.returncode == 0:
        row.update(result=json.loads(result.stdout), image_sha256=sha(png))
    else:
        row['stdout'] = result.stdout
    report['cases'].append(row)
    (output/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    print(f'{name}: {result.returncode}', flush=True)
report['passed'] = all(row['exit_code'] == 0 for row in report['cases'])
(output/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print(json.dumps({'passed': report['passed'], 'report': str(output/'report.json')}))
raise SystemExit(not report['passed'])
