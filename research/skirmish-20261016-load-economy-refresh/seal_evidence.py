"""Pin this author slice; no builds, game asset writes or oracle mutation."""
from pathlib import Path
import json,hashlib
ROOT=Path(__file__).resolve().parents[2]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def pin(paths):
 return [dict(path=p,sha256=H(ROOT/p),bytes=(ROOT/p).stat().st_size)for p in paths]
common='research/skirmish-20261016-load-economy-init/'
log=ROOT/common/'runtime-final-v7.log'
complete='14 passed; 0 failed' in log.read_text() and '1 passed; 0 failed' in (ROOT/'research/skirmish-20261016-load-economy-refresh/rust-final-external-direct.log').read_text()
assert complete,'final fixture replay is not complete'
refresh='research/skirmish-20261016-load-economy-refresh/'
doc=dict(scope='Same held Economy/actual stored Kingdom refresh; no complete load or battle readiness claim',status='frozen',
 native=dict(actual_actors=[784],stored_kingdoms=[782],runtime_precisions=[24,53,64],refresh_rows=6,refresh_machines=12,source_getter_machines=30,executed_address_pins=558,intercepted_entry_pins=7,
  entries=['570020','5704C0'],scope='Full refresh/vector/Kingdom arithmetic; outer source/predicate seams separately recomputed using original/v14 getters on actual captured graph; allocator/free and Properties-context lookup named seams'),
 rust=dict(combined_unit_passed=14,external_actual_passed=1,failed=0,fixture_comparison='Fresh actual immediate-load input/output equals both native entrypoint rows at all3 arithmetic PCs; source PC remains53'),
 limits=['held Kingdom derived owner is unsupported; stored Kingdom path is bound','Organization stays in its true component slot; no fabricated context','no flow reset or ActorStore/Kingdom clone','positive optional callback branches outside existing kernels remain explicit errors'],
 files=pin(['crates/aa-runtime/src/load_economy_refresh_world.rs','crates/aa-runtime/tests/load_economy_refresh_world.rs','crates/aa-runtime/src/saved_economy_values.rs','crates/aa-runtime/src/saved_economy_refresh.rs','crates/aa-runtime/src/load_actor_state_reentry.rs','crates/aa-runtime/src/load_actor_state_reentry_tests.rs',refresh+'native_held_refresh.py',refresh+'native-held-refresh.json',refresh+'actual-held-input.json',refresh+'native-replay-final.log',refresh+'rust-final-external-direct.log',refresh+'README.txt',common+'runtime-combined-v2.log',common+'runtime-final-v7.log']))
(ROOT/refresh/'findings.json').write_text(json.dumps(doc,indent=2)+'\n')
doc=dict(scope='Held Economy InitExisting56F280 and constant AfterInit4B2830; no whole Actor Init/battle acceptance',status='frozen',
 native=dict(generated_rows=48,actual_rows=3,fresh_original_v14_machines=102,executed_address_pins=139,intercepted_allocator_entries=1,
  actual='tutorial02 decoded Actor784, sourcePC53; actual constructor queues/restoredRng, explicit arithmeticPC24/53/64; not a natural whole-immediate-lifecycle claim',
  debug_recorders='Native inactive returns execute at explicit fixture B3655C/B36560 owners; startup/active debug tracing not proved; distinct from SAI B36528'),
 rust=dict(combined_unit_passed=14,failed=0,own_init_tests=2,actual_rows=3,mode='release/locked/offline/-j4/include-ignored',fixture_replayed=True),
 files=pin(['crates/aa-runtime/src/load_economy_init_world.rs','crates/aa-runtime/src/load_economy_init_world_tests.rs',common+'native_economy_init.py',common+'native-economy-init.json',common+'actual-economy-init-current.json',common+'native-replay-current.log',common+'README.txt',common+'runtime-final-v7.log']))
(ROOT/common/'findings.json').write_text(json.dumps(doc,indent=2)+'\n')
# Separate16D findings remain frozen with their qualified attempt history.
print(json.dumps({str(p.relative_to(ROOT)):H(p)for p in [ROOT/refresh/'findings.json',ROOT/common/'findings.json']},indent=2))
