//! Group order handout of the group actor (`KKC_GGroup`): which command each
//! member is given and in what order.
//!
//! - `0x4DE7F0` ([`give`]): the member at `index` gets the group's command
//!   when it accepts it. Otherwise, when the command's type wants a position
//!   (`0x496F10`) or a target (`0x496EF0`), it is offered a type-5 Move to the
//!   command's position (command +C bit 2, `0x4CA760`) or a type-5 Follow of
//!   the command's target (`0x530F10` + `0x4DDE60`), with a zero aux vector.
//!   The formation flag (`0x4DDB60`) is stored before the delivery.
//! - `0x4DEA20` ([`take_places`]): one type-5 Move to `at`, re-aimed at each
//!   member's place (`0x4DE020`) and delivered member by member; the member
//!   count is read once, before the loop.
//! - `0x4DEE40` ([`rest`]): the group goes idle at `at`, reforms there when
//!   asked and nothing is queued, clears the member formation flags (count
//!   read again), removes its slot-7 zone and reschedules itself after 1e-4.
//!
//! The command constructors `0x4D9E20` (base, type 5), `0x4CA760` (Move) and
//! `0x4DDE60` / `0x4D9BF0` (Follow) are ported in [`Command`]. Proven against
//! research/phase8-orders-input/group/native_group_orders.py (the original
//! code with recorded seams for the world calls of [`GroupOrderWorld`]).
use aa_numeric::{Fast, Precision};
use serde::{Deserialize, Serialize};

/// Native 1e-4 (`0x38D1B717`), the delay `0x4DEE40` reschedules with.
pub const REST_RESCHEDULE_BITS: u32 = 0x38d1_b717;
/// The command type of the Move and Follow fallbacks.
pub const MOVE_KIND: u32 = 5;
/// The native zero vector at `B279C0` (the fallbacks' aux vector).
pub const ZERO_AUX: [u32; 2] = [0, 0];

/// The command words the handout reads and writes (+8 .. +28).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    /// +8: the command type's descriptor (`[[B36530]+298][type]`).
    pub descriptor: u32,
    /// +C: bit 0 target, bit 2 position, bit 3 aux, bit 4 word +28, bit 6 descriptor.
    pub flags: u32,
    /// +10: the target handle.
    pub target: u32,
    /// +14.
    pub word14: u32,
    /// +18, +1C: the position (binary32 words).
    pub position: [u32; 2],
    /// +20, +24: the aux vector (binary32 words).
    pub aux: [u32; 2],
    /// +28.
    pub word28: u32,
}

impl Command {
    /// `0x4D9E20`: a cleared command of the type whose descriptor is given.
    pub fn base(descriptor: u32) -> Self {
        Self {
            descriptor,
            flags: if descriptor != 0 { 0x40 } else { 0 },
            ..Self::default()
        }
    }
    /// `0x4CA760`: a type-5 Move to `position`.
    pub fn move_to(descriptor5: u32, position: [u32; 2], word28: u32, aux: [u32; 2]) -> Self {
        let mut c = Self::base(descriptor5);
        c.flags |= 4;
        c.position = position;
        c.flags |= 8;
        c.aux = aux;
        c.flags |= 0x10;
        c.word28 = word28;
        c
    }
    /// `0x4DDE60`: a type-5 Follow of the object whose own handle (its word
    /// +14) is `target`; None when `0x530F10` did not resolve it.
    pub fn follow(descriptor5: u32, target: Option<u32>, word28: u32, aux: [u32; 2]) -> Self {
        let mut c = Self::base(descriptor5);
        // 0x4D9BF0.
        match target {
            Some(handle) => {
                c.flags |= 1;
                c.target = handle;
            }
            None => {
                c.flags &= !1;
                c.target = 0;
            }
        }
        c.flags |= 8;
        c.aux = aux;
        c.flags |= 0x10;
        c.word28 = word28;
        c
    }
}

/// `0x496F10` on a command type mask: the type wants a position.
pub fn wants_position(mask: u32, only: bool) -> bool {
    (mask >> 8) & 0xff != 0 && (!only || mask & 0xffff_00f0 == 0)
}
/// `0x496EF0` on a command type mask: the type wants a target.
pub fn wants_target(mask: u32, only: bool) -> bool {
    mask & 0xf0 != 0 && (!only || mask & 0xffff_ff00 == 0)
}

/// The world calls of the handout, in the order the original makes them.
pub trait GroupOrderWorld {
    type Member: Copy;
    type Error;
    /// Group +E8.
    fn kingdom(&self) -> u32;
    /// Group +18C.
    fn member_count(&self) -> i32;
    /// `0x4DE1F0` for an index inside the list: the member's actor, None
    /// when its handle no longer resolves.
    fn member(&mut self, index: i32) -> Result<Option<Self::Member>, Self::Error>;
    /// Member virtual +150 (kingdom, command, 1).
    fn accepts(&mut self, member: Self::Member, kingdom: u32, command: &Command) -> Result<bool, Self::Error>;
    /// `0x4DDB60`.
    fn formation_flag(&mut self, member: Self::Member, command: &Command) -> Result<u8, Self::Error>;
    /// Member entry `index` byte +C.
    fn set_formation_flag(&mut self, index: i32, flag: u8) -> Result<(), Self::Error>;
    /// `0x4D9B60`: deliver `command` to `member`; its low byte.
    fn deliver(&mut self, command: &Command, member: Self::Member) -> Result<u8, Self::Error>;
    /// Descriptor +2C, the command type mask.
    fn command_mask(&self, descriptor: u32) -> Result<u32, Self::Error>;
    /// `[[B36530]+298][kind]`.
    fn command_descriptor(&self, kind: u32) -> Result<u32, Self::Error>;
    /// `0x530F10` on a command's target handle, then that object's word +14.
    fn target_handle(&mut self, handle: u32) -> Result<Option<u32>, Self::Error>;
    /// `0x4DE020`: the place of member entry `index` at `point`, relative to it.
    fn place(&mut self, index: i32, point: [u32; 2]) -> Result<[u32; 2], Self::Error>;
    /// Group +1B8 = 0.
    fn set_idle(&mut self) -> Result<(), Self::Error>;
    /// `0x4EEB40`.
    fn set_position(&mut self, at: [u32; 2]) -> Result<(), Self::Error>;
    /// Group +19C.
    fn queued(&self) -> u32;
    /// Group +14C > 7 and its slot 7 (+148 list word 7) is nonzero.
    fn has_zone7(&self) -> bool;
    /// `0x514CE0` on the slot-7 zone.
    fn remove_zone7(&mut self) -> Result<(), Self::Error>;
    /// Virtual +4 of the group's +180 scheduler (delay bits, word).
    fn schedule(&mut self, delay_bits: u32, word: u32) -> Result<(), Self::Error>;
    /// The numeric error of a binary32 sum.
    fn numeric(error: aa_numeric::Error) -> Self::Error;
}

/// `0x4DE7F0`: the low byte of its return.
pub fn give<W: GroupOrderWorld>(world: &mut W, index: i32, command: &Command) -> Result<u8, W::Error> {
    let Some(member) = world.member(index)? else {
        return Ok(0);
    };
    let kingdom = world.kingdom();
    if world.accepts(member, kingdom, command)? {
        let flag = world.formation_flag(member, command)?;
        world.set_formation_flag(index, flag)?;
        return world.deliver(command, member);
    }
    let mask = world.command_mask(command.descriptor)?;
    if !wants_position(mask, false) && !wants_target(mask, false) {
        return Ok(0);
    }
    let descriptor5 = world.command_descriptor(MOVE_KIND)?;
    let fallback = if command.flags >> 2 & 1 != 0 {
        Command::move_to(descriptor5, command.position, 0, ZERO_AUX)
    } else {
        let target = world.target_handle(command.target)?;
        Command::follow(descriptor5, target, 0, ZERO_AUX)
    };
    if world.accepts(member, kingdom, &fallback)? {
        let flag = world.formation_flag(member, &fallback)?;
        world.set_formation_flag(index, flag)?;
        return world.deliver(&fallback, member);
    }
    Ok(0)
}

/// `0x4DEA20`: each member takes its place at `at` (binary32 words).
pub fn take_places<W: GroupOrderWorld>(world: &mut W, at: [u32; 2], p: Precision) -> Result<(), W::Error> {
    let descriptor5 = world.command_descriptor(MOVE_KIND)?;
    let mut command = Command::move_to(descriptor5, at, 0, ZERO_AUX);
    let count = world.member_count();
    let load = |bits: u32| Fast::from_f32_bits(bits).map_err(W::numeric);
    for index in 0..count {
        let Some(member) = world.member(index)? else {
            continue;
        };
        let place = world.place(index, at)?;
        command.flags |= 4;
        let x = load(at[0])?.add(&load(place[0])?, p).and_then(|v| v.store_f32());
        let y = load(place[1])?.add(&load(at[1])?, p).and_then(|v| v.store_f32());
        command.position = [x.map_err(W::numeric)?, y.map_err(W::numeric)?];
        let kingdom = world.kingdom();
        if world.accepts(member, kingdom, &command)? {
            world.deliver(&command, member)?;
        }
    }
    Ok(())
}

/// `0x4DEE40`: the group rests at `at`; returns its byte (always 1).
pub fn rest<W: GroupOrderWorld>(world: &mut W, at: [u32; 2], reform: bool, p: Precision) -> Result<u8, W::Error> {
    world.set_idle()?;
    world.set_position(at)?;
    if world.queued() == 0 && reform {
        take_places(world, at, p)?;
    }
    for index in 0..world.member_count() {
        world.set_formation_flag(index, 0)?;
    }
    if world.has_zone7() {
        world.remove_zone7()?;
    }
    world.schedule(REST_RESCHEDULE_BITS, 0)?;
    Ok(1)
}
