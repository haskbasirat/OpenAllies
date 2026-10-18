"""Seal this bounded checkpoint; absent independent review remains pending."""
from pathlib import Path
import json,hashlib,difflib,zipfile
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def write(name,doc):
 with(HERE/name).open('x',encoding='utf-8')as f:json.dump(doc,f,indent=2);f.write('\n')
def main():
 for old,new,out in [('performer-before.rs','crates/aa-combat/src/performer.rs','performer-prefix.diff'),('actor-init-before.rs','crates/aa-runtime/src/load_actor_init_world.rs','actor-init-hook.diff')]:
  text=''.join(difflib.unified_diff((HERE/old).read_text().splitlines(True),(ROOT/new).read_text().splitlines(True),fromfile=old,tofile=new));target=HERE/out
  if target.exists():assert target.read_text()==text
  else:
   with target.open('x',encoding='utf-8')as f:f.write(text)
 actual=json.loads((HERE/'actual-tutorial4097.json').read_text());native=json.loads((HERE/'native-performer-init.json').read_text());real=json.loads((HERE/'actual-native-prefix.json').read_text())
 assert actual['ability_count']==0 and actual['owner']==4097 and all(r['result']=={'Ok':True}and r['same_allocations']for r in actual['rows'])
 assert len(native['cases'])==144 and native['separate_machine_runs']==288 and len(native['executed_instructions'])==87
 assert len(real['rows'])==3 and real['separate_machine_runs']==6 and len(real['executed_instructions'])==22
 assert '2 passed; 0 failed'in(HERE/'rust-immutable-replay.log').read_text()
 assert '1 passed; 0 failed'in(HERE/'legacy-rank-regression.log').read_text()
 with zipfile.ZipFile(ROOT/'backups/source-public-build-options-20261017-005510.zip')as z:
  manifests={p:dict(prior_sha256=hashlib.sha256(z.read(p)).hexdigest(),current_sha256=sha(ROOT/p),equal=z.read(p)==(ROOT/p).read_bytes())for p in ['Cargo.toml','Cargo.lock']}
 assert manifests['Cargo.toml']['equal']
 production=['crates/aa-combat/src/performer.rs','crates/aa-runtime/src/load_performer_init_world.rs','crates/aa-runtime/src/load_performer_init_world_tests.rs','crates/aa-runtime/src/load_actor_init_world.rs']
 artifacts=[p for p in HERE.iterdir()if p.is_file()and p.name not in ['pins.json','findings.json']]
 pins=dict(schema=1,production={p:sha(ROOT/p)for p in production},shared_lib_export_snapshot=sha(ROOT/'crates/aa-runtime/src/lib.rs'),research={p.name:sha(p)for p in artifacts},workspace_manifest_observation=manifests)
 write('pins.json',pins)
 findings=dict(schema=1,scope='Concrete saved Performer57E000->57DCF0(NULL) and typed4B2830 AfterInit; actual tutorial4097 EMPTY primary list, not whole Actor/world/battle readiness',frozen=True,production=pins['production'],
  native=dict(paired_rows=144,separate_original_v14_runs=288,executed_pins=87,inspected_data_source_pins=6,stopped_boundary_pins=1,successful_seams=0,changed_byte_required_tail_rows=33,sha256=sha(HERE/'native-performer-init.json')),
  actual=dict(owner=4097,definition='usa_corps',primary_ability_count=0,live_pc_count=3,source_precision='Double53',same_component_shell_allocation=True,wrong_owner_refused_before_mutation=True,rust_capture_sha256=sha(HERE/'actual-tutorial4097.json'),native_paired_rows=3,native_separate_runs=6,native_executed_pins=22,native_sha256=sha(HERE/'actual-native-prefix.json')),
  tests=dict(production_check='author-check-02.log PASS20.08s',current_lib_test_build='author-build-02.log PASS2m51s',portable_native_prefix='rust-native-prefix.log PASS1/1',actual_map_capture='actual-capture.log PASS1/1 9.25s',immutable_current_binary_replay='rust-immutable-replay.log PASS2/2 9.38s, capture env unset',old_public_rank_wrapper_regression='legacy-rank-regression.log PASS1/1',native_immutable='native-immutable-replay.log PASS144/288',actual_native_immutable='actual-native-immutable-replay.log PASS3/6',frozen_binary='frozen-performer-init-tests.exe',frozen_binary_sha256=sha(HERE/'frozen-performer-init-tests.exe')),
  limits=[
   'Actual4097 has zero primary Ability handles. Positive signed-bound reads and33 changed-byte cases are synthetic native/commonalgorithm evidence; no natural positive local or full4D1350 tail call was reached by that actual record.',
   'SourceGetInteger/ctor virtual store/table bytes are inspected pins, not fullsource parser/constructor execution.',
   'Required4D1350 entry is not executed as a native body. Production calls full actual saved_actor_actions only when changed, after releasing immutable loans; errors retain enabled-byte prefix.',
   'Library test build01 was blocked by shared depot unsafe-lint snapshot; no test assertions ran. Current fixed shared source was used by build02.',
   'No full ActorInit, initialized/published world, combat, multiplayer, campaign, rendering improvement, playable battle or release claim. Root owns strict integration and future release.',
   'Independent review is pending; absent review must not be treated as accepted. Shared lib export snapshot may differ after separately owned additive exports.',
   'Current Cargo.lock differs from old005510 checkpoint due subsequent workspace dependency changes; exact hashes are retained. This checkpoint did not edit manifests/lock; Root owns other work provenance.'],
  preservation=dict(manifest_observation=manifests,no_original_asset_release_or_workspace_manifest_writes_by_this_checkpoint=True,source_backups=['performer-before.rs','actor-init-before.rs'],pins_sha256=sha(HERE/'pins.json')),
  independent_review=dict(status='pending',accepted=False))
 write('findings.json',findings)
 print(json.dumps(dict(findings=sha(HERE/'findings.json'),pins=sha(HERE/'pins.json'),source=pins['production'],binary=findings['tests']['frozen_binary_sha256'])))
if __name__=='__main__':main()
