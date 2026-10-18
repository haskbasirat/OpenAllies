//! ModifierState Add/RemovePropertyModifiers (502B30 / 502C60) for the
//! table modifier kinds: ENCHANTMENT_IMMUNITY (+68, with the 4FE6F0 purge),
//! ABILITY_EFFECT (+8C lists, 502AD0 / 501EF0), PERIODIC_DAMAGE (+98,
//! 502560 / 501820), PERIODIC_HEAL (+A4, 5018C0), REVERSE_ABILITY (+B0,
//! 502640 / 5023B0) and RESOURCE_PRODUCTION (+80, 501F60). FLAG/ADD/
//! MULTIPLY/RESIST_DAMAGE stay the recompute path (502A70).
//! Proven by research/phase8-combat/native_modifier_tables.py.
use crate::{
    Error, F32, Result,
    credit::Object,
    properties::{
        ModifierId, ModifierState, PeriodicDamageEntry, PeriodicHealEntry, ReverseAbilityEntry,
    },
};
use aa_numeric::Precision;
use serde::{Deserialize, Serialize};

/// The table kinds' loaded fields (4C4030 per kind).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableModifier {
    /// 5010A0: `+C` EnchantmentType (`None` = null, matching untyped
    /// enchantments).
    Immunity { enchantment_type: Option<Object> },
    /// 501150: `+C` AbilityType (its `+C` registry index), `+10` effect.
    AbilityEffect {
        ability_type_index: u32,
        effect: Object,
    },
    /// 501350: `+C` damage type, `+10` amount.
    PeriodicDamage { damage_type: Object, amount: F32 },
    /// 4FEE90: `+C` amount.
    PeriodicHeal { amount: F32 },
    /// 501410: `+C` AbilityDefinition.
    ReverseAbility { definition: Object },
    /// 4FEEC0: `+C` dense values in Resource registry order.
    ResourceProduction { values: Vec<F32> },
}

/// The instance whose property's modifiers are added (502B30's first
/// argument): its resolved `+14` source and, for enchantment instances,
/// resolved `+18` allegiance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceIdentity {
    /// `+14` GTID when it still resolves, else 0.
    pub source_id: u32,
    /// Enchantment instances: `+18` GTID when it resolves, else 0. Plain
    /// instances (virtual +4 != 1) contribute 0.
    pub allegiance_id: u32,
}

/// One actor property instance as 4FE6F0 reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorInstance {
    pub definition: Object,
    /// Instance virtual `+4` == 1 (vtable A2EBE0).
    pub enchantment: bool,
    /// Definition `+64` (enchantment type).
    pub enchantment_type: Option<Object>,
    /// Resolved `+14` source.
    pub source: Option<Object>,
    /// Resolved `+18` allegiance (enchantment instances).
    pub allegiance: Option<Object>,
}

pub trait TableWorld {
    /// The ModifierState being edited (re-borrowed after every callback).
    fn modifier_state(&mut self) -> &mut ModifierState;
    /// `[B36530]+38` Resource registry count.
    fn resource_count(&self) -> Result<usize>;
    /// 4CF0F0 on the actor (after a production change).
    fn refresh_actor(&mut self, actor: Object) -> Result<()>;
    /// 502640: a new Ability for `definition` (4C86D0, main power 4BE3F0(0)).
    fn create_reverse_ability(&mut self, definition: Object) -> Result<Object>;
    /// 5023B0: 4CE080 (summoned list freed) and delete.
    fn destroy_reverse_ability(&mut self, ability: Object) -> Result<()>;
    /// The actor's `+78` Properties `+10/+14` instance list, current order.
    fn actor_instances(&mut self, actor: Object) -> Result<Vec<ActorInstance>>;
    /// Actor virtual `+138(allegiance)`.
    fn allied(&mut self, actor: Object, allegiance: Option<Object>) -> Result<bool>;
    /// 581810(property, source) on the actor's Properties.
    fn remove_property(
        &mut self,
        actor: Object,
        definition: Object,
        source: Option<Object>,
    ) -> Result<()>;
}

/// 501F60(modifier, actor, add): the `+80` table is first resized and
/// zero-filled to the registry count when its size differs; then each
/// entry is `entry + value` or `entry - value` (binary32 stores); the actor
/// is refreshed (4CF0F0) when present.
fn production(
    values: &[F32],
    actor: Option<Object>,
    add: bool,
    world: &mut impl TableWorld,
    p: Precision,
) -> Result<()> {
    let count = world.resource_count()?;
    if values.len() < count {
        return Err(Error::Callback(
            "resource production modifier shorter than the registry".into(),
        ));
    }
    let table = &mut world.modifier_state().production;
    if table.len() != count {
        table.clear();
        table.resize(count, F32::ZERO);
    }
    for (entry, value) in table.iter_mut().zip(values) {
        let current = entry.number()?;
        let delta = value.number()?;
        let next = if add {
            delta.add(&current, p)?
        } else {
            current.sub(&delta, p)?
        };
        *entry = F32(next.store_f32()?);
    }
    if let Some(actor) = actor {
        world.refresh_actor(actor)?;
    }
    Ok(())
}

/// 4FE6F0(actor), the immunity modifier's `+20`: every enchantment instance
/// of the immune type whose allegiance the actor is not allied with is
/// removed (581810 by definition and source); the scan restarts from the
/// first entry after each removal, bounded by the decremented initial count.
pub fn purge_hostile_enchantments(
    actor: Object,
    enchantment_type: Option<Object>,
    world: &mut impl TableWorld,
) -> Result<()> {
    let mut count = world.actor_instances(actor)?.len();
    let mut index = 0;
    while index < count {
        let instances = world.actor_instances(actor)?;
        let entry = *instances.get(index).ok_or_else(|| {
            Error::Callback("Properties list shrank during the immunity purge".into())
        })?;
        if entry.enchantment
            && entry.enchantment_type == enchantment_type
            && !world.allied(actor, entry.allegiance)?
        {
            world.remove_property(actor, entry.definition, entry.source)?;
            count -= 1;
            index = 0;
            continue;
        }
        index += 1;
    }
    Ok(())
}

/// 502B30's case for one table modifier of `instance`.
pub fn add_modifier(
    id: ModifierId,
    modifier: &TableModifier,
    instance: InstanceIdentity,
    actor: Option<Object>,
    world: &mut impl TableWorld,
    p: Precision,
) -> Result<()> {
    match modifier {
        TableModifier::Immunity { enchantment_type } => {
            world
                .modifier_state()
                .immunity_types
                .push(*enchantment_type);
            if let Some(actor) = actor {
                purge_hostile_enchantments(actor, *enchantment_type, world)?;
            }
        }
        TableModifier::AbilityEffect {
            ability_type_index,
            effect,
        } => {
            let lists = &mut world.modifier_state().ability_effects;
            let index = *ability_type_index as usize;
            if lists.len() <= index {
                lists.resize(index + 1, Vec::new());
            }
            lists[index].insert(0, *effect);
        }
        TableModifier::PeriodicDamage {
            damage_type,
            amount,
        } => world
            .modifier_state()
            .periodic_damage
            .push(PeriodicDamageEntry {
                modifier: id,
                damage_type: *damage_type,
                amount: *amount,
                source_id: instance.source_id,
                allegiance_id: instance.allegiance_id,
            }),
        TableModifier::PeriodicHeal { amount } => {
            world
                .modifier_state()
                .periodic_heals
                .push(PeriodicHealEntry {
                    modifier: id,
                    amount: *amount,
                })
        }
        TableModifier::ReverseAbility { definition } => {
            let ability = world.create_reverse_ability(*definition)?;
            world
                .modifier_state()
                .reverse_abilities
                .push(ReverseAbilityEntry {
                    ability,
                    definition: *definition,
                });
        }
        TableModifier::ResourceProduction { values } => {
            production(values, actor, true, world, p)?;
        }
    }
    Ok(())
}

/// 502C60's case for one table modifier.
pub fn remove_modifier(
    id: ModifierId,
    modifier: &TableModifier,
    actor: Option<Object>,
    world: &mut impl TableWorld,
    p: Precision,
) -> Result<()> {
    match modifier {
        // 5017D0: the first equal type.
        TableModifier::Immunity { enchantment_type } => {
            let types = &mut world.modifier_state().immunity_types;
            if let Some(i) = types.iter().position(|t| t == enchantment_type) {
                types.remove(i);
            }
        }
        // 501EF0: unlink the first node holding the effect (within range).
        TableModifier::AbilityEffect {
            ability_type_index,
            effect,
        } => {
            if let Some(list) = world
                .modifier_state()
                .ability_effects
                .get_mut(*ability_type_index as usize)
                && let Some(i) = list.iter().position(|e| e == effect)
            {
                list.remove(i);
            }
        }
        // 501820: the record of this modifier.
        TableModifier::PeriodicDamage { .. } => {
            let records = &mut world.modifier_state().periodic_damage;
            if let Some(i) = records.iter().position(|r| r.modifier == id) {
                records.remove(i);
            }
        }
        // 5018C0.
        TableModifier::PeriodicHeal { .. } => {
            let heals = &mut world.modifier_state().periodic_heals;
            if let Some(i) = heals.iter().position(|r| r.modifier == id) {
                heals.remove(i);
            }
        }
        // 5023B0: the first Ability with this definition.
        TableModifier::ReverseAbility { definition } => {
            let abilities = &mut world.modifier_state().reverse_abilities;
            if let Some(i) = abilities.iter().position(|a| a.definition == *definition) {
                let entry = abilities.remove(i);
                world.destroy_reverse_ability(entry.ability)?;
            }
        }
        TableModifier::ResourceProduction { values } => {
            production(values, actor, false, world, p)?;
        }
    }
    Ok(())
}
