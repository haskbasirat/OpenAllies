//! Which world-RNG getter entries explain a film's S residuals?
//!
//! After the clock and decree entries ([`crate::slog::account`]), the S log of
//! a film holds only `(int)m_rng.GetSeed()` entries, whose values are
//! world-RNG states. The static census (research/byte-exact/SYNC_LOG.md)
//! finds no other S logger.
//!
//! Every hash step is congruent mod 65536 to `37 * h + v`. The low 16 bits
//! of the LCG 0x85D840 (`state * 0x41C64E6D + 0x3039`) form their own affine
//! map mod 2^16, `x -> 0x4E6D * x + 0x3039`. So, for a fixed placement of the
//! getters, a checkpoint hash is an affine function `alpha * x + beta` of the
//! low 16 bits `x` of the first getter's state, and the placements of an
//! interval give a set of possible `x` (a linear congruence each).
//!
//! The search works under a draw hypothesis: every getter is followed by
//! `draws` LCG steps and nothing else draws. It then chains the intervals
//! (the next interval starts `residual * draws` steps later) and keeps the
//! starting states that explain every interval. A unique survivor fixes the
//! low 16 bits of the state at battle start and the slot of every getter.
//!
//! Slots of an interval, in log order:
//! * after the S entries of each S-logging decree of records `r1..r2` (a
//!   getter run while that decree is processed in its admin tick);
//! * after the clock entry of each update `k` in `r1 + 1 ..= r2` (a getter
//!   run during GWorld update `k`).
use crate::film::Film;
use crate::slog::{account, clock_bits_before_update, decree_s_entries};
use serde::Serialize;

const MASK: u32 = 0xffff;
const MUL: u32 = 0x4e6d;
const ADD: u32 = 0x3039;

/// Low 16 bits of one native LCG step.
pub fn lcg16(x: u32) -> u32 {
    (x.wrapping_mul(MUL).wrapping_add(ADD)) & MASK
}
/// `n` steps as an affine map `x -> a * x + b` (mod 2^16).
fn lcg16_affine(n: u32) -> (u32, u32) {
    let (mut a, mut b) = (1u32, 0u32);
    for _ in 0..n {
        a = a.wrapping_mul(MUL) & MASK;
        b = (b.wrapping_mul(MUL).wrapping_add(ADD)) & MASK;
    }
    (a, b)
}
pub fn lcg16_n(x: u32, n: u32) -> u32 {
    let (a, b) = lcg16_affine(n);
    (a.wrapping_mul(x).wrapping_add(b)) & MASK
}
fn pow37(n: u32) -> u32 {
    let mut r = 1u32;
    for _ in 0..n {
        r = r.wrapping_mul(37) & MASK;
    }
    r
}

/// Where a getter entry sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Slot {
    /// After the S entries of decree `index` of film record `record`.
    Decree {
        record: u32,
        index: usize,
        descriptor: u16,
    },
    /// During GWorld update `update`, after its clock entry.
    Update { update: u32 },
}

/// One checkpoint interval prepared for the search.
#[derive(Clone, Debug)]
pub struct Interval {
    pub from_tick: u32,
    pub to_tick: u32,
    pub residual: u32,
    pub slots: Vec<Slot>,
    start_hash: u32,
    target: u32,
    /// Fixed hash steps after each slot, and the value of the fixed steps
    /// between slot `k - 1` and slot `k` (segment `k`, the last one closes
    /// the interval), evaluated at the segment end.
    after_slot: Vec<u32>,
    segments: Vec<(u32, u32)>,
    fixed_steps: u32,
}

/// Prepare every checkpoint interval of a film.
pub fn prepare(film: &Film) -> Vec<Interval> {
    let mut out = Vec::new();
    let mut prev = (0u32, 0i32);
    for (accounted, c) in account(film).into_iter().zip(&film.checkpoints) {
        let mut steps: Vec<u32> = Vec::new();
        let mut slots: Vec<(Slot, usize)> = Vec::new();
        let decrees = |record: u32, steps: &mut Vec<u32>, slots: &mut Vec<(Slot, usize)>| {
            for (index, d) in film.decrees_at(record).iter().enumerate() {
                if let Some(name) = decree_s_entries(d.descriptor as usize) {
                    steps.extend(name.encode_utf16().map(u32::from));
                    steps.push(u32::from(d.descriptor));
                    slots.push((
                        Slot::Decree {
                            record,
                            index,
                            descriptor: d.descriptor,
                        },
                        steps.len(),
                    ));
                }
            }
        };
        decrees(prev.0, &mut steps, &mut slots);
        for k in prev.0 + 1..=c.tick {
            steps.push(clock_bits_before_update(k) & MASK);
            slots.push((Slot::Update { update: k }, steps.len()));
            if k < c.tick {
                decrees(k, &mut steps, &mut slots);
            }
        }
        let n = steps.len();
        let mut segments = Vec::new();
        let mut start = 0usize;
        for &(_, end) in slots
            .iter()
            .chain(std::iter::once(&(Slot::Update { update: 0 }, n)))
        {
            let mut v = 0u32;
            for &s in &steps[start..end] {
                v = (v.wrapping_mul(37).wrapping_add(s)) & MASK;
            }
            segments.push((v, (n - end) as u32));
            start = end;
        }
        out.push(Interval {
            from_tick: prev.0,
            to_tick: c.tick,
            residual: accounted.residual.max(0) as u32,
            after_slot: slots.iter().map(|&(_, at)| (n - at) as u32).collect(),
            slots: slots.into_iter().map(|(s, _)| s).collect(),
            start_hash: (prev.1 as u32) & MASK,
            target: (c.hash as u32) & MASK,
            segments,
            fixed_steps: n as u32,
        });
        prev = (c.tick, c.hash);
    }
    out
}

/// `alpha * x + beta` for getters at `pick` (non-decreasing slot indices).
fn affine(iv: &Interval, pick: &[usize], draws: u32) -> (u32, u32) {
    let r = pick.len() as u32;
    let mut beta = iv.start_hash.wrapping_mul(pow37(iv.fixed_steps + r)) & MASK;
    for (k, &(value, after)) in iv.segments.iter().enumerate() {
        // getters placed at slot k or later come after segment k
        let later = pick.iter().filter(|&&p| p >= k).count() as u32;
        beta = beta.wrapping_add(value.wrapping_mul(pow37(after + later))) & MASK;
    }
    let mut alpha = 0u32;
    for (j, &p) in pick.iter().enumerate() {
        let coef = pow37(iv.after_slot[p] + (r - 1 - j as u32));
        let (a, b) = lcg16_affine(j as u32 * draws);
        alpha = alpha.wrapping_add(coef.wrapping_mul(a)) & MASK;
        beta = beta.wrapping_add(coef.wrapping_mul(b)) & MASK;
    }
    (alpha, beta)
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}
/// Modular inverse of odd `a` mod 2^16.
fn inverse_odd(a: u32) -> u32 {
    let mut x = 1u32;
    for _ in 0..5 {
        x = x.wrapping_mul(2u32.wrapping_sub(a.wrapping_mul(x)));
    }
    x & MASK
}

/// For every `x`, the first placement (slot indices) explaining the interval
/// under `draws`, or `None`.
pub fn solutions(iv: &Interval, draws: u32) -> Vec<Option<Vec<usize>>> {
    let mut out: Vec<Option<Vec<usize>>> = vec![None; 1 << 16];
    let r = iv.residual as usize;
    let mut pick = vec![0usize; r];
    fn walk(
        iv: &Interval,
        draws: u32,
        depth: usize,
        from: usize,
        pick: &mut Vec<usize>,
        out: &mut Vec<Option<Vec<usize>>>,
    ) {
        if depth == pick.len() {
            let (alpha, beta) = affine(iv, pick, draws);
            let rhs = iv.target.wrapping_sub(beta) & MASK;
            let g = gcd(alpha, 1 << 16);
            if alpha == 0 {
                if rhs == 0 {
                    for slot in out.iter_mut().filter(|s| s.is_none()) {
                        *slot = Some(pick.clone());
                    }
                }
                return;
            }
            if !rhs.is_multiple_of(g) {
                return;
            }
            let modulus = (1u32 << 16) / g;
            // g is the power of two dividing alpha, so alpha / g is odd.
            let x0 = ((rhs / g).wrapping_mul(inverse_odd(alpha / g))) % modulus;
            for k in 0..g {
                let x = (x0 + k * modulus) & MASK;
                if out[x as usize].is_none() {
                    out[x as usize] = Some(pick.clone());
                }
            }
            return;
        }
        for i in from..iv.slots.len() {
            pick[depth] = i;
            walk(iv, draws, depth + 1, i, pick, out);
        }
    }
    walk(iv, draws, 0, 0, &mut pick, &mut out);
    out
}

/// Every low half `x` of the first getter's state that explains `iv` with
/// the getters at `pick` (non-decreasing indices into `iv.slots`), under
/// `draws` draws per getter. `pick.len()` must be the interval's residual.
pub fn solve(iv: &Interval, pick: &[usize], draws: u32) -> Vec<u32> {
    if pick.len() != iv.residual as usize || pick.iter().any(|&p| p >= iv.slots.len()) {
        return Vec::new();
    }
    let (alpha, beta) = affine(iv, pick, draws);
    let rhs = iv.target.wrapping_sub(beta) & MASK;
    if alpha == 0 {
        return if rhs == 0 {
            (0..1u32 << 16).collect()
        } else {
            Vec::new()
        };
    }
    let g = gcd(alpha, 1 << 16);
    if !rhs.is_multiple_of(g) {
        return Vec::new();
    }
    let modulus = (1u32 << 16) / g;
    let x0 = ((rhs / g).wrapping_mul(inverse_odd(alpha / g))) % modulus;
    (0..g).map(|k| (x0 + k * modulus) & MASK).collect()
}

/// Every placement (up to `limit`) explaining `iv` when its first getter's
/// state has low half `x`.
pub fn placements(iv: &Interval, x: u32, draws: u32, limit: usize) -> Vec<Vec<Slot>> {
    let mut out = Vec::new();
    let mut pick = vec![0usize; iv.residual as usize];
    #[allow(clippy::too_many_arguments)]
    fn walk(
        iv: &Interval,
        x: u32,
        draws: u32,
        depth: usize,
        from: usize,
        pick: &mut Vec<usize>,
        out: &mut Vec<Vec<Slot>>,
        limit: usize,
    ) {
        if out.len() >= limit {
            return;
        }
        if depth == pick.len() {
            let (alpha, beta) = affine(iv, pick, draws);
            if (alpha.wrapping_mul(x).wrapping_add(beta)) & MASK == iv.target {
                out.push(pick.iter().map(|&p| iv.slots[p]).collect());
            }
            return;
        }
        for i in from..iv.slots.len() {
            pick[depth] = i;
            walk(iv, x, draws, depth + 1, i, pick, out, limit);
        }
    }
    walk(iv, x, draws, 0, 0, &mut pick, &mut out, limit);
    out
}

/// The getters of one interval for a surviving starting state.
#[derive(Clone, Debug, Serialize)]
pub struct Explained {
    pub from_tick: u32,
    pub to_tick: u32,
    pub getters: Vec<(Slot, u32)>,
}

/// Chain the first `intervals` intervals of a film under `draws` draws per
/// getter (and no unlogged draws). Returns every starting low half `x` that
/// explains all of them, with the getter slots and states.
pub fn search(film: &Film, intervals: usize, draws: u32) -> Vec<(u32, Vec<Explained>)> {
    let prepared: Vec<Interval> = prepare(film).into_iter().take(intervals).collect();
    let sols: Vec<Vec<Option<Vec<usize>>>> =
        prepared.iter().map(|iv| solutions(iv, draws)).collect();
    let mut out = Vec::new();
    'x: for x in 0..(1u32 << 16) {
        let mut state = x;
        let mut explained = Vec::new();
        for (iv, sol) in prepared.iter().zip(&sols) {
            let Some(pick) = &sol[state as usize] else {
                continue 'x;
            };
            let mut s = state;
            let mut getters = Vec::new();
            for &p in pick {
                getters.push((iv.slots[p], s));
                s = lcg16_n(s, draws);
            }
            explained.push(Explained {
                from_tick: iv.from_tick,
                to_tick: iv.to_tick,
                getters,
            });
            state = lcg16_n(state, iv.residual * draws);
        }
        out.push((x, explained));
    }
    out
}
