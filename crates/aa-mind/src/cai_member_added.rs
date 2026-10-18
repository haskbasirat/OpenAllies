//! Original regiment Mind "member added" `524B80` (CAI.cpp), called by the
//! Organization `57CBA0` on the owner's Mind (`OrganizationWorld::
//! mind_member_added` in aa-actors):
//!
//! 1. the member's Mind (member +70) virtual +3C with the owner Mind's +24;
//! 2. the member's Mind virtual +40 with the owner Mind's +28 GTID resolved
//!    through `[B364C0]` (null when stale);
//! 3. `522890(member, 0)` on the owner Mind;
//! 4. unless `[[B36550]+E4]` is 3, the member's GTID (+14) is pushed at the
//!    head of the owner Mind's arrival list +2BC/+2C0 (`4D0900` node
//!    `{gtid, next}`; the tail is set when the list was empty), the list
//!    the sleep Think drains (`cai_sleep`).
use crate::{Arena, MindId, Result};

pub trait MemberAddedWorld {
    /// `[B364C0]` GTID registry.
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>>;
    /// Owner Mind +24 and +28 (`Mind::words_24_30[0..2]`).
    fn mind_word(&mut self, mind: MindId, offset: u32, arena: &Arena) -> Result<u32>;
    /// Member +70 Mind virtual +3C.
    fn member_mind_3c(&mut self, member: u32, value: u32, arena: &mut Arena) -> Result<()>;
    /// Member +70 Mind virtual +40 (the resolved object or null).
    fn member_mind_40(&mut self, member: u32, object: Option<u32>, arena: &mut Arena)
    -> Result<()>;
    /// `522890(member, 0)` on the owner Mind.
    fn helper_522890(
        &mut self,
        mind: MindId,
        member: u32,
        flag: u32,
        arena: &mut Arena,
    ) -> Result<()>;
    /// `[[B36550]+E4]`.
    fn game_mode(&mut self) -> Result<u32>;
    /// Actor +14.
    fn actor_gtid(&mut self, actor: u32) -> Result<u32>;
    /// Push `gtid` at the head of the owner Mind's +2BC list (tail +2C0 set
    /// when empty).
    fn arrival_push_front(&mut self, mind: MindId, gtid: u32, arena: &mut Arena) -> Result<()>;
}

/// Complete `524B80(member)` on the owner Mind.
pub fn member_added(
    w: &mut impl MemberAddedWorld,
    mind: MindId,
    member: u32,
    arena: &mut Arena,
) -> Result<()> {
    let word24 = w.mind_word(mind, 0x24, arena)?;
    w.member_mind_3c(member, word24, arena)?;
    let home = w.mind_word(mind, 0x28, arena)?;
    let object = w.resolve(home)?;
    w.member_mind_40(member, object, arena)?;
    w.helper_522890(mind, member, 0, arena)?;
    if w.game_mode()? != 3 {
        let gtid = w.actor_gtid(member)?;
        w.arrival_push_front(mind, gtid, arena)?;
    }
    Ok(())
}
