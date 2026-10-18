//! Original Denizen component (slot 10): constructor `56B720` and the
//! Inventory part of saved Load `56C320`. Owned lists never fabricate child
//! actors or command objects.
//!
//! The Inventory slot list (`+18` head, `+1C` tail; 0x14-byte slots
//! `{+0 word, +4 target ActorData, +8 value, +C/+D flags, +10 next}`) is built
//! by `56B720` from ActorData `+35C` (`count` slots per entry, each
//! PREPENDED, flags `568880(target, 2)` / `568880(target, 0x2C)`). That
//! producer side is aa-rules `ActorCatalog::native_denizen_inventory`
//! (definitions agent, oracle-proven); this module consumes its head-first
//! slot list ([`DenizenState::with_inventory`]) and ports the saved Load of
//! the slots (`56C39E..56C45B`: [`DenizenState::load_occupants`] streaming,
//! [`DenizenState::apply_occupants`] for an already decoded list).
//! Oracle: research/phase8-saved-components/native-denizen-inventory.json
//! (synthetic cases plus the 168 shipped carrier/airfield payloads).
use crate::{
    ActorId,
    ground_states::ComponentHeader,
    organization_state::{Cursor, LoadError, store32},
};
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use std::{collections::BTreeMap, convert::Infallible};

/// One producer slot (aa-rules `DenizenInventorySlot`), head first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventorySlotDefinition {
    /// Slot `+4`: target ActorData (canonical IDS).
    pub target: String,
    /// Slot `+C`: `568880(target, 2)`.
    pub worker: bool,
    /// Slot `+D`: `568880(target, 0x2C)`.
    pub militia_or_air_defensive: bool,
}
/// One constructed Inventory slot (0x14 bytes natively).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventorySlot {
    /// `+0`: constructor 0, saved occupant word on Load.
    pub word_00: u32,
    pub target: String,
    /// `+8`: constructor 0; Load stores target ActorData `+284` (45D730 with
    /// a null modifier) times the saved multiplier, as binary32.
    pub value_08: F32,
    pub worker: bool,
    pub militia_or_air_defensive: bool,
}
#[derive(Clone, Debug)]
pub struct DenizenState {
    pub header: ComponentHeader,
    /// Only initialized scalar fields. Padding and pointer words are excluded.
    pub words: BTreeMap<u32, u32>,
    pub byte_20: u8,
    pub byte_21: u8,
    /// `+18` Inventory slots, head first.
    pub inventory: Vec<InventorySlot>,
    /// Native head-first order and duplicates; later native callbacks bind IDs.
    pub handles_2c: Vec<u32>,
    pub groups_48: [Vec<Infallible>; 8],
}
impl DenizenState {
    /// `56B720(owner)` with the producer's slot list (head first, exactly as
    /// `ActorCatalog::native_denizen_inventory(..).slots` returns it).
    pub fn with_inventory(owner: ActorId, slots: &[InventorySlotDefinition]) -> Self {
        let mut words: BTreeMap<_, _> = [0x10, 0x14, 0x24, 0x28, 0x34, 0x38, 0x3c, 0x8c]
            .into_iter()
            .map(|o| (o, 0))
            .collect();
        words.insert(0x90, 0xc2c80000);
        Self {
            header: ComponentHeader::new(owner),
            words,
            byte_20: 0,
            byte_21: 0,
            inventory: slots
                .iter()
                .map(|s| InventorySlot {
                    word_00: 0,
                    target: s.target.clone(),
                    value_08: F32::ZERO,
                    worker: s.worker,
                    militia_or_air_defensive: s.militia_or_air_defensive,
                })
                .collect(),
            handles_2c: vec![],
            groups_48: std::array::from_fn(|_| vec![]),
        }
    }
    fn occupant(
        slot: &mut InventorySlot,
        multiplier: u32,
        health: &mut impl FnMut(&str) -> std::result::Result<u32, LoadError>,
        pc: Precision,
    ) -> std::result::Result<(), LoadError> {
        let numeric = |e| LoadError::Unsupported(format!("x87 arithmetic outside the finite domain: {e}"));
        let base = Binary::from_f32_bits(health(&slot.target)?).map_err(numeric)?;
        let product = base
            .mul(&Binary::from_f32_bits(multiplier).map_err(numeric)?, pc)
            .map_err(numeric)?;
        slot.value_08 = F32(store32(&product)?);
        Ok(())
    }
    /// `56C39E..56C45B` streaming from the saved occupant count word (saved
    /// version 3 or later): the first `min(count, slots)` pairs go to the slots head first
    /// (`+0` read straight into the slot, then the multiplier), the rest are
    /// read and discarded. Returns the bytes consumed. `health(target)` is
    /// ActorData `+284` bits.
    pub fn load_occupants(
        &mut self,
        bytes: &[u8],
        mut health: impl FnMut(&str) -> std::result::Result<u32, LoadError>,
        pc: Precision,
    ) -> std::result::Result<usize, LoadError> {
        let mut c = Cursor::new(bytes);
        let count = c.i32()?;
        let applied = (count as i64).min(self.inventory.len() as i64).max(0) as usize;
        for slot in self.inventory.iter_mut().take(applied) {
            slot.word_00 = c.u32()?;
            let multiplier = c.u32()?;
            Self::occupant(slot, multiplier, &mut health, pc)?;
        }
        for _ in applied as i64..count as i64 {
            c.u32()?;
            c.u32()?;
        }
        Ok(c.position())
    }
    /// Same effect for an already decoded occupant list (the restoration
    /// decoder reads all pairs first; a truncated list therefore fails before
    /// any slot write, where the original has written the earlier slots).
    pub fn apply_occupants(
        &mut self,
        saved: &[[u32; 2]],
        mut health: impl FnMut(&str) -> std::result::Result<u32, LoadError>,
        pc: Precision,
    ) -> std::result::Result<(), LoadError> {
        for (slot, pair) in self.inventory.iter_mut().zip(saved) {
            slot.word_00 = pair[0];
            Self::occupant(slot, pair[1], &mut health, pc)?;
        }
        Ok(())
    }
}
