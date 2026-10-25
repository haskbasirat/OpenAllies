//! The generator's random numbers: the world stream itself, as in the
//! original (KKC_Sim::CreateGWorld 0x76AA90 logs the GetRNG 0x4F02B0 getter
//! with the default state, seeds GWorld+0xD4 with 0x85D880(creator seed),
//! GWorld::Init 0x4F27E0 runs, then the random map generator draws every
//! value through 0x4F02B0 / 0x85D840). Every stage draws from that one
//! stream in turn; the world continues from where generation left it
//! (`GameMap::rng_state`, with the getter states in `GameMap::rng_log`).
//! Unit values keep the generator's `u32 * 2^-32` reading.
//!
//! Parity tracing: every draw can be recorded with the native function it
//! stands for (`Rng::site`, the original function address), so the draw
//! sequence can be compared with the original's (aa-parity `rmg_parity`).
use std::cell::RefCell;
use std::rc::Rc;

/// `0x9FDA98`: binary64 0x3DF0000000100000, the nearest to 1/(2^32 - 1).
pub(super) const UNIT: f64 = f64::from_bits(0x3DF0_0000_0010_0000);

#[derive(Debug)]
struct Stream {
    rng: aa_simulation::NativeRng,
    /// The state each getter call logged.
    log: Vec<u32>,
    /// The native function the next draws stand for.
    site: u32,
    /// (state before the draw, site) of every draw, while tracing.
    trace: Option<Vec<(u32, u32)>>,
}

#[derive(Clone, Debug)]
pub(super) struct Rng {
    stream: Rc<RefCell<Stream>>,
}

impl Rng {
    /// The world stream of a new world created with `seed` (the creator
    /// seed, a u32 natively), after GWorld::Init's draws
    /// (`trace::GWORLD_INIT_DRAWS`, none assumed).
    pub(super) fn new(seed: u64) -> Self {
        let default = aa_simulation::NativeRng::default();
        let stream = Stream { rng: aa_simulation::NativeRng::from_native_seed(seed as u32), log: vec![default.state], site: 0, trace: None };
        let mut rng = Self { stream: Rc::new(RefCell::new(stream)) };
        for _ in 0..crate::trace::GWORLD_INIT_DRAWS {
            rng.next_u32();
        }
        rng
    }
    /// The world stream as a caller left it: `state` now, `log` every getter
    /// entry so far (the creator's kingdoms and picks draw before the
    /// generator in the original).
    pub(super) fn resume(state: u32, log: Vec<u32>) -> Self {
        let stream = Stream { rng: aa_simulation::NativeRng { state }, log, site: 0, trace: None };
        Self { stream: Rc::new(RefCell::new(stream)) }
    }
    /// Set the native site of the next draws and return the stream, for
    /// `rng.at(0x59717E).next_u32()`.
    pub(super) fn at(&mut self, native: u32) -> &mut Self {
        self.stream.borrow_mut().site = native;
        self
    }
    /// Record every draw from now on (see [`Self::take_trace`]).
    pub(super) fn trace(&self) {
        self.stream.borrow_mut().trace.get_or_insert_with(Vec::new);
    }
    /// The recorded draws, (state before, native site).
    pub(super) fn take_trace(&self) -> Vec<(u32, u32)> {
        self.stream.borrow_mut().trace.take().unwrap_or_default()
    }
    /// The same stream: the original has one, and the stages draw from it
    /// one after the other.
    pub(super) fn fork(&mut self, _salt: u64) -> Self {
        self.clone()
    }
    /// GetRNG 0x4F02B0 alone (it logs the state; the caller then draws
    /// nothing, e.g. 0x4A04D0 with fewer than two choices).
    pub(super) fn getter(&mut self) {
        let mut s = self.stream.borrow_mut();
        let state = s.rng.state;
        s.log.push(state);
    }
    /// The 0x85D840 draw on a state a GetRNG getter already logged (the
    /// original calls 0x4F02B0 in the caller and 0x85D840 in the callee).
    pub(super) fn draw_after_getter(&mut self) -> u32 {
        let mut s = self.stream.borrow_mut();
        let state = s.rng.state;
        let site = s.site;
        if let Some(t) = &mut s.trace {
            t.push((state, site));
        }
        s.rng.next_u32()
    }
    /// GetRNG 0x4F02B0 (logs the state) then one 0x85D840 draw.
    pub(super) fn next_u32(&mut self) -> u32 {
        let mut s = self.stream.borrow_mut();
        let state = s.rng.state;
        s.log.push(state);
        let site = s.site;
        if let Some(t) = &mut s.trace {
            t.push((state, site));
        }
        s.rng.next_u32()
    }
    /// The stream state now (GWorld+0xD4).
    pub(super) fn state(&self) -> u32 {
        self.stream.borrow().rng.state
    }
    /// Every getter entry so far.
    pub(super) fn log(&self) -> Vec<u32> {
        self.stream.borrow().log.clone()
    }
    /// The original's unit idiom on the x87 register: `fild` the draw (plus
    /// 2^32 when negative) then `fmul qword [0x9FDA98]`, the binary64
    /// nearest 1/(2^32 - 1), so 0xFFFFFFFF reads 1.0. Callers keep the
    /// value in the register (binary64 at PC53) until the original stores.
    pub(super) fn unit_d(&mut self) -> f64 {
        f64::from(self.next_u32()) * UNIT
    }
    /// 0x85D840 result scaled like the native `u32 * 2.3283064e-10`: [0, 1).
    pub(super) fn unit(&mut self) -> f32 {
        let v = (f64::from(self.next_u32()) * (1.0 / 4_294_967_296.0)) as f32;
        if v >= 1.0 { 0.999_999_94 } else { v }
    }
    /// `(max - min) * unit + min`, the native range idiom.
    pub(super) fn range(&mut self, r: [f32; 2]) -> f32 {
        (r[1] - r[0]) * self.unit() + r[0]
    }
    /// `u32 % n` (n > 0).
    pub(super) fn below(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { self.next_u32() % n }
    }
    /// `unit * 2 - 1`.
    pub(super) fn signed(&mut self) -> f32 {
        let u = self.unit();
        u + u - 1.0
    }
}
