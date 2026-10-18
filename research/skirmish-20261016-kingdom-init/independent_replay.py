"""Read-only independent replay of the frozen Kingdom Init oracle."""
from pathlib import Path
import sys,json,hashlib,importlib.util,copy
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
source=HERE/'native_kingdom_init.py';fixture=HERE/'native-kingdom-init.json'
paths=[source,fixture,ROOT/'crates/aa-economy/src/kingdom_init.rs',ROOT/'crates/aa-economy/tests/kingdom_init.rs']
before={str(p.relative_to(ROOT)).replace('\\','/'):sha(p) for p in paths}
spec=importlib.util.spec_from_file_location('reviewed_kingdom_init',source);mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod)
doc=json.loads(fixture.read_text());assert len(doc['cases'])==99 and doc['executions']==198
executions=0;instructions=set()
for row in doc['cases']:
    for exe in [mod.ORIGINAL,mod.V14]:
        actual,seen=mod.run(exe,row['precision'],copy.deepcopy(row['case']))
        assert actual==row['expected'],(row['case']['name'],row['precision'])
        instructions.update(seen);executions+=1
assert before=={str(p.relative_to(ROOT)).replace('\\','/'):sha(p) for p in paths}
mod.nm.original=mod.ORIGINAL
out={'frozen_source_and_fixture_sha256':before,'native_replays':executions,'distinct_executed_native_instructions':len(instructions),'fixture_preserved':True}
(HERE/'independent-replay.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out))
