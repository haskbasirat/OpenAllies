"""Seal the bounded CREATED CREATE_VFX graph/event proof."""
from pathlib import Path
import json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 replay=json.loads((HERE/'author-native-replay.json').read_text());assert replay['status']=='passed' and replay['native_executions']==528
 assert '2 passed; 0 failed;' in (HERE/'author-core-replay-final.log').read_text(encoding='utf-8-sig')
 assert '1 passed; 0 failed;' in (HERE/'author-source-capture-02.log').read_text(encoding='utf-8-sig')
 actual=json.loads((HERE/'actual-source-inputs.json').read_text())
 assert len(actual['results'])==582 and sum(r['error'] is None for r in actual['results'])==312
 assert len(actual['tutorial02_created'])==54 and all(r['error'] is None for r in actual['tutorial02_created'])
 assert len(set(r['id'] for r in actual['tutorial02_created']))==8
 fixtures=[json.loads((HERE/('native-'+name+'.json')).read_text()) for name in ['fx-graph','fx-command','bone-ordinal','fx-enums']]
 pins={i['va']:i['bytes'] for fixture in fixtures for i in fixture['instructions']}
 files=['crates/aa-runtime/src/fx_graph.rs','crates/aa-runtime/src/fx_graph_tests.rs']
 files+=[str(p.relative_to(ROOT)).replace('\\','/') for p in HERE.iterdir() if p.is_file() and p.name in [
 'native_fx_graph.py','native_fx_command.py','native_bone_ordinal.py','native_fx_enums.py',
 'native-fx-graph.json','native-fx-command.json','native-bone-ordinal.json','native-fx-enums.json',
 'actual-source-inputs.json','author-native-replay.json','author-core-replay-final.log','author-source-capture-02.log','README.md','replay_native.py']]
 report=dict(schema=1,status='bounded_created_fx_graph_and_constructor_author_accepted',
 original_sha256='5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26',
 v14_sha256='3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1',
 native_paired_rows=264,native_executions=528,unique_executed_instruction_pins=len(pins),rust_tests_passed=2,
 actual_source_census_tests_passed=1,actual_created_source_definitions=582,completed_create_vfx_graphs=312,
 tutorial02_created_instances=54,tutorial02_created_unique_graphs=8,tutorial02_source_compilation_errors=0,
 scope=['Created list only:42BF10/42B990,42AEF0,42C1B0,42C5E0',
 '4296B0/428F40/428480/624100(NULL) event construction; no effect execution',
 '486650/486870/486980/93D4E0/47B400 Bone ordinal source producer',
 '497A40/497E20/42BF70 enum-registering callers'],
 native_seams=['typed source getters, enums, current registry answers, allocation and string transport',
 'command fixture current Thing virtuals, Win32 thread IDs and selected clocks',
 'Bone unique-ID null Find, hash/string factory and unused reserve-element lifecycle',
 'enum insertion transport; caller registrations run unchanged'],
 boundaries=['Full source census still requires162 PLAY_SFX,63 EQUIPMENT,45 CREATE_RFX loaders.',
 'Only Created lists share the correct initially empty first-VFX reference context; other lists refuse.',
 'Non-finite getter/point inputs remain explicit unproved domains.',
 'Positive predefined ActorState ordinal source is required; no guessed ordinal.',
 'Native source context completion and runtime queue submission are distinct from compiled graph ownership.',
 'Live captured App/current OS thread identities and selected View/World clock must be supplied; no role or PC inference.',
 'No renderer/audio execution, complete battle/multiplayer, allocator failure or reentrancy claim. Original/release files untouched.'],
 files={name:sha(ROOT/name) for name in sorted(files)})
 path=HERE/'findings.json';path.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(dict(status=report['status'],sha256=sha(path),pins=len(pins),source=sha(ROOT/files[0]),test=sha(ROOT/files[1]))))
if __name__=='__main__':main()
