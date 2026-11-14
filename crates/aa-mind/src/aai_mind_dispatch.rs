//! Original aircraft Mind dispatcher `51C6F0` (aircraft Mind virtual +140,
//! thiscall with one unused argument, `ret 4`) and the guard/escort state
//! exits `51C8C0` and `517190` (state virtual +1C).
//!
//! Mind states named by offset: `+9C` (status `+A8`, word `+B0` = its `+14`),
//! `+E8` the attack state (`aai_attack`, status `+F4`, phase `+118` = its
//! `+30`) and `+12C`. `52A000` (the state is the stack head of its `+4`
//! owner Mind),
//! `529C20` (leave a state whose `+C` status is nonzero) and `52A340` (pop
//! the head state) are the Mind primitives; the Mind `+8` scheduler's
//! virtual +4 takes `(delay, 0)`.
//!
//! `51C6F0`:
//! 1. `+9C` current: when Mind `+F4` is 2, `+B0 = 0`, and when the `+9C`
//!    state's owner (`[Mind+A0]`) has its own `+F4` 2 and `+118` 0 or 4, its
//!    `+E8` state is left (`529C20`). Schedules 0.6 s; returns 0.
//! 2. `+E8` current: schedules 1e-4 s; returns 1.
//! 3. `+12C` current: `52A340`, schedules 0.6 s; returns 1.
//! 4. Otherwise schedules 0.6 s; returns 0.
//!
//! `51C8C0` (state at `+9C` class): `+14 = 0`; owner (`+4`) `+F4` 2 with
//! `+118` 0 or 4 leaves the owner's `+E8` state; `+C = 0`.
//! `517190`: owner `+F4` 2 leaves the owner's `+E8` state; `+14 = 0`,
//! `+C = 0`.

/// binary32 0.6 and 1e-4, the two scheduler delays.
pub const POINT_SIX_BITS: u32 = 0x3f19_999a;
pub const IMMEDIATE_BITS: u32 = 0x38d1_b717;

/// Mind states `51C6F0` tests, by offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MindState {
    At9C,
    AtE8,
    At12C,
}

/// The reads and calls of `51C6F0`, in native order.
pub trait DispatchWorld {
    type Error;
    /// `52A000` on the Mind state.
    fn is_current(&mut self, state: MindState) -> Result<bool, Self::Error>;
    /// Fact: Mind `+F4`.
    fn attack_status(&mut self) -> Result<u32, Self::Error>;
    /// Mind `+B0` store (the `+9C` state's `+14`).
    fn clear_b0(&mut self) -> Result<(), Self::Error>;
    /// Fact: the `+9C` state's owner (`[Mind+A0]`) `+F4` and `+118`.
    fn owner_attack_status_phase(&mut self) -> Result<(u32, u32), Self::Error>;
    /// `529C20` on the owner's `+E8` state.
    fn leave_owner_attack(&mut self) -> Result<(), Self::Error>;
    /// `52A340` on the Mind.
    fn pop_state(&mut self) -> Result<(), Self::Error>;
    /// Mind `+8` scheduler virtual +4 `(delay, 0)`.
    fn schedule(&mut self, delay_bits: u32) -> Result<(), Self::Error>;
}

/// `51C6F0`: the returned AL.
pub fn dispatch<W: DispatchWorld + ?Sized>(world: &mut W) -> Result<u8, W::Error> {
    if world.is_current(MindState::At9C)? {
        if world.attack_status()? == 2 {
            world.clear_b0()?;
            let (status, phase) = world.owner_attack_status_phase()?;
            if status == 2 && (phase == 0 || phase == 4) {
                world.leave_owner_attack()?;
            }
        }
        world.schedule(POINT_SIX_BITS)?;
        return Ok(0);
    }
    if world.is_current(MindState::AtE8)? {
        world.schedule(IMMEDIATE_BITS)?;
        return Ok(1);
    }
    if world.is_current(MindState::At12C)? {
        world.pop_state()?;
        world.schedule(POINT_SIX_BITS)?;
        return Ok(1);
    }
    world.schedule(POINT_SIX_BITS)?;
    Ok(0)
}

/// The state words and owner reads of the two state exits.
pub trait StateExitWorld {
    type Error;
    /// State `+14` and `+C` stores.
    fn set_word_14(&mut self, value: u32) -> Result<(), Self::Error>;
    fn set_status(&mut self, value: u32) -> Result<(), Self::Error>;
    /// Fact: the state's owner (`+4`) `+F4` and `+118`.
    fn owner_attack_status_phase(&mut self) -> Result<(u32, u32), Self::Error>;
    /// `529C20` on the owner's `+E8` state.
    fn leave_owner_attack(&mut self) -> Result<(), Self::Error>;
}

/// `51C8C0`.
pub fn exit_51c8c0<W: StateExitWorld + ?Sized>(world: &mut W) -> Result<(), W::Error> {
    world.set_word_14(0)?;
    let (status, phase) = world.owner_attack_status_phase()?;
    if status == 2 && (phase == 0 || phase == 4) {
        world.leave_owner_attack()?;
    }
    world.set_status(0)
}

/// `517190`.
pub fn exit_517190<W: StateExitWorld + ?Sized>(world: &mut W) -> Result<(), W::Error> {
    let (status, _) = world.owner_attack_status_phase()?;
    if status == 2 {
        world.leave_owner_attack()?;
    }
    world.set_word_14(0)?;
    world.set_status(0)
}
