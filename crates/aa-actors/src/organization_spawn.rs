//! Organization `57D950`: create an empty slot's actor through `4F9C10`,
//! pair its existing recruitment item, attach it, then notify `578040`.
//!
//! This does not choose a slot or search for a recruiting structure. Those
//! are the existing `57AD30` and `4FBF40` coordinators. The callbacks borrow
//! the current owners; they must not rebuild an Organization or a member.
use crate::actor_creation::ActorParams;
use crate::world_creation::Result;

pub trait World {
    type Object: Copy;
    type Definition;
    type Layout;
    /// Current native recruitment item represented by its owning arena's
    /// typed identity, never a truncated native pointer or allocation index.
    type RecruitItem;
    /// Organization `+28[slot]`, before constructing a descriptor.
    fn occupied(&mut self, organization: Self::Object, slot: i32) -> Result<bool>;
    /// Organization `+1C[LayoutData+28[slot]]`.
    fn slot_definition(&mut self, organization: Self::Object, slot: i32)
        -> Result<Self::Definition>;
    /// Owner's virtual `108`, before copying the incoming pose.
    fn allegiance(&mut self, organization: Self::Object) -> Result<u32>;
    /// Reread Organization `+4`; null maps to zero, otherwise Thing `+14`.
    fn owner_gtid(&mut self, organization: Self::Object) -> Result<u32>;
    /// Exact `4F9C10`, not an alternative higher-level creation entry.
    fn create(&mut self, params: &mut ActorParams<Self::Definition, Self::Layout>)
        -> Result<Option<Self::Object>>;
    /// Reread Organization `+34[slot]` only after successful creation.
    fn recruit_item(&mut self, organization: Self::Object, slot: i32) -> Result<Option<Self::RecruitItem>>;
    /// `4CEA60(item)` on the newly created actor, including item pairing and
    /// its saved health/experience when those components are present.
    fn pair_recruit_item(&mut self, member: Self::Object, item: Self::RecruitItem) -> Result<()>;
    /// `57CBA0(slot, actor)`.
    fn add_member(&mut self, organization: Self::Object, slot: i32, member: Self::Object)
        -> Result<()>;
    /// `578040(member)` on the same Organization.
    fn notify_member(&mut self, organization: Self::Object, member: Self::Object) -> Result<()>;
}

/// Native `57D950`, including arbitrary byte values at creation `+50/+51`.
/// Completed creation and earlier writes remain in their owners if a later
/// recruitment pairing, membership, or notification child fails.
pub fn create_member<W: World>(
    world: &mut W,
    organization: W::Object,
    slot: i32,
    position_bits: [u32; 3],
    direction_bits: [u32; 2],
    byte_50: u8,
    byte_51: u8,
) -> Result<Option<W::Object>> {
    if world.occupied(organization, slot)? {
        return Ok(None);
    }
    let definition = world.slot_definition(organization, slot)?;
    let mut params = ActorParams::new(definition);
    params.owner = world.allegiance(organization)?;
    params.position_bits = position_bits;
    params.direction_bits = direction_bits;
    params.byte_50 = byte_50;
    params.byte_51 = byte_51;
    params.word_40 = world.owner_gtid(organization)?;
    let Some(member) = world.create(&mut params)? else { return Ok(None) };
    if let Some(item) = world.recruit_item(organization, slot)? {
        world.pair_recruit_item(member, item)?;
    }
    world.add_member(organization, slot, member)?;
    world.notify_member(organization, member)?;
    Ok(Some(member))
}
