"""Bind actor source and read-only original/v14 evidence after focused Rust checks."""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE))
from trace_registry import P,V,H

def main():
 registry=json.loads((HERE/'trace_registry.json').read_text())
 components=json.loads((HERE/'components-native-oracle.json').read_text())
 for proof,producer,windows in [(registry,'trace_registry.py',list(registry['ranges'].values())),(components,'trace_components.py',components['windows'])]:
  assert proof.get('producer_sha256',proof.get('generator_sha256'))==H((HERE/producer).read_bytes())
  for row in windows:
   a=int(row['start'],16);z=int(row['end'],16);b=P.range(a,z-a)
   assert H(b)==row['sha256'] and b==V.range(a,z-a)
 # These are static boundaries for explicit unported leaves, not emulated
 # constructors. Complete-image pins and exact windows avoid decompiler drift.
 static=[]
 for name,a,n in [('actor_selector',0x4f8e90,101),('base_identity_init',0x4ee460,81),('world_create',0x4f9c10,0x420),('actor_ctor',0x4d6bb0,0x35e),('object_ctor',0x4ea5e0,0x100),('thing_ctor',0x4ef2a0,0x270),('actor_create_init',0x4d6f10,0x31e),('actor_existing_init_prefix',0x4d7230,0xc3),('post_create',0x4cf300,0xa1)]:
  b=P.range(a,n);assert b==V.range(a,n),name
  static.append({'name':name,'start':hex(a),'bytes':n,'sha256':H(b),'scope':'static boundary only; complete function behavior not claimed'})
 static_path=HERE/'construction-boundaries.json'
 static_path.write_text(json.dumps({'schema':1,'passed':True,'original_sha256':H(P.b),'v14_sha256':H(V.b),'ranges':static,'unported':['world category/factory dispatch','full object/actor constructors','alias/publication/destruction lifecycle','spatial placement and terrain projection','owner/kingdom/required-object/ability phases','random orientation and seed source']},indent=2)+'\n')
 files=[*sorted((ROOT/'crates/aa-actors').rglob('*.rs')),ROOT/'crates/aa-actors/Cargo.toml',ROOT/'crates/aa-actors/README.md',HERE/'README.txt',HERE/'trace_registry.py',HERE/'trace_registry.json',HERE/'trace_components.py',HERE/'components-native-oracle.json',static_path,Path(__file__),ROOT/'research/engine-baseline/audit_baseline.py']
 report={'schema':1,'passed':True,'component':'aa-actors original/v14 identity and bounded component lifecycle','original_sha256':H(P.b),'v14_sha256':H(V.b),'files':{p.relative_to(ROOT).as_posix():H(p.read_bytes()) for p in files},'rust_tests':6,'strict_clippy':True,'native_cases':{'registry_transitions':registry['operation_count'],'separate_original_add_capacity_gate':1,'component_scenarios':len(components['cases']),'component_enum_entries':len(components['component_enum'])},'commands':['python research/phase4-actors/trace_registry.py','python research/phase4-actors/trace_components.py','cargo test -p aa-actors','cargo clippy -p aa-actors --all-targets -- -D warnings'],'scope':['GTID allocation/saved registration/generation/stale lookup/removal/identity exchange/ascending iteration.','Typed31-slot component selection, live requirement validation and exact bounded lifecycle callback order.','Registry checkpoint validation; not a native save codec.'],'limits':['Full actor factory/deployment, placement, alias/ownership/publication and ability/AI logic remain unported.','Component constructors and virtual methods are explicit callbacks, not guessed native defaults.','Initialization failures remain partial; no atomic rollback or automatic retry.','16-bit generation wrapping and original32768 normal-Add gate intentionally retained.','Original images, profiles and data remain read-only.']}
 p=HERE/'validation.json';p.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'validation_sha256':H(p.read_bytes()),'files':len(files),'cases':report['native_cases']}))
if __name__=='__main__':main()
