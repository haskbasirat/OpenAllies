//! Concrete PropertiesComponent/Modifier defaults and native numeric helpers.
//! Active effect handles and unresolved table handles refer to runtime-owned
//! records; this module never treats native process pointers as portable data.
use crate::{Error, F32, Result, credit::Object};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageFactor {
    pub damage_type: Object,
    pub factor: F32,
    /// Native resistance entry+8, unclamped aggregate minus its base. None is
    /// used only by legacy getter-only fixtures where this word was not read.
    #[serde(default)]
    pub resistance: Option<F32>,
}

/// A loaded modifier within its property definition's `+1C` list: the
/// portable identity of the native modifier object the tables point to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ModifierId {
    pub definition: Object,
    pub index: u32,
}
/// One `+98` record (5015C0): the modifier (its `+C` damage type and `+10`
/// amount) with the instance's source and allegiance GTIDs (zero when the
/// identity no longer resolves, or for a non-enchantment allegiance).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodicDamageEntry {
    pub modifier: ModifierId,
    pub damage_type: Object,
    pub amount: F32,
    pub source_id: u32,
    pub allegiance_id: u32,
}
/// One `+A4` entry: the PERIODIC_HEAL modifier and its `+C` amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodicHealEntry {
    pub modifier: ModifierId,
    pub amount: F32,
}
/// One `+B0` entry: the created Ability and its `+4` definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReverseAbilityEntry {
    pub ability: Object,
    pub definition: Object,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifierState {
    pub flags: u32,
    /// Native +4..+64 inclusive. Defaults: +30=-1, +38..+64=1, others0.
    pub scalars: [F32; 25],
    /// +68 ordered type list; None is the actual native wildcard zero entry.
    pub immunity_types: Vec<Option<Object>>,
    /// +74 ordered entries; first matching identity wins, including duplicates.
    pub damage_factors: Vec<DamageFactor>,
    /// +80 resource production in Resource registry order (501F60).
    #[serde(default)]
    pub production: Vec<F32>,
    /// +8C/+90 ability-effect lists by AbilityType registry index, each head
    /// first (502AD0 prepends; 501EF0 unlinks).
    #[serde(default)]
    pub ability_effects: Vec<Vec<Object>>,
    /// +98 periodic damage records (502560/5015C0; read by the Properties
    /// tick through 501690/5016B0/5016D0/5016F0).
    #[serde(default)]
    pub periodic_damage: Vec<PeriodicDamageEntry>,
    /// +A4 periodic heal modifiers (read by the tick through 501710).
    #[serde(default)]
    pub periodic_heals: Vec<PeriodicHealEntry>,
    /// +B0 reverse abilities (502640: a new 4C86D0 Ability per modifier).
    #[serde(default)]
    pub reverse_abilities: Vec<ReverseAbilityEntry>,
    /// Original initial vector growth metadata; these are not storage limits.
    pub growth_hints: [u32; 7],
}
impl ModifierState {
    /// Exact502750 constructor. Fourth vector metadata comes from the loaded
    /// definition domain at globalB36530+3A8/+28, rather than a guessed count.
    pub fn new(definition_domain_count: u32) -> Self {
        let mut scalars = [F32::ZERO; 25];
        scalars[11] = F32(0xbf80_0000);
        scalars[13..].fill(F32::ONE);
        Self {
            flags: 0,
            scalars,
            immunity_types: vec![],
            damage_factors: vec![],
            production: vec![],
            ability_effects: vec![],
            periodic_damage: vec![],
            periodic_heals: vec![],
            reverse_abilities: vec![],
            growth_hints: [1, 1, 8, definition_domain_count, 1, 1, 1],
        }
    }
    pub fn scalar(&self, offset: u32) -> Result<F32> {
        if !(4..=0x64).contains(&offset) || !offset.is_multiple_of(4) {
            return Err(Error::Callback(format!(
                "invalid native modifier scalar offset {offset:#x}"
            )));
        }
        Ok(self.scalars[(offset / 4 - 1) as usize])
    }
    /// Native501610. A zero entry is a wildcard, not a missing loaded type.
    pub fn immune_to(&self, effect_type: Object) -> bool {
        self.immunity_types
            .iter()
            .any(|id| id.is_none() || *id == Some(effect_type))
    }
    pub fn damage_factor(&self, damage_type: Object) -> F32 {
        self.damage_factors
            .iter()
            .find(|entry| entry.damage_type == damage_type)
            .map_or(F32::ONE, |entry| entry.factor)
    }
    /// Native501530: first positive+58, otherwise positive+5C, otherwise1.
    pub fn offense_factor(&self) -> Result<F32> {
        for offset in [0x58, 0x5c] {
            let value = self.scalar(offset)?;
            if value.compare(F32::ZERO)? == Ordering::Greater {
                return Ok(value);
            }
        }
        Ok(F32::ONE)
    }
    /// Exact unrounded multiplication getters5014C0/5014E0/5014F0/501500/501520.
    pub fn multiply(&self, offset: u32, base: F32, p: Precision) -> Result<Binary> {
        if ![0x38, 0x3c, 0x44, 0x50, 0x54].contains(&offset) {
            return Err(Error::Callback(
                "unsupported modifier multiply getter".into(),
            ));
        }
        Ok(base.number()?.mul(&self.scalar(offset)?.number()?, p)?)
    }
    /// Exact unrounded division getters5014D0/501510/501560.
    pub fn divide(&self, offset: u32, base: F32, p: Precision) -> Result<Binary> {
        if ![0x40, 0x48, 0x64].contains(&offset) {
            return Err(Error::Callback("unsupported modifier divide getter".into()));
        }
        Ok(base.number()?.div(&self.scalar(offset)?.number()?, p)?)
    }
    pub fn health_maximum(&self, base: F32, p: Precision) -> Result<Binary> {
        self.multiply(0x50, base, p)
    }
    pub fn recharge(&self, base: F32, p: Precision) -> Result<Binary> {
        self.divide(0x64, base, p)
    }
    ///501B10..501DB0: store the adjusted base to f32, then floor to
    /// min(base,setting), then at0. Offset+24 (501CF0) subtracts its delta;
    /// the other listed getters add it. The global setting is explicit.
    pub fn adjust_with_floor(
        &self,
        offset: u32,
        base: F32,
        minimum: F32,
        p: Precision,
    ) -> Result<F32> {
        if !(0x10..=0x2c).contains(&offset) || !offset.is_multiple_of(4) {
            return Err(Error::Callback(
                "unsupported modifier additive getter".into(),
            ));
        }
        let base_value = base.number()?;
        let delta = self.scalar(offset)?.number()?;
        let adjusted = if offset == 0x24 {
            base_value.sub(&delta, p)?
        } else {
            base_value.add(&delta, p)?
        };
        let mut value = F32(adjusted.store_f32()?);
        let floor = if base.compare(minimum)? == Ordering::Less {
            base
        } else {
            minimum
        };
        if value.compare(floor)? == Ordering::Less {
            value = floor;
        }
        if value.compare(F32::ZERO)? == Ordering::Less {
            value = F32::ZERO;
        }
        Ok(value)
    }
    /// Native501A30 (melee, scalar +8) / 501AA0 (ranged, scalar +C): the
    /// extended sum base+kind, plus scalar +4 stored to binary32, raised to
    /// min(extended base+kind, clamp) stored to binary32, then to0. `clamp`
    /// is setting ModifierAttributeClampAdd (settings +1A4).
    pub fn power(&self, ranged: bool, base: F32, clamp: F32, p: Precision) -> Result<F32> {
        let kind = self.scalar(if ranged { 0xc } else { 8 })?.number()?;
        let x = base.number()?.add(&kind, p)?;
        let mut value = F32(x.add(&self.scalar(4)?.number()?, p)?.store_f32()?);
        let clamp_value = clamp.number()?;
        let floor = if x.compare(&clamp_value) == Ordering::Less {
            x
        } else {
            clamp_value
        };
        if value.number()?.compare(&floor) == Ordering::Less {
            value = F32(floor.store_f32()?);
        }
        if value.compare(F32::ZERO)? == Ordering::Less {
            return Ok(F32::ZERO);
        }
        Ok(value)
    }
    pub fn defense(&self, base: F32, minimum: F32, p: Precision) -> Result<F32> {
        self.adjust_with_floor(0x10, base, minimum, p)
    }
    pub fn projectile_range(&self, base: F32, minimum: F32, p: Precision) -> Result<F32> {
        self.adjust_with_floor(0x28, base, minimum, p)
    }
    pub fn bombard_range(&self, base: F32, minimum: F32, p: Precision) -> Result<F32> {
        self.adjust_with_floor(0x2c, base, minimum, p)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertiesState {
    pub owner: Object,
    /// Base scheduled-component +C. Constructor starts unlinked.
    pub scheduler_link: Option<Object>,
    pub effects: Vec<Object>,
    pub effect_growth_hint: u32,
    pub modifier: ModifierState,
    pub modifier_flags: u32,
}
impl PropertiesState {
    ///5812F0 plus4DB020/50AB90/502750. Vec allocation details are not portable
    ///native addresses; allocation failure remains explicit at this constructor.
    pub fn new(owner: Object, definition_domain_count: u32) -> Result<Self> {
        let mut effects = Vec::new();
        effects
            .try_reserve_exact(2)
            .map_err(|e| Error::Callback(format!("property vector allocation failed: {e}")))?;
        Ok(Self {
            owner,
            scheduler_link: None,
            effects,
            effect_growth_hint: 2,
            modifier: ModifierState::new(definition_domain_count),
            modifier_flags: 0,
        })
    }
}
