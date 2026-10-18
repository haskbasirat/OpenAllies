//! Native Team4E68C0 and base4EFA40 for saved Init(NULL) from4FAE80.
//! Existing fresh-creation helpers assume a nonnull creation block and cannot
//! substitute for this path. Required spatial/placement/log services have no
//! default success. Earlier mutations survive false or an error.
use aa_actors::ActorId;
use aa_numeric::{Binary, Precision};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("saved Team Init requires service: {0}")]
    World(String),
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Each method reads or mutates the actual current owner. Team virtual+C is
///4E3E90, returning the constant grid context B3A8C4 without consuming args;
///46EE00 ignores that context and mutates its actual Thing+20 argument.
pub trait World {
    fn flags(&self, team: ActorId) -> Result<u32>;
    fn tile_size_bits(&self) -> Result<u32>;
    ///46EE00 on the actual Thing+20 vector; tile size is captured before
    /// virtual+C. The helper's rounding remains a required numeric service.
    fn snap_to_grid(&mut self, team: ActorId, tile_size_bits: u32, pc: Precision) -> Result<()>;
    ///4EF6A0 with the real null creation parameter; no fabricated block.
    fn check_placement_null(&mut self, team: ActorId, pc: Precision) -> Result<bool>;
    ///4EE4C0, including real Team virtual+B0=false and any reached spatial
    /// descriptor/insert/remove dependencies. This is a required service.
    fn update_index_membership(&mut self, team: ActorId) -> Result<()>;
    fn position_bits(&self, team: ActorId) -> Result<[u32; 3]>;
    ///4F0E50 returns extended ST0; the caller performs the binary32 store.
    fn terrain_height(&mut self, position: [u32; 3], flags: u32, pc: Precision) -> Result<Binary>;
    fn set_height_bits(&mut self, team: ActorId, bits: u32) -> Result<()>;
    ///Thing+18 name, not Team+3C/+40 display/localization fields.
    fn name_utf16(&self, team: ActorId) -> Result<&[u16]>;
    ///404F60 diagnostic after successful base Init. Its name snapshot matches
    ///4BEDF0's temporary CString reference; no Team payload is replaced.
    fn log_team_initialized(&mut self, team: ActorId, name_utf16: &[u16]) -> Result<()>;
}

///4EFA40 with NULL params, as passed by saved4FAE80. Nonnull creation block
/// writes are deliberately outside this API. No Team-specific fields are read.
pub fn base_init_null(world: &mut impl World, team: ActorId, pc: Precision) -> Result<bool> {
    if world.flags(team)? & (1 << 16) != 0 {
        let size = world.tile_size_bits()?;
        world.snap_to_grid(team, size, pc)?;
    }
    if !world.check_placement_null(team, pc)? {
        return Ok(false);
    }
    world.update_index_membership(team)?;
    let flags = world.flags(team)?;
    let height_flags = if flags & (1 << 21) != 0 {
        Some(0x123)
    } else if flags & (1 << 12) == 0 {
        Some(0x122)
    } else {
        None
    };
    if let Some(flags) = height_flags {
        let position = world.position_bits(team)?;
        let height = world.terrain_height(position, flags, pc)?.store_f32()?;
        world.set_height_bits(team, height)?;
    }
    Ok(true)
}

///Team virtual+E8=4E68C0. False placement returns before name/logging.
/// A later log failure cannot roll back snapping, membership or height.
pub fn init_saved(world: &mut impl World, team: ActorId, pc: Precision) -> Result<bool> {
    if !base_init_null(world, team, pc)? {
        return Ok(false);
    }
    let name = world.name_utf16(team)?.to_vec();
    world.log_team_initialized(team, &name)?;
    Ok(true)
}
