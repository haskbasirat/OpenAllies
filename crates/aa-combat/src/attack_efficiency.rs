//! Performer attack efficiency.
//!
//! - `0x57DBF0`: the shooter's offense factor. A member of an organization
//!   (actor `+80` membership with its `+10` organization actor) takes the
//!   organization's attack efficiency (that actor's `+7C` component `+84`);
//!   anything else takes its own modifier preference (`0x4CF110`, the actor
//!   `+78` holder's set at `+1C`, then `0x501530`: `+58` when positive, else
//!   `+5C` when positive, else 1.0), or 1.0 without modifiers. Every path
//!   loads one binary32, so the x87 result is that binary32 exactly.
//!
//! Proven at PC24, PC53 and PC64 by
//! research/phase8-combat/native_attack_efficiency.py
//! (tests/native_attack_efficiency.rs).
use crate::{Result, offensive_modifier};
use aa_body::F32;

/// Services of the attack efficiency.
pub trait AttackEfficiencyWorld {
    /// Actor `+80` membership and its `+10` organization actor, when both
    /// are set.
    fn organization_actor(&self, actor: u32) -> Result<Option<u32>>;
    /// Organization actor `+7C` component `+84`.
    fn organization_attack_efficiency(&self, organization: u32) -> Result<F32>;
    /// `0x4CF110`: the actor's modifier set (`+78` holder `+1C`), as its
    /// `+58` and `+5C` words; None without a holder.
    fn offense_modifiers(&self, actor: u32) -> Result<Option<[F32; 2]>>;
}

/// Native `0x57DBF0` for the Performer's owner `actor`.
pub fn attack_efficiency(actor: u32, world: &impl AttackEfficiencyWorld) -> Result<F32> {
    if let Some(organization) = world.organization_actor(actor)? {
        return world.organization_attack_efficiency(organization);
    }
    match world.offense_modifiers(actor)? {
        Some([primary, secondary]) => offensive_modifier(primary, secondary),
        None => Ok(F32::ONE),
    }
}
