"""Seal the bounded detection-memory owner/binding evidence after replay."""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 fixture=json.loads((HERE/'native-memory.json').read_text())
 assert fixture['paired_rows']==249 and fixture['native_executions']==498
 native=json.loads((HERE/'author-native-replay.log').read_text(encoding='utf-8-sig'))
 assert native==dict(rows=249,runs=498,pins=389,sha256=sha(HERE/'native-memory.json'))
 for name,count in [('author-rust-replay.log',2),('author-actual-immutable.log',1),('author-actor-constructor-regressions-final.log',1)]:
  assert f'{count} passed; 0 failed;' in (HERE/name).read_text(encoding='utf-8-sig')
 actual=json.loads((HERE/'actual-first-object-entry.json').read_text())
 source=['crates/aa-actors/src/detection_memory.rs','crates/aa-runtime/src/load_detection_memory.rs',
 'crates/aa-runtime/src/load_detection_memory_tests.rs']
 evidence=['native_memory.py','native-memory.json','actual-first-object-entry.json','README.md',
 'author-native-replay.log','author-rust-replay.log','author-actual-immutable.log',
 'author-actual-capture-object-final.log','author-actor-constructor-regressions-final.log']
 files=source+[str((HERE/n).relative_to(ROOT)).replace('\\','/') for n in evidence]
 report=dict(schema=1,status='bounded_detection_memory_author_accepted',
 original_sha256='5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26',
 v14_sha256='3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1',
 native_paired_rows=249,native_executions=498,executed_instruction_pins=389,static_vtable_pins=18,
 focused_rust_tests_passed=2,actual_immediate_entry_tests_passed=1,constructor_integration_tests_passed=2,
 functions=['4EBC70 refresh','4EA3E0 slot allocation','4EB9F0 record constructor',
 '4ECA80/4EC4A0 creation prefix','4EB970 deletion','4EBB80 clearing'],
 native_seams=['refresh fixtures explicitly provide eligibility and create/delete/clear virtual effects',
 'allocator/free and empty-name destructor transport','creation stops at the real virtual24 name getter after timestamp store'],
 ownership=['one Object+58/+5C typed count/allocation/record owner moved from shell into runtime',
 'actual persistent major-Kingdom vector including null holes','same held Object and current Kingdom190/18C/228',
 'actual managed observer presence/active state','actual Catalog load begin clock'],
 actual_entry=['original tutorial02, actual scoped source completion and Session borrow, real Attempt::load_immediate',
 'GTID28/class9 temperate_pine_trees; majors [781,782,null]; count0/no allocation before and after'],
 boundaries=['No complete memory refresh/name/aspect/stat/UI observer implementation is claimed.',
 'Positive creation retains allocation and timestamp then refuses the reached missing Object virtual24 service.',
 'Actor/Kingdom14C wrapper reads Structure container and refuses reached Wall/Settlement dependencies; its extra wrapper is static-pinned, not separately dynamically proved here.',
 'Actual capture is a bounded startup entry, not a full map/battle or full native Session constructor proof.',
 'Record+24/+28 remain explicitly unwritten after constructor; live handles are never native pointers.',
 'No allocator exhaustion, invalid-pointer parity, concurrent reentrancy or complete map/battle/multiplayer claim.'],
 files={n:sha(ROOT/n) for n in sorted(files)})
 path=HERE/'findings.json';path.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(dict(status=report['status'],sha256=sha(path),production={n:sha(ROOT/n) for n in source})))
if __name__=='__main__':main()
