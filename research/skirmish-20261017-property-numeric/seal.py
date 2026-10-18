"""Seal the completed bounded author checkpoint; never generates native rows."""
from pathlib import Path
import json,hashlib,shutil
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sha=lambda b:hashlib.sha256(b).hexdigest()
def main():
 log=(HERE/'author-final-immutable.log').read_text()
 assert 'test actual_multiply_cell_and_numeric_consequences_all_pc ... ok' in log
 assert 'test result: FAILED' not in log
 assert log.count('test result: ok.')==2,log[-2000:]
 own=['crates/aa-runtime/src/load_property_numeric.rs','crates/aa-runtime/tests/property_numeric.rs']
 bridges=['crates/aa-runtime/src/load_property_instances.rs','crates/aa-runtime/src/load_view_variables.rs','crates/aa-runtime/src/load_performer_value_refresh.rs']
 proof=['actual-inputs.json','native_numeric.py','native-numeric.json','native_modifier_variables.py','native-modifier-variables.json','README.txt']
 dependencies=['research/skirmish-20261016-load-property-instances/native_property_instances.py','research/skirmish-20261017-performer-value/native_current_combat_vars.py','research/phase8-ai/native_machine.py','research/phase8-movement/harness.py']
 binaries=['target/s-owner-recruitment/release/deps/property_numeric-05ff61eeac63a70e.exe','target/s-owner-recruitment/release/deps/load_property_instances-7bd782e89c57cdcf.exe']
 pins=[]
 for path in own+bridges+dependencies+binaries:
  raw=(ROOT/path).read_bytes();pins.append(dict(path=path,sha256=sha(raw),bytes=len(raw)))
  if path in own+bridges:
   target=HERE/'source-v2'/path;target.parent.mkdir(parents=True,exist_ok=True)
   assert not target.exists();target.write_bytes(raw)
 for path in proof:
  raw=(HERE/path).read_bytes();pins.append(dict(path=str((HERE/ path).relative_to(ROOT)).replace('\\','/'),sha256=sha(raw),bytes=len(raw)))
 # load_world is owned concurrently by root. Pin only the authorized numeric
 # getter/forwarder; do not attribute unrelated evolving world code to this seal.
 text=(ROOT/'crates/aa-runtime/src/load_world.rs').read_text()
 start=text.index('    fn minimum_multiplier(',text.index('struct PropertyEffects'))
 end=text.index('\n}\nfn current_property_time',start)+2
 region=text[start:end];(HERE/'load-world-numeric-binding.txt').write_text(region)
 pins.append(dict(path='crates/aa-runtime/src/load_world.rs',region_start='fn minimum_multiplier in impl PropertyEffects',region_end='before fn current_property_time',sha256=sha(region.encode())))
 receipt=dict(schema=1,scope='pre-staging same-owned numeric callbacks only; no completed battle, live ActorStore or positive unbound child claim',pins=pins,source_scope={'own':own,'approved_narrow_bridges':bridges,'shared_region_only':'load_world PropertyEffects minimum_multiplier/numeric_changed'},commands=['python research/skirmish-20261017-property-numeric/native_numeric.py','python research/skirmish-20261017-property-numeric/native_modifier_variables.py','AA_GAME_ROOT=<actual root> target/s-owner-recruitment/release/deps/property_numeric-05ff61eeac63a70e.exe actual_multiply_cell_and_numeric_consequences_all_pc --ignored --nocapture','AA_GAME_ROOT=<actual root> target/s-owner-recruitment/release/deps/load_property_instances-7bd782e89c57cdcf.exe actual_saved_allocations_property_prefixes_all_pc --ignored --nocapture'])
 (HERE/'freeze.json').write_text(json.dumps(receipt,indent=2)+'\n')
 findings=dict(schema=1,status='bounded author proof passed; independent review pending',native={'numeric':dict(rows=66,provided_rows=9,executions=150,instruction_addresses=971),'variables':dict(rows=12,executions=24,instruction_addresses=168),'images':['original','v14'],'precision':['Single24','Double53','Extended64']},rust={'actual_numeric_and_current_cells':'1/1 passed immutable','prior_property_instance_regression':'1/1 passed immutable','log':'author-final-immutable.log'},storage={'same_held_properties':True,'same_ability_arena':True,'same_world_variable_owner':True,'temporary_instance_arena':False,'source_precision':'actual same Bank Double53; live arithmetic all3PC','staged_bank':'not supported by this pre-staging binding; missing Catalog Bank is required error'},guard_rows={'missing_sources_default_or_absent':12,'required_mover_or_morale_boundary':6},gaps=['576C70 positive Mover speed/group/Mind','578300 positive maximum morale','561D40 positive linked-Body recursion','post-staging moved-Bank numeric binder','all prior checkpoint Enchantment/Settlement/positive propagation/FX limitations'],historical_correction='v1 missing-clamp proof did not supply a SVar. Root supplied runtime now correctly reads Multiply index6A/+1A8/default0.05; Add index69/+1A4/default1 remains separate. No ClampSub exists.',freeze_sha256=sha((HERE/'freeze.json').read_bytes()))
 (HERE/'findings.json').write_text(json.dumps(findings,indent=2)+'\n')
 print(json.dumps(dict(findings_sha256=sha((HERE/'findings.json').read_bytes()),freeze_sha256=findings['freeze_sha256'],pins=len(pins),binaries=[p for p in pins if p['path']in binaries])))
if __name__=='__main__':main()
