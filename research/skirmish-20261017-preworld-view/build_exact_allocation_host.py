"""Exact production client move-boundary + real AD field owner tests.
Only unrelated World integration tests are omitted from world_revision.
No aggregate/assets/physicalRAM/camera/Directory child is simulated or granted.
"""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
source=ROOT/'crates/aa-client/src/view_world_owner.rs';text=source.read_text()
part=text[text.index('//The generic move boundary'):text.index('///Nonowning identity')]
world=ROOT/'crates/aa-runtime/src/world_revision.rs';whole=world.read_text();body=whole.split('#[cfg(test)]',1)[0]
prefix=ROOT/'crates/aa-runtime/src/view_init_prefix.rs'
header='extern crate self as aa_runtime;\nuse std::sync::{Arc,Weak};\n'
header+='pub mod world_revision {\n'+body.replace('//!','//')+'}\n'
header+=f'#[path=r"{prefix}"]pub mod view_init_prefix;\n'
tests=r'''
fn retained(origin:&Arc<()>)->FreshVWorld<view_init_prefix::State<()>>{
 let native=view_init_prefix::State::from_constructor_fields();
 let allocation=Arc::new(Allocation{origin:Arc::downgrade(origin),handle:1,ad:native.initialized_reader()});
 FreshVWorld{native,allocation}
}
#[test]fn failed_binding_preserves_same_native_then_adopts_it_once(){
 let origin=Arc::new(());let seed=retained(&origin);let current=Arc::clone(&seed.allocation);
 let mut pending=Some(seed);assert!(!current.ad.read().unwrap());
 assert!(adopt_constructed::<_,()>(&current,&mut pending,||Err("reached real binding failure".into())).is_err());
 assert!(current.ad.is_for(&pending.as_ref().unwrap().native));assert_eq!(current.handle,1);
 let(native,bound)=adopt_constructed(&current,&mut pending,||Ok(37)).unwrap();assert_eq!(bound,37);
 assert!(Arc::ptr_eq(&native.allocation,&current));assert!(current.ad.is_for(&native.native));
 assert!(pending.is_none());assert!(!current.ad.read().unwrap());
 assert!(adopt_constructed::<_,()>(&current,&mut pending,||panic!("second adoption must not invoke binding")).is_err());
 drop(native);assert!(current.ad.require_live().is_err());assert!(current.require_live().is_err());
}
#[test]fn equal_numeric_handle_cannot_adopt_foreign_native_allocation(){
 let origin=Arc::new(());let seed=retained(&origin);let foreign=retained(&origin);
 assert_eq!(seed.allocation.handle,foreign.allocation.handle);
 let current=Arc::clone(&foreign.allocation);let mut pending=Some(seed);
 assert!(adopt_constructed::<_,()>(&current,&mut pending,||panic!("foreign owner must stop before binding")).is_err());
 assert!(pending.is_some());assert!(!current.ad.is_for(&pending.as_ref().unwrap().native));
}
#[test]fn dead_application_refuses_adoption_without_consuming_actual_native(){
 let origin=Arc::new(());let seed=retained(&origin);let current=Arc::clone(&seed.allocation);let mut pending=Some(seed);
 drop(origin);assert!(current.require_live().is_err());
 assert!(adopt_constructed::<_,()>(&current,&mut pending,||panic!("retired application must stop before binding")).is_err());
 assert!(pending.is_some());assert!(!pending.as_ref().unwrap().native.initialized());
}
'''
program=header+part+tests
out=HERE/'exact_allocation_host.rs'
with out.open('x') as f:f.write(program)
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
doc=dict(schema=1,scope=__doc__,client_source=str(source.relative_to(ROOT)),client_source_sha256=H(source),
 client_fragment_sha256=hashlib.sha256(part.encode()).hexdigest(),world_source_sha256=H(world),world_fragment_sha256=hashlib.sha256(body.encode()).hexdigest(),
 exact_prefix_sha256=H(prefix),host_sha256=H(out),producer_sha256=H(Path(__file__)),
 exclusions=['Whole ApplicationViews child construction and actual Directory binding are not executed. Generic move boundary lends an explicit scalar test binding; no World/Directory readiness is asserted.',
  'N is the real production view_init_prefix::State in boundary tests; product N is the complete view_init_world::State. Neither is copied/cloned by the unchanged generic boundary.'])
with (HERE/'exact-allocation-source-pins.json').open('x') as f:f.write(json.dumps(doc,indent=1)+'\n')
print(json.dumps(doc))
