//! Seeded random numbers for the generator. The original draws every value
//! from the world's random stream (0x4F02B0 / 0x85D840, a u32 per call, unit
//! values as u32 * 2^-32); this is our own PCG32 with the same call shapes,
//! so the same params + seed always give the same map.

#[derive(Clone, Debug)]
pub(super) struct Rng {
    state: u64,
    inc: u64,
}

impl Rng {
    pub(super) fn new(seed: u64) -> Self {
        let mut rng = Self { state: 0, inc: (seed.rotate_left(17) ^ 0xDA3E_39CB_94B9_5BDB) << 1 | 1 };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }
    /// An independent stream for one generation stage (keeps a stage's
    /// output stable when an earlier stage changes how many values it draws).
    pub(super) fn fork(&mut self, salt: u64) -> Self {
        let a = u64::from(self.next_u32());
        let b = u64::from(self.next_u32());
        Self::new((a << 32 | b) ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }
    pub(super) fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
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
