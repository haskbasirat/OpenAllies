"""Freeze this additive view-init/source/render transport checkpoint only."""
from pathlib import Path
import hashlib,json,shutil
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
SOURCES=[
 'crates/aa-client/src/view_directory.rs','crates/aa-client/src/view_object_binding.rs',
 'crates/aa-client/src/view_request_services.rs','crates/aa-client/src/view_actor_init.rs',
 'crates/aa-client/src/view_actor_fields.rs','crates/aa-client/src/live_spawn.rs',
 'crates/aa-client/src/model_fixes.rs','crates/aa-client/src/loading.rs',
 'crates/aa-ui/src/application_random.rs','crates/aa-ui/src/ui.rs',
 'crates/aa-ui/src/lib.rs','crates/aa-ui/Cargo.toml','crates/aa-client/src/frontend.rs',
 'crates/aa-runtime/src/data_actor_view_source.rs','crates/aa-runtime/src/data_actor_view_source_tests.rs',
 'crates/aa-runtime/src/view_replacement_color.rs','crates/aa-runtime/src/view_startup_inputs.rs',
]
PROOFS=[
 'native-object-init.json','native-actor-init.json','native-actor-source.json',
 'native-replacement-color.json','native-ui-random.json','native-application-random.json',
 'white-global-audit.json','ui-global-random-audit.json',
 'native_object_init.py','native_actor_init.py','native_actor_source.py',
 'native_replacement_color.py','native_ui_random.py','native_application_random.py',
 'audit_white_globals.py','audit_ui_global_random.py',
 'current-tests-check05.log','actor-kernel-tests03.log','kernel-tests03.log',
 'application-random-tests01.log','model-fixes-tests01.log','model-fixes-actual01.log',
 'replay-application-random-final.log','replay-replacement-color-final.log',
 'actor-kernel-tests.exe','model-fixes-tests.exe',
]
EXTERNAL=[
 'research/skirmish-20261017-application-rng-lead/producer-lead.json',
 'research/skirmish-20261017-application-rng-lead/vworld-consumer-slot.json',
 'research/skirmish-20261017-application-rng-lead/direct-parent-order.json',
]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
 snapshot=HERE/'checkpoint01-sources'
 if snapshot.exists():raise RuntimeError('immutable snapshot already exists')
 sources={}
 for name in SOURCES:
  source=ROOT/name;destination=snapshot/name
  destination.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(source,destination)
  sources[name]=sha(source);assert sha(destination)==sources[name]
 proofs={str(path.relative_to(ROOT)).replace('\\','/'):sha(path)for path in [*(HERE/p for p in PROOFS),*(ROOT/p for p in EXTERNAL)]}
 document=dict(schema=1,
  scope='Additive native base/Actor Init and same-Data source binding; strict selected-model render transport; concrete fresh offline App/UI RNG interval. No complete VWorld Init/AD/74FF50 or playable claim.',
  sources=sources,proofs=proofs,
  results=[
   'Client current --tests check05 passed51.08s; later RNG tests compiled independently in aa-ui.',
   'VObject exact production-source kernel648 native rows passed1/1.',
   'Actor/Organization exact production-source kernel456 native rows passed1/1; positive Track cutoff retains prefix.',
   'Application ctor/time32/UI reseed native108 rows across both images/all3PC and54 executed pins passed immutable replay.',
   'Actual OS clock/native108-stream/actual UI owner tests3/3 passed6.35s.',
   'Exact model_fixes production-source11 regressions passed0.30s, selected input/output hash and same-read mutation refusal included.',
   'Actual3 original models passed strict selected paired-artifact mapping1/1 in0.10s; no GPU receipt or native Model loader flag claimed.',
   'ReplacementColorDefault native30 rows/143 pins passed immutable replay; current typed source actual test pending root runtime cohort.',
   'Actor source actual completed Bank test pending root runtime cohort.',
  ],
  boundaries=[
   'Whole shared files are snapshots for review, not an ownership or independent acceptance claim for peer regions.',
   'App/UI RNG ordering accepted only for the mapped offline fresh startup interval; full transitive callback/thread/alias closure excluded.',
   'Actual UTC time32 input is queried once by production; native fixture uses labelled OS-return inputs and explicit prior-draw laboratories.',
   'Other global random consumers must use the same retained owner when ported; no reseed per Actor or VWorld.',
   'Source Data request preparation remains pre-World; a missing preparation receipt errors after native Data20 store.',
   'Track42D7B0/42D830, positive radius4306C0, unsupported source children and false-Init C4 remain required.',
   'Selected model uses current native7C/B4 and actual retained SourceFile bytes, paired input/output/manifest/node mapping; no filename-only new digest.',
   'GPU receipt does not manufacture native Init/list/publication, model metadata/load flags, AD, activation or readiness.',
  ])
 out=HERE/'checkpoint01.json';out.write_text(json.dumps(document,indent=2)+'\n')
 print(json.dumps(dict(path=str(out),sha256=sha(out),source_count=len(sources),proof_count=len(proofs))))
if __name__=='__main__':main()
