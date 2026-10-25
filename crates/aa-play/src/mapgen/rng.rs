//! The generator's random numbers: the world stream itself, as in the
//! original (KKC_Sim::CreateGWorld 0x76AA90 logs the GetRNG 0x4F02B0 getter
//! with the default state, seeds GWorld+0xD4 with 0x85D880(creator seed),
//! GWorld::Init 0x4F27E0 runs, then the random map generator draws every
//! value through 0x4F02B0 / 0x85D840). Every stage draws from that one
//! stream in turn; the world continues from where generation left it
//! (`GameMap::rng_state`, with the getter states in `GameMap::rng_log`).
//! Unit values keep the generator's `u32 * 2^-32` reading.
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
struct Stream {
    rng: aa_simulation::NativeRng,
    /// The state each getter call logged.
    log: Vec<u32>,
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
        let stream = Stream { rng: aa_simulation::NativeRng::from_native_seed(seed as u32), log: vec![default.state] };
        let mut rng = Self { stream: Rc::new(RefCell::new(stream)) };
        for _ in 0..crate::trace::GWORLD_INIT_DRAWS {
            rng.next_u32();
        }
        rng
    }
    /// The same stream: the original has one, and the stages draw from it
    /// one after the other.
    pub(super) fn fork(&mut self, _salt: u64) -> Self {
        self.clone()
    }
    /// GetRNG 0x4F02B0 (logs the state) then one 0x85D840 draw.
    pub(super) fn next_u32(&mut self) -> u32 {
        let mut s = self.stream.borrow_mut();
        let state = s.rng.state;
        s.log.push(state);
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
