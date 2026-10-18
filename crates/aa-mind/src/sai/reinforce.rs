//! Original SAI goal execution helper `7A5E10` (Source/Sai/SAIGoalEngine.cpp):
//! a military goal's execute slot (`+58`: ATTACK_REGION `7C9710`,
//! ATTACK_ACTOR `7C6FC0`, CONSTRUCT `7C1DF0`, UNPACK `7CB940`, and the
//! player routines `7B7D30`/`7B8540`/`7B8800`) hands it the GTID list of
//! actors near the goal; `7A5E10` decides which of them to pull in and
//! issues their move commands. With its callees:
//!
//! - `7C9CE0` (actor goal: may the actor leave it for a goal valuing it at
//!   `value`?), `4D19A0` list append (`4D0900` node), `4F1430` list removal,
//!   `4D03C0` (object `+154` SAI actor record), `7B9050` region entry;
//! - the region-entry influence sums `7B96E0` (hostile mobile + settlement),
//!   `7B9600` (allied mobile + settlement), `7B97C0` (hostile border) and
//!   `7B9840` (allied border) over the `map_analysis` influence vectors.
//!
//! Per listed actor (live, not deleting, with an SAI record whose `+3C`
//! actor goal is set): the actor goal's priority is finalized (`7A5200`
//! with the owner's economy, no fuzzy draw), this goal's held value of the
//! record (`+2C`) is offered to `7C9CE0`; actors this goal commits (`+40`)
//! or that refuse go to the *stay* list, approved ones to the *pull* list.
//! With a pull list and a target region (`+20`): unless an actor is
//! committed, the stay actors' combat value (`4D0180`) is compared with ego
//! `+304` times the region's hostile influence; when it is lower (or
//! unordered) every listed actor is ordered to the region and the list is
//! consumed, otherwise only the pull actors are (and leave the list). Orders
//! are built by the actor goal (`7C8AF0`) and queued on the goal's player
//! (`7B4FA0`, destroyed by `7ADC60`); they leave the SAI through
//! `KKC_SAICommandData::Send` (`7AF220`, `sai::command`).
use super::base_priority::Obj;
use super::map_analysis::RegionEntry;
use crate::{
    Error, Result,
    x87::{n, num, store},
};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// Services of `7A5E10`/`7C9CE0` in native order. Goals and SAI actor
/// records are native object handles (`Obj`).
pub trait ReinforceWorld {
    /// `[B364C0]` GTID registry.
    fn resolve(&mut self, gtid: u32) -> Result<Option<Obj>>;
    /// Field: object `+8` flags.
    fn flags(&mut self, obj: Obj) -> Result<u32>;
    /// Field: object `+14` GTID.
    fn object_gtid(&mut self, obj: Obj) -> Result<u32>;
    /// Field: object `+154` SAI actor record (`4D03C0`).
    fn sai_record(&mut self, obj: Obj) -> Result<Option<Obj>>;
    /// Field: SAI actor record `+3C` (its actor goal).
    fn record_goal(&mut self, record: Obj) -> Result<Option<Obj>>;
    /// Field: SAI actor record `+10` (the goal holding it).
    fn record_owner_goal(&mut self, record: Obj) -> Result<Option<Obj>>;
    /// Field: SAI actor record `+8` GTID of its object.
    fn record_object(&mut self, record: Obj) -> Result<u32>;
    /// Field: the word at SAI actor record `+30` (`None` when null).
    fn record_route_state(&mut self, record: Obj) -> Result<Option<u32>>;
    /// `7A0FC0(0)` on the record: release its `+30` route and clear it.
    fn clear_route(&mut self, record: Obj) -> Result<()>;
    /// Record virtual `+C` (dead).
    fn record_dead(&mut self, record: Obj) -> Result<bool>;
    /// Field: actor goal `+3C` (its SAI actor record).
    fn goal_actor(&mut self, goal: Obj) -> Result<Option<Obj>>;
    /// Field: goal `+38` (binary32).
    fn goal_priority(&mut self, goal: Obj) -> Result<u32>;
    /// Field: the SAI player of a goal (`[[goal+4]+4]`).
    fn goal_player(&mut self, goal: Obj) -> Result<Obj>;
    /// Field: SAI player `+8` (its game object).
    fn player_object(&mut self, player: Obj) -> Result<Option<Obj>>;
    /// Field: ego `+304` of the SAI player (binary32).
    fn player_ego_304(&mut self, player: Obj) -> Result<u32>;
    /// Goal virtual `+4`.
    fn goal_type(&mut self, goal: Obj) -> Result<u32>;
    /// Goal virtual `+20`.
    fn goal_location(&mut self, goal: Obj) -> Result<Option<Obj>>;
    /// Goal virtual `+2C(record)` (ST0).
    fn goal_held_value(&mut self, goal: Obj, record: Obj) -> Result<Binary>;
    /// Goal virtual `+40(record)`.
    fn goal_virtual_40(&mut self, goal: Obj, record: Obj) -> Result<bool>;
    /// Object virtual `+108` (owner).
    fn owner(&mut self, obj: Obj) -> Result<Option<Obj>>;
    /// `7A5200(owner +1C4, 0)` on a goal (`goal_priority::finalize`).
    fn finalize(&mut self, goal: Obj, owner: Option<Obj>) -> Result<()>;
    /// `7C8AA0` on an actor goal.
    fn goal_ready(&mut self, goal: Obj) -> Result<bool>;
    /// `4D0180` combat value (ST0).
    fn combat_value(&mut self, obj: Obj) -> Result<Binary>;
    /// Field: `[region +1C][player slot]`: the player's entry (vectors).
    fn region_entry(&mut self, region: Obj, player: Obj) -> Result<RegionEntry>;
    /// Field: `[B36534]+148[i]` when `i < +14C`, else null.
    fn kingdom_at(&mut self, index: usize) -> Result<Option<Obj>>;
    /// Object virtual `+138(other)`.
    fn virtual_138(&mut self, obj: Obj, other: Option<Obj>) -> Result<bool>;
    /// Object virtual `+140(other)`.
    fn virtual_140(&mut self, obj: Obj, other: Option<Obj>) -> Result<bool>;
    /// `7C8AF0(&command, actor, location)` on an actor goal, then
    /// `7B4FA0(&command)` on `player` and the command's destructor `7ADC60`.
    fn issue_move(
        &mut self,
        goal: Obj,
        actor: Option<Obj>,
        location: Option<Obj>,
        player: Obj,
    ) -> Result<()>;
}

fn null(what: &str) -> Error {
    Error::World(format!("{what} (native null dereference)"))
}

/// Which influence a region-entry sum reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Influence {
    /// `7B96E0`: mobile (+ settlement unless `mobile_only`) of kingdoms the
    /// player's object is hostile to (virtual `+140`).
    Hostile { mobile_only: bool },
    /// `7B9600`: the same over kingdoms it is allied with (virtual `+138`).
    Allied { mobile_only: bool },
    /// `7B97C0`: border influence of hostile kingdoms.
    HostileBorder,
    /// `7B9840`: border influence of allied kingdoms.
    AlliedBorder,
}

/// `7B96E0` / `7B9600` / `7B97C0` / `7B9840` on a region entry: the
/// binary32 sum of the positive entries (highest owner index first) whose
/// kingdom (`[B36534]+148`) passes the player object's relation virtual.
pub fn influence(
    w: &mut impl ReinforceWorld,
    entry: &RegionEntry,
    player_object: Option<Obj>,
    which: Influence,
    p: Precision,
) -> Result<u32> {
    let (vectors, hostile): (Vec<&Vec<u32>>, bool) = match which {
        Influence::Hostile { mobile_only } => (
            if mobile_only {
                vec![&entry.mobile]
            } else {
                vec![&entry.mobile, &entry.settlement]
            },
            true,
        ),
        Influence::Allied { mobile_only } => (
            if mobile_only {
                vec![&entry.mobile]
            } else {
                vec![&entry.mobile, &entry.settlement]
            },
            false,
        ),
        Influence::HostileBorder => (vec![&entry.border], true),
        Influence::AlliedBorder => (vec![&entry.border], false),
    };
    let mut sum = 0_u32;
    for vector in vectors {
        for i in (0..vector.len()).rev() {
            let v = vector[i];
            if !(f32::from_bits(v) > 0.0) {
                continue;
            }
            let kingdom = w.kingdom_at(i)?;
            let object = player_object.ok_or_else(|| null("relation virtual of a null player object"))?;
            let related = if hostile {
                w.virtual_140(object, kingdom)?
            } else {
                w.virtual_138(object, kingdom)?
            };
            if related {
                sum = store(num(n(sum)?.add(&n(v)?, p))?)?;
            }
        }
    }
    Ok(sum)
}

/// Complete `7C9CE0(value, flag)` on an actor goal `goal`: true when its
/// actor may serve a goal valuing it at `value` (binary32). With `flag`
/// the actor is also ordered towards its holding goal's region.
pub fn release_for(
    w: &mut impl ReinforceWorld,
    goal: Obj,
    value: u32,
    flag: bool,
) -> Result<bool> {
    let priority = w.goal_priority(goal)?;
    if f32::from_bits(priority) <= 0.0 {
        return Ok(false);
    }
    let Some(record) = w.goal_actor(goal)? else {
        return Ok(false);
    };
    if w.record_dead(record)? {
        return Ok(false);
    }
    if !w.goal_ready(goal)? {
        return Ok(false);
    }
    let location = match w.record_owner_goal(record)? {
        Some(owner) => w.goal_location(owner)?,
        None => None,
    };
    if !w.goal_virtual_40(goal, record)? && !(f32::from_bits(value) < f32::from_bits(priority)) {
        if w.record_route_state(record)? == Some(2) {
            w.clear_route(record)?;
        }
        return Ok(false);
    }
    if flag && !w.record_dead(record)? {
        let id = w.record_object(record)?;
        let actor = w.resolve(id)?;
        let player = w.goal_player(goal)?;
        w.issue_move(goal, actor, location, player)?;
    }
    Ok(true)
}

/// Complete `7A5E10(&list)` on `goal`. `list` is the native GTID list
/// (`list +8` head first); it is consumed or pruned as natively.
pub fn reinforce(
    w: &mut impl ReinforceWorld,
    goal: Obj,
    list: &mut Vec<u32>,
    p: Precision,
) -> Result<()> {
    let player = w.goal_player(goal)?;
    if w.goal_type(goal)? != 9 {
        w.goal_type(goal)?;
    }
    let mut stay: Vec<u32> = Vec::new();
    let mut pull: Vec<u32> = Vec::new();
    let mut committed = false;
    for &gtid in list.iter() {
        let Some(obj) = w.resolve(gtid)? else {
            continue;
        };
        if w.flags(obj)? & 1 != 0 {
            continue;
        }
        let Some(record) = w.sai_record(obj)? else {
            continue;
        };
        let Some(actor_goal) = w.record_goal(record)? else {
            continue;
        };
        let owner = w.owner(obj)?;
        w.finalize(actor_goal, owner)?;
        let value = store(w.goal_held_value(goal, record)?)?;
        let released = release_for(w, actor_goal, value, false)?;
        let id = w.object_gtid(obj)?;
        if w.goal_virtual_40(goal, record)? {
            committed = true;
            stay.push(id);
        } else if released {
            pull.push(id);
        } else {
            stay.push(id);
        }
    }
    if pull.is_empty() {
        return Ok(());
    }
    let Some(location) = w.goal_location(goal)? else {
        return Ok(());
    };
    if !committed {
        let mut strength = 0_u32;
        for &gtid in &stay {
            let obj = w
                .resolve(gtid)?
                .ok_or_else(|| null("4D0180 of a stale stay actor"))?;
            let cv = w.combat_value(obj)?;
            strength = store(num(cv.add(&n(strength)?, p))?)?;
        }
        let entry = w.region_entry(location, player)?;
        let object = w.player_object(player)?;
        let threat = influence(w, &entry, object, Influence::Hostile { mobile_only: false }, p)?;
        let scale = w.player_ego_304(player)?;
        let needed = num(n(scale)?.mul(&n(threat)?, p))?;
        // fcompp; test ah,1: below the need (or unordered) orders everyone.
        if n(strength)?.compare(&needed) == Ordering::Less {
            while !list.is_empty() {
                let gtid = list.remove(0);
                order(w, gtid, location, player)?;
            }
            return Ok(());
        }
    }
    for gtid in pull {
        if let Some(i) = list.iter().position(|g| *g == gtid) {
            list.remove(i);
        }
        order(w, gtid, location, player)?;
    }
    Ok(())
}

/// One order of `7A5E10`: a live, non-deleting actor with an actor goal is
/// sent to `location` by that goal.
fn order(w: &mut impl ReinforceWorld, gtid: u32, location: Obj, player: Obj) -> Result<()> {
    let Some(obj) = w.resolve(gtid)? else {
        return Ok(());
    };
    if w.flags(obj)? & 1 != 0 {
        return Ok(());
    }
    let Some(record) = w.sai_record(obj)? else {
        return Ok(());
    };
    let Some(actor_goal) = w.record_goal(record)? else {
        return Ok(());
    };
    w.issue_move(actor_goal, Some(obj), Some(location), player)
}
