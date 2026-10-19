//! Native TerrainMap ray query 0x71D840 (AA.exe, image base 0x400000) and
//! every child it reaches, ported instruction by instruction with the
//! original x87 order, precision-control rounding and binary32 stores.
//!
//! `thiscall 0x71D840(eye*, dirX, dirY, dirZ, point*, secondary*, margin)`,
//! `ret 0x1C`. Reached children:
//!
//! * 0x42D750 normalize (reused: `native_camera::ni::normalize`),
//!   0x869C60/0x869C30 NiPlane constructors, 0x42ED60 plane intersection,
//! * 0x71C930 margin-rectangle clip with 0x53A3A0 segment parameters
//!   (reused: `aa_navigation::low_path::segment_parameter_53a3a0`),
//!   0x71AB10 clamp, 0x53A7D0 containment, 0x42EE20/0x42EE70 block column/row
//!   (and their inline copies at 0x71DDE2..0x71DE71) with 0x860680 binary32
//!   truncation, 0x71ADC0 block at point, CRT `_CIfmod` 0x96DB7A,
//! * 0x5E8910 block-edge crossing, 0x71D570 block query:
//!   0x71A320 side plane gate, 0x71B9B0 mesh triangle walk (0x7151C0/0x436310
//!   triangle plane, 0x875610 containment) with the optional water child
//!   0x736E70/0x736C60, and the absent-mesh 0x71CD30 flat-block fallback.
//!
//! Every value is read through [`Terrain`] / [`TerrainBlock`]; a missing
//! datum is an error, never a default. Native paths that read undefined
//! memory (parallel ray against the TerrainMap up plane: 0x71D8BD reads the
//! unwritten 0x42ED60 locals; 0x71CD30 clamps an unwritten caller point) are
//! errors. The returned AL is native data only: 0x43C4B0 ignores it.
use aa_navigation::low_path::segment_parameter_53a3a0;
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

type Result<T> = std::result::Result<T, String>;

const ZERO: u32 = 0; // 0x9F32DC
const ONE: u32 = 0x3f80_0000; // 0x9F32E0
const QUARTER: u32 = 0x3e80_0000; // 0xA033BC
const HALF: u32 = 0x3f00_0000; // 0x9FBDB8
const RECT_INSET: u32 = 0x3c23_d70a; // 0xA03078 (0.01)
/// 0x71C930 function-static 0xB40420, initialized once to 0x38D1B717 under
/// guard bit 0xB40424 & 1 before its first use; constant thereafter.
const CLIP_EPSILON: u32 = 0x38d1_b717;
const CONTAIN_EPSILON: u32 = 0x38d1_b717; // 0xAB7800 (.rdata)
const FLAT_EPSILON: u32 = 0x38d1_b717; // 0xA88AC8 (.rdata)
const UNIT_THRESHOLD: u32 = 0x3586_37bd; // 0x9FD38C
const TWO_POW_31: u32 = 0x4f00_0000; // 0x9FD478
const MINUS_TWO_POW_31: u32 = 0xcf00_0000; // 0x9FD474

/// The TerrainMap (`this` of 0x71D840). Each method replaces one native read.
pub trait Terrain {
    type Block: TerrainBlock;
    /// +0x04: the up vector (three binary32 words). 0x71D840 builds its
    /// max/min height planes from it (0x869C60).
    fn up_normal(&self) -> Result<[u32; 3]>;
    /// +0x74: map width in world units (signed 32-bit).
    fn width(&self) -> Result<i32>;
    /// +0x78: map height in world units (signed 32-bit).
    fn height(&self) -> Result<i32>;
    /// +0x84: block columns (0x42EE20 clamp, cell-array stride).
    fn blocks_x(&self) -> Result<i32>;
    /// +0x88: block rows (0x42EE70 clamp).
    fn blocks_y(&self) -> Result<i32>;
    /// +0x8C: minimum terrain height (binary32 word).
    fn min_height(&self) -> Result<u32>;
    /// +0x90: maximum terrain height (binary32 word).
    fn max_height(&self) -> Result<u32>;
    /// +0xA0: block size in world units (signed 32-bit, idiv divisor).
    fn block_size(&self) -> Result<i32>;
    /// +0xBC: loaded byte (nonzero: walk the blocks).
    fn loaded(&self) -> Result<u8>;
    /// `[+0xB8][cell]`: the block pointer at `blocks_x * row + col` (32-bit
    /// wrapping arithmetic). A cell outside the native array must be an error.
    fn block(&self, cell: i32) -> Result<&Self::Block>;
}

/// One terrain block (cell pointer read from TerrainMap+0xB8).
pub trait TerrainBlock {
    /// `[+0x40]+0x04`: up vector of the block's owner TerrainMap (0x71CD30).
    fn owner_up_normal(&self) -> Result<[u32; 3]>;
    /// `[+0x40]+0x10`: owner X side normal (0x71A320 sides 2/3).
    fn owner_x_normal(&self) -> Result<[u32; 3]>;
    /// `[+0x40]+0x1C`: owner Y side normal (0x71A320 sides 0/1).
    fn owner_y_normal(&self) -> Result<[u32; 3]>;
    /// +0x44 nonzero: the block has its Ni mesh (0x71D570 chooses 0x71B9B0;
    /// otherwise 0x71CD30).
    fn has_mesh(&self) -> Result<bool>;
    /// `[[[+0x44]+0xCC]+0x20] + 12*index`: mesh vertex (three binary32 words).
    fn mesh_vertex(&self, index: i32) -> Result<[u32; 3]>;
    /// +0x80: block minimum height (binary32 word).
    fn min_height(&self) -> Result<u32>;
    /// +0x84: block maximum height (binary32 word).
    fn max_height(&self) -> Result<u32>;
    /// +0x9C: water byte (nonzero: 0x71B9B0 first queries 0x736E70).
    fn water(&self) -> Result<u8>;
    /// `D+0x40` with `D = [[[[+0x90]]+4]+0xCC]`: the water index count
    /// (0x736C60 walks `count / 3` triangles).
    fn water_index_count(&self) -> Result<u32>;
    /// `D+0x44 + 6*triangle`: three u16 water vertex indices.
    fn water_triangle(&self, triangle: u32) -> Result<[u16; 3]>;
    /// `D+0x20 + 12*index`: water vertex (three binary32 words).
    fn water_vertex(&self, index: u16) -> Result<[u32; 3]>;
    /// +0xAC: block column index (signed 32-bit).
    fn index_x(&self) -> Result<i32>;
    /// +0xB0: block row index (signed 32-bit).
    fn index_y(&self) -> Result<i32>;
    /// +0xC8: mesh vertex row stride.
    fn vertex_stride(&self) -> Result<i32>;
    /// +0xCC: quads per block side.
    fn quads(&self) -> Result<i32>;
    /// +0xD4: quad size in world units.
    fn quad_size(&self) -> Result<i32>;
}

/// Destination of one native store through the caller's pointers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// The fifth argument (primary point).
    Point,
    /// The optional sixth argument: 0x71B9B0's land hit point.
    Secondary,
}
/// One 32-bit store, in native order (`offset` is 0, 4 or 8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Write {
    pub target: Target,
    pub offset: u32,
    pub value: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Native AL (0x71D840 returns exactly 0 or 1). Callers ignore it.
    pub al: bool,
    /// Primary point words as last written; `None` means never written.
    pub point: [Option<u32>; 3],
    /// Every store to the primary and secondary points, in native order.
    pub writes: Vec<Write>,
}
impl Outcome {
    /// The fully written primary point; an unwritten word is an error.
    pub fn point_words(&self) -> Result<[u32; 3]> {
        let mut words = [0; 3];
        for (word, value) in words.iter_mut().zip(self.point) {
            *word = value.ok_or("71D840 left a primary point word unwritten")?;
        }
        Ok(words)
    }
    /// Stores the written words into a caller point buffer, leaving the
    /// unwritten ones untouched (0x59A6E0 passes its own point buffer).
    pub fn apply_to(&self, point: &mut [u32; 3]) {
        for (word, value) in point.iter_mut().zip(self.point) {
            if let Some(value) = value {
                *word = value;
            }
        }
    }
}

/// 0x71A320 side codes chosen by 0x71D840 (0x71DB8F..0x71DC0E).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// 0: Y normal, constant `(row+1) * quad_size * quads`.
    YHigh = 0,
    /// 1: Y normal, constant `row * quad_size * quads`.
    YLow = 1,
    /// 2: X normal, constant `column * quad_size * quads`.
    XLow = 2,
    /// 3: X normal, constant `(column+1) * quad_size * quads`.
    XHigh = 3,
}

struct Out<'a> {
    point: [Option<u32>; 3],
    secondary: Option<&'a mut [u32; 3]>,
    writes: Vec<Write>,
}
impl Out<'_> {
    fn point(&mut self, word: usize, value: u32) {
        self.point[word] = Some(value);
        self.writes.push(Write { target: Target::Point, offset: 4 * word as u32, value });
    }
    fn point3(&mut self, value: [u32; 3]) {
        for (word, value) in value.into_iter().enumerate() {
            self.point(word, value);
        }
    }
    fn read_point(&self, word: usize) -> Result<u32> {
        self.point[word].ok_or_else(|| "71CD30 reads an unwritten caller point word".to_string())
    }
    /// 0x71BB5B/0x71BBB2: stores only through a non-null secondary pointer.
    fn secondary3(&mut self, value: [u32; 3]) {
        if let Some(secondary) = self.secondary.as_mut() {
            for (word, value) in value.into_iter().enumerate() {
                secondary[word] = value;
                self.writes.push(Write { target: Target::Secondary, offset: 4 * word as u32, value });
            }
        }
    }
    fn finish(self, al: bool) -> Outcome {
        Outcome { al, point: self.point, writes: self.writes }
    }
}

fn b(word: u32) -> Result<Binary> {
    Binary::from_f32_bits(word).map_err(|e| format!("71D840 binary32 {word:#010x}: {e}"))
}
fn st(value: &Binary) -> Result<u32> {
    value.store_f32().map_err(|e| format!("71D840 binary32 store: {e}"))
}
fn add(a: &Binary, c: &Binary, p: Precision) -> Result<Binary> {
    a.add(c, p).map_err(|e| e.to_string())
}
fn sub(a: &Binary, c: &Binary, p: Precision) -> Result<Binary> {
    a.sub(c, p).map_err(|e| e.to_string())
}
fn mul(a: &Binary, c: &Binary, p: Precision) -> Result<Binary> {
    a.mul(c, p).map_err(|e| e.to_string())
}
fn div(a: &Binary, c: &Binary, p: Precision) -> Result<Binary> {
    a.div(c, p).map_err(|e| e.to_string())
}
fn less(a: &Binary, c: &Binary) -> bool {
    a.compare(c) == Ordering::Less
}
fn greater(a: &Binary, c: &Binary) -> bool {
    a.compare(c) == Ordering::Greater
}
/// FABS. Only compared afterwards, so the sign of zero is immaterial.
fn abs(value: Binary) -> Binary {
    if value.is_negative_nonzero() { value.negated() } else { value }
}
/// `cdq; idiv`: divide error (#DE) for a zero divisor or INT_MIN / -1.
fn idiv(value: i32, divisor: i32) -> Result<i32> {
    value
        .checked_div(divisor)
        .ok_or_else(|| format!("71D840 block index idiv {value} / {divisor} raises #DE"))
}

/// NiPlane: normal words and constant word (0x869C60 copies them unchanged).
#[derive(Clone, Copy)]
pub(crate) struct Plane {
    pub(crate) normal: [u32; 3],
    pub(crate) constant: u32,
}

/// 0x42ED60(origin*, direction*, out*, accept_behind). `None` is AL=0 with
/// no store: a zero normal/direction dot, or t < 0 without the flag.
pub(crate) fn plane_42ed60(
    plane: &Plane,
    origin: [u32; 3],
    direction: [u32; 3],
    accept_behind: bool,
    p: Precision,
) -> Result<Option<[u32; 3]>> {
    let n = [b(plane.normal[0])?, b(plane.normal[1])?, b(plane.normal[2])?];
    let d = [b(direction[0])?, b(direction[1])?, b(direction[2])?];
    let dot = add(&add(&mul(&d[0], &n[0], p)?, &mul(&n[2], &d[2], p)?, p)?, &mul(&d[1], &n[1], p)?, p)?;
    if dot.is_zero() {
        return Ok(None);
    }
    let o = [b(origin[0])?, b(origin[1])?, b(origin[2])?];
    let along = add(&add(&mul(&o[0], &n[0], p)?, &mul(&o[1], &n[1], p)?, p)?, &mul(&n[2], &o[2], p)?, p)?;
    let t = div(&sub(&b(plane.constant)?, &along, p)?, &dot, p)?;
    if less(&t, &b(ZERO)?) && !accept_behind {
        return Ok(None);
    }
    let z = st(&mul(&t, &d[2], p)?)?;
    let x = st(&add(&mul(&t, &d[0], p)?, &o[0], p)?)?;
    let y = st(&add(&mul(&t, &d[1], p)?, &o[1], p)?)?;
    Ok(Some([x, y, st(&add(&b(z)?, &o[2], p)?)?]))
}

/// 0x71C930/0x53A7D0 containment: min <= v <= max on both axes.
fn inside_53a7d0(rect: &[u32; 4], point: [u32; 2]) -> Result<bool> {
    let (x, y) = (b(point[0])?, b(point[1])?);
    Ok(!less(&x, &b(rect[0])?) && !greater(&x, &b(rect[2])?) && !less(&y, &b(rect[1])?) && !greater(&y, &b(rect[3])?))
}

/// 0x71AB10: clamp to [min, max]; a store happens only past a bound.
fn clamp_71ab10(rect: &[u32; 4], point: &mut [u32; 2]) -> Result<()> {
    for axis in 0..2 {
        let v = b(point[axis])?;
        if less(&v, &b(rect[axis])?) {
            point[axis] = rect[axis];
        } else if greater(&v, &b(rect[axis + 2])?) {
            point[axis] = rect[axis + 2];
        }
    }
    Ok(())
}

fn segment_53a3a0(a: [u32; 2], c: [u32; 2], s: [u32; 2], e: [u32; 2], p: Precision) -> Result<Option<u32>> {
    segment_parameter_53a3a0(a, c, s, e, p).map_err(|e| format!("53A3A0: {e}"))
}

/// 0x71C930(p0*, p1*): clip the segment to the rectangle (minx, miny, maxx,
/// maxy). Edges are tested A-B, B-C, C-D, D-A; dx is stored binary32, dy
/// stays in the register; the inset is the static 1e-4.
fn clip_71c930(rect: &[u32; 4], p0: &mut [u32; 2], p1: &mut [u32; 2], p: Precision) -> Result<bool> {
    let inside0 = inside_53a7d0(rect, *p0)?;
    let inside1 = inside_53a7d0(rect, *p1)?;
    if inside0 && inside1 {
        return Ok(true);
    }
    let corners = [[rect[0], rect[1]], [rect[2], rect[1]], [rect[2], rect[3]], [rect[0], rect[3]]];
    let (mut count, mut low, mut high) = (0u32, 0u32, 0u32);
    for edge in 0..4 {
        if let Some(t) = segment_53a3a0(*p0, *p1, corners[edge], corners[(edge + 1) % 4], p)? {
            if count == 0 || less(&b(t)?, &b(low)?) {
                low = t;
            }
            if count == 0 || greater(&b(t)?, &b(high)?) {
                high = t;
            }
            count += 1;
        }
    }
    if count == 0 {
        return Ok(false);
    }
    let epsilon = b(CLIP_EPSILON)?;
    let dx = b(st(&sub(&b(p1[0])?, &b(p0[0])?, p)?)?)?;
    let dy = sub(&b(p1[1])?, &b(p0[1])?, p)?;
    let (x0, y0) = (b(p0[0])?, b(p0[1])?);
    let place = |s: &Binary| -> Result<[u32; 2]> {
        Ok([st(&add(&mul(&dx, s, p)?, &x0, p)?)?, st(&add(&mul(s, &dy, p)?, &y0, p)?)?])
    };
    if count == 2 {
        *p1 = place(&sub(&b(high)?, &epsilon, p)?)?;
    } else if inside0 {
        *p1 = place(&sub(&b(low)?, &epsilon, p)?)?;
        return Ok(true);
    }
    *p0 = place(&add(&epsilon, &b(low)?, p)?)?;
    Ok(true)
}

/// 0x860680: binary32 bits truncated toward zero by shifts (exponents of 32
/// and more wrap the shift count exactly like `shr eax, cl`).
fn truncate_860680(bits: u32) -> i32 {
    let exponent = ((bits as i32 >> 23) & 0xff) - 0x7f;
    let magnitude = (bits | 0xff80_0000) << 8;
    let shifted = magnitude >> ((31i32.wrapping_sub(exponent)) as u32 & 31);
    let sign = (bits as i32 >> 31) as u32;
    let mask = !((exponent >> 31) as u32);
    (((shifted ^ sign).wrapping_sub(sign)) & mask) as i32
}
/// 0x42EE20 (count = TerrainMap+0x84) / 0x42EE70 (count = +0x88): values
/// beyond +-2^31 become 0, then idiv by the block size; only the upper
/// bound is clamped.
fn block_index(value: u32, block_size: i32, count: i32) -> Result<i32> {
    let v = b(value)?;
    let truncated = if greater(&v, &b(TWO_POW_31)?) || less(&v, &b(MINUS_TWO_POW_31)?) {
        0
    } else {
        truncate_860680(value)
    };
    let index = idiv(truncated, block_size)?;
    Ok(if index >= count { count.wrapping_sub(1) } else { index })
}
fn column_42ee20(terrain: &impl Terrain, x: u32) -> Result<i32> {
    block_index(x, terrain.block_size()?, terrain.blocks_x()?)
}
fn row_42ee70(terrain: &impl Terrain, y: u32) -> Result<i32> {
    block_index(y, terrain.block_size()?, terrain.blocks_y()?)
}
fn cell<T: Terrain>(terrain: &T, column: i32, row: i32) -> Result<&T::Block> {
    terrain.block(terrain.blocks_x()?.wrapping_mul(row).wrapping_add(column))
}
/// 0x71ADC0(point*): row (0x42EE70 of y) first, then column.
fn block_at_71adc0<T: Terrain>(terrain: &T, point: [u32; 2]) -> Result<&T::Block> {
    let row = row_42ee70(terrain, point[1])?;
    let column = column_42ee20(terrain, point[0])?;
    cell(terrain, column, row)
}

/// CRT `_CIfmod` 0x96DB7A through `_trandisp2` 0x9791C0 (table 0xB350F0):
/// valid/valid runs the exact FPREM loop 0x96DB84; a zero (or x87 denormal)
/// dividend returns +0.0 (0x977754, FLDZ); a zero divisor is the domain
/// error path 0x9777F6, an error here.
fn fmod_96db7a(x: &Binary, y: &Binary) -> Result<Binary> {
    if y.is_zero() {
        return Err("71D840 fmod by a zero block size reaches the CRT domain error".into());
    }
    if x.is_zero() {
        return Ok(Binary::from_i32(0));
    }
    x.remainder_trunc(y).map_err(|e| e.to_string())
}

/// 0x5E8910(p1, p2, p3, p4, out): the p1-p2 segment parameter against the
/// p3-p4 line (only the first parameter is range checked). 0x71D840 only
/// uses AL; the point is still computed and stored like the native code.
fn crossing_5e8910(p1: [u32; 2], p2: [u32; 2], p3: [u32; 2], p4: [u32; 2], p: Precision) -> Result<Option<[u32; 2]>> {
    let a = sub(&b(p4[1])?, &b(p3[1])?, p)?;
    let dx = b(st(&sub(&b(p2[0])?, &b(p1[0])?, p)?)?)?;
    let c = sub(&b(p4[0])?, &b(p3[0])?, p)?;
    let first = mul(&dx, &a, p)?;
    let dy = sub(&b(p2[1])?, &b(p1[1])?, p)?;
    let denominator = sub(&first, &mul(&dy, &c, p)?, p)?;
    if denominator.is_zero() {
        return Ok(None);
    }
    let numerator = sub(
        &mul(&sub(&b(p1[1])?, &b(p3[1])?, p)?, &c, p)?,
        &mul(&sub(&b(p1[0])?, &b(p3[0])?, p)?, &a, p)?,
        p,
    )?;
    let t = b(st(&div(&numerator, &denominator, p)?)?)?;
    if less(&t, &b(ZERO)?) || greater(&t, &b(ONE)?) {
        return Ok(None);
    }
    let x = st(&add(&mul(&dx, &t, p)?, &b(p1[0])?, p)?)?;
    let y = st(&add(&mul(&sub(&b(p2[1])?, &b(p1[1])?, p)?, &t, p)?, &b(p1[1])?, p)?)?;
    Ok(Some([x, y]))
}

/// 0x436310(u, out, v): u x v with the Y component stored binary32; zero
/// vector when the length is <= 1e-6 (0x9FD38C).
fn unit_cross_436310(u: [u32; 3], v: [u32; 3], p: Precision) -> Result<[u32; 3]> {
    let u = [b(u[0])?, b(u[1])?, b(u[2])?];
    let v = [b(v[0])?, b(v[1])?, b(v[2])?];
    let x = sub(&mul(&u[1], &v[2], p)?, &mul(&v[1], &u[2], p)?, p)?;
    let y = b(st(&sub(&mul(&v[0], &u[2], p)?, &mul(&u[0], &v[2], p)?, p)?)?)?;
    let z = sub(&mul(&v[1], &u[0], p)?, &mul(&u[1], &v[0], p)?, p)?;
    let squared = add(&add(&mul(&y, &y, p)?, &mul(&z, &z, p)?, p)?, &mul(&x, &x, p)?, p)?;
    let length = squared.sqrt(p).map_err(|e| e.to_string())?;
    if !greater(&length, &b(UNIT_THRESHOLD)?) {
        return Ok([0; 3]);
    }
    let inverse = div(&b(ONE)?, &length, p)?;
    Ok([st(&mul(&inverse, &x, p)?)?, st(&mul(&y, &inverse, p)?)?, st(&mul(&inverse, &z, p)?)?])
}
/// 0x7151C0(a, b, c): binary32 edges b-a and c-b, 0x436310 normal, and the
/// constant ((n.z*a.z + n.y*a.y) + a.x*n.x).
fn triangle_plane_7151c0(a: [u32; 3], c1: [u32; 3], c2: [u32; 3], p: Precision) -> Result<Plane> {
    let mut u = [0; 3];
    let mut v = [0; 3];
    for i in 0..3 {
        u[i] = st(&sub(&b(c1[i])?, &b(a[i])?, p)?)?;
    }
    for i in 0..3 {
        v[i] = st(&sub(&b(c2[i])?, &b(c1[i])?, p)?)?;
    }
    let normal = unit_cross_436310(u, v, p)?;
    let n = [b(normal[0])?, b(normal[1])?, b(normal[2])?];
    let constant = add(
        &add(&mul(&n[2], &b(a[2])?, p)?, &mul(&n[1], &b(a[1])?, p)?, p)?,
        &mul(&b(a[0])?, &n[0], p)?,
        p,
    )?;
    Ok(Plane { normal, constant: st(&constant)? })
}
/// 0x875610(point, a, b, c): three XY edge tests, each `sum + 1e-4 >= 0`.
fn contains_875610(point: [u32; 3], a: [u32; 3], c1: [u32; 3], c2: [u32; 3], p: Precision) -> Result<bool> {
    let s0 = sub(&b(a[1])?, &b(c1[1])?, p)?;
    let s1 = sub(&b(c1[0])?, &b(a[0])?, p)?;
    let e1 = b(st(&sub(&b(c1[1])?, &b(c2[1])?, p)?)?)?;
    let e2 = b(st(&sub(&b(c2[0])?, &b(c1[0])?, p)?)?)?;
    let e3 = b(st(&sub(&b(c2[1])?, &b(a[1])?, p)?)?)?;
    let e4 = b(st(&sub(&b(a[0])?, &b(c2[0])?, p)?)?)?;
    let r0 = sub(&b(point[0])?, &b(a[0])?, p)?;
    let r1 = sub(&b(point[1])?, &b(a[1])?, p)?;
    let q0 = b(st(&sub(&b(point[0])?, &b(c1[0])?, p)?)?)?;
    let q1 = b(st(&sub(&b(point[1])?, &b(c1[1])?, p)?)?)?;
    let w0 = b(st(&sub(&b(point[0])?, &b(c2[0])?, p)?)?)?;
    let w1 = b(st(&sub(&b(point[1])?, &b(c2[1])?, p)?)?)?;
    let (epsilon, zero) = (b(CONTAIN_EPSILON)?, b(ZERO)?);
    for (l0, r0, l1, r1) in [(&r1, &s1, &r0, &s0), (&q1, &e2, &q0, &e1), (&w1, &e4, &w0, &e3)] {
        let sum = add(&add(&mul(l0, r0, p)?, &mul(l1, r1, p)?, p)?, &epsilon, p)?;
        if less(&sum, &zero) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// 0x71A320(side, eye*, direction*): the side plane hit (t < 0 accepted)
/// must lie strictly below the block maximum height.
fn side_71a320(block: &impl TerrainBlock, side: Side, eye: [u32; 3], direction: [u32; 3], p: Precision) -> Result<bool> {
    let (normal, constant) = match side {
        Side::YHigh => (
            block.owner_y_normal()?,
            block.index_y()?.wrapping_add(1).wrapping_mul(block.quad_size()?).wrapping_mul(block.quads()?),
        ),
        Side::YLow => (
            block.owner_y_normal()?,
            block.quad_size()?.wrapping_mul(block.quads()?).wrapping_mul(block.index_y()?),
        ),
        Side::XLow => (
            block.owner_x_normal()?,
            block.quad_size()?.wrapping_mul(block.quads()?).wrapping_mul(block.index_x()?),
        ),
        Side::XHigh => (
            block.owner_x_normal()?,
            block.index_x()?.wrapping_add(1).wrapping_mul(block.quad_size()?).wrapping_mul(block.quads()?),
        ),
    };
    let plane = Plane { normal, constant: st(&Binary::from_i32(constant))? };
    match plane_42ed60(&plane, eye, direction, true, p)? {
        None => Ok(false),
        Some(hit) => Ok(less(&b(hit[2])?, &b(block.max_height()?)?)),
    }
}

/// 0x736E70 -> 0x736C60: first water triangle (in index order) whose plane
/// hit 0x875610 contains.
fn water_736c60(block: &impl TerrainBlock, eye: [u32; 3], direction: [u32; 3], p: Precision) -> Result<Option<[u32; 3]>> {
    let triangles = block.water_index_count()? / 3;
    for triangle in 0..triangles {
        let [i0, i1, i2] = block.water_triangle(triangle)?;
        let (a, c1, c2) = (block.water_vertex(i0)?, block.water_vertex(i1)?, block.water_vertex(i2)?);
        let plane = triangle_plane_7151c0(a, c1, c2, p)?;
        if let Some(hit) = plane_42ed60(&plane, eye, direction, true, p)? {
            if contains_875610(hit, a, c1, c2, p)? {
                return Ok(Some(hit));
            }
        }
    }
    Ok(None)
}

/// 0x71B9B0(eye*, direction*, point*, secondary*): quads x quads cells, two
/// triangles each (v[e], v[e+1], v[s+e]) then (v[e+1], v[s+e+1], v[s+e]);
/// the land hit goes to the secondary pointer, the primary receives the
/// water hit instead when it is strictly higher.
fn mesh_71b9b0(block: &impl TerrainBlock, eye: [u32; 3], direction: [u32; 3], out: &mut Out, p: Precision) -> Result<bool> {
    let water = if block.water()? != 0 { water_736c60(block, eye, direction, p)? } else { None };
    let quads = block.quads()?;
    let mut land = None;
    let mut e = 0i32;
    'rows: for _ in 0..quads.max(0) {
        for _ in 0..quads {
            let stride = block.vertex_stride()?;
            for corners in [[e, e.wrapping_add(1), stride.wrapping_add(e)], [
                e.wrapping_add(1),
                stride.wrapping_add(e).wrapping_add(1),
                stride.wrapping_add(e),
            ]] {
                let [a, c1, c2] = corners.map(|i| block.mesh_vertex(i));
                let (a, c1, c2) = (a?, c1?, c2?);
                let plane = triangle_plane_7151c0(a, c1, c2, p)?;
                if let Some(hit) = plane_42ed60(&plane, eye, direction, true, p)? {
                    if contains_875610(hit, a, c1, c2, p)? {
                        land = Some(hit);
                        break 'rows;
                    }
                }
            }
            e = e.wrapping_add(1);
        }
        e = e.wrapping_add(1);
    }
    match (land, water) {
        (Some(hit), water) => {
            out.secondary3(hit);
            let higher = match water {
                Some(water) if greater(&b(water[2])?, &b(hit[2])?) => Some(water),
                _ => None,
            };
            out.point3(higher.unwrap_or(hit));
            Ok(true)
        }
        (None, Some(water)) => {
            out.point3(water);
            Ok(true)
        }
        (None, None) => Ok(false),
    }
}

/// 0x71CD30(eye*, direction*, point*): plane at the mid height of the block
/// bounds widened by 1e-4, stored straight to the caller point, then X/Y
/// clamped to the block rectangle widened by 1e-4.
fn flat_71cd30(block: &impl TerrainBlock, eye: [u32; 3], direction: [u32; 3], out: &mut Out, p: Precision) -> Result<()> {
    let quad_size = block.quad_size()?;
    let quads = block.quads()?;
    let column = block.index_x()?;
    let size = quads.wrapping_mul(quad_size);
    let epsilon = b(FLAT_EPSILON)?;
    let low = |v: i32| st(&sub(&Binary::from_i32(v), &epsilon, p)?);
    let high = |v: i32| st(&add(&Binary::from_i32(v), &epsilon, p)?);
    let x0 = low(column.wrapping_mul(size))?;
    let x1 = high(column.wrapping_add(1).wrapping_mul(quads).wrapping_mul(quad_size))?;
    let row = block.index_y()?;
    let y0 = low(row.wrapping_mul(size))?;
    let y1 = high(row.wrapping_add(1).wrapping_mul(quads).wrapping_mul(quad_size))?;
    let middle = mul(
        &add(&add(&b(block.max_height()?)?, &epsilon, p)?, &sub(&b(block.min_height()?)?, &epsilon, p)?, p)?,
        &b(HALF)?,
        p,
    )?;
    let plane = Plane { normal: block.owner_up_normal()?, constant: st(&middle)? };
    let hit = plane_42ed60(&plane, eye, direction, true, p)?
        .ok_or("71CD30 parallel ray: native clamps the unwritten caller point")?;
    out.point3(hit);
    for (word, low, high) in [(0, x0, x1), (1, y0, y1)] {
        let v = b(out.read_point(word)?)?;
        if less(&v, &b(low)?) {
            out.point(word, low);
        } else if greater(&v, &b(high)?) {
            out.point(word, high);
        }
    }
    Ok(())
}

/// 0x71D570(column, row, side, eye*, direction*, point*, secondary*).
fn block_query_71d570<T: Terrain>(
    terrain: &T,
    column: i32,
    row: i32,
    side: Side,
    eye: [u32; 3],
    direction: [u32; 3],
    out: &mut Out,
    p: Precision,
) -> Result<bool> {
    let block = cell(terrain, column, row)?;
    if !side_71a320(block, side, eye, direction, p)? {
        return Ok(false);
    }
    if block.has_mesh()? {
        mesh_71b9b0(block, eye, direction, out, p)
    } else {
        flat_71cd30(block, eye, direction, out, p)?;
        Ok(true)
    }
}

/// 0x71D840 with its seven native arguments. `direction` is the by-value
/// triple (normalized in place by 0x42D750 first); `secondary` is the
/// optional sixth pointer (stores only when present); `margin` is the
/// signed seventh argument. Errors cover undefined native reads, unproven
/// numeric domains (nonfinite values, binary32 overflow, idiv faults) and
/// missing terrain data.
pub fn intersect_71d840<T: Terrain>(
    terrain: &T,
    eye: [u32; 3],
    direction: [u32; 3],
    secondary: Option<&mut [u32; 3]>,
    margin: i32,
    p: Precision,
) -> Result<Outcome> {
    let direction = crate::native_camera::ni::normalize(direction, p)?;
    let up = terrain.up_normal()?;
    let top_plane = Plane { normal: up, constant: terrain.max_height()? };
    let bottom_plane = Plane { normal: up, constant: terrain.min_height()? };
    let top = plane_42ed60(&top_plane, eye, direction, true, p)?
        .ok_or("71D840 parallel ray: 0x71D8BD reads unwritten 42ED60 locals")?;
    let bottom = plane_42ed60(&bottom_plane, eye, direction, true, p)?
        .ok_or("71D840 parallel ray: second 42ED60 left its local unwritten")?;
    let mut out = Out { point: [None; 3], secondary, writes: Vec::new() };
    if terrain.loaded()? == 0 {
        out.point3(bottom);
        return Ok(out.finish(true));
    }
    let inset = b(RECT_INSET)?;
    let low = st(&Binary::from_i32(margin))?;
    let rect = [
        low,
        low,
        st(&sub(&Binary::from_i32(terrain.width()?.wrapping_sub(margin)), &inset, p)?)?,
        st(&sub(&Binary::from_i32(terrain.height()?.wrapping_sub(margin)), &inset, p)?)?,
    ];
    let mut p0 = [top[0], top[1]];
    let mut p1 = [bottom[0], bottom[1]];
    if !clip_71c930(&rect, &mut p0, &mut p1, p)? {
        // 0x71D97B: clamped endpoints select two blocks; the point mixes the
        // unclamped plane hits by the averaged block heights.
        let mut near = [top[0], top[1]];
        clamp_71ab10(&rect, &mut near)?;
        let mut far = [bottom[0], bottom[1]];
        clamp_71ab10(&rect, &mut far)?;
        let near = block_at_71adc0(terrain, near)?;
        let far = block_at_71adc0(terrain, far)?;
        let average = mul(
            &add(
                &add(&add(&b(far.max_height()?)?, &b(far.min_height()?)?, p)?, &b(near.max_height()?)?, p)?,
                &b(near.min_height()?)?,
                p,
            )?,
            &b(QUARTER)?,
            p,
        )?;
        out.point(2, st(&average)?);
        let min = b(terrain.min_height()?)?;
        let range = sub(&b(terrain.max_height()?)?, &min, p)?;
        let fraction = if range.is_zero() { average } else { div(&sub(&average, &min, p)?, &range, p)? };
        let rest = sub(&b(ONE)?, &fraction, p)?;
        for word in 0..2 {
            let mixed = add(&mul(&b(top[word])?, &fraction, p)?, &mul(&rest, &b(bottom[word])?, p)?, p)?;
            out.point(word, st(&mixed)?);
        }
        return Ok(out.finish(false));
    }
    if b(p0[0])?.compare(&b(p1[0])?).is_eq() && b(p0[1])?.compare(&b(p1[1])?).is_eq() {
        // 0x71DAC6: degenerate clipped segment.
        out.point(0, p0[0]);
        out.point(1, p0[1]);
        let block = block_at_71adc0(terrain, p0)?;
        out.point(2, block.max_height()?);
        return Ok(out.finish(true));
    }
    // 0x71DAF6: walk along the major axis; (a, c) = (major, minor).
    let x_major = !less(&abs(sub(&b(p1[0])?, &b(p0[0])?, p)?), &abs(sub(&b(p1[1])?, &b(p0[1])?, p)?));
    let swap = |v: [u32; 2]| if x_major { v } else { [v[1], v[0]] };
    let (start, end) = (swap(p0), swap(p1));
    let world = |a: u32, c: u32| if x_major { [a, c] } else { [c, a] };
    let size = terrain.block_size()?;
    let signed = |back: bool| if back { size.wrapping_neg() } else { size };
    let step_major = st(&Binary::from_i32(signed(less(&b(end[0])?, &b(start[0])?))))?;
    let minor_register = Binary::from_i32(signed(less(&b(end[1])?, &b(start[1])?)));
    let step_minor = st(&minor_register)?;
    let zero = b(ZERO)?;
    let major_forward = greater(&b(step_major)?, &zero);
    let minor_forward = greater(&minor_register, &zero);
    let (side_major, side_minor) = if x_major {
        (if major_forward { Side::XHigh } else { Side::XLow }, if minor_forward { Side::YHigh } else { Side::YLow })
    } else {
        (if major_forward { Side::YHigh } else { Side::YLow }, if minor_forward { Side::XHigh } else { Side::XLow })
    };
    let major_span = column_42ee20(terrain, end[0])?.wrapping_sub(column_42ee20(terrain, start[0])?);
    let minor_span = row_42ee70(terrain, end[1])?.wrapping_sub(row_42ee70(terrain, start[1])?);
    let mut counter = minor_span.wrapping_abs().wrapping_add(major_span.wrapping_abs());
    let (mut a, mut c) = (start[0], start[1]);
    if counter != 0 {
        'walk: loop {
            counter = counter.wrapping_sub(1);
            let size_word = st(&Binary::from_i32(size))?;
            let block_size = b(size_word)?;
            let major_start = st(&sub(&b(a)?, &fmod_96db7a(&b(a)?, &block_size)?, p)?)?;
            let minor_start = sub(&b(c)?, &fmod_96db7a(&b(c)?, &block_size)?, p)?;
            let minor_end = add(&Binary::from_i32(size), &minor_start, p)?;
            let edge = if less(&b(step_major)?, &zero) {
                major_start
            } else {
                st(&add(&block_size, &b(major_start)?, p)?)?
            };
            let crossed =
                crossing_5e8910([edge, st(&minor_start)?], [edge, st(&minor_end)?], start, end, p)?.is_some();
            if !crossed {
                // 0x71DD37: leave through the minor side of this block.
                let [x, y] = world(a, c);
                let row = row_42ee70(terrain, y)?;
                let column = column_42ee20(terrain, x)?;
                if block_query_71d570(terrain, column, row, side_minor, eye, direction, &mut out, p)? {
                    return Ok(out.finish(true));
                }
                c = st(&add(&b(c)?, &b(step_minor)?, p)?)?;
                let before = counter;
                counter = counter.wrapping_sub(1);
                if before == 0 {
                    break 'walk;
                }
            }
            // 0x71DDC0 (inline 0x42EE70 then 0x42EE20 copies).
            let [x, y] = world(a, c);
            let row = row_42ee70(terrain, y)?;
            let column = column_42ee20(terrain, x)?;
            if block_query_71d570(terrain, column, row, side_major, eye, direction, &mut out, p)? {
                return Ok(out.finish(true));
            }
            a = st(&add(&b(a)?, &b(step_major)?, p)?)?;
            if counter == 0 {
                break 'walk;
            }
        }
    }
    // 0x71DEB3: the final block, major side then minor side.
    for side in [side_major, side_minor] {
        let [x, y] = world(a, c);
        let row = row_42ee70(terrain, y)?;
        let column = column_42ee20(terrain, x)?;
        if block_query_71d570(terrain, column, row, side, eye, direction, &mut out, p)? {
            return Ok(out.finish(true));
        }
    }
    // 0x71DF72: no block answered.
    let chosen = if inside_53a7d0(&rect, [top[0], top[1]])? { bottom } else { top };
    out.point(0, chosen[0]);
    out.point(1, chosen[1]);
    let mut clamped = [chosen[0], chosen[1]];
    clamp_71ab10(&rect, &mut clamped)?;
    let block = block_at_71adc0(terrain, clamped)?;
    let middle = mul(&add(&b(block.max_height()?)?, &b(block.min_height()?)?, p)?, &b(HALF)?, p)?;
    out.point(2, st(&middle)?);
    Ok(out.finish(false))
}

#[cfg(test)]
#[path = "terrain_ray_tests.rs"]
mod tests;
