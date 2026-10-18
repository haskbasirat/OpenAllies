from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
paths=[ROOT/'crates/aa-runtime/src/map_denizen_init_create.rs',ROOT/'crates/aa-actors/tests/native_denizen_init_create.rs']
paths += [HERE/n for n in ['native_denizen.py','native-denizen-init-create.json','audit_denizen.py','pin-audit.json','actors-tests-01.log','README.md','seal.py']]
shared=[ROOT/'crates/aa-actors/src/component_virtuals.rs',ROOT/'crates/aa-runtime/src/map_component_init_create.rs',ROOT/'crates/aa-runtime/src/lib.rs']
predecessor=ROOT/'research/development10/playable-startup/attempt25.fresh-actor-prefix.json'
assert H(predecessor)=='0e35d35c5f2d0c50aa0d7ac5679f0234f736061ec097a8e522d6ce0deb87ffa4'
doc=dict(schema=1,status='bounded native kernel and affected Rust tests passed; next actual map advancement pending',
 native=dict(paired_rows=195,executions=390,precision=['Single24','Double53','Extended64'],decoded_equal_executed_pins=53,required_read_queue_cutoffs=8,actual_child='45D730 null-context target health',seam='50AC60 queue boundary; live implementation uses existing actual queues/RNG'),
 rust=dict(passed_tests=['new native_denizen_init_create 195-row replay','existing native_actor_creation_leaves full fixture'],production_check='No owned errors reported; shared check02 failed resource_sound Entry migration, check03 failed audio_data_pre_request result annotation. No complete production check claimed yet.'),
 ownership='same Staged ActorStore/Denizen Box, Catalog binding allocation checked against moved Bank, actual stable Inventory and canonical target health, same queue owner/Queues/RNG; no native gameplay defaults',
 reached_predecessor=dict(path=predecessor.relative_to(ROOT).as_posix(),sha256=H(predecessor),gtid=1688,key=2362,definition='ger_corps',phase='AwaitingInitCreate',scope='actual diagnostic predecessor, never constructor/source input'),
 source_producer='3A4 constructor-zero at4629F8, overridden only by optional DenizenComponent/Runway.delay at4675A8; source PC independent of live PC.35C tests source Inventory-node head, not constructed slot count.',
 limitations=['Stable valid typed Inventory links and source owners during direct null-context getter','Finite target health; no signallingNaN CPU exception equivalence','Native pointer/invalid-link/unwind behavior excluded; portable source/owner preflight explicit','No whole fresh Actor PostCreate/publication/StartedMission/activation or playable-battle certificate'],
 pins={p.relative_to(ROOT).as_posix():H(p)for p in paths},
 shared_source_snapshots={p.relative_to(ROOT).as_posix():H(p)for p in shared})
data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'findings.json'
if '--write'in sys.argv:p.write_bytes(data)
else:assert p.read_bytes()==data
print(json.dumps(dict(sha256=hashlib.sha256(data).hexdigest(),pins=len(paths),shared_snapshots=len(shared))))
