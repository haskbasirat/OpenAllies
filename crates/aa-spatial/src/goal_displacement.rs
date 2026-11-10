//! Native goal displacement `0x53CA30`, the second chance `0x53D340` gives a
//! candidate goal that `0x53CE40` refused ([`crate::gworld_placement::endpoint_acceptable`]).
//!
//! `0x53CA30(fallback x, fallback y, &candidate, descriptor)`, cdecl, returns
//! a byte. Native order:
//! 1. `0x5543F0(GMap, candidate)`: 0 unless `0 <= x < extent.x` and
//!    `0 <= y < extent.y` (the same x87 compares as `0x4F3C40`, see
//!    [`outside_world`]), else the `0x42A530` tile ([`GMapView::tile_at_point`]).
//! 2. `0x45BC70` on the tile's word +0x1C (generation-checked handle): its
//!    owner, or 0. It is called twice with the same tile; it only reads.
//! 3. The owner's virtual +0x100 must be nonzero, and `0x552340(descriptor,
//!    owner)` must not exclude it.
//! 4. The owner's facing words +0x2C/+0x30 are copied. The reference point
//!    is the candidate when the descriptor's source (+0x1C) is nonnull, its
//!    virtual +0xB4 holds, its word +0x70 (its Mind) is nonnull and that
//!    Mind's virtual +0xA4 holds; otherwise it is the fallback.
//! 5. `along = (ref.y - pos.y) * facing.y + stored(ref.x - pos.x) * facing.x`
//!    with the owner position +0x20/+0x24; only `ref.x - pos.x` is stored to
//!    binary32 (`0x53CAEF`). When `along` compares below `0x9F32DC` (binary32
//!    0.0; in the native an unordered compare does not negate: `test ah,5` +
//!    `jp`), both facing words are multiplied by `0x9FDC48` (binary32 -1.0)
//!    and stored.
//! 6. With the cell size at GMap +8: `x = stored(facing.x * cell + x)` (the
//!    product stays in the register) and `y = stored(stored(facing.y * cell)
//!    + y)`. Return 1; the caller tests the moved point with `0x53CE40` again.
//!
//! Divergence: [`Binary::from_f32_bits`] refuses a NaN or infinite word, so a
//! non-finite owner pose or candidate is an `Err` here where the native goes
//! on (with an unordered `along` it keeps the facing). Only reachable when a
//! tile has an owner and the data hold non-finite words.
//!
//! The tile word +0x1C has two writers. `0x54F6F0` (a thing joins the tile's
//! list) stores through `0x54ED80`; `0x54F780` (a thing leaves it) stores
//! directly (`mov [ebx+0x1c],0` at `0x54F79F`, then `mov [ebx+0x1c],ecx` at
//! `0x54F7E6` for the next list member). Both store zero or a thing whose
//! virtual +0xB4 holds, whose word +0x90 (its Wall component) is nonnull and
//! that `0x54EE60` places on the tile: the owner is a wall piece. Its virtual
//! +0x100 is then `0x4CED60`, the Structure (+0x94) +0x10 parent or the Wall's
//! `0x58CB30` parent.
use crate::gworld_placement::{outside_world, GMapView, Result, ZERO_BITS};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// `0x9FDC48`: binary32 -1.0, the factor that turns the owner's facing.
pub const NEGATIVE_ONE_BITS: u32 = 0xbf80_0000;

/// Leaves of `0x53CA30`. No defaults.
pub trait DisplacementWorld {
    /// A live object (the tile owner).
    type Object: Copy;
    /// `0x45BC70` on GMap tile `tile`'s word +0x1C: the live owner, `None`
    /// for a null or stale handle.
    fn tile_owner(&mut self, tile: usize) -> Result<Option<Self::Object>>;
    /// The owner's virtual +0x100 (no arguments) is nonzero.
    fn owner_virtual_100(&mut self, owner: Self::Object) -> Result<bool>;
    /// `0x552340(descriptor, owner)`: true excludes the owner.
    fn excludes(&mut self, owner: Self::Object) -> Result<bool>;
    /// The owner's binary32 words +0x20/+0x24 (position) and +0x2C/+0x30
    /// (facing).
    fn owner_pose(&mut self, owner: Self::Object) -> Result<OwnerPose>;
    /// Step 4: the descriptor's source passes virtual +0xB4 and its Mind's
    /// virtual +0xA4 (false for a null source or a null Mind).
    fn source_refers_to_candidate(&mut self) -> Result<bool>;
}

/// Owner words read by step 5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnerPose {
    pub position_bits: [u32; 2],
    pub facing_bits: [u32; 2],
}

fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn stored(value: &Binary) -> Result<u32> {
    Ok(value.store_f32()?)
}

/// `0x53CA30`: true when `candidate` was moved one cell along the owner's
/// facing; `candidate` is unchanged when it returns false.
pub fn displace_blocked<W: DisplacementWorld + ?Sized>(
    map: &GMapView,
    fallback: [u32; 2],
    candidate: &mut [u32; 2],
    p: Precision,
    world: &mut W,
) -> Result<bool> {
    if outside_world(*candidate, map.extent_bits) {
        return Ok(false);
    }
    let tile = map.tile_at_point(*candidate)?;
    let Some(owner) = world.tile_owner(tile)? else {
        return Ok(false);
    };
    if !world.owner_virtual_100(owner)? || world.excludes(owner)? {
        return Ok(false);
    }
    let pose = world.owner_pose(owner)?;
    let reference = if world.source_refers_to_candidate()? { *candidate } else { fallback };
    let [px, py] = pose.position_bits;
    let [fx, fy] = pose.facing_bits;
    let dx = stored(&b(reference[0])?.sub(&b(px)?, p)?)?;
    let along = b(reference[1])?
        .sub(&b(py)?, p)?
        .mul(&b(fy)?, p)?
        .add(&b(dx)?.mul(&b(fx)?, p)?, p)?;
    let mut facing = [fx, fy];
    if along.compare(&b(ZERO_BITS)?) == Ordering::Less {
        let turn = b(NEGATIVE_ONE_BITS)?;
        facing = [stored(&b(fx)?.mul(&turn, p)?)?, stored(&b(fy)?.mul(&turn, p)?)?];
    }
    let cell = b(map.cell_size_bits)?;
    let y_step = stored(&b(facing[1])?.mul(&cell, p)?)?;
    let x = stored(&b(facing[0])?.mul(&cell, p)?.add(&b(candidate[0])?, p)?)?;
    let y = stored(&b(y_step)?.add(&b(candidate[1])?, p)?)?;
    *candidate = [x, y];
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PC53: Precision = Precision::Double53;

    /// 16 x 16 tiles of 4.0 (shift 2), extent 64.0.
    fn map() -> GMapView {
        GMapView { width: 16, height: 16, coordinate_shift: 2, cell_size_bits: 4.0f32.to_bits(), extent_bits: [64.0f32.to_bits(); 2] }
    }
    fn bits(p: [f32; 2]) -> [u32; 2] {
        p.map(f32::to_bits)
    }

    #[derive(Default)]
    struct World {
        owner_tile: Option<usize>,
        virtual_100: bool,
        excluded: bool,
        pose: Option<OwnerPose>,
        refers: bool,
        calls: Vec<&'static str>,
    }
    impl DisplacementWorld for World {
        type Object = u32;
        fn tile_owner(&mut self, tile: usize) -> Result<Option<u32>> {
            self.calls.push("tile_owner");
            Ok((self.owner_tile == Some(tile)).then_some(7))
        }
        fn owner_virtual_100(&mut self, owner: u32) -> Result<bool> {
            assert_eq!(owner, 7);
            self.calls.push("virtual_100");
            Ok(self.virtual_100)
        }
        fn excludes(&mut self, _: u32) -> Result<bool> {
            self.calls.push("excludes");
            Ok(self.excluded)
        }
        fn owner_pose(&mut self, _: u32) -> Result<OwnerPose> {
            self.calls.push("pose");
            Ok(self.pose.expect("pose read"))
        }
        fn source_refers_to_candidate(&mut self) -> Result<bool> {
            self.calls.push("source");
            Ok(self.refers)
        }
    }
    fn wall(tile: usize, position: [f32; 2], facing: [f32; 2]) -> World {
        World { owner_tile: Some(tile), virtual_100: true, pose: Some(OwnerPose { position_bits: bits(position), facing_bits: bits(facing) }), ..World::default() }
    }

    #[test]
    fn moves_one_cell_towards_the_fallback_side() {
        // Tile (5, 5) = 85; wall at (22, 22) facing +x; fallback on its -x side.
        let mut w = wall(85, [22.0, 22.0], [1.0, 0.0]);
        let mut candidate = bits([21.5, 22.5]);
        assert!(displace_blocked(&map(), bits([2.0, 22.0]), &mut candidate, PC53, &mut w).unwrap());
        assert_eq!(candidate, bits([17.5, 22.5]));
        assert_eq!(w.calls, ["tile_owner", "virtual_100", "excludes", "pose", "source"]);
        // Fallback on the +x side: the facing is kept.
        let mut candidate = bits([21.5, 22.5]);
        assert!(displace_blocked(&map(), bits([40.0, 22.0]), &mut candidate, PC53, &mut wall(85, [22.0, 22.0], [1.0, 0.0])).unwrap());
        assert_eq!(candidate, bits([25.5, 22.5]));
    }

    #[test]
    fn the_mover_side_uses_the_candidate_as_reference() {
        // Candidate on the wall's +y side, fallback on its -y side.
        let mut w = wall(85, [22.0, 22.0], [0.0, 1.0]);
        w.refers = true;
        let mut candidate = bits([22.5, 23.0]);
        assert!(displace_blocked(&map(), bits([22.0, 2.0]), &mut candidate, PC53, &mut w).unwrap());
        assert_eq!(candidate, bits([22.5, 27.0]));
        let mut w = wall(85, [22.0, 22.0], [0.0, 1.0]);
        let mut candidate = bits([22.5, 23.0]);
        assert!(displace_blocked(&map(), bits([22.0, 2.0]), &mut candidate, PC53, &mut w).unwrap());
        assert_eq!(candidate, bits([22.5, 19.0]));
    }

    #[test]
    fn zero_along_keeps_the_facing() {
        // The reference lies on the wall's line: 0.0 is not below 0.0.
        let mut w = wall(85, [22.0, 22.0], [1.0, 0.0]);
        let mut candidate = bits([21.0, 21.0]);
        assert!(displace_blocked(&map(), bits([22.0, 40.0]), &mut candidate, PC53, &mut w).unwrap());
        assert_eq!(candidate, bits([25.0, 21.0]));
    }

    #[test]
    fn refusals_leave_the_candidate() {
        let start = bits([21.5, 22.5]);
        for (label, mut w, point) in [
            ("no owner", World::default(), start),
            ("other tile", wall(84, [22.0, 22.0], [1.0, 0.0]), start),
            ("virtual +0x100 zero", World { virtual_100: false, ..wall(85, [22.0, 22.0], [1.0, 0.0]) }, start),
            ("excluded", World { excluded: true, ..wall(85, [22.0, 22.0], [1.0, 0.0]) }, start),
            ("outside", wall(85, [22.0, 22.0], [1.0, 0.0]), bits([64.0, 22.5])),
            ("negative", wall(85, [22.0, 22.0], [1.0, 0.0]), bits([-0.5, 22.5])),
        ] {
            let mut candidate = point;
            assert!(!displace_blocked(&map(), bits([2.0, 22.0]), &mut candidate, PC53, &mut w).unwrap(), "{label}");
            assert_eq!(candidate, point, "{label}");
        }
        // -0.0 compares equal to 0.0: inside, tile 0.
        let mut w = wall(0, [1.0, 1.0], [1.0, 0.0]);
        let mut candidate = [0x8000_0000, 1.0f32.to_bits()];
        assert!(displace_blocked(&map(), bits([3.0, 1.0]), &mut candidate, PC53, &mut w).unwrap());
        assert_eq!(candidate, bits([4.0, 1.0]));
    }

    #[test]
    fn only_the_x_difference_is_stored() {
        // ref.x - pos.x = 2^-20 - 22 is exact in binary64 and rounds (a tie,
        // to even) to -22.0 in binary32; ref.y - pos.y = 22 - 2^-21 stays in
        // the register. With facing (1, 1) the stored x difference gives
        // along = -2^-21 (turned), the unstored one +2^-21 (kept).
        let tiny = |e: i32| 2f32.powi(e);
        let mut w = wall(85, [22.0, tiny(-21)], [1.0, 1.0]);
        let mut candidate = bits([21.5, 22.5]);
        assert!(displace_blocked(&map(), bits([tiny(-20), 22.0]), &mut candidate, PC53, &mut w).unwrap());
        assert_eq!(candidate, bits([17.5, 18.5]));
    }

    #[test]
    fn the_y_step_is_stored_before_the_add() {
        // Cell 3.0: facing.y * cell is not a binary32. x keeps its product
        // in the register; y stores it first (0x53CB44).
        let map = GMapView { cell_size_bits: 3.0f32.to_bits(), ..map() };
        let fy = f32::from_bits(0x3f2f_f645);
        let cy = f32::from_bits(0x41d7_309d);
        let expected_y = (f64::from((f64::from(fy) * 3.0) as f32) + f64::from(cy)) as f32;
        assert_ne!(expected_y, (f64::from(fy) * 3.0 + f64::from(cy)) as f32, "the case must tell the two apart");
        // Tile (5, 6) = 101; the fallback on the facing side keeps it.
        let mut w = wall(101, [0.0, 0.0], [0.5, fy]);
        let mut candidate = bits([21.5, cy]);
        assert!(displace_blocked(&map, bits([10.0, 10.0]), &mut candidate, PC53, &mut w).unwrap());
        assert_eq!(candidate, bits([23.0, expected_y]));
    }
}
