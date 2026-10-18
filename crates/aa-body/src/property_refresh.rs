//! Original Properties581420 applies existing numeric actor consequences only.
//! It does not recompute aggregates, does not filter disabled instances, and
//! captures the initial instance count while rereading each live slot.
use crate::{
    Error, Result,
    properties::PropertiesState,
    property_effects::{Instances, NumericModifier},
    property_flags::{Definitions, Modifier},
};
pub fn refresh(
    attribute: u32,
    state: &mut PropertiesState,
    instances: &mut Instances,
    definitions: &Definitions,
    mut changed: impl FnMut(&NumericModifier, &mut PropertiesState, &mut Instances) -> Result<()>,
) -> Result<()> {
    let count = state.effects.len();
    for index in 0..count {
        let handle = *state.effects.get(index).ok_or_else(|| {
            Error::Callback("property list shrank past native captured count during refresh".into())
        })?;
        let record = instances
            .get(&handle)
            .ok_or_else(|| Error::Callback("missing property instance during refresh".into()))?;
        let definition = definitions
            .get(&record.definition)
            .ok_or_else(|| Error::Callback("missing property definition during refresh".into()))?;
        for modifier in &definition.modifiers {
            if let Modifier::Numeric(numeric) = modifier
                && numeric.index == attribute
            {
                changed(numeric, state, instances)?;
            }
        }
    }
    Ok(())
}
/// Null source pointers are absent from the owned valid-handle domain. This
/// helper makes a caller's constructor-empty list proof explicit when it has no
/// resolved property definitions yet; a nonempty list must use `refresh`.
pub fn require_empty(state: &PropertiesState) -> Result<()> {
    if !state.effects.is_empty() {
        return Err(Error::Callback(
            "nonempty property refresh requires actual definitions/consequences".into(),
        ));
    }
    Ok(())
}
