from pathlib import Path
import hashlib,json,shutil
folder=Path(__file__).resolve().parent;root=folder.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
source=json.loads((folder/'bundle5-source-pins.json').read_text())
for path,expected in source.items():assert sha(root/path)==expected
before=json.loads((folder/'bundle-extra-target-before.json').read_text());after=json.loads((folder/'bundle5-target-after.json').read_text());assert before==after
snapshot=folder/'bundle5-source-snapshot';snapshot.mkdir(exist_ok=True)
for path in source:shutil.copyfile(root/path,snapshot/Path(path).name)
shutil.copyfile(root/'crates/aa-runtime/src/world_ready_services.rs',snapshot/'world_ready_services.rs')
report={
 'result':'PASS concrete five-owner source factory and retained-prefix replay; production check passes',
 'source_pins':source,'application_source_sha256':sha(root/'crates/aa-runtime/src/world_ready_services.rs'),
 'factory':['SAME sealed Seeds','current Grid Int cells','current RendererDefaultOrientation Point3 cell','current17 disjoint Camera AVar cells','current23 disjoint view-asset CString cells, including TextureEnvironmentMap'],
 'no_new_current_cells':True,
 'actual_replay':{'binary':sha(folder/'bundle5-factory-probe.exe'),'log':sha(folder/'bundle5-actual-02.log'),'harness':sha(folder/'bundle5_factory_harness.rs'),
  'assertions':['requires completed real source Bank before member construction','real source current8/4 and Point3[0,-1,0]','retains same Grid partial current8 when next declaration check fails','late Alias source/projection error retains all prior owners and incomplete Alias','current v14 gameplay zoom2.1 and current source aliases','actual sealed seed constructor writes14/18zero but leaves28unknown'],
  'mechanical_extractions':['exact crate-private require_reference_numeric guard','exact pure native hash/indices/public wrapper (old compiled aa_rules predates new export)','current Settings five-file constant/load_v14/collect bodies, with return type rewritten to existing actual Settings and Self::from_files renamed accordingly','factory/prefix portion of current production unit test; Application section is excluded'],
  'no_stubbed_checks_or_services':True,'dependency_artifacts':len(before),'all_dependency_hashes_unchanged':True},
 'extra_component_replay':{'log':sha(folder/'bundle-extra-actual-03.log'),'scope':'current Camera v14zoom/read-only lazy gate/clamp; all22 aliases vs actual Settings; counted Alias survives rawUTF16 same-cell setter; SAME Bank move'},
 'production_check':{'result':'pass','seconds':30.74,'log':sha(folder/'bundle5-production-check-01.log')},
 'application':['optional private Inputs installed exactly once','duplicate/error returns incoming Inputs owner whole','dedicated disjoint loan of SAME Loading and Inputs; old Parts unchanged','actual installation/duplicate assertions saved in ignored unit test, not yet executed'],
 'native_composition_claim':'None. Existing component-native proofs stay separate; composition only moves and validates those owners.',
 'history':['Three-member checkpoint retained in bundle3-actual-findings.json/snapshot','Failed CameraHeight required-diagnostic probe retained','Native flags2 diagnostic continuation fixed by camera author; subsequent missing public row exposed Settings4 omission','Root corrected actual Settings source projection to includeAVars_AA; current five-file source accepted without relaxed guards'],
 'gdata630_static_receipt_sha256':sha(folder/'gdata630-static-receipt.json'),
 'remaining':['Application install execution','concrete VWorld startup and resource children','camera/rendering/native input/picking or playable battle acceptance'],
}
(folder/'bundle5-findings.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'report':sha(folder/'bundle5-findings.json'),'source':source},indent=2))
