//! `Fast` against `Binary` and the native x87 fixtures.
//!
//! - every existing native fixture row (arithmetic and store, FSQRT, FPREM)
//!   through `Fast`;
//! - randomized differential runs against `Binary` for every operation and
//!   precision, with operands drawn from binary32/binary64/integer inputs,
//!   full 64-bit extended values over the whole exponent domain, the binary64
//!   and extended range edges, and constructed exact ties at 24/53/64 bits;
//! - an ignored benchmark (`cargo test --release -p aa-numeric --test fast --
//!   --ignored --nocapture`).
//!
//! `AA_FAST_CASES=<n>` sets the number of random operand pairs.
use aa_numeric::{Binary, Error, Fast, Precision, Result};
use serde::Deserialize;
use std::cmp::Ordering;

const PRECISIONS: [Precision; 3] = [Precision::Single24, Precision::Double53, Precision::Extended64];

fn same(f: &Result<Fast>, b: &Result<Binary>) -> bool {
    match (f, b) {
        (Ok(f), Ok(b)) => b.to_x87_extended_bits() == Some(f.to_x87_extended_bits()),
        (Err(x), Err(y)) => x == y,
        _ => false,
    }
}
fn pair_from_x87(bits: u128) -> Option<(Fast, Binary)> {
    match (Fast::from_x87_extended_bits(bits), Binary::from_x87_extended_bits(bits)) {
        (Ok(f), Ok(b)) => Some((f, b)),
        (Err(x), Err(y)) => {
            assert_eq!(x, y, "{bits:x}");
            None
        }
        other => panic!("constructors disagree for {bits:x}: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Native fixtures.

fn fixture_value(v: &serde_json::Value) -> (Fast, Binary) {
    let bits = || v["bits"].as_u64().unwrap();
    match v["kind"].as_str().unwrap() {
        "f32" => (Fast::from_f32_bits(bits() as u32).unwrap(), Binary::from_f32_bits(bits() as u32).unwrap()),
        "f64" => (Fast::from_f64_bits(bits()).unwrap(), Binary::from_f64_bits(bits()).unwrap()),
        "i32" => (Fast::from_i32(v["bits"].as_i64().unwrap() as i32), Binary::from_i32(v["bits"].as_i64().unwrap() as i32)),
        "u32" => (Fast::from_u32(bits() as u32), Binary::from_u32(bits() as u32)),
        _ => panic!("unknown fixture kind"),
    }
}

#[test]
fn native_arithmetic_and_store_fixture() {
    let report: serde_json::Value = serde_json::from_str(include_str!("../../../research/phase4-body/numeric-native-oracle.json")).unwrap();
    let rows = report["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 234);
    for row in rows {
        let mode: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let (a, _) = fixture_value(&row["values"][0]);
        let (b, _) = fixture_value(&row["values"][1]);
        let result = match row["operation"].as_str().unwrap() {
            "add" => a.add(&b, mode),
            "sub" => a.sub(&b, mode),
            "mul" => a.mul(&b, mode),
            "div" => a.div(&b, mode),
            "reciprocal_mul" => a.div(&b, mode).and_then(|v| v.mul(&fixture_value(&row["values"][2]).0, mode)),
            _ => panic!("unknown fixture operation"),
        }
        .unwrap()
        .store_f32();
        let expected = row["result"].as_u64().unwrap() as u32;
        if expected & 0x7f80_0000 == 0x7f80_0000 {
            assert_eq!(result, Err(Error::Binary32Overflow), "{row}");
        } else {
            assert_eq!(result.unwrap(), expected, "{row}");
        }
    }
}

#[derive(Deserialize)]
struct SqrtCase {
    precision: Precision,
    kind: String,
    inputs: Vec<u64>,
    result80: String,
    result32: u32,
}
#[derive(Deserialize)]
struct SqrtFixture {
    cases: Vec<SqrtCase>,
}
fn raw80(text: &str) -> u128 {
    let bytes: [u8; 10] = std::array::from_fn(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).unwrap());
    let mut wide = [0u8; 16];
    wide[..10].copy_from_slice(&bytes);
    u128::from_le_bytes(wide)
}
fn expected80(text: &str) -> Fast {
    let bits = raw80(text);
    // An all-zero significand with any exponent is a (signed) zero.
    Fast::from_x87_extended_bits(bits).unwrap()
}

#[test]
fn native_square_root_fixture() {
    let fixture: SqrtFixture = serde_json::from_str(include_str!("../../../research/phase5-numeric/sqrt-vectors.json")).unwrap();
    assert_eq!(fixture.cases.len(), 3267);
    for row in fixture.cases {
        let pc = row.precision;
        let input = match row.kind.as_str() {
            "f32" => Fast::from_f32_bits(row.inputs[0] as u32).unwrap(),
            "f64" => Fast::from_f64_bits(row.inputs[0]).unwrap(),
            "sum_squares_f32" => {
                let x = Fast::from_f32_bits(row.inputs[0] as u32).unwrap();
                let y = Fast::from_f32_bits(row.inputs[1] as u32).unwrap();
                x.mul(&x, pc).unwrap().add(&y.mul(&y, pc).unwrap(), pc).unwrap()
            }
            _ => panic!("unknown arithmetic fixture"),
        };
        let actual = input.sqrt(pc).unwrap();
        assert_eq!(actual.compare(&expected80(&row.result80)), Ordering::Equal, "{} {:?} {:?}", row.kind, row.inputs, pc);
        if row.result32 & 0x7fff_ffff == 0x7f80_0000 {
            assert_eq!(actual.store_f32(), Err(Error::Binary32Overflow));
        } else {
            assert_eq!(actual.store_f32().unwrap(), row.result32);
        }
    }
}

#[derive(Deserialize)]
struct RemRow {
    kind: String,
    inputs: [u64; 2],
    result80: String,
    result32: u32,
}
#[derive(Deserialize)]
struct RemFixture {
    cases: Vec<RemRow>,
}

#[test]
fn native_remainder_fixture() {
    let fixture: RemFixture = serde_json::from_str(include_str!("../../../research/phase5-numeric/remainder-hardware-vectors.json")).unwrap();
    assert_eq!(fixture.cases.len(), 3696);
    for row in fixture.cases {
        let input = |n: usize| if row.kind == "f32" { Fast::from_f32_bits(row.inputs[n] as u32).unwrap() } else { Fast::from_f64_bits(row.inputs[n]).unwrap() };
        let value = input(0).remainder_trunc(&input(1)).unwrap();
        assert_eq!(value.compare(&expected80(&row.result80)), Ordering::Equal, "{} {:?}", row.kind, row.inputs);
        if row.result32 & 0x7fff_ffff == 0x7f80_0000 {
            assert_eq!(value.store_f32(), Err(Error::Binary32Overflow));
        } else {
            assert_eq!(value.store_f32().unwrap(), row.result32);
        }
    }
}

// ---------------------------------------------------------------------------
// Randomized differential runs against Binary.

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + self.below((hi - lo + 1) as u64) as i64
    }
}

/// An 80-bit image: sign, top bit position `top`, a significand with only
/// its `bits` leading bits random.
fn x87_image(rng: &mut Rng, top: i64, bits: u32) -> u128 {
    let mut sig = rng.next() | (1 << 63);
    if bits < 64 {
        sig &= !((1u64 << (64 - bits)) - 1);
    }
    let sign = u128::from(rng.next() & 1 == 1) << 79;
    sign | (((top + 16383) as u128) << 64) | u128::from(sig)
}

fn operand(rng: &mut Rng) -> (Fast, Binary) {
    loop {
        let kind = rng.below(10);
        let made = match kind {
            0 => {
                let bits = rng.next() as u32;
                (bits & 0x7f80_0000 != 0x7f80_0000).then(|| (Fast::from_f32_bits(bits).unwrap(), Binary::from_f32_bits(bits).unwrap()))
            }
            1 => {
                let bits = rng.next();
                (bits & 0x7ff0_0000_0000_0000 != 0x7ff0_0000_0000_0000).then(|| (Fast::from_f64_bits(bits).unwrap(), Binary::from_f64_bits(bits).unwrap()))
            }
            2 => {
                let v = rng.next() as i32 >> rng.below(31);
                Some((Fast::from_i32(v), Binary::from_i32(v)))
            }
            // ordinary game magnitudes, binary64 precision
            3 => {
                let v = (rng.next() >> 11) as f64 / (1u64 << 53) as f64 * 2f64.powi(rng.range(-30, 30) as i32) * if rng.next() & 1 == 1 { -1.0 } else { 1.0 };
                Some((Fast::from_f64(v).unwrap(), Binary::from_f64_bits(v.to_bits()).unwrap()))
            }
            // full extended values over the whole domain
            4 => {
                let top = rng.range(-16382, 16383);
                pair_from_x87(x87_image(rng, top, 64))
            }
            // the binary64 range edges, 53 or 64 bits
            5 => {
                let top = if rng.next() & 1 == 1 { rng.range(-1110, -1000) } else { rng.range(990, 1100) };
                let bits = if rng.next() & 1 == 1 { 53 } else { 64 };
                pair_from_x87(x87_image(rng, top, bits))
            }
            // the extended range edges
            6 => {
                let top = if rng.next() & 1 == 1 { rng.range(-16382, -16250) } else { rng.range(16250, 16383) };
                pair_from_x87(x87_image(rng, top, 64))
            }
            // binary32 range edges
            7 => {
                let top = if rng.next() & 1 == 1 { rng.range(-160, -120) } else { rng.range(120, 135) };
                let bits = [24, 53, 64][rng.below(3) as usize];
                pair_from_x87(x87_image(rng, top, bits))
            }
            // signed zeros and small integers
            8 => {
                let v = rng.range(-4, 4) as i32;
                if v == 0 && rng.next() & 1 == 1 { Some((Fast::from_f32_bits(0x8000_0000).unwrap(), Binary::from_f32_bits(0x8000_0000).unwrap())) } else { Some((Fast::from_i32(v), Binary::from_i32(v))) }
            }
            // moderate exponents, 24/53/64 bits
            _ => {
                let top = rng.range(-80, 80);
                let bits = [24, 53, 64][rng.below(3) as usize];
                pair_from_x87(x87_image(rng, top, bits))
            }
        };
        if let Some(v) = made {
            return v;
        }
    }
}

/// (a, b) whose exact sum is a tie at `bits` significand bits: a has at most
/// `bits` bits, b is half an ulp of it (both signs, either order).
fn tie(rng: &mut Rng, bits: u32) -> ((Fast, Binary), (Fast, Binary)) {
    loop {
        let top = rng.range(-1200, 1200);
        let a = x87_image(rng, top, bits);
        let half_top = top - i64::from(bits);
        let sign = u128::from(rng.next() & 1 == 1) << 79;
        let b = sign | (((half_top + 16383) as u128) << 64) | (1u128 << 63);
        if let (Some(a), Some(b)) = (pair_from_x87(a), pair_from_x87(b)) {
            return if rng.next() & 1 == 1 { (a, b) } else { (b, a) };
        }
    }
}

fn check_pair(a: &(Fast, Binary), b: &(Fast, Binary), counts: &mut [u64; 4]) {
    let (fa, ba) = a;
    let (fb, bb) = b;
    for pc in PRECISIONS {
        let ops: [(&str, Result<Fast>, Result<Binary>); 4] = [
            ("add", fa.add(fb, pc), ba.add(bb, pc)),
            ("sub", fa.sub(fb, pc), ba.sub(bb, pc)),
            ("mul", fa.mul(fb, pc), ba.mul(bb, pc)),
            ("div", fa.div(fb, pc), ba.div(bb, pc)),
        ];
        for (name, f, b) in &ops {
            assert!(same(f, b), "{name} {pc:?}: {:x} {:x} -> fast {f:?} binary {:?}", fa.to_x87_extended_bits(), fb.to_x87_extended_bits(), b.as_ref().map(|v| v.to_x87_extended_bits()));
            if let (Ok(f), Ok(b)) = (f, b) {
                assert_eq!(f.store_f32(), b.store_f32(), "{name} {pc:?} store");
                assert_eq!(f.truncate_i32(), b.truncate_i32(), "{name} {pc:?} truncate");
                counts[0] += 1;
            } else {
                counts[1] += 1;
            }
        }
        let (fs, bs) = (fa.sqrt(pc), ba.sqrt(pc));
        assert!(same(&fs, &bs), "sqrt {pc:?}: {:x} -> {fs:?}", fa.to_x87_extended_bits());
    }
    assert!(same(&fa.remainder_trunc(fb), &ba.remainder_trunc(bb)), "rem {:x} {:x}", fa.to_x87_extended_bits(), fb.to_x87_extended_bits());
    assert_eq!(fa.compare(fb), ba.compare(bb), "compare");
    assert_eq!(fa.store_f32(), ba.store_f32(), "store");
    assert_eq!(fa.truncate_i32(), ba.truncate_i32(), "truncate");
    assert_eq!(Fast::from_binary(ba), Some(*fa), "conversion");
    assert!(fa.to_binary().compare(ba).is_eq() && fa.to_binary().to_x87_extended_bits() == ba.to_x87_extended_bits());
    counts[2] += 1;
}

fn cases() -> u64 {
    std::env::var("AA_FAST_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(if cfg!(debug_assertions) { 20_000 } else { 200_000 })
}

#[test]
fn random_operands_match_binary_in_every_precision() {
    let mut rng = Rng(0x5eed_aa00_0000_0001);
    let mut counts = [0u64; 4];
    let n = cases();
    for _ in 0..n {
        let a = operand(&mut rng);
        let b = operand(&mut rng);
        check_pair(&a, &b, &mut counts);
    }
    eprintln!("{} pairs: {} rounded results, {} errors", counts[2], counts[0], counts[1]);
}

#[test]
fn exact_ties_round_to_even_like_binary() {
    let mut rng = Rng(0x7135_0000_0000_0002);
    let mut counts = [0u64; 4];
    for _ in 0..cases() / 4 {
        for bits in [24, 53, 64] {
            let (a, b) = tie(&mut rng, bits);
            check_pair(&a, &b, &mut counts);
        }
    }
    eprintln!("{} tie pairs", counts[2]);
}

#[test]
fn integer_path_alone_matches_binary_where_the_host_path_would_run() {
    // The host shortcut handles normal binary64/binary32 operands at PC53/PC24;
    // force the integer path on the same operands so each path is proven by
    // itself, and check the two agree.
    let mut rng = Rng(0x1a7e_0000_0000_0004);
    for _ in 0..cases() {
        let pick = |rng: &mut Rng| loop {
            let v = if rng.next() & 1 == 1 { f64::from_bits(rng.next()) } else { f64::from(f32::from_bits(rng.next() as u32)) };
            if v.is_finite() {
                return (Fast::from_f64(v).unwrap(), Binary::from_f64_bits(v.to_bits()).unwrap());
            }
        };
        let (fa, ba) = pick(&mut rng);
        let (fb, bb) = pick(&mut rng);
        for pc in PRECISIONS {
            for op in ['+', '-', '*', '/', 'r'] {
                let exact = fa.integer_path(&fb, op, pc);
                let host = match op {
                    '+' => fa.add(&fb, pc),
                    '-' => fa.sub(&fb, pc),
                    '*' => fa.mul(&fb, pc),
                    '/' => fa.div(&fb, pc),
                    _ => fa.sqrt(pc),
                };
                let binary = match op {
                    '+' => ba.add(&bb, pc),
                    '-' => ba.sub(&bb, pc),
                    '*' => ba.mul(&bb, pc),
                    '/' => ba.div(&bb, pc),
                    _ => ba.sqrt(pc),
                };
                assert!(same(&exact, &binary), "integer {op} {pc:?} {:x} {:x}: {exact:?}", fa.to_x87_extended_bits(), fb.to_x87_extended_bits());
                assert!(same(&host, &binary), "host {op} {pc:?}");
            }
        }
    }
}

#[test]
fn results_chain_like_binary() {
    // Long chains keep 64-bit (PC64) and 53-bit (PC53) results flowing into
    // later operations, as native expression sequences do.
    let mut rng = Rng(0xc4a1_0000_0000_0003);
    for _ in 0..(cases() / 100).max(50) {
        let pc = PRECISIONS[rng.below(3) as usize];
        let (mut f, mut b) = operand(&mut rng);
        for _ in 0..40 {
            let (g, c) = operand(&mut rng);
            let (fr, br) = match rng.below(5) {
                0 => (f.add(&g, pc), b.add(&c, pc)),
                1 => (f.sub(&g, pc), b.sub(&c, pc)),
                2 => (f.mul(&g, pc), b.mul(&c, pc)),
                3 => (f.div(&g, pc), b.div(&c, pc)),
                _ => (f.sqrt(pc), b.sqrt(pc)),
            };
            assert!(same(&fr, &br), "chain {pc:?}: {fr:?}");
            if let (Ok(fr), Ok(br)) = (fr, br) {
                f = fr;
                b = br;
            }
        }
    }
}

#[test]
fn edge_values() {
    let one = Fast::from_u32(1);
    let pc = Precision::Double53;
    // signed zeros
    let nz = Fast::from_f32_bits(0x8000_0000).unwrap();
    assert_eq!(nz.add(&nz, pc).unwrap().store_f32().unwrap(), 0x8000_0000);
    assert_eq!(nz.add(&Fast::zero(false), pc).unwrap().store_f32().unwrap(), 0);
    assert_eq!(one.sub(&one, pc).unwrap().store_f32().unwrap(), 0);
    assert_eq!(nz.sqrt(pc).unwrap().store_f32().unwrap(), 0x8000_0000);
    assert_eq!(one.div(&Fast::zero(false), pc), Err(Error::DivisionByZero));
    assert_eq!(Fast::from_i32(-1).sqrt(pc), Err(Error::NegativeSquareRoot));
    // binary64 overflow stays finite in the extended domain at PC53
    let max = Fast::from_f64(f64::MAX).unwrap();
    let big = max.add(&max, pc).unwrap();
    assert!(big.to_f64_exact().is_none());
    assert_eq!(big.store_f32(), Err(Error::Binary32Overflow));
    assert_eq!(big.to_binary().compare(&Binary::from_f64_bits(f64::MAX.to_bits()).unwrap().add(&Binary::from_f64_bits(f64::MAX.to_bits()).unwrap(), pc).unwrap()), Ordering::Equal);
    // binary64 underflow stays normal at PC53
    let tiny = Fast::from_f64(f64::MIN_POSITIVE).unwrap();
    let t2 = tiny.mul(&tiny, pc).unwrap();
    assert!(!t2.is_zero() && t2.to_f64_exact().is_none());
    assert_eq!(t2.store_f32().unwrap(), 0);
    // binary32 stores: the 2^-149 boundary, ties, overflow by rounding; each
    // also against Binary
    let store = |v: f64| {
        let f = Fast::from_f64(v).unwrap().store_f32();
        assert_eq!(f, Binary::from_f64_bits(v.to_bits()).unwrap().store_f32(), "{v:e}");
        f
    };
    let max = f64::from(f32::MAX);
    assert_eq!(store(max).unwrap(), f32::MAX.to_bits());
    // below half an ulp: f32::MAX; exactly half (odd significand): overflow
    assert_eq!(store(max * (1.0 + 2f64.powi(-25))).unwrap(), f32::MAX.to_bits());
    assert_eq!(store(max + 2f64.powi(103)), Err(Error::Binary32Overflow));
    assert_eq!(store(2f64.powi(-150)).unwrap(), 0);
    assert_eq!(store(2f64.powi(-150) * 1.5).unwrap(), 1);
    assert_eq!(store(-(2f64.powi(-150) * 3.0)).unwrap(), 0x8000_0002);
    assert_eq!(store(2f64.powi(-126) * (1.0 - 2f64.powi(-25))).unwrap(), 0x0080_0000);
}

// ---------------------------------------------------------------------------
// Benchmark.

#[test]
#[ignore]
fn benchmark_fast_against_binary() {
    use std::hint::black_box;
    use std::time::Instant;
    let mut rng = Rng(42);
    // typical movement/combat magnitudes: binary32 inputs
    let values: Vec<(Fast, Binary)> = (0..1024)
        .map(|_| {
            let v = ((rng.next() >> 40) as f32 / (1u64 << 24) as f32 + 0.01) * 300.0;
            (Fast::from_f32(v).unwrap(), Binary::from_f32_bits(v.to_bits()).unwrap())
        })
        .collect();
    let n = 4_000_000usize;
    type BinOp = fn(&Binary, &Binary, Precision) -> Result<Binary>;
    let ops: [(&str, BinOp); 5] = [
        ("add", |a, b, p| a.add(b, p)),
        ("mul", |a, b, p| a.mul(b, p)),
        ("div", |a, b, p| a.div(b, p)),
        ("sqrt", |a, _, p| a.sqrt(p)),
        ("none", |a, _, _| Ok(a.clone())),
    ];
    // generic, so the call inlines as it does in game code
    #[inline(never)]
    fn run_fast<F: Fn(&Fast, &Fast) -> Result<Fast>>(values: &[(Fast, Binary)], n: usize, op: F) -> (f64, u32) {
        let t = Instant::now();
        let mut acc = 0u32;
        for i in 0..n {
            let (a, b) = (black_box(&values[i & 1023].0), black_box(&values[(i * 7 + 3) & 1023].0));
            acc ^= op(a, b).unwrap().store_f32().unwrap();
        }
        (t.elapsed().as_nanos() as f64 / n as f64, acc)
    }
    let fast_run = |name: &str, pc: Precision| -> (f64, u32) {
        match name {
            "add" => run_fast(&values, n, |a, b| a.add(b, pc)),
            "mul" => run_fast(&values, n, |a, b| a.mul(b, pc)),
            "div" => run_fast(&values, n, |a, b| a.div(b, pc)),
            "sqrt" => run_fast(&values, n, |a, _| a.sqrt(pc)),
            _ => run_fast(&values, n, |a, _| Ok(*a)),
        }
    };
    // the same loop with host binary64 arithmetic, for scale
    let host: Vec<(f64, f64)> = (0..1024).map(|i| (values[i].0.to_f64_exact().unwrap(), values[(i * 7 + 3) & 1023].0.to_f64_exact().unwrap())).collect();
    let t = Instant::now();
    let mut acc = 0u32;
    for i in 0..n {
        let (a, b) = host[i & 1023];
        acc ^= (black_box(a) * black_box(b)) as f32 as u32;
    }
    eprintln!("host f64 mul+store loop: {:.1} ns [{acc:x}]", t.elapsed().as_nanos() as f64 / n as f64);
    for pc in [Precision::Double53, Precision::Extended64] {
        for (name, bin_op) in ops {
            let (fast, acc) = fast_run(name, pc);
            let m = n / 20;
            let t = Instant::now();
            let mut acc2 = 0u32;
            for i in 0..m {
                let (a, b) = (&values[i & 1023].1, &values[(i * 7 + 3) & 1023].1);
                acc2 ^= black_box(bin_op(a, b, pc)).unwrap().store_f32().unwrap();
            }
            let slow = t.elapsed().as_nanos() as f64 / m as f64;
            eprintln!("{pc:?} {name:4}+store: Fast {fast:6.1} ns  Binary {slow:7.1} ns  ({:.0}x) [{acc:x} {acc2:x}]", slow / fast);
        }
    }
}
