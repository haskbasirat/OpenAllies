"""Seal only successful final immutable checks and the owned frozen sources."""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
 source='crates/aa-runtime/src/load_kingdom_init_world.rs';test='crates/aa-runtime/src/load_kingdom_init_world_tests.rs'
 assert sha(ROOT/source)=='323216cda3cdb29adfa687fbc452a2f1a5a419052eaedd0f4adaa18d7434b070'
 assert sha(ROOT/test)=='2b71417d594c115d7883f0ae8d1665451a048b0173acf8bbb52d956ecb3132c8'
 replay=json.loads((HERE/'author-native-replay.json').read_text());assert replay['status']=='passed' and replay['native_runs']==324
 rows=json.loads((HERE/'actual-first-kingdom-final.json').read_text())['rows'];assert len(rows)==3
 for row in rows:
  assert row['outcome']=={'Ok':True} and len(row['definitions'])==354 and len(row['recruit_items']['entries'])==169
  assert row['source_precision']=='Double53' and row['same_held_allocation'] and row['object_init_calls']==1
 logs={}
 for name,count in [('author-final-capture.log',4),('author-final-immutable-rust.log',4),('author-special-state.log',2)]:
  raw=(HERE/name).read_text(encoding='utf-8-sig');assert f'{count} passed; 0 failed;' in raw
  logs[name]=dict(passed=count,sha256=sha(HERE/name))
 owned=[source,test]+[str(p.relative_to(ROOT)).replace('\\','/') for p in HERE.iterdir()
  if p.is_file() and (p.name.startswith('native_') or p.name.startswith('native-') or p.name in
      ['README.md','actual-first-kingdom-final.json','author-native-replay.json','replay_native.py'])]
 report=dict(schema=1,status='bounded_concrete_kingdom_team_init_author_accepted',
  original_sha256='5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26',
  v14_sha256='3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1',
  native_executions=324,unique_executed_instruction_pins=1004,source_precision='Double53',
  type_only_addendum=dict(previous_source_sha256='65830ec59d52ab65012dc239f6117b21061b158018a4910e2843e9fd8848021b',
      change='TeamWorld Catalog asset lifetime is independent of its short mutable reborrow; no executable behavior changed.',
      compiled_composed_root_observation='research/development10/immediate-world-05.json',
      observation_sha256=sha(ROOT/'research/development10/immediate-world-05.json')),
  live_precision=[v['live_precision'] for v in rows],actual_kingdom=781,
  actual_map_version=27,actual_recruit_source_definitions=354,actual_plain_recruits_per_init=169,
  native_service_seams=['Actor4D7230 success from separately bound actual Actor tail','restriction4E1810 actual source-context answers',
      'successful allocator/free','diagnostic/name sink transport'],
  rust_object_fixture_seam='Object Init only; actual source-completed Actor and owned Kingdom tail execute',
  composed_root_observation=dict(path='research/development10/immediate-world-04.json',
      sha256=sha(ROOT/'research/development10/immediate-world-04.json'),
      scope='root real terrain/Point/session Object+Actor+Kingdom Init, PostCreate and publication for three actual Kingdoms; separate next dynamic-class boundary'),
  tests=logs,files={name:sha(ROOT/name) for name in sorted(owned)},
  boundaries=['Legacy scheduling, active SAI, changed Team spatial membership and placement-failure diagnostic owners remain explicit required services.',
      'Sparse/uninitialized or invalid native index domains refuse at reached unsafe dependencies while retaining prior writes.',
      'Successful allocation only; no heap exhaustion/SEH/reentrancy claim.',
      'No full world, battle, map completion, multiplayer or original-game save readiness claim.',
      'Source/TGI producers are actual bound projections with separate native evidence; full native TGI loader is not emulated by this tail oracle.',
      'Original files and release packages untouched.'])
 path=HERE/'findings.json';path.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(dict(status=report['status'],sha256=sha(path))))
if __name__=='__main__':main()
