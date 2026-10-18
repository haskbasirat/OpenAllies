//! Native weapon-hit calculation and callers of the reconstructed Body system.
//! Actor identity, rules, components and delivery remain required world services.
//! Phase 8 adds morale/attack efficiency (`morale`), ROE damage history (`roe`),
//! capture (`capture`), death queue (`death`), reactive engagement
//! (`engagement`), shortfall attrition (`shortfall`) and combat reactions
//! (`reactions`); round 2 adds organization maintenance and member removal
//! (`organization`), ownership change (`ownership`), under-siege state
//! (`siege`), denizen intruders (`denizen`) and damage numbers
//! (`presentation`); round 3 adds combat value evaluation and means
//! (`combat_value`, on the ported CRT pow). See research/phase8-combat/README.txt
//! and inventory.json.
#![forbid(unsafe_code)]

pub mod ability;
pub mod admission;
pub mod begin_death;
pub mod capture;
pub mod combat_value;
pub mod commander;
pub mod constructors;
pub mod death;
pub mod denizen;
pub mod engagement;
pub mod execution;
pub mod fx;
pub mod launch;
pub mod morale;
pub mod organization;
pub mod ownership;
pub mod payment;
pub mod performer;
pub mod presentation;
pub mod periodic;
pub mod projectile;
pub mod reactions;
pub mod regeneration;
pub mod roe;
pub mod shortfall;
pub mod special_ops;
pub mod siege;
pub mod statistics_tracking;
pub mod summon;
pub mod surface;
pub mod types;
pub mod walls;
pub mod weights;

use aa_body::{
    F32,
    damage::{DamageRecord, DamageRequest, WeaponPropagation},
};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error(transparent)]
    Pow(#[from] aa_numeric::crt_pow::CrtPowError),
    #[error("invalid native combat input: {0}")]
    Invalid(&'static str),
    #[error("required combat world service failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct DamageRule {
    pub kind: u32,
    pub damage_type: u32,
    pub amount: F32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Weapon {
    pub id: u32,
    /// Native weapon/action instance+20; independent of the rule's base amount.
    pub amount: F32,
    /// Native weapon definition+54: bit0 ignores defense, bit1 ignores offense.
    pub flags: u32,
    pub propagation: WeaponPropagation,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Hit {
    pub rule: DamageRule,
    pub weapon: Option<Weapon>,
    pub event_kind: u32,
    /// These are world object handles. Their current GTID is read at dispatch.
    pub source_actor: Option<u32>,
    pub allegiance_actor: Option<u32>,
    pub target_actor: Option<u32>,
    pub scale: F32,
    pub time: F32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub actor: u32,
    pub body: u32,
}

pub trait World {
    /// 4C8330 actor-eligibility virtual call plus the required actor+60 Body.
    fn resolve_target(&mut self, actor: Option<u32>) -> Result<Option<Target>>;
    fn actor_id(&self, actor: u32) -> Result<u32>;
    fn modifier(&self, actor: u32) -> Result<Option<u32>>;
    fn damage_type_factor(&self, modifier: u32, damage_type: u32) -> Result<Binary>;
    fn offensive_component(&self, source_actor: u32) -> Result<Option<u32>>;
    /// 57DBF0 may return an extended value; do not round it prematurely.
    fn offensive_factor(&self, component: u32) -> Result<Binary>;
    fn defense(&self, body: u32) -> Result<F32>;
    fn minimum_weapon_damage(&self) -> Result<F32>;
    /// Must call the real Body damage operation for the resolved target.
    fn deliver(
        &mut self,
        target: Target,
        request: DamageRequest,
        precision: Precision,
    ) -> Result<()>;
}

fn number(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}
fn nonnegative(value: F32) -> Result<F32> {
    if number(value)?.compare(&Binary::from_i32(0)) == Ordering::Less {
        Ok(F32::ZERO)
    } else {
        Ok(value)
    }
}

/// Native 0x501530 offensive modifier preference: first positive primary,
/// otherwise positive secondary, otherwise1.0. Organization leader factors
/// selected by57DBF0 bypass this preference and may themselves be nonpositive.
pub fn offensive_modifier(primary: F32, secondary: F32) -> Result<F32> {
    if number(primary)?.compare(&Binary::from_i32(0)) == Ordering::Greater {
        Ok(primary)
    } else if number(secondary)?.compare(&Binary::from_i32(0)) == Ordering::Greater {
        Ok(secondary)
    } else {
        Ok(F32::ONE)
    }
}

/// Complete finite 0x4CA400 hit arithmetic and downstream call arguments.
/// True means a Body received the call, even if that Body ignores its amount.
pub fn hit(hit: Hit, world: &mut impl World, precision: Precision) -> Result<bool> {
    let Some(target) = world.resolve_target(hit.target_actor)? else {
        return Ok(false);
    };
    let source_id = match hit.source_actor {
        Some(actor) => world.actor_id(actor)?,
        None => 0,
    };
    let allegiance_id = match hit.allegiance_actor {
        Some(actor) => world.actor_id(actor)?,
        None => 0,
    };
    let base = nonnegative(if hit.rule.kind == 4 {
        hit.weapon
            .ok_or(Error::Invalid("kind4 hit requires a weapon"))?
            .amount
    } else {
        hit.rule.amount
    })?;
    let type_factor = if let Some(modifier) = world.modifier(target.actor)? {
        // Native stores this getter result to binary32 before the hit formula.
        F32(world
            .damage_type_factor(modifier, hit.rule.damage_type)?
            .store_f32()?)
    } else {
        F32::ONE
    };
    let amount = if hit.rule.kind == 4 {
        let weapon = hit
            .weapon
            .ok_or(Error::Invalid("kind4 hit requires a weapon"))?;
        let mut offense = if let Some(source) = hit.source_actor {
            if let Some(component) = world.offensive_component(source)? {
                world.offensive_factor(component)?
            } else {
                number(F32::ONE)?
            }
        } else {
            number(F32::ONE)?
        };
        let mut defense = nonnegative(world.defense(target.body)?)?;
        if weapon.flags & 2 != 0 {
            offense = number(F32::ONE)?;
        }
        if weapon.flags & 1 != 0 {
            defense = F32::ZERO;
        }
        let stored = F32(offense
            .mul(&number(base)?, precision)?
            .mul(&number(hit.scale)?, precision)?
            .sub(&number(defense)?, precision)?
            .mul(&number(type_factor)?, precision)?
            .store_f32()?);
        let minimum = world.minimum_weapon_damage()?;
        if number(stored)?.compare(&number(minimum)?) == Ordering::Less {
            minimum
        } else {
            stored
        }
    } else {
        F32(number(type_factor)?
            .mul(&number(base)?, precision)?
            .store_f32()?)
    };
    world.deliver(
        target,
        DamageRequest {
            record: DamageRecord {
                source_id,
                allegiance_id,
                damage_type: hit.rule.damage_type,
                amount,
                time: hit.time,
                event_kind: hit.event_kind,
                reason: if hit.rule.kind == 4 { 4 } else { 5 },
            },
            // 4CA400 already applied the modifier; passing true would apply it twice.
            apply_type_modifier: false,
            weapon_id: hit.weapon.map(|weapon| weapon.id),
            weapon_definition: hit.weapon.map(|weapon| weapon.propagation),
            propagate_to_neighbors: true,
        },
        precision,
    )?;
    Ok(true)
}
pub mod organization_combat_value;
pub mod combat_mean_coordinator;
pub mod performer_property_gate;
