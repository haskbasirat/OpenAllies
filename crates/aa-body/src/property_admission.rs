//! Actor4CEF10 admission and trigger541FE0 technology grant. The applicability
//! service binds aa-actors::state_effects::applicable over the canonical store;
//! insertion must execute5030F0 and the real mixed581630 implementation.
use crate::{Error, Result, credit::Object};

pub trait World {
    fn property_component(&self, actor: Object) -> Result<Option<Object>>;
    /// Actual actor+98 presence; only kinds8/10 use this admission exception.
    fn component_98_present(&self, actor: Object) -> Result<bool>;
    fn applicable(&mut self, actor: Object, property: Object) -> Result<bool>;
    /// Original5817A0 allocates an instance, initializes it with5030F0 and calls
    ///581630. Bind Instance::new and property_flags::insert on the shared state.
    /// Its duplicate/disabled return does not change outer4CEF10 success.
    /// Failed insertion must retain ownership of any partially committed record.
    fn create_and_insert(
        &mut self,
        component: Object,
        property: Object,
        kind: u32,
        source: Option<Object>,
    ) -> Result<()>;
}

pub fn add(
    actor: Object,
    property: Option<Object>,
    kind: u32,
    source: Option<Object>,
    world: &mut impl World,
) -> Result<bool> {
    let Some(property) = property else {
        return Ok(false);
    };
    if world.property_component(actor)?.is_none() {
        return Ok(false);
    }
    if !([8, 10].contains(&kind) && world.component_98_present(actor)?)
        && !world.applicable(actor, property)?
    {
        return Ok(false);
    }
    // Native rereads actor+78 after applicability. Never silently use a detached
    // old component when the callback changed the actor's component table.
    let component = world
        .property_component(actor)?
        .ok_or_else(|| Error::Callback("Properties component removed during admission".into()))?;
    world.create_and_insert(component, property, kind, source)?;
    Ok(true)
}

///541FE0 returns true for nonnull actor/property even if4CEF10 rejects admission.
pub fn grant_technology(
    actor: Option<Object>,
    property: Option<Object>,
    world: &mut impl World,
) -> Result<bool> {
    let (Some(actor), Some(property)) = (actor, property) else {
        return Ok(false);
    };
    add(actor, Some(property), 2, None, world)?;
    Ok(true)
}
