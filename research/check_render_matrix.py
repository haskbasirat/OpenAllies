"""Read-only source asset / headless-render smoke matrix. Never launches a GUI."""
from pathlib import Path
import argparse
import hashlib
import json
import subprocess
import time

root = Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser()
parser.add_argument('--exe',type=Path,default=root/'target/release/AxisAlliesRust.exe')
parser.add_argument('--output',type=Path,default=root/'qa-output/all-maps-vulkan')
args=parser.parse_args()
assets = root.parent / 'reconstruction'
exe = args.exe.resolve()
output = args.output.resolve()
output.mkdir(exist_ok=False)
maps = json.loads((assets / 'game/data/maps/index.json').read_text(encoding='utf-8'))
report = {
    'executable_sha256': hashlib.sha256(exe.read_bytes()).hexdigest(),
    'purpose': 'headless original-asset load/upload/render smoke check, not visual or gameplay parity',
    'backend': 'vulkan', 'workers': 4, 'maps': [],
}
for number, entry in enumerate(maps, 1):
    map_id = entry['id']
    filename = hashlib.sha256(map_id.encode()).hexdigest()[:16] + '.png'
    start = time.monotonic()
    row = {'id': map_id, 'image': filename}
    try:
        result = subprocess.run([
            str(exe), '--assets', str(assets), '--backend', 'vulkan', '--workers', '4',
            '--map', map_id, '--render-check', str(output / filename),
        ], capture_output=True, text=True, encoding='utf-8', timeout=90)
        row.update(exit_code=result.returncode, stderr=result.stderr)
        if result.returncode == 0:
            row['result'] = json.loads(result.stdout)
            row['png_sha256'] = hashlib.sha256((output / filename).read_bytes()).hexdigest()
        else:
            row['stdout'] = result.stdout
    except Exception as error:
        row['error'] = str(error)
    row['wall_seconds'] = time.monotonic() - start
    report['maps'].append(row)
    (output / 'report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(f'{number}/{len(maps)} {map_id}: {row.get("exit_code", "error")}', flush=True)
failed = [row['id'] for row in report['maps'] if row.get('exit_code') != 0]
print(json.dumps({'passed': len(maps)-len(failed), 'failed': failed, 'report': str(output/'report.json')}))
raise SystemExit(bool(failed))
