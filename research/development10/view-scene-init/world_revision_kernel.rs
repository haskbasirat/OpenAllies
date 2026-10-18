//! Process B3CC94, zero in the PE loader's uninitialized section tail.
//! World constructor4F8072..4F809C increments it with native u32 wrapping.
//! Kept by the actual managed application; each World retains its own stamp.
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
    pub(crate) fn value(&self) -> u32 { self.value }
    pub(crate) fn owner_identity(&self) -> &std::sync::Weak<()> { &self.identity }
    ///Readonly correlation with the actual retained World allocation. This
    ///does not certify a completed frame or promote render data to a receipt.
    pub fn is_for(&self, world: &ConstructedWorldRevision) -> bool {
        self.value == world.value && self.identity.upgrade().is_some_and(|identity|
            std::sync::Arc::ptr_eq(&identity, &world.identity))
    }
}
impl ConstructedWorldRevision {
    pub fn value(&self) -> u32 {
        self.value
    }
    pub fn witness(&self) -> Witness {
        Witness { value: self.value, identity: std::sync::Arc::downgrade(&self.identity) }
    }
    /// Stable pointer identity metadata. Equal numeric native revisions in
    /// two applications still name different actual World allocations.
    pub(crate) fn owner_identity(&self) -> &std::sync::Arc<()> {
        &self.identity
    }
}

