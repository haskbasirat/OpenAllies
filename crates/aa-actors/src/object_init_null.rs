//! Saved GObject Init(NULL): 4EBE50, 4EE700, 4EFA40 and 4EF6A0's NULL path.
//!
//! This coordinator requires real host services; it is not a placement or
//! visibility implementation. It never constructs creation parameters, replaces
//! loaded +58, or substitutes Team virtuals. Evidence and explicit finite numeric
//! domain: research/skirmish-20261016-object-init-null/README.txt.

use crate::{
    ActorId,
    actor_creation::{Error, Result, Shape},
};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// Mandatory access to the current object, including a still-loading record.
/// Reads must fail for absent/uninitialized storage, not synthesize zero. Raw
/// word/definition accessors read owned storage without external calls. Service
/// callbacks may mutate owned state; the coordinator rereads at native boundaries.
pub trait World {
    fn word(&mut self, actor: ActorId, offset: u32) -> Result<u32>;
    fn set_word(&mut self, actor: ActorId, offset: u32, value: u32) -> Result<()>;
    /// Store only byte +4C; adjacent saved bytes are preserved.
    fn clear_byte_4c(&mut self, actor: ActorId) -> Result<()>;
    fn definition_height_bits(&mut self, actor: ActorId) -> Result<u32>;
    fn tile_size_bits(&mut self) -> Result<u32>;
    fn tile_shift(&mut self) -> Result<u32>;
    /// Actual virtual +4 (Actor/Kingdom: 4D1E40), including its extended return.
    fn radius(&mut self, actor: ActorId) -> Result<Binary>;
    /// Actual virtual +C; this is called separately at every native dispatch.
    fn shape(&mut self, actor: ActorId) -> Result<Shape>;
    fn thickness_warning(&mut self, actor: ActorId) -> Result<()>;
    /// 46EE00 on the owned position and actual shape context. Tile is captured
    /// before virtual +C; the service must preserve the native x87 precision.
    fn snap(&mut self, actor: ActorId, shape: Shape, tile_bits: u32, pc: Precision) -> Result<()>;
    /// GWorld 4F4590(actor, 0, 0, 0, 0), never GACD placement 4F3640.
    fn placement_null(&mut self, actor: ActorId) -> Result<u32>;
    fn placement_failure(&mut self, actor: ActorId, code: u32) -> Result<()>;
    /// 4F0460 reads the current optional world +28 configuration and its +A8.
    fn map_mode(&mut self) -> Result<Option<u32>>;
    fn placement_map_warning(&mut self, actor: ActorId) -> Result<()>;
    /// Entire 4EE4C0 service, including actual +B0 and descriptor dispatches.
    fn spatial_membership(&mut self, actor: ActorId) -> Result<()>;
    /// 4F0E50 receives the live position address; service reads owned coordinates.
    fn terrain_height(&mut self, actor: ActorId, flags: u32) -> Result<Binary>;
    /// Entire 4EB6E0, including its own flag gate and real spatial notifications.
    fn blocking_notify(&mut self, actor: ActorId) -> Result<()>;
    fn tile_footprint(&mut self, actor: ActorId) -> Result<()>;
    /// Entire 4F1F70; includes its flag8 gate and list/backlink ownership.
    fn world_list_insert(&mut self, actor: ActorId) -> Result<()>;
    /// Actual +8C; Actor/Kingdom 4EA3D0 tests signed loaded +58 > 0.
    fn has_instances(&mut self, actor: ActorId) -> Result<bool>;
    /// Actual 4EB490, with the current class's +A8 (Kingdom differs from Actor).
    fn retain_instances(&mut self, actor: ActorId) -> Result<bool>;
    /// Entire 4EBB80; owns SAI notifications, instance destruction and +58/+5C.
    fn clear_instances(&mut self, actor: ActorId) -> Result<()>;
    /// Entire 4EBC70 with real visibility, majors and instance ownership.
    fn visibility_refresh(&mut self, actor: ActorId) -> Result<()>;
}

fn f32b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn ftol(value: &Binary) -> Result<i32> {
    value
        .truncate_i32()
        .map_err(|_| Error::Domain("Object Init _ftol2 outside i32"))
}

/// Execute the saved-object prefix. False retains completed writes; a missing
/// mandatory service returns its error without rolling back earlier effects.
pub fn initialize<W: World>(actor: ActorId, pc: Precision, world: &mut W) -> Result<bool> {
    // Native captures the definition pointer, loads scale, multiplies by height,
    // clears byte 4C, then
    // stores the binary32 product at +38; do not conflate this with fresh params.
    let scale = world.word(actor, 0x3c)?;
    let height = world.definition_height_bits(actor)?;
    let product = f32b(scale)?.mul(&f32b(height)?, pc)?;
    world.clear_byte_4c(actor)?;
    world.set_word(actor, 0x38, product.store_f32()?)?;
    let quarter_tile = Binary::from_i32(ftol(&f32b(world.tile_size_bits()?)?)?)
        .mul(&f32b(0x3e80_0000)?, pc)?
        .store_f32()?;
    // 4EE700 rereads tile size only after the flags gate.
    if world.word(actor, 8)? & 0x2000_0400 == 0x2000_0400 {
        let tile = ftol(&f32b(world.tile_size_bits()?)?)?;
        let radius4 = world.radius(actor)?.mul(&f32b(0x4080_0000)?, pc)?;
        if Binary::from_i32(tile).compare(&radius4) != Ordering::Greater
            && matches!(world.shape(actor)?.kind, 2 | 3)
        {
            let shape = world.shape(actor)?;
            let thickness = if matches!(shape.kind, 2 | 3) {
                let a = f32b(shape.extent_10_bits)?;
                let b = f32b(shape.extent_14_bits)?;
                if a.compare(&b) == Ordering::Less {
                    a
                } else {
                    b
                }
            } else {
                f32b(shape.radius_bits)?
            };
            if thickness.compare(&f32b(quarter_tile)?) == Ordering::Less {
                world.thickness_warning(actor)?;
            }
        }
    }
    if !place_null(actor, pc, world)? {
        return Ok(false);
    }
    world.blocking_notify(actor)?;
    world.tile_footprint(actor)?;
    // Native captures both coordinates before shift; stores neither tile index
    // until both _ftol2 calls have completed.
    let x = world.word(actor, 0x20)?;
    let y = world.word(actor, 0x24)?;
    let shift = world.tile_shift()? & 31;
    let x = ftol(&f32b(x)?)? >> shift;
    let y = ftol(&f32b(y)?)? >> shift;
    world.set_word(actor, 0x44, x as u32)?;
    world.set_word(actor, 0x48, y as u32)?;
    world.world_list_insert(actor)?;
    if world.has_instances(actor)? && !world.retain_instances(actor)? {
        world.clear_instances(actor)?;
    }
    world.visibility_refresh(actor)?;
    Ok(true)
}

/// NULL skips all creation-block reads/writes in 4EFA40 and 4EF6A0.
fn place_null<W: World>(actor: ActorId, pc: Precision, world: &mut W) -> Result<bool> {
    if world.word(actor, 8)? & (1 << 16) != 0 {
        let tile = world.tile_size_bits()?;
        let shape = world.shape(actor)?;
        world.snap(actor, shape, tile, pc)?;
    }
    let code = world.placement_null(actor)?;
    if code != 0 {
        world.placement_failure(actor, code)?;
        if world.map_mode()? == Some(2) && matches!(code, 1 | 2) {
            world.placement_map_warning(actor)?;
        } else {
            return Ok(false);
        }
    }
    world.spatial_membership(actor)?;
    let flags = world.word(actor, 8)?;
    let height_flags = if flags & (1 << 21) != 0 {
        Some(0x123)
    } else if flags & (1 << 12) == 0 {
        Some(0x122)
    } else {
        None
    };
    if let Some(flags) = height_flags {
        let height = world.terrain_height(actor, flags)?;
        world.set_word(actor, 0x28, height.store_f32()?)?;
    }
    Ok(true)
}
