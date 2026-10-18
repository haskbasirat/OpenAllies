//!Native4FA150 member completion and4FA3D0 created-member hero postpass.
//!The ordinary pass delegates the existing proved aa-mapworld kernel. Hero
//!callbacks borrow the SAME component/member/Kingdom/recruit owners; none
//!has a default. A required child failure retains all preceding mutations.
pub use aa_mapworld::fresh_creation::finish_organization_members;
pub type Result<T> = std::result::Result<T, String>;

pub trait HeroWorld {
    type Actor: Copy;
    type Organization: Copy;
    type Definition: Copy;
    type RecruitTable: Copy;
    ///4FA404 captures this original Organization component once. A missing
    ///component is a native dereference, never a successful absent result.
    fn capture_organization(&mut self, actor: Self::Actor) -> Result<Self::Organization>;
    ///Original captured Org+18 -> Layout+2C, initially and at every loop tail.
    fn slot_count(&mut self, captured: Self::Organization) -> Result<i32>;
    ///Current Layout+28[slot] -> group+8. The comparison is SIGNED <=1;
    ///negative group types follow the same eligible native branch.
    fn group_type(&mut self, captured: Self::Organization, slot: i32) -> Result<i32>;
    ///Fresh actor+7C -> Org+28[slot], distinct from captured component above.
    fn current_member(&mut self, actor: Self::Actor, slot: i32) -> Result<Option<Self::Actor>>;
    ///Member+4 is captured before the first virtual108 owner callback.
    fn member_definition(&mut self, member: Self::Actor) -> Result<Self::Definition>;
    fn first_owner_present_108(&mut self, member: Self::Actor) -> Result<bool>;
    ///Second CURRENT virtual108, then native wrapping owner+1FC null test.
    ///None denotes that exact derived-address-null branch. A second NULL
    ///owner instead derives invalid1FC and must fail in a concrete valid-ID
    ///adapter, rather than pretending this is the native null-table branch.
    fn second_owner_recruit_table_108(
        &mut self,
        member: Self::Actor,
    ) -> Result<Option<Self::RecruitTable>>;
    ///4E2A60(capturedDefinition,1) on that SAME current Kingdom recruit table.
    fn recruit_contains_4e2a60(
        &mut self,
        table: Self::RecruitTable,
        definition: Self::Definition,
        flag: u8,
    ) -> Result<bool>;
    ///Fresh actor+7C calls57B830(capturedDefinition). Must bind actual hero
    ///insertion and refresh effects; a missing positive owner is required.
    fn attach_hero_57b830(
        &mut self,
        actor: Self::Actor,
        definition: Self::Definition,
    ) -> Result<bool>;
    ///Reached only after successful attachment: current localization key
    ///editor_hero_attached|ss, org virtual24 and captured Data+10 names,
    ///4B9F40 formatting then40BC50 actual notification/temporary cleanup.
    fn notify_hero_attached(
        &mut self,
        actor: Self::Actor,
        definition: Self::Definition,
    ) -> Result<()>;
}

///Postpass of4FA3D0, AFTER the actual4FA150 result. An empty creation result
///returns without dereferencing any component. This does not create actors or
///claim the positive hero/recruit/notification children are already bound.
pub fn hero_postpass<W: HeroWorld>(
    world: &mut W,
    created: Option<W::Actor>,
) -> Result<Option<W::Actor>> {
    let Some(actor) = created else {
        return Ok(None);
    };
    let captured = world.capture_organization(actor)?;
    let mut slot = 0i32;
    let mut count = world.slot_count(captured)?;
    while slot < count {
        if world.group_type(captured, slot)? <= 1 {
            if let Some(member) = world.current_member(actor, slot)? {
                let definition = world.member_definition(member)?;
                if world.first_owner_present_108(member)? {
                    if let Some(table) = world.second_owner_recruit_table_108(member)? {
                        if world.recruit_contains_4e2a60(table, definition, 1)? {
                            if world.attach_hero_57b830(actor, definition)? {
                                world.notify_hero_attached(actor, definition)?;
                            }
                        }
                    }
                }
            }
        }
        //4FA54F restores the ORIGINAL captured Organization component;
        //4FA557..55A rereads its current layout and count before index++.
        count = world.slot_count(captured)?;
        slot = slot
            .checked_add(1)
            .ok_or("native hero slot signed-index overflow")?;
    }
    Ok(Some(actor))
}

#[cfg(test)]
#[path = "fresh_organization_completion_tests.rs"]
mod tests;
