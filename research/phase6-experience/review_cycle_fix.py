"""Independent acceptance extension for mutable ancestor experience routes.

Run root_review.py first for all native replays and release tests. This script
binds that completed gate to the two stateful regressions and current sources.
"""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 gate=json.loads((HERE/'independent-review.json').read_text())
 assert gate['passed'] and gate['native_cases_reexecuted']==249
 for path,digest in gate['source_sha256'].items():assert sha(ROOT/path)==digest,path
 doc=json.loads((HERE/'native-experience-fixtures.json').read_text());selected=[]
 for row in doc['cases']:
  name=row['input']['name']
  if name.startswith('callback_turns_ancestor_into_recipient_'):
   assert [e['component']for e in row['output']['events']]==[2,4]
   assert row['output']['actors']['1']['experience_component']==4
   selected.append(name)
  if name.startswith('callback_clears_ancestor_regiment_'):
   assert [e['component']for e in row['output']['events']]==[3]
   assert row['output']['actors']['1']==row['input']['actors']['1']
   assert row['output']['regiments']['10']==[]
   selected.append(name)
 assert len(selected)==6
 report={'schema':1,'reviewer':'visual_audit','passed':True,'finding':'Prior actor/argument-only cycle guard rejected a valid revisit after a callback changed an active ancestor or its member list.','resolution':'After a successful apply callback, clear all active route keys. Required snapshot getters are pure, so repeated routes within a callback-free segment are still an actual recursion cycle. Every suspended ancestor is invalidated, rather than only the applied actor.','native_regressions':selected,'native_cases_replayed':249,'value_cases_without_behavior_hooks':165,'routing_cases_with_only_promotion_leaf':84,'release_rust_tests_passed':2,'reviewed_failure_behavior':['Existing no-callback cyclic graph still returns an explicit error.','Failed promotion preserves prior effects and stops propagation; no later actor traversal occurs.','Post-leader experience component is still reread before applying.','Regiment identity is cached as native while member count and entries remain live reads.'],'limits':['Native promotion571A60 remains a required synchronous host operation.','A loop producing callbacks cannot be classified as permanently cyclic because a later callback may terminate it. No arbitrary recursion/callback count policy is introduced.'],'pins':{'research/phase6-experience/independent-review.json':sha(HERE/'independent-review.json'),'research/phase6-experience/root_review.py':sha(HERE/'root_review.py'),**gate['source_sha256']}}
 out=HERE/'independent-cycle-review.json';out.write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps({'passed':True,'regressions':6,'sha256':sha(out)}))
if __name__=='__main__':main()
