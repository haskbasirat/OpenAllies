extern crate self as aa_runtime;
use std::sync::{Arc,Weak};
pub mod world_revision {
// Process B3CC94, zero in the PE loader's uninitialized section tail.
// World constructor4F8072..4F809C increments it with native u32 wrapping.
// Kept by the actual managed application; each World retains its own stamp.
#[derive(Debug)]
pub(crate) struct Counter {
    value: u32,
}
impl Counter {
    pub(crate) fn fresh_application() -> Self {
        Self { value: 0 }
    }
    pub(crate) fn construct(&mut self) -> ConstructedWorldRevision {
        self.value = self.value.wrapping_add(1);
        ConstructedWorldRevision {
            value: self.value,
            identity: std::sync::Arc::new(()),
        }
    }
}
/// Native revision value created by the same application's World constructor.
/// No Clone/Default/public scalar constructor can manufacture this receipt.
#[derive(Debug)]
pub struct ConstructedWorldRevision {
    value: u32,
    identity: std::sync::Arc<()>,
}
/// Nonowning identity metadata for a read of the SAME retained World. It
/// cannot keep a destroyed World allocation alive or be made from a scalar.
/// The caller must derive it from the current validated load/frame owner.
#[derive(Debug, Clone)]
pub struct Witness {
    value: u32,
    identity: std::sync::Weak<()>,
}
impl Witness {
    pub(crate) fn require_live(&self)->Result<(),String>{
        self.identity.upgrade().map(|_|()).ok_or_else(||"World revision witness belongs to a destroyed allocation".into())
    }
    pub(crate) fn value(&self) -> u32 {
        self.value
    }
    pub(crate) fn owner_identity(&self) -> &std::sync::Weak<()> {
        &self.identity
    }
    ///Readonly correlation with the actual retained World allocation. This
    ///does not certify a completed frame or promote render data to a receipt.
    pub fn is_for(&self, world: &ConstructedWorldRevision) -> bool {
        self.value == world.value
            && self
                .identity
                .upgrade()
                .is_some_and(|identity| std::sync::Arc::ptr_eq(&identity, &world.identity))
    }
}
impl ConstructedWorldRevision {
    pub fn value(&self) -> u32 {
        self.value
    }
    pub fn witness(&self) -> Witness {
        Witness {
            value: self.value,
            identity: std::sync::Arc::downgrade(&self.identity),
        }
    }
    /// Stable pointer identity metadata. Equal numeric native revisions in
    /// two applications still name different actual World allocations.
    pub(crate) fn owner_identity(&self) -> &std::sync::Arc<()> {
        &self.identity
    }
}

}
#[path=r"E:\Games\Axis and Alies\rust-rebuild\crates\aa-runtime\src\view_init_prefix.rs"]pub mod view_init_prefix;
//The generic move boundary never inspects, clones or reconstructs native N.
//Production N is the complete same view_init_world::State above.
struct FreshVWorld<N>{allocation:Arc<Allocation>,native:N}
struct Allocation{origin:Weak<()>,handle:usize,ad:aa_runtime::view_init_prefix::InitializedRead}
impl Allocation{fn require_live(&self)->std::result::Result<(),String>{
    if self.origin.upgrade().is_none(){return Err("VWorld application allocation owner was dropped".into())}
    self.ad.require_live()
}}
fn adopt_constructed<N,D>(current:&Arc<Allocation>,fresh:&mut Option<FreshVWorld<N>>,
    binding:impl FnOnce()->std::result::Result<D,String>)->std::result::Result<(FreshVWorld<N>,D),String>{
    let retained=fresh.as_ref().ok_or("current VWorld constructor allocation was already adopted; actual43E400 teardown/rebinding is required")?;
    current.require_live()?;
    if !Arc::ptr_eq(&retained.allocation,current){return Err("current VWorld allocation differs from retained constructor owner".into())}
    let directory=binding()?;
    let native=fresh.take().ok_or("current VWorld constructor allocation was retired during binding")?;
    Ok((native,directory))
}

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
