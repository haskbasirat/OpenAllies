//! Parity trace: the simulation S-log the original keeps for films
//! (TGC_Synchronizer S = [0xB36560], checked against the films'
//! KKC_SetSimSyncValueOrderDecree 311 checkpoints) and a per-tick digest of
//! the world. Owner: core. The interface was agreed with the aa-parity
//! harness, which reads `Trace::log` as `(tick, kind, value)`.
//!
//! What is logged, as the original logs it:
//! - one clock entry per world tick (0x752AE0 after the tick's decrees,
//!   0x76F835): `g_gworld->GetGameTimeBeginTick()`, the GWorld+0xE0 begin
//!   time's binary32 bits before the update, so 0, 0, 1/16, 2/16...;
//! - one getter entry per world-RNG use: GWorld::GetRNG 0x4F02B0 logs
//!   `(int)m_rng.GetSeed()` (the GWorld+0xD4 state before the draw) to A and
//!   then S;
//! - two entries per executed decree with an S log (all but SetPlayer 298):
//!   its class name as text, then its descriptor id (`World::step_decrees`).
//!
//! Those three are everything the original logs to S (the parity census,
//! research/byte-exact/SYNC_LOG.md); markers, bools and dumps go to A only.
//!
//! The S instance starts as session launch 0x775BF0 leaves it: reset
//! 0x77D900(index 0, hash 0), so the first entry gets index 1. What world
//! creation logged before that reset is wiped (only the RNG state carries
//! on). Hashing is the native rolling hash (`aa_orders::net::sync::step`);
//! labels are never hashed. Everything aa-play does not port yet (markers and
//! value logs of other systems, RNG sites it does not share with the
//! original) is missing from the stream, so only the clock and the RNG call
//! sequence can match today.
use serde::Serialize;

/// Kind of one S entry. Open for the markers and value logs of systems as
/// they become exact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SKind {
    /// log_float_bits "g_gworld->GetGameTimeBeginTick()".
    Clock,
    /// log_int "(int)m_rng.GetSeed()".
    RngGetter,
    /// log_int (0x77DA70) with its label.
    Int(&'static str),
    /// log_float_bits (0x77D930): the value is the binary32 bits.
    Float(&'static str),
    /// log_bool (0x77D9D0): the value is 0 or 1.
    Bool(&'static str),
    /// log_marker (0x77DC50): hashes 0.
    Marker(&'static str),
    /// log_text (0x77DB10) "m_decree_type.m_name" of an executed decree: one
    /// hash step per UTF-16 unit of the decree class name. The value is the
    /// decree descriptor id; the name is `Trace::decree_name(id)`.
    DecreeName,
}

/// Label of the int entry that follows a decree name.
pub const DECREE_ID_LABEL: &str = "m_decree_type.m_id";

impl SKind {
    /// The native label (stored by the Synchronizer, never hashed).
    pub fn label(self) -> &'static str {
        match self {
            SKind::Clock => "g_gworld->GetGameTimeBeginTick()",
            SKind::RngGetter => "(int)m_rng.GetSeed()",
            SKind::Int(l) | SKind::Float(l) | SKind::Bool(l) | SKind::Marker(l) => l,
            SKind::DecreeName => "m_decree_type.m_name",
        }
    }
    /// The value the native logger hashes.
    fn hashed(self, value: u32) -> u32 {
        match self {
            SKind::Bool(_) => u32::from(value != 0),
            SKind::Marker(_) => 0,
            _ => value,
        }
    }
}

/// One S entry: the world tick it was logged in, its kind and raw value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SEntry {
    pub tick: u32,
    pub kind: SKind,
    pub value: u32,
}

/// The world after one tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct TickDigest {
    pub tick: u32,
    /// Battle time (end of the tick) bits.
    pub time_bits: u32,
    /// GWorld+0xD4.
    pub rng_state: u32,
    /// S window end (the index the next entry gets) and hash.
    pub s_end: i32,
    pub s_hash: i32,
    /// FNV-1a over the kingdoms' stockpile bits, then every actor
    /// (definition, position, facing and health bits, alive / removed), then
    /// every organization (member count, anchor, alive).
    pub world: u64,
}

/// S-log emulation and digests (`World::trace`).
#[derive(Clone, Debug, Serialize)]
pub struct Trace {
    /// Rolling S hash.
    pub s_hash: i32,
    /// Index the next S entry gets.
    pub s_end: i32,
    /// The full S stream while recording.
    pub log: Option<Vec<SEntry>>,
    /// One digest per tick while recording.
    pub digests: Vec<TickDigest>,
    /// The world tick entries are logged in.
    pub tick: u32,
    /// The class name (UTF-16) of every decree descriptor logged.
    pub decree_names: std::collections::BTreeMap<u32, Vec<u16>>,
}

impl Default for Trace {
    /// Session launch: reset(0, 0).
    fn default() -> Self {
        Self::new(0, 0)
    }
}

impl Trace {
    /// 0x77D900 reset(index, hash): the next entry gets `index + 1`.
    pub fn new(initial_index: i32, initial_hash: i32) -> Self {
        Self {
            s_hash: initial_hash,
            s_end: initial_index.wrapping_add(1),
            log: None,
            digests: Vec::new(),
            tick: 0,
            decree_names: Default::default(),
        }
    }
    /// Reset the S state (keeps recording on or off).
    pub fn reset(&mut self, initial_index: i32, initial_hash: i32) {
        let recording = self.recording();
        let names = std::mem::take(&mut self.decree_names);
        *self = Self::new(initial_index, initial_hash);
        self.decree_names = names;
        self.record(recording);
    }
    /// Keep (or stop keeping) the entry stream and the per-tick digests.
    /// The S hash and index always run.
    pub fn record(&mut self, on: bool) {
        if on {
            self.log.get_or_insert_with(Vec::new);
        } else {
            self.log = None;
            self.digests.clear();
        }
    }
    pub fn recording(&self) -> bool {
        self.log.is_some()
    }
    /// One S entry (the native loggers' common arithmetic).
    pub fn log(&mut self, kind: SKind, value: u32) {
        self.s_hash = aa_orders::net::sync::step(self.s_hash, kind.hashed(value));
        self.s_end = self.s_end.wrapping_add(1);
        if let Some(log) = &mut self.log {
            log.push(SEntry { tick: self.tick, kind, value });
        }
    }
    /// The S entries of one executed decree (admin tick, before its effect):
    /// its class name as text (one hash step per UTF-16 unit), then its
    /// descriptor id as an int.
    pub fn log_decree(&mut self, id: u32, name: &[u16]) {
        for &u in name {
            self.s_hash = aa_orders::net::sync::step(self.s_hash, u32::from(u));
        }
        self.s_end = self.s_end.wrapping_add(1);
        if let Some(log) = &mut self.log {
            log.push(SEntry { tick: self.tick, kind: SKind::DecreeName, value: id });
        }
        self.decree_names.entry(id).or_insert_with(|| name.to_vec());
        self.log(SKind::Int(DECREE_ID_LABEL), id);
    }
    /// The class name logged for decree descriptor `id`.
    pub fn decree_name(&self, id: u32) -> Option<&[u16]> {
        self.decree_names.get(&id).map(Vec::as_slice)
    }
    /// What a 311 checkpoint carries (0x74E670): the last entry's index and
    /// the hash after it.
    pub fn checkpoint(&self) -> (i32, i32) {
        (self.s_end.wrapping_sub(1), self.s_hash)
    }
    /// The recorded stream as `(tick, kind, value)`.
    pub fn entries(&self) -> impl Iterator<Item = (u32, SKind, u32)> + '_ {
        self.log.iter().flatten().map(|e| (e.tick, e.kind, e.value))
    }
}

/// World-RNG draws GWorld::Init 0x4F27E0 makes after CreateGWorld seeds the
/// stream (before any map is generated or loaded). Not established; none is
/// assumed.
pub const GWORLD_INIT_DRAWS: u32 = 0;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn mix(h: &mut u64, v: u32) {
    for b in v.to_le_bytes() {
        *h ^= u64::from(b);
        *h = h.wrapping_mul(FNV_PRIME);
    }
}

/// The digest of `world` now.
pub fn digest(world: &crate::World) -> TickDigest {
    let mut h = FNV_OFFSET;
    for k in &world.kingdoms {
        for v in k.stockpile {
            mix(&mut h, v.to_bits());
        }
    }
    for a in &world.actors {
        mix(&mut h, a.def);
        mix(&mut h, a.pos.x.to_bits());
        mix(&mut h, a.pos.y.to_bits());
        mix(&mut h, a.facing.to_bits());
        mix(&mut h, a.health.to_bits());
        mix(&mut h, u32::from(a.alive) | u32::from(a.removed) << 1);
    }
    for o in &world.orgs {
        mix(&mut h, o.members.len() as u32);
        mix(&mut h, o.anchor.x.to_bits());
        mix(&mut h, o.anchor.y.to_bits());
        mix(&mut h, u32::from(o.alive));
    }
    TickDigest {
        tick: world.clock.tick,
        time_bits: world.time.to_bits(),
        rng_state: world.native_rng.state,
        s_end: world.trace.s_end,
        s_hash: world.trace.s_hash,
        world: h,
    }
}

/// 0x45D400 / inline 0x4F62BB at PC53: the draw read as a signed integer
/// (plus 2^32 when negative), times the binary64 constant
/// 0x3DF0000000100000, rounded once to 53 bits, stored as binary32. Every
/// u32 is exact in binary64 and the product is a normal number, so host f64
/// rounds as PC53 does (checked against `aa_simulation::native_unit_float`
/// in the tests). The result can equal 1.0.
pub fn native_unit(value: u32) -> f32 {
    (f64::from(value) * f64::from_bits(0x3df0_0000_0010_0000)) as f32
}
