//! Original Settlement component Tick `0x587290(controller)`: the periodic
//! call `0x586150`, the damaged-member tasks and the wall piece tasks.
//!
//! The branch order, every reread of component and actor memory, the x87
//! interval sum and the per-piece rotation follow the original
//! instructions; `0x529C30` (the settlement actor's definition) and
//! `0x471A40` (a style's definition by kind) are ported inline. The task
//! holder calls `0x568040` / `0x56A6B0` / `0x56B590`, the periodic
//! `0x586150` and the wall calls CreateFullWall `0x587210`, the piece
//! creation `0x586DB0`, UpgradeFullWall `0x585C60` and the piece upgrade
//! `0x585BF0` are required world callbacks in native order. Proven by
//! research/phase8-combat/native_settlement_tick.py.
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// A native object (pointer in the original); 0 is null.
pub type Ref = u32;

/// `[A30DE0]`: seconds between the `0x586150` calls.
pub const PERIOD_BITS: u32 = 0x4120_0000;
/// Settlement actor +0x100 bit: the Tick does nothing.
pub const INACTIVE_FLAG: u32 = 0x4000_0000;
/// Actor +0x100 bits read for a damaged actor.
pub const FLAG_200000: u32 = 0x0020_0000;
pub const FLAG_20000000: u32 = 0x2000_0000;
/// Settlement actor +0x100 bit that blocks wall creation.
pub const FLAG_100000: u32 = 0x0010_0000;

/// The component fields the Tick reads once or writes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Settlement {
    /// +4, the settlement actor.
    pub actor: Ref,
    /// +0x10, the style index into definition +0x464.
    pub style_index: u32,
    /// +0x48, the time of the last `0x586150` call.
    pub last_periodic: f32,
    /// +0x78, the first piece slot of the scan.
    pub rotation: i32,
    /// +0x7C byte: nonzero scans the slots forwards, zero backwards.
    pub forwards: u8,
}

/// Whose virtual +0x108 supplies the +0x1F0 value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Owner {
    /// Settlement +0x14.
    Component14,
    /// Settlement +4 (the actor).
    Actor,
}

/// Reads and calls in the order the Tick makes them.
pub trait SettlementWorld {
    /// Actor +0x100.
    fn flags(&mut self, actor: Ref) -> Result<u32>;
    /// Actor +4.
    fn definition(&mut self, actor: Ref) -> Result<Ref>;
    /// Actor +0x60 record +0x10 / +0x14; `None` when +0x60 is null.
    fn health(&mut self, actor: Ref) -> Result<Option<[f32; 2]>>;
    /// Settlement +0x1C / +0x18[index].
    fn member_count(&mut self) -> Result<i32>;
    fn member(&mut self, index: i32) -> Result<Ref>;
    /// Settlement +0x28 / +0x24[slot].
    fn piece_count(&mut self) -> Result<i32>;
    fn piece(&mut self, slot: i32) -> Result<Ref>;
    /// Virtual +0x108 of the owner, then +0x1F0 of its result.
    fn owner_1f0(&mut self, owner: Owner) -> Result<f32>;
    /// Settlement definition +0x4A9 / +0x4A8 bytes.
    fn definition_4a9(&mut self, definition: Ref) -> Result<u8>;
    fn definition_4a8(&mut self, definition: Ref) -> Result<u8>;
    /// Definition +0x464 entry `index` (12 bytes) word +8.
    fn style(&mut self, definition: Ref, index: u32) -> Result<Ref>;
    /// Definition +0x47C entry `slot` (0x14 bytes) word +0.
    fn slot_kind(&mut self, definition: Ref, slot: i32) -> Result<u32>;
    /// Style +0x18 / +0x1C / +0x20 (`0x471A40` kinds 0, 1, 2).
    fn style_definitions(&mut self, style: Ref) -> Result<[Ref; 3]>;
    /// `0x586150(controller)` on the component.
    fn periodic(&mut self) -> Result<()>;
    /// `0x56B590(kind, target)` on the controller.
    fn start(&mut self, kind: u32, target: Ref) -> Result<()>;
    /// `0x568040` on the controller: every slot but 5, 6 and 7 is empty.
    fn idle(&mut self) -> Result<bool>;
    /// `0x56A6B0(kind)` on the controller.
    fn admits(&mut self, kind: u32) -> Result<bool>;
    /// CreateFullWall `0x587210(1)`.
    fn create_full_wall(&mut self) -> Result<bool>;
    /// `0x586DB0(slot, 1, 1)`: the created piece or 0.
    fn create_piece(&mut self, slot: i32) -> Result<Ref>;
    /// UpgradeFullWall `0x585C60`.
    fn upgrade_full_wall(&mut self) -> Result<bool>;
    /// `0x585BF0(slot)`.
    fn upgrade_piece(&mut self, slot: i32) -> Result<bool>;
}

fn bin(value: f32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.to_bits())?)
}

/// `0x471A40`.
fn by_kind(definitions: [Ref; 3], kind: u32) -> Ref {
    match kind {
        0 => definitions[0],
        1 => definitions[1],
        2 => definitions[2],
        _ => 0,
    }
}

/// True for the members' `+0x10 < +0x14` (ordered) test.
fn damaged(health: Option<[f32; 2]>) -> bool {
    health.is_some_and(|[now, full]| now < full)
}

/// Complete `0x587290`. `time` is GWorld +0xE0.
pub fn tick<W: SettlementWorld>(
    settlement: &mut Settlement,
    time: f32,
    precision: Precision,
    world: &mut W,
) -> Result<()> {
    let actor = settlement.actor;
    if world.flags(actor)? & INACTIVE_FLAG != 0 {
        return Ok(());
    }
    let last = settlement.last_periodic;
    let due = if last.is_finite() && time.is_finite() {
        bin(last)?.add(&Binary::from_f32_bits(PERIOD_BITS)?, precision)?.compare(&bin(time)?)
            == Ordering::Less
    } else {
        // The extended sum keeps an infinity or NaN operand (no overflow is
        // possible), and an unordered compare skips.
        f64::from(last) + f64::from(f32::from_bits(PERIOD_BITS)) < f64::from(time)
    };
    if due {
        settlement.last_periodic = time;
        world.periodic()?;
    }
    let mut index = 0;
    while index < world.member_count()? {
        let member = world.member(index)?;
        if damaged(world.health(member)?) {
            let flags = world.flags(member)?;
            if flags & FLAG_200000 != 0 || flags & FLAG_20000000 != 0 {
                world.start(2, member)?;
            } else if !(world.owner_1f0(Owner::Component14)? > 0.0) {
                let member = world.member(index)?;
                world.start(4, member)?;
            }
        }
        index += 1;
    }
    if !world.idle()? {
        return Ok(());
    }
    let definition = world.definition(actor)?;
    if settlement.style_index == 0 && world.definition_4a9(definition)? == 0 {
        return Ok(());
    }
    let style = world.style(definition, settlement.style_index)?;
    if world.piece_count()? <= 0 {
        return Ok(());
    }
    let (mut forward, mut backward) = (0i32, 0i32);
    loop {
        let offset = if settlement.forwards != 0 { forward } else { backward };
        let count = world.piece_count()?;
        if count == 0 {
            return Err(Error::Invalid("settlement piece count zero at the native idiv"));
        }
        let slot = settlement.rotation.wrapping_add(count).wrapping_add(offset).wrapping_rem(count);
        let piece = world.piece(slot)?;
        let kind = world.slot_kind(definition, slot)?;
        if kind != 1 {
            if piece == 0 {
                create(settlement, slot, definition, world)?;
            } else if damaged(world.health(piece)?) {
                let flags = world.flags(piece)?;
                if flags & FLAG_200000 != 0 {
                    world.start(6, piece)?;
                } else if flags & FLAG_20000000 != 0 {
                    world.start(7, piece)?;
                } else if !(world.owner_1f0(Owner::Component14)? > 0.0) {
                    world.start(5, piece)?;
                }
            } else {
                let current = world.definition(piece)?;
                let wanted = by_kind(world.style_definitions(style)?, kind);
                if current != wanted && world.admits(7)? {
                    if world.definition_4a8(definition)? != 0 {
                        if world.upgrade_full_wall()? {
                            world.start(7, actor)?;
                        }
                    } else if world.upgrade_piece(slot)? {
                        let piece = world.piece(slot)?;
                        world.start(7, piece)?;
                    }
                }
            }
        }
        forward += 1;
        backward -= 1;
        if forward >= world.piece_count()? {
            return Ok(());
        }
    }
}

/// The empty-slot branch of `0x587290`.
fn create<W: SettlementWorld>(
    settlement: &Settlement,
    slot: i32,
    definition: Ref,
    world: &mut W,
) -> Result<()> {
    if world.owner_1f0(Owner::Actor)? > 0.0 || !world.admits(6)? {
        return Ok(());
    }
    if world.flags(settlement.actor)? & FLAG_100000 != 0 {
        return Ok(());
    }
    if world.definition_4a8(definition)? != 0 {
        if world.create_full_wall()? {
            world.start(6, settlement.actor)?;
        }
    } else {
        let piece = world.create_piece(slot)?;
        if piece != 0 {
            world.start(6, piece)?;
        }
    }
    Ok(())
}
