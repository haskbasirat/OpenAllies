//! Public actor property entry points for other crates' callbacks (e.g.
//! aa-economy Kingdom AddProperty `0x4E3AD0` / RemoveProperty `0x4E23D0` and
//! the kingdom registry's property records).
//!
//! * [`apply_property`] is native `0x4CEF10` (proven by
//!   `research/phase7-properties/native-admission.json`; body in
//!   [`crate::property_admission::add`]).
//! * [`remove_property`] is native `0x4CEF70`: `actor+0x78` Properties
//!   component, absent -> `ret 8` without a call, otherwise a tail jump to
//!   `0x581810(property, source)` on that component, whose body is
//!   [`crate::property_effects::remove`] (proven by the property-effects
//!   fixtures). The wrapper itself is proven by
//!   `research/phase8-combat/native-property-wrappers-fixtures.json`.
use crate::{
    Result,
    credit::Object,
    property_admission::{self, World as AdmissionWorld},
    property_effects::Selection,
};

/// Native `0x4CEF10(actor; property, kind, source)`; returns its `al`.
pub fn apply_property(
    actor: Object,
    property: Option<Object>,
    kind: u32,
    source: Option<Object>,
    world: &mut impl AdmissionWorld,
) -> Result<bool> {
    property_admission::add(actor, property, kind, source, world)
}

pub trait RemovalWorld {
    /// Actor `+0x78` Properties component.
    fn property_component(&self, actor: Object) -> Result<Option<Object>>;
    /// `0x581810` on `component`: bind [`crate::property_effects::remove`]
    /// over that component's `PropertiesState` with this selection.
    fn remove_from_component(&mut self, component: Object, selection: Selection) -> Result<bool>;
}

/// Native `0x4CEF70(actor; property, source)`. `None` when the actor has no
/// Properties component: the native returns without a call and leaves `eax`
/// unspecified, so no result is invented. Otherwise the `0x581810` result.
pub fn remove_property(
    actor: Object,
    property: Object,
    source: Option<Object>,
    world: &mut impl RemovalWorld,
) -> Result<Option<bool>> {
    let Some(component) = world.property_component(actor)? else {
        return Ok(None);
    };
    world
        .remove_from_component(
            component,
            Selection {
                definition: property,
                source,
            },
        )
        .map(Some)
}

/// Native `0x5032D0(list, definition, source, exclude)`: the first instance
/// in list order whose definition (`+C`) matches, whose source GTID (`+14`,
/// resolved through the registry; `None` when stale) equals `source` when a
/// source is given, and which is not `exclude`.
pub fn find_instance<T: Copy + PartialEq>(
    instances: &[T],
    definition: Object,
    source: Option<Object>,
    exclude: Option<T>,
    mut facts: impl FnMut(T) -> Result<(Object, Option<Object>)>,
) -> Result<Option<T>> {
    for &instance in instances {
        let (instance_definition, instance_source) = facts(instance)?;
        if instance_definition != definition {
            continue;
        }
        if source.is_some() && instance_source != source {
            continue;
        }
        if Some(instance) != exclude {
            return Ok(Some(instance));
        }
    }
    Ok(None)
}

pub trait LookupWorld {
    /// Actor `+0x78` Properties component.
    fn property_component(&self, actor: Object) -> Result<Option<Object>>;
    /// The component's `+10` instance list, in order.
    fn instances(&self, component: Object) -> Result<Vec<Object>>;
    /// Instance `+C` definition and its resolved `+14` source.
    fn instance_facts(&self, instance: Object) -> Result<(Object, Option<Object>)>;
}

/// Native `0x4CEF80(actor; definition, source)`: no Properties component is
/// a null result; otherwise `0x581280` = [`find_instance`] over the
/// component's list with no exclusion.
pub fn find_actor_property(
    actor: Object,
    definition: Object,
    source: Option<Object>,
    world: &impl LookupWorld,
) -> Result<Option<Object>> {
    let Some(component) = world.property_component(actor)? else {
        return Ok(None);
    };
    let instances = world.instances(component)?;
    find_instance(&instances, definition, source, None, |i| {
        world.instance_facts(i)
    })
}
