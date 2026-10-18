//! Native Properties callback5818D0, including expired-property removal and
//! ordered periodic healing/damage. Outer world/component effects are required.
use crate::{Error, Result};
use aa_body::{
    F32,
    damage::{DamageRecord, DamageRequest},
};
use aa_numeric::Precision;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effect {
    pub handle: u32,
    pub property: u32,
    pub source_id: u32,
    pub disabled: bool,
    pub property_flags: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodicDamage {
    pub source_id: u32,
    pub allegiance_id: u32,
    pub damage_type: u32,
    pub amount: F32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Properties {
    pub owner: u32,
    pub effects: Vec<Effect>,
    pub modifier_flags: u32,
    pub healing: Vec<F32>,
    pub damage: Vec<PeriodicDamage>,
}
/// The tick's `healing` and `damage` lists from the ModifierState tables, as
/// 5818D0 reads them: `+A4` heal modifiers' `+C` amounts (501710) and `+98`
/// records' damage type / amount (501690 / 5016B0) with their source and
/// allegiance GTIDs (5016D0 / 5016F0). The tables are maintained by
/// `aa_body::modifier_tables` (502B30 / 502C60).
pub fn periodic_from_tables(
    modifier: &aa_body::properties::ModifierState,
) -> (Vec<F32>, Vec<PeriodicDamage>) {
    let healing = modifier.periodic_heals.iter().map(|h| h.amount).collect();
    let damage = modifier
        .periodic_damage
        .iter()
        .map(|r| PeriodicDamage {
            source_id: r.source_id,
            allegiance_id: r.allegiance_id,
            damage_type: r.damage_type.0,
            amount: r.amount,
        })
        .collect();
    (healing, damage)
}

/// Effect membership/fields remain stable while expiry and source resolution
/// inspect an entry. Removed-effect callbacks may recompute healing/damage
/// vectors. Their lengths are sampled at each native phase boundary; shrinking
/// a sampled vector before the remaining reads is rejected instead of reproducing
/// invalid native memory access. Recursive same-Properties ticking is unsupported.
pub trait World {
    fn expired(&mut self, effect: &Effect) -> Result<bool>;
    fn resolve_source(&self, id: u32) -> Result<Option<u32>>;
    fn actor_effect_removed(&mut self, state: &mut Properties, effect: &Effect) -> Result<()>;
    fn effect_removed(&mut self, state: &mut Properties, effect: &Effect) -> Result<()>;
    fn recompute_modifier(&mut self, state: &mut Properties, effect: &Effect) -> Result<()>;
    fn release_effect(&mut self, effect: &Effect) -> Result<()>;
    fn body(&self, actor: u32) -> Result<Option<u32>>;
    fn outer_actor(&mut self, actor: u32) -> Result<Option<u32>>;
    fn ancestor(&self, actor: u32) -> Result<Option<u32>>;
    fn actor_flags(&self, actor: u32) -> Result<u32>;
    fn heal(
        &mut self,
        state: &mut Properties,
        body: u32,
        amount: F32,
        precision: Precision,
    ) -> Result<()>;
    fn now(&self) -> Result<F32>;
    fn damage(
        &mut self,
        state: &mut Properties,
        body: u32,
        request: DamageRequest,
        precision: Precision,
    ) -> Result<()>;
}

/// 5034F0 plus its5032D0 duplicate-property selection. Source identities are
/// generation-resolved before comparison; two stale identities both compare null.
pub fn remove_property(
    state: &mut Properties,
    property: u32,
    source: Option<u32>,
    world: &mut impl World,
) -> Result<Option<Effect>> {
    let mut selected = None;
    for (index, effect) in state.effects.iter().enumerate() {
        if effect.property == property && world.resolve_source(effect.source_id)? == source {
            selected = Some(index);
            break;
        }
    }
    let Some(index) = selected else {
        return Ok(None);
    };
    let removed = state.effects.remove(index);
    if !removed.disabled {
        if let Some(replacement) = state
            .effects
            .iter_mut()
            .find(|effect| effect.property == property)
        {
            replacement.disabled = false;
        } else {
            return Ok(Some(removed));
        }
    }
    world.release_effect(&removed)?;
    Ok(None)
}

/// Complete finite5818D0 sequencing. This is a world-dispatched callback, not
/// a timer or a frame-rate-dependent update invented by the reconstruction.
pub fn tick(state: &mut Properties, world: &mut impl World, precision: Precision) -> Result<()> {
    let mut index = 0;
    while index < state.effects.len() {
        let effect = state.effects[index].clone();
        if world.expired(&effect)? {
            let source = world.resolve_source(effect.source_id)?;
            if let Some(removed) = remove_property(state, effect.property, source, world)? {
                if removed.property_flags != 0 {
                    state.modifier_flags = state
                        .effects
                        .iter()
                        .filter(|effect| !effect.disabled)
                        .fold(0, |flags, effect| flags | effect.property_flags);
                }
                world.actor_effect_removed(state, &removed)?;
                world.effect_removed(state, &removed)?;
                world.recompute_modifier(state, &removed)?;
                world.release_effect(&removed)?;
            }
        }
        // Native increments after removal; the shifted next entry is skipped.
        index += 1;
    }
    let Some(body) = world.body(state.owner)? else {
        return Ok(());
    };
    let mut heal_allowed = true;
    if let Some(outer) = world.outer_actor(state.owner)? {
        // Native resolves ancestor even when outer flags will block healing.
        let ancestor = world.ancestor(outer)?;
        heal_allowed = world.actor_flags(outer)? & 0x100000 == 0
            && match ancestor {
                Some(actor) => world.actor_flags(actor)? & 0x40000000 == 0,
                None => true,
            };
    }
    if heal_allowed {
        heal_allowed = world.actor_flags(state.owner)? & (0x100000 | 0x40000000) == 0;
    }
    if heal_allowed {
        let count = state.healing.len();
        for index in 0..count {
            let amount = *state
                .healing
                .get(index)
                .ok_or(Error::Invalid("healing list shrank during callback"))?;
            world.heal(state, body, amount, precision)?;
        }
    }
    let count = state.damage.len();
    for index in 0..count {
        let damage = *state
            .damage
            .get(index)
            .ok_or(Error::Invalid("damage list shrank during callback"))?;
        let record = DamageRecord {
            source_id: damage.source_id,
            allegiance_id: damage.allegiance_id,
            damage_type: damage.damage_type,
            amount: damage.amount,
            time: world.now()?,
            event_kind: 2,
            reason: 10,
        };
        world.damage(
            state,
            body,
            DamageRequest {
                record,
                apply_type_modifier: true,
                weapon_id: None,
                weapon_definition: None,
                propagate_to_neighbors: true,
            },
            precision,
        )?;
    }
    Ok(())
}
