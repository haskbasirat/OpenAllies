//!57AD30 ->4FBF40 ->57D950 on the same staged actor, spatial and World owners.
//!Native stack creation blocks are owned locally; installed components are
//!borrowed for each field access and never extracted or replaced.
use crate::{
    actor_creation_startup::StartupInputs,
    actor_world::{ActorStore, ObjectKey, RuntimeComponent, Source},
    catalog::Catalog,
    map_actor_creation::{self, Params},
    map_replacement_queue::DefinitionRef,
    restoration::Staged,
};
use aa_actors::{
    ActorId,
    components::ComponentSlot as Slot,
    ground_states::OrganizationState,
    organization_spawn,
    world_creation::{self, SpawnQueries, SpawnRequest, SpawnWorld},
};
use aa_mapworld::{fresh_creation::CompletionWorld, restore::RestoreDefinitions};
use aa_numeric::Precision;

type Result<T> = std::result::Result<T, String>;
#[path = "stored_organization_spawn_fields.rs"]
mod fields;
use fields::{NotifyFields, SlotFields, spawn_slot_fields};
fn text(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn error(e: impl std::fmt::Display) -> world_creation::Error {
    world_creation::Error::World(e.to_string())
}
fn required<T>(s: &str) -> Result<T> {
    Err(s.to_owned())
}
fn org(store: &ActorStore, key: ObjectKey) -> Result<&OrganizationState<String>> {
    let owner = store
        .id(key)
        .ok_or("member spawn actual Organization owner is retired")?;
    match store.component(owner, Slot::Organization).map_err(text)? {
        Some(RuntimeComponent::Organization(v)) if v.header.owner == owner => Ok(v),
        _ => Err("member spawn actual Organization component/owner differs".into()),
    }
}
fn source_precision(staged: &Staged, catalog: &Catalog<'_>) -> Result<Precision> {
    catalog.require_source_owner()?;
    let source = staged.definitions.combat_source()?;
    source.completed_means()?;
    if source.source_identities() != catalog.actors.sources.as_slice() {
        return Err("member spawn current Bank/Catalog source differs".into());
    }
    Ok(source.source_precision())
}
fn definition(staged: &Staged, key: ObjectKey) -> Result<DefinitionRef> {
    let value = &staged.actors.object(key).map_err(text)?.definition;
    let token = staged
        .definitions
        .token("ActorDefinition", &value.proof.id)
        .ok_or("member spawn actual ActorDefinition identity absent from current Bank")?;
    staged.definitions.require_usable(token)?;
    Ok(DefinitionRef {
        token,
        canonical_ids: value.proof.id.clone(),
    })
}
fn slot_definition(
    staged: &Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    slot: i32,
    pc: Precision,
) -> Result<DefinitionRef> {
    let current = org(&staged.actors, key)?;
    let info = catalog.organization_layout_info(&current.layout.definition, pc)?;
    let index = usize::try_from(slot).map_err(|_| "member spawn negative native slot")?;
    let group = *info
        .slot_groups
        .get(index)
        .ok_or("member spawn native slot beyond current LayoutData28")? as usize;
    let id = current
        .layout
        .parameters
        .get(group)
        .and_then(Option::as_ref)
        .ok_or("member spawn native null/out-of-range current slot definition")?;
    catalog.require_untranslated_actor_identity(id)?;
    let token = staged
        .definitions
        .token("ActorDefinition", id)
        .ok_or("member spawn slot ActorDefinition absent from same Bank")?;
    staged.definitions.require_usable(token)?;
    Ok(DefinitionRef {
        token,
        canonical_ids: id.clone(),
    })
}
fn occupied(staged: &Staged, key: ObjectKey, slot: i32) -> Result<bool> {
    let index = usize::try_from(slot).map_err(|_| "member spawn negative native slot")?;
    Ok(!org(&staged.actors, key)?
        .members
        .get(index)
        .ok_or("member spawn slot beyond actual member array")?
        .is_null())
}
fn placement(staged: &Staged, key: ObjectKey) -> Result<crate::actor_world::PlacementState> {
    if let Ok(v) = staged.actors.staged_placement(key) {
        return Ok(v);
    }
    let world = staged
        .load_world
        .as_ref()
        .ok_or("member spawn actual World placement absent")?;
    staged
        .actors
        .placement(key, world.live_spatial()?)
        .map_err(text)
}
fn allegiance(staged: &Staged, key: ObjectKey) -> Result<u32> {
    let actor = staged.actors.object(key).map_err(text)?;
    if actor.definition.native_class != 3 {
        return Err("member spawn virtual108 ordinary class3 required".into());
    }
    let id = staged
        .actors
        .id(key)
        .ok_or("member spawn actual current Actor GTID absent")?;
    Ok(staged
        .actors
        .ordinary_owner(id)
        .map_err(text)?
        .map_or(0, ActorId::raw))
}

struct SlotOwner<'a, 'source> {
    staged: &'a mut Staged,
    catalog: &'a Catalog<'source>,
    key: ObjectKey,
    startup: &'a StartupInputs,
    source_pc: Precision,
    pc: Precision,
}
impl SlotFields for SlotOwner<'_, '_> {
    type Definition = DefinitionRef;
    fn occupied(&mut self, s: i32) -> Result<bool> {
        occupied(self.staged, self.key, s)
    }
    fn definition(&mut self, s: i32) -> Result<DefinitionRef> {
        slot_definition(self.staged, self.catalog, self.key, s, self.source_pc)
    }
    fn allegiance(&mut self) -> Result<u32> {
        allegiance(self.staged, self.key)
    }
    fn create(&mut self, params: &mut Params, slot: i32) -> Result<bool> {
        let request = SpawnRequest {
            structure_definitions: Vec::new(),
            word_28: params.owner,
            byte_50: params.byte_50,
            byte_51: params.byte_51,
        };
        let mut bound = Member {
            staged: self.staged,
            catalog: self.catalog,
            startup: self.startup,
            params,
            source_pc: self.source_pc,
            pc: self.pc,
        };
        world_creation::spawn_member_with_queries::<DefinitionRef, _, LiveQueries>(
            &request, self.key, slot, &mut bound, self.pc,
        )
        .map(|v| v.is_some())
        .map_err(text)
    }
}
///57AD30 borrows the actual current component and constructs its real stack
///ActorParams. All later partial allocation/membership effects remain staged.
pub(crate) fn spawn_slot(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    slot: i32,
    flag: u8,
    startup: &StartupInputs,
    pc: Precision,
) -> Result<bool> {
    let source_pc = source_precision(staged, catalog)?;
    spawn_slot_fields(
        &mut SlotOwner {
            staged,
            catalog,
            key,
            startup,
            source_pc,
            pc,
        },
        slot,
        flag,
    )
}

///4FA150's post-create member pass. The caller has already completed ordinary
///class3 creation (its native B4 IsGActor predicate is true).
pub(crate) fn finish(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    params: &Params,
    key: ObjectKey,
    startup: &StartupInputs,
    pc: Precision,
) -> Result<Option<ObjectKey>> {
    let source_pc = source_precision(staged, catalog)?;
    if staged
        .actors
        .object(key)
        .map_err(text)?
        .definition
        .native_class
        != 3
    {
        return Err("4FA150 completion requires actual class3 IsGActor result".into());
    }
    crate::fresh_organization_completion::finish_organization_members(
        &mut Completion {
            staged,
            catalog,
            params,
            startup,
            source_pc,
            pc,
            captured: None,
        },
        key,
        |current| Ok(current.params.byte_51),
    )
    .map_err(text)
}
///The actual4FA3D0 tail after4FA150. A null result skips every Organization,
///source and recruitment read. Positive hero attachment remains a child.
pub(crate) fn hero_postpass(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    created: Option<ObjectKey>,
) -> Result<Option<ObjectKey>> {
    let Some(actor) = created else {
        return Ok(None);
    };
    let source_pc = source_precision(staged, catalog)?;
    crate::fresh_organization_completion::hero_postpass(
        &mut Hero {
            staged,
            catalog,
            source_pc,
            captured: None,
        },
        Some(actor),
    )
}
struct Completion<'a, 'source> {
    staged: &'a mut Staged,
    catalog: &'a Catalog<'source>,
    params: &'a Params,
    startup: &'a StartupInputs,
    source_pc: Precision,
    pc: Precision,
    captured: Option<*const OrganizationState<String>>,
}
impl Completion<'_, '_> {
    fn captured(&self, key: ObjectKey) -> Result<&OrganizationState<String>> {
        let current = org(&self.staged.actors, key)?;
        if !self.captured.is_some_and(|p| std::ptr::eq(p, current)) {
            return Err("4FA150 captured Organization allocation was replaced".into());
        }
        Ok(current)
    }
}
impl CompletionWorld for Completion<'_, '_> {
    type Object = ObjectKey;
    type Error = String;
    fn organization(&mut self, key: &ObjectKey) -> Result<bool> {
        let object = self.staged.actors.object(*key).map_err(text)?;
        if !object
            .components()
            .map_err(text)?
            .present(Slot::Organization)
        {
            return Ok(false);
        }
        self.captured = Some(org(&self.staged.actors, *key)? as *const _);
        Ok(true)
    }
    fn leader_slot(&mut self, key: &ObjectKey) -> Result<i32> {
        Ok(self
            .catalog
            .organization_layout_info(&self.captured(*key)?.layout.definition, self.source_pc)?
            .primary_slot)
    }
    fn organization_slot_count(&mut self, key: &ObjectKey) -> Result<i32> {
        let current = self.captured(*key)?;
        let data = self.catalog.layout_data(self.source_pc)?;
        let layout = data
            .layouts
            .iter()
            .find(|v| v.ids.eq_ignore_ascii_case(&current.layout.definition))
            .ok_or("4FA150 captured current layout source absent")?;
        i32::try_from(layout.slot_groups.len()).map_err(text)
    }
    fn slot_defined(&mut self, key: &ObjectKey, slot: i32) -> Result<bool> {
        let current = self.captured(*key)?;
        let info = self
            .catalog
            .organization_layout_info(&current.layout.definition, self.source_pc)?;
        let index = usize::try_from(slot).map_err(text)?;
        let group = *info
            .slot_groups
            .get(index)
            .ok_or("4FA150 captured slot source absent")? as usize;
        Ok(current
            .layout
            .parameters
            .get(group)
            .ok_or("4FA150 captured parameter source absent")?
            .is_some())
    }
    fn spawn_member(&mut self, key: &ObjectKey, slot: i32, flag: u8) -> Result<()> {
        spawn_slot(
            self.staged,
            self.catalog,
            *key,
            slot,
            flag,
            self.startup,
            self.pc,
        )
        .map(|_| ())
    }
    fn member_count(&mut self, key: &ObjectKey) -> Result<u32> {
        u32::try_from(
            org(&self.staged.actors, *key)?
                .members
                .iter()
                .filter(|id| !id.is_null())
                .count(),
        )
        .map_err(text)
    }
    fn log_no_members(&mut self, _: &ObjectKey) -> Result<()> {
        required("4FA150 actual404F60 no-members diagnostic before destruction")
    }
    fn destroy(&mut self, _: ObjectKey) -> Result<()> {
        required("4FA150 actual4F5490 no-members destruction")
    }
}

struct Member<'a, 'source> {
    staged: &'a mut Staged,
    catalog: &'a Catalog<'source>,
    startup: &'a StartupInputs,
    params: &'a mut Params,
    source_pc: Precision,
    pc: Precision,
}
struct LiveQueries;
impl SpawnQueries<DefinitionRef, Member<'_, '_>> for LiveQueries {
    type Cursor = aa_spatial::rectangle_actors::Cursor;
    fn begin(w: &mut Member<'_, '_>, p: [u32; 2], r: u32) -> world_creation::Result<Self::Cursor> {
        map_actor_creation::begin_recruit_candidates(w.staged, w.catalog, p, r, w.pc).map_err(error)
    }
    fn next(
        w: &mut Member<'_, '_>,
        c: &mut Self::Cursor,
    ) -> world_creation::Result<Option<ObjectKey>> {
        map_actor_creation::next_recruit_candidate(w.staged, w.catalog, c, w.pc).map_err(error)
    }
    fn finish(w: &mut Member<'_, '_>, c: &mut Self::Cursor) -> world_creation::Result<()> {
        map_actor_creation::finish_recruit_candidates(w.staged, c).map_err(error)
    }
}
impl Member<'_, '_> {
    fn source(&self, d: &DefinitionRef) -> Result<crate::stored_member_deployment::SourceFields> {
        let b = self
            .catalog
            .actor_bindings(&d.canonical_ids, self.source_pc)?;
        crate::stored_member_deployment::SourceFields::bind(
            self.catalog,
            &self.staged.definitions,
            &b.definition,
        )
    }
    fn pair(&mut self, organization: ObjectKey, slot: i32, member: ObjectKey) -> Result<()> {
        let index = usize::try_from(slot).map_err(text)?;
        let value = *org(&self.staged.actors, organization)?
            .words_34
            .get(index)
            .ok_or("57D860 actual recruit slot absent")?;
        if value != 0 {
            return required(
                "4CEA60 actual current recruitment item pairing after member creation",
            );
        }
        let id = self
            .staged
            .actors
            .id(member)
            .ok_or("57CBA0 created member identity absent")?;
        crate::stored_organization_members::add_member(
            self.catalog,
            self.staged,
            organization,
            slot,
            id,
            self.pc,
        )?;
        notify_member(self.staged, self.catalog, organization, member, self.pc)
    }
}
impl SpawnWorld<DefinitionRef> for Member<'_, '_> {
    type Object = ObjectKey;
    type Owner = ActorId;
    fn game_mode(&self) -> world_creation::Result<u32> {
        self.catalog
            .load_session
            .ok_or_else(|| error("4FBF40 actual current SessionE4 absent"))?
            .application_state()
            .map_err(error)
    }
    fn actor_position(&self, k: ObjectKey) -> world_creation::Result<[u32; 2]> {
        let p = placement(self.staged, k).map_err(error)?;
        Ok([p.position_bits[0], p.position_bits[1]])
    }
    fn actor_facing(&self, k: ObjectKey) -> world_creation::Result<[u32; 2]> {
        let p = placement(self.staged, k).map_err(error)?;
        Ok([p.facing_bits[0], p.facing_bits[1]])
    }
    fn actor_gtid(&self, k: ObjectKey) -> world_creation::Result<u32> {
        self.staged
            .actors
            .id(k)
            .map(ActorId::raw)
            .ok_or_else(|| error("4FBF40 current Actor14 absent"))
    }
    fn organization_radius(&self, k: ObjectKey) -> world_creation::Result<u32> {
        org(&self.staged.actors, k)
            .map_err(error)?
            .words
            .get(&0xac)
            .copied()
            .ok_or_else(|| error("4FBF40 actual OrganizationAC unwritten"))
    }
    fn organization_position(&self, k: ObjectKey) -> world_creation::Result<[u32; 2]> {
        Ok(org(&self.staged.actors, k)
            .map_err(error)?
            .position_xy
            .map(|v| v.0))
    }
    fn slot_offset(&self, k: ObjectKey, s: i32) -> world_creation::Result<[u32; 2]> {
        org(&self.staged.actors, k)
            .map_err(error)?
            .offsets_78
            .get(usize::try_from(s).map_err(error)?)
            .map(|p| p.map(|v| v.0))
            .ok_or_else(|| error("4FBF40 actual Organization78 slot absent"))
    }
    fn query(&mut self, _: [u32; 2], _: u32) -> world_creation::Result<Vec<ObjectKey>> {
        Err(error("4FBF40 product requires SAME live query cursor"))
    }
    fn owner(&mut self, k: ObjectKey) -> world_creation::Result<Option<ActorId>> {
        allegiance(self.staged, k)
            .map(|v| (v != 0).then_some(ActorId::from_raw(v)))
            .map_err(error)
    }
    fn related(&mut self, o: ActorId, other: Option<ActorId>) -> world_creation::Result<bool> {
        if Some(o) == other {
            return Ok(true);
        };
        crate::relations::borrowed::kingdom(
            &RelationRead {
                actors: &self.staged.actors,
            },
            o,
            other,
        )
        .map(|v| v == 1)
        .map_err(error)
    }
    fn definition(&self, k: ObjectKey) -> world_creation::Result<DefinitionRef> {
        definition(self.staged, k).map_err(error)
    }
    fn recruit_structure(&self, d: &DefinitionRef) -> world_creation::Result<bool> {
        self.source(d).map(|s| s.enabled_3ec()).map_err(error)
    }
    fn accepts_structure(
        &self,
        _: &SpawnRequest<DefinitionRef>,
        d: &DefinitionRef,
    ) -> world_creation::Result<bool> {
        let b = self
            .catalog
            .actor_bindings(&self.params.definition.canonical_ids, self.source_pc)
            .map_err(error)?;
        let current = crate::stored_member_deployment::RecruitingStructures::bind(
            self.catalog,
            &self.staged.definitions,
            &b.definition,
        )
        .map_err(error)?;
        Ok(current
            .ids()
            .iter()
            .any(|id| id.eq_ignore_ascii_case(&d.canonical_ids)))
    }
    fn object_position(&self, k: ObjectKey) -> world_creation::Result<[u32; 2]> {
        self.actor_position(k)
    }
    fn create_at_structure(
        &mut self,
        _: &SpawnRequest<DefinitionRef>,
        id: u32,
        k: ObjectKey,
    ) -> world_creation::Result<Option<ObjectKey>> {
        self.params.word_40 = id;
        let pc = self.pc;
        let mut deployment = Deployment {
            staged: self.staged,
            catalog: self.catalog,
            startup: self.startup,
            source_pc: self.source_pc,
            pc,
        };
        crate::stored_member_deployment::deploy(&mut deployment, self.params, k, pc).map_err(error)
    }
    fn set_member(&mut self, k: ObjectKey, s: i32, m: ObjectKey) -> world_creation::Result<()> {
        self.pair(k, s, m).map_err(error)
    }
    fn slot_placement(
        &mut self,
        _: ObjectKey,
        _: i32,
        _: [u32; 2],
        _: [u32; 2],
    ) -> world_creation::Result<bool> {
        Err(error("578290 product requires mutable actual XY writeback"))
    }
    fn slot_placement_at(
        &mut self,
        k: ObjectKey,
        s: i32,
        p: &mut [u32; 2],
        f: [u32; 2],
    ) -> world_creation::Result<bool> {
        let d = slot_definition(self.staged, self.catalog, k, s, self.source_pc).map_err(error)?;
        member_slot_placement(self.staged, self.catalog, &d, p, f, self.pc).map_err(error)
    }
    fn create_member(
        &mut self,
        k: ObjectKey,
        s: i32,
        p: [u32; 3],
        f: [u32; 2],
        a: u8,
        b: u8,
    ) -> world_creation::Result<Option<ObjectKey>> {
        organization_spawn::create_member(self, k, s, p, f, a, b)
    }
}
impl organization_spawn::World for Member<'_, '_> {
    type Object = ObjectKey;
    type Definition = DefinitionRef;
    type Layout = String;
    type RecruitItem = (ObjectKey, i32, u32);
    fn occupied(&mut self, k: ObjectKey, s: i32) -> world_creation::Result<bool> {
        occupied(self.staged, k, s).map_err(error)
    }
    fn slot_definition(&mut self, k: ObjectKey, s: i32) -> world_creation::Result<DefinitionRef> {
        slot_definition(self.staged, self.catalog, k, s, self.source_pc).map_err(error)
    }
    fn allegiance(&mut self, k: ObjectKey) -> world_creation::Result<u32> {
        allegiance(self.staged, k).map_err(error)
    }
    fn owner_gtid(&mut self, k: ObjectKey) -> world_creation::Result<u32> {
        self.actor_gtid(k)
    }
    fn create(&mut self, p: &mut Params) -> world_creation::Result<Option<ObjectKey>> {
        map_actor_creation::create_member_thing(self.staged, self.catalog, p, self.startup, self.pc)
            .map_err(error)
    }
    fn recruit_item(
        &mut self,
        k: ObjectKey,
        s: i32,
    ) -> world_creation::Result<Option<Self::RecruitItem>> {
        let word = *org(&self.staged.actors, k)
            .map_err(error)?
            .words_34
            .get(usize::try_from(s).map_err(error)?)
            .ok_or_else(|| error("57D950 actual recruit slot absent"))?;
        Ok((word != 0).then_some((k, s, word)))
    }
    fn pair_recruit_item(
        &mut self,
        _: ObjectKey,
        _: Self::RecruitItem,
    ) -> world_creation::Result<()> {
        Err(error(
            "4CEA60 actual current recruitment item pairing after member creation",
        ))
    }
    fn add_member(&mut self, k: ObjectKey, s: i32, m: ObjectKey) -> world_creation::Result<()> {
        let id = self
            .staged
            .actors
            .id(m)
            .ok_or_else(|| error("57CBA0 actual created GTID absent"))?;
        crate::stored_organization_members::add_member(self.catalog, self.staged, k, s, id, self.pc)
            .map_err(error)
    }
    fn notify_member(&mut self, k: ObjectKey, m: ObjectKey) -> world_creation::Result<()> {
        notify_member(self.staged, self.catalog, k, m, self.pc).map_err(error)
    }
}

///578290 ->554AD0; facing is the native stack copy, while XY is the caller's
///actual local and must retain its successful free-placement writeback.
fn member_slot_placement(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    d: &DefinitionRef,
    xy: &mut [u32; 2],
    facing: [u32; 2],
    pc: Precision,
) -> Result<bool> {
    let source_pc = source_precision(staged, catalog)?;
    let object = catalog
        .actors
        .native_object_definition(&d.canonical_ids, source_pc)
        .map_err(text)?;
    let flags = catalog
        .actors
        .native_final_flags(&d.canonical_ids, source_pc)
        .map_err(text)?;
    let shape = object.blocking.words;
    let world = staged
        .load_world
        .as_mut()
        .ok_or("578290 actual World absent")?;
    if world.terrain.source_precision() != source_pc
        || world.source_fields.source_precision() != source_pc
        || world.source_fields.receipt().sources != catalog.actors.sources
    {
        return Err("578290 actual source geometry differs".into());
    }
    let crate::load_world::SpatialOwner::Adopted(adopted) = &mut world.spatial else {
        return Err("578290 actual adopted spatial owner absent".into());
    };
    if adopted.error.is_some()
        || !adopted.pending.is_empty()
        || adopted.world.index().precision() != pc
    {
        return Err("578290 actual adoption/live precision differs".into());
    }
    let extent = world
        .terrain
        .bound
        .layout
        .extent_i32
        .map(|n| (n as f32).to_bits());
    let geometry =
        aa_spatial::query::QueryGeometry::from_native_extents(adopted.world.index(), extent, pc)
            .map_err(text)?;
    let radius = aa_spatial::placement::wrapper_radius(
        shape[1],
        adopted.world.index().grid().cell_size_bits,
        pc,
    )
    .map_err(text)?;
    let search = aa_spatial::placement::PlacementSearch {
        shape: aa_spatial::collision::Shape {
            kind: shape[0],
            radius_bits: shape[1],
            bounds: [shape[2], shape[3], shape[4], shape[5]],
        },
        radius_bits: radius,
        rings: 10,
        keep_direction: true,
        strict: false,
        large: flags & (1 << 21) != 0,
    };
    let (index, bindings) = adopted.world.split_index();
    let mut policy = map_actor_creation::PlacementPolicy {
        catalog,
        definitions: &staged.definitions,
        source_fields: &world.source_fields,
        bindings,
    };
    let mut stack_facing = facing;
    aa_spatial::placement::find_free_placement(
        index,
        &geometry,
        xy,
        &mut stack_facing,
        &search,
        &mut policy,
    )
    .map_err(text)
}

struct Deployment<'a, 'source> {
    staged: &'a mut Staged,
    catalog: &'a Catalog<'source>,
    startup: &'a StartupInputs,
    source_pc: Precision,
    pc: Precision,
}
impl crate::stored_member_deployment::World<DefinitionRef, String> for Deployment<'_, '_> {
    type Actor = ObjectKey;
    type Definition = DefinitionRef;
    fn definition(&mut self, k: ObjectKey) -> Result<DefinitionRef> {
        definition(self.staged, k)
    }
    fn enabled_3ec(&mut self, d: &DefinitionRef) -> Result<bool> {
        let b = self
            .catalog
            .actor_bindings(&d.canonical_ids, self.source_pc)?;
        Ok(crate::stored_member_deployment::SourceFields::bind(
            self.catalog,
            &self.staged.definitions,
            &b.definition,
        )?
        .enabled_3ec())
    }
    fn source_word(&mut self, d: &DefinitionRef, o: u32) -> Result<u32> {
        let b = self
            .catalog
            .actor_bindings(&d.canonical_ids, self.source_pc)?;
        crate::stored_member_deployment::SourceFields::bind(
            self.catalog,
            &self.staged.definitions,
            &b.definition,
        )?
        .word(o)
    }
    fn actor_word(&mut self, k: ObjectKey, o: u32) -> Result<u32> {
        let p = placement(self.staged, k)?;
        match o {
            0x20 => Ok(p.position_bits[0]),
            0x24 => Ok(p.position_bits[1]),
            0x2c => Ok(p.facing_bits[0]),
            0x30 => Ok(p.facing_bits[1]),
            _ => Err("4FACA0 unexpected actor word".into()),
        }
    }
    fn contains_46ec70(&mut self, key: ObjectKey, point: [u32; 2]) -> Result<bool> {
        let current = &self.staged.actors.object(key).map_err(text)?.definition;
        if current.native_class != 3 {
            return required("4FACA0 virtualC current class3 shape required");
        }
        //A273FC ->4EA010 is current ActorData+38, without an actor-state
        //override. The class3 getter has its own both-image byte/execution proof.
        crate::stored_member_deployment::SourceFields::bind(
            self.catalog,
            &self.staged.definitions,
            current,
        )?;
        let shape = self
            .catalog
            .actors
            .native_object_definition(&current.proof.id, self.source_pc)
            .map_err(text)?
            .blocking
            .words;
        crate::stored_member_deployment::fields::contains_46ec70(
            &aa_spatial::collision::Shape {
                kind: shape[0],
                radius_bits: shape[1],
                bounds: [shape[2], shape[3], shape[4], shape[5]],
            },
            point,
            self.pc,
        )
    }
    fn warn_inside_404ed0(&mut self, _: ObjectKey) -> Result<()> {
        required("4FACA0 actual404ED0 deployment-inside diagnostic")
    }
    fn create_4f9c10(&mut self, p: &mut Params) -> Result<Option<ObjectKey>> {
        map_actor_creation::create_member_thing(self.staged, self.catalog, p, self.startup, self.pc)
    }
    fn mind_deploy_198(&mut self, _: ObjectKey, _: [u32; 2], _: ObjectKey) -> Result<bool> {
        required("4FACA0 actual new member Mind198 deployment")
    }
    fn log_failed_404f60(&mut self, _: ObjectKey) -> Result<()> {
        required("4FACA0 actual404F60 failed deployment diagnostic")
    }
    fn destroy_4f6760(&mut self, _: ObjectKey, _: u8) -> Result<()> {
        required("4FACA0 actual4F6760 destruction")
    }
}

struct RelationRead<'a> {
    actors: &'a ActorStore,
}
struct Hero<'a, 'source> {
    staged: &'a mut Staged,
    catalog: &'a Catalog<'source>,
    source_pc: Precision,
    captured: Option<*const OrganizationState<String>>,
}
impl Hero<'_, '_> {
    fn current_captured(&self, key: ObjectKey) -> Result<&OrganizationState<String>> {
        let current = org(&self.staged.actors, key)?;
        if !self.captured.is_some_and(|p| std::ptr::eq(p, current)) {
            return Err("4FA3D0 original captured Organization allocation differs".into());
        }
        Ok(current)
    }
}
impl crate::fresh_organization_completion::HeroWorld for Hero<'_, '_> {
    type Actor = ObjectKey;
    type Organization = ObjectKey;
    type Definition = u32;
    type RecruitTable = ActorId;
    fn capture_organization(&mut self, key: ObjectKey) -> Result<ObjectKey> {
        self.captured = Some(org(&self.staged.actors, key)? as *const _);
        Ok(key)
    }
    fn slot_count(&mut self, key: ObjectKey) -> Result<i32> {
        let current = self.current_captured(key)?;
        let data = self.catalog.layout_data(self.source_pc)?;
        let layout = data
            .layouts
            .iter()
            .find(|l| l.ids.eq_ignore_ascii_case(&current.layout.definition))
            .ok_or("4FA3D0 captured LayoutData source absent")?;
        i32::try_from(layout.slot_groups.len()).map_err(text)
    }
    fn group_type(&mut self, key: ObjectKey, slot: i32) -> Result<i32> {
        let current = self.current_captured(key)?;
        let data = self.catalog.layout_data(self.source_pc)?;
        let layout = data
            .layouts
            .iter()
            .find(|l| l.ids.eq_ignore_ascii_case(&current.layout.definition))
            .ok_or("4FA3D0 captured LayoutData source absent")?;
        let group = *layout
            .slot_groups
            .get(usize::try_from(slot).map_err(text)?)
            .ok_or("4FA3D0 current slot group absent")? as usize;
        //The native group+8 signed word is LayoutSlotEntry.number.
        Ok(layout
            .entries
            .get(group)
            .ok_or("4FA3D0 current group entry absent")?
            .number)
    }
    fn current_member(&mut self, key: ObjectKey, slot: i32) -> Result<Option<ObjectKey>> {
        let id = *org(&self.staged.actors, key)?
            .members
            .get(usize::try_from(slot).map_err(text)?)
            .ok_or("4FA3D0 current Organization member slot absent")?;
        if id.is_null() {
            return Ok(None);
        }
        self.staged
            .actors
            .key(id)
            .map(Some)
            .ok_or("4FA3D0 current member generation absent".into())
    }
    fn member_definition(&mut self, key: ObjectKey) -> Result<u32> {
        definition(self.staged, key).map(|d| d.token)
    }
    fn first_owner_present_108(&mut self, key: ObjectKey) -> Result<bool> {
        Ok(allegiance(self.staged, key)? != 0)
    }
    fn second_owner_recruit_table_108(&mut self, key: ObjectKey) -> Result<Option<ActorId>> {
        let word = allegiance(self.staged, key)?;
        if word == 0 {
            return Err(
                "4FA3D0 second owner became NULL before native1FC table dereference".into(),
            );
        }
        //A live typed Kingdom owns its embedded1FC table. No native address
        //or wrapping pointer is fabricated from its portable GTID.
        let id = ActorId::from_raw(word);
        RelationRead {
            actors: &self.staged.actors,
        }
        .special(id, 4)?;
        Ok(Some(id))
    }
    fn recruit_contains_4e2a60(&mut self, owner: ActorId, token: u32, flag: u8) -> Result<bool> {
        if flag != 1 {
            return Err("4FA3D0 native recruit query argument differs from literal1".into());
        }
        let entry = self.staged.definitions.require_usable(token)?;
        if entry.key.domain != "actordefinition" {
            return Err("4FA3D0 captured Data definition domain differs".into());
        }
        Ok(RelationRead {
            actors: &self.staged.actors,
        }
        .special(owner, 4)?
        .recruit_items
        .find_single(token, true)
        .is_some())
    }
    fn attach_hero_57b830(&mut self, _: ObjectKey, _: u32) -> Result<bool> {
        required("4FA3D0 actual positive57B830 hero attachment owner")
    }
    fn notify_hero_attached(&mut self, _: ObjectKey, _: u32) -> Result<()> {
        required("4FA3D0 actual editor_hero_attached localization/name/40BC50 notification")
    }
}
impl RelationRead<'_> {
    fn special(
        &self,
        id: ActorId,
        class: u32,
    ) -> Result<&aa_mapworld::special_state::SpecialShell> {
        let k = self
            .actors
            .key(id)
            .ok_or("4FBF40 current relation owner generation absent")?;
        if self.actors.object(k).map_err(text)?.definition.native_class != class {
            return Err("4FBF40 current relation owner class differs".into());
        }
        let Source::Special(s) = &self.actors.entity(k).map_err(text)?.source else {
            return Err("4FBF40 current relation derived owner absent".into());
        };
        s.derived
            .as_ref()
            .ok_or("4FBF40 current relation fields absent".into())
    }
}
impl crate::relations::borrowed::Read for RelationRead<'_> {
    type Id = ActorId;
    type Error = String;
    fn kingdom_team(&self, id: ActorId) -> Result<Option<ActorId>> {
        self.special(id, 4)?
            .identity_references
            .get(&0x1ec)
            .copied()
            .ok_or("4FBF40 Kingdom1EC field absent".into())
    }
    fn kingdom_slot(&self, id: ActorId) -> Result<u32> {
        self.special(id, 4)?
            .words
            .get(&0x188)
            .copied()
            .ok_or("4FBF40 Kingdom188 field absent".into())
    }
    fn team_slot(&self, id: ActorId) -> Result<u32> {
        self.special(id, 5)?
            .words
            .get(&0x38)
            .copied()
            .ok_or("4FBF40 Team38 field absent".into())
    }
    fn kingdom_word(&self, id: ActorId, team: bool, slot: u32) -> Result<u32> {
        self.special(id, 4)?
            .vectors
            .get(&if team { 0x1dc } else { 0x1d0 })
            .and_then(|v| v.values.get(slot as usize))
            .copied()
            .ok_or("4FBF40 current Kingdom relation slot absent".into())
    }
    fn team_word(&self, id: ActorId, team: bool, slot: u32) -> Result<u32> {
        self.special(id, 5)?
            .vectors
            .get(&if team { 0x58 } else { 0x4c })
            .and_then(|v| v.values.get(slot as usize))
            .copied()
            .ok_or("4FBF40 current Team relation slot absent".into())
    }
    fn orphan_relation(&self) -> Result<u32> {
        required("4FBF40 current mutable B28158 relation owner")
    }
    fn team_null_relation(&self) -> Result<u32> {
        required("4FBF40 current mutable B2815C relation owner")
    }
}

///The real578040 guard sequence; positive stats/SAI callbacks remain required
///at their reached call, after successful same-owner member installation.
fn notify_member(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    member: ObjectKey,
    pc: Precision,
) -> Result<()> {
    fields::notify_fields(&mut Notify {
        staged,
        catalog,
        key,
        member,
        pc,
    })
}
struct Notify<'a, 'source> {
    staged: &'a mut Staged,
    catalog: &'a Catalog<'source>,
    key: ObjectKey,
    member: ObjectKey,
    pc: Precision,
}
impl NotifyFields for Notify<'_, '_> {
    type Owner = ActorId;
    fn allegiance(&mut self) -> Result<Option<ActorId>> {
        let word = allegiance(self.staged, self.key)?;
        Ok((word != 0).then_some(ActorId::from_raw(word)))
    }
    fn hq_present(&mut self) -> Result<bool> {
        //ActorD8 is table60[30], the installed HQ component. It has no
        //parallel pointer word in ActorShell.words.
        let owner = self
            .staged
            .actors
            .id(self.key)
            .ok_or("578040 actual owner GTID absent")?;
        match self
            .staged
            .actors
            .component(owner, Slot::Hq)
            .map_err(text)?
        {
            None => Ok(false),
            Some(RuntimeComponent::Hq(value)) if value.header.owner == owner => Ok(true),
            _ => Err("578040 actual Hq payload/owner differs".into()),
        }
    }
    fn member_is_primary(&mut self) -> Result<bool> {
        let primary = *org(&self.staged.actors, self.key)?
            .words
            .get(&0x44)
            .ok_or("578040 actual Organization44 unwritten")?;
        Ok(self
            .staged
            .actors
            .id(self.member)
            .ok_or("578040 member GTID absent")?
            .raw()
            == primary)
    }
    fn refresh_kingdom(&mut self, owner: ActorId) -> Result<()> {
        refresh_kingdom_actors(self.staged, self.catalog, owner, self.pc)
    }
    fn statistics_present(&mut self, owner: ActorId) -> Result<bool> {
        Ok(*RelationRead {
            actors: &self.staged.actors,
        }
        .special(owner, 4)?
        .words
        .get(&0x32c)
        .ok_or("578040 actual Kingdom32C stats owner unbound")?
            != 0)
    }
    fn state_1(&mut self) -> Result<bool> {
        Ok(self
            .staged
            .actors
            .object(self.key)
            .map_err(text)?
            .components()
            .map_err(text)?
            .state
            .low
            & 2
            != 0)
    }
    fn add_score(&mut self, _: ActorId, _: u32) -> Result<()> {
        required("578040 actual58E870 Kingdom statistics score before58EDD0")
    }
    fn classified_58edd0(&mut self) -> Result<bool> {
        required("578040 actual58EDD0 source classification")
    }
    fn sdk_present(&mut self) -> Result<bool> {
        self.staged
            .inactive_load
            .as_ref()
            .ok_or("578040 actual SDK lifecycle owner absent")?
            .observer_present()
    }
    fn sai_actor_present(&mut self) -> Result<bool> {
        let actor = self
            .staged
            .actors
            .object(self.key)
            .map_err(text)?
            .actor
            .as_ref()
            .ok_or("578040 actual Actor shell absent")?;
        actor
            .words
            .get(&0x154)
            .copied()
            .map(|v| v != 0)
            .ok_or("578040 actual typed Actor154 SAI actor owner".into())
    }
    fn notify_current_sai_actor(&mut self) -> Result<()> {
        if !self.sai_actor_present()? {
            return required("578040 current4D03C0 became null before SAIActor0C dereference");
        }
        required("578040 actual current SAIActor0C controller/7AD8E0")
    }
}
fn refresh_kingdom_actors(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    owner: ActorId,
    pc: Precision,
) -> Result<()> {
    fn count(staged: &Staged, owner: ActorId) -> Result<usize> {
        let read = RelationRead {
            actors: &staged.actors,
        };
        Ok(read
            .special(owner, 4)?
            .vectors
            .get(&0x2e8)
            .ok_or("4E14E0 actual Kingdom2E8 list absent")?
            .values
            .len())
    }
    fn actor(staged: &Staged, owner: ActorId, index: usize) -> Result<u32> {
        let read = RelationRead {
            actors: &staged.actors,
        };
        read.special(owner, 4)?
            .vectors
            .get(&0x2e8)
            .ok_or("4E14E0 actual Kingdom2E8 list absent")?
            .values
            .get(index)
            .copied()
            .ok_or("4E14E0 current list shrank during callback".into())
    }
    //4E14E5 captures the native2EC count once, but rereads actual2E8 at
    //each4E14F1 before dispatching that same current Actor's4D1350.
    let count = count(staged, owner)?;
    for index in 0..count {
        let id = ActorId::from_raw(actor(staged, owner, index)?);
        let key = staged
            .actors
            .key(id)
            .ok_or("4E14E0 current listed Actor generation absent")?;
        let world = staged
            .load_world
            .as_ref()
            .ok_or("4E14E0 actual World absent")?;
        crate::stored_actor_actions::rebuild_with_variables(
            catalog,
            &mut staged.actors,
            &mut staged.definitions,
            key,
            &staged.inventory,
            world.live_spatial()?,
            Some(&world.variables),
            pc,
        )?;
    }
    Ok(())
}
