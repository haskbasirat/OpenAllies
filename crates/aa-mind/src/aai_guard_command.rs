//! Original aircraft Mind guard/escort command `518850` (aircraft Mind
//! virtual +C8, vtable `A32710`; thiscall `(command)`,
//! `ret 4`).
//!
//! AL 0 unless the owner has a Performer (`+74`), the command's `+C` bit 0 is
//! set and its `+10` GTID resolves through `[B364C0]`. Then:
//! 1. When the Mind's home (`+28` GTID) resolves and the home object
//!    (`4D09A0`) has the `+14` GTID of the command's object (`530F10`): an
//!    escort already current (`52A000` on Mind `+9C`) of that object (Mind
//!    `+AC`) answers 0; otherwise the escort state is activated,
//!    `516FD0(2, object, 0)` on Mind `+9C` ([`crate::aai_guard_activate`]),
//!    and its AL returned.
//! 2. Otherwise, with a command object, a home object (`4D09A0`) and its
//!    Denizen (`+88`), the Denizen's leader for the object's GTID (`4CF1B0`,
//!    `568ED0`) is guarded: a guard already current (`52A000` on Mind `+D0`)
//!    with that leader (Mind `+E0`) answers 0, otherwise `518170(2, leader)`
//!    on Mind `+D0` and its AL. Any missing piece answers 0.

/// The reads and calls of `518850`, in native order.
pub trait GuardCommandWorld {
    type Error;
    type Object: Copy;
    type Leader: Copy + PartialEq;
    /// Fact: owner `+74` (Performer) is nonnull.
    fn has_performer(&mut self) -> Result<bool, Self::Error>;
    /// Fact: command `+C` bit 0.
    fn command_flag(&mut self) -> Result<bool, Self::Error>;
    /// Fact: command `+10` and Mind `+28` GTIDs.
    fn command_gtid(&mut self) -> Result<u32, Self::Error>;
    fn home_gtid(&mut self) -> Result<u32, Self::Error>;
    /// Fact: `[B364C0]` resolves the GTID (generation match, nonnull slot).
    fn gtid_live(&mut self, gtid: u32) -> Result<bool, Self::Error>;
    /// `4D09A0` on the Mind (null is `None`).
    fn home_object(&mut self) -> Result<Option<Self::Object>, Self::Error>;
    /// `530F10` on the command (null is `None`).
    fn command_object(&mut self) -> Result<Option<Self::Object>, Self::Error>;
    /// Fact: object `+14` GTID (`4CF1B0` for a nonnull object).
    fn object_gtid(&mut self, object: Self::Object) -> Result<u32, Self::Error>;
    /// `52A000` on Mind `+9C` / `+D0`.
    fn escort_current(&mut self) -> Result<bool, Self::Error>;
    fn guard_current(&mut self) -> Result<bool, Self::Error>;
    /// Fact: Mind `+AC` (escorted GTID) and `+E0` (guarded leader).
    fn escorted_gtid(&mut self) -> Result<u32, Self::Error>;
    fn guarded_leader(&mut self) -> Result<Option<Self::Leader>, Self::Error>;
    /// `516FD0(mode, object, formation)` on Mind `+9C`: AL.
    fn activate_escort(
        &mut self,
        mode: u32,
        object: Option<Self::Object>,
        formation: u8,
    ) -> Result<u8, Self::Error>;
    /// Fact: object `+88` (Denizen) is nonnull.
    fn has_denizen(&mut self, object: Self::Object) -> Result<bool, Self::Error>;
    /// `568ED0(gtid)` on the object's Denizen (null is `None`).
    fn denizen_leader(
        &mut self,
        object: Self::Object,
        gtid: u32,
    ) -> Result<Option<Self::Leader>, Self::Error>;
    /// The original dereferences a null home or command object in step 1:
    /// the error to return instead.
    fn null_object(&mut self) -> Self::Error;
    /// `518170(mode, leader)` on Mind `+D0`: AL.
    fn activate_guard(&mut self, mode: u32, leader: Option<Self::Leader>) -> Result<u8, Self::Error>;
}

/// `518850`: the returned AL.
pub fn command<W: GuardCommandWorld + ?Sized>(world: &mut W) -> Result<u8, W::Error> {
    if !world.has_performer()? || !world.command_flag()? {
        return Ok(0);
    }
    let gtid = world.command_gtid()?;
    if !world.gtid_live(gtid)? {
        return Ok(0);
    }
    let home = world.home_gtid()?;
    if world.gtid_live(home)? {
        let home_gtid = match world.home_object()? {
            Some(object) => world.object_gtid(object)?,
            None => return Err(world.null_object()),
        };
        let target = world.command_object()?;
        let target_gtid = match target {
            Some(object) => world.object_gtid(object)?,
            None => return Err(world.null_object()),
        };
        if target_gtid == home_gtid {
            if world.escort_current()? {
                let escorted = world.escorted_gtid()?;
                let again = world.command_object()?;
                let again_gtid = match again {
                    Some(object) => world.object_gtid(object)?,
                    None => return Err(world.null_object()),
                };
                if escorted == again_gtid {
                    return Ok(0);
                }
            }
            let object = world.command_object()?;
            return world.activate_escort(2, object, 0);
        }
    }
    let Some(target) = world.command_object()? else { return Ok(0) };
    let Some(home) = world.home_object()? else { return Ok(0) };
    if !world.has_denizen(home)? {
        return Ok(0);
    }
    let gtid = world.object_gtid(target)?;
    let leader = world.denizen_leader(home, gtid)?;
    if world.guard_current()? && world.guarded_leader()? == leader {
        return Ok(0);
    }
    world.activate_guard(2, leader)
}
