//! Native4F5490's immediate destruction body, without4F6760 request effects.
//! Owners provide actual virtual dispatch, list changes and final teardown.

pub trait World {
    type Object: Copy;
    type Error;

    fn actor_b4(&mut self, object: Self::Object) -> Result<bool, Self::Error>;
    fn resolve_handle(&mut self, object: Self::Object) -> Result<Option<Self::Object>, Self::Error>;
    fn actor_2c(&mut self, object: Self::Object, reason: u32) -> Result<(), Self::Error>;
    fn or_object_flags(&mut self, object: Self::Object, flags: u32) -> Result<(), Self::Error>;
    fn availability_and_classification(&mut self, object: Self::Object) -> Result<(), Self::Error>;
    /// Actual F0(false),50C1B0 and E0(true), preserving their error prefix.
    fn finalize(&mut self, object: Self::Object) -> Result<(), Self::Error>;
}

pub fn run<W: World + ?Sized>(world: &mut W, object: W::Object) -> Result<(), W::Error> {
    let mut target = object;
    if world.actor_b4(object)? {
        if let Some(resolved) = world.resolve_handle(object)? {
            target = resolved;
        }
    }
    world.actor_2c(target, 3)?;
    world.or_object_flags(target, 2)?;
    world.availability_and_classification(target)?;
    world.finalize(target)
}
