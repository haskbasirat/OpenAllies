"""Seal the bounded SAI component after its current focused replay succeeds."""
from pathlib import Path
import json,hashlib
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
H=lambda b:hashlib.sha256(b).hexdigest()
def pin(path):
 p=ROOT/path;return dict(path=path,sha256=H(p.read_bytes()),bytes=p.stat().st_size)
def main():
 frozen=json.loads((HERE/'frozen-source-hashes.json').read_text())
 for p in frozen['files']:assert H((ROOT/p['path']).read_bytes())==p['sha256'],p
 current=(HERE/'sai-final-nine-tests.log').read_text();assert '9 passed; 0 failed; 0 ignored'in current
 nav=(HERE/'ai-inplace-source-tests.log').read_text();assert '3 passed; 0 failed'in nav and '1 passed; 0 failed'in nav
 fixtures=[];total=0
 for p in sorted(HERE.glob('native-*.json')):
  j=json.loads(p.read_text());prefix=len(j.get('prefix_rows',[]))+len(j.get('source_prefix_rows',[]));n=len(j['rows'])+prefix
  assert j['machines']==2*n;total+=j['machines']
  fixtures.append(dict(**pin(p.relative_to(ROOT).as_posix()),complete_rows=len(j['rows']),controlled_prefix_rows=prefix,machines=j['machines'],executed_pins=len(j['executed_pins']),intercepted_leaf_entries=len(j.get('intercepted_entries',[])),virtual_seams=j.get('virtual_seams',[]),scope=j['scope']))
 actual=json.loads((HERE/'actual-profiles-current.json').read_text())
 proofs=[pin(p.relative_to(ROOT).as_posix())for p in sorted(HERE.glob('native_*.py'))]
 docs=[pin((HERE/n).relative_to(ROOT).as_posix())for n in ['README.txt','frozen-source-hashes.json','actual-profiles-current.json','actual-map-input.json','sai-final-nine-tests.log','actor-player-startup-tests-final-v2.log','ai-inplace-source-tests.log','map-list-check.log']]
 snapshot=HERE/'source-snapshot'
 for p in frozen['files']:
  target=snapshot/p['path'];target.parent.mkdir(parents=True,exist_ok=True)
  data=(ROOT/p['path']).read_bytes()
  if target.exists():assert target.read_bytes()==data
  else:target.write_bytes(data)
 doc=dict(status='author-frozen-focused-validated',scope='Same-owner SAI source/setup/topology/startup bindings; bounded native kernels, not completed whole-world startup/Think/application activation or playable battle.',
  source_files=frozen['files'],shared_sections='lib.rs module exports; restore_lifecycle.rs Arena field/constructor/disjoint loans/two4C54 pristine checks only. Other owners may change unrelated sections; no exclusive whole-file freeze claim.',
  fixtures=fixtures,native_fresh_machine_executions=total,producers=proofs,
  actual_source=dict(profiles=len(actual['profiles']),ego_roots=len(actual['ego_ids']),source_identities=len(actual['sources']),source_precision=actual['source_precision'],ego_projection='all75 narrow exploration counts and neighbor depth checked against prior full native Ego decoded images'),
  rust=dict(current_runtime='9/9 including actual source/map guards and list owners',navigation='4/4 (3 in-place +1 old wrapper)',baseline='8/8 earlier complete scoped snapshot; not substituted for current9'),
  explicit_remaining=['No completed actual full-map ActorStore startup replay yet','Positive Commander ability and nonempty RecruitSingle allocation references','Positive initialized-controller region/actor/definition notifications','Positive Actor154 SAIActor pointer owner/time store','Flare retirement,MARKER side effects,positive Wall container','Ready=true AddPlayer completes publication prefix then refuses unbound initialization/notifications','AI Think/fibers and application/mission activation seal not completed by this component'],
  errors_preserve='Native manager ready/revision/list prefixes, constructed AI allocations/controller/goals/scripts, region and manager list publications are not rolled back. Repeated binding attempts refuse instead of interpreting a partial ready byte as success.',
  game_files='No original archives,assets,saves,profiles,releases modified',artifacts=docs)
 out=HERE/'findings.json';out.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps(dict(path=str(out),sha256=H(out.read_bytes()),machines=total,fixtures=len(fixtures))))
if __name__=='__main__':main()
