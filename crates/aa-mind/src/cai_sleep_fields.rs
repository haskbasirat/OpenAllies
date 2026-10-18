//!522F60/523000 field kernels. Live iterator and callback accesses are lazy;
//!Installed/Arena/Organization storage remains with the concrete caller.
use crate::{Result, construct::Kind};
///535E70 reads Element+54 status; Regiment/Aircraft inherit6A71F0.
///Base's slot is96CAC2 purecall and cannot supply a false gameplay value.
pub fn member_busy(kind: Kind, deploy_status: impl FnOnce() -> Result<u32>) -> Result<bool> {
    match kind {
        Kind::Element => Ok(deploy_status()? == 2),
        Kind::Regiment | Kind::Aircraft => Ok(false),
        Kind::Base => Err(crate::Error::World(
            "actual Base Mind vf78 purecall96CAC2".into(),
        )),
    }
}
pub trait RefreshFields {
    type Member: Copy;
    fn retry(&mut self) -> Result<u32>;
    fn store_retry(&mut self, value: u32) -> Result<()>;
    fn next_attached(&mut self, index: &mut usize) -> Result<Option<Self::Member>>;
    fn mode(&mut self) -> Result<u32>;
    fn primary(&mut self, member: Self::Member) -> Result<bool>;
    fn flag(&mut self) -> Result<u8>;
    fn dispatch(&mut self, member: Self::Member, flag: u8) -> Result<()>;
    fn store_pending(&mut self, value: u8) -> Result<()>;
}
pub fn refresh_members(w: &mut impl RefreshFields) -> Result<()> {
    let retry = w.retry()?.wrapping_add(1);
    w.store_retry(retry)?;
    let mut index = 0;
    while let Some(member) = w.next_attached(&mut index)? {
        if w.mode()? == 2 && w.primary(member)? {
            continue;
        }
        let flag = w.flag()?;
        w.dispatch(member, flag)?;
    }
    w.store_pending(1)
}
pub trait RetryFields {
    fn any_active(&mut self) -> Result<bool>;
    fn all_at_slots(&mut self) -> Result<bool>;
    fn retry(&mut self) -> Result<u32>;
    fn store_pending(&mut self, value: u8) -> Result<()>;
    fn refresh(&mut self) -> Result<()>;
}
pub fn retry_refresh(w: &mut impl RetryFields) -> Result<()> {
    if w.any_active()? {
        return Ok(());
    }
    if w.all_at_slots()? || (w.retry()? as i32) >= 2 {
        return w.store_pending(0);
    }
    w.refresh()
}
