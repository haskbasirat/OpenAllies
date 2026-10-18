//! Native capture rules called by Body damage (`5625C0`) at nonpositive
//! health: eligibility `4D1120` and the attempt `4D5230` (ownership transfer,
//! announcements, strategic-AI notifications, statistics and the `4CEBE0`
//! reward). [`attempt`] implements `aa_body::damage::World::attempt_capture`.
use crate::{Error, Result};
use aa_body::{F32, damage::DamageRecord};
use serde::{Deserialize, Serialize};

/// Direct actor fields read by `4D1120`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureView {
    /// `+98` wall-segment component; the inner value is its `+14` owner actor
    /// (the `4CEDA0` result on this path).
    pub segment_owner: Option<Option<u32>>,
    /// `+100`; bit21 blocks capture.
    pub flags100: u32,
    /// `+78` properties present: its modifier flags word (`+1C`). Bit10
    /// (`0x400`) is NOT_CAPTURABLE.
    pub modifier_flags: Option<u32>,
    /// `+94` component present: its `+34` GTID.
    pub component94_target: Option<u32>,
    /// `+D4` nonnull.
    pub component_d4: bool,
    /// Actor definition `+290` byte (authored `captureable`).
    pub captureable: u8,
}

pub trait World {
    fn capture_view(&self, actor: u32) -> Result<CaptureView>;
    /// Generation-checked registry lookup (`4579D0` on the `+94` component).
    fn resolve(&self, gtid: u32) -> Result<Option<u32>>;
}

/// Native `4D1120`. Wall segments delegate to their owner actor, repeatedly.
/// The native routine returns the raw definition byte; every caller tests it
/// for nonzero. A segment-owner cycle (a native infinite loop) is an error.
pub fn eligible(actor: u32, world: &impl World) -> Result<bool> {
    let mut actor = actor;
    let mut visited = Vec::new();
    loop {
        let view = world.capture_view(actor)?;
        let Some(owner) = view.segment_owner else {
            if view.flags100 & 0x20_0000 != 0 {
                return Ok(false);
            }
            if view.modifier_flags.is_some_and(|flags| flags & 0x400 != 0) {
                return Ok(false);
            }
            if let Some(target) = view.component94_target
                && world.resolve(target)?.is_some()
            {
                return Ok(false);
            }
            if view.component_d4 {
                return Ok(false);
            }
            return Ok(view.captureable != 0);
        };
        let Some(owner) = owner else {
            return Ok(false);
        };
        if visited.contains(&actor) {
            return Err(Error::Invalid(
                "capture segment-owner cycle (native infinite loop)",
            ));
        }
        visited
            .try_reserve(1)
            .map_err(|e| Error::World(format!("capture visit allocation failed: {e}")))?;
        visited.push(actor);
        actor = owner;
    }
}

/// Services used by `4D5230`. Virtual calls are listed with their native
/// vtable offsets; every one may run arbitrary live-world code.
pub trait AttemptWorld {
    /// Virtual `+158`: this actor delegates capture to its outer actor.
    fn delegates_capture(&mut self, actor: u32) -> Result<bool>;
    /// Virtual `+100`.
    fn outer(&mut self, actor: u32) -> Result<Option<u32>>;
    /// Virtual `+108`.
    fn allegiance(&mut self, actor: u32) -> Result<Option<u32>>;
    /// Virtual `+A8`: the actor is itself an allegiance (kingdom).
    fn is_allegiance(&mut self, actor: u32) -> Result<bool>;
    /// `54BEA0(B36520 manager, captured, source)`: the trigger manager's
    /// `+50` queue prepend of a `54B820` Captured payload. Bind
    /// `aa_triggers::predicates::queue_captured` (the proven 54BEA0/54B820
    /// port owned by aa-triggers).
    fn capture_notification(&mut self, captured: u32, source: u32) -> Result<()>;
    /// Source label (`4E9560` display name of `source_kingdom`, or the source
    /// name virtual `+24` when it is `None`), captured name `+24`, then
    /// `42D490(event 0x10, old_owner, captured, captured name, source label)`.
    fn announce_capture(
        &mut self,
        captured: u32,
        source: u32,
        old_owner: Option<u32>,
        source_kingdom: Option<u32>,
    ) -> Result<()>;
    /// Component presence: `+98` wall, `+88` component88, `+D4`.
    fn has_wall(&self, actor: u32) -> Result<bool>;
    fn has_component88(&self, actor: u32) -> Result<bool>;
    fn has_component_d4(&self, actor: u32) -> Result<bool>;
    /// `585A20(new_owner)` on the wall component; bind
    /// [`crate::walls::transfer_wall`].
    fn transfer_wall(&mut self, actor: u32, new_owner: Option<u32>) -> Result<()>;
    /// `56ACB0(new_owner)` on component88; bind
    /// [`crate::walls::transfer_component_88`].
    fn transfer_component88(&mut self, actor: u32, new_owner: Option<u32>) -> Result<()>;
    /// Virtual `+160(new_owner)`: the ownership change itself.
    fn set_allegiance(&mut self, actor: u32, new_owner: Option<u32>) -> Result<()>;
    /// Captured name `+24`, then `42D490(event 0xF, new_owner, captured, name)`.
    fn announce_owner_change(&mut self, captured: u32, new_owner: Option<u32>) -> Result<()>;
    /// SAI global `B36528` nonnull.
    fn strategic_ai_present(&self) -> Result<bool>;
    /// `79EA90(owner)`: that owner's strategic AI, when any.
    fn strategic_ai(&mut self, owner: Option<u32>) -> Result<Option<u32>>;
    /// `7B2050(captured)` / `7B21C0(captured)` on an SAI.
    fn strategic_ai_lost(&mut self, ai: u32, captured: u32) -> Result<()>;
    fn strategic_ai_gained(&mut self, ai: u32, captured: u32) -> Result<()>;
    /// Owner `+32C` statistics component.
    fn statistics(&self, owner: u32) -> Result<Option<u32>>;
    /// `58E870(index, amount)`.
    fn add_statistic(&mut self, statistics: u32, index: u32, amount: F32) -> Result<()>;
    /// `58F010(captured)` on the new owner's statistics: bind
    /// [`crate::statistics_tracking::track`] on that kingdom's aa-economy
    /// `Statistics` and the captured actor's class word.
    fn capture_statistics(&mut self, statistics: u32, captured: u32) -> Result<()>;
    /// `58EE00(captured)` classification (aa-economy
    /// `statistics::structure_class`).
    fn classification(&mut self, captured: u32) -> Result<i32>;
    /// `4CEBE0(captured, record)` on the source (see `morale::capture_reward`).
    fn capture_reward(&mut self, source: u32, captured: u32, record: &DamageRecord) -> Result<()>;
}

fn statistic_pair(
    old: Option<u32>,
    new: Option<u32>,
    lost: u32,
    won: u32,
    captured: u32,
    world: &mut impl AttemptWorld,
) -> Result<()> {
    if let Some(stats) = old {
        world.add_statistic(stats, lost, F32::ONE)?;
    }
    if let Some(stats) = new {
        world.add_statistic(stats, won, F32::ONE)?;
        world.capture_statistics(stats, captured)?;
    }
    Ok(())
}

/// Native `4D5230(source, record)` on `captured`. Returns the native bool.
/// Delegation follows outer actors; a cycle (native unbounded recursion) or a
/// null outer actor (native null call) is an error.
pub fn attempt(
    captured: u32,
    source: Option<u32>,
    record: &DamageRecord,
    world: &mut impl AttemptWorld,
) -> Result<bool> {
    let mut captured = captured;
    let mut visited = Vec::new();
    while world.delegates_capture(captured)? {
        if visited.contains(&captured) {
            return Err(Error::Invalid("capture delegation cycle"));
        }
        visited
            .try_reserve(1)
            .map_err(|e| Error::World(format!("capture visit allocation failed: {e}")))?;
        visited.push(captured);
        captured = world.outer(captured)?.ok_or(Error::Invalid(
            "capture delegation reached a null outer actor",
        ))?;
    }
    if record.event_kind != 1 {
        return Ok(false);
    }
    let old_owner = world.allegiance(captured)?;
    let Some(source) = source else {
        return Ok(false);
    };
    let new_owner = if world.is_allegiance(source)? {
        Some(source)
    } else {
        world.allegiance(source)?
    };
    if old_owner == new_owner {
        return Ok(false);
    }
    world.capture_notification(captured, source)?;
    let source_kingdom = if world.allegiance(source)?.is_some() {
        Some(world.allegiance(source)?.ok_or(Error::Invalid(
            "source allegiance vanished between native calls",
        ))?)
    } else {
        None
    };
    world.announce_capture(captured, source, old_owner, source_kingdom)?;
    if world.has_wall(captured)? {
        world.transfer_wall(captured, new_owner)?;
    }
    if world.has_component88(captured)? {
        world.transfer_component88(captured, new_owner)?;
    }
    world.set_allegiance(captured, new_owner)?;
    world.announce_owner_change(captured, new_owner)?;
    if world.strategic_ai_present()? {
        if let Some(ai) = world.strategic_ai(old_owner)? {
            world.strategic_ai_lost(ai, captured)?;
        }
        if let Some(ai) = world.strategic_ai(new_owner)? {
            world.strategic_ai_gained(ai, captured)?;
        }
    }
    let old = match old_owner {
        Some(owner) => world.statistics(owner)?,
        None => None,
    };
    let new = match new_owner {
        Some(owner) => world.statistics(owner)?,
        None => None,
    };
    if world.has_wall(captured)? {
        statistic_pair(old, new, 0x1a, 0x19, captured, world)?;
    }
    if world.has_component_d4(captured)? {
        statistic_pair(old, new, 0x2b, 0x2a, captured, world)?;
    }
    if world.classification(captured)? == 1 {
        statistic_pair(old, new, 0x1d, 0x1c, captured, world)?;
    }
    world.capture_reward(source, captured, record)?;
    Ok(true)
}
