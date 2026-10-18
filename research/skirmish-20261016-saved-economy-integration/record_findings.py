from pathlib import Path
import datetime, hashlib, json, zipfile
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
sources=[
'crates/aa-runtime/src/saved_economy_refresh.rs',
'crates/aa-runtime/src/saved_organization_actions.rs',
'crates/aa-runtime/src/economy_refresh.rs',
'crates/aa-runtime/src/saved_economy_values.rs',
'crates/aa-runtime/src/saved_economy_values_tests.rs',
'crates/aa-runtime/src/saved_kingdom_flows.rs',
'crates/aa-runtime/src/catalog.rs',
'crates/aa-runtime/src/restoration.rs',
'crates/aa-runtime/src/restore_lifecycle.rs',
'crates/aa-runtime/src/kingdom_economy.rs',
'crates/aa-economy/src/borrowed_flows.rs',
'crates/aa-actors/src/economy_values.rs',
'crates/aa-rules/src/native_recruit_backrefs.rs',
]
assert sha(ROOT/sources[0])=='ac82a36e8af525863a68a2556b0741f95924feee1a67e857f608533392f47226'
assert sha(ROOT/sources[1])=='abf5b2ba931f1eb996f475cb67557eb7e27f4a549d2dd47291805690e8d9b0ce'
prefix=lambda p:p.read_bytes().split(b'#[cfg(test)]\nmod tests')[0]
prior=HERE/'census-start-source-reconstructed.rs'
assert sha(prior)=='ab202df10e54b1a15b1385ca6caae57a0afe8dba0105fed8f1d8f3f02e30c478'
assert prefix(prior)==prefix(ROOT/sources[0])
natural=HERE/'actual-natural-refresh.json'
assert natural.read_bytes()==(HERE/'actual-natural-refresh-final.json').read_bytes()
snap=HERE/'frozen-source.zip'
with zipfile.ZipFile(snap,'w',compression=zipfile.ZIP_DEFLATED) as archive:
    for name in sources:archive.write(ROOT/name,name)
artifacts=['native_saved_refresh.py','native-initialized-refresh.json','native-natural-hq-refresh.json',
    'actual-initialized-input.json','actual-natural-refresh.json','actual-natural-refresh-final.json',
    'native-replay.log','final-native-recompare-tests.log','independent_replay.py',
    'independent-native-replay.log','independent-tests.log','independent-review.json',
    'census-start-source-reconstructed.rs','README.txt']
document={
 'schema':1,'recorded_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),
 'scope':'Bounded saved Economy ownership/value/flow integration and Catalog observer forwarding; no record Init/PostCreate or playable map claim.',
 'backup':'backups/source-before-saved-economy-integration-20261016.zip',
 'source_pins':{name:sha(ROOT/name) for name in sources},
 'artifact_pins':{name:sha(HERE/name) for name in artifacts},
 'snapshot_sha256':sha(snap),
 'census_start_full_source_sha256':sha(prior),
 'production_prefix_sha256':hashlib.sha256(prefix(prior)).hexdigest(),
 'freeze_note':'Exact reconstructed census-start full-file hash matches the independently recorded start pin; final production prefix is byte-identical. Only test additions/formatting differ.',
 'native_new':{'initialized_refresh':6,'natural_hq_refresh':12,'source_getter_executions':54,'precision':['Single24','Double53','Extended64'],'images':['original','v14']},
 'tests':{'focused_rust_passed':4,'independent_rust_passed':4,'independent_native_refresh':18,'independent_source_getter_executions':54},
 'natural_record_diagnostic':{'map':'Campaign/tutorial02','mode':1,'actors':46,'economy_absent':17,'economy_null_owner':0,'economy_nonnull_owner':29,'specific_missing_kingdom_1b8_0':27,'gated_success_actor_ids':[4100,8131],'Init_PostCreate_completed':False},
 'ownership_fixture':'Artificial link of two actual loaded Economy owners; parent failure keeps same Boxes and committed child Cost/parent Upkeep prefix. This is not a natural map relationship or recursive gameplay parity claim.',
 'remaining':['Immediate real Kingdom Init before actor Load is not bound; no missing arrays are synthesized.','Active SAI and existing-world teardown remain outside the inactive managed lifecycle.','Source preflight inherits existing valid-constructed-source limits.','Combined census, broad compatibility, release packaging and game/multiplayer tests belong to root and are not claimed by this subtask.'],
}
(HERE/'findings.json').write_text(json.dumps(document,indent=2)+'\n')
print(json.dumps({'findings_sha256':sha(HERE/'findings.json'),'snapshot_sha256':sha(snap),'source_pins':len(sources)}))
