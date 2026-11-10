//! An air base's HUD orders: the Denizen actor command handler 0x56E730 for
//! air_recon (0x1A) and air_attack (0x19). Owner: air.
//!
//! 0x4D9B60 delivers a command only when the actor takes it (virtual +0x38:
//! the command mask 0x4D1350, the Denizen's contributor 0x569C70). 0x56E730
//! then, by the command index less 0x17 (its jump table 0x56E838):
//! - air_attack: a target (command flag bit 0) the registry resolves
//!   (0x530F10) goes to 0x56D5A0(target); else a location (flag bit 2) to
//!   0x56D7C0(x, y); else the command fails;
//! - air_recon: a location goes to 0x56DA00(x, y); else the command fails
//!   (an actor target alone carries no location).
//!
//! The purchase is `crate::denizen::purchase` (aa-production
//! `purchase_denizen`): false when the kingdom cannot afford it (the
//! notice), true otherwise, even when no aircraft could be created.
//!
//! **Not yet:** air_attack stays refused here. Its aircraft would take off,
//! but the attack state (`super::attack`) and the release of the bomb
//! (MoveTick's 0x57E030 on the Performer) are not switched on, so a bomber
//! would fly without striking. What the release still needs:
//! - MoveTick (0x56130E..0x5613E1) with an armed ability (flight +0xA4, an
//!   Ability whose +4 definition's +0x54 bit 3 gives the descriptor flag 2;
//!   +0x54 is the flag word 0x4BEE20 builds, `Weapon::flags`: ger_bomb1's
//!   0x9008 has bit 3, bombard):
//!   0x4C8300 (the descriptor: the flags, eleven words cleared), the target
//!   handle +0x94 or else the point +0x98, then 0x4C8F20(descriptor, 1), and
//!   when it answers 0, 0x57E030 on actor+0x74. +0x94 and +0xA4 clear after.
//! - 0x57E030: with a definition +0x44 other than 0, an impulse on the
//!   source (descriptor +8) along the target-to-source line (0x4453A0,
//!   0x50C6F0) when the source's definition has +0x168 bit 1; then the use
//!   0x4C9E90 -> 0x4C9550 from the Performer's start (+0x3C) when the
//!   ability is its current one (+0x34), else from the world time; then the
//!   Performer's mana +0x20 less the definition's +0x40.
//! - The use at a point: `crate::combat` uses an ability on an actor only.
//! - Most of the corpus's air_attack wires name a target (flags 0x51): the
//!   attack on an actor follows it through 0x518A30, not ported.
use crate::books::AirOrder;
use crate::denizen::{COMMAND_AIR_ATTACK, COMMAND_AIR_RECON};
use crate::world::{AbilityTarget, ActorId, KingdomId, Unit, World};

/// 0x4D9B60 -> 0x56E730 for an air order of `kingdom` to `building`; Err is
/// the refusal reason.
pub(crate) fn apply(world: &mut World, kingdom: KingdomId, building: ActorId, target: AbilityTarget, attack: bool) -> Result<(), String> {
    let index = if attack { COMMAND_AIR_ATTACK } else { COMMAND_AIR_RECON };
    if !crate::orders::mask::takes(world, kingdom, Unit::Actor(building), index) {
        // The mask (0x4D1350) refuses it: not the owner, not live, or no
        // aircraft of the order ready.
        return Err(if attack { "the air base does not take air_attack now" } else { "the air base does not take air_recon now" }.into());
    }
    let order = if attack {
        match target {
            AbilityTarget::Actor(t) if world.ids.actor(t).is_some() => AirOrder::AttackActor(t),
            AbilityTarget::Point(p) => AirOrder::AttackPoint(p),
            AbilityTarget::Actor(_) => return Err("no target for the air attack".into()),
        }
    } else {
        match target {
            AbilityTarget::Point(p) => AirOrder::Recon(p),
            AbilityTarget::Actor(_) => return Err("an air recon needs a location".into()),
        }
    };
    if attack {
        let _ = order;
        return Err("air attack: the aircraft attack and its release are not ported yet".into());
    }
    let (paid, _) = crate::denizen::purchase(world, building, order);
    if paid { Ok(()) } else { Err("cannot afford the air recon".into()) }
}
