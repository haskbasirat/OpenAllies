"""Record the narrowly executed three-member actual source composition."""
from pathlib import Path
import hashlib,json,shutil
folder=Path(__file__).resolve().parent;root=folder.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
before=json.loads((folder/'bundle-probe-target-before.json').read_text());after=json.loads((folder/'bundle-probe-target-after.json').read_text())
assert before==after
pins=json.loads((folder/'bundle-probe-source-pins.json').read_text())
snapshot=folder/'bundle3-source-snapshot';snapshot.mkdir(exist_ok=True)
for name,expected in pins.items():
 if name=='private_guard_mechanical_slice_sha256':continue
 p=root/name;assert sha(p)==expected;shutil.copyfile(p,snapshot/p.name)
for name in ['view_startup_inputs_tests.rs','world_ready_services.rs']:
 shutil.copyfile(root/'crates/aa-runtime/src'/name,snapshot/name)
report={
 'result':'PASS actual three-member source factory; Application install remains compiled-only',
 'scope':'Real actual V14Sources/ActorCatalog/Settings/verified source receipt/completed source Bank; current production component sources are path-included in a small standalone main',
 'source_pins':pins,
 'execution':{
  'assertions':['Incomplete Bank refuses before any member construction','Same reached TerrainBlockSize=8 survives later VGridCellSize source-binding error','Incomplete retained Grid refuses validated use; prior Seeds retained, unreached Orientation absent','Complete actual Int current values8/4, exact current Point3[0,-1,0]','Actual sealed seed owner runs real Rust VGrid constructor; writes14/18zero and leaves28unwritten','Unknown profile/console source owner refuses before member construction'],
  'private_guard':'Exact mechanical require_reference_numeric function slice from production; no substitute/stub',
  'existing_runtime_dependency':'target/native-runtime/release/deps/libaa_runtime-4004dde273efdec2.rlib (root netcohort47)',
  'target_artifact_count':len(before),'target_artifacts_unchanged':True,
  'binary_sha256':sha(folder/'bundle-factory-probe.exe'),
 },
 'evidence':{p:sha(folder/p)for p in ['build_bundle_probe.py','bundle_factory_harness.rs','bundle-probe-build-05.log','bundle-probe-actual-01.log','bundle-probe-target-before.json','bundle-probe-target-after.json']},
 'not_executed':['Application optional install/duplicate refusal/disjoint Loading loan (production compiled; ignored current unit test saved)','Camera/alias future bundle members','VWorld activation, native view/rendering or playable battle'],
 'history':'Prior failed target-dependency/private-old-API compilations preserved in build01..03; build04 omitted only harness world_creator_extents reexport. No production guard changed to make those attempts compile.',
}
(folder/'bundle3-actual-findings.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'sha256':sha(folder/'bundle3-actual-findings.json'),'source_pins':pins},indent=2))
