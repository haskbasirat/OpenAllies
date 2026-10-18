"""Pin the saved successor, its native evidence and existing replay artifacts."""
from pathlib import Path
import hashlib,json,shutil
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
sources=[
 'crates/aa-runtime/src/stored_organization_init_world.rs',
 'crates/aa-runtime/src/organization_state_modes.rs',
 'crates/aa-runtime/src/stored_organization_members.rs',
 'crates/aa-runtime/src/saved_member_properties.rs',
 'crates/aa-runtime/src/map_actor_init_existing.rs',
 'crates/aa-runtime/src/stored_component_schedule.rs',
 'crates/aa-actors/src/organization_state.rs',
 'crates/aa-actors/src/organization_formation_fields.rs',
 'crates/aa-actors/src/component_virtuals.rs',
 'crates/aa-actors/tests/native_organization_formation_fields.rs',
]
snapshot=HERE/'source-v2';snapshot.mkdir(exist_ok=True)
for name in sources:
 target=snapshot/Path(name).name
 if target.exists():assert target.read_bytes()==(ROOT/name).read_bytes(),'different checkpoint already saved'
 else:shutil.copyfile(ROOT/name,target)
binary=ROOT/'target/s-state-binding-review/release/deps/native_organization_formation_fields-2b760c1801619067.exe'
frozen=HERE/'frozen-formation-fields-tests.exe'
if frozen.exists():assert frozen.read_bytes()==binary.read_bytes()
else:shutil.copyfile(binary,frozen)
files=sources+[str(p.relative_to(ROOT))for p in HERE.iterdir()if p.is_file()and p.suffix not in{'.pdb'}
 and p.name not in{'successor-freeze.json'}]
doc={
 'schema':1,'scope':'Organization57C870/57C450/579DD0 same-owner successor and shared formation fields',
 'status':'bounded source/native kernels tested; whole initialized Organization owner execution pending natural runner',
 'native':{'formation_executions':756,'formation_executed_pins':284,'formation_child_entries':8,
   'default_ctor_layout_executions':72,'default_address_byte_pins_including_transports':656,
   'enum_machines':6,'enum_ctor_calls':12,'enum_queries':108,'enum_executed_pins':355,'enum_transport_entries':3,
   'pins_not_deduplicated_across_fixtures':True},
 'tests':{'new_actor_fields':2,'prior_actor_regressions':4,'exact_enum_source':1,
   'all_native_producers':'fresh immutable PASS','runtime_owner_execution':0,'whole_runtime_links':0},
 'production':'successor-production-check04.log',
 'old_all_tests_check':'successor-tests-check02.log failed only saved_command_ownership.rs391/405 List.as_ptr successor; no own source error',
 'remaining':['actual member5709D0 attachment/terrain child','positive57BA80 current-leader permutation',
   'later positive property/state/diagnostic services not claimed by their existing owners'],
 'shared_regions':['only exact Organization InitExisting/After arms were added to shared map_actor_init_existing',
   'component_virtuals Organization coordinator and MaximumMoraleWorld factoring were inherited from WorldStartup draft',
   'old saved Organization public set_formation delegates same new field coordinator',
   'new member-module formation/lookup helpers are a successor to its separately frozen v1'],
 'source_snapshots':{str(p.relative_to(ROOT)):sha(p)for p in snapshot.iterdir()},
 'files':{name:sha(ROOT/name)for name in sorted(set(files))},
}
(HERE/'successor-freeze.json').write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps({'sha256':sha(HERE/'successor-freeze.json'),'files':len(doc['files'])}))
