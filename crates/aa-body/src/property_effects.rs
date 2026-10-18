//! Concrete Add/Multiply modifier aggregation and property-instance lifetime.
//! Loaded property definitions must contain only the supported numeric effects;
//! callers may not discard unsupported native effect kinds when constructing it.
//! Definition identities are immutable while a call runs. Instance handles and
//! the owner identity must remain coherent across callbacks; live numeric state,
//! instance flags and effect membership are reread where native code rereads them.
//! Failures retain preceding writes. In particular, `released` receives ownership
//! of the detached record, and must retain it if external cleanup fails.
use crate::{Error, F32, Result, credit::Object, properties::PropertiesState};
use aa_numeric::Precision;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::BTreeMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NumericKind {
    Add,
    Multiply,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NumericModifier {
    pub kind: NumericKind,
    pub index: u32,
    /// Native loaded amount. The Multiply loader negates authored indices18/23;
    /// callers must perform that proven definition conversion before this API.
    pub amount: F32,
    /// AddModifier+14 bit0, accepted by the native loader only for index4.
    /// Retained for actor consequences; aggregation matches kind/index only.
    pub repair: bool,
}
impl NumericModifier {
    fn validate(&self) -> Result<()> {
        if !match self.kind {
            NumericKind::Add => self.index <= 12,
            NumericKind::Multiply => (13..=24).contains(&self.index),
        } {
            return Err(Error::Callback(
                "unsupported numeric modifier index/kind".into(),
            ));
        }
        self.amount.number()?;
        if self.repair && (self.kind != NumericKind::Add || self.index != 4) {
            return Err(Error::Callback(
                "repair modifier requires native Add index4".into(),
            ));
        }
        Ok(())
    }
    fn special(&self) -> bool {
        match self.kind {
            NumericKind::Add => self.index == 11,
            NumericKind::Multiply => [21, 22].contains(&self.index),
        }
    }
    fn base(&self) -> F32 {
        match self.kind {
            NumericKind::Add => F32::ZERO,
            NumericKind::Multiply => F32::ONE,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Definition<M = NumericModifier> {
    pub flags: u32,
    pub modifiers: Vec<M>,
}
pub type Definitions<M = NumericModifier> = BTreeMap<Object, Definition<M>>;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instance {
    pub handle: Object,
    pub definition: Object,
    pub kind: u32,
    pub created: F32,
    pub source_id: u32,
    pub disabled: bool,
    /// Enchantment instances (503140, vtable A2EBE0, virtual +4 = 1): their
    /// `+18` allegiance GTID (0 when none). `None` for 5030F0 instances
    /// (omitted from serialized plain instances).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enchantment_allegiance: Option<u32>,
}
impl Instance {
    /// Native5030F0. Source is the actor's raw generation identity, not address.
    pub fn new(
        handle: Object,
        definition: Object,
        kind: u32,
        created: F32,
        source_id: Option<u32>,
    ) -> Self {
        Self {
            handle,
            definition,
            kind,
            created,
            source_id: source_id.unwrap_or(0),
            disabled: false,
            enchantment_allegiance: None,
        }
    }
    /// Native 503140(property, source, allegiance): kind 1, created at world
    /// time, the source and allegiance GTIDs.
    pub fn enchantment(
        handle: Object,
        definition: Object,
        created: F32,
        source_id: Option<u32>,
        allegiance_id: Option<u32>,
    ) -> Self {
        Self {
            enchantment_allegiance: Some(allegiance_id.unwrap_or(0)),
            ..Self::new(handle, definition, 1, created, source_id)
        }
    }
}
pub type Instances = BTreeMap<Object, Instance>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub definition: Object,
    pub source: Option<Object>,
}
fn record(instances: &Instances, id: Object) -> Result<&Instance> {
    instances
        .get(&id)
        .ok_or_else(|| Error::Callback(format!("missing property instance {}", id.0)))
}
fn definition<M>(definitions: &Definitions<M>, id: Object) -> Result<&Definition<M>> {
    definitions
        .get(&id)
        .ok_or_else(|| Error::Callback(format!("missing property definition {}", id.0)))
}

///5020A0 and numeric branches of502A70 using actual owned records. The bool is
/// the pre-clamp changed flag, exactly as used to decide the actor callback.
pub fn recompute_numeric(
    modifier: &NumericModifier,
    state: &mut PropertiesState,
    instances: &Instances,
    definitions: &Definitions,
    minimum_multiplier: F32,
    p: Precision,
) -> Result<bool> {
    recompute_numeric_with(
        modifier,
        state,
        instances,
        definitions,
        minimum_multiplier,
        p,
        |m| Some(m),
    )
}

/// The same numeric scan over ordered mixed descriptors. Unsupported descriptor
/// kinds must be dispatched separately, never dropped from a definition.
pub fn recompute_numeric_with<M>(
    modifier: &NumericModifier,
    state: &mut PropertiesState,
    instances: &Instances,
    definitions: &Definitions<M>,
    minimum_multiplier: F32,
    p: Precision,
    numeric: impl Fn(&M) -> Option<&NumericModifier>,
) -> Result<bool> {
    recompute_numeric_with_lazy_minimum(
        modifier,
        state,
        instances,
        definitions,
        || Ok(minimum_multiplier),
        p,
        numeric,
    )
}
/// Same ordered aggregate, reading the current clamp only after the native
/// aggregate store, and skipping the read for a negative special aggregate.
pub fn recompute_numeric_with_lazy_minimum<M>(
    modifier: &NumericModifier,
    state: &mut PropertiesState,
    instances: &Instances,
    definitions: &Definitions<M>,
    minimum_multiplier: impl FnOnce() -> Result<F32>,
    p: Precision,
    numeric: impl Fn(&M) -> Option<&NumericModifier>,
) -> Result<bool> {
    modifier.validate()?;
    let base = modifier.base();
    let special = modifier.special();
    let mut value = if special { F32(0xff7f_ffff) } else { base };
    let mut found = false;
    for handle in &state.effects {
        let instance = record(instances, *handle)?;
        if instance.disabled {
            continue;
        }
        for descriptor in &definition(definitions, instance.definition)?.modifiers {
            let Some(candidate) = numeric(descriptor) else {
                continue;
            };
            candidate.validate()?;
            if candidate.kind != modifier.kind || candidate.index != modifier.index {
                continue;
            }
            if special {
                let adjusted = base.number()?.add(&candidate.amount.number()?, p)?;
                if adjusted.compare(&value.number()?) != Ordering::Less {
                    value = F32(adjusted.store_f32()?);
                }
            } else {
                value = F32(candidate
                    .amount
                    .number()?
                    .add(&value.number()?, p)?
                    .store_f32()?);
            }
            found = true;
        }
    }
    if !found {
        value = if special { F32(0xbf80_0000) } else { base };
    }
    let slot = modifier.index as usize;
    let changed = state.modifier.scalars[slot].compare(value)? != Ordering::Equal;
    if changed {
        state.modifier.scalars[slot] = value;
    }
    if modifier.kind == NumericKind::Multiply {
        let value = state.modifier.scalars[slot];
        if !special || value.compare(F32::ZERO)? != Ordering::Less {
            let minimum_multiplier = minimum_multiplier()?;
            if value.compare(minimum_multiplier)? == Ordering::Less {
                state.modifier.scalars[slot] = minimum_multiplier;
            }
        }
    }
    Ok(changed)
}

pub trait Effects {
    fn resolve_source(&self, identity: u32) -> Result<Option<Object>>;
    ///581630 queries actor allegiance before duplicate selection, even though
    ///503680 does not use the returned value in its recovered body.
    fn allegiance(
        &mut self,
        state: &mut PropertiesState,
        instances: &mut Instances,
    ) -> Result<Option<Object>>;
    /// Native numeric modifier virtual+30. Called between sequential aggregate
    /// recalculations so actor/component mutations preserve original ordering.
    fn numeric_changed(
        &mut self,
        modifier: &NumericModifier,
        state: &mut PropertiesState,
        instances: &mut Instances,
    ) -> Result<()>;
    fn property_added(
        &mut self,
        handle: Object,
        state: &mut PropertiesState,
        instances: &mut Instances,
    ) -> Result<()>;
    fn actor_added(
        &mut self,
        handle: Object,
        state: &mut PropertiesState,
        instances: &mut Instances,
    ) -> Result<()>;
    fn actor_removed(
        &mut self,
        handle: Object,
        state: &mut PropertiesState,
        instances: &mut Instances,
    ) -> Result<()>;
    fn property_removed(
        &mut self,
        handle: Object,
        state: &mut PropertiesState,
        instances: &mut Instances,
    ) -> Result<()>;
    fn released(&mut self, instance: Instance) -> Result<()>;
}

fn recompute_definition<E: Effects>(
    handle: Object,
    state: &mut PropertiesState,
    instances: &mut Instances,
    definitions: &Definitions,
    minimum: F32,
    p: Precision,
    effects: &mut E,
) -> Result<()> {
    // Definition modifier lists are immutable loaded data; instance/actor fields
    // may change through callbacks. Current property identity is read here.
    let id = record(instances, handle)?.definition;
    for modifier in &definition(definitions, id)?.modifiers {
        if recompute_numeric(modifier, state, instances, definitions, minimum, p)? {
            effects.numeric_changed(modifier, state, instances)?;
        }
    }
    Ok(())
}

///503680/581630 for already created numeric-property instances. Duplicate
/// instances remain in insertion order but are disabled and emit no add effects.
pub fn insert<E: Effects>(
    instance: Instance,
    state: &mut PropertiesState,
    instances: &mut Instances,
    definitions: &Definitions,
    minimum: F32,
    p: Precision,
    effects: &mut E,
) -> Result<bool> {
    insert_with(
        instance,
        state,
        instances,
        definitions,
        effects,
        |handle, state, instances, definitions, effects| {
            recompute_definition(handle, state, instances, definitions, minimum, p, effects)
        },
    )
}

/// Shared581630 lifetime path with a mandatory ordered descriptor dispatcher.
/// The dispatcher runs after attachment and before property/actor added effects.
pub fn insert_with<M, E: Effects>(
    instance: Instance,
    state: &mut PropertiesState,
    instances: &mut Instances,
    definitions: &Definitions<M>,
    effects: &mut E,
    mut recompute: impl FnMut(
        Object,
        &mut PropertiesState,
        &mut Instances,
        &Definitions<M>,
        &mut E,
    ) -> Result<()>,
) -> Result<bool> {
    definition(definitions, instance.definition)?;
    if instances.contains_key(&instance.handle) {
        return Err(Error::Callback("property handle already exists".into()));
    }
    let handle = instance.handle;
    instances.insert(handle, instance);
    effects.allegiance(state, instances)?;
    let property = record(instances, handle)?.definition;
    let duplicate = state.effects.iter().try_fold(false, |found, id| {
        Ok::<_, Error>(found || record(instances, *id)?.definition == property)
    })?;
    state
        .effects
        .try_reserve(1)
        .map_err(|e| Error::Callback(format!("property allocation failed: {e}")))?;
    if duplicate {
        instances
            .get_mut(&handle)
            .ok_or_else(|| Error::Callback("new property disappeared".into()))?
            .disabled = true;
    }
    state.effects.push(handle);
    if duplicate {
        return Ok(false);
    }
    recompute(handle, state, instances, definitions, effects)?;
    state.modifier_flags |= definition(definitions, record(instances, handle)?.definition)?.flags;
    effects.property_added(handle, state, instances)?;
    effects.actor_added(handle, state, instances)?;
    Ok(true)
}

///5034F0/581810 selection, duplicate reactivation and active removal ordering.
pub fn remove<E: Effects>(
    selection: Selection,
    state: &mut PropertiesState,
    instances: &mut Instances,
    definitions: &Definitions,
    minimum: F32,
    p: Precision,
    effects: &mut E,
) -> Result<bool> {
    remove_with(
        selection,
        state,
        instances,
        definitions,
        effects,
        |handle, state, instances, definitions, effects| {
            recompute_definition(handle, state, instances, definitions, minimum, p, effects)
        },
    )
}

/// Shared581810 lifetime path; a later dispatcher failure retains the detached
/// record in `instances`, together with preceding callback and field effects.
pub fn remove_with<M, E: Effects>(
    selection: Selection,
    state: &mut PropertiesState,
    instances: &mut Instances,
    definitions: &Definitions<M>,
    effects: &mut E,
    mut recompute: impl FnMut(
        Object,
        &mut PropertiesState,
        &mut Instances,
        &Definitions<M>,
        &mut E,
    ) -> Result<()>,
) -> Result<bool> {
    let Selection {
        definition: property,
        source,
    } = selection;
    let mut found = None;
    for (index, handle) in state.effects.iter().enumerate() {
        let instance = record(instances, *handle)?;
        if instance.definition == property && effects.resolve_source(instance.source_id)? == source
        {
            found = Some(index);
            break;
        }
    }
    let Some(index) = found else {
        return Ok(false);
    };
    let handle = state.effects.remove(index);
    let removed = record(instances, handle)?.clone();
    let mut replacement = None;
    if !removed.disabled {
        for id in &state.effects {
            if record(instances, *id)?.definition == property {
                replacement = Some(*id);
                break;
            }
        }
    }
    if let Some(id) = replacement {
        instances
            .get_mut(&id)
            .ok_or_else(|| Error::Callback("replacement property disappeared".into()))?
            .disabled = false;
    }
    if !removed.disabled && replacement.is_none() {
        if definition(definitions, removed.definition)?.flags != 0 {
            state.modifier_flags = 0;
            for id in &state.effects {
                let instance = record(instances, *id)?;
                if !instance.disabled {
                    state.modifier_flags |= definition(definitions, instance.definition)?.flags;
                }
            }
        }
        effects.actor_removed(handle, state, instances)?;
        effects.property_removed(handle, state, instances)?;
        recompute(handle, state, instances, definitions, effects)?;
    }
    let released = instances
        .remove(&handle)
        .ok_or_else(|| Error::Callback("removed property disappeared before release".into()))?;
    effects.released(released)?;
    Ok(!removed.disabled && replacement.is_none())
}
