//! Original aircraft escort and guard state activations `516FD0` (thiscall
//! `(mode, target, formation)`, `ret 0xC`) and `518170` (thiscall
//! `(mode, leader)`, `ret 8`).
//!
//! `516FD0`: the state `+10` becomes the target's `+14` GTID (0 without a
//! target). When that GTID resolves through `[B364C0]` (generation match and
//! a nonnull slot), the Mind's move state (`[state+4]+34`) gets
//! `516D70(formation)` (aa-movement `aircraft_move_state::set_formation`)
//! and the state is activated with `52A810(mode)`, whose AL is returned;
//! otherwise AL 0 is returned and nothing else happens.
//!
//! `518170`: the state `+10` holds a reference-counted leader object (count
//! `+4`, virtual 0 the deleting destructor). A different leader is
//! referenced first, then the old one is released and, when its count
//! reaches zero, destroyed with `(1)`; `+10` becomes the new leader. Then
//! the move state gets `516DD0(+10)` and the state `52A810(mode)`.

/// The reads and calls of `516FD0`, in native order.
pub trait EscortActivateWorld {
    type Error;
    type Object: Copy;
    /// Fact: the target's `+14` GTID.
    fn object_gtid(&mut self, object: Self::Object) -> Result<u32, Self::Error>;
    /// State `+10` store.
    fn set_word_10(&mut self, gtid: u32) -> Result<(), Self::Error>;
    /// Fact: `[B364C0]` resolves the GTID (generation match, nonnull slot).
    fn gtid_live(&mut self, gtid: u32) -> Result<bool, Self::Error>;
    /// `516D70(formation)` on the Mind's `+34` move state.
    fn move_state_set_formation(&mut self, formation: u8) -> Result<(), Self::Error>;
    /// `52A810(mode)` on this state: AL.
    fn activate(&mut self, mode: u32) -> Result<u8, Self::Error>;
}

/// `516FD0`: the returned AL.
pub fn activate_escort<W: EscortActivateWorld + ?Sized>(
    world: &mut W,
    mode: u32,
    target: Option<W::Object>,
    formation: u8,
) -> Result<u8, W::Error> {
    let gtid = match target {
        Some(object) => world.object_gtid(object)?,
        None => 0,
    };
    world.set_word_10(gtid)?;
    if !world.gtid_live(gtid)? {
        return Ok(0);
    }
    world.move_state_set_formation(formation)?;
    world.activate(mode)
}

/// The reads and calls of `518170`, in native order.
pub trait GuardActivateWorld {
    type Error;
    type Leader: Copy + PartialEq;
    /// Fact: state `+10`.
    fn word_10(&mut self) -> Result<Option<Self::Leader>, Self::Error>;
    /// Leader `+4 += 1`.
    fn add_ref(&mut self, leader: Self::Leader) -> Result<(), Self::Error>;
    /// Leader `+4 -= 1`: the new count.
    fn release(&mut self, leader: Self::Leader) -> Result<i32, Self::Error>;
    /// Leader virtual 0 `(1)`.
    fn destroy(&mut self, leader: Self::Leader) -> Result<(), Self::Error>;
    /// State `+10` store.
    fn set_word_10(&mut self, leader: Option<Self::Leader>) -> Result<(), Self::Error>;
    /// `516DD0(leader)` on the Mind's `+34` move state.
    fn move_state_follow(&mut self, leader: Option<Self::Leader>) -> Result<(), Self::Error>;
    /// `52A810(mode)` on this state.
    fn activate(&mut self, mode: u32) -> Result<(), Self::Error>;
}

/// `518170`.
pub fn activate_guard<W: GuardActivateWorld + ?Sized>(
    world: &mut W,
    mode: u32,
    leader: Option<W::Leader>,
) -> Result<(), W::Error> {
    let old = world.word_10()?;
    if old != leader {
        if let Some(new) = leader {
            world.add_ref(new)?;
        }
        if let Some(old) = old
            && world.release(old)? == 0
        {
            world.destroy(old)?;
        }
        world.set_word_10(leader)?;
    }
    let current = world.word_10()?;
    world.move_state_follow(current)?;
    world.activate(mode)
}
