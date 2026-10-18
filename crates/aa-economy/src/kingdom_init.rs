//! GKingdom::Init `4E5DF0`, called by LoadThing `4FAE80` immediately after
//! loading a non-bit7 kingdom. This coordinator never skips Actor::Init and
//! never treats resetting resource flows as completion of the whole phase.
//!
//! Every required runtime effect is an explicit host callback. Earlier effects
//! stay applied when a callback fails. Temporary native iterator allocation is
//! represented by Rust locals; allocation failure/SEH destructor parity is not
//! claimed. No gameplay state or host operation is defaulted to success.
use crate::{EconomyError, X87Precision};
use serde::{Deserialize, Serialize};

pub type Kingdom = u32;
pub type Definition = u32;
pub type DefinitionNode = u32;
pub type Team = u32;
pub type Result<T> = std::result::Result<T, EconomyError>;

/// Original category filter constructed by4BC6C0: UNIT, then TRUCK.
pub const RECRUIT_CATEGORIES: [u32; 2] = [0x13, 0x2a];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Flow {
    Provided1a0,
    Gross1ac,
    Consumed1b8,
}
impl Flow {
    pub const fn offset(self) -> u32 {
        match self {
            Self::Provided1a0 => 0x1a0,
            Self::Gross1ac => 0x1ac,
            Self::Consumed1b8 => 0x1b8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecruitLink {
    /// First head at *(ActorData+2C8).
    Primary,
    /// Second head at *(ActorData+2C8)+C; read only if Primary is empty.
    Secondary,
    /// ActorData+17C; read only if both backreference heads are empty.
    Property17c,
}

/// Plain recruit item constructed at4E6061..4E6078 (vtable A29650, size C).
/// This is not a RecruitSingle, whose native vtable is A29694.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlainRecruit {
    pub definition: Definition,
    pub available: u8,
}

/// Required authoritative host operations. IDs are owned host identities;
/// definition-list nodes remain distinct from the ActorData IDs they contain.
/// Read callbacks must read current state, not an earlier collected snapshot.
pub trait World {
    ///4D7230, with the incoming Init argument and incoming x87 precision.
    fn actor_init(
        &mut self,
        kingdom: Kingdom,
        argument: u32,
        precision: X87Precision,
    ) -> Result<bool>;
    ///4BEDF0/404F60 diagnostic after successful Actor Init.
    fn initialized_diagnostic(&mut self, kingdom: Kingdom) -> Result<()>;
    /// Current global GWorld+2C, read after Actor Init/diagnostic.
    fn world_version(&mut self) -> Result<u32>;
    fn schedule_word_184(&mut self, kingdom: Kingdom) -> Result<u32>;
    ///50AC00 on kingdom+180 with argument zero. Own scheduling is required.
    fn legacy_schedule(&mut self, kingdom: Kingdom) -> Result<()>;
    /// Signed GameData+38. Native rereads it after each complete three-cell row.
    fn resource_count(&mut self) -> Result<i32>;
    /// Grow only if needed and store +0 at this index. Existing suffix and
    /// stockpiles must survive. Native order is1A0[i],1AC[i],1B8[i].
    fn clear_flow_cell(&mut self, kingdom: Kingdom, flow: Flow, index: usize) -> Result<()>;
    fn team(&mut self, kingdom: Kingdom) -> Result<Option<Team>>;
    /// Prepend at Team+44, initializing +48 only if it was empty.
    fn prepend_team_member(&mut self, team: Team, kingdom: Kingdom) -> Result<()>;
    fn team_member_count(&mut self, team: Team) -> Result<u32>;
    fn set_team_member_count(&mut self, team: Team, value: u32) -> Result<()>;
    /// Manager B39F58+5C category head, read lazily when entering a category.
    fn definition_category_head(&mut self, category: u32) -> Result<Option<DefinitionNode>>;
    ///46C8C0, *current node.
    fn definition_at_node(&mut self, node: DefinitionNode) -> Result<Definition>;
    ///4BB3F0, current node+8, read after any recruit callbacks.
    fn next_definition_node(&mut self, node: DefinitionNode) -> Result<Option<DefinitionNode>>;
    fn definition_component_mask(&mut self, definition: Definition) -> Result<u32>;
    /// Lazy short-circuit reads. A required missing +2C8 owner is an error,
    /// never a fabricated empty list.
    fn recruit_link_nonempty(&mut self, definition: Definition, link: RecruitLink) -> Result<bool>;
    ///4E1810, actual team/kingdom restriction profiles.
    fn recruit_allowed(&mut self, kingdom: Kingdom, definition: Definition) -> Result<bool>;
    /// Actual4E4C30 ownership/hash insertion. The plain item has +8=1.
    fn append_plain_recruit(&mut self, kingdom: Kingdom, item: PlainRecruit) -> Result<()>;
    fn kingdom_word_190(&mut self, kingdom: Kingdom) -> Result<u32>;
    /// Global SAI+20; read only for word190==0.
    fn sai_active(&mut self) -> Result<bool>;
    ///79E170, required when the SAI gate is active.
    fn register_sai(&mut self, kingdom: Kingdom) -> Result<()>;
    ///4F8AF0, actual world registration, including its owned arrays/counts.
    fn register_world(&mut self, kingdom: Kingdom) -> Result<()>;
    /// Signed live major-kingdom vector count at world+158.
    fn major_count(&mut self) -> Result<i32>;
    /// Live world+154 vector slot, preserving null slots.
    fn major_at(&mut self, index: usize) -> Result<Option<Kingdom>>;
    /// Other kingdom virtual+138(new kingdom).
    fn related(&mut self, other: Kingdom, kingdom: Kingdom) -> Result<bool>;
    ///54EBA0 on other+228 with argument other.
    fn refresh_relation_228(&mut self, other: Kingdom) -> Result<()>;
    ///559600 on other+22C with argument other, after the +228 callback.
    fn refresh_relation_22c(&mut self, other: Kingdom) -> Result<()>;
}

fn below(index: usize, count: i32) -> bool {
    count > 0 && index < count as usize
}

///4F0680/4F06B0 null skipping against the current vector, not a snapshot.
fn skip_null_majors(world: &mut impl World, index: &mut usize) -> Result<()> {
    while below(*index, world.major_count()?) {
        if world.major_at(*index)?.is_some() {
            return Ok(());
        }
        *index += 1;
    }
    Ok(())
}

/// Entire4E5DF0 phase coordinator. The caller must apply the LoadThing bit7
/// gate before invoking this function. `false` means Actor Init returned false;
/// an error identifies a required unbound/failed host effect and never permits
/// post-init publication. Host mutations are not rolled back.
pub fn initialize(
    kingdom: Kingdom,
    argument: u32,
    precision: X87Precision,
    world: &mut impl World,
) -> Result<bool> {
    if !world.actor_init(kingdom, argument, precision)? {
        return Ok(false);
    }
    world.initialized_diagnostic(kingdom)?;
    if world.world_version()? < 22 && world.schedule_word_184(kingdom)? == 0 {
        world.legacy_schedule(kingdom)?;
    }
    if world.resource_count()? > 0 {
        let mut index = 0;
        loop {
            for flow in [Flow::Provided1a0, Flow::Gross1ac, Flow::Consumed1b8] {
                world.clear_flow_cell(kingdom, flow, index)?;
            }
            index += 1;
            if !below(index, world.resource_count()?) {
                break;
            }
        }
    }
    if let Some(team) = world.team(kingdom)? {
        world.prepend_team_member(team, kingdom)?;
        let count = world.team_member_count(team)?;
        world.set_team_member_count(team, count.wrapping_add(1))?;
    }
    //4BC6C0 initializes directly from the first category. Unlike next(), it
    // does not skip a null first category head; native46C8C0 would dereference
    // it. Reject explicitly rather than changing that load behavior.
    let mut category = 0;
    let mut cursor = world.definition_category_head(RECRUIT_CATEGORIES[category])?;
    while category < RECRUIT_CATEGORIES.len() {
        let node = cursor.ok_or_else(|| {
            EconomyError::World(
                "4E5DF0 ActorData iterator has a null current node (native46C8C0 dereference)"
                    .into(),
            )
        })?;
        let definition = world.definition_at_node(node)?;
        if world.definition_component_mask(definition)? & (1 << 8) != 0
            && (world.recruit_link_nonempty(definition, RecruitLink::Primary)?
                || world.recruit_link_nonempty(definition, RecruitLink::Secondary)?
                || world.recruit_link_nonempty(definition, RecruitLink::Property17c)?)
            && world.recruit_allowed(kingdom, definition)?
        {
            world.append_plain_recruit(
                kingdom,
                PlainRecruit {
                    definition,
                    available: 1,
                },
            )?;
        }
        cursor = world.next_definition_node(node)?;
        while cursor.is_none() {
            category += 1;
            if category == RECRUIT_CATEGORIES.len() {
                break;
            }
            cursor = world.definition_category_head(RECRUIT_CATEGORIES[category])?;
        }
    }
    if world.kingdom_word_190(kingdom)? == 0 && world.sai_active()? {
        world.register_sai(kingdom)?;
    }
    world.register_world(kingdom)?;
    let mut index = 0;
    skip_null_majors(world, &mut index)?;
    while below(index, world.major_count()?) {
        let other = world.major_at(index)?.ok_or_else(|| {
            EconomyError::World(
                "major kingdom slot became null after native iterator selected it".into(),
            )
        })?;
        if world.related(other, kingdom)? {
            world.refresh_relation_228(other)?;
            world.refresh_relation_22c(other)?;
        }
        //4F06B0 checks the *current* count before incrementing the cursor.
        if below(index, world.major_count()?) {
            index += 1;
            skip_null_majors(world, &mut index)?;
        }
    }
    Ok(true)
}
