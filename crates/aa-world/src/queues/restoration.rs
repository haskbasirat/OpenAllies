//! Native saved list installation, after successful component ID restoration.
//! Saved queue lists append in file order while each owner's cancellation list
//! receives the new event at its head. No random bucket selection is involved.
use super::*;

#[cfg(test)]
#[path = "restoration_review_tests.rs"]
mod review_tests;

#[derive(Clone, Debug)]
pub struct SavedQueue {
    pub id: u8,
    pub kind: Kind,
    pub current: usize,
    pub buckets: Vec<Vec<(Owner, u32)>>,
}
impl Queues {
    /// Checked decoded input. As in50BF10/50BFF0 the existing queue is cleared,
    /// deferred length is restored, and deferred last-end time becomes zero.
    /// Periodic length must match world initialization. Invalid index/counts
    /// are rejected instead of reproducing native out-of-allocation accesses.
    pub fn restore_saved(&mut self, saved: SavedQueue) -> Result<()> {
        if self.dispatching {
            return Err(Error::NestedDispatch);
        }
        let queue = self.queues.get(&saved.id).ok_or(Error::QueueKind)?;
        if queue.kind != saved.kind {
            return Err(Error::QueueKind);
        }
        let count = saved.buckets.len();
        if count == 0
            || count > i32::MAX as usize
            || saved.current >= count
            || (saved.kind == Kind::Periodic && count != queue.buckets.len())
        {
            return Err(Error::BucketCount);
        }
        let old_count = queue.buckets.len();
        let queue = self.queues.get_mut(&saved.id).ok_or(Error::QueueKind)?;
        queue.current = saved.current;
        if saved.kind == Kind::Deferred {
            queue.last_end_bits = 0;
        }
        for bucket in 0..old_count {
            while let Some(event) = self.queues[&saved.id].buckets[bucket] {
                self.cancel(event)?;
            }
        }
        let queue = self.queues.get_mut(&saved.id).ok_or(Error::QueueKind)?;
        if count > queue.buckets.len() {
            queue
                .buckets
                .try_reserve_exact(count - queue.buckets.len())
                .map_err(|_| Error::Allocation)?;
        }
        queue.buckets.resize(count, None);
        for (bucket, events) in saved.buckets.into_iter().enumerate() {
            let mut tail: Option<EventId> = None;
            for (owner, data) in events {
                let old_head = self.queues[&saved.id].buckets[bucket];
                let new = self.insert(saved.id, bucket, owner, data)?;
                self.nodes.get_mut(&new.0).ok_or(Error::InvalidCursor)?.next = None;
                if let Some(previous) = tail {
                    self.nodes
                        .get_mut(&previous.0)
                        .ok_or(Error::InvalidCursor)?
                        .next = Some(new);
                    self.queues
                        .get_mut(&saved.id)
                        .ok_or(Error::QueueKind)?
                        .buckets[bucket] = old_head;
                }
                tail = Some(new);
            }
            // The list is in file order now: its back links follow it.
            let mut previous: Option<EventId> = None;
            let mut cursor = self.queues[&saved.id].buckets[bucket];
            while let Some(event) = cursor {
                let node = self.nodes.get_mut(&event.0).ok_or(Error::InvalidCursor)?;
                node.prev = previous;
                previous = Some(event);
                cursor = node.next;
            }
        }
        Ok(())
    }
}
