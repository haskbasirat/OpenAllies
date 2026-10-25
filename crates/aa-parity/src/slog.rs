//! The original simulation sync log S (`TGC_Synchronizer` `[0xB36560]`),
//! recomputed from an entry stream and compared with a film's
//! `KKC_SetSimSyncValueOrderDecree` (311) checkpoints.
//!
//! The arithmetic is `aa_orders::net::sync::Synchronizer` itself:
//! * the reset: session launch 0x775BF0 calls 0x77D900(index 0, hash 0) at
//!   0x775D4A, so the hash starts at 0 and the first entry gets index 1;
//! * every entry: `h = (h * 37 + v) % 65536`, with ints and float bits as
//!   the value, bools as 0/1 and markers as 0. Labels are never hashed;
//! * a checkpoint: the request 0x74E670 sends {last index = end - 1, hash}.
//!   [`SLog::compare`] checks the hash stored after entry `count`, the native
//!   0x77D050 ValidateChecksum rule, so a stream that has run past a
//!   checkpoint is still checked exactly at it.
//!
//! What feeds S per world tick: one clock entry
//! `g_gworld->GetGameTimeBeginTick()` (0x76F835, GWorld+0xE0 before the
//! update: 0, 0, 1/16, ...). Then one `(int)m_rng.GetSeed()` entry (the state
//! before the draw) for every world-RNG getter call 0x4F02B0, plus whatever
//! markers and values the systems log (42 native marker callers and 379
//! int-logger callers, split between A and S, not yet catalogued).
//!
//! A clock value `k/16` has zero low 16 bits, so a clock entry turns the
//! hash into `h * 37 mod 65536` and adds nothing. The film hashes therefore
//! come from the other entries.
use crate::film::Checkpoint;
use aa_orders::net::sync::Synchronizer;
use serde::Serialize;

/// Kind of one S entry (mirrors `aa_play::trace::SKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SEntryKind {
    /// log_float_bits "g_gworld->GetGameTimeBeginTick()".
    Clock,
    /// log_int "(int)m_rng.GetSeed()".
    RngGetter,
    Int(&'static str),
    Float(&'static str),
    Bool(&'static str),
    Marker(&'static str),
    /// log_text (0x77DB10, S wrapper 0x40F950): one entry, every UTF-16
    /// unit hashed in turn; pushed with [`SLog::push_text`].
    Text(&'static str),
}
impl SEntryKind {
    pub fn label(self) -> &'static str {
        match self {
            SEntryKind::Clock => "g_gworld->GetGameTimeBeginTick()",
            SEntryKind::RngGetter => "(int)m_rng.GetSeed()",
            SEntryKind::Int(l)
            | SEntryKind::Float(l)
            | SEntryKind::Bool(l)
            | SEntryKind::Marker(l)
            | SEntryKind::Text(l) => l,
        }
    }
}

/// The S entries an executed decree logs: the native decree handlers and the
/// host request path (aa-orders `flow::log_decree`) write the decree class
/// name (text, label `m_decree_type.m_name`) and its descriptor id (int,
/// `m_decree_type.m_id`) to S when the handler's shape has `s_log` and the
/// decree is not `TGC_SetPlayerOrderDecree` (the 0x96CB83 name compare
/// against 0x9FD610 in every handler, e.g. 0x750DD0, 0x7509F0). Returns the
/// name; the id is `descriptor`.
pub fn decree_s_entries(descriptor: usize) -> Option<&'static str> {
    let shape = aa_orders::flow::shape(descriptor).ok()?;
    if !shape.s_log || descriptor == aa_orders::flow::SET_PLAYER_DECREE {
        return None;
    }
    aa_orders::flow::decree_name(descriptor).ok()
}
#[cfg(feature = "play")]
impl From<aa_play::trace::SKind> for SEntryKind {
    fn from(kind: aa_play::trace::SKind) -> Self {
        use aa_play::trace::SKind;
        match kind {
            SKind::Clock => SEntryKind::Clock,
            SKind::RngGetter => SEntryKind::RngGetter,
            SKind::Int(l) => SEntryKind::Int(l),
            SKind::Float(l) => SEntryKind::Float(l),
            SKind::Bool(l) => SEntryKind::Bool(l),
            SKind::Marker(l) => SEntryKind::Marker(l),
            // (core) the name units are `Trace::decree_name(value)`; see
            // `SLog::from_play_trace`.
            SKind::DecreeName => SEntryKind::Text("m_decree_type.m_name"),
        }
    }
}

/// GWorld+0xE0 begin time logged before world update `update` (1-based):
/// 0 for the first two updates, then `(update - 2) / 16` (binary32 bits;
/// the native end is `f32(i32 tick * f32(1/16))`, exact for these ticks).
pub fn clock_bits_before_update(update: u32) -> u32 {
    let k = update.saturating_sub(2);
    (k as f32 * 0.0625).to_bits()
}

/// The recomputed S log.
#[derive(Clone, Debug)]
pub struct SLog {
    sync: Synchronizer,
    /// The 0x77D900 reset arguments (index, hash).
    reset: (i32, i32),
    /// World tick of every entry, from the first entry after the reset.
    ticks: Vec<u32>,
    kinds: Vec<SEntryKind>,
}

impl Default for SLog {
    fn default() -> Self {
        Self::new()
    }
}

impl SLog {
    /// The session-launch state: 0x77D900(0, 0).
    pub fn new() -> Self {
        Self::with_reset(0, 0)
    }
    /// Another reset hypothesis: the next entry gets `index + 1`.
    pub fn with_reset(index: i32, hash: i32) -> Self {
        // Retain every entry: the comparison wants the whole stream (the
        // native window keeps 0x10000).
        let mut sync = Synchronizer::new(i32::MAX);
        sync.reset(index, hash);
        Self {
            sync,
            reset: (index, hash),
            ticks: Vec::new(),
            kinds: Vec::new(),
        }
    }
    /// One entry, logged during world tick `tick`.
    pub fn push(&mut self, tick: u32, kind: SEntryKind, value: u32) {
        let label = kind.label();
        match kind {
            SEntryKind::Clock | SEntryKind::Float(_) => {
                self.sync.log_float_bits(true, label, value)
            }
            SEntryKind::RngGetter | SEntryKind::Int(_) => self.sync.log_int(true, label, value),
            SEntryKind::Bool(_) => self.sync.log_bool(true, label, value != 0),
            SEntryKind::Marker(_) => self.sync.log_marker(true, label),
            SEntryKind::Text(_) => self.sync.log_text(true, label, &[]),
        }
        self.ticks.push(tick);
        self.kinds.push(kind);
    }
    /// One text entry (0x77DB10): every unit of `units` is hashed.
    pub fn push_text(&mut self, tick: u32, label: &'static str, units: &[u16]) {
        self.sync.log_text(true, label, units);
        self.ticks.push(tick);
        self.kinds.push(SEntryKind::Text(label));
    }
    /// The two S entries of an executed decree ([`decree_s_entries`]);
    /// nothing for a decree that does not log to S.
    pub fn push_decree(&mut self, tick: u32, descriptor: usize) {
        if let Some(name) = decree_s_entries(descriptor) {
            let units: Vec<u16> = name.encode_utf16().collect();
            self.push_text(tick, "m_decree_type.m_name", &units);
            self.push(
                tick,
                SEntryKind::Int("m_decree_type.m_id"),
                descriptor as u32,
            );
        }
    }
    pub fn extend<K: Into<SEntryKind>>(
        &mut self,
        entries: impl IntoIterator<Item = (u32, K, u32)>,
    ) {
        for (tick, kind, value) in entries {
            self.push(tick, kind.into(), value);
        }
    }
    /// The recorded stream of an aa-play trace (`World::trace` with
    /// recording on), from its reset state (0, 0).
    #[cfg(feature = "play")]
    pub fn from_play_trace(trace: &aa_play::trace::Trace) -> Self {
        let mut out = Self::new();
        for (tick, kind, value) in trace.entries() {
            if kind == aa_play::trace::SKind::DecreeName {
                // (core) a decree name entry carries the descriptor id; its
                // UTF-16 name is kept by the trace.
                let units = trace.decree_name(value).unwrap_or(&[]).to_vec();
                out.push_text(tick, "m_decree_type.m_name", &units);
            } else {
                out.push(tick, kind.into(), value);
            }
        }
        out
    }
    /// What a 311 checkpoint would carry now: (last index, hash).
    pub fn checkpoint(&self) -> (i32, i32) {
        (self.sync.end().wrapping_sub(1), self.sync.hash)
    }
    pub fn len(&self) -> usize {
        self.ticks.len()
    }
    pub fn is_empty(&self) -> bool {
        self.ticks.is_empty()
    }
    /// Entry `index` (native numbering): (tick, kind, value, hash after).
    pub fn entry(&self, index: i32) -> Option<(u32, SEntryKind, u32, i32)> {
        let entry = self.sync.entry(index)?;
        let at = index.wrapping_sub(self.sync.begin()) as usize;
        Some((
            *self.ticks.get(at)?,
            *self.kinds.get(at)?,
            entry.value,
            entry.hash,
        ))
    }
    /// (last index, hash) after the last entry of world tick `tick`, i.e.
    /// what a checkpoint emitted before update `tick + 1` carries.
    pub fn checkpoint_after_tick(&self, tick: u32) -> (i32, i32) {
        let n = self.ticks.partition_point(|&t| t <= tick);
        if n == 0 {
            return self.reset;
        }
        let index = self.sync.begin().wrapping_add(n as i32 - 1);
        (
            index,
            self.sync.entry(index).map_or(self.reset.1, |e| e.hash),
        )
    }

    /// Compare with a film's checkpoints.
    pub fn compare(&self, checkpoints: &[Checkpoint]) -> Comparison {
        let mut results = Vec::new();
        for c in checkpoints {
            let ours = self.entry(c.count);
            let outcome = match ours {
                None if c.count >= self.sync.end() => Outcome::NotReached,
                None => Outcome::Forgotten,
                Some((_, _, _, hash)) if hash == c.hash => Outcome::Match,
                Some(_) => Outcome::Mismatch,
            };
            results.push(CheckResult {
                film_tick: c.tick,
                count: c.count,
                film_hash: c.hash,
                our_hash: ours.map(|o| o.3),
                our_tick: ours.map(|o| o.0),
                our_count_at_film_tick: self.checkpoint_after_tick(c.tick).0,
                outcome,
            });
        }
        Comparison { results }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Outcome {
    Match,
    Mismatch,
    /// Our stream has fewer entries than the checkpoint index.
    NotReached,
    /// Below our first entry (another reset hypothesis).
    Forgotten,
}

#[derive(Clone, Debug, Serialize)]
pub struct CheckResult {
    pub film_tick: u32,
    pub count: i32,
    pub film_hash: i32,
    /// Our hash after entry `count`.
    pub our_hash: Option<i32>,
    /// The tick in which our stream logged entry `count`.
    pub our_tick: Option<u32>,
    /// Our last index at the film's tick (tick alignment, informational).
    pub our_count_at_film_tick: i32,
    pub outcome: Outcome,
}

#[derive(Clone, Debug, Serialize)]
pub struct Comparison {
    pub results: Vec<CheckResult>,
}
impl Comparison {
    pub fn matched(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.outcome == Outcome::Match)
            .count()
    }
    pub fn first_mismatch(&self) -> Option<&CheckResult> {
        self.results.iter().find(|r| r.outcome != Outcome::Match)
    }
}

/// Film-only check of intervals between consecutive checkpoints whose entry
/// count grew exactly by the number of world ticks: if only clock entries
/// were logged there, the next hash is `h * 37^n mod 65536`. Returns
/// (interval start tick, ticks, predicted == film hash).
pub fn clock_only_intervals(checkpoints: &[Checkpoint]) -> Vec<(u32, u32, bool)> {
    checkpoints
        .windows(2)
        .filter_map(|w| {
            let ticks = w[1].tick - w[0].tick;
            (w[1].count - w[0].count == ticks as i32).then(|| {
                let mut h = w[0].hash;
                for k in 0..ticks {
                    h = aa_orders::net::sync::step(h, clock_bits_before_update(w[0].tick + 1 + k));
                }
                (w[0].tick, ticks, h == w[1].hash)
            })
        })
        .collect()
}

/// One interval between consecutive film checkpoints, with the S entries the
/// film itself accounts for: one clock entry per world update and the two
/// entries of every S-logging decree executed in the interval. `residual` is
/// what remains for the world-RNG getters and the other S loggers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Interval {
    /// Checkpoint record that opens the interval (0 = the session-launch
    /// reset).
    pub from_tick: u32,
    pub to_tick: u32,
    /// S entries the film's checkpoints say were logged in the interval.
    pub entries: i32,
    pub clock: i32,
    pub decree_entries: i32,
    pub residual: i32,
}

/// Account a film's S entries interval by interval. A checkpoint in record
/// `r` is captured at the start of its admin tick (0x74E670 before 0x752E60),
/// so it holds `r` world updates and the decrees of records `0..r`; the
/// decrees of record `r` itself (the 311 included) fall in the next interval.
pub fn account(film: &crate::Film) -> Vec<Interval> {
    let mut out = Vec::new();
    let mut prev = Checkpoint {
        tick: 0,
        count: 0,
        hash: 0,
    };
    for c in &film.checkpoints {
        let decrees: i32 = film
            .ticks
            .iter()
            .filter(|t| t.tick >= prev.tick && t.tick < c.tick)
            .flat_map(|t| &t.decrees)
            .filter(|d| decree_s_entries(d.descriptor as usize).is_some())
            .count() as i32
            * 2;
        let entries = c.count - prev.count;
        let clock = (c.tick - prev.tick) as i32;
        out.push(Interval {
            from_tick: prev.tick,
            to_tick: c.tick,
            entries,
            clock,
            decree_entries: decrees,
            residual: entries - clock - decrees,
        });
        prev = *c;
    }
    out
}
/// Replay the film's own S model over every interval whose
/// [`Interval::residual`] is zero (only clock and decree entries): start
/// from the opening checkpoint's hash, log the decrees of record `r1`, then
/// for every update `k = r1 + 1 ..= r2` its clock entry and the decrees of
/// record `k` (except record `r2`, which follows the checkpoint). Returns
/// (interval, predicted hash, film hash) for those intervals. Matching hashes
/// confirm the entry kinds, their values and their order.
pub fn verify_zero_residual(film: &crate::Film) -> Vec<(Interval, i32, i32)> {
    let mut out = Vec::new();
    let mut prev = Checkpoint {
        tick: 0,
        count: 0,
        hash: 0,
    };
    for (interval, c) in account(film).into_iter().zip(&film.checkpoints) {
        if interval.residual == 0 {
            let mut log = SLog::with_reset(prev.count, prev.hash);
            for d in film.decrees_at(prev.tick) {
                log.push_decree(prev.tick, d.descriptor as usize);
            }
            for k in prev.tick + 1..=c.tick {
                log.push(k, SEntryKind::Clock, clock_bits_before_update(k));
                if k < c.tick {
                    for d in film.decrees_at(k) {
                        log.push_decree(k, d.descriptor as usize);
                    }
                }
            }
            debug_assert_eq!(log.checkpoint().0, c.count);
            out.push((interval, log.checkpoint().1, c.hash));
        }
        prev = *c;
    }
    out
}
