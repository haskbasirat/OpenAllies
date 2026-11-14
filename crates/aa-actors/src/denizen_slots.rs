//! Denizen slot scans (the +18 list of 0x14-byte slot entries, the +C list
//! of regiment GTIDs) and the slot-count helper `573AF0`.
//!
//! * `568860`: true when an entry of the +18 list (next at +10) has its +D
//!   byte set ([`any_active`]).
//! * `568D40(object)`: the first entry whose +8 binary32 is not above 0.0
//!   (zero, negative or unordered) and whose +4 word equals the object's +4
//!   word ([`free_slot_for`]).
//! * `568550`: walking the +C node list {GTID, next}, true for the first
//!   GTID that resolves (`[B364C0]`) to an actor with an
//!   OrganizationComponent (+7C) whose `57A730` member count is below its
//!   +2C word (read before the call) ([`has_open_regiment`]).
//! * `573AF0`: the number of non-null words in the +14 array of +18 words
//!   ([`count_present`]).
//! * `573D10(value)`: removal of a kind-2 record from that array
//!   ([`remove_kind2`]).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("denizen slot input unavailable: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

/// The words of a +18 slot entry these scans read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotEntry {
    /// +4.
    pub word4: u32,
    /// +8 (binary32).
    pub timer_bits: u32,
    /// +D byte.
    pub active: u8,
}

/// Complete `568860` over the +18 list in order.
pub fn any_active(entries: &[SlotEntry]) -> bool {
    entries.iter().any(|e| e.active != 0)
}

/// Complete `568D40(object)` over the +18 list in order: the index of the
/// entry the original returns, `None` for its null.
pub fn free_slot_for(entries: &[SlotEntry], object_word4: u32) -> Option<usize> {
    // fcomp [9F32DC]: C0, C3 or unordered is "not above 0.0".
    entries
        .iter()
        .position(|e| !(f32::from_bits(e.timer_bits) > 0.0) && e.word4 == object_word4)
}

/// Services of `568550`, in native order.
pub trait OpenRegimentWorld {
    /// GTID registry `[B364C0]`: `None` for a mismatch or a null entry.
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>>;
    /// Actor +7C (OrganizationComponent).
    fn organization(&mut self, actor: u32) -> Result<Option<u32>>;
    /// Component +2C (signed).
    fn capacity(&mut self, organization: u32) -> Result<i32>;
    /// `57A730` on the component (signed member count).
    fn member_count(&mut self, organization: u32) -> Result<i32>;
}

/// Complete `568550` over the +C GTID list in order.
pub fn has_open_regiment(gtids: &[u32], world: &mut impl OpenRegimentWorld) -> Result<bool> {
    for &gtid in gtids {
        let Some(actor) = world.resolve(gtid)? else { continue };
        let Some(organization) = world.organization(actor)? else { continue };
        let capacity = world.capacity(organization)?;
        if world.member_count(organization)? < capacity {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Complete `573AF0` over the +14 array (+18 words).
pub fn count_present(words: &[u32]) -> i32 {
    words.iter().filter(|&&w| w != 0).count() as i32
}

/// The words of a +14 array record `573D10` reads: +0 kind and +8.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record {
    pub kind: u32,
    pub word8: u32,
}

/// Complete `573D10(value)` over the +14 array: the first non-null record
/// of kind 2 whose +8 word is `value` is freed (`6F7430`) and its slot
/// cleared; the result is its index, or -1.
pub fn remove_kind2(records: &mut [Option<Record>], value: u32) -> i32 {
    for (index, slot) in records.iter_mut().enumerate() {
        if let Some(r) = slot
            && r.kind == 2
            && r.word8 == value
        {
            *slot = None;
            return index as i32;
        }
    }
    -1
}
