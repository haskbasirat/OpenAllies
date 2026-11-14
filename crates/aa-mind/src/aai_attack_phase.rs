//! Original aircraft attack-state phase step `51B4C0` (fastcall on the
//! `aai_attack` state, Mind+E8), run by the attack Think `51B8C0`
//! ([`crate::uai_thinks::aai_attack`]) for a busy phase without a target.
//!
//! Phase `+30` 1 sets the retarget byte `+14`. A set `+14` is cleared and the
//! phase advances: 0 or 4 -> 1, 1 or 2 -> 3, 3 -> 4, anything else -> 0. A
//! phase (re-read) other than 0/4 runs the plan `51A920`; 0 or 4 clears the
//! owner's flight `+10` byte and its path (`55E1B0(flight+28)`). Then, with
//! the phase (re-read again, after the plan) other than 0/4, the Mind's move
//! state (`+34`) Thinks (virtual +C, `51A720`). The Mind scheduler (`+8`
//! virtual +4) always gets `(1e-4, 0)`.

/// binary32 1e-4.
pub const IMMEDIATE_BITS: u32 = 0x38d1_b717;

/// The reads and calls of `51B4C0`, in native order.
pub trait AttackPhaseWorld {
    type Error;
    /// State `+30` read and store.
    fn phase(&mut self) -> Result<u32, Self::Error>;
    fn set_phase(&mut self, phase: u32) -> Result<(), Self::Error>;
    /// State `+14` byte read and store.
    fn retarget_byte(&mut self) -> Result<u8, Self::Error>;
    fn set_retarget_byte(&mut self, value: u8) -> Result<(), Self::Error>;
    /// `51A920` on the state.
    fn plan(&mut self) -> Result<(), Self::Error>;
    /// The owner's flight `+10` byte = 0, then `55E1B0(flight+28)`.
    fn clear_flight_byte_10(&mut self) -> Result<(), Self::Error>;
    fn clear_flight_path(&mut self) -> Result<(), Self::Error>;
    /// Mind `+34` move state virtual +C.
    fn move_think(&mut self) -> Result<(), Self::Error>;
    /// Mind `+8` scheduler virtual +4 `(delay, 0)`.
    fn schedule(&mut self, delay_bits: u32) -> Result<(), Self::Error>;
}

fn busy(phase: u32) -> bool {
    phase != 0 && phase != 4
}

/// `51B4C0`.
pub fn step<W: AttackPhaseWorld + ?Sized>(world: &mut W) -> Result<(), W::Error> {
    let phase = world.phase()?;
    if phase == 1 {
        world.set_retarget_byte(1)?;
    }
    if world.retarget_byte()? != 0 {
        world.set_retarget_byte(0)?;
        let next = match phase {
            0 | 4 => 1,
            1 | 2 => 3,
            3 => 4,
            _ => 0,
        };
        world.set_phase(next)?;
        if busy(world.phase()?) {
            world.plan()?;
        } else {
            world.clear_flight_byte_10()?;
            world.clear_flight_path()?;
        }
    }
    if busy(world.phase()?) {
        world.move_think()?;
    }
    world.schedule(IMMEDIATE_BITS)
}
