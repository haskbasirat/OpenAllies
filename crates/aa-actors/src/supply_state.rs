//! Original Supply component (slot 11, 0x20 bytes): constructor 58AA50,
//! saved Load 58ABD0 (writer 58AB30). Oracle: research/phase8-restoration/
//! native-supply.json (540 whole ctor+Load cases).
use crate::{ActorId, ground_states::ComponentHeader};
use aa_body::F32;
use aa_numeric::{Binary, Precision};

/// ActorData fields written by 464BD0's SupplyComponent block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Definition {
    /// +420 `supply_range` (fixed getter, binary32 store).
    pub supply_range_bits: u32,
    /// +424 `is_relay`.
    pub is_relay: bool,
    /// +428 `stockpile_max`; read only for relays (validated nonnegative).
    pub stockpile_max_bits: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupplyState {
    pub header: ComponentHeader,
    /// +10 copied from ActorData+420 through 45D550(0).
    pub supply_range: F32,
    /// +14, constructor zero; no ported code path writes it yet.
    pub byte_14: u8,
    /// +18 relay stockpile; Load stores saved multiplier * ActorData+428.
    pub stockpile: F32,
    /// +1C raw saved word.
    pub word_1c: u32,
}
impl SupplyState {
    /// 58AA50 body after the 4DB020 base. The owner-Kingdom supply-source
    /// registration (virtual +108, thunk 4E38A0 -> 4E1F00) is performed by the
    /// caller with [`register_source`], immediately after construction.
    pub fn new(owner: ActorId, definition: &Definition) -> Self {
        Self {
            header: ComponentHeader::new(owner),
            supply_range: F32(definition.supply_range_bits),
            byte_14: 0,
            stockpile: F32::ZERO,
            word_1c: 0,
        }
    }
    /// 58ABD0 after the version read: the Updatable sub-object (version > 0)
    /// and, for version >= 2, the saved multiplier and +1C word.
    pub fn load(
        &mut self,
        definition: &Definition,
        updatable: bool,
        version_2: Option<(u32, u32)>,
        pc: Precision,
    ) -> aa_numeric::Result<()> {
        if updatable {
            self.header.scheduled_head = None;
        }
        if let Some((multiplier, word_1c)) = version_2 {
            // 58AC21: fld saved; fmul ActorData+428; fstp [+18] for relays.
            self.stockpile = if definition.is_relay {
                F32(Binary::from_f32_bits(multiplier)?
                    .mul(&Binary::from_f32_bits(definition.stockpile_max_bits)?, pc)?
                    .store_f32()?)
            } else {
                F32::ZERO
            };
            self.word_1c = word_1c;
        }
        Ok(())
    }
}
/// 4E1F00 on the owner Kingdom's +264 list (head-first; +268 is the tail,
/// set only when the list was empty, so it stays the oldest entry). Returns
/// whether the actor was inserted (already present: no change).
pub fn register_source(list: &mut Vec<ActorId>, actor: ActorId) -> bool {
    if list.contains(&actor) {
        return false;
    }
    list.insert(0, actor);
    true
}
