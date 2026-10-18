"""Focused headless old/current material scenes and cross-backend smoke checks."""
from pathlib import Path
import hashlib
import json
import subprocess

ROOT = Path(__file__).resolve().parents[2]
out = ROOT/'qa-output/phase5/material-focus'
out.mkdir(parents=True, exist_ok=False)
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
old = ROOT/'release/Development-04/AxisAlliesRust.exe'
new = ROOT/'target/x86_64-pc-windows-msvc/release/AxisAlliesRust.exe'
models = [
    ('city', 'Buildings/Villages/Mediterranean/MediterraneanCityCenter01/MediterraneanCityCenter01.nif'),
    ('farm', 'Buildings/Villages/Mediterranean/MediterraneanFarm01/MediterraneanFarm01.nif'),
    ('vineyard01', 'Doodads/Chaparral/ChaparralVineyard/ChaparralVineyard01.nif'),
    ('vineyard02', 'Doodads/Chaparral/ChaparralVineyard/ChaparralVineyard02.nif'),
]
cases = [(f'{name}-{label}', binary, 'vulkan', ['--model', model])
         for name,model in models for label,binary in [('before',old),('after',new)]]
cases += [
    ('crete-dx12', new, 'dx12', ['--map','Campaign/axis01_crete']),
    ('city-dx12', new, 'dx12', ['--model',models[0][1]]),
    ('corps-dx12', new, 'dx12', ['--model','Buildings/GBR/GBRCorps/GBRCorps.nif']),
]
report = {'schema':1,'method':'Read-only source-asset headless render, not original-game frame parity.',
          'before_exe_sha256':sha(old),'after_exe_sha256':sha(new),'cases':[]}
for name,binary,backend,selector in cases:
    png = out/f'{name}.png'
    result = subprocess.run([str(binary),'--assets',str(ROOT.parent/'reconstruction'),'--workers','4',
                             '--backend',backend,*selector,'--render-check',str(png)],
                            cwd=ROOT.parent.parent,capture_output=True,text=True,encoding='utf-8',timeout=90)
    row = {'name':name,'backend':backend,'selector':selector,'exit_code':result.returncode,'stderr':result.stderr}
    if result.returncode == 0:
        row.update(result=json.loads(result.stdout),image_sha256=sha(png))
    else:
        row['stdout'] = result.stdout
    report['cases'].append(row)
    (out/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    print(f'{name}: {result.returncode}',flush=True)
report['passed'] = all(row['exit_code'] == 0 for row in report['cases'])
(out/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print(json.dumps({'passed':report['passed'],'cases':len(cases),'report':str(out/'report.json')}))
raise SystemExit(not report['passed'])
