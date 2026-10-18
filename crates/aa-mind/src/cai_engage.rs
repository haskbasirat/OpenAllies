//! Original CAI engage-state target list (`KKC_CAIStateEngage`, regiment
//! state `cai_engage` at Mind+1CC) and the Mind-level engage request.
//!
//! The list (state +10 head, +14 tail) holds `{gtid, prev, next, priority}`
//! nodes in ascending priority order (`51FE60`). Combat's damage-driven
//! target acquisition (aa_combat::engagement, `579490`) calls `524C10`.
//!
//! - `521060` can-add predicate, `523DD0` AddTarget, `524C10` Mind request,
//!   `51EFB0` RemoveTarget, `51F040` ClearTargetList (+`5C2CD0`),
//!   `523EA0` refresh (element targets replaced by their organizations,
//!   stale/invalid targets removed), `51FE60` sorted insertion.
//!
//! Diagnostic logging (`404F90`) is not modelled.
use crate::{Arena, Result, StateId, is_current};
use serde::{Deserialize, Serialize};

/// One engage target node (`+0` GTID, `+C` priority).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetNode {
    pub gtid: u32,
    pub priority: i32,
}

/// Head-to-tail engage target list of one engage state.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Targets {
    pub nodes: Vec<TargetNode>,
}

/// `51FE60`: insert a node with `priority` and return its index. With
/// `from_tail` false (all callers here) the node goes before the first node
/// whose priority is >= it; with `from_tail` it goes after the last node whose
/// priority is <= it.
pub fn insert_sorted(targets: &mut Targets, gtid: u32, priority: i32, from_tail: bool) -> usize {
    let node = TargetNode { gtid, priority };
    let index = if targets.nodes.is_empty() {
        0
    } else if from_tail {
        targets
            .nodes
            .iter()
            .rposition(|n| n.priority <= priority)
            .map_or(0, |i| i + 1)
    } else {
        targets
            .nodes
            .iter()
            .position(|n| priority <= n.priority)
            .unwrap_or(targets.nodes.len())
    };
    targets.nodes.insert(index, node);
    index
}

/// Live services, called in native order. `Actor` handles are opaque.
pub trait EngageWorld {
    /// GTID registry lookup (generation checked).
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>>;
    /// Actor +14 GTID.
    fn gtid(&mut self, actor: u32) -> Result<u32>;
    /// Actor +8 bit 0 (dead/removed).
    fn removed(&mut self, actor: u32) -> Result<bool>;
    /// Actor +80 membership: its organization (+10) when the membership
    /// exists, the organization is non-null and byte +1C is set.
    fn attached_organization(&mut self, actor: u32) -> Result<Option<u32>>;
    /// Mind virtual +A0(target).
    fn can_target(&mut self, target: u32) -> Result<bool>;
    /// `4D4FF0(target, 1, 0)` on the organization actor (0 = allowed).
    fn relation(&mut self, target: u32) -> Result<i32>;
    /// `4D3990(organization)` on the target (attacker registration).
    fn register_attacker(&mut self, target: u32) -> Result<()>;
    /// `4D0CD0(organization)` on the target.
    fn unregister_attacker(&mut self, target: u32) -> Result<()>;
    /// Organization actor +7C component present.
    fn has_component(&mut self) -> Result<bool>;
    /// `57A3C0(target)` on the organization component.
    fn component_target_added(&mut self, target: u32) -> Result<()>;
    /// `579BA0()` on the organization component (no native null check).
    fn component_update(&mut self) -> Result<()>;
    /// Mind virtual +1AC(target): target priority.
    fn priority(&mut self, target: u32) -> Result<i32>;
    /// Mind virtual +F0(target): start engaging.
    fn engage(&mut self, target: u32) -> Result<bool>;
    /// Chasing state target word (Mind+130, chasing state +10).
    fn chasing_target(&mut self) -> Result<u32>;
    /// Organization actor +148 array entry 4 when its count (+14C) > 4.
    fn organization_entry4(&mut self) -> Result<Option<u32>>;
    /// Target virtual +130(entry4, 1).
    fn target_check130(&mut self, target: u32, entry: u32) -> Result<bool>;
    /// Target +90 word is non-zero.
    fn target_word90(&mut self, target: u32) -> Result<bool>;
    /// `4D4420(b)` on actor `a`.
    fn related(&mut self, a: u32, b: u32) -> Result<bool>;
    /// Organization actor (Mind +4).
    fn organization(&mut self) -> Result<u32>;
    /// `57A500(target, 1, 1)` on the organization component.
    fn component_accepts(&mut self, target: u32) -> Result<bool>;
}

/// `521060`: target is live, not already listed, targetable by the Mind and
/// related as an enemy.
pub fn can_add(
    targets: &Targets,
    target: Option<u32>,
    world: &mut impl EngageWorld,
) -> Result<bool> {
    let Some(target) = target else {
        return Ok(false);
    };
    if world.removed(target)? {
        return Ok(false);
    }
    let gtid = world.gtid(target)?;
    if targets.nodes.iter().any(|n| n.gtid == gtid) {
        return Ok(false);
    }
    if !world.can_target(target)? {
        return Ok(false);
    }
    Ok(world.relation(target)? == 0)
}

fn add_unchecked(
    targets: &mut Targets,
    target: u32,
    world: &mut impl EngageWorld,
) -> Result<usize> {
    world.register_attacker(target)?;
    if world.has_component()? {
        world.component_target_added(target)?;
    }
    let priority = world.priority(target)?;
    let gtid = world.gtid(target)?;
    let index = insert_sorted(targets, gtid, priority, false);
    world.component_update()?;
    Ok(index)
}

/// `523DD0(target, check)`; returns the native bool.
pub fn add_target(
    targets: &mut Targets,
    target: u32,
    check: bool,
    world: &mut impl EngageWorld,
) -> Result<bool> {
    if check && !can_add(targets, Some(target), world)? {
        return Ok(false);
    }
    add_unchecked(targets, target, world)?;
    Ok(true)
}

/// `524C10(target)` on the regiment Mind. `engage_state` (Mind+1CC) and
/// `chasing_state` (Mind+120) are the embedded states; returns the native
/// bool (false whenever the target was added to an active engage list).
pub fn mind_add_target(
    targets: &mut Targets,
    engage_state: StateId,
    chasing_state: StateId,
    target: Option<u32>,
    arena: &Arena,
    world: &mut impl EngageWorld,
) -> Result<bool> {
    let Some(mut target) = target else {
        return Ok(false);
    };
    if let Some(organization) = world.attached_organization(target)? {
        target = organization;
    }
    if is_current(engage_state, arena)? {
        if can_add(targets, Some(target), world)? {
            add_unchecked(targets, target, world)?;
        }
        return Ok(false);
    }
    if is_current(chasing_state, arena)? {
        let chased = world.chasing_target()?;
        if let Some(chased) = world.resolve(chased)? {
            let current = world.priority(chased)?;
            if current >= world.priority(target)? {
                return Ok(false);
            }
        }
    }
    world.engage(target)
}

/// `51EFB0`: remove the node at `index` (the iterator then designates the
/// next node, i.e. the same index).
pub fn remove_target(
    targets: &mut Targets,
    index: usize,
    world: &mut impl EngageWorld,
) -> Result<()> {
    let gtid = targets.nodes[index].gtid;
    if let Some(target) = world.resolve(gtid)? {
        world.unregister_attacker(target)?;
    }
    targets.nodes.remove(index);
    Ok(())
}

/// `51F040` (+`5C2CD0`): remove every node from the head.
pub fn clear_targets(targets: &mut Targets, world: &mut impl EngageWorld) -> Result<()> {
    while !targets.nodes.is_empty() {
        remove_target(targets, 0, world)?;
    }
    Ok(())
}

/// `523EA0` on the engage state.
pub fn refresh_targets(
    targets: &mut Targets,
    engage_state: StateId,
    arena: &Arena,
    world: &mut impl EngageWorld,
) -> Result<()> {
    if is_current(engage_state, arena)? {
        let mut index = 0;
        while index < targets.nodes.len() {
            let gtid = targets.nodes[index].gtid;
            let organization = match world.resolve(gtid)? {
                Some(actor) => world.attached_organization(actor)?,
                None => None,
            };
            match organization {
                Some(organization) => {
                    if can_add(targets, Some(organization), world)?
                        && add_unchecked(targets, organization, world)? <= index
                    {
                        // Inserted before the iterator node, which moves down.
                        index += 1;
                    }
                    remove_target(targets, index, world)?;
                }
                None => index += 1,
            }
        }
    }
    let mut index = 0;
    while index < targets.nodes.len() {
        let gtid = targets.nodes[index].gtid;
        let target = world.resolve(gtid)?;
        let entry = world.organization_entry4()?;
        let keep_check = match (target, entry) {
            (Some(t), Some(e)) if !world.target_check130(t, e)? => {
                if world.target_word90(t)? {
                    true
                } else {
                    let organization = world.organization()?;
                    world.related(t, organization)? || world.related(organization, t)?
                }
            }
            (None, _) => false,
            (Some(_), _) => true,
        };
        let keep = match (keep_check, target) {
            (true, Some(t)) => world.can_target(t)? && world.component_accepts(t)?,
            _ => false,
        };
        if keep {
            index += 1;
        } else {
            remove_target(targets, index, world)?;
        }
    }
    Ok(())
}
