//! The PC53 binary64 fast path (`fpu::pc53` and the routines that use it)
//! against the Binary x87 model at `Precision::Double53`: the primitive
//! operations over the whole finite binary32 domain, and the zone routines
//! over map-scale inputs including exact circle boundaries. The PC53 cases
//! of the native fixtures replay the fast path against AA.exe itself.
use aa_numeric::{Binary, Precision};
use aa_zones::fpu::{self, pc53};
use aa_zones::solver::{self, Circle};
use aa_zones::tile::{MapGeometry, ZoneMap};
use aa_zones::zone;
use std::cmp::Ordering;

const P: Precision = Precision::Double53;

/// xorshift64*: deterministic inputs without extra dependencies.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn bits(&mut self) -> u32 {
        // every class: zeros, subnormals, normals, the extremes, non-finite
        match self.next() % 16 {
            0 => 0,
            1 => 0x8000_0000,
            2 => (self.next() as u32) & 0x807f_ffff,
            3 => 0x7f7f_ffff ^ ((self.next() as u32) & 0x8000_000f),
            4 => 0x7f80_0000 | (self.next() as u32 & 0x8000_0001),
            _ => self.next() as u32,
        }
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * ((self.next() >> 40) as f32 / (1u64 << 24) as f32)
    }
}

fn b(bits: u32) -> Option<Binary> {
    fpu::load(bits).ok()
}

fn same(fast: f64, slow: &Binary) -> bool {
    Binary::from_f64_bits(fast.to_bits()).is_ok_and(|v| v.compare(slow) == Ordering::Equal)
        && (fast == 0.0 || fast.is_sign_negative() == slow.is_negative_nonzero())
}

/// Each primitive either declines (None) or returns exactly the Binary
/// result; on the map-scale domain it never declines.
#[test]
fn primitives_match_the_binary_model() {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let (mut fast, mut declined) = (0u32, 0u32);
    for _ in 0..200_000 {
        let (x, y) = (r.bits(), r.bits());
        let (Some(bx), Some(by)) = (b(x), b(y)) else {
            assert!(pc53::load(x).is_none() || pc53::load(y).is_none());
            continue;
        };
        let (fx, fy) = (pc53::load(x).unwrap(), pc53::load(y).unwrap());
        type Op = fn(f64, f64) -> Option<f64>;
        let ops: [(Op, Result<Binary, aa_numeric::Error>); 4] = [
            (pc53::add as Op, bx.add(&by, P)),
            (pc53::sub as Op, bx.sub(&by, P)),
            (pc53::mul as Op, bx.mul(&by, P)),
            (pc53::div as Op, bx.div(&by, P)),
        ];
        for (i, (op, slow)) in ops.into_iter().enumerate() {
            match op(fx, fy) {
                Some(v) => {
                    let slow = slow.unwrap_or_else(|e| panic!("op {i} {x:#x} {y:#x}: fast {v} but Binary {e}"));
                    assert!(same(v, &slow), "op {i} {x:#x} {y:#x}: {v:e} vs {slow:?}");
                    // the stored binary32 agrees too
                    if let Ok(s) = slow.store_f32() {
                        assert_eq!(pc53::store(v), Some(s), "store op {i} {x:#x} {y:#x}");
                    }
                    fast += 1;
                }
                None => declined += 1,
            }
        }
        if let Some(v) = pc53::fmod(fx, fy) {
            let slow = bx.remainder_trunc(&by).unwrap();
            assert!(same(v, &slow), "fmod {x:#x} {y:#x}");
        }
        if let Some(t) = pc53::ftol(fx) {
            assert_eq!(bx.truncate_i32().ok(), Some(t), "ftol {x:#x}");
        } else {
            assert!(bx.truncate_i32().is_err(), "ftol {x:#x}");
        }
        assert_eq!(fpu::compare_bits(x, y).unwrap(), bx.compare(&by), "compare {x:#x} {y:#x}");
    }
    assert!(fast > 400_000, "{fast} fast, {declined} declined");
    // map-scale operands never decline
    for _ in 0..200_000 {
        let x = r.range(-5000.0, 5000.0).to_bits();
        let y = r.range(0.25, 600.0).to_bits();
        let (fx, fy) = (pc53::load(x).unwrap(), pc53::load(y).unwrap());
        for v in [pc53::add(fx, fy), pc53::sub(fx, fy), pc53::mul(fx, fy), pc53::div(fx, fy), pc53::fmod(fx, fy)] {
            assert!(v.is_some(), "{x:#x} {y:#x}");
        }
    }
}

fn geometry(width: i32, height: i32, tile: f32) -> MapGeometry {
    MapGeometry {
        shift: (tile as u32).max(1).trailing_zeros(),
        tile_size_bits: tile.to_bits(),
        extent_bits: [(width as f32 * tile).to_bits(), (height as f32 * tile).to_bits()],
        width,
        height,
    }
}

/// `0x53FE90`, the `0x540020` plan, `0x53F650`/`0x53F6C0` and `0x552640`
/// with random map-scale inputs: identical results, and every tile test of
/// the plan's circles agrees (exact boundary radii included).
#[test]
fn zone_routines_match_the_binary_model() {
    let mut r = Rng(0xD1B5_4A32_D192_ED03);
    let mut boundary = 0;
    for case in 0..3_000 {
        let tile = [1.0f32, 2.0, 4.0, 8.0, 16.0][case % 5];
        let (w, h) = (64 + (r.next() % 64) as i32, 64 + (r.next() % 64) as i32);
        let map = ZoneMap::new(geometry(w, h, tile)).unwrap();
        // radii: random, whole / quarter tiles, exact distances to tile
        // centres (the `<=` boundary), and the inactive -1
        let radius = match case % 6 {
            0 => -1.0f32,
            1 => (r.next() % 40) as f32 * tile * 0.25,
            2 => {
                let (dx, dy) = ((r.next() % 12) as f32, (r.next() % 12) as f32);
                ((dx * dx + dy * dy) * tile * tile).sqrt()
            }
            _ => r.range(0.0, 60.0 * tile),
        };
        let old_r = if case % 7 == 0 { -1.0f32 } else { r.range(0.0, 60.0 * tile) };
        let rb = radius.to_bits();
        let slow = solver::adjust_radius(&map, rb, P).unwrap();
        let fast = solver::adjust_radius_pc53(&map, rb).expect("map-scale radius");
        assert!(same(fast, &slow), "adjust {rb:#x} tile {tile}");
        let (x, y) = ((r.next() % w as u64) as i32, (r.next() % h as u64) as i32);
        let (ox, oy) = (x + (r.next() % 3) as i32 - 1, y + (r.next() % 3) as i32 - 1);
        let ps = solver::move_plan(&map, ox, oy, old_r.to_bits(), x, y, rb, P).unwrap();
        let pf = solver::move_plan_pc53(&map, ox, oy, old_r.to_bits(), x, y, rb).expect("map-scale plan");
        assert!(matches!(pf.new, Circle::Pc53 { .. }));
        assert_eq!((pf.rn_tiles, pf.ro_tiles, pf.add), (ps.rn_tiles, ps.ro_tiles, ps.add), "plan {case}");
        assert_eq!(solver::circle_bits(&pf.new).unwrap(), solver::circle_bits(&ps.new).unwrap());
        assert_eq!(solver::circle_bits(&pf.old).unwrap(), solver::circle_bits(&ps.old).unwrap());
        let reach = 2 + (60.0 * tile / tile) as i32;
        for _ in 0..64 {
            let tx = x + (r.next() % (2 * reach as u64 + 1)) as i32 - reach;
            let ty = y + (r.next() % (2 * reach as u64 + 1)) as i32 - reach;
            for (cf, cs) in [(&pf.new, &ps.new), (&pf.old, &ps.old)] {
                let a = solver::in_circle(&map, cf, tx, ty, P).unwrap();
                let s = solver::in_circle(&map, cs, tx, ty, P).unwrap();
                assert_eq!(a, s, "circle case {case} tile ({tx}, {ty})");
            }
        }
        if case % 6 == 2 {
            boundary += 1;
        }
        // positions around and past the clamp bounds (and on them)
        let pos = match case % 4 {
            0 => [tile.to_bits(), (h as f32 * tile - tile - 1.0e-4).to_bits()],
            _ => [r.range(-20.0, w as f32 * tile + 20.0).to_bits(), r.range(-20.0, h as f32 * tile + 20.0).to_bits()],
        };
        assert_eq!(zone::clamp_position_pc53(&map, pos), Some(zone::clamp_position_exact(&map, pos, P).unwrap()), "clamp {pos:?}");
        for rt in [ps.rn_tiles, ps.ro_tiles] {
            assert_eq!(solver::circle_spans(x, y, rt, P).unwrap(), solver::circle_spans_exact(x, y, rt, P).unwrap(), "spans {rt:#x}");
        }
    }
    assert!(boundary > 400);
}

/// A circle radius exactly on a tile-centre distance stays inside (`<=`) on
/// both paths, and a radius one ulp below drops the tile on both.
#[test]
fn exact_boundary_ties() {
    let map = ZoneMap::new(geometry(64, 64, 4.0)).unwrap();
    for (dx, dy) in [(3, 4), (5, 12), (0, 7), (6, 8)] {
        let d = (((dx * dx + dy * dy) * 16) as f32).sqrt();
        for r in [d, f32::from_bits(d.to_bits() - 1), f32::from_bits(d.to_bits() + 1)] {
            let ps = solver::move_plan(&map, 20, 20, aa_zones::zone::INACTIVE, 20, 20, r.to_bits(), P).unwrap();
            let pf = solver::move_plan_pc53(&map, 20, 20, aa_zones::zone::INACTIVE, 20, 20, r.to_bits()).unwrap();
            for (x, y) in [(20 + dx, 20 + dy), (20 - dx, 20 + dy), (20 + dy, 20 - dx)] {
                assert_eq!(solver::in_circle(&map, &pf.new, x, y, P).unwrap(), solver::in_circle(&map, &ps.new, x, y, P).unwrap(), "r {r} ({x}, {y})");
            }
        }
    }
}
