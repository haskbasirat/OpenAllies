//! Original structure production queue; authoritative world callbacks are required.
//! Native order and binary32 store boundaries are preserved. Errors can follow
//! state changes or external effects: no transactional rollback is implied.
#![forbid(unsafe_code)]

pub mod construction;
pub mod purchases;
pub mod research;
pub mod structure_state;

use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error(transparent)]
    Economy(#[from] aa_economy::EconomyError),
    #[error("invalid native production input: {0}")]
    Invalid(&'static str),
    #[error("production queue allocation failed")]
    Allocation,
    #[error("required production world callback failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: u32,
    /// Original discriminator: 0 recruit, 1 research, 2 location recruit, 3 empty.
    pub kind: u32,
    pub actor_definition: u32,
    pub organization_layout: u32,
    pub location_bits: [u32; 2],
    pub property: u32,
    pub paid_cost: Vec<u32>,
    pub requested_upkeep: Vec<u32>,
    pub current_upkeep: Vec<u32>,
    pub organization_slot: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Queue {
    pub items: Vec<QueueItem>,
    pub deadline_bits: u32,
    pub duration_bits: u32,
    pub paused_at_bits: u32,
    /// Native -1 running, 0 limited resource, 1 supply, 2 disabled.
    pub pause_reason: i32,
    pub scheduled: bool,
}
impl Default for Queue {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            deadline_bits: 0,
            duration_bits: 0,
            paused_at_bits: 0xbf80_0000,
            pause_reason: -1,
            scheduled: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Context {
    pub now_bits: u32,
    pub delta_bits: u32,
    pub instant_production: bool,
    pub precision: Precision,
}

/// Required native leaves. Implementations must preserve native registry order
/// and own real actor creation/research/supply; this trait supplies no substitutes.
///
/// While start/poll holds a queued item, callbacks must preserve queue membership,
/// order and stored item fields. Scheduler/timer fields and external world state
/// may change. After removal, completion/refund/release callbacks may enqueue
/// new items. Arbitrary aliases to native item storage and recursive processing
/// of this same queue are outside this owned-value API's supported domain.
pub trait World {
    fn actor_state(&mut self, queue: &mut Queue, state: u32, enabled: bool) -> Result<()>;
    fn cancel_schedule(&mut self, queue: &mut Queue) -> Result<()>;
    fn schedule_zero(&mut self, queue: &mut Queue) -> Result<()>;
    fn organization_definition(&mut self, actor_definition: u32) -> Result<bool>;
    fn organization_recruit_time(&mut self, actor_definition: u32) -> Result<u32>;
    fn recruit_rate(&mut self, actor_definition: u32) -> Result<Binary>;
    fn research_duration(&mut self, property: u32) -> Result<u32>;
    fn limited_upkeep(&mut self, item: &QueueItem) -> Result<Vec<u32>>;
    fn can_reserve_upkeep(
        &mut self,
        queue: &mut Queue,
        item: &QueueItem,
        notify: bool,
    ) -> Result<bool>;
    fn adjust_upkeep(&mut self, queue: &mut Queue, values: &[u32], add: bool) -> Result<()>;
    fn actor_disabled(&mut self, queue: &mut Queue) -> Result<bool>;
    fn actor_requires_supply(&mut self, queue: &mut Queue) -> Result<bool>;
    fn actor_in_supply(&mut self, queue: &mut Queue) -> Result<bool>;
    fn notify_pause(&mut self, queue: &mut Queue, reason: i32) -> Result<()>;
    fn detach_organization_slot(
        &mut self,
        queue: &mut Queue,
        item: &QueueItem,
    ) -> Result<Option<u32>>;
    fn fulfill(&mut self, queue: &mut Queue, item: &QueueItem) -> Result<bool>;
    fn refund(&mut self, queue: &mut Queue, item: &QueueItem) -> Result<()>;
    fn release_item(&mut self, queue: &mut Queue, item: &QueueItem) -> Result<()>;
}

/// Original recruit-request boundary. Price/rule/registry evaluation and actual
/// kingdom ownership remain required services. Quote is intentionally called
/// twice: 0x582950 affordability, then 0x582AE0 debit, without a second check.
pub trait Recruitment: World {
    /// Opaque stable owner identity, retained at the native capture boundaries.
    fn kingdom(&mut self) -> Result<Option<u32>>;
    fn can_enroll_organization(&mut self, definition: u32) -> Result<bool>;
    fn default_layout(&mut self, definition: u32) -> Result<u32>;
    fn recruitability(&mut self, definition: u32, layout: u32) -> Result<i32>;
    fn quote_cost(&mut self, definition: u32, layout: u32) -> Result<Vec<u32>>;
    fn can_afford(&mut self, kingdom: u32, cost: &[u32]) -> Result<bool>;
    fn notify_unaffordable(&mut self) -> Result<()>;
    fn debit(&mut self, kingdom: u32, cost: &[u32]) -> Result<()>;
    fn reserve_recruit(&mut self, kingdom: u32, definition: u32, layout: u32) -> Result<()>;
    fn quote_upkeep(&mut self, definition: u32, layout: u32) -> Result<Vec<u32>>;
    fn next_item_id(&mut self) -> Result<u32>;
    fn register_organization_item(&mut self, queue: &mut Queue, item_id: u32) -> Result<()>;
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RecruitRequest {
    pub actor_definition: u32,
    pub organization_layout: u32,
    pub location_bits: Option<[u32; 2]>,
}

/// 0x5836E0 -> 0x582AE0 -> 0x589E00. No rollback on a downstream failure:
/// native payment/reservation precedes item allocation and publication.
pub fn request_recruit(
    queue: &mut Queue,
    request: RecruitRequest,
    context: Context,
    world: &mut impl Recruitment,
) -> Result<bool> {
    if world.kingdom()?.is_none() {
        return Ok(false);
    }
    if request.actor_definition == 0 {
        return Err(Error::Invalid(
            "top-level recruit request requires a definition",
        ));
    }
    if world.organization_definition(request.actor_definition)?
        && !world.can_enroll_organization(request.actor_definition)?
    {
        return Ok(false);
    }
    let Some(payment_kingdom) = world.kingdom()? else {
        return Ok(false);
    };
    let mut layout = request.organization_layout;
    if layout == 0 && world.organization_definition(request.actor_definition)? {
        layout = world.default_layout(request.actor_definition)?;
    }
    if layout != 0 && !world.organization_definition(request.actor_definition)? {
        return Ok(false);
    }
    if world.recruitability(request.actor_definition, layout)? != 0 {
        return Ok(false);
    }
    let Some(affordability_kingdom) = world.kingdom()? else {
        return Ok(false);
    };
    let quoted = world.quote_cost(request.actor_definition, layout)?;
    if !world.can_afford(affordability_kingdom, &quoted)? {
        world.notify_unaffordable()?;
        return Ok(false);
    }
    let paid_cost = world.quote_cost(request.actor_definition, layout)?;
    world.debit(payment_kingdom, &paid_cost)?;
    world.reserve_recruit(payment_kingdom, request.actor_definition, layout)?;
    let requested_upkeep = world.quote_upkeep(request.actor_definition, layout)?;
    let id = world.next_item_id()?;
    let item = QueueItem {
        id,
        kind: if request.location_bits.is_some() {
            2
        } else {
            0
        },
        actor_definition: request.actor_definition,
        // Native 582AE0 resolves a local layout for pricing/reservation; 589E00
        // still copies the caller's original layout reference into the item.
        organization_layout: request.organization_layout,
        location_bits: request.location_bits.unwrap_or([0; 2]),
        property: 0,
        current_upkeep: vec![0; requested_upkeep.len()],
        paid_cost,
        requested_upkeep,
        // Original allocator leaves +4C unspecified until organization removal;
        // safe Rust initializes this unused-before-detach field deterministically.
        organization_slot: 0,
    };
    enqueue_prepared(queue, item, context, world)?;
    if world.organization_definition(request.actor_definition)? {
        world.register_organization_item(queue, id)?;
    }
    Ok(true)
}

fn value(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}

/// 0x588D60. Rate is reread after the positive test, as in the original.
pub fn duration(item: &QueueItem, context: Context, world: &mut impl World) -> Result<Binary> {
    let fallback = || -> Result<Binary> {
        Ok(value(context.delta_bits)?.mul(&value(0x3f00_0000)?, context.precision)?)
    };
    if context.instant_production {
        return fallback();
    }
    match item.kind {
        0 | 2 => {
            if item.actor_definition == 0 {
                return Err(Error::Invalid("recruit definition missing"));
            }
            if world.organization_definition(item.actor_definition)? {
                value(world.organization_recruit_time(item.actor_definition)?)
            } else if world
                .recruit_rate(item.actor_definition)?
                .compare(&Binary::from_i32(0))
                == Ordering::Greater
            {
                Ok(value(0x3f80_0000)?.div(
                    &world.recruit_rate(item.actor_definition)?,
                    context.precision,
                )?)
            } else {
                fallback()
            }
        }
        1 => {
            if item.property == 0 {
                return Err(Error::Invalid("research property missing"));
            }
            value(world.research_duration(item.property)?)
        }
        _ => fallback(),
    }
}

/// Queue admission starts after native recruitment/research validation and payment.
/// It must not be called to bypass those prerequisites.
pub fn enqueue_prepared(
    queue: &mut Queue,
    item: QueueItem,
    context: Context,
    world: &mut impl World,
) -> Result<()> {
    queue.items.try_reserve(1).map_err(|_| Error::Allocation)?;
    let first = queue.items.is_empty();
    queue.items.push(item);
    if first {
        start_head(queue, context, world)?;
    }
    Ok(())
}

/// 0x589420, including immediate process-before-schedule ordering.
pub fn start_head(queue: &mut Queue, context: Context, world: &mut impl World) -> Result<()> {
    let Some(item) = queue.items.first().cloned() else {
        queue.deadline_bits = 0;
        queue.duration_bits = 0;
        queue.paused_at_bits = 0xbf80_0000;
        queue.pause_reason = -1;
        world.actor_state(queue, 0x1b, true)?;
        return world.cancel_schedule(queue);
    };
    match item.kind {
        0 | 2 => world.actor_state(queue, 0x1f, true)?,
        1 => world.actor_state(queue, 0x1c, true)?,
        _ => (),
    }
    let duration = duration(&item, context, world)?;
    queue.duration_bits = duration.store_f32()?;
    // FST duration does not pop; deadline uses the unstored extended value.
    queue.deadline_bits = duration
        .add(&value(context.now_bits)?, context.precision)?
        .store_f32()?;
    process(queue, context, world)?;
    if !queue.scheduled {
        world.schedule_zero(queue)?;
    }
    Ok(())
}

/// 0x5894C0 and 0x588CD0. Removed item stays alive through completion/refund.
pub fn remove(
    queue: &mut Queue,
    index: usize,
    world: &mut impl World,
) -> Result<Option<QueueItem>> {
    if index >= queue.items.len() {
        return Ok(None);
    }
    let mut item = queue.items.remove(index);
    if index == 0 {
        queue.paused_at_bits = 0xbf80_0000;
        queue.pause_reason = -1;
        world.cancel_schedule(queue)?;
    }
    world.adjust_upkeep(queue, &item.current_upkeep, false)?;
    item.current_upkeep.fill(0);
    if item.actor_definition != 0
        && world.organization_definition(item.actor_definition)?
        && let Some(slot) = world.detach_organization_slot(queue, &item)?
    {
        item.organization_slot = slot;
    }
    Ok(Some(item))
}

/// 0x589BD0: cancelling the first item removes its timer but does not itself
/// call start_head; that subsequent scheduling belongs to its native caller.
pub fn cancel(queue: &mut Queue, item_id: u32, world: &mut impl World) -> Result<Option<usize>> {
    let Some(index) = queue.items.iter().position(|item| item.id == item_id) else {
        return Ok(None);
    };
    let item =
        remove(queue, index, world)?.ok_or(Error::Invalid("queue changed during removal"))?;
    if matches!(item.kind, 0..=2) {
        world.refund(queue, &item)?;
    }
    world.release_item(queue, &item)?;
    Ok(Some(index))
}

/// 0x589F90. Completion is tested before paused/disabled/supply checks.
pub fn process(queue: &mut Queue, context: Context, world: &mut impl World) -> Result<()> {
    let Some(item) = queue.items.first().cloned() else {
        return start_head(queue, context, world);
    };
    if value(context.now_bits)?.compare(&value(queue.deadline_bits)?) != Ordering::Less {
        let item = remove(queue, 0, world)?.ok_or(Error::Invalid("head disappeared"))?;
        // Native due research with no property logs a diagnostic, skips both
        // completion/refund, then releases the item and advances the queue.
        let has_completion = matches!(item.kind, 0 | 2) || (item.kind == 1 && item.property != 0);
        if has_completion && !world.fulfill(queue, &item)? {
            world.refund(queue, &item)?;
        }
        world.release_item(queue, &item)?;
        return start_head(queue, context, world);
    }
    let mut notify = value(queue.paused_at_bits)?.compare(&Binary::from_i32(0)) == Ordering::Less;
    let mut reason = -1;
    if matches!(item.kind, 0 | 2) {
        let limited = world.limited_upkeep(&item)?;
        let mut equal = item.current_upkeep.len() == limited.len();
        for (&a, &b) in item.current_upkeep.iter().zip(&limited) {
            equal &= value(a)?.compare(&value(b)?) == Ordering::Equal;
        }
        if !equal {
            if world.can_reserve_upkeep(queue, &item, notify)? {
                let head = queue
                    .items
                    .first_mut()
                    .ok_or(Error::Invalid("callback removed queue head"))?;
                // This catches an obvious contract violation; matching IDs are
                // not an identity proof when native IDs wrap or are reused.
                if head.id != item.id {
                    return Err(Error::Invalid("callback replaced queue head"));
                }
                head.current_upkeep = limited.clone();
                world.adjust_upkeep(queue, &limited, true)?;
                notify = true;
            } else {
                reason = 0;
            }
        }
    }
    if reason == -1 {
        if world.actor_disabled(queue)? {
            reason = 2;
        } else if !world.actor_requires_supply(queue)? || world.actor_in_supply(queue)? {
            queue.paused_at_bits = 0xbf80_0000;
            queue.pause_reason = -1;
            return Ok(());
        } else {
            reason = 1;
        }
        if notify && queue.pause_reason != reason {
            world.notify_pause(queue, reason)?;
        }
    }
    if value(queue.paused_at_bits)?.compare(&Binary::from_i32(0)) != Ordering::Less {
        queue.deadline_bits = value(context.now_bits)?
            .sub(&value(queue.paused_at_bits)?, context.precision)?
            .add(&value(queue.deadline_bits)?, context.precision)?
            .store_f32()?;
    }
    queue.paused_at_bits = context.now_bits;
    queue.pause_reason = reason;
    Ok(())
}
