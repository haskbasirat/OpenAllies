from pathlib import Path
import hashlib,json,shutil
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
if (HERE/'findings.json').exists():raise SystemExit('Checkpoint already sealed; preserve historical bytes.')
native=json.loads((HERE/'native-member-deployment.json').read_text())
assert [len(native[k])for k in ['rows','source_rows','constructor_rows','shape_rows','instructions']]==[168,12,3,648,317]
assert native['producer_sha256']==sha(HERE/'native_member_deployment.py')
assert '2 passed; 0 failed' in(HERE/'exact-tests02.log').read_text()
assert 'Finished `release`' in(HERE/'check-tests02.log').read_text()
source=[
 'crates/aa-runtime/src/stored_member_deployment.rs',
 'crates/aa-runtime/src/member_deployment_fields.rs',
 'crates/aa-runtime/src/member_deployment_fields_tests.rs',
 'crates/aa-runtime/src/stored_member_deployment_tests.rs',
]
snap=HERE/'source-checkpoint';snap.mkdir();source_records=[]
for name in source:
 p=ROOT/name;q=snap/name;q.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(p,q)
 source_records.append(dict(path=name,snapshot=str(q.relative_to(HERE)).replace('\\','/'),sha256=sha(p)))
exports=[v for v in(ROOT/'crates/aa-runtime/src/lib.rs').read_text().splitlines()if'pub mod stored_member_deployment;'in v]
assert len(exports)==1;(snap/'lib-export.txt').write_text(exports[0]+'\n')
artifacts=[
 'native_member_deployment.py','native-member-deployment.json','native-write.log','native-replay.log',
 'native_actor_shape_getter.py','native-actor-shape-getter.json','native-actor-shape-write.log','native-actor-shape-replay.log',
 'exact-fields-tests.rs','exact-fields-tests.exe','exact-build02.log','exact-tests02.log',
 'actual-source-census.rs','actual-source-census.exe','actual-source-census.json','actual-source-census.log','actual-source-census-build02.log',
 'check-lib.log','check-tests02.log','native-actor-ctor.txt','README.md','seal_checkpoint.py',
]
external=[
 'crates/aa-rules/src/native_recruit_backrefs.rs','crates/aa-rules/tests/native_recruit_backrefs.rs',
 'research/skirmish-20261016-recruit-backrefs/native-recruit-backrefs.json',
 'research/skirmish-20261016-recruit-backrefs/native-real-recruit-backrefs.json',
 'research/skirmish-20261017-map-actor-creation/native-constructor.json',
 '../reconstruction/assets/original/Data/Templates/template_deployment.tgi',
 '../reconstruction/assets/original/Data/Templates/Components/template_denizen.tgi',
]
doc={
 'schema':1,'status':'author portable/source checkpoint; independent review and actual same-Bank runtime test pending',
 'scope':'Native4FACA0 same mutable ActorParams and exact finite transforms/order; completed canonical source3DC..3EC projection; reuse existing completed17C arena.',
 'native':{'deployment_rows':168,'typed_source_rows':12,'constructor_fragment_rows':3,'shape_rows':648,'executions':1662,'executed_address_byte_pins':317,
 'explicit_children':['actor virtualC/46EC70 outcome in coordinator (separate full predicate proof)','4F9C10 creation','current new-member Mind198','404ED0 warning','404F60 failed log','4F6760 destruction','5B4D80 typed point getter service'],
 'constructor_qualification':'462A81..99 only, requested FPCW and enclosing constructor EBX=0 explicit; not whole constructor execution'},
 'getter':{'executions':6,'executed_address_byte_pins':3,'static_pins':3,'scope':'class3 Actor virtualC only; ctor storesA273F0, tableC4EA010 returns currentData+38; predecessor fresh-ctor executed pin cited'},
 'rust':{'exact_production_core_tests':'2/2 pass','scoped_runtime_lib_tests_typecheck':'pass54.95sec','new_runtime_unit_link':False,'actual_runtime_source_test':'saved ignored, typechecked only'},
 'actual_source_census':{'precisions':'three explicit lab PCs; no startupPC authority','canonical_actordata':1028,'deployment_present':40,'deployment_absent':988,'typed_success':39,'typed_required_error':'usa_hq_infantry creation_offset contains1.5.0; strict scalar parser refuses; native tokenizer proof required before change'},
 'not_claimed':['initialized Staged World','natural mission deployment acceptance','actual positive creation/Mind/destructor child success','whole ActorData LoadData/partial source allocation owner','new17C algorithm or parser change','nonfinite arithmetic domain'],
 'source':source_records,
 'artifacts':[dict(path=n,sha256=sha(HERE/n))for n in artifacts],
 'external_evidence':[dict(path=n,sha256=sha(ROOT/n))for n in external],
}
(HERE/'findings.json').write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps(dict(findings_sha256=sha(HERE/'findings.json'),source_pins=len(source_records),artifact_pins=len(artifacts),external_pins=len(external))))
