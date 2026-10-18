//! Original HQ component (slot 30, 0x2C bytes, vtable `A3CA4C`, Updatable
//! vtable `A3CA24`): constructor `574490` and saved Load `574040` (writer
//! `573F40`, PostLoad `5736B0` in [`crate::post_load::hq_post_load`]).
//!
//! Layout: `+10` badge index (constructor -1), `+14/+18/+1C` slot vector
//! (ThingData `+530` null slots, 1 when that word is <= 0; `430240`),
//! `+20/+24/+28` pending items (the constructor leaves a null buffer with
//! capacity 1). Items are 12-byte records `{+0 kind (3 = none), +4, +8}`.
//! Vector capacities are storage metadata only and are not modelled.
//! Oracle: research/phase8-saved-components/native-hq.json (native_hq.py).
use crate::{
    ActorId,
    ground_states::ComponentHeader,
    organization_state::{Arg, Cursor, LoadError, Message},
};

pub type Result<T> = std::result::Result<T, LoadError>;

/// `5737E0` item record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HqItem {
    /// `+0`; 3 = no item (the allocation default).
    pub kind: u32,
    pub word_04: u32,
    pub word_08: u32,
}
/// The kind an item keeps when its record does not set one.
pub const HQ_ITEM_NONE: u32 = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HqState {
    pub header: ComponentHeader,
    /// `+10`: the nation badge index (-1 = none).
    pub badge_index: i32,
    /// `+14` slot vector (null = `None`).
    pub slots: Vec<Option<HqItem>>,
    /// `+20` items that found no slot; freed by PostLoad `5736B0`.
    pub pending: Vec<HqItem>,
}

/// `404F60` diagnostic of `574040` (HQ.cpp line 0x20F).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HqLog {
    pub line: u32,
    pub function: &'static str,
    pub message: Message,
}
pub const HQ_SOURCE: &str = ".\\Source\\Sim\\Component\\HQ.cpp";
pub const HQ_LOAD_FUNCTION: &str = "KKC_HQComponent::Load";
pub const HQ_BADGE_INVALID: &str = "HQComponent (%s): loaded badge index %d no longer valid for nation '%s'";

/// Services of `574040`.
pub trait HqWorld {
    /// Fact: `[GWorld+284]+0` (the Updatable table bound, `50BD26`).
    fn updatable_table_word0(&self) -> Result<i32>;
    /// Fact: the table slot for `id` is occupied.
    fn updatable_slot_occupied(&self, id: i32) -> Result<bool>;
    /// `50BD37`: store this component's Updatable at `id`.
    fn register_updatable(&mut self, id: i32) -> Result<()>;
    /// Owner virtual `+108` (GActor `4D1E80`: `+E8`); a handle of the Kingdom.
    fn owner_kingdom(&self) -> Result<Option<u32>>;
    /// `[Kingdom+234]+40` (the nation's badge count); None when `+234` is null.
    fn nation_badge_count(&self, kingdom: u32) -> Result<Option<i32>>;
    /// `[Kingdom+234]+10` (the nation name).
    fn nation_name(&self, kingdom: u32) -> Result<Vec<u16>>;
    /// Owner virtual `+24` (GActor `4D99D0`: the display name).
    fn owner_name(&mut self) -> Result<Vec<u16>>;
    fn log(&mut self, log: HqLog) -> Result<()>;
}

impl HqState {
    /// `574490(owner)`; `slots_530` is ThingData `+530`.
    pub fn new(owner: ActorId, slots_530: i32) -> Self {
        let n = if slots_530 > 0 { slots_530 as usize } else { 1 };
        Self {
            header: ComponentHeader::new(owner),
            badge_index: -1,
            slots: vec![None; n],
            pending: Vec::new(),
        }
    }

    /// `574040` on the constructed state; returns the bytes consumed.
    /// Effects before an error stay (the original has no rollback).
    pub fn load(&mut self, bytes: &[u8], world: &mut impl HqWorld) -> Result<usize> {
        let mut c = Cursor::new(bytes);
        // 50BCD0: 50B250 cancels the scheduled entries before the id is read.
        let updatable = c.version(1)?;
        self.header.scheduled_head = None;
        if updatable >= 1 {
            let id = c.i32()?;
            if id >= 0 {
                let bound = world.updatable_table_word0()?;
                if id > bound || world.updatable_slot_occupied(id)? {
                    return Err(LoadError::Fatal(Message::new(
                        "Bad or duplicate updatable ID %d. There are %d.",
                        vec![Arg::Int(id), Arg::Int(bound)],
                    )));
                }
                world.register_updatable(id)?;
            }
        }
        let version = c.version(3)?;
        self.badge_index = c.i32()?;
        if self.badge_index != -1 {
            let kingdom = world.owner_kingdom()?.ok_or(LoadError::NullDereference(0x5740aa))?;
            if let Some(count) = world.nation_badge_count(kingdom)?
                && self.badge_index >= count
            {
                let name = world.owner_name()?;
                let nation = world.nation_name(kingdom)?;
                world.log(HqLog {
                    line: 0x20f,
                    function: HQ_LOAD_FUNCTION,
                    message: Message::new(
                        HQ_BADGE_INVALID,
                        vec![
                            Arg::Text(String::from_utf16_lossy(&name)),
                            Arg::Int(self.badge_index),
                            Arg::Text(String::from_utf16_lossy(&nation)),
                        ],
                    ),
                })?;
                self.badge_index = -1;
            }
        }
        if version == 1 {
            // 4D1AF0: a counted word list, read and freed.
            let count = c.i32()?;
            for _ in 0..count.max(0) {
                c.u32()?;
            }
            return Ok(c.position());
        }
        let count = c.i32()?;
        let mut slot = 0usize;
        for _ in 0..count.max(0) {
            if c.u8()? == 0 {
                continue;
            }
            let mut item = HqItem {
                kind: HQ_ITEM_NONE,
                word_04: 0,
                word_08: u32::MAX,
            };
            // 5737E0.
            if version < 3 {
                if c.u8()? != 0 {
                    item.kind = 0;
                    item.word_04 = c.u32()?;
                } else {
                    c.u32()?;
                }
            } else {
                item.kind = c.u32()?;
                item.word_04 = c.u32()?;
                item.word_08 = c.u32()?;
            }
            if item.kind != HQ_ITEM_NONE && slot != self.slots.len() {
                self.slots[slot] = Some(item);
                slot += 1; // 573750 with its skip flag clear
            } else {
                self.pending.push(item);
            }
        }
        Ok(c.position())
    }
}
