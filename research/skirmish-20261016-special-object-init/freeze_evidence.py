"""Seal the bounded special Object Init evidence after author tests pass."""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 source='crates/aa-runtime/src/load_special_object_init.rs';test='crates/aa-runtime/src/load_special_object_init_tests.rs'
 assert sha(ROOT/source)=='8754def15a6d1fd67882638e77de01ca283e4d22cfe9595efa0c6cab422f2792'
 assert sha(ROOT/test)=='9c8de1ccbaebeb705397b7706ea9bbb5b8e333eb72b151d8796ebb730b73d994'
 assert '4 passed; 0 failed;' in (HERE/'author-rust-tests-lib-final.log').read_text(encoding='utf-8-sig')
 replay=json.loads((HERE/'author-native-replay.json').read_text());assert replay['status']=='passed' and replay['native_executions']==708
 fixtures=[json.loads((HERE/('native-'+name+'.json')).read_text()) for name in ('special-classes','special-tail')]
 pins={i['va']:i['bytes'] for fixture in fixtures for i in fixture['instructions']}
 files=[source,test]+[str(p.relative_to(ROOT)).replace('\\','/') for p in HERE.iterdir()
  if p.is_file() and p.name in ['native_special_classes.py','native_special_tail.py','native-special-classes.json','native-special-tail.json',
      'author-native-replay.json','author-rust-tests-lib-final.log','replay_native.py','README.md']]
 report=dict(schema=1,status='bounded_special_object_init_author_accepted',
  original_sha256='5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26',
  v14_sha256='3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1',
  native_executions=708,unique_executed_instruction_pins=len(pins),rust_tests_passed=4,
  functions=['Area4DB200','Camera4DBCF0','Doodad4DC730','Disturbable5904A0','Breakable58F960','Updatable50ACC0/50B250'],
  native_seams=['baseObject4EBE50 success/failure','baseObject constructor4EA690','empty-string/successful node allocation/free',
      'Session phase76AC90','diagnostic logger77DA70 transport','deferred bucket allocation callback; current owner links run unchanged'],
  ownership=['same held derived payload','same World camera/area generation owner','same NativeRng stream',
      'same Catalog load Queues and Owners moved into staged runtime; no head mirror/copyback'],
  reached_errors=['legacy pre-v22 Area schedule','missing actual source/slot definition','missing real queue owner','queue/live precision mismatch'],
  boundaries=['Full Object source/placement/spatial services have separate native evidence, not re-proven by the named base seam.',
      'No complete map/playable battle/multiplayer claim follows from these bounded Init proofs.',
      'Source canonical-owner/slot checks are explicit host validity boundaries, not native invalid-pointer parity.',
      'Finite period arithmetic covered including zero/subnormal/negative/max-finite; unsupported numeric values remain explicit errors.',
      'No allocator exhaustion, corruption or concurrent reentrancy claim. Original game/release files untouched.'],
  files={name:sha(ROOT/name) for name in sorted(files)})
 path=HERE/'findings.json';path.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(dict(status=report['status'],sha256=sha(path))))
if __name__=='__main__':main()
