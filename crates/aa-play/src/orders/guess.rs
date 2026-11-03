//! `KKC_GActorCommand::GuessCommandData` 0x4DA570 over the battle: which
//! ActorCommand a default order means for one selected actor, and the command
//! mask (Actor +F0/+F4, rebuilt by 0x4D1350) the guess reads through
//! `vt+34` = 0x4D1F90.
//!
//! The kernel below is the instruction order of 0x4DA570, the same as the exact
//! port `aa_runtime::player_order_world::guess::guess_command_data` (whose
//! fixtures are `research/phase8-orders-input/guess-4da570/`). aa-play cannot
//! depend on aa-runtime, so it is restated here over `World`.
//!
//! The loop, the requirement mask and the definition fields are native; the command
//! mask is `super::mask`.
//!
//! The controls (`non_group_command` +3C, `can_hot_command_actor` +3F,
//! `can_hot_command_location` +40) are the v14 values of
//! `Data/Game/actor_commands.tgi`, `actor_commands_aa.tgi` and
//! `Game/ActorCommands/*.tgi` with the 0x48CF0A code rule; `GameData` does not
//! load them. The two UVars are their `UVars.tgi` defaults
//! (`SwapRightClickWithAltRightClick` false, `HotkeyAltRightClickCommand`
//! retreat): 0x4DA570 reads them from each node's own profile, so a film played
//! with other settings would already diverge in the original.
use crate::data::Weapon;
use crate::world::{ActorId, Unit, World};
use aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS as COMMANDS;

/// `cmp ebx, 0x27`: the ActorCommand definitions.
pub const DEFINITION_COUNT: u32 = 0x27;
/// The command indices (0x4DA2F0).
pub const REPAIR: u32 = 3;
pub const CONSTRUCT: u32 = 4;
pub const MOVE: u32 = 5;
pub const RETREAT: u32 = 7;
pub const SET_RALLY_POINT: u32 = 16;

/// Requirement mask bits (EBP of 0x4DA570) tested against definition +2C.
pub mod requirement {
    pub const POSITION: u32 = 0x10;
    pub const AUXILIARY: u32 = 0x20;
    pub const TARGET: u32 = 0x100;
    pub const CONSTRUCTION: u32 = 0x200;
    pub const CONSTRUCTION_STATE: u32 = 0x400;
    pub const DAMAGED: u32 = 0x800;
    pub const FRIENDLY: u32 = 0x1000;
    pub const HOSTILE: u32 = 0x2000;
}

/// +3C `non_group_command`: the TGI value, then 0x48CF0A forces it for a
/// command with behaviour bits 16..23 and for unpack, pack_organization, pack.
const NON_GROUP: [u32; 12] = [10, 11, 12, 16, 20, 25, 26, 27, 28, 30, 31, 32];
fn non_group(index: u32) -> bool {
    NON_GROUP.contains(&index)
        || index == 34
        || COMMANDS.get(index as usize).is_some_and(|d| d.behavior_flags & 0x00ff_0000 != 0)
}
/// +3F `can_hot_command_actor`: repair, construct and move.
fn hot_actor(index: u32) -> bool {
    matches!(index, REPAIR | CONSTRUCT | MOVE)
}
/// +40 `can_hot_command_location`: move and set_rally_point.
fn hot_location(index: u32) -> bool {
    matches!(index, MOVE | SET_RALLY_POINT)
}

/// A default order's fields as 0x4DA570 reads them (command +0C, +10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fields {
    pub flags: u32,
    /// The target GTID (flag bit 0).
    pub target: u32,
}

/// Actor +100 bit 21: under construction.
fn under_construction(world: &World, a: ActorId) -> bool {
    world.books.actor(a).is_some_and(|b| b.constructing)
}
fn construction_9c(world: &World, a: ActorId) -> bool {
    world.actors.get(a as usize).is_some_and(|x| world.data.def(x.def).has_component("ConstructionComponent"))
}
/// Body +10 < +14 as an ordered compare, `None` without a body.
fn damaged(world: &World, a: ActorId) -> Option<bool> {
    let x = world.actors.get(a as usize)?;
    if !world.data.def(x.def).has_component("BodyComponent") {
        return None;
    }
    let maximum = world.combat.actor(a).filter(|c| c.initialized && c.max_health > 0.0).map_or_else(|| crate::production::max_health(world, x.kingdom, x.def), |c| c.max_health);
    Some(x.health < maximum)
}

/// The target's part of the requirement mask, and its kingdom.
fn target_mask(world: &World, kingdom: Option<u32>, target: Unit) -> (u32, Option<u32>) {
    use requirement::*;
    let target_kingdom = match target {
        Unit::Org(o) => world.orgs.get(o as usize).map(|o| o.kingdom),
        Unit::Actor(a) => world.actors.get(a as usize).and_then(|x| x.kingdom),
    };
    let mut mask = TARGET;
    let relation = crate::setup::relation(world, kingdom, target_kingdom);
    // vt+140 = 0x4E52A0 (relation 3); vt+138 = 0x4D1EB0 (same owner or relation 1).
    if relation == crate::books::ENEMY {
        mask |= HOSTILE;
    }
    if kingdom == target_kingdom || relation == crate::books::FRIEND {
        mask |= FRIENDLY;
    }
    match target {
        Unit::Org(o) => {
            let members = world.orgs.get(o as usize).map_or(&[][..], |o| &o.members);
            // 0x579FD0, 0x579F70 (0x4D1490: Actor +100 bit 21 clear).
            if members.iter().any(|&m| construction_9c(world, m)) {
                mask |= CONSTRUCTION;
            }
            if members.iter().any(|&m| damaged(world, m) == Some(true) && !under_construction(world, m)) {
                mask |= DAMAGED;
            }
        }
        Unit::Actor(a) => {
            if construction_9c(world, a) {
                mask |= CONSTRUCTION;
                if under_construction(world, a) {
                    mask |= CONSTRUCTION_STATE;
                }
            }
            if damaged(world, a) == Some(true) {
                mask |= DAMAGED;
            }
        }
    }
    (mask, target_kingdom)
}

/// The unit's kingdom (vt+108).
fn unit_kingdom(world: &World, unit: Unit) -> Option<u32> {
    match unit {
        Unit::Org(o) => world.orgs.get(o as usize).map(|o| o.kingdom),
        Unit::Actor(a) => world.actors.get(a as usize).and_then(|x| x.kingdom),
    }
}

/// One ActorCommand definition as 0x4DA570 reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// +0C: the type; 4B4E60 stores the registration index here.
    pub kind: u32,
    /// +2C.
    pub requirements: u32,
    /// +3C.
    pub non_group: bool,
    /// +3F.
    pub hot_actor: bool,
    /// +40.
    pub hot_location: bool,
}
/// The stock row of command `index`.
pub fn row(index: u32) -> Row {
    Row {
        kind: index,
        requirements: COMMANDS.get(index as usize).map_or(0, |d| d.behavior_flags),
        non_group: non_group(index),
        hot_actor: hot_actor(index),
        hot_location: hot_location(index),
    }
}

/// What the loop of 0x4DA570 reads per pass.
pub trait Kernel {
    /// Actor vt+34(index): the command mask.
    fn allowed(&mut self, index: u32) -> bool;
    fn row(&mut self, index: u32) -> Row;
    /// Actor vt+13C(kingdom), only with a target that has a kingdom.
    fn neutral(&mut self, kingdom: u32) -> bool;
    /// Type 4: a construct candidate (0x4D44E0) whose +4E0 is the target's
    /// definition.
    fn construct_candidate(&mut self) -> bool;
}

/// The loop of 0x4DA570 from 0x4DA7F0 on: 39 definitions in order; the mask of
/// the actor, the hot index filter, +3C when `non_group_excluded`, +2C against
/// the requirement mask. Proven against the original's rows by
/// `crates/aa-play/tests/orders.rs` (research/phase8-orders-input/guess-4da570).
pub fn kernel(k: &mut impl Kernel, mask: u32, target_kingdom: Option<u32>, hot: i32, non_group_excluded: bool) -> Option<u32> {
    use requirement::*;
    for index in 0..DEFINITION_COUNT {
        if !k.allowed(index) {
            continue;
        }
        if hot >= 0 && index as i32 != hot {
            continue;
        }
        let row = k.row(index);
        if non_group_excluded && row.non_group {
            continue;
        }
        let matched = row.requirements & mask;
        let hot_index = hot == index as i32;
        let found = if matched & 0xf0 != 0 {
            let wanted = row.requirements & 0xffff_00ff;
            (row.hot_location || hot_index) && matched == wanted
        } else {
            let wanted = row.requirements & 0xffff_ff0f;
            if !row.hot_actor && !hot_index {
                false
            } else if row.kind != CONSTRUCT {
                matched == wanted
            } else {
                (mask | 0x50000) & wanted == wanted && k.construct_candidate()
            }
        };
        if found {
            return Some(index);
        }
        // The vt+13C (neutral) alternative at the end of each pass.
        if let Some(t) = target_kingdom
            && k.neutral(t)
            && (matched | FRIENDLY == mask || matched | HOSTILE == mask)
        {
            return Some(index);
        }
    }
    None
}

struct Live<'a> {
    world: &'a World,
    kingdom: Option<u32>,
    bits: u64,
}
impl Kernel for Live<'_> {
    fn allowed(&mut self, index: u32) -> bool {
        self.bits >> index & 1 != 0
    }
    fn row(&mut self, index: u32) -> Row {
        row(index)
    }
    fn neutral(&mut self, kingdom: u32) -> bool {
        crate::setup::relation(self.world, self.kingdom, Some(kingdom)) == crate::books::NEUTRAL
    }
    /// `Def` does not hold +4E0: this branch finds nothing here.
    fn construct_candidate(&mut self) -> bool {
        false
    }
}

/// 0x4DA570(command, actor, `non_group_excluded`): the command index this unit
/// takes, `None` when it takes none (or the target GTID does not resolve, which
/// the original logs at line 0x12A and rejects).
pub fn guess(world: &World, unit: Unit, fields: Fields, non_group_excluded: bool) -> Option<u32> {
    use requirement::*;
    let flags = fields.flags;
    let mut mask = 0;
    if flags >> 2 & 1 != 0 {
        mask = POSITION;
    }
    if flags >> 3 & 1 != 0 {
        mask |= AUXILIARY;
    }
    let kingdom = unit_kingdom(world, unit);
    let mut target_kingdom = None;
    if flags & 1 != 0 {
        let target = crate::ids::unit(world.ids.thing(fields.target)?)?;
        let (part, k) = target_mask(world, kingdom, target);
        mask |= part;
        target_kingdom = k;
    }
    // Alt = flag bit 10 XOR the (default false) swap UVar; the hot command is
    // HotkeyAltRightClickCommand, by default retreat.
    let hot: i32 = if flags >> 10 & 1 != 0 { RETREAT as i32 } else { -1 };
    let bits = action_bits(world, unit);
    kernel(&mut Live { world, kingdom, bits }, mask, target_kingdom, hot, non_group_excluded)
}

fn enabled(w: &Weapon) -> bool {
    !w.defender
}
/// Ability flag bit 3 (0x57E280).
fn indirect(w: &Weapon) -> bool {
    enabled(w) && w.flags & 8 != 0
}

/// The unit's command mask, 0x4D1350 (`super::mask`).
pub fn action_bits(world: &World, unit: Unit) -> u64 {
    super::mask::action_bits(world, unit)
}

/// 0x578AC0 (0x578A30(1)): a locked layout entry whose element has an ability
/// with flag bit 3, and 0x57A5E0: a member with the bombard bit. Such a regiment's
/// kill goes through the bombard state (0x527FB0).
pub fn bombard_layout(world: &World, o: u32) -> bool {
    let Some(org) = world.orgs.get(o as usize) else { return false };
    let def = world.data.def(org.def);
    let elements = world.books.org(o).and_then(|b| b.elements.as_ref());
    let locked = def.slots.iter().enumerate().any(|(i, (d, _))| {
        let element = elements.and_then(|e| e.get(i).copied()).unwrap_or(*d);
        def.slot_locked.get(i).copied().unwrap_or(false) && element != crate::data::NO_DEF && world.data.def(element).weapons.iter().any(indirect)
    });
    locked && org.members.iter().any(|&m| world.actors.get(m as usize).is_some_and(|x| x.alive && !x.removed && world.data.def(x.def).weapons.iter().any(indirect)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The control table against the TGI values listed in the module docs.
    #[test]
    fn hot_commands_are_the_data_values() {
        let hot: Vec<u32> = (0..DEFINITION_COUNT).filter(|i| hot_actor(*i)).collect();
        assert_eq!(hot, [3, 4, 5]);
        let hot: Vec<u32> = (0..DEFINITION_COUNT).filter(|i| hot_location(*i)).collect();
        assert_eq!(hot, [5, 16]);
        for (i, id) in [(10, "disband"), (11, "unpack"), (12, "pack_organization"), (16, "set_rally_point"), (34, "pack"), (4, "construct"), (15, "recruit")] {
            assert_eq!(COMMANDS[i as usize].id, id);
            assert!(non_group(i), "{id}");
        }
        for i in [0, 5, 6, 7, 33] {
            assert!(!non_group(i), "{}", COMMANDS[i as usize].id);
        }
    }
}
