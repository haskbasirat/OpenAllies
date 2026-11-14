//! Small OrganizationComponent helpers (Organization.cpp):
//!
//! * `578850(message)`: every member slot of +28 (count +2C, reread after
//!   each slot) that is non-null, has a +78 word, and either has no +A4 word
//!   or the message +8 kind is not 7, gets `4CEF70(message +C, owner)` while
//!   the message +10 byte is clear; the owner is the component +4 actor
//!   ([`notify_members`]).
//! * `578430(object)`: the target id +D4 becomes 0 for a null object, the
//!   object's +14 GTID when its +C0 word is set, and is kept otherwise
//!   ([`set_target_id`]).
//! * `578D50(item, slot)` (AddRecruitItem): recruit array +34 slot `slot`
//!   takes the item; the diagnostic `404F90` (line 0xD03) only reads state
//!   ([`add_recruit_item`]).
//! * `577FA0`: true when +40 is set, +44 is clear and the +18 object's +18
//!   word is not negative ([`ready`]).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("organization helper input unavailable: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

/// The message words `578850` reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Message {
    /// +8 kind.
    pub kind: u32,
    /// +C word handed to the members.
    pub word: u32,
    /// +10 byte.
    pub handled: bool,
}

/// The message kind that skips members with a +A4 word.
pub const SKIP_ATTACHED_KIND: u32 = 7;

/// Services of `578850`, in native order.
pub trait MemberNotifyWorld {
    /// Component +2C (signed member count).
    fn member_count(&mut self) -> Result<i32>;
    /// Component +28 slot `index`.
    fn member(&mut self, index: i32) -> Result<Option<u32>>;
    /// Member +78 and +A4 words.
    fn member_word_78(&mut self, member: u32) -> Result<u32>;
    fn member_word_a4(&mut self, member: u32) -> Result<u32>;
    /// Component +4 (the owner actor).
    fn owner(&mut self) -> Result<u32>;
    /// `4CEF70(word, owner)` on the member.
    fn notify(&mut self, member: u32, word: u32, owner: u32) -> Result<()>;
}

/// Complete `578850(message)`.
pub fn notify_members(message: &Message, world: &mut impl MemberNotifyWorld) -> Result<()> {
    let mut index = 0;
    while index < world.member_count()? {
        if let Some(member) = world.member(index)?
            && world.member_word_78(member)? != 0
            && (world.member_word_a4(member)? == 0 || message.kind != SKIP_ATTACHED_KIND)
            && !message.handled
        {
            let owner = world.owner()?;
            world.notify(member, message.word, owner)?;
        }
        index += 1;
    }
    Ok(())
}

/// The object words `578430` reads: +14 GTID and +C0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetObject {
    pub gtid: u32,
    pub word_c0: u32,
}

/// Complete `578430(object)` on the component's +D4 word.
pub fn set_target_id(target_id: &mut u32, object: Option<TargetObject>) {
    match object {
        None => *target_id = 0,
        Some(o) if o.word_c0 != 0 => *target_id = o.gtid,
        Some(_) => {}
    }
}

/// Complete `578D50(item, slot)` on the component's +34 recruit array.
pub fn add_recruit_item(recruits: &mut [u32], item: u32, slot: usize) -> Result<()> {
    let cell = recruits
        .get_mut(slot)
        .ok_or_else(|| Error::World(format!("recruit slot {slot} outside the +34 array")))?;
    *cell = item;
    Ok(())
}

/// Complete `577FA0`: `word40` (+40), `word44` (+44) and the +18 object's
/// +18 word.
pub fn ready(word40: u32, word44: u32, inner18: i32) -> bool {
    word40 != 0 && word44 == 0 && inner18 >= 0
}
