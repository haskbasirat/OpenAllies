//! Original Settlement component wall piece helpers: the piece's current
//! style level `0x5856F0`, the piece upgrade `0x585BF0`, UpgradeFullWall
//! `0x585C60` and CreateFullWall `0x587210`, with `0x529C30` and `0x471A40`
//! inline. The component upgrade `0x566360`, the piece creation `0x586DB0`
//! and the debug report `0x404F30` are required world callbacks in native
//! order. Proven by research/phase8-combat/native_settlement_walls.py.
//!
//! The piece creation itself `0x586DB0`, the health copy to the other
//! pieces `0x5850F0` and the `0x4CEF70` broadcast `0x585040` are ported
//! below, proven by research/phase8-combat/native_settlement_pieces.py.
use crate::Result;
use crate::settlement_tick::{Ref, Settlement};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};

/// The `0x404F30` reports (Settlement.cpp line numbers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Report {
    /// Line 0x3AE, UpgradeFullWall: "Unable to upgrade wall piece".
    UpgradePiece,
    /// Line 0x626, CreateFullWall: "unable to create wall piece".
    CreatePiece,
}

/// Reads and calls in the order the helpers make them.
pub trait WallWorld {
    /// Actor +4.
    fn definition(&mut self, actor: Ref) -> Result<Ref>;
    /// Settlement +0x28 / +0x24[slot].
    fn piece_count(&mut self) -> Result<i32>;
    fn piece(&mut self, slot: i32) -> Result<Ref>;
    /// Settlement definition +0x4A9 byte.
    fn definition_4a9(&mut self, definition: Ref) -> Result<u8>;
    /// Definition +0x464 entry `index` (12 bytes) word +8.
    fn style(&mut self, definition: Ref, index: i32) -> Result<Ref>;
    /// Definition +0x47C entry `slot` (0x14 bytes) word +0.
    fn slot_kind(&mut self, definition: Ref, slot: i32) -> Result<u32>;
    /// Style +0x18 / +0x1C / +0x20 (`0x471A40` kinds 0, 1, 2).
    fn style_definitions(&mut self, style: Ref) -> Result<[Ref; 3]>;
    /// Actor +0x9C component (0: none).
    fn component_9c(&mut self, actor: Ref) -> Result<Ref>;
    /// `0x566360(definition)` on that component.
    fn upgrade(&mut self, component: Ref, definition: Ref) -> Result<()>;
    /// `0x586DB0(slot, flag, second)`: the created piece or 0.
    fn create_piece(&mut self, slot: i32, flag: u32, second: u32) -> Result<Ref>;
    /// `0x404F30` debug report.
    fn report(&mut self, report: Report) -> Result<()>;
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

/// `0x5856F0(slot)`: the highest style level, from Settlement +0x10 down to
/// 0, whose definition for the slot's kind is the piece's; -1 for a slot
/// out of range, an empty slot or no match.
pub fn piece_level<W: WallWorld>(settlement: &Settlement, slot: i32, world: &mut W) -> Result<i32> {
    if slot < 0 || slot >= world.piece_count()? {
        return Ok(-1);
    }
    let piece = world.piece(slot)?;
    if piece == 0 {
        return Ok(-1);
    }
    let definition = world.definition(settlement.actor)?;
    let kind = world.slot_kind(definition, slot)?;
    let mut level = settlement.style_index as i32;
    while level >= 0 {
        let style = world.style(definition, level)?;
        let current = world.definition(piece)?;
        if by_kind(world.style_definitions(style)?, kind) == current {
            return Ok(level);
        }
        level -= 1;
    }
    Ok(-1)
}

/// `0x585BF0(slot)`: upgrade the piece to the next level's definition for
/// its kind through its +0x9C component (true even without one).
pub fn upgrade_piece<W: WallWorld>(settlement: &Settlement, slot: i32, world: &mut W) -> Result<bool> {
    let level = piece_level(settlement, slot, world)?;
    if level < 0 {
        return Ok(false);
    }
    let piece = world.piece(slot)?;
    let definition = world.definition(settlement.actor)?;
    let kind = world.slot_kind(definition, slot)?;
    let style = world.style(definition, level + 1)?;
    let wanted = by_kind(world.style_definitions(style)?, kind);
    let component = world.component_9c(piece)?;
    if component != 0 {
        world.upgrade(component, wanted)?;
    }
    Ok(true)
}

/// UpgradeFullWall `0x585C60`: every nonempty piece (count read once);
/// true when any upgrade succeeded.
pub fn upgrade_full_wall<W: WallWorld>(settlement: &Settlement, world: &mut W) -> Result<bool> {
    let count = world.piece_count()?;
    let mut any = false;
    for slot in 0..count.max(0) {
        if world.piece(slot)? != 0 {
            if upgrade_piece(settlement, slot, world)? {
                any = true;
            } else {
                world.report(Report::UpgradePiece)?;
            }
        }
    }
    Ok(any)
}

/// CreateFullWall `0x587210(flag)`: `0x586DB0(slot, flag, 0)` for every
/// empty slot (count reread); false at once without a style index and
/// definition +0x4A9.
pub fn create_full_wall<W: WallWorld>(settlement: &Settlement, flag: u32, world: &mut W) -> Result<bool> {
    if settlement.style_index == 0 {
        let definition = world.definition(settlement.actor)?;
        if world.definition_4a9(definition)? == 0 {
            return Ok(false);
        }
    }
    let mut any = false;
    let mut slot = 0;
    while slot < world.piece_count()? {
        if world.piece(slot)? == 0 {
            if world.create_piece(slot, flag, 0)? != 0 {
                any = true;
            } else {
                world.report(Report::CreatePiece)?;
            }
        }
        slot += 1;
    }
    Ok(any)
}

/// Definition +0x47C entry (0x14 bytes) of a piece slot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SlotEntry {
    /// +0.
    pub kind: u32,
    /// +4 / +8, added to the settlement actor's X / Y.
    pub offset: [f32; 2],
    /// +0xC / +0x10.
    pub facing: [f32; 2],
}

/// The actor creation block (`0x4C8C20` + `0x4C8C70` defaults) with the
/// fields `0x586DB0` writes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PieceParams {
    /// +4.
    pub definition: Ref,
    /// +8..+0x10.
    pub position: [f32; 3],
    /// +0x14 / +0x18 (defaults 0, -1.0).
    pub facing: [f32; 2],
    /// +0x20 (default -1).
    pub variant: u32,
    /// +0x28.
    pub kingdom: Ref,
    /// +0x34 (default -1).
    pub word_34: u32,
    /// +0x38 (1 when the flag byte is set).
    pub mode: u32,
    /// +0x44: the settlement actor's handle (`0x4CF1B0`).
    pub parent: u32,
}
impl PieceParams {
    fn new(definition: Ref) -> Self {
        Self {
            definition,
            position: [0.0; 3],
            facing: [0.0, -1.0],
            variant: u32::MAX,
            kingdom: 0,
            word_34: u32::MAX,
            mode: 0,
            parent: 0,
        }
    }
}

/// The further reads and calls of the piece creation and its neighbours.
pub trait PieceWorld: WallWorld {
    fn slot_entry(&mut self, definition: Ref, slot: i32) -> Result<SlotEntry>;
    /// Actor +0x20..+0x28.
    fn position(&mut self, actor: Ref) -> Result<[f32; 3]>;
    /// Actor virtual +0x108.
    fn kingdom(&mut self, actor: Ref) -> Result<Ref>;
    /// Actor +0x14 (`0x4CF1B0`).
    fn handle_of(&mut self, actor: Ref) -> Result<u32>;
    /// `0x4F9C10` on GWorld.
    fn create(&mut self, params: &PieceParams) -> Result<Ref>;
    /// `0x587800(slot, piece, second)` on the component.
    fn attach_piece(&mut self, slot: i32, piece: Ref, second: u32) -> Result<()>;
    /// Settlement definition +0x4A8 byte.
    fn definition_4a8(&mut self, definition: Ref) -> Result<u8>;
    /// Actor +0x60 body; `None` when null.
    fn body(&mut self, actor: Ref) -> Result<Option<Ref>>;
    /// Body +0x10 (raw binary32 bits).
    fn body_health(&mut self, body: Ref) -> Result<u32>;
    /// `0x561E50(health, 0, 0, 0)` on a body.
    fn set_health(&mut self, body: Ref, health: u32) -> Result<()>;
    /// Settlement +0x14 and the member list +0x18 / +0x1C.
    fn holder(&mut self) -> Result<Ref>;
    fn member_count(&mut self) -> Result<i32>;
    fn member(&mut self, index: i32) -> Result<Ref>;
    /// Object +0x78 (tested for zero).
    fn word_78(&mut self, object: Ref) -> Result<u32>;
    /// `0x4CEF70(value, actor)` on an object.
    fn notify(&mut self, object: Ref, value: u32, actor: Ref) -> Result<()>;
}

/// `0x586DB0(slot, flag, second)`: a piece for an empty slot from the
/// current style (Settlement +0x10), at the actor position plus the slot
/// offset; `0x587800` attaches it. Returns the piece or 0.
pub fn create_piece<W: PieceWorld>(
    settlement: &Settlement,
    slot: i32,
    flag: u8,
    second: u32,
    precision: Precision,
    world: &mut W,
) -> Result<Ref> {
    if slot < 0 || slot >= world.piece_count()? || world.piece(slot)? != 0 {
        return Ok(0);
    }
    let definition = world.definition(settlement.actor)?;
    if settlement.style_index == 0 && world.definition_4a9(definition)? == 0 {
        return Ok(0);
    }
    let entry = world.slot_entry(definition, slot)?;
    let style = world.style(definition, settlement.style_index as i32)?;
    let mut params = PieceParams::new(by_kind(world.style_definitions(style)?, entry.kind));
    params.kingdom = world.kingdom(settlement.actor)?;
    let position = world.position(settlement.actor)?;
    let sum = |a: f32, b: f32| -> Result<f32> {
        let value = Binary::from_f32_bits(a.to_bits())?.add(&Binary::from_f32_bits(b.to_bits())?, precision)?;
        Ok(f32::from_bits(value.store_f32()?))
    };
    let y = sum(entry.offset[1], position[1])?;
    let x = sum(entry.offset[0], position[0])?;
    params.position = [x, y, position[2]];
    params.facing = entry.facing;
    params.parent = world.handle_of(settlement.actor)?;
    if flag != 0 {
        params.mode = 1;
    }
    let piece = world.create(&params)?;
    if piece != 0 {
        world.attach_piece(slot, piece, second)?;
    }
    Ok(piece)
}

/// `0x5850F0(piece)`: with definition +0x4A8 set, every other nonempty
/// piece with a body takes this piece's body +0x10 (count read once; the
/// piece's own body is dereferenced).
pub fn share_health<W: PieceWorld>(settlement: &Settlement, piece: Ref, world: &mut W) -> Result<()> {
    let definition = world.definition(settlement.actor)?;
    if world.definition_4a8(definition)? == 0 {
        return Ok(());
    }
    let count = world.piece_count()?;
    let body = world
        .body(piece)?
        .ok_or_else(|| crate::Error::World("native null dereference: piece +0x60".into()))?;
    let health = world.body_health(body)?;
    for slot in 0..count.max(0) {
        let other = world.piece(slot)?;
        if other != 0 && other != piece {
            if let Some(body) = world.body(other)? {
                world.set_health(body, health)?;
            }
        }
    }
    Ok(())
}

/// `0x585040(event)`: `0x4CEF70(event +0xC, actor)` on Settlement +0x14,
/// then every member (count reread), then every piece (count read once),
/// for each object with a nonzero +0x78 while the event +0x10 byte is zero.
pub fn notify_all<W: PieceWorld>(
    settlement: &Settlement,
    event_value: u32,
    event_handled: u8,
    world: &mut W,
) -> Result<()> {
    let each = |object: Ref, world: &mut W| -> Result<()> {
        if object != 0 && world.word_78(object)? != 0 && event_handled == 0 {
            world.notify(object, event_value, settlement.actor)?;
        }
        Ok(())
    };
    let holder = world.holder()?;
    each(holder, world)?;
    let mut index = 0;
    while index < PieceWorld::member_count(world)? {
        let member = PieceWorld::member(world, index)?;
        each(member, world)?;
        index += 1;
    }
    let count = world.piece_count()?;
    for slot in 0..count.max(0) {
        let piece = world.piece(slot)?;
        each(piece, world)?;
    }
    Ok(())
}
