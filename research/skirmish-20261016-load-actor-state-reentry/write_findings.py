from pathlib import Path
import json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def pin(p):
 b=p.read_bytes();return dict(path=p.relative_to(ROOT).as_posix(),bytes=len(b),sha256=hashlib.sha256(b).hexdigest())
def game_pin(p):
 b=p.read_bytes();return dict(path=p.relative_to(ROOT.parent).as_posix(),bytes=len(b),sha256=hashlib.sha256(b).hexdigest())
assert 'test result: ok. 2 passed; 0 failed; 0 ignored' in (HERE/'rust-tests-frozen.log').read_text()
d=json.loads((HERE/'native-state-entry.json').read_text());assert len(d['cases'])==108 and len(d['actual_rows'])==3 and not d['seams']
a=json.loads((HERE/'actual-3172-frozen.json').read_text());assert len(a['cases'])==3
out=dict(schema=1,status='author frozen; independent review separate',
 scope='Reached saved Actor registration4D4A90 state-entry null/empty branches; no full positive state effects or playable battle claim',
 native=dict(generated_rows=108,actual_saved_field_rows=3,fresh_original_v14_machines=222,executed_pins=len(d['executed_address_byte_pins']),seams=[],executed='Full4D4A90,45E620/4B4590 BodyNULL,45E600 nullproperty,4F0460,4D0A00 nullFX,generation checks; bounded constructor stores and79E130',not_executed='full ActorData constructor/TLS/loader and FXC graph construction; static source-list provenance mapping is separately pinned'),
 rust=dict(passed=2,failed=0,ignored=0,actual_record=3172,map='Campaign/tutorial02.RMP',precision_modes=['Single24','Double53','Extended64'],source_pc='Double53',actual_definition='kingdom',actual_state_low=0x08000000,actual_state_high=0,actual_index=27,actual_state='structure_normal',stance_count=0,stance_before=-1,stance_after=-1,guards=['forged state identity','same-ID changed typed priority/provenance equality','canonical component-mask mismatch','positive Organization stops before stance mutation','reached missing mode stops after successful stance15C mutation']),
 ownership=['actual held Actor/Kingdom shell and current Loading generation','fresh canonical Source and fixed mode2 native descriptor index','Bank-owned sourcePC and source identities; runtime PC is separate','actualComponents state owner and actual15C, no raw100/104 shadow','one canonical ActorData source declaration, both sourceFXlists empty due WhenLeave->WhenEnter copying','retained MapResource mode, never missing=>zero'],
 remaining=['positive Organization/Economy/Property/Body-rule/Character/FXC/Group children explicitly unsupported','invalid old stance diagnostics unsupported','native proof no general exception/unwind certification','actual decoded-record check omits full Init/PostCreate and world stage; root immediate-world integration is separate','no renderer or complete battle parity claim'],
 production_pins=[pin(ROOT/'crates/aa-runtime/src'/n)for n in ['load_actor_state_reentry.rs','load_actor_state_reentry_tests.rs']],
 original_map_pin=game_pin(ROOT.parent/'reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP'),
 evidence_pins=[pin(HERE/n)for n in ['native_state_entry.py','native-state-entry.json','native-replay-final.log','inspect_native.py','native-instructions.log','actual-3172-frozen.json','actual-3172-initial.json','actual-capture-build.log','rust-tests-final.log','rust-tests-frozen.log','README.txt']],
 preservation='No original native executable/assets/save/profile/release changed; no workspace Cargo/lock edits by this slice; new runtime module/tests and module export only')
(HERE/'findings.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(pin(HERE/'findings.json')))
