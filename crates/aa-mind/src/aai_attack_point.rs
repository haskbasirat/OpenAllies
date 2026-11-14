//! Original aircraft attack-state point activation `517360` (thiscall on the
//! `aai_attack` state, `(mode, x, y, formation)`, `ret 0x10`): attack a map
//! point.
//!
//! AL 0 when the owner (`529C30`) has no Performer (`+74`). Otherwise the
//! point is kept when the map (`[B36534]+30`) holds it (`4282A0`) and clamped
//! in place (`552640`) when not; `+24`/`+28` = the point, `+2C` = 0 and then
//! `+2C` = binary32 of the ground height `4F0E50(&point, 0x123)`; `+3C`/`+40`
//! = `+24`/`+28`; the Mind's move state (`+34`) gets `516D70(formation)` and
//! `516BA0(+24, +28, null)`; `+20` = 0, `+14` = 0, `+30` = 0; the state is
//! activated with `52A810(mode)`, whose AL is returned.
use crate::Result;
use crate::x87::store;
use aa_numeric::Binary;

/// The reads and calls of `517360`, in native order.
pub trait AttackPointWorld {
    /// Fact: the owner's `+74` (Performer) is nonnull.
    fn has_performer(&mut self) -> Result<bool>;
    /// `4282A0(x, y)` on the map: AL.
    fn map_contains(&mut self, xy: [u32; 2]) -> Result<bool>;
    /// `552640(&xy)` on the map: the clamped point.
    fn map_clamp(&mut self, xy: [u32; 2]) -> Result<[u32; 2]>;
    /// State `+24`/`+28`/`+2C` stores.
    fn set_point(&mut self, point: [u32; 3]) -> Result<()>;
    /// `4F0E50(&xy, 0x123)` on GWorld (ST0).
    fn ground_height(&mut self, xy: [u32; 2]) -> Result<Binary>;
    /// State `+3C`/`+40` stores.
    fn set_previous_point(&mut self, xy: [u32; 2]) -> Result<()>;
    /// `516D70(formation)` and `516BA0(x, y, null)` on the Mind's `+34`
    /// move state.
    fn move_state_set_formation(&mut self, formation: u8) -> Result<()>;
    fn move_state_retarget(&mut self, xy: [u32; 2]) -> Result<()>;
    /// State `+20` byte, `+14` byte and `+30` word cleared.
    fn clear_flags(&mut self) -> Result<()>;
    /// `52A810(mode)` on the state: AL.
    fn activate(&mut self, mode: u32) -> Result<u8>;
}

/// `517360`: the returned AL.
pub fn attack_point<W: AttackPointWorld + ?Sized>(
    world: &mut W,
    mode: u32,
    point: [u32; 2],
    formation: u8,
) -> Result<u8> {
    if !world.has_performer()? {
        return Ok(0);
    }
    let mut xy = point;
    if !world.map_contains(xy)? {
        xy = world.map_clamp(xy)?;
    }
    world.set_point([xy[0], xy[1], 0])?;
    let height = store(world.ground_height(xy)?)?;
    world.set_point([xy[0], xy[1], height])?;
    world.set_previous_point(xy)?;
    world.move_state_set_formation(formation)?;
    world.move_state_retarget(xy)?;
    world.clear_flags()?;
    world.activate(mode)
}
