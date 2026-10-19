//! The Actor+144 TrackInfo position-history objects of the live World.
//!
//! 4D7230 allocates `6F7810(0x20)` and constructs it with `50AB10(actor,
//! period)`; the pointer is stored at Actor+144. The native object is
//! `{+0 actor, +4 50AAA0 ring {data, count, capacity, +C, +10}, +18 latch,
//! +1C last frame}`. This owner keeps one `aa_spatial::position::
//! PositionHistory` per actor GTID; Actor+144 holds that GTID as the handle.
use aa_spatial::position::PositionHistory;
use std::collections::BTreeMap;

type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Default)]
pub struct Owner {
    entries: BTreeMap<u32, PositionHistory>,
}

impl Owner {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get(&self, actor: u32) -> Option<&PositionHistory> {
        self.entries.get(&actor)
    }
    pub fn get_mut(&mut self, actor: u32) -> Option<&mut PositionHistory> {
        self.entries.get_mut(&actor)
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Publishes the constructed object for the actor handle stored at +144.
    pub fn insert(&mut self, actor: u32, history: PositionHistory) -> Result<()> {
        if actor == 0 {
            return Err("Actor+144 history handle needs a non-null actor GTID".into());
        }
        if self.entries.contains_key(&actor) {
            return Err(format!("Actor {actor:#x} already owns an Actor+144 history"));
        }
        self.entries.insert(actor, history);
        Ok(())
    }
}

/// `50AB10(actor, period)` after its `6F7810(0x20)` allocation succeeded.
///
/// `50AAA0(period)` constructs the ring through `50A830(period + 1, 1)`: the
/// `n * 12` byte allocation is built by the `401000` vector iterator with
/// `79E130`, which zeroes x and y and leaves the third word unconstructed.
/// The ring indices +C/+10 start at 0, so no reader observes that third word
/// before `50A970` writes it; it is held as 0 here. +18 = 0. Then `[[B39AB0]
/// +28]` is read, `4F02B0` logs the seed (the caller's `draw`), `85D840` draws
/// and +1C = `-(draw % TickCountPerTrackPlacement)` (unsigned `div`).
pub fn construct_50ab10(
    period: i32,
    tick_count_per_track_placement: i32,
    draw: impl FnOnce() -> Result<u32>,
) -> Result<PositionHistory> {
    let count = period.wrapping_add(1);
    // 50AAA0 compares the count with capacity and twice capacity; only a
    // positive count with a successful allocation skips 50A8C0.
    if count <= 0 {
        return Err(format!(
            "50AAA0 non-positive ring count {count} (50A8C0 resize path) is not ported"
        ));
    }
    let samples = vec![[0u32; 3]; count as usize];
    let divisor = tick_count_per_track_placement as u32;
    // The draw executes before the 50AB68 `div` faults.
    let value = draw()?;
    if divisor == 0 {
        return Err("50AB68 div by a zero TickCountPerTrackPlacement (#DE)".into());
    }
    let remainder = value % divisor;
    Ok(PositionHistory {
        samples,
        read_index: 0,
        write_index: 0,
        reset_latch: false,
        last_frame: (remainder as i32).wrapping_neg(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_ring_of_period_plus_one() {
        let h = construct_50ab10(6, 10, || Ok(0x1234_5677)).unwrap();
        assert_eq!(h.samples.len(), 7);
        assert_eq!(h.read_index, 0);
        assert_eq!(h.write_index, 0);
        assert!(!h.reset_latch);
        assert_eq!(h.last_frame, -((0x1234_5677u32 % 10) as i32));
    }

    #[test]
    fn unsigned_division_of_the_draw() {
        let h = construct_50ab10(1, 10, || Ok(0xffff_ffff)).unwrap();
        assert_eq!(h.last_frame, -5);
    }

    #[test]
    fn zero_divisor_and_negative_count_are_explicit() {
        assert!(construct_50ab10(3, 0, || Ok(1)).is_err());
        assert!(construct_50ab10(-1, 10, || Ok(1)).is_err());
    }
}
