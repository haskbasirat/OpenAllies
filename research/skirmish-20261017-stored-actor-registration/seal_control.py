from pathlib import Path
import hashlib,json,shutil
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
snapshot=HERE/'source-control-v1';snapshot.mkdir(exist_ok=True)
paths=[
 'crates/aa-zones/src/zone.rs','crates/aa-zones/tests/native_enemy_entry_fields.rs',
 'crates/aa-runtime/src/stored_registration_control.rs','crates/aa-runtime/src/stored_actor_registration.rs',
 'crates/aa-runtime/src/stored_actor_registration_tests.rs','crates/aa-runtime/src/load_zone_entry.rs',
]
sources={}
for name in paths:
 p=ROOT/name;q=snapshot/name;q.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(p,q);sources[name]=sha(p)
proof=json.loads((HERE/'native-control-entry.json').read_text())
binary=ROOT/'target/s-state-binding-review/release/deps/native_enemy_entry_fields-7f8af273f196c406.exe'
frozen=HERE/'frozen-control-fields-tests.exe';shutil.copyfile(binary,frozen)
evidence={name:sha(HERE/name)for name in[
 'native_control_entry.py','native-control-entry.json','control-native-generate02.log',
 'control-native-replay01.log','control-zone-regressions02.log','control-production-check02.log',
 'control-all-tests-check03.log','frozen-control-fields-tests.exe',
]}
doc=dict(schema=1,status='bounded source/kernel/native checkpoint; actual44 pending',sources=sources,evidence=evidence,
 native=dict(rows=len(proof['rows']),separate_machine_executions=2*len(proof['rows']),
  executed_address_pins=len(proof['executed_pins']),intercepted_address_pins=len(proof['intercepted_pins']),
  null_fault_address_pins=len(proof['null_fault_pins']),
  limits='Owner108/enemy140/component implementations are explicit transports. Native getter/guard/redirect/null/child order executes. Lab inputs are not mission values.'),
 validation=dict(rust='3/3 aa-zones tests PASS: new72-row field replay and existing full entered/tile regressions',
  production='aa-runtime --lib release --locked --offline PASS17.77s before whitespace-only new-module formatting',
  all_runtime_tests='Source check03 blocked by foreign in-flight view_ui_world_tests.rs missing; no registration error reported; no runtime unit execution or full link performed',
  source_review='WorldStartup independently scanned native and shared kernel order; Globals inspected actual actor/Bank/zone/relation loans, zero fresh reviewer executions'),
 behavior='Published registration forwards actual Entry.actor and entering ActorId, reads same Zone4 and current placement/source/Kingdom/Team fields through native guard order. No stored Actor/Zone/Bank clone, detach, callback default or assumed self-zone.',
 limitations=['Positive enemy Guard/Organization/Defender bodies remain contextual required after their null checks.',
  'Slot5, slot3, city and discovery registration retain their prior reached required boundaries.',
  'Source/generation/constructor/index checks are portable ownership guards, not invalid-native-pointer parity.',
  'No new actual mission/startup/battle completion claim; natural44 is a separate evidence window.'],
 replay=['python research/skirmish-20261017-stored-actor-registration/native_control_entry.py',
  'research/skirmish-20261017-stored-actor-registration/frozen-control-fields-tests.exe --nocapture'])
p=HERE/'control-freeze.json';p.write_text(json.dumps(doc,indent=2)+'\n');print(str(p),sha(p))
if __name__=='__main__':pass
