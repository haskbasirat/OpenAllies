//! Original aircraft Mind move-state helpers (`aai_move`, vtable `A326C0`):
//! the formation setter `516D70`, the leader follow `516DD0` and the state
//! activation `518010` (virtual +18).
//!
//! The state's `+4` is the Mind; `529C30` on the state yields the owner actor
//! whose flight component (`+BC`) receives the formation join `55EB30`, the
//! follow `55F160` and the path clear `55E1B0(flight+28)`
//! ([`crate::aircraft`]). State fields: `+C` status, `+14` followed GTID,
//! `+34` cruise value, `+38`/`+3C` followed position, `+48` formation byte.
//! Mind `+1D4` is the aircraft's group.

/// The fields and flight calls of the three helpers, in native order.
pub trait MoveStateWorld {
    type Error;
    type Group: Copy;
    type Leader: Copy;
    /// State `+48` byte store and read.
    fn set_formation_byte(&mut self, value: u8) -> Result<(), Self::Error>;
    fn formation_byte(&mut self) -> Result<u8, Self::Error>;
    /// Fact: Mind `+1D4` (null is `None`).
    fn mind_group(&mut self) -> Result<Option<Self::Group>, Self::Error>;
    /// `55EB30(group)` on the owner's flight component (`None` passes 0).
    fn join_formation(&mut self, group: Option<Self::Group>) -> Result<(), Self::Error>;
    /// `55F160(group, leader)` on the owner's flight component.
    fn follow_leader(&mut self, group: Self::Group, leader: Self::Leader) -> Result<(), Self::Error>;
    /// State `+C` word store.
    fn set_status(&mut self, status: u32) -> Result<(), Self::Error>;
    /// Fact: the owner's Mover (`+6C`) `+28` word.
    fn mover_word_28(&mut self) -> Result<u32, Self::Error>;
    /// State `+34` word store.
    fn set_cruise(&mut self, bits: u32) -> Result<(), Self::Error>;
    /// Fact: the state `+14` GTID through `[B364C0]` (generation match and a
    /// nonnull slot): the object's `+20`/`+24` words, `None` when stale.
    fn followed_position(&mut self) -> Result<Option<[u32; 2]>, Self::Error>;
    /// State `+38`/`+3C` stores.
    fn set_followed_position(&mut self, xy: [u32; 2]) -> Result<(), Self::Error>;
    /// `55E1B0` on the owner's flight `+28` path.
    fn clear_flight_path(&mut self) -> Result<(), Self::Error>;
}

/// `516D70(flag)`: `+48 = flag`; a set flag joins the Mind's group, or
/// clears `+48` when the Mind has none; a clear `+48` (re-read) leaves the
/// formation with `55EB30(0)`.
pub fn set_formation<W: MoveStateWorld + ?Sized>(world: &mut W, flag: u8) -> Result<(), W::Error> {
    world.set_formation_byte(flag)?;
    if flag != 0 {
        match world.mind_group()? {
            Some(group) => world.join_formation(Some(group))?,
            None => world.set_formation_byte(0)?,
        }
    }
    if world.formation_byte()? == 0 {
        world.join_formation(None)?;
    }
    Ok(())
}

/// `516DD0(leader)`: no leader clears `+48` and leaves the formation; else
/// `+48 = 1` and the flight follows the leader within the Mind's group, or
/// without a group leaves the formation and clears `+48` again.
pub fn follow<W: MoveStateWorld + ?Sized>(
    world: &mut W,
    leader: Option<W::Leader>,
) -> Result<(), W::Error> {
    let Some(leader) = leader else {
        world.set_formation_byte(0)?;
        return world.join_formation(None);
    };
    world.set_formation_byte(1)?;
    match world.mind_group()? {
        Some(group) => world.follow_leader(group, leader),
        None => {
            world.join_formation(None)?;
            world.set_formation_byte(0)
        }
    }
}

/// `518010` (state virtual +18): status 2, `+34` = Mover `+28`, a live `+14`
/// object's position into `+38`/`+3C`, the flight path cleared, the
/// formation left, then `+48 = 0`.
pub fn activate<W: MoveStateWorld + ?Sized>(world: &mut W) -> Result<(), W::Error> {
    world.set_status(2)?;
    let cruise = world.mover_word_28()?;
    world.set_cruise(cruise)?;
    if let Some(xy) = world.followed_position()? {
        world.set_followed_position(xy)?;
    }
    world.clear_flight_path()?;
    world.join_formation(None)?;
    world.set_formation_byte(0)
}

/// The reads and calls of the move-state Think `51A720` (virtual +C), in
/// native order. State fields beyond [`MoveStateWorld`]: `+10` byte (has a
/// destination), `+18`/`+1C`/`+20` destination, `+24` direction (passed by
/// address when the `+30` byte is set), `+40`/`+44` the last commanded point.
pub trait MoveThinkWorld {
    type Error;
    type Group: Copy;
    type Actor: Copy + PartialEq;
    /// `529E60` (the state Think base).
    fn think_base(&mut self) -> Result<(), Self::Error>;
    /// `529C30`: the owner actor.
    fn owner(&mut self) -> Result<Self::Actor, Self::Error>;
    /// Fact: state `+48` byte.
    fn formation_byte(&mut self) -> Result<u8, Self::Error>;
    /// Fact: Mind `+1D4` (null is `None`).
    fn mind_group(&mut self) -> Result<Option<Self::Group>, Self::Error>;
    /// `5685B0` on the group: its leader (null is `None`).
    fn group_leader(&mut self, group: Self::Group) -> Result<Option<Self::Actor>, Self::Error>;
    /// Fact: state `+14` (followed GTID).
    fn followed_gtid(&mut self) -> Result<u32, Self::Error>;
    /// `432E40(&state+14)`: the GTID lookup whose result is unused.
    fn resolve_followed(&mut self) -> Result<(), Self::Error>;
    /// Fact: `[B364C0]` resolves the GTID (generation match, nonnull slot).
    fn gtid_live(&mut self, gtid: u32) -> Result<bool, Self::Error>;
    /// `518A30(0)` on the state.
    fn follow_target(&mut self) -> Result<(), Self::Error>;
    /// Fact: state `+18`/`+1C`/`+20`.
    fn destination(&mut self) -> Result<[u32; 3], Self::Error>;
    /// State `+40`/`+44` stores.
    fn set_commanded(&mut self, xy: [u32; 2]) -> Result<(), Self::Error>;
    /// Fact: the owner's flight (`+BC`) `+90` byte and `+34` word.
    fn flight_byte_90(&mut self, actor: Self::Actor) -> Result<u8, Self::Error>;
    fn flight_word_34(&mut self, actor: Self::Actor) -> Result<u32, Self::Error>;
    /// `4CF1B0(actor)`: the actor's `+14` GTID.
    fn actor_gtid(&mut self, actor: Self::Actor) -> Result<u32, Self::Error>;
    /// `5685E0(&out, gtid, 1)` on the group: the slot offset.
    fn formation_offset(&mut self, group: Self::Group, gtid: u32) -> Result<[u32; 3], Self::Error>;
    /// Fact: state `+30` byte (a direction at `+24`).
    fn has_direction(&mut self) -> Result<u8, Self::Error>;
    /// `560F10(offset, destination, direction)` on the owner's flight; a set
    /// `with_direction` passes the address of state `+24`, else null.
    fn fly_formation(
        &mut self,
        actor: Self::Actor,
        offset: [u32; 3],
        destination: [u32; 3],
        with_direction: bool,
    ) -> Result<(), Self::Error>;
    /// Fact: state `+10` byte.
    fn has_destination(&mut self) -> Result<u8, Self::Error>;
    /// Fact: state `+34` word.
    fn cruise(&mut self) -> Result<u32, Self::Error>;
    /// The owner's flight `+14` and `+84` stores (both the cruise word).
    fn set_flight_speeds(&mut self, actor: Self::Actor, bits: u32) -> Result<(), Self::Error>;
    /// `560ED0(destination, direction)` on the owner's flight.
    fn fly_to(
        &mut self,
        actor: Self::Actor,
        destination: [u32; 3],
        with_direction: bool,
    ) -> Result<(), Self::Error>;
}

fn commanded<W: MoveThinkWorld + ?Sized>(world: &mut W) -> Result<(), W::Error> {
    let [x, y, _] = world.destination()?;
    world.set_commanded([x, y])
}

fn flight_free<W: MoveThinkWorld + ?Sized>(world: &mut W, actor: W::Actor) -> Result<bool, W::Error> {
    Ok(world.flight_byte_90(actor)? != 0 || world.flight_word_34(actor)? == 0)
}

/// `51A720`: the aircraft move-state Think.
///
/// In formation (`+48` set and a Mind group): the group leader `5685B0` is
/// compared with the owner. A leader with a followed GTID (`+14` nonzero)
/// runs `432E40`, `518A30(0)` and copies `+18`/`+1C` to `+40`/`+44`. When the
/// flight is free (`+90` set or `+34` zero) the leader then flies its own
/// slot: `560F10(5685E0(owner GTID, 1), destination, direction)`.
/// Otherwise: a live followed GTID runs `518A30(0)` and the copy; a free
/// flight with a destination (`+10`) gets the cruise `+34` in flight
/// `+14`/`+84`, `560ED0(destination, direction)` and the copy (re-read after
/// the call).
pub fn think<W: MoveThinkWorld + ?Sized>(world: &mut W) -> Result<(), W::Error> {
    world.think_base()?;
    let actor = world.owner()?;
    let group = if world.formation_byte()? != 0 { world.mind_group()? } else { None };
    if let Some(group) = group {
        let leader = world.group_leader(group)? == Some(actor);
        if leader && world.followed_gtid()? != 0 {
            world.resolve_followed()?;
            world.follow_target()?;
            commanded(world)?;
        }
        if flight_free(world, actor)? && leader {
            let gtid = world.actor_gtid(actor)?;
            let offset = world.formation_offset(group, gtid)?;
            let with_direction = world.has_direction()? != 0;
            let destination = world.destination()?;
            world.fly_formation(actor, offset, destination, with_direction)?;
        }
        return Ok(());
    }
    let gtid = world.followed_gtid()?;
    if world.gtid_live(gtid)? {
        world.follow_target()?;
        commanded(world)?;
    }
    if flight_free(world, actor)? && world.has_destination()? != 0 {
        let cruise = world.cruise()?;
        world.set_flight_speeds(actor, cruise)?;
        let with_direction = world.has_direction()? != 0;
        let destination = world.destination()?;
        world.fly_to(actor, destination, with_direction)?;
        commanded(world)?;
    }
    Ok(())
}
