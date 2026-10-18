//! Remaining regiment (CAI) Mind-state Thinks, one submodule per Think in
//! `src/cai_thinks/` (owned by the regiment-thinks helper).
//!
//! Regiment embedded states (research/phase8-ai/inventory.json): sleep +34,
//! rampage +54, move +68, kill +C0, bombard +D4, siege +F8, chasing +120,
//! construct +134, repair +160, guard +17C, regroup +1A8, reform +1B8,
//! engage +1CC, retreat +1F0, rout +20C, explore +220, join +24C,
//! exhausted +270, unpack +280, pack +2A4.
use crate::{Arena, MindId, Result, StateId, World};

pub mod bombard;
pub mod construct;
pub mod explore;
pub mod join;
pub mod kill;
pub mod pack;
pub mod rampage;
pub mod reform;
pub mod repair;
pub mod siege;
pub mod siege_target;
pub mod stack_action;
pub mod unit_order;
pub mod unpack;

pub const SLEEP_OFFSET: u32 = 0x34;
pub const RAMPAGE_OFFSET: u32 = 0x54;
pub const MOVE_OFFSET: u32 = 0x68;
pub const KILL_OFFSET: u32 = 0xc0;
pub const BOMBARD_OFFSET: u32 = 0xd4;
pub const SIEGE_OFFSET: u32 = 0xf8;

/// Services shared by the regiment Thinks (`World` drives the Mind
/// primitives). Methods have no permissive defaults.
pub trait RegimentWorld: World {
    /// GTID registry (`[B364C0]`): the entry for a GTID whose generation
    /// matches, `None` for a mismatch or a null entry. No removed check.
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>>;
    /// Object+14 (its GTID).
    fn object_gtid(&mut self, object: u32) -> Result<u32>;
    /// Identity of the embedded state at Mind+`offset`.
    fn embedded_state(&mut self, mind: MindId, offset: u32) -> Result<StateId>;
    /// Virtual +C (Think) of the embedded state at Mind+`offset`.
    fn embedded_think(&mut self, mind: MindId, offset: u32, arena: &mut Arena) -> Result<()>;
    /// Mind virtual +140(gtid).
    fn mind_virtual_140(&mut self, mind: MindId, gtid: u32, arena: &mut Arena) -> Result<()>;
    /// Mind virtual +B8.
    fn mind_virtual_b8(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
}

/// `529C20` on the embedded state at Mind+`offset`: its virtual +1C (leave)
/// when its status is nonzero.
pub fn leave_active(
    world: &mut impl RegimentWorld,
    mind: MindId,
    offset: u32,
    arena: &mut Arena,
) -> Result<()> {
    let state = world.embedded_state(mind, offset)?;
    if arena.state(state)?.status != 0 {
        world.leave(state, arena)?;
    }
    Ok(())
}

/// `fld dword`/`fstp dword` of a binary32: a signalling NaN is quieted.
pub fn x87_copy(bits: u32) -> u32 {
    let nan = bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0;
    if nan { bits | 0x0040_0000 } else { bits }
}
