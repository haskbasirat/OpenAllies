//! GWorld::Load post-record stage: the PostLoad pass `4FD3B3..4FD3F1` of
//! `4FC600`, run once after every saved record (Teams, Kingdoms, required
//! objects, DYNAMIC_THINGS) has been loaded and registered.
//!
//! Native order (all synchronous, single threaded):
//! * [`post_load_pass`]: iterator `4EFBE0` over the GTID table `[B364C0]`
//!   (slots ascending from 1 to `+6000C`, null slots skipped, the cursor is
//!   the visited object's own GTID `+14`), calling the object's virtual `+DC`
//!   (PostLoad) for each one. Objects created or removed by an earlier
//!   PostLoad are seen (or not) by the live iteration.
//! * Class virtuals `+DC` ([`PostLoadClass`]): GThing `A2A680` and Team
//!   `A29780` are `943800` (`ret 4`); GObject `4EAA30` ([`object_post_load`]);
//!   GActor `4D3560` ([`actor_post_load`]); GKingdom `4E4220`
//!   ([`kingdom_post_load`]).
//! * GActor `4D3560`: `4EAA30`, then every non-null component's virtual `+30`
//!   in slot order 0..=30 ([`component_post_load`]), then the `+118`, `+120`
//!   and `+128` GTID lists drop every unresolved or flag-bit-0 entry (`50C510`).
//! * Component `+30` (factory `4CF480` vtables): Body `561DF0`, Properties and
//!   Supply `581260`, Denizen `569F80`, Wall `58CB40`, Settlement `586F60`
//!   (= `586BC0(0)`, [`settlement_create_things`]), Defender `5676B0`, Hq
//!   `5736B0`; every other class (all three Mind classes, Aircraft `A3AF78`
//!   included) is `943800`.
//! * GGroup `4DDFA0` ([`group_post_load`]) and the required-object-lost
//!   virtual `+12C` (GObject `4ECC50`, GActor `4CFF70`,
//!   [`required_object_lost`]).
//!
//! The world RNG (`4F02B0` + `85D840`) is drawn by the periodic queue inserts
//! (`50AC00`/`50AC60` -> `50B2C0`) and by the Hq index allocation `4E2640`;
//! the pass order above therefore fixes the RNG stream. Oracle:
//! research/phase8-saved-components/native-post-load.json
//! (native_post_load.py).
use crate::actor_creation::ActorParams;
use crate::components::{ALL_SLOTS, ComponentSlot};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};

/// A raw GTID (`generation << 16 | slot`).
pub type Id = u32;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("post-load world operation failed: {0}")]
    World(String),
    /// The original reads through a null pointer here (access violation).
    #[error("native null dereference: {0}")]
    NullDereference(&'static str),
    /// The original would read fields beyond the object (wrong class).
    #[error("native undefined behaviour: {0}")]
    Undefined(String),
    #[error("unported native PostLoad: {0}")]
    Unsupported(&'static str),
    #[error("native arithmetic: {0}")]
    Numeric(String),
}
impl From<aa_numeric::Error> for Error {
    fn from(e: aa_numeric::Error) -> Self {
        Self::Numeric(e.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;

/// The `+DC` implementation of a GTID-table object's class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PostLoadClass {
    /// `943800` (`ret 4`): GThing (`A2A680`) and Team (`A29780`).
    Inert,
    /// `4EAA30`: GObject (`A2A2A0`), Area (`A28A00`), Camera (`A28B88`),
    /// `A28D18` and DynamicDoodad (`A28E90`).
    Object,
    /// `4D3560`: GActor (`A273F0`).
    Actor,
    /// `4E4220`: GKingdom (`A299D8`).
    Kingdom,
    /// `4DDFA0`: GGroup (saved class 8, constructor `4DE490`, vtable
    /// `A29490`): the GActor part followed by `4DDCD0`/`515F40` on `+1A4`
    /// ([`group_post_load`]; no tutorial record has class 8).
    Group,
}
impl PostLoadClass {
    /// Virtual `+B4`: `6A71D0` (true) for the GActor-derived classes,
    /// `6A71F0` (false) otherwise.
    pub fn is_actor(self) -> bool {
        matches!(self, Self::Actor | Self::Kingdom | Self::Group)
    }
}

/// The three GActor GTID lists `4D3560` prunes (`{head, tail}`; nodes
/// `{+0 GTID, +4 next}`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ActorList {
    /// `+118` (also edited by `4D3990`/`4D0CD0`).
    List118,
    List120,
    List128,
}
pub const ACTOR_LISTS: [ActorList; 3] = [ActorList::List118, ActorList::List120, ActorList::List128];

/// The component Updatable at `Component+8` whose event list (`+4`, i.e.
/// `Component+C`) the queue helpers edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Updatable {
    pub actor: Id,
    pub slot: ComponentSlot,
}

/// `404ED0(B38B70, ".\Source\Sim\GObject.cpp", 0x2D8, "KKC_GObject::PostLoad",
/// REQUIRED_NOT_LOADED, required, object, name)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiredObjectLog {
    pub required: Id,
    pub object: Id,
    /// `[object+4]+10`: the ThingData display name.
    pub name: Vec<u16>,
}
pub const GOBJECT_SOURCE: &str = ".\\Source\\Sim\\GObject.cpp";
pub const POST_LOAD_LINE: u32 = 0x2d8;
pub const POST_LOAD_FUNCTION: &str = "KKC_GObject::PostLoad";
pub const REQUIRED_NOT_LOADED: &str = "Actor '%d' for required object '%d' (%s) not loaded";
/// `5A1040` format (`A29878`) of `4E4220`.
pub const RECRUIT_ITEM_DEAD_ACTOR: &str = "Recruit item single '%s' attached to dead actor";
/// `50ACC0` delay of an active Defender (`3F800000`).
pub const DEFENDER_QUEUE3_DELAY_BITS: u32 = 0x3f80_0000;

/// ThingData `+47C[index]` settlement entry (0x14 bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementEntry {
    /// `+0`: `45D7C0` kind (0, 1, 2 -> ThingData `+49C`, `+4A0`, `+4A4`).
    pub kind: u32,
    /// `+4`/`+8`: offset from the owner (binary32 bits).
    pub offset_bits: [u32; 2],
    /// `+C`/`+10`: direction stored into the creation block `+14/+18`.
    pub direction_bits: [u32; 2],
}

/// GGroup `+1A4` target object (`4DDCD0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupTarget {
    /// `+C`: bit 2 = position target, bit 0 = object target.
    pub flags: u32,
    /// `+10`: the target object's GTID.
    pub object: Id,
    /// `+18`/`+1C` position (binary32 bits).
    pub position_bits: [u32; 2],
}

/// World services of the PostLoad stage. Every method is a native field
/// access or a native helper named in its documentation; implementations
/// must not substitute defaults for missing state.
pub trait PostLoadWorld {
    /// A Denizen command object (`51B580` result; refcounted, shared with
    /// the Minds it is assigned to).
    type Command;
    /// A Kingdom recruit item (`+214` vector element: RecruitSingle, vtable
    /// `A29694`).
    type RecruitItem;
    /// A ThingData handle (settlement creation blocks).
    type Definition: Clone;
    /// An Organization layout handle (`ActorParams` `+2C`, always None here).
    type Layout;

    // ---- GTID table [B364C0] ----
    /// `+6000C` (signed; compared with `jg`).
    fn highest_slot(&self) -> Result<i32>;
    /// `+20004[slot]` non-null: the object's own GTID (`+14`) and class.
    fn object_in_slot(&self, slot: u32) -> Result<Option<(Id, PostLoadClass)>>;
    /// Generation `+4[slot]` equals `id >> 16` and the slot object is non-null.
    fn live(&self, id: Id) -> Result<bool>;
    /// Class of a live object.
    fn object_class(&self, id: Id) -> Result<PostLoadClass>;
    /// `+8` flags word of a live object.
    fn object_flags(&self, id: Id) -> Result<u32>;

    // ---- GObject 4EAA30 ----
    /// `+50`.
    fn required_object(&self, object: Id) -> Result<Id>;
    /// `[object+4]+10`: the ThingData display name as `47D960` stores it
    /// (aa-rules `ActorCatalog::native_display_name`).
    fn definition_name(&self, object: Id) -> Result<Vec<u16>>;
    /// `404ED0` log (see [`RequiredObjectLog`]).
    fn log_required_not_loaded(&mut self, entry: RequiredObjectLog) -> Result<()>;

    // ---- virtual +12C (GObject 4ECC50, GActor-derived 4CFF70) ----
    /// `+50 = 0`.
    fn clear_required_object(&mut self, object: Id) -> Result<()>;
    /// ThingData `+1C` flags.
    fn definition_flags_1c(&self, object: Id) -> Result<u32>;
    /// Virtual `+50(flag)`: GActor `4D4A10`
    /// (`aa_actors::actor_creation::set_blocking`), GObject `4EE840`.
    fn set_blocking(&mut self, object: Id, blocking: bool) -> Result<()>;
    /// Writes the `+8` flags word.
    fn set_object_flags(&mut self, object: Id, flags: u32) -> Result<()>;
    /// GWorld `+154` kingdom vector (count `+158`; null entries as None).
    fn world_kingdoms(&self) -> Result<Vec<Option<Id>>>;
    /// Kingdom `+190`.
    fn kingdom_word_190(&self, kingdom: Id) -> Result<u32>;
    /// Kingdom `+18C` (its index).
    fn kingdom_index(&self, kingdom: Id) -> Result<u32>;
    /// Object `+58` (virtual `+8C` `4EA3D0` = `+58 > 0`).
    fn memory_count(&self, object: Id) -> Result<i32>;
    /// Object `+5C[index]`: a detection memory record (None: null).
    fn memory_record(&self, object: Id, index: u32) -> Result<Option<u32>>;
    /// `[B36528]` non-null with its `+20` byte set.
    fn ui_agent_active(&self) -> Result<bool>;
    /// UI agent `79DEB0(record, kingdom)` (`after` false) / `79E7B0(record,
    /// kingdom)` (`after` true; the record is re-read, None = null).
    fn ui_memory_record(&mut self, record: Option<u32>, kingdom: Id, after: bool) -> Result<()>;
    /// `4EC4A0(record)`: the memory record is refreshed from the object.
    fn refresh_memory_record(&mut self, object: Id, record: u32) -> Result<()>;
    /// ThingData virtual `+24` (`462E50`: the `+34` byte,
    /// needs_space_for_settlement_zone).
    fn settlement_space(&self, object: Id) -> Result<bool>;
    /// `[GWorld+30]+8` (tile size bits).
    fn tile_size_bits(&self) -> Result<u32>;
    /// `[B39F58]+74` (binary32 bits; the `4BB000` term).
    fn thing_manager_74_bits(&self) -> Result<u32>;
    /// Object `+20/+24` (binary32 bits).
    fn object_position(&self, object: Id) -> Result<[u32; 2]>;
    /// Actor `+14C` (zone vector size, signed).
    fn zone_count(&self, actor: Id) -> Result<i32>;
    /// `515F40(x, y, radius)` on zone slot `index`
    /// (`aa_zones::zone::move_to_position`).
    fn move_zone(&mut self, actor: Id, index: u32, xy: [u32; 2], radius_bits: u32) -> Result<()>;
    /// The simulation thread's x87 precision control.
    fn precision(&self) -> Precision;

    // ---- GActor ----
    /// `+60 + 4 * slot` non-null.
    fn has_component(&self, actor: Id, slot: ComponentSlot) -> Result<bool>;
    /// The list's GTIDs, head first.
    fn actor_list(&self, actor: Id, list: ActorList) -> Result<Vec<Id>>;
    /// Unlinks and frees the node at `index` (head first) (`50C510`,
    /// `4F1430`).
    fn remove_actor_list_node(&mut self, actor: Id, list: ActorList, index: usize) -> Result<()>;
    /// Virtual `+108` (`4D1E80`: `+E8`, the stored owner Kingdom).
    fn actor_kingdom(&self, actor: Id) -> Result<Option<Id>>;
    /// `Element+10` (the Element component's organization actor), None when
    /// the actor has no Element component or the field is null.
    fn element_organization(&self, actor: Id) -> Result<Option<Id>>;

    // ---- Component Updatable (+8) queue helpers; their gates (Updatable
    // vt+8/+C) are 6A71D0 (true) for every component reached here ----
    /// `50B220(queue, data)` exactly as the original walks it: from the
    /// Updatable's most recent entry (`Updatable+4`) along the entry's QUEUE
    /// BUCKET successor (`+8`), not along the Updatable's own list (`+10`);
    /// entries of other Updatables later in that bucket count, the
    /// Updatable's own entries in other buckets do not (see
    /// [`native_has_scheduled`]; `aa_world::queues::Queues::has` is the
    /// owner-list test and differs).
    fn has_scheduled(&mut self, at: Updatable, queue: u8, data: u32) -> Result<bool>;
    /// `Component+C` (Updatable `+4`, the list head) is non-null.
    fn has_any_scheduled(&self, at: Updatable) -> Result<bool>;
    /// `50AC00(data)` (queue 0) / `50AC60(data)` (queue 1): periodic insert
    /// `50B2C0`, one world RNG draw.
    fn add_periodic(&mut self, at: Updatable, queue: u8, data: u32) -> Result<()>;
    /// `50ACC0(delay, data)`: queue 3 `add_after` (`50AF10`).
    fn add_after(&mut self, at: Updatable, queue: u8, data: u32, delay_bits: u32) -> Result<()>;
    /// `50B1C0(queue, data)`: removes every matching entry, returns the count.
    fn remove_scheduled(&mut self, at: Updatable, queue: u8, data: u32) -> Result<u32>;
    /// `50B250`: cancels every entry.
    fn cancel_scheduled(&mut self, at: Updatable) -> Result<()>;
    /// `4F02B0` + `85D840`: one world RNG draw (the stream `50B2C0` uses).
    fn random(&mut self) -> Result<u32>;

    // ---- Body 561DF0 ----
    /// SVar `UnitSupplyEnabled` (`561B40` static `B3D654`).
    fn unit_supply_enabled(&self) -> Result<bool>;
    /// ThingData `+2A8` (BodyComponent unit shortfall damage rate) bits.
    fn body_shortfall_rate_bits(&self, actor: Id) -> Result<u32>;
    /// Actor `+16C`.
    fn word_16c(&self, actor: Id) -> Result<Id>;

    // ---- Wall 58CB40 ----
    /// GWorld `+2C` (map version).
    fn world_version(&self) -> Result<u32>;
    /// ThingData `+42C` byte non-zero.
    fn wall_flag_42c(&self, actor: Id) -> Result<bool>;

    // ---- Denizen 569F80 ----
    /// Denizen `+40` list, head first (a null node value is `None`).
    fn denizen_commands(&self, actor: Id) -> Result<Vec<Option<Self::Command>>>;
    /// Command `+C` list of member GTIDs, head first.
    fn command_members(&self, command: &Self::Command) -> Result<Vec<Id>>;
    /// `52A020(command)` on the member's Mind (`+70`): refcounted
    /// assignment of Mind `+30` (no effect when already equal).
    fn set_mind_command(&mut self, member: Id, command: &Self::Command) -> Result<()>;

    // ---- Defender 5676B0 ----
    /// `+24` list `{key, value}`, head first.
    fn defender_entries(&self, actor: Id) -> Result<Vec<[Id; 2]>>;
    fn set_defender_entry_value(&mut self, actor: Id, index: usize, value: Id) -> Result<()>;
    /// `79E3C0`: unlinks and frees the entry at `index`.
    fn remove_defender_entry(&mut self, actor: Id, index: usize) -> Result<()>;
    /// `+1C`.
    fn defender_word_1c(&self, actor: Id) -> Result<u32>;
    /// `+14` (Ability vector count).
    fn defender_ability_count(&self, actor: Id) -> Result<usize>;
    /// `Ability+14` of `+10[index]`.
    fn ability_target(&self, actor: Id, index: usize) -> Result<Id>;
    fn set_ability_target(&mut self, actor: Id, index: usize, target: Id) -> Result<()>;

    // ---- Hq 5736B0 / Kingdom 4E2640 ----
    /// Frees every non-null `+20[i]`, zeroes it, then `+24 = 0`.
    fn hq_release_items(&mut self, actor: Id) -> Result<()>;
    /// `+10`.
    fn hq_index(&self, actor: Id) -> Result<i32>;
    fn set_hq_index(&mut self, actor: Id, index: i32) -> Result<()>;
    /// `[Kingdom+234]+40`; None when `+234` is null.
    fn kingdom_hq_slots(&self, kingdom: Id) -> Result<Option<i32>>;
    /// `+36C` list, head first.
    fn kingdom_hq_taken(&self, kingdom: Id) -> Result<Vec<i32>>;
    /// Prepends to `+36C` (`+370` tail set when it was empty).
    fn push_kingdom_hq_taken(&mut self, kingdom: Id, index: i32) -> Result<()>;

    // ---- Kingdom 4E4220 ----
    /// `+350` (signed).
    fn kingdom_group_count(&self, kingdom: Id) -> Result<i32>;
    /// Group `+34C[group]`: its `+C/+10` member GTIDs.
    fn kingdom_group_members(&self, kingdom: Id, group: usize) -> Result<Vec<Id>>;
    /// `4D19F0(id, 0)` reduced to the member at `index` (the first
    /// occurrence of that GTID): shift down, count - 1.
    fn remove_kingdom_group_member(&mut self, kingdom: Id, group: usize, index: usize) -> Result<()>;
    /// Shift `+34C` down, `+350 - 1`, then `4E3ED0` + free of the (empty)
    /// group (`4E28D0` visits no member).
    fn remove_kingdom_group(&mut self, kingdom: Id, group: usize) -> Result<()>;
    /// `+218` (signed).
    fn recruit_item_count(&self, kingdom: Id) -> Result<i32>;
    /// `+214[index]` when `index < +218` (current count), else None.
    fn recruit_item_at(&self, kingdom: Id, index: usize) -> Result<Option<Self::RecruitItem>>;
    /// Item `+8` byte.
    fn recruit_item_byte_8(&self, item: &Self::RecruitItem) -> Result<u8>;
    /// Item `+10` (RecruitSingle: the organization GTID).
    fn recruit_item_organization(&self, item: &Self::RecruitItem) -> Result<Id>;
    fn set_recruit_item_organization(&mut self, item: &Self::RecruitItem, organization: Id) -> Result<()>;
    /// `[item+4]+10`: the item ActorData's display name (`47D960`).
    fn recruit_item_name(&self, item: &Self::RecruitItem) -> Result<Vec<u16>>;
    /// Item virtual `+8` (RecruitSingle `4E0EC0`): `+8 = 1` (available).
    fn set_recruit_item_available(&mut self, item: &Self::RecruitItem) -> Result<()>;
    /// `Kingdom+224 += 1`.
    fn increment_kingdom_224(&mut self, kingdom: Id) -> Result<()>;
    /// `5A1040(.., CString(format), name)`.
    fn map_warning(&mut self, format: &'static str, name: Vec<u16>) -> Result<()>;

    // ---- Settlement 586BC0 ----
    /// Settlement `+28` (signed entry count).
    fn settlement_count(&self, actor: Id) -> Result<i32>;
    /// The actor ThingData's `+47C[index]`.
    fn settlement_entry(&self, actor: Id, index: usize) -> Result<SettlementEntry>;
    /// `45D7C0(kind)` on the actor's ThingData (None: kind > 2 or a null field).
    fn settlement_definition(&self, actor: Id, kind: u32) -> Result<Option<Self::Definition>>;
    /// Object `+28` (z, binary32 bits).
    fn object_z(&self, object: Id) -> Result<u32>;
    /// `4F9C10(&block)` (KKC_GWorld::CreateGThing; `aa_actors::actor_creation`
    /// / `world_creation`): the created object's GTID, None for null.
    fn create_gthing(&mut self, params: ActorParams<Self::Definition, Self::Layout>) -> Result<Option<Id>>;
    /// Settlement `+3C[index] = id`.
    fn set_settlement_thing(&mut self, actor: Id, index: usize, id: Id) -> Result<()>;

    // ---- GGroup 4DDFA0 ----
    /// Group `+1A4` (None: null).
    fn group_target(&self, group: Id) -> Result<Option<GroupTarget>>;
    /// Writes the target's `+18/+1C` and `+C`.
    fn set_group_target(&mut self, group: Id, target: GroupTarget) -> Result<()>;
    /// Group `+1B4` (radius bits).
    fn group_radius_bits(&self, group: Id) -> Result<u32>;
    /// `552640(&xy)` on `[GWorld+30]` (`aa_zones::zone::clamp_position`).
    fn clamp_position(&mut self, xy: [u32; 2]) -> Result<[u32; 2]>;
}

/// `50B220` over the native links: `bucket_chain` is `(queue byte +18,
/// data +14)` of the Updatable's most recent entry followed by that entry's
/// bucket successors (`+8`), in order; empty when the Updatable has no
/// entry. Oracle: native-post-load.json `updatable_has_50b220`.
pub fn native_has_scheduled(bucket_chain: impl IntoIterator<Item = (u8, u32)>, queue: u8, data: u32) -> bool {
    bucket_chain.into_iter().any(|entry| entry == (queue, data))
}

fn require_actor(world: &impl PostLoadWorld, id: Id, site: &str) -> Result<()> {
    let class = world.object_class(id)?;
    if class.is_actor() {
        Ok(())
    } else {
        Err(Error::Undefined(format!("{site} reads GActor fields of {id:#x} ({class:?})")))
    }
}

/// The pass `4FD3B3..4FD3F1` with iterator `4EFBE0`.
pub fn post_load_pass<W: PostLoadWorld>(world: &mut W) -> Result<()> {
    let mut cursor: Id = 0;
    loop {
        let mut slot = if cursor == 0 { 1 } else { (cursor & 0xffff) + 1 };
        let mut found = None;
        while i64::from(slot) <= i64::from(world.highest_slot()?) {
            if let Some(object) = world.object_in_slot(slot)? {
                found = Some(object);
                break;
            }
            slot += 1;
        }
        let Some((id, class)) = found else {
            return Ok(());
        };
        if id == 0 {
            return Err(Error::Undefined(format!(
                "object in slot {slot} has GTID 0; 4EFBE0 would restart the iteration"
            )));
        }
        cursor = id;
        class_post_load(world, id, class)?;
    }
}

/// Virtual `+DC` of one object.
pub fn class_post_load<W: PostLoadWorld>(world: &mut W, id: Id, class: PostLoadClass) -> Result<()> {
    match class {
        PostLoadClass::Inert => Ok(()),
        PostLoadClass::Object => object_post_load(world, id),
        PostLoadClass::Actor => actor_post_load(world, id),
        PostLoadClass::Kingdom => kingdom_post_load(world, id),
        PostLoadClass::Group => group_post_load(world, id),
    }
}

/// GObject::PostLoad `4EAA30`: a non-zero `+50` that does not resolve
/// (generation mismatch or empty slot; the destroyed flag is not checked) is
/// logged and dropped through virtual `+12C`.
pub fn object_post_load<W: PostLoadWorld>(world: &mut W, object: Id) -> Result<()> {
    let required = world.required_object(object)?;
    if required != 0 && !world.live(required)? {
        let name = world.definition_name(object)?;
        world.log_required_not_loaded(RequiredObjectLog {
            required,
            object,
            name,
        })?;
        required_object_lost(world, object)?;
    }
    Ok(())
}

/// GActor::PostLoad `4D3560`.
pub fn actor_post_load<W: PostLoadWorld>(world: &mut W, actor: Id) -> Result<()> {
    object_post_load(world, actor)?;
    for slot in ALL_SLOTS {
        if world.has_component(actor, slot)? {
            component_post_load(world, actor, slot)?;
        }
    }
    for list in ACTOR_LISTS {
        let values = world.actor_list(actor, list)?;
        let mut removed = 0;
        for (index, id) in values.into_iter().enumerate() {
            let keep = world.live(id)? && world.object_flags(id)? & 1 == 0;
            if !keep {
                world.remove_actor_list_node(actor, list, index - removed)?;
                removed += 1;
            }
        }
    }
    Ok(())
}

/// Component virtual `+30` (factory `4CF480` vtables).
pub fn component_post_load<W: PostLoadWorld>(world: &mut W, actor: Id, slot: ComponentSlot) -> Result<()> {
    use ComponentSlot::*;
    match slot {
        Body => body_post_load(world, actor),
        Properties | Supply => scheduled_post_load(world, Updatable { actor, slot }),
        Denizen => denizen_post_load(world, actor),
        Wall => wall_post_load(world, actor),
        Settlement => settlement_create_things(world, actor, false).map(|_| ()),
        Defender => defender_post_load(world, actor),
        Hq => hq_post_load(world, actor),
        _ => Ok(()),
    }
}

/// `561B40`: UnitSupplyEnabled, no Organization component (`+7C`), and
/// ThingData `+2A8 > 0.0` (`fcomp`; NaN and -0.0 are not greater, +inf and
/// positive subnormals are).
pub fn body_supply_enabled(world: &impl PostLoadWorld, actor: Id) -> Result<bool> {
    if !world.unit_supply_enabled()? || world.has_component(actor, ComponentSlot::Organization)? {
        return Ok(false);
    }
    Ok(f32::from_bits(world.body_shortfall_rate_bits(actor)?) > 0.0)
}

/// `561C60`: [`body_supply_enabled`], no Element organization (`+80`
/// component's `+10`), and an unresolved `+16C`.
pub fn body_needs_supply(world: &impl PostLoadWorld, actor: Id) -> Result<bool> {
    if !body_supply_enabled(world, actor)? || world.element_organization(actor)?.is_some() {
        return Ok(false);
    }
    Ok(!world.live(world.word_16c(actor)?)?)
}

/// Body PostLoad `561DF0`: schedules queue 1 / data 0 when supply ticks are
/// needed and absent; removes them when no longer needed.
pub fn body_post_load<W: PostLoadWorld>(world: &mut W, actor: Id) -> Result<()> {
    let at = Updatable {
        actor,
        slot: ComponentSlot::Body,
    };
    if body_needs_supply(world, actor)? && !world.has_scheduled(at, 1, 0)? {
        world.add_periodic(at, 1, 0)?;
        return Ok(());
    }
    if !body_needs_supply(world, actor)? && world.has_scheduled(at, 1, 0)? {
        world.remove_scheduled(at, 1, 0)?;
    }
    Ok(())
}

/// Properties / Supply PostLoad `581260`: `50AC60(0)` when the Updatable
/// list is empty.
pub fn scheduled_post_load<W: PostLoadWorld>(world: &mut W, at: Updatable) -> Result<()> {
    if !world.has_any_scheduled(at)? {
        world.add_periodic(at, 1, 0)?;
    }
    Ok(())
}

/// Wall PostLoad `58CB40`: maps older than version 26 cancel the Updatable
/// and reschedule queue 1 when ThingData `+42C` is set.
pub fn wall_post_load<W: PostLoadWorld>(world: &mut W, actor: Id) -> Result<()> {
    if world.world_version()? < 0x1a {
        let at = Updatable {
            actor,
            slot: ComponentSlot::Wall,
        };
        world.cancel_scheduled(at)?;
        if world.wall_flag_42c(actor)? {
            world.add_periodic(at, 1, 0)?;
        }
    }
    Ok(())
}

/// Denizen PostLoad `569F80`: every live command member with a Mind gets
/// the command (`52A020`). The command's transient reference (`+4` up, then
/// down) cannot reach zero while the Denizen list holds it.
pub fn denizen_post_load<W: PostLoadWorld>(world: &mut W, actor: Id) -> Result<()> {
    for command in world.denizen_commands(actor)?.into_iter().flatten() {
        for member in world.command_members(&command)? {
            if world.live(member)? {
                require_actor(world, member, "569FFF (+70 Mind)")?;
                if world.has_component(member, ComponentSlot::Mind)? {
                    world.set_mind_command(member, &command)?;
                }
            }
        }
    }
    Ok(())
}

/// `4CE6C0`: the actor itself when it has an Organization component or no
/// Element organization, else the Element's organization (`+80`, `+10`).
pub fn organization_of(world: &impl PostLoadWorld, actor: Id) -> Result<Id> {
    require_actor(world, actor, "4CE6C0")?;
    if world.has_component(actor, ComponentSlot::Organization)? {
        return Ok(actor);
    }
    Ok(world.element_organization(actor)?.unwrap_or(actor))
}

/// `4D0CD0(organization)` -> `4F1430`: removes the first `+118` node equal
/// to `value`.
pub fn remove_from_list_118<W: PostLoadWorld>(world: &mut W, actor: Id, value: Id) -> Result<bool> {
    require_actor(world, actor, "4D0CD0 (+118)")?;
    let values = world.actor_list(actor, ActorList::List118)?;
    match values.iter().position(|v| *v == value) {
        Some(index) => {
            world.remove_actor_list_node(actor, ActorList::List118, index)?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Defender PostLoad `5676B0`.
pub fn defender_post_load<W: PostLoadWorld>(world: &mut W, actor: Id) -> Result<()> {
    let mut index = 0;
    loop {
        let entries = world.defender_entries(actor)?;
        let Some(&[key, value]) = entries.get(index) else {
            break;
        };
        if !world.live(key)? {
            if world.live(value)? {
                let organization = organization_of(world, actor)?;
                remove_from_list_118(world, value, organization)?;
            }
            world.remove_defender_entry(actor, index)?;
        } else {
            if !world.live(value)? {
                let organization = organization_of(world, key)?;
                world.set_defender_entry_value(actor, index, organization)?;
            }
            index += 1;
        }
    }
    let at = Updatable {
        actor,
        slot: ComponentSlot::Defender,
    };
    if world.defender_word_1c(actor)? == 0 {
        let count = world.defender_ability_count(actor)?;
        for ability in 0..count {
            defender_clear_ability_target(world, actor, ability)?;
        }
        world.cancel_scheduled(at)?;
    } else {
        if !world.has_scheduled(at, 0, 0)? {
            world.add_periodic(at, 0, 0)?;
        }
        if !world.has_scheduled(at, 3, 0)? {
            world.add_after(at, 3, 0, DEFENDER_QUEUE3_DELAY_BITS)?;
        }
    }
    Ok(())
}

/// `567140(ability, null)`: an Ability whose `+14` target is a live actor
/// drops its first `{target, value}` entry (a live value first loses this
/// Defender's organization from its `+118`); the target becomes 0. A dead
/// target is left unchanged.
pub fn defender_clear_ability_target<W: PostLoadWorld>(world: &mut W, actor: Id, ability: usize) -> Result<()> {
    let target = world.ability_target(actor, ability)?;
    if !world.live(target)? {
        return Ok(());
    }
    if world.object_class(target)?.is_actor() {
        let entries = world.defender_entries(actor)?;
        let found = entries.iter().position(|e| e[0] == target);
        let value = found.map_or(0, |i| entries[i][1]);
        if world.live(value)? {
            let organization = organization_of(world, actor)?;
            remove_from_list_118(world, value, organization)?;
        }
        if let Some(index) = found {
            world.remove_defender_entry(actor, index)?;
        }
    }
    world.set_ability_target(actor, ability, 0)?;
    if world.live(0)? {
        return Err(Error::Unsupported("567140 attach branch for a live GTID 0"));
    }
    Ok(())
}

/// Hq PostLoad `5736B0`.
pub fn hq_post_load<W: PostLoadWorld>(world: &mut W, actor: Id) -> Result<()> {
    world.hq_release_items(actor)?;
    if world.hq_index(actor)? != -1 {
        return Ok(());
    }
    let kingdom = world.actor_kingdom(actor)?;
    if let Some(kingdom) = kingdom
        && world.hq_index(actor)? == -1
        && world.kingdom_hq_slots(kingdom)?.is_some()
    {
        let index = kingdom_allocate_hq_index(world, kingdom)?;
        world.set_hq_index(actor, index)?;
    }
    Ok(())
}

/// `4E2640`: `random % slots` (unsigned), or the lowest free index when
/// taken; -1 without a free index (nothing recorded) or with no slots (no
/// draw).
pub fn kingdom_allocate_hq_index<W: PostLoadWorld>(world: &mut W, kingdom: Id) -> Result<i32> {
    let slots = world
        .kingdom_hq_slots(kingdom)?
        .ok_or(Error::NullDereference("4E2643: Kingdom+234 is null"))?;
    if slots <= 0 {
        return Ok(-1);
    }
    let drawn = (world.random()? % slots as u32) as i32;
    let taken = world.kingdom_hq_taken(kingdom)?;
    let index = if taken.contains(&drawn) {
        (0..slots).find(|i| !taken.contains(i)).unwrap_or(-1)
    } else {
        drawn
    };
    if index != -1 {
        world.push_kingdom_hq_taken(kingdom, index)?;
    }
    Ok(index)
}

/// GKingdom::PostLoad `4E4220`.
pub fn kingdom_post_load<W: PostLoadWorld>(world: &mut W, kingdom: Id) -> Result<()> {
    actor_post_load(world, kingdom)?;
    if world.kingdom_group_count(kingdom)? > 0 {
        let mut group = 0usize;
        loop {
            let mut member = 0;
            loop {
                let members = world.kingdom_group_members(kingdom, group)?;
                let Some(&id) = members.get(member) else {
                    break;
                };
                if world.live(id)? {
                    member += 1;
                } else {
                    let first = members.iter().position(|v| *v == id).unwrap_or(member);
                    world.remove_kingdom_group_member(kingdom, group, first)?;
                }
            }
            if world.kingdom_group_members(kingdom, group)?.is_empty() {
                world.remove_kingdom_group(kingdom, group)?;
            } else {
                group += 1;
            }
            if i64::try_from(group).unwrap_or(i64::MAX) >= i64::from(world.kingdom_group_count(kingdom)?) {
                break;
            }
        }
    }
    let count = world.recruit_item_count(kingdom)?;
    for index in 0..count.max(0) as usize {
        let item = world
            .recruit_item_at(kingdom, index)?
            .ok_or(Error::NullDereference("4E4349: recruit item index past the current +218 count"))?;
        if world.recruit_item_byte_8(&item)? != 0 {
            continue;
        }
        let organization = world.recruit_item_organization(&item)?;
        if organization == 0 || (world.live(organization)? && world.object_flags(organization)? & 1 == 0) {
            continue;
        }
        let name = world.recruit_item_name(&item)?;
        world.map_warning(RECRUIT_ITEM_DEAD_ACTOR, name)?;
        world.set_recruit_item_available(&item)?;
        detach_recruit_item(world, &item)?;
    }
    Ok(())
}

/// `4E1C70(-1)` as `4E4220` calls it: a live `+10` organization with an
/// Organization component reaches `578DA0(item, -1)`, which reads and clears
/// Organization `+34[-1]` (the word before the slot buffer: undefined, an
/// explicit error here); without one, its `+E8` Kingdom `+224` is bumped.
/// `+10` becomes 0.
pub fn detach_recruit_item<W: PostLoadWorld>(world: &mut W, item: &W::RecruitItem) -> Result<()> {
    let organization = world.recruit_item_organization(item)?;
    if world.live(organization)? {
        require_actor(world, organization, "4E1C9A (+7C, vt+108)")?;
        if world.has_component(organization, ComponentSlot::Organization)? {
            return Err(Error::Undefined(
                "578DA0(item, -1) from 4E4220 reads and clears Organization+34[-1]".into(),
            ));
        }
        if let Some(kingdom) = world.actor_kingdom(organization)? {
            world.increment_kingdom_224(kingdom)?;
        }
    }
    world.set_recruit_item_organization(item, 0)
}

/// `4BB000` on `[B39F58]`: the single shared port
/// (`component_virtuals::discovery_zone_radius`).
pub use crate::component_virtuals::discovery_zone_radius;

/// Virtual `+12C`: GObject `4ECC50` for the GObject classes; GActor `4CFF70`
/// (`4ECC50`, then the settlement-space zone 2 moves to the actor with the
/// `4BB000` radius) for the GActor-derived ones.
pub fn required_object_lost<W: PostLoadWorld>(world: &mut W, object: Id) -> Result<()> {
    let class = world.object_class(object)?;
    match class {
        PostLoadClass::Inert => {
            return Err(Error::Undefined(format!("virtual +12C on {object:#x} ({class:?})")));
        }
        PostLoadClass::Object => return gobject_required_lost(world, object),
        _ => {}
    }
    gobject_required_lost(world, object)?;
    if world.settlement_space(object)? {
        let pc = world.precision();
        let radius = discovery_zone_radius(world.tile_size_bits()?, world.thing_manager_74_bits()?, pc)?;
        let xy = world.object_position(object)?;
        if world.zone_count(object)? <= 2 {
            return Err(Error::NullDereference("4CFFAE: 515F40 on a null zone 2"));
        }
        world.move_zone(object, 2, xy, radius)?;
    }
    Ok(())
}

/// `4ECC50`: with a non-zero `+50`: clear it, virtual `+50(ThingData+1C bit
/// 10)`, clear flag bit 30, then for every kingdom of GWorld `+154` but the
/// last (non-null, `+190 == 0`) while the object keeps memory (`+58 > 0`),
/// refresh its memory record for that kingdom (`4EC4A0`, wrapped by the UI
/// agent's `79DEB0`/`79E7B0`).
pub fn gobject_required_lost<W: PostLoadWorld>(world: &mut W, object: Id) -> Result<()> {
    if world.required_object(object)? == 0 {
        return Ok(());
    }
    world.clear_required_object(object)?;
    let blocking = (world.definition_flags_1c(object)? >> 10) & 1 != 0;
    world.set_blocking(object, blocking)?;
    let flags = world.object_flags(object)?;
    world.set_object_flags(object, flags & !0x4000_0000)?;
    let mut index = 0usize;
    loop {
        let kingdoms = world.world_kingdoms()?;
        if index as i64 >= kingdoms.len() as i64 - 1 {
            break;
        }
        if let Some(kingdom) = kingdoms[index]
            && world.kingdom_word_190(kingdom)? == 0
            && world.memory_count(object)? > 0
        {
            let slot = world.kingdom_index(kingdom)?;
            if let Some(record) = world.memory_record(object, slot)? {
                if world.ui_agent_active()? {
                    world.ui_memory_record(Some(record), kingdom, false)?;
                }
                world.refresh_memory_record(object, record)?;
                if world.ui_agent_active()? {
                    let again = world.memory_record(object, slot)?;
                    world.ui_memory_record(again, kingdom, true)?;
                }
            }
        }
        index += 1;
    }
    Ok(())
}

/// `586BC0(stop)` (Settlement PostLoad `586F60` passes 0): for every entry
/// `i < +28` a GACD block (vtable A25B28; defaults of `4C8C20`/`4C8C70`)
/// with definition `45D7C0(kind)`, position owner `+20/+24` + offset (x87
/// `fadd` at the thread precision, stored binary32) and owner `+28`,
/// direction from the entry and owner = the owner's virtual `+108` goes to
/// `4F9C10`; a created object's GTID is stored at `+3C[i]`. A null result
/// fails the call (returned when `stop`, otherwise after the loop).
pub fn settlement_create_things<W: PostLoadWorld>(world: &mut W, actor: Id, stop: bool) -> Result<bool> {
    let owner = world.actor_kingdom(actor)?.unwrap_or(0);
    let mut failed = false;
    let mut index = 0usize;
    while (index as i64) < i64::from(world.settlement_count(actor)?) {
        let entry = world.settlement_entry(actor, index)?;
        let definition = world
            .settlement_definition(actor, entry.kind)?
            .ok_or(Error::NullDereference("4F9C92: 4F9C10 with a null definition"))?;
        let pc = world.precision();
        let origin = world.object_position(actor)?;
        let x = Binary::from_f32_bits(entry.offset_bits[0])?
            .add(&Binary::from_f32_bits(origin[0])?, pc)?
            .store_f32()?;
        let y = Binary::from_f32_bits(entry.offset_bits[1])?
            .add(&Binary::from_f32_bits(origin[1])?, pc)?
            .store_f32()?;
        let mut params = ActorParams::new(definition);
        params.position_bits = [x, y, world.object_z(actor)?];
        params.direction_bits = entry.direction_bits;
        params.owner = owner;
        match world.create_gthing(params)? {
            Some(id) => world.set_settlement_thing(actor, index, id)?,
            None => {
                failed = true;
                if stop {
                    return Ok(false);
                }
            }
        }
        index += 1;
    }
    Ok(!failed)
}

/// `4DDCD0(&+1A4, &out)`: a position target (flag bit 2) is clamped
/// (`552640`) and written back (flag bit 2 set again); an object target (flag
/// bit 0) with a live `+10` yields that object's clamped position.
pub fn group_target_position<W: PostLoadWorld>(
    world: &mut W,
    group: Id,
    target: GroupTarget,
) -> Result<Option<[u32; 2]>> {
    if target.flags & 4 != 0 {
        let xy = world.clamp_position(target.position_bits)?;
        world.set_group_target(
            group,
            GroupTarget {
                flags: target.flags | 4,
                position_bits: xy,
                ..target
            },
        )?;
        return Ok(Some(xy));
    }
    if target.flags & 1 != 0 && world.live(target.object)? {
        let xy = world.object_position(target.object)?;
        return Ok(Some(world.clamp_position(xy)?));
    }
    Ok(None)
}

/// GGroup PostLoad `4DDFA0`: `4D3560`, then a `+1A4` target with a position
/// moves zone 7 there with radius `+1B4` (`515F40`).
pub fn group_post_load<W: PostLoadWorld>(world: &mut W, group: Id) -> Result<()> {
    actor_post_load(world, group)?;
    let Some(target) = world.group_target(group)? else {
        return Ok(());
    };
    if let Some(xy) = group_target_position(world, group, target)? {
        if world.zone_count(group)? <= 7 {
            return Err(Error::NullDereference("4DE010: 515F40 on a null zone 7"));
        }
        let radius = world.group_radius_bits(group)?;
        world.move_zone(group, 7, xy, radius)?;
    }
    Ok(())
}
