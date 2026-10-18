"""Independent reversed-order replay; never writes the author's oracle files."""
from pathlib import Path
import hashlib,importlib.util,json
HERE=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
fixture=HERE/'native-object-init-null.json'
producer=HERE/'native_object_init_null.py'
before={str(p.name):sha(p) for p in [fixture,producer]}
spec=importlib.util.spec_from_file_location('reviewed_object_init_null',producer)
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
data=json.loads(fixture.read_text())
assert before[producer.name]==data['producer_sha256']
assert sha(Path(module.nm.__file__))==data['harness_sha256']
assert hashlib.sha256(module.nm.P.b).hexdigest()==data['original_sha256']
assert hashlib.sha256(module.nm.V.b).hexdigest()==data['v14_sha256']
count=0
for row in reversed(data['cases']):
    for name,image in [('v14',module.nm.V),('original',module.nm.P)]:
        actual=module.run(row['input'],image,row['precision'])
        assert actual==row['output'],(name,row['precision'],row['input']['label'],actual,row['output'])
        count+=1
assert before=={str(p.name):sha(p) for p in [fixture,producer]},'Author oracle bytes changed during review'
result={'passed':True,'fixture_rows':len(data['cases']),'native_executions':count,
        'order':'reverse rows; v14 then original','author_oracle_files_unchanged':True,
        'pins':before,'harness_sha256':data['harness_sha256'],
        'scope':'Object Init(NULL) coordinator with explicit service seams; not complete placement/spatial/visibility or publication.'}
(HERE/'independent-replay-result.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
