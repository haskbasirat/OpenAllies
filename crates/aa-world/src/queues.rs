//! Native periodic and deferred world queues (50B330/50B020).
//!
//! One owner commits queue mutations. Host callbacks are required; these queues
//! do not infer actor behavior, world time, wall time, or parallel execution.
//! The native skip on every thousandth deferred event and periodic removal
//! cursor behavior are intentional. Errors can follow committed effects.

use aa_numeric::{Binary, Precision};
use aa_simulation::NativeRng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Owner = u64;
pub mod restoration;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("queue bucket count must be within the positive signed native domain")]
    BucketCount,
    #[error("unknown queue or queue kind")]
    QueueKind,
    #[error("queue allocation failed")]
    Allocation,
    #[error("event identity exhausted")]
    Identity,
    #[error("nested queue dispatch is outside the proven ownership contract")]
    NestedDispatch,
    #[error("invalid event cursor")]
    InvalidCursor,
    #[error("world step must be finite and positive")]
    Step,
    #[error("required queue callback failed: {0}")]
    Callback(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventId(u64);
impl EventId {
    /// Portable node identity; never a native pointer or saved Updatable ID.
    pub fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clock {
    pub begin_bits: u32,
    pub end_bits: u32,
    pub step_bits: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Periodic,
    Deferred,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delivery {
    pub queue: u8,
    pub owner: Owner,
    pub data: u32,
    pub elapsed_bits: Option<u32>,
}

pub trait Callbacks {
    fn deliver(&mut self, queues: &mut Queues, event: Delivery) -> Result<()>;
}

/// One queue update in progress ([`Queues::begin_update`]).
#[derive(Debug)]
pub struct Dispatch {
    id: u8,
    kind: Kind,
    elapsed_bits: Option<u32>,
    /// Deferred deliveries of this update (the thousandth is skipped).
    count: u32,
    /// A delivery was handed out: the next step first applies the native
    /// post-callback advance.
    delivered: bool,
}

#[derive(Clone, Debug)]
struct Node {
    queue: u8,
    bucket: usize,
    owner: Owner,
    data: u32,
    next: Option<EventId>,
}

#[derive(Clone, Debug)]
struct Queue {
    kind: Kind,
    current: usize,
    buckets: Vec<Option<EventId>>,
    last_end_bits: u32,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueSnapshot {
    pub id: u8,
    pub kind: Kind,
    pub current: usize,
    pub last_end_bits: u32,
    pub buckets: Vec<Vec<(Owner, u32)>>,
}

/// Shared cancellation/current-node state matches the world's single +228
/// cursor. Numeric precision and both clocks are supplied explicitly.
pub struct Queues {
    queues: BTreeMap<u8, Queue>,
    nodes: BTreeMap<u64, Node>,
    owners: BTreeMap<Owner, Vec<EventId>>,
    next_id: u64,
    cursor: Option<EventId>,
    dispatching: bool,
    pub clock: Clock,
    pub precision: Precision,
}

fn number(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}

/// 4F0720: divide at incoming precision, spill to f64, ceil, signed conversion.
/// Finite f32 operands keep this quotient within normal f64 exponent bounds.
pub fn period_ticks(period_bits: u32, step_bits: u32, pc: Precision) -> Result<i32> {
    let step = number(step_bits)?;
    if step.is_zero() || step.is_negative_nonzero() {
        return Err(Error::Step);
    }
    let quotient = number(period_bits)?
        .div(&step, pc)?
        .mul(&Binary::from_i32(1), Precision::Double53)?;
    let integer = quotient.truncate_i32()?;
    if quotient.compare(&Binary::from_i32(integer)).is_gt() {
        integer.checked_add(1).ok_or(Error::BucketCount)
    } else {
        Ok(integer)
    }
}

impl Queues {
    pub fn new(clock: Clock, precision: Precision) -> Result<Self> {
        period_ticks(0, clock.step_bits, precision)?;
        number(clock.begin_bits)?;
        number(clock.end_bits)?;
        Ok(Self {
            queues: BTreeMap::new(),
            nodes: BTreeMap::new(),
            owners: BTreeMap::new(),
            next_id: 1,
            cursor: None,
            dispatching: false,
            clock,
            precision,
        })
    }

    /// Caller-proven initialization count. The stock queue periods are supplied
    /// by world initialization, not hard-coded into this container.
    pub fn initialize(&mut self, id: u8, kind: Kind, count: i32) -> Result<()> {
        if count <= 0 || self.queues.contains_key(&id) {
            return Err(Error::BucketCount);
        }
        let mut buckets = Vec::new();
        buckets
            .try_reserve_exact(count as usize)
            .map_err(|_| Error::Allocation)?;
        buckets.resize(count as usize, None);
        self.queues.insert(
            id,
            Queue {
                kind,
                current: 0,
                buckets,
                last_end_bits: 0,
            },
        );
        Ok(())
    }

    /// Native periodic insertion50B2C0 consumes exactly one world RNG result,
    /// chooses its remainder bucket, and inserts at that bucket and owner's head.
    /// Actor availability predicates are caller-owned and precede this call.
    pub fn add_periodic(
        &mut self,
        id: u8,
        owner: Owner,
        data: u32,
        rng: &mut NativeRng,
    ) -> Result<EventId> {
        let q = self.queues.get(&id).ok_or(Error::QueueKind)?;
        if q.kind != Kind::Periodic {
            return Err(Error::QueueKind);
        }
        let bucket = rng.next_u32() as usize % q.buckets.len();
        self.insert(id, bucket, owner, data)
    }

    /// Native relative scheduling50AF10: a positive delay below (dt-epsilon)
    /// is raised to that value before adding the stored world begin time.
    pub fn add_after(
        &mut self,
        id: u8,
        owner: Owner,
        data: u32,
        delay_bits: u32,
    ) -> Result<EventId> {
        let mut delay = number(delay_bits)?;
        if delay.compare(&Binary::from_i32(0)).is_gt() {
            let minimum =
                number(self.clock.step_bits)?.sub(&number(0x38d1_b717)?, self.precision)?;
            if delay.compare(&minimum).is_lt() {
                delay = number(minimum.store_f32()?)?;
            }
        }
        let when = number(self.clock.begin_bits)?
            .add(&delay, self.precision)?
            .store_f32()?;
        self.add_at(id, owner, data, when)
    }

    /// Native absolute scheduling50BA50. It uses max(world.begin, queue.last_end),
    /// subtracts epsilon, stores f32, caps delays above ten seconds, then ceil/dt.
    /// The ten-second behavior is an original scheduling rule, not a storage cap.
    pub fn add_at(&mut self, id: u8, owner: Owner, data: u32, when_bits: u32) -> Result<EventId> {
        let q = self.queues.get(&id).ok_or(Error::QueueKind)?;
        if q.kind != Kind::Deferred {
            return Err(Error::QueueKind);
        }
        let begin = number(self.clock.begin_bits)?;
        let last = number(q.last_end_bits)?;
        let origin = if last.compare(&begin).is_gt() {
            last
        } else {
            begin
        };
        let relative_bits = number(when_bits)?
            .sub(&origin, self.precision)?
            .sub(&number(0x38d1_b717)?, self.precision)?
            .store_f32()?;
        let relative_bits = if number(relative_bits)?
            .compare(&number(10.0_f32.to_bits())?)
            .is_gt()
        {
            10.0_f32.to_bits()
        } else {
            relative_bits
        };
        let offset =
            period_ticks(relative_bits, self.clock.step_bits, self.precision)?.max(0) as usize;
        self.grow_deferred(id, offset)?;
        let q = self.queues.get(&id).ok_or(Error::QueueKind)?;
        let bucket = (q.current + offset) % q.buckets.len();
        self.insert(id, bucket, owner, data)
    }

    fn grow_deferred(&mut self, id: u8, offset: usize) -> Result<()> {
        let q = self.queues.get_mut(&id).ok_or(Error::QueueKind)?;
        let old = q.buckets.len();
        if offset < old {
            return Ok(());
        }
        let count = offset
            .checked_add(1)
            .filter(|n| *n <= i32::MAX as usize)
            .ok_or(Error::BucketCount)?;
        let mut grown = Vec::new();
        grown
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        grown.resize(count, None);
        //50B710 keeps the current index, relocates wrapped heads after the old
        //end, then fills the newly inserted gap. Relative timing is preserved.
        for (index, head) in q.buckets.iter().copied().enumerate() {
            let target = (q.current + (index + old - q.current) % old) % count;
            grown[target] = head;
            let mut cursor = head;
            while let Some(node) = cursor {
                let value = self.nodes.get_mut(&node.0).ok_or(Error::InvalidCursor)?;
                value.bucket = target;
                cursor = value.next;
            }
        }
        q.buckets = grown;
        Ok(())
    }

    fn insert(&mut self, id: u8, bucket: usize, owner: Owner, data: u32) -> Result<EventId> {
        let next_id = self.next_id.checked_add(1).ok_or(Error::Identity)?;
        let owners = self.owners.entry(owner).or_default();
        owners.try_reserve(1).map_err(|_| Error::Allocation)?;
        let q = self.queues.get_mut(&id).ok_or(Error::QueueKind)?;
        let event = EventId(self.next_id);
        self.nodes.insert(
            event.0,
            Node {
                queue: id,
                bucket,
                owner,
                data,
                next: q.buckets[bucket],
            },
        );
        owners.insert(0, event);
        q.buckets[bucket] = Some(event);
        self.next_id = next_id;
        Ok(event)
    }

    /// Native Updatable+C nonnull test across all queues. The live links here
    /// are authoritative; saved constructor head words are not queried later.
    pub fn has_owner(&self, owner: Owner) -> bool {
        self.owners.get(&owner).is_some_and(|ids| !ids.is_empty())
    }
    /// The one authoritative live cancellation-chain head (Updatable+4).
    pub fn owner_head(&self, owner: Owner) -> Option<EventId> {
        self.owners.get(&owner).and_then(|ids| ids.first()).copied()
    }

    pub fn has(&self, owner: Owner, queue: u8, data: u32) -> bool {
        self.owners.get(&owner).is_some_and(|ids| {
            ids.iter().any(|id| {
                self.nodes
                    .get(&id.0)
                    .is_some_and(|node| node.queue == queue && node.data == data)
            })
        })
    }

    /// Native `50B220(queue, data)` (added for the post-load stage,
    /// research/phase8-saved-components): the walk starts at the owner's most
    /// recent event (Updatable+4) and follows that event's BUCKET successor
    /// (entry +8), matching any owner's event; the owner's events in other
    /// buckets are not seen. [`Self::has`] (the owner-list test) is unchanged.
    /// Proven by 8 native runs (native-post-load.json `updatable_has_50b220`).
    pub fn has_from_owner_head(&self, owner: Owner, queue: u8, data: u32) -> bool {
        let mut cursor = self.owners.get(&owner).and_then(|ids| ids.first()).copied();
        while let Some(id) = cursor {
            let Some(node) = self.nodes.get(&id.0) else {
                return false;
            };
            if node.queue == queue && node.data == data {
                return true;
            }
            cursor = node.next;
        }
        false
    }

    /// Native `50B1C0(queue, data)`: the owner's own list (Updatable+4, entry
    /// +10) from its most recent event, every matching event cancelled in that
    /// order (`50ADE0`, with the `4F1A70` cursor move of [`Self::cancel`]);
    /// returns the count. Proven by 7 native runs (native-post-load.json
    /// `updatable_remove_50b1c0`).
    pub fn remove_matching(&mut self, owner: Owner, queue: u8, data: u32) -> Result<u32> {
        let matching: Vec<EventId> = self
            .owners
            .get(&owner)
            .map(|ids| {
                ids.iter()
                    .copied()
                    .filter(|id| {
                        self.nodes
                            .get(&id.0)
                            .is_some_and(|node| node.queue == queue && node.data == data)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let count = u32::try_from(matching.len()).map_err(|_| Error::Identity)?;
        for id in matching {
            self.cancel(id)?;
        }
        Ok(count)
    }

    pub fn cancel_owner(&mut self, owner: Owner) -> Result<()> {
        while let Some(id) = self
            .owners
            .get(&owner)
            .and_then(|list| list.first())
            .copied()
        {
            self.cancel(id)?;
        }
        Ok(())
    }

    pub fn cancel(&mut self, id: EventId) -> Result<()> {
        let Some(node) = self.nodes.remove(&id.0) else {
            return Ok(());
        };
        if self.cursor == Some(id) {
            self.cursor = node.next;
        }
        let q = self.queues.get_mut(&node.queue).ok_or(Error::QueueKind)?;
        if q.buckets[node.bucket] == Some(id) {
            q.buckets[node.bucket] = node.next;
        } else {
            let mut cursor = q.buckets[node.bucket];
            while let Some(previous) = cursor {
                let previous = self
                    .nodes
                    .get_mut(&previous.0)
                    .ok_or(Error::InvalidCursor)?;
                if previous.next == Some(id) {
                    previous.next = node.next;
                    break;
                }
                cursor = previous.next;
            }
        }
        if let Some(owned) = self.owners.get_mut(&node.owner) {
            owned.retain(|value| *value != id);
            if owned.is_empty() {
                self.owners.remove(&node.owner);
            }
        }
        Ok(())
    }

    /// Callback failure leaves committed removals/deliveries intact, does not
    /// advance the bucket, and clears the dispatch guard for an explicit retry.
    pub fn update<C: Callbacks + ?Sized>(&mut self, id: u8, callbacks: &mut C) -> Result<()> {
        if self.dispatching {
            return Err(Error::NestedDispatch);
        }
        self.dispatching = true;
        let result = self.update_inner(id, callbacks);
        self.dispatching = false;
        result
    }

    fn update_inner<C: Callbacks + ?Sized>(&mut self, id: u8, callbacks: &mut C) -> Result<()> {
        let mut dispatch = self.start_dispatch(id)?;
        while let Some(event) = self.step_dispatch(&mut dispatch)? {
            callbacks.deliver(self, event)?;
        }
        self.finish_dispatch(dispatch)
    }

    fn start_dispatch(&mut self, id: u8) -> Result<Dispatch> {
        let q = self.queues.get(&id).ok_or(Error::QueueKind)?;
        let kind = q.kind;
        self.cursor = q.buckets[q.current];
        let elapsed_bits = if kind == Kind::Periodic {
            Some(
                Binary::from_i32(q.buckets.len() as i32)
                    .mul(&number(self.clock.step_bits)?, self.precision)?
                    .store_f32()?,
            )
        } else {
            None
        };
        Ok(Dispatch {
            id,
            kind,
            elapsed_bits,
            count: 0,
            delivered: false,
        })
    }

    /// The next delivery of `dispatch`, applying the native advance that
    /// follows the previous delivery's callback first.
    fn step_dispatch(&mut self, d: &mut Dispatch) -> Result<Option<Delivery>> {
        if d.kind == Kind::Deferred {
            loop {
                if d.delivered {
                    let q = self.queues.get(&d.id).ok_or(Error::QueueKind)?;
                    self.cursor = q.buckets[q.current]; //Includes new heads inserted by callbacks.
                }
                d.delivered = true;
                let Some(event_id) = self.cursor else {
                    return Ok(None);
                };
                let node = self
                    .nodes
                    .get(&event_id.0)
                    .cloned()
                    .ok_or(Error::InvalidCursor)?;
                self.cancel(event_id)?; //Both intrusive lists are unlinked before delivery.
                d.count += 1;
                if d.count == 1000 {
                    d.count = 0; //The native skips every thousandth deferred callback.
                    continue;
                }
                return Ok(Some(Delivery {
                    queue: d.id,
                    owner: node.owner,
                    data: node.data,
                    elapsed_bits: None,
                }));
            }
        }
        if d.delivered {
            //Cancellation itself advances the native cursor. The dispatcher
            //then advances again: the successor of a removed current entry
            //is skipped for this visit, as in50B371..380.
            if let Some(current) = self.cursor {
                self.cursor = self.nodes.get(&current.0).ok_or(Error::InvalidCursor)?.next;
            }
        }
        d.delivered = true;
        let Some(event_id) = self.cursor else {
            return Ok(None);
        };
        let node = self.nodes.get(&event_id.0).ok_or(Error::InvalidCursor)?;
        Ok(Some(Delivery {
            queue: d.id,
            owner: node.owner,
            data: node.data,
            elapsed_bits: d.elapsed_bits,
        }))
    }

    fn finish_dispatch(&mut self, d: Dispatch) -> Result<()> {
        let q = self.queues.get_mut(&d.id).ok_or(Error::QueueKind)?;
        q.current = (q.current + 1) % q.buckets.len();
        if d.kind == Kind::Deferred {
            q.last_end_bits = self.clock.end_bits;
        }
        Ok(())
    }

    /// Step-by-step form of [`Self::update`] for a host that keeps the queues
    /// inside its own world and needs `&mut` access to that world between
    /// deliveries (the same native order, cursor moves and thousandth-callback
    /// skip): call [`Self::next_delivery`] until it returns `None`, run each
    /// delivery's leaf in between (it may add or cancel events on these
    /// queues), then [`Self::end_update`]. After a failed leaf, call
    /// [`Self::abort_update`]: the committed effects stay and the bucket does
    /// not advance, as with [`Self::update`].
    pub fn begin_update(&mut self, id: u8) -> Result<Dispatch> {
        if self.dispatching {
            return Err(Error::NestedDispatch);
        }
        let d = self.start_dispatch(id)?;
        self.dispatching = true;
        Ok(d)
    }
    pub fn next_delivery(&mut self, dispatch: &mut Dispatch) -> Result<Option<Delivery>> {
        let result = self.step_dispatch(dispatch);
        if result.is_err() {
            self.dispatching = false;
        }
        result
    }
    pub fn end_update(&mut self, dispatch: Dispatch) -> Result<()> {
        self.dispatching = false;
        self.finish_dispatch(dispatch)
    }
    pub fn abort_update(&mut self, dispatch: Dispatch) {
        let _ = dispatch;
        self.dispatching = false;
    }

    pub fn snapshot(&self) -> Vec<QueueSnapshot> {
        self.queues
            .iter()
            .map(|(&id, q)| QueueSnapshot {
                id,
                kind: q.kind,
                current: q.current,
                last_end_bits: q.last_end_bits,
                buckets: q
                    .buckets
                    .iter()
                    .map(|head| {
                        let mut cursor = *head;
                        let mut values = Vec::new();
                        while let Some(id) = cursor {
                            let n = &self.nodes[&id.0];
                            values.push((n.owner, n.data));
                            cursor = n.next;
                        }
                        values
                    })
                    .collect(),
            })
            .collect()
    }
}
