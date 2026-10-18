//!Native57AD30's stack ActorParams and ordered current field reads.
//!The installed Organization is represented only by borrowed field services.
use aa_actors::actor_creation::ActorParams;
type Result<T> = std::result::Result<T, String>;
pub(super) trait SlotFields {
    type Definition;
    fn occupied(&mut self, slot: i32) -> Result<bool>;
    fn definition(&mut self, slot: i32) -> Result<Self::Definition>;
    fn allegiance(&mut self) -> Result<u32>;
    fn create(
        &mut self,
        params: &mut ActorParams<Self::Definition, String>,
        slot: i32,
    ) -> Result<bool>;
}
pub(super) fn spawn_slot_fields<F: SlotFields>(
    fields: &mut F,
    slot: i32,
    flag: u8,
) -> Result<bool> {
    if fields.occupied(slot)? {
        return Ok(false);
    }
    let mut params = ActorParams::new(fields.definition(slot)?);
    params.owner = fields.allegiance()?;
    params.byte_51 = flag;
    fields.create(&mut params, slot)
}
///578040. Its captured allegiance survives the refresh callback, while
///later Actor state and SAI fields remain current lazy reads.
pub(super) trait NotifyFields {
    type Owner: Copy;
    fn allegiance(&mut self) -> Result<Option<Self::Owner>>;
    fn hq_present(&mut self) -> Result<bool>;
    fn member_is_primary(&mut self) -> Result<bool>;
    fn refresh_kingdom(&mut self, owner: Self::Owner) -> Result<()>;
    fn statistics_present(&mut self, owner: Self::Owner) -> Result<bool>;
    fn state_1(&mut self) -> Result<bool>;
    fn add_score(&mut self, owner: Self::Owner, index: u32) -> Result<()>;
    fn classified_58edd0(&mut self) -> Result<bool>;
    fn sdk_present(&mut self) -> Result<bool>;
    fn sai_actor_present(&mut self) -> Result<bool>;
    fn notify_current_sai_actor(&mut self) -> Result<()>;
}
pub(super) fn notify_fields<F: NotifyFields>(fields: &mut F) -> Result<()> {
    if let Some(owner) = fields.allegiance()? {
        if fields.hq_present()? && fields.member_is_primary()? {
            fields.refresh_kingdom(owner)?;
        }
        if fields.statistics_present(owner)? && fields.state_1()? {
            fields.add_score(owner, 8)?;
            if fields.classified_58edd0()? {
                fields.add_score(owner, 11)?;
            }
        }
    }
    if fields.sdk_present()? && fields.sai_actor_present()? {
        fields.notify_current_sai_actor()?;
    }
    Ok(())
}
#[cfg(test)]
#[path = "stored_organization_spawn_fields_tests.rs"]
mod tests;
