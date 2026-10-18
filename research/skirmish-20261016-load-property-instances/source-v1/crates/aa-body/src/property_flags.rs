//! Original FlagModifier aggregation and ordered mixed numeric/flag lifetime.
//! Descriptor lists remain immutable; callbacks see and may change live state.
//! Native flag words and property metadata masks are distinct storage.
use crate::{
    Error, F32, Result,
    credit::Object,
    properties::PropertiesState,
    property_effects::{self as life, Instance, Instances, NumericModifier, Selection},
};
use aa_numeric::Precision;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlagModifier {
    pub index: u32,
}
impl FlagModifier {
    ///4FEBB0 accepts a nonnegative resolved registration index. Name resolution
    /// must use the original flag domain, not an arbitrary enum/string guess.
    pub fn from_resolved(index: i32) -> Result<Self> {
        if index < 0 {
            return Err(Error::Callback(
                "unknown native FlagModifier attribute".into(),
            ));
        }
        Ok(Self {
            index: index as u32,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Modifier {
    Flag(FlagModifier),
    Numeric(NumericModifier),
    Resistance(crate::property_resistance::ResistDamage),
    /// ENCHANTMENT_IMMUNITY / ABILITY_EFFECT / PERIODIC_DAMAGE /
    /// PERIODIC_HEAL / REVERSE_ABILITY / RESOURCE_PRODUCTION
    /// ([`crate::modifier_tables`]).
    Table(crate::modifier_tables::TableModifier),
}
pub type Definition = life::Definition<Modifier>;
pub type Definitions = life::Definitions<Modifier>;

///501E10: find any active instance containing exactly the requested kind/index.
/// Index equality is full-width; the eventual x86 bit shift masks it to five bits.
pub fn recompute_flag(
    modifier: FlagModifier,
    state: &mut PropertiesState,
    instances: &Instances,
    definitions: &Definitions,
) -> Result<bool> {
    let mut found = false;
    'instances: for id in &state.effects {
        let instance = instances
            .get(id)
            .ok_or_else(|| Error::Callback("missing property instance during flag scan".into()))?;
        if instance.disabled {
            continue;
        }
        let definition = definitions.get(&instance.definition).ok_or_else(|| {
            Error::Callback("missing property definition during flag scan".into())
        })?;
        for candidate in &definition.modifiers {
            if matches!(candidate, Modifier::Flag(f) if f.index == modifier.index) {
                found = true;
                break 'instances;
            }
        }
    }
    let bit = 1u32 << (modifier.index & 31);
    let changed = (state.modifier.flags & bit != 0) != found;
    if changed {
        if found {
            state.modifier.flags |= bit;
        } else {
            state.modifier.flags &= !bit;
        }
    }
    Ok(changed)
}

/// Required concrete actor consequences of4FE460. Callers bind the named live
/// component/actor methods. No absent method is treated as successful work.
pub trait Effects: life::Effects {
    /// Current cached SVar read only at the native reached clamp instruction.
    fn minimum_multiplier(&mut self) -> Result<F32> {
        Err(Error::Callback(
            "current modifier clamp SVar is not bound".into(),
        ))
    }
    fn has_mover(&self, actor: Object) -> Result<bool>;
    fn set_mover_byte_4c(
        &mut self,
        actor: Object,
        value: bool,
        state: &mut PropertiesState,
        instances: &mut Instances,
    ) -> Result<()>;
    fn actor_stop_4d4c60(
        &mut self,
        actor: Object,
        value: bool,
        state: &mut PropertiesState,
        instances: &mut Instances,
    ) -> Result<()>;
    fn has_character(&self, actor: Object) -> Result<bool>;
    fn character_pause_4daf20(
        &mut self,
        actor: Object,
        state: &mut PropertiesState,
        instances: &mut Instances,
    ) -> Result<()>;
    fn character_resume_4daf40(
        &mut self,
        actor: Object,
        state: &mut PropertiesState,
        instances: &mut Instances,
    ) -> Result<()>;
    fn mover_576800(
        &mut self,
        actor: Object,
        value: bool,
        state: &mut PropertiesState,
        instances: &mut Instances,
    ) -> Result<()>;
    fn has_economy(&self, actor: Object) -> Result<bool>;
    fn economy_570020(
        &mut self,
        actor: Object,
        state: &mut PropertiesState,
        instances: &mut Instances,
    ) -> Result<()>;
    fn rebuild_actions_4d1350(
        &mut self,
        actor: Object,
        state: &mut PropertiesState,
        instances: &mut Instances,
    ) -> Result<()>;
    /// Table modifiers (502B30/502C60) only. Each default is an explicit
    /// error: a definition holding table modifiers needs these bound.
    /// `[B36530]+38` Resource registry count.
    fn resource_count(&self) -> Result<usize> {
        Err(Error::Callback("resource_count not bound (501F60)".into()))
    }
    /// 4CF0F0 on the actor.
    fn refresh_actor_4cf0f0(
        &mut self,
        _actor: Object,
        _state: &mut PropertiesState,
        _instances: &mut Instances,
    ) -> Result<()> {
        Err(Error::Callback("refresh_actor_4cf0f0 not bound".into()))
    }
    /// 4C86D0(definition) with main power 4BE3F0(0); returns the Ability.
    fn create_reverse_ability(&mut self, _definition: Object) -> Result<Object> {
        Err(Error::Callback(
            "create_reverse_ability not bound (502640)".into(),
        ))
    }
    /// 4CE080 and delete.
    fn destroy_reverse_ability(&mut self, _ability: Object) -> Result<()> {
        Err(Error::Callback(
            "destroy_reverse_ability not bound (5023B0)".into(),
        ))
    }
    /// Property definition `+64` enchantment type.
    fn enchantment_type(&self, _definition: Object) -> Result<Option<Object>> {
        Err(Error::Callback(
            "enchantment_type not bound (4FE6F0)".into(),
        ))
    }
    /// Actor virtual `+138(allegiance)`.
    fn allied(
        &mut self,
        _actor: Object,
        _allegiance: Option<Object>,
        _state: &mut PropertiesState,
        _instances: &mut Instances,
    ) -> Result<bool> {
        Err(Error::Callback("allied not bound (4FE6F0)".into()))
    }
}
///4FE460 rereads the actual modifier flags. For NOT_DISPOSABLE(index11), the
/// consequence is the existing actor action-mask rebuild, on both set and clear.
pub fn consequence<E: Effects>(
    modifier: FlagModifier,
    actor: Object,
    state: &mut PropertiesState,
    instances: &mut Instances,
    effects: &mut E,
) -> Result<()> {
    let value = state.modifier.flags & (1u32 << (modifier.index & 31)) != 0;
    match modifier.index {
        0 => {
            if effects.has_mover(actor)? {
                effects.set_mover_byte_4c(actor, value, state, instances)?;
            }
        }
        1 => {
            effects.actor_stop_4d4c60(actor, value, state, instances)?;
            if effects.has_character(actor)? {
                if value {
                    effects.character_pause_4daf20(actor, state, instances)?;
                } else {
                    effects.character_resume_4daf40(actor, state, instances)?;
                }
            }
        }
        2 => {
            if effects.has_mover(actor)? {
                effects.mover_576800(actor, value, state, instances)?;
            }
        }
        9 => {
            if effects.has_economy(actor)? {
                effects.economy_570020(actor, state, instances)?;
            }
        }
        11 => effects.rebuild_actions_4d1350(actor, state, instances)?,
        // All other branches return without an actor callback in4FE460.
        _ => {}
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Minimum {
    Fixed(F32),
    Current,
}
impl Minimum {
    fn read(self, effects: &mut impl Effects) -> Result<F32> {
        match self {
            Self::Fixed(v) => Ok(v),
            Self::Current => effects.minimum_multiplier(),
        }
    }
}
/// [`crate::modifier_tables::TableWorld`] over the live Properties state.
struct Tables<'a, E> {
    state: &'a mut PropertiesState,
    instances: &'a mut Instances,
    definitions: &'a Definitions,
    minimum: Minimum,
    p: Precision,
    effects: &'a mut E,
}
impl<E: Effects> crate::modifier_tables::TableWorld for Tables<'_, E> {
    fn modifier_state(&mut self) -> &mut crate::properties::ModifierState {
        &mut self.state.modifier
    }
    fn resource_count(&self) -> Result<usize> {
        self.effects.resource_count()
    }
    fn refresh_actor(&mut self, actor: Object) -> Result<()> {
        self.effects
            .refresh_actor_4cf0f0(actor, self.state, self.instances)
    }
    fn create_reverse_ability(&mut self, definition: Object) -> Result<Object> {
        self.effects.create_reverse_ability(definition)
    }
    fn destroy_reverse_ability(&mut self, ability: Object) -> Result<()> {
        self.effects.destroy_reverse_ability(ability)
    }
    fn actor_instances(
        &mut self,
        _actor: Object,
    ) -> Result<Vec<crate::modifier_tables::ActorInstance>> {
        let mut out = Vec::new();
        for id in &self.state.effects {
            let instance = self
                .instances
                .get(id)
                .ok_or_else(|| Error::Callback("missing property instance in purge".into()))?;
            if instance.enchantment_allegiance.is_none() {
                //4FE6F0 skips plain instances immediately after virtual+4;
                //the remaining carrier fields are not observed by its filter.
                out.push(crate::modifier_tables::ActorInstance {
                    definition: instance.definition,
                    enchantment: false,
                    enchantment_type: None,
                    source: None,
                    allegiance: None,
                });
                continue;
            }
            let allegiance = match instance.enchantment_allegiance {
                Some(gtid) => self.effects.resolve_source(gtid)?,
                None => None,
            };
            out.push(crate::modifier_tables::ActorInstance {
                definition: instance.definition,
                enchantment: instance.enchantment_allegiance.is_some(),
                enchantment_type: self.effects.enchantment_type(instance.definition)?,
                source: self.effects.resolve_source(instance.source_id)?,
                allegiance,
            });
        }
        Ok(out)
    }
    fn allied(&mut self, actor: Object, allegiance: Option<Object>) -> Result<bool> {
        self.effects
            .allied(actor, allegiance, self.state, self.instances)
    }
    fn remove_property(
        &mut self,
        _actor: Object,
        definition: Object,
        source: Option<Object>,
    ) -> Result<()> {
        remove_with_minimum(
            Selection { definition, source },
            self.state,
            self.instances,
            self.definitions,
            self.minimum,
            self.p,
            self.effects,
        )?;
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn recompute<E: Effects>(
    handle: Object,
    adding: bool,
    state: &mut PropertiesState,
    instances: &mut Instances,
    definitions: &Definitions,
    minimum: Minimum,
    p: Precision,
    effects: &mut E,
) -> Result<()> {
    let id = instances
        .get(&handle)
        .ok_or_else(|| Error::Callback("missing current property instance".into()))?
        .definition;
    let definition = definitions
        .get(&id)
        .ok_or_else(|| Error::Callback("missing current property definition".into()))?;
    for (index, modifier) in definition.modifiers.iter().enumerate() {
        match modifier {
            Modifier::Table(table) => {
                use crate::modifier_tables::{InstanceIdentity, add_modifier, remove_modifier};
                let modifier_id = crate::properties::ModifierId {
                    definition: id,
                    index: index as u32,
                };
                let actor = Some(state.owner);
                let resolved = |gtid: u32, e: &E| -> Result<u32> {
                    Ok(if e.resolve_source(gtid)?.is_some() {
                        gtid
                    } else {
                        0
                    })
                };
                //502B30 case6 alone resolves current instance identities.
                //Other table bodies and all502C60 removal cases never read
                //them. Earlier actor callbacks may have changed this record.
                let identity = if adding
                    && matches!(
                        table,
                        crate::modifier_tables::TableModifier::PeriodicDamage { .. }
                    ) {
                    let current = instances.get(&handle).ok_or_else(|| {
                        Error::Callback("current periodic-damage instance disappeared".into())
                    })?;
                    InstanceIdentity {
                        source_id: resolved(current.source_id, effects)?,
                        allegiance_id: match current.enchantment_allegiance {
                            Some(gtid) => resolved(gtid, effects)?,
                            None => 0,
                        },
                    }
                } else {
                    InstanceIdentity {
                        source_id: 0,
                        allegiance_id: 0,
                    }
                };
                let mut world = Tables {
                    state: &mut *state,
                    instances: &mut *instances,
                    definitions,
                    minimum,
                    p,
                    effects: &mut *effects,
                };
                if adding {
                    add_modifier(modifier_id, table, identity, actor, &mut world, p)?;
                } else {
                    remove_modifier(modifier_id, table, actor, &mut world, p)?;
                }
            }
            Modifier::Flag(flag) => {
                if recompute_flag(*flag, state, instances, definitions)? {
                    consequence(*flag, state.owner, state, instances, effects)?;
                }
            }
            Modifier::Resistance(resistance) => {
                crate::property_resistance::recompute_with_lazy_minimum(
                    resistance,
                    state,
                    instances,
                    definitions,
                    || minimum.read(effects),
                    p,
                    |m| match m {
                        Modifier::Resistance(r) => Some(r),
                        _ => None,
                    },
                )?
            }
            Modifier::Numeric(numeric) => {
                if life::recompute_numeric_with_lazy_minimum(
                    numeric,
                    state,
                    instances,
                    definitions,
                    || minimum.read(effects),
                    p,
                    |m| match m {
                        Modifier::Numeric(n) => Some(n),
                        _ => None,
                    },
                )? {
                    effects.numeric_changed(numeric, state, instances)?;
                }
            }
        }
    }
    Ok(())
}
///502B30 over an attached instance with actual reached clamp reads. This
/// preserves earlier descriptor callbacks and numeric/resistance partial stores.
#[allow(clippy::too_many_arguments)]
pub fn add_modifiers<E: Effects>(
    handle: Object,
    state: &mut PropertiesState,
    instances: &mut Instances,
    definitions: &Definitions,
    p: Precision,
    effects: &mut E,
) -> Result<()> {
    recompute(
        handle,
        true,
        state,
        instances,
        definitions,
        Minimum::Current,
        p,
        effects,
    )
}

pub fn insert<E: Effects>(
    instance: Instance,
    state: &mut PropertiesState,
    instances: &mut Instances,
    definitions: &Definitions,
    minimum: F32,
    p: Precision,
    effects: &mut E,
) -> Result<bool> {
    life::insert_with(
        instance,
        state,
        instances,
        definitions,
        effects,
        |handle, state, instances, definitions, effects| {
            recompute(
                handle,
                true,
                state,
                instances,
                definitions,
                Minimum::Fixed(minimum),
                p,
                effects,
            )
        },
    )
}
pub fn remove<E: Effects>(
    selection: Selection,
    state: &mut PropertiesState,
    instances: &mut Instances,
    definitions: &Definitions,
    minimum: F32,
    p: Precision,
    effects: &mut E,
) -> Result<bool> {
    remove_with_minimum(
        selection,
        state,
        instances,
        definitions,
        Minimum::Fixed(minimum),
        p,
        effects,
    )
}
fn remove_with_minimum<E: Effects>(
    selection: Selection,
    state: &mut PropertiesState,
    instances: &mut Instances,
    definitions: &Definitions,
    minimum: Minimum,
    p: Precision,
    effects: &mut E,
) -> Result<bool> {
    life::remove_with(
        selection,
        state,
        instances,
        definitions,
        effects,
        |handle, state, instances, definitions, effects| {
            recompute(
                handle,
                false,
                state,
                instances,
                definitions,
                minimum,
                p,
                effects,
            )
        },
    )
}
