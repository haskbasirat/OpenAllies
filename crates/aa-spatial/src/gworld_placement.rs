//! Native GWorld placement test `0x4F3C40` and its structure-crossing test
//! `0x4F22C0`, both thiscall on GWorld.
//!
//! `0x4F3C40(source actor, collision descriptor)`, ret 8, returns
//! 0 clear, 1 region change, 2 structure crossing, 3 outside the world,
//! 4 blocked by the first hit. Its callers only test the result against zero:
//! the low-path reachability test `0x53A6F0` and `0x53CE40` (source =
//! descriptor +0x1C), the region prepass `0x53DC00` and `0x4F46E0` (its actor
//! argument, from which the descriptor was built by `0x554350`).
//!
//! Native order of `0x4F3C40`:
//! 1. Copy the descriptor position (+0x4C/+0x50) to the frame once.
//! 2. Return 3 unless `0 <= x < extent.x` and `0 <= y < extent.y`, with the
//!    extent at GMap (`[GWorld+0x30]`) +0xC/+0x10. These are x87 FCOMPs of
//!    exactly loaded binary32 words: an unordered result (NaN point or extent)
//!    is outside, and -0.0 equals 0.0 (see [`outside_world`]).
//! 3. Return 0 when descriptor +0x3C & 0x300 is zero; nothing else is read.
//! 4. Read the source +0x20/+0x24 (no null test: a null source faults) and
//!    compare the `0x54ECF0` regions of the `0x42A530` tiles under the source
//!    and under the descriptor copy. Different words return 1.
//! 5. Re-read the source +0x20/+0x24 and return 2 when
//!    `0x4F22C0(source, source position, descriptor copy, descriptor)` is
//!    nonzero ([`structure_crossing`]).
//! 6. Return 4 when `0x553BA0(GMap, result buffer, descriptor)` stores a
//!    nonzero result word 0 (full dword compare). The first argument is the
//!    16-byte result buffer, not a position: the query reads the descriptor.
//! 7. Return 0.
//!
//! Neither function performs floating-point arithmetic: the bounds test only
//! loads and compares binary32 words, and `0x42A530` truncates them exactly
//! through CRT `_ftol` (`0x96CC4C`). Results are therefore identical at
//! PC24/PC53/PC64; the oracle proves this at all three. 0x553BA0's own
//! precision is the spatial index's explicit precision.
//!
//! Every leaf is a required [`CrossingWorld`] / [`PlacementWorld`] callback;
//! none has a default. The descriptor is read-only here: the native leaves
//! receive no descriptor pointer and the oracle checks that the descriptor is
//! unchanged after every call.
//!
//! Not ported (explicit errors): a null source when the mask passes (native
//! access violation at `0x4F3CC0`), and source positions whose `_ftol` lies
//! outside [`GMapView::tile_at_point`]'s domain (NaN, infinities and
//! truncations outside i32, which the native maps to the low dword of the
//! integer-indefinite or 64-bit result) or whose tile lies outside the tile
//! allocation (the native reads foreign memory).
//!
//! The two callers with their own logic are ported here too:
//! [`endpoint_acceptable`] (`0x53CE40`, the destination test of `0x53D340`)
//! and [`nearest_placement`] (`0x4F46E0`, the ring search around a point).
//! Both reach `0x4F3C40` through a recorded callback.
use crate::{
    collision::Descriptor,
    index::ActorKey,
    placement::{ROTATION_COS_BITS, ROTATION_SIN_BITS},
    query::{Hit, PreparedQuery},
    water_areas::{self, WaterMap},
};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `0x42A530` input outside the ported domain, or a tile index outside
    /// the native tile allocation.
    #[error(transparent)]
    Tile(#[from] water_areas::Error),
    /// A spatial boundary such as `aa_spatial::query::first_hit` failed.
    #[error(transparent)]
    Spatial(#[from] crate::Error),
    /// `0x4F46E0` arithmetic on a non-finite word, or a binary32 overflow at
    /// one of its stores (the original keeps computing with NaN/infinity).
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    /// `0x4F3C40` reads +0x20 of a null source actor (`0x4F3CC0`) when the
    /// descriptor mask has 0x300 bits; the original faults.
    #[error("0x4F3C40 dereferences a null source actor")]
    NullSource,
    /// `0x4F46E0` reads the vtable of a null actor (`0x4F46EC`); the original
    /// faults.
    #[error("0x4F46E0 dereferences a null actor")]
    NullActor,
    #[error("GWorld placement boundary failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

/// `0x4F3C40` return values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    /// 0: nothing objects to the placement.
    Clear,
    /// 1: the source tile and the target tile have different `0x54ECF0` words.
    RegionChange,
    /// 2: `0x4F22C0` reported a structure crossing.
    StructureCrossing,
    /// 3: the descriptor position is not strictly inside the GMap extent.
    OutsideWorld,
    /// 4: `0x553BA0` returned a nonzero result kind.
    Blocked,
}
impl Verdict {
    /// The native return value.
    pub fn code(self) -> u32 {
        match self {
            Self::Clear => 0,
            Self::RegionChange => 1,
            Self::StructureCrossing => 2,
            Self::OutsideWorld => 3,
            Self::Blocked => 4,
        }
    }
}

/// Descriptor +0x3C bits that enable the region, crossing and hit tests.
pub const REGION_TEST_MASK: u32 = 0x300;
/// `0x9F32DC`: binary32 0.0, the lower bound of both axes.
pub const ZERO_BITS: u32 = 0;
/// `0x4F471B`: bit `0x4F46E0` ORs into descriptor +0x38 (required actor flags).
pub const NEAREST_REQUIRED_FLAGS: u32 = 0x2000_0000;
/// `0x4F4864`: rotations per ring of `0x4F46E0`.
pub const NEAREST_ROTATIONS: u32 = 8;

/// The GMap (`[GWorld+0x30]`) words these functions read: +0x1C/+0x20 tile
/// counts, +4 coordinate shift, +8 binary32 cell size and +0xC/+0x10
/// binary32 world extents. The tile allocation holds `width * height` tiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMapView {
    pub width: i32,
    pub height: i32,
    pub coordinate_shift: u32,
    pub cell_size_bits: u32,
    pub extent_bits: [u32; 2],
}
impl From<&WaterMap> for GMapView {
    fn from(map: &WaterMap) -> Self {
        Self {
            width: map.width,
            height: map.height,
            coordinate_shift: map.coordinate_shift,
            cell_size_bits: map.cell_size_bits,
            extent_bits: map.extent_bits,
        }
    }
}
impl GMapView {
    /// `width * height`, which must be a positive i32 like the native layout.
    pub fn tile_count(&self) -> Result<usize> {
        match self.width.checked_mul(self.height) {
            Some(n) if self.width > 0 && self.height > 0 => Ok(n as usize),
            _ => Err(water_areas::Error::InvalidLayout.into()),
        }
    }
    /// Exact `0x42A530` arithmetic (inlined again at `0x53CEBD..0x53CEFD`):
    /// CRT `_ftol` truncation of each binary32 word, arithmetic right shift by
    /// +4 & 31, then the wrapping `width * y + x`; no clamp. The domain matches
    /// [`water_areas::tile_at_point`]: non-finite words and truncations outside
    /// i32 are refused, and an index outside the `width * height` allocation
    /// is [`water_areas::Error::TileOutside`].
    pub fn tile_at_point(&self, point_bits: [u32; 2]) -> Result<usize> {
        let shift = self.coordinate_shift & 31;
        let truncate = |bits: u32| -> Result<i32> {
            let value = Binary::from_f32_bits(bits).map_err(water_areas::Error::from)?;
            Ok(value.truncate_i32().map_err(water_areas::Error::from)? >> shift)
        };
        let x = truncate(point_bits[0])?;
        let y = truncate(point_bits[1])?;
        let index = self.width.wrapping_mul(y).wrapping_add(x);
        if index < 0 || index as usize >= self.tile_count()? {
            return Err(water_areas::Error::TileOutside(i64::from(index)).into());
        }
        Ok(index as usize)
    }
}

/// Leaves of `0x4F22C0`. Objects (actors and structures) are spatial actor
/// keys; `None` is a null pointer. Boolean results are the native AL test.
pub trait CrossingWorld {
    /// Object virtual +0xB4 (no arguments).
    fn actor_virtual_b4(&mut self, actor: ActorKey) -> Result<bool>;
    /// `0x4F2220(GWorld, point)`: the structure whose footprint holds the
    /// point (binary32 words, passed through unchanged), or `None`.
    fn structure_at(&mut self, point: [u32; 2]) -> Result<Option<ActorKey>>;
    /// `0x4D36F0`, thiscall on the actor with the structure argument.
    fn actor_4d36f0(&mut self, actor: ActorKey, structure: ActorKey) -> Result<bool>;
    /// Structure virtual +0x168 with the actor argument.
    fn structure_virtual_168(&mut self, structure: ActorKey, actor: ActorKey) -> Result<bool>;
    /// `0x585280` on the structure's word +0x98 (read at the call), with the
    /// actor argument.
    fn footprint_585280(&mut self, structure: ActorKey, actor: ActorKey) -> Result<bool>;
    /// Object word +0x90 is nonnull (read right after virtual +0xB4).
    fn actor_word_90_present(&mut self, actor: ActorKey) -> Result<bool>;
    /// Object virtual +0x100 (no arguments): an object pointer.
    fn actor_virtual_100(&mut self, actor: ActorKey) -> Result<Option<ActorKey>>;
    /// `0x585160` on the structure's word +0x98 (read at the call).
    fn footprint_585160(&mut self, structure: ActorKey) -> Result<bool>;
    /// Actor virtual +0x164 with the structure argument.
    fn actor_virtual_164(&mut self, actor: ActorKey, structure: ActorKey) -> Result<bool>;
}

/// Additional leaves of `0x4F3C40`.
pub trait PlacementWorld: CrossingWorld {
    /// Raw actor words +0x20/+0x24, read at each native read point.
    fn actor_position(&mut self, actor: ActorKey) -> Result<[u32; 2]>;
    /// GMap tile word +0x10 (the region node; `None` is null) of the tile at
    /// the native linear index (`0x42A530`).
    fn tile_region_node(&mut self, tile: usize) -> Result<Option<u32>>;
    /// Region node word +8 (`None` is 0).
    fn region_node_region(&mut self, node: u32) -> Result<Option<u32>>;
    /// `0x553BA0` on GWorld+0x30 with this descriptor (its own position):
    /// `aa_spatial::query::first_hit` over the authoritative index.
    fn first_hit(&mut self, query: &PreparedQuery) -> Result<Hit>;
}

fn is_nan(bits: u32) -> bool {
    bits & 0x7fff_ffff > 0x7f80_0000
}

/// x87 FCOM of two exactly loaded binary32 words; `None` is unordered.
/// Signed zeros are equal; infinities and subnormals order normally.
fn fcom(left: u32, right: u32) -> Option<Ordering> {
    if is_nan(left) || is_nan(right) {
        return None;
    }
    let key = |bits: u32| {
        let magnitude = i64::from(bits & 0x7fff_ffff);
        if bits >> 31 != 0 { -magnitude } else { magnitude }
    };
    Some(key(left).cmp(&key(right)))
}

/// `0x4F3C49..0x4F3C9A`. The `< 0` tests branch on C0 alone (`test ah,1`),
/// which an unordered compare also sets; the `< extent` tests pass only when
/// C0 is the sole flag among C0/C2 (`test ah,5` with parity). So only an
/// ordered "less" passes either kind of test.
pub fn outside_world(position: [u32; 2], extent_bits: [u32; 2]) -> bool {
    let below_zero = |v: u32| matches!(fcom(v, ZERO_BITS), Some(Ordering::Less) | None);
    let not_below = |v: u32, extent: u32| fcom(v, extent) != Some(Ordering::Less);
    below_zero(position[0])
        || not_below(position[0], extent_bits[0])
        || below_zero(position[1])
        || not_below(position[1], extent_bits[1])
}

/// Complete `0x54ECF0` on a tile: word +0x10, null giving 0, else its +8.
pub fn tile_region<W: PlacementWorld + ?Sized>(world: &mut W, tile: usize) -> Result<Option<u32>> {
    match world.tile_region_node(tile)? {
        None => Ok(None),
        Some(node) => world.region_node_region(node),
    }
}

/// Complete `0x4F22C0`; `true` is the native return value 2.
///
/// `from`/`to` are only forwarded to `0x4F2220`. The exempt structure is
/// descriptor +0x28, or, when that is null, the +0x100 object of descriptor
/// +0x20 if that object's +0xB4 is true and its +0x90 word is nonnull. The
/// admitted structure (source structure accepting the source through +0x168
/// and `0x585280`) is skipped only for the source-side test.
pub fn structure_crossing<W: CrossingWorld + ?Sized>(
    source: Option<ActorKey>,
    from: [u32; 2],
    to: [u32; 2],
    descriptor: &Descriptor,
    world: &mut W,
) -> Result<bool> {
    let Some(source) = source else {
        return Ok(false);
    };
    if !world.actor_virtual_b4(source)? {
        return Ok(false);
    }
    let from_structure = world.structure_at(from)?;
    let to_structure = world.structure_at(to)?;
    if let Some(structure) = from_structure {
        if from_structure == to_structure {
            return Ok(false);
        }
        if world.actor_4d36f0(source, structure)? {
            return Ok(true);
        }
    } else if to_structure.is_none() {
        return Ok(false);
    }
    let mut admitted = None;
    if let Some(structure) = from_structure
        && world.structure_virtual_168(structure, source)?
        && world.footprint_585280(structure, source)?
    {
        admitted = Some(structure);
    }
    let mut exempt = descriptor.collision_owner;
    if exempt.is_none()
        && let Some(ignore) = descriptor.ignore_actor
        && world.actor_virtual_b4(ignore)?
        && world.actor_word_90_present(ignore)?
    {
        exempt = world.actor_virtual_100(ignore)?;
    }
    if let Some(structure) = from_structure
        && from_structure != exempt
        && from_structure != admitted
        && world.footprint_585160(structure)?
        && !world.actor_virtual_164(source, structure)?
    {
        return Ok(true);
    }
    if let Some(structure) = to_structure
        && to_structure != exempt
        && world.footprint_585160(structure)?
        && !world.actor_virtual_164(source, structure)?
    {
        return Ok(true);
    }
    Ok(false)
}

/// Complete `0x4F3C40`. `map` is the GMap (`[GWorld+0x30]`): its
/// `extent_bits` are +0xC/+0x10 and its shift, width and tile count define
/// `0x42A530`. `query` is the collision descriptor: position +0x4C/+0x50,
/// mask +0x3C, ignore actor +0x20 and collision owner +0x28 are read here and
/// the whole descriptor is handed to `0x553BA0`.
pub fn placement_result<W: PlacementWorld + ?Sized>(
    map: &GMapView,
    source: Option<ActorKey>,
    query: &PreparedQuery,
    world: &mut W,
) -> Result<Verdict> {
    let position = query.placement.position;
    if outside_world(position, map.extent_bits) {
        return Ok(Verdict::OutsideWorld);
    }
    if query.descriptor.mask & REGION_TEST_MASK == 0 {
        return Ok(Verdict::Clear);
    }
    let source = source.ok_or(Error::NullSource)?;
    let source_tile = map.tile_at_point(world.actor_position(source)?)?;
    let source_region = tile_region(world, source_tile)?;
    let target_tile = map.tile_at_point(position)?;
    let target_region = tile_region(world, target_tile)?;
    if source_region != target_region {
        return Ok(Verdict::RegionChange);
    }
    // 0x4F3D28 reads the source position again for the crossing arguments.
    let from = world.actor_position(source)?;
    if structure_crossing(Some(source), from, position, &query.descriptor, world)? {
        return Ok(Verdict::StructureCrossing);
    }
    if world.first_hit(query)?.kind != 0 {
        return Ok(Verdict::Blocked);
    }
    Ok(Verdict::Clear)
}

/// Descriptor services shared by `0x53CE40` and `0x4F46E0`.
pub trait DescriptorWorld {
    /// `0x552250` on the descriptor with the point passed by value: stores
    /// +0x4C/+0x50, snapping through the source definition when its
    /// ObjectData flag asks (`PreparedQuery::set_position`).
    fn set_descriptor_position(
        &mut self,
        descriptor: &mut PreparedQuery,
        point: [u32; 2],
    ) -> Result<()>;
    /// `0x4F3C40(GWorld, source, descriptor)`, the full EAX result; only its
    /// zero test is used ([`placement_result`] supplies it).
    fn placement_code(&mut self, source: Option<ActorKey>, descriptor: &PreparedQuery)
    -> Result<u32>;
}

/// Leaves of `0x53CE40`. Tiles are native linear GMap tile indices.
pub trait EndpointWorld: DescriptorWorld {
    /// GMap tile byte +0x20 (the low byte of the tile flags).
    fn tile_byte_20(&mut self, tile: usize) -> Result<u8>;
    /// `0x54F370`, thiscall on the tile with the descriptor argument
    /// (`CollisionView::blocked`); it may update the descriptor.
    fn tile_blocked(&mut self, tile: usize, descriptor: &mut PreparedQuery) -> Result<bool>;
}

/// Complete `0x53CE40` (cdecl; its first argument is never read). Returns AL.
///
/// The point must pass the same half-open, unordered-rejecting bounds as
/// `0x4F3C40` ([`outside_world`]); then `0x552250` places the descriptor, and
/// the tile under the caller's point words (re-read after `0x552250`, not the
/// possibly snapped descriptor position) is rejected when its byte +0x20 has
/// bit 0 or `0x54F370` reports it blocked. Otherwise the result is whether
/// `0x4F3C40(GWorld, descriptor +0x1C read after 0x54F370, descriptor)` is 0.
pub fn endpoint_acceptable<W: EndpointWorld + ?Sized>(
    map: &GMapView,
    point: [u32; 2],
    descriptor: &mut PreparedQuery,
    world: &mut W,
) -> Result<bool> {
    if outside_world(point, map.extent_bits) {
        return Ok(false);
    }
    world.set_descriptor_position(descriptor, point)?;
    let tile = map.tile_at_point(point)?;
    if world.tile_byte_20(tile)? & 1 != 0 {
        return Ok(false);
    }
    if world.tile_blocked(tile, descriptor)? {
        return Ok(false);
    }
    let source = descriptor.descriptor.source_actor;
    Ok(world.placement_code(source, descriptor)? == 0)
}

/// Leaves of `0x4F46E0`.
pub trait NearestPlacementWorld: DescriptorWorld {
    /// Actor virtual +0xB4 (no arguments).
    fn actor_virtual_b4(&mut self, actor: ActorKey) -> Result<bool>;
    /// Actor word +0x7C (`None` is null).
    fn actor_word_7c(&mut self, actor: ActorKey) -> Result<Option<u32>>;
    /// Word +0x40 of the actor's +0x7C object: the replacement actor.
    fn object_word_40(&mut self, object: u32) -> Result<Option<ActorKey>>;
    /// `0x554350(descriptor, actor, 0)`: the actor's collision descriptor
    /// (`actor_context::prepare`).
    fn prepare_descriptor(&mut self, actor: ActorKey) -> Result<PreparedQuery>;
    /// Actor words +0x2C/+0x30 (facing), read once after the first failure.
    fn actor_facing(&mut self, actor: ActorKey) -> Result<[u32; 2]>;
}

fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn stored(value: &Binary) -> Result<u32> {
    Ok(value.store_f32()?)
}
/// x87 compare of an unstored extended value with a binary32 word.
fn compare_extended(value: &Binary, bits: u32) -> Result<Option<Ordering>> {
    if is_nan(bits) {
        return Ok(None);
    }
    if bits & 0x7fff_ffff == 0x7f80_0000 {
        return Ok(Some(if bits >> 31 != 0 {
            Ordering::Greater
        } else {
            Ordering::Less
        }));
    }
    Ok(Some(value.compare(&b(bits)?)))
}
/// `test ah,0x41` + parity: only an ordered "less" or "equal" continues.
fn at_most(order: Option<Ordering>) -> bool {
    matches!(order, Some(Ordering::Less | Ordering::Equal))
}

/// Complete `0x4F46E0` (thiscall GWorld (actor, point*, radius), ret 0xC).
/// Returns AL; on success the point holds the accepted candidate.
///
/// The actor is replaced by `[[actor+0x7C]+0x40]` when its +0xB4 is true and
/// +0x7C is nonnull; a null result returns false. The descriptor comes from
/// `0x554350(actor, 0)` with +0x38 |= 0x20000000. The point itself is tried
/// first. Otherwise, with `cell` = GMap +8 and `dir` = actor +0x2C/+0x30,
/// while `step <= radius` (starting at `step = cell`): the offset is
/// `f32(dir * step)`, and up to eight candidates `f32(offset + point)` are
/// tried, each failure rotating the offset by the table values for -pi/4
/// (`0x85DAA0` sin stored to binary32, `0x85DA80` cos kept unstored; both are
/// exact binary32 table words, [`ROTATION_SIN_BITS`]/[`ROTATION_COS_BITS`]).
/// After each ring `step + cell` is stored to binary32 but the UNSTORED sum
/// is compared with the radius. Every failure also performs a second
/// rotation (two further table calls) of a facing copy that is reset at
/// the next ring and never read; table lookups are pure, so it is omitted.
pub fn nearest_placement<W: NearestPlacementWorld + ?Sized>(
    map: &GMapView,
    actor: Option<ActorKey>,
    point: &mut [u32; 2],
    radius_bits: u32,
    precision: Precision,
    world: &mut W,
) -> Result<bool> {
    let p = precision;
    let caller = actor.ok_or(Error::NullActor)?;
    let mut actor = Some(caller);
    if world.actor_virtual_b4(caller)?
        && let Some(object) = world.actor_word_7c(caller)?
    {
        actor = world.object_word_40(object)?;
    }
    let Some(actor) = actor else {
        return Ok(false);
    };
    let mut descriptor = world.prepare_descriptor(actor)?;
    descriptor.required_actor_flags |= NEAREST_REQUIRED_FLAGS;
    world.set_descriptor_position(&mut descriptor, *point)?;
    if world.placement_code(Some(actor), &descriptor)? == 0 {
        return Ok(true);
    }
    let cell_bits = map.cell_size_bits;
    let direction = world.actor_facing(actor)?;
    if !at_most(fcom(cell_bits, radius_bits)) {
        return Ok(false);
    }
    let (sin, cos) = (b(ROTATION_SIN_BITS)?, b(ROTATION_COS_BITS)?);
    let mut step = cell_bits;
    loop {
        let mut offset = [
            stored(&b(direction[0])?.mul(&b(step)?, p)?)?,
            stored(&b(direction[1])?.mul(&b(step)?, p)?)?,
        ];
        for _ in 0..NEAREST_ROTATIONS {
            let candidate = [
                stored(&b(offset[0])?.add(&b(point[0])?, p)?)?,
                stored(&b(offset[1])?.add(&b(point[1])?, p)?)?,
            ];
            world.set_descriptor_position(&mut descriptor, candidate)?;
            if world.placement_code(Some(actor), &descriptor)? == 0 {
                *point = [
                    stored(&b(offset[0])?.add(&b(point[0])?, p)?)?,
                    stored(&b(offset[1])?.add(&b(point[1])?, p)?)?,
                ];
                return Ok(true);
            }
            let (x, y) = (b(offset[0])?, b(offset[1])?);
            offset = [
                stored(&x.mul(&cos, p)?.sub(&y.mul(&sin, p)?, p)?)?,
                stored(&y.mul(&cos, p)?.add(&sin.mul(&x, p)?, p)?)?,
            ];
        }
        let sum = b(step)?.add(&b(cell_bits)?, p)?;
        step = stored(&sum)?;
        if !at_most(compare_extended(&sum, radius_bits)?) {
            return Ok(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAN: u32 = 0x7fc0_0000;
    const SNAN: u32 = 0x7f80_0001;
    const INF: u32 = 0x7f80_0000;

    #[test]
    fn fcom_orders_like_x87() {
        assert_eq!(fcom(0x8000_0000, 0), Some(Ordering::Equal));
        assert_eq!(fcom(0x8000_0001, 0), Some(Ordering::Less));
        assert_eq!(fcom(1, 0x8000_0000), Some(Ordering::Greater));
        assert_eq!(fcom(INF, 0x7f7f_ffff), Some(Ordering::Greater));
        assert_eq!(fcom(0xff80_0000, 0xff7f_ffff), Some(Ordering::Less));
        assert_eq!(fcom(NAN, 0), None);
        assert_eq!(fcom(0, SNAN | 0x8000_0000), None);
        assert_eq!(fcom(16f32.to_bits(), 16f32.to_bits()), Some(Ordering::Equal));
    }

    #[test]
    fn bounds_are_half_open_and_reject_unordered() {
        let extent = [16f32.to_bits(), 14f32.to_bits()];
        let y = 5f32.to_bits();
        assert!(!outside_world([0, y], extent));
        assert!(!outside_world([0x8000_0000, y], extent));
        assert!(!outside_world([0x417f_ffff, y], extent));
        assert!(outside_world([16f32.to_bits(), y], extent));
        assert!(outside_world([0x8000_0001, y], extent));
        assert!(outside_world([NAN, y], extent));
        assert!(outside_world([y, SNAN], extent));
        assert!(outside_world([INF, y], extent));
        assert!(outside_world([y, y], [NAN, extent[1]]));
        assert!(!outside_world([y, y], [INF, extent[1]]));
    }

    #[test]
    fn gmap_view_tile_lookup_matches_the_water_map_lookup() {
        use crate::water_areas::{AREA_TABLE_LEN, WaterTile, tile_at_point};
        let tile = WaterTile { terrain_class: None, link: None, visit_stamp: 0, area: 0, reached_from: 0 };
        for (width, height, shift) in [(8, 7, 1), (5, 4, 2), (8, 8, 0), (3, 9, 31)] {
            let map = WaterMap {
                width,
                height,
                coordinate_shift: shift,
                cell_size_bits: 2f32.to_bits(),
                extent_bits: [16f32.to_bits(), 14f32.to_bits()],
                visit_stamp: 0,
                tiles: vec![tile.clone(); (width * height) as usize],
                areas: vec![None; AREA_TABLE_LEN],
            };
            let view = GMapView::from(&map);
            assert_eq!(view.cell_size_bits, map.cell_size_bits);
            let values = [
                0f32.to_bits(),
                0x8000_0000,
                1,
                (-0.75f32).to_bits(),
                (-2.5f32).to_bits(),
                3f32.to_bits(),
                15.99f32.to_bits(),
                17f32.to_bits(),
                (-40f32).to_bits(),
                2.1e9f32.to_bits(),
                3e9f32.to_bits(),
                NAN,
                SNAN,
                INF,
                0xff80_0000,
            ];
            for x in values {
                for y in values {
                    let ours = view.tile_at_point([x, y]);
                    let theirs = tile_at_point(&map, [x, y]);
                    match (&ours, &theirs) {
                        (Ok(a), Ok(b)) => assert_eq!(a, b),
                        (Err(Error::Tile(a)), Err(b)) => assert_eq!(a.to_string(), b.to_string()),
                        _ => panic!("{x:#x},{y:#x}: {ours:?} vs {theirs:?}"),
                    }
                }
            }
        }
        let empty = GMapView { width: 0, height: 7, coordinate_shift: 1, cell_size_bits: 0, extent_bits: [0, 0] };
        assert!(matches!(
            empty.tile_at_point([0, 0]),
            Err(Error::Tile(water_areas::Error::InvalidLayout))
        ));
    }

    #[test]
    fn verdict_codes() {
        let all = [
            Verdict::Clear,
            Verdict::RegionChange,
            Verdict::StructureCrossing,
            Verdict::OutsideWorld,
            Verdict::Blocked,
        ];
        assert_eq!(all.map(Verdict::code), [0, 1, 2, 3, 4]);
    }
}
