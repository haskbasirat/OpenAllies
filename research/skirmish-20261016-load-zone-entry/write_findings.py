from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def pin(p):
 b=p.read_bytes();return dict(path=p.relative_to(ROOT).as_posix(),bytes=len(b),sha256=hashlib.sha256(b).hexdigest())
assert 'test result: ok. 4 passed; 0 failed; 0 ignored' in (HERE/'rust-tests-frozen.log').read_text()
d=json.loads((HERE/'native-grid-registration.json').read_text())
assert len(d['cases'])==300 and len(d['executed_address_byte_pins'])==932 and not d['seams']
out=dict(schema=1,status='author frozen; independent review separate',
 scope='Actual4CFEA0 grid half,554CC0/554330/54EF10 and reached515620 no-effect prefixes; not complete Actor Register/world load/battle',
 native=dict(rows=300,fresh_original_v14_machines=600,executed_pins=932,seams=[],point='explicit fixture words; startup source certified separately'),
 rust=dict(final_combined_passed=4,own_native_replay_tests=1,failed=0,ignored=0,guards=['native selected-cell order and leaf gates','first callback refusal stops prefix','private PointShape validates current destination Catalog']),
 owners=['held current Actor/Kingdom shell and canonical Bank source-PC definition','same persistent LoadSpatial generation and ZoneMap geometry','actual SessionE4 callback each tile, separate from F8','actual saved WorldState.word_48 when reached','current native-order ZoneMap sets and ActorId.raw entry domain'],
 remaining=['effectful zone component/guard/control/marker/discovery children refuse at reached position','caller must retain same actual map; dimensions alone do not authenticate association','whole-world actual retained Session integration belongs to caller','no native exception/unwind, visual parity or playable battle claim'],
 production_pins=[pin(ROOT/'crates/aa-runtime/src'/n)for n in ['load_zone_entry.rs','load_zone_entry_tests.rs']],
 evidence_pins=[pin(HERE/n)for n in ['native_grid_registration.py','native-grid-registration.json','native-replay.log','inspect_native.py','native-instructions.log','point-binding-review.json','rust-tests-initial.log','rust-tests-final-v2.log','rust-tests-frozen.log','README.txt']],
 preservation='No Cargo/lock, original assets/native binary/save/profile/release changed by this slice; only new module/tests and module export')
(HERE/'findings.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(pin(HERE/'findings.json')))
