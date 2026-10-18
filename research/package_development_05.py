"""Package the verified visual fixes and Phase5 engine components separately."""
from pathlib import Path
import hashlib,json,re,shutil,struct,zipfile

root=Path(__file__).resolve().parents[1]
qa=root/'qa-output/phase5'
exe=root/'target/x86_64-pc-windows-msvc/release/AxisAlliesRust.exe'
prior=root/'release/Development-04'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
tests=(qa/'workspace-tests-final.txt').read_text(encoding='utf-8-sig')
count=sum(map(int,re.findall(r'test result: ok\. (\d+) passed;',tests)))
assert count==187 and 'FAILED' not in tests
assert not re.search(r'; [1-9]\d* (?:failed|ignored);',tests)
clippy=(qa/'workspace-clippy-final.txt').read_text(encoding='utf-8-sig')
assert 'Finished' in clippy and not re.search(r'^(?:warning|error)',clippy,re.M)
integration=read(qa/'executable-check.json')
assert integration['passed'] and integration['exe_sha256']==sha(exe)
maps=read(qa/'all-maps-final/report.json')
clips=read(qa/'animations-final/report.json')
assert len(maps['maps'])==80 and all(r['exit_code']==0 for r in maps['maps'])
assert len(clips['results'])==85 and all(r['exit_code']==0 for r in clips['results'])
assert len({(r['rig'],r['clip_index']) for r in clips['results']})==85
assert maps['executable_sha256']==clips['executable_sha256']==sha(exe)
visuals=read(qa/'visuals-after/report.json')
assert visuals['passed'] and visuals['sha256']==sha(exe)

reviews=sorted((root/'research').glob('phase5*/independent-review.json'))
assert len(reviews)==11
pin_rows=[]
for path in reviews:
    report=read(path);assert report['passed'],path
    pins={}
    for field in ['files','source_sha256','sha256']:
        value=report.get(field,{})
        if isinstance(value,dict):
            for name,digest in value.items():
                if isinstance(digest,str) and len(digest)==64 and ('/' in name or '\\' in name):
                    assert (root/name).is_file(),name
                    pins[name]=digest
    for name,digest in pins.items():
        assert sha(root/name)==digest,f'review pin changed: {name}'
    pin_rows.append({'review':path.relative_to(root).as_posix(),'sha256':sha(path),'pins_checked':len(pins)})
(qa/'review-pins-final.json').write_text(json.dumps({'passed':True,'reviews':pin_rows},indent=2))

previous=read(prior/'manifest.json')
animation_rows=[r for r in previous['files'] if r['path'].startswith('assets-cache/animation/')]
assert len(animation_rows)==48
for row in animation_rows:
    assert sha(root/row['path'])==row['sha256']
sidecar=root/'assets-cache/model-materials.json'
material_review=read(root/'research/phase5-material-render/independent-review.json')
assert material_review['source_sha256']['assets-cache/model-materials.json']==sha(sidecar)

data=exe.read_bytes();pe=struct.unpack_from('<I',data,0x3c)[0]
assert data[pe:pe+4]==b'PE\0\0'
assert struct.unpack_from('<H',data,pe+4)[0]==0x8664
assert struct.unpack_from('<H',data,pe+24)[0]==0x20b
assert b'PerMonitorV2' in data and b'asInvoker' in data
release=root/'release/Development-05'
release.mkdir(parents=True,exist_ok=False)
def copy(source,name):
    destination=release/name;destination.parent.mkdir(parents=True,exist_ok=True)
    shutil.copy2(source,destination)

copy(exe,exe.name);copy(root/'README.txt','README.txt')
for name in ['DEVELOPMENT-05','VALIDATION-05','PROGRESS','PARITY','BASELINE','SPEC','NEXT-ENGINE-STEPS']:
    copy(root/f'docs/{name}.txt',f'docs/{name}.txt')
copy(sidecar,'assets-cache/model-materials.json')
for row in animation_rows:copy(root/row['path'],row['path'])
for path in reviews:copy(path,'evidence/'+path.relative_to(root/'research').as_posix())
for name in ['workspace-tests-final.txt','workspace-clippy-final.txt','review-pins-final.json','executable-check.json',
             'all-maps-final/report.json','animations-final/report.json','visuals-after/report.json','material-focus/report.json']:
    copy(qa/name,'evidence/'+name)
copy(root/'research/phase5-visual-audit/visual-review.json','evidence/visual-review.json')
copy(qa/'visuals-after/crete.png','Preview-Crete.png')
copy(qa/'material-focus/farm-after.png','Preview-Farm.png')
manifest={'status':'Development05; incomplete reconstruction, original recovered assets required',
          'standalone_game':False,'full_game_parity':False,'architecture':'AMD64 PE32+',
          'per_monitor_v2':True,'large_address_aware':bool(struct.unpack_from('<H',data,pe+22)[0]&0x20),
          'tests_passed':count,'strict_clippy_passed':True,'maps_rendered':80,'animation_bindings_rendered':85,
          'original_animation_models_preserved':46,'independent_reviews':11,
          'headless_mechanics_checks':53,'headless_world_samples':24,'files':[]}
for p in sorted(release.rglob('*')):
    if p.is_file():manifest['files'].append({'path':p.relative_to(release).as_posix(),'bytes':p.stat().st_size,'sha256':sha(p)})
(release/'manifest.json').write_text(json.dumps(manifest,indent=2),encoding='utf-8')
archive=release/'AxisAlliesRust-Development-05.zip'
with zipfile.ZipFile(archive,'x',zipfile.ZIP_DEFLATED,compresslevel=6) as z:
    for p in sorted(release.rglob('*')):
        if p.is_file() and p!=archive:z.write(p,p.relative_to(release).as_posix())
with zipfile.ZipFile(archive) as z:
    assert z.testzip() is None
    for row in manifest['files']:assert hashlib.sha256(z.read(row['path'])).hexdigest()==row['sha256']
    assert z.read(exe.name)==data
result={'folder':str(release),'exe_sha256':sha(exe),'zip_sha256':sha(archive),
        'zip_bytes':archive.stat().st_size,'all_payload_hashes_and_crc_verified':True}
(qa/'package-report.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
print(json.dumps(result))
