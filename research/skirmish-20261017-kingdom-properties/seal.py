"""Seal the bounded checkpoint; no capture/oracle generation occurs here."""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sha=lambda b:hashlib.sha256(b).hexdigest()
def main():
 tests=[('author-immutable-kingdom.log','actual_kingdom_property_allocations_and_effects_all_pc'),('author-immutable-instance.log','actual_saved_allocations_property_prefixes_all_pc'),('author-immutable-numeric.log','actual_multiply_cell_and_numeric_consequences_all_pc')]
 for path,test in tests:
  log=(HERE/path).read_text();assert f'test {test} ... ok' in log and 'test result: FAILED' not in log,path
 own=['crates/aa-runtime/src/load_kingdom_properties.rs','crates/aa-runtime/tests/kingdom_properties.rs']
 bridges=['crates/aa-runtime/src/load_property_instances.rs','crates/aa-mapworld/src/special_state.rs']
 dependencies=['research/phase8-ai/native_machine.py','research/phase8-movement/harness.py']
 binaries=['target/s-owner-recruitment/release/deps/kingdom_properties-9170f603efb3a37f.exe','target/s-owner-recruitment/release/deps/load_property_instances-7bd782e89c57cdcf.exe','target/s-owner-recruitment/release/deps/property_numeric-05ff61eeac63a70e.exe']
 proof=['actual-inputs.json','native_kingdom_properties.py','native-kingdom-properties.json','README.txt']+[x[0]for x in tests]
 pins=[]
 for path in own+bridges+dependencies+binaries:
  raw=(ROOT/path).read_bytes();pins.append(dict(path=path,sha256=sha(raw),bytes=len(raw)))
  if path in own+bridges:
   target=HERE/'source-v1'/path;target.parent.mkdir(parents=True,exist_ok=True)
   assert not target.exists();target.write_bytes(raw)
 for path in proof:
  raw=(HERE/path).read_bytes();pins.append(dict(path=(HERE/path).relative_to(ROOT).as_posix(),sha256=sha(raw),bytes=len(raw)))
 fixture=json.loads((HERE/'native-kingdom-properties.json').read_text())
 assert len(fixture['rows'])==45
 commands=['python research/skirmish-20261017-kingdom-properties/native_kingdom_properties.py']
 commands += [f'AA_GAME_ROOT=<actual root> {b} {test} --ignored --nocapture'for b,(_,test)in zip(binaries,tests)]
 freeze=dict(schema=1,scope='Stored pre-staging Kingdom4E3AD0 plus unchanged factored constructor/native vector growth; actual direct laboratory grants, not natural mission collection or notification success',pins=pins,source_scope={'own':own,'approved_bridges':bridges,'bridge_scope':'construct_instance factors existing prefix; NativeVectorState.push only. Whole shared files retained as contemporaneous snapshots, not attribution of other authors work.'},commands=commands)
 (HERE/'freeze.json').write_text(json.dumps(freeze,indent=2)+'\n')
 findings=dict(schema=1,status='bounded author proof passed; independent review pending',native=dict(rows=45,executions=90,instruction_addresses=846,images=['original','v14'],precision=['Single24','Double53','Extended64'],successful_property_or_effect_seams=False,leaves=['heap allocation/free','4F0400/504530 exact captured restriction key/profile lookup transport'],boundaries=['50312F missing World time after constructor prefix','4E3BF5 required Data+10 name/GameEventManager before notify']),rust=dict(kingdom='1/1 immutable passed',prior_instance='1/1 immutable passed',prior_numeric='1/1 immutable passed'),actual_preparation=dict(map='Campaign/tutorial02',actor=2696,kingdom=3172,source_precision='Double53 same completed Bank',live_precision='all3PC',add_actor_prefix='actual4E6180 class insertion; later economy1B8 Init still required',intrinsics=['vehicle','infantry','hq'],grants='four genuine installed source lists explicitly applied in laboratory; saved2697 list is empty'),storage=dict(same_instances_owner=True,same_actual_318_vector=True,duplicate_append_before_disable=True,constructor_before_duplicate=True,live_actor_allocation_preserved_on_errors=True),gaps=['actual technology_gained GameEventManager/name owner','actual denial logger/name','class-list callback mutation needs native node owner; precise post-callback refusal retained','held-Kingdom derived borrow binding','post-staging moved-Bank binding','prior positive Enchantment/Settlement/propagation/FX and numeric Mover/morale/linked-Body boundaries'],freeze_sha256=sha((HERE/'freeze.json').read_bytes()))
 (HERE/'findings.json').write_text(json.dumps(findings,indent=2)+'\n')
 print(json.dumps(dict(findings_sha256=sha((HERE/'findings.json').read_bytes()),freeze_sha256=findings['freeze_sha256'],pins=len(pins),binaries=[p for p in pins if p['path']in binaries])))
if __name__=='__main__':main()
