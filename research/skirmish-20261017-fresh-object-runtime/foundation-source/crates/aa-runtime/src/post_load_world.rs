//! `aa_actors::post_load::PostLoadWorld` over the staged load: the GTID table
//! and objects of [`ActorStore`], the component Updatables of
//! [`Owners`]/[`Queues`] and the world [`NativeRng`] (the stream `50B2C0`
//! and `4E2640` draw from). Every method reads or writes one native field or
//! calls one native helper (research/phase8-saved-components
//! requests-to-lead.md R4-1); state the staged load does not model is an
//! explicit error, never a substituted value.
use crate::{
    actor_world::{ActorStore, Entity, ObjectKey, RuntimeComponent, RuntimeObject, Source, SpecialSource},
    localization::Localization,
    scheduler_owners::{self, Owners, Target},
    settings::Settings,
};
use aa_actors::{
    ActorId,
    actor_creation::ActorParams,
    components::ComponentSlot,
    owned::Component,
    post_load::{self, ActorList, Error, Id, PostLoadClass, RequiredObjectLog, Updatable},
};
use aa_mapworld::special_state::SpecialShell;
use aa_numeric::Precision;
use aa_rules::actors::ActorCatalog;
use aa_simulation::NativeRng;
use aa_world::queues::{Owner, Queues};
use serde::Serialize;
use std::convert::Infallible;

type Result<T> = std::result::Result<T, Error>;

/// A diagnostic written by a PostLoad, in pass order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PostLoadMessage {
    /// `404ED0` (debug log, GObject.cpp) or `5A1040` (map warning).
    pub function: u32,
    pub line: Option<u32>,
    pub text: String,
}

/// One recruit item (`Kingdom+214[index]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecruitItem {
    pub kingdom: Id,
    pub index: usize,
}

pub struct World<'a> {
    pub actors: &'a mut ActorStore,
    pub owners: &'a mut Owners,
    pub queues: &'a mut Queues,
    pub rng: &'a mut NativeRng,
    /// Portable definition tokens (recruit item ActorData keys).
    pub definitions: &'a crate::definitions::Bank,
    pub catalog: &'a ActorCatalog,
    /// Definition-time text table (ThingData `+10` names are stored by
    /// `47D960` when the definitions load, before MAPSTRINGS).
    pub definition_text: &'a Localization,
    pub settings: &'a Settings,
    /// GWorld `+2C` after the record sections (GActor::Load may raise it).
    pub world_version: u32,
    pub precision: Precision,
    pub messages: &'a mut Vec<PostLoadMessage>,
}

fn world(error: impl std::fmt::Display) -> Error {
    Error::World(error.to_string())
}

/// `+DC` class of a staged object (saved class code / special record kind).
pub fn object_class(entity: &Entity) -> Result<PostLoadClass> {
    match &entity.source {
        Source::Special(special) => Ok(match special.source {
            SpecialSource::Team(_) => PostLoadClass::Inert,
            SpecialSource::Kingdom(_) => PostLoadClass::Kingdom,
        }),
        Source::Loaded(loaded) => match loaded.placement.class_code {
            1 => Ok(PostLoadClass::Inert),
            2 | 6 | 7 | 9 => Ok(PostLoadClass::Object),
            3 => Ok(PostLoadClass::Actor),
            8 => Ok(PostLoadClass::Group),
            code => Err(Error::Undefined(format!(
                "saved class {code} has no identified +DC implementation"
            ))),
        },
        Source::New { .. } | Source::NewObject { .. } => Err(Error::World(
            "a fresh object exists before the PostLoad pass (record sections create none)".into(),
        )),
    }
}

impl World<'_> {
    fn key(&self, id: Id) -> Result<ObjectKey> {
        self.actors
            .key(ActorId::from_raw(id))
            .ok_or_else(|| Error::World(format!("{id:#010x} is not a live GTID")))
    }
    fn object(&self, id: Id) -> Result<&RuntimeObject> {
        self.actors.object(self.key(id)?).map_err(world)
    }
    fn object_mut(&mut self, id: Id) -> Result<&mut RuntimeObject> {
        let key = self.key(id)?;
        self.actors.object_mut(key).map_err(world)
    }
    fn actor_words(&self, id: Id) -> Result<&std::collections::BTreeMap<u32, u32>> {
        Ok(&self
            .object(id)?
            .actor
            .as_ref()
            .ok_or_else(|| Error::Undefined(format!("{id:#x} has no GActor fields")))?
            .words)
    }
    fn component(&self, id: Id, slot: ComponentSlot) -> Result<Option<&RuntimeComponent>> {
        Ok(self.object(id)?.components().map_err(world)?.get(slot))
    }
    fn defender(&self, id: Id) -> Result<&aa_actors::defender_state::DefenderState> {
        match self.component(id, ComponentSlot::Defender)? {
            Some(Component::Defender(state)) => Ok(state),
            _ => Err(Error::Undefined(format!("{id:#x} has no Defender component"))),
        }
    }
    fn defender_mut(&mut self, id: Id) -> Result<&mut aa_actors::defender_state::DefenderState> {
        match self
            .object_mut(id)?
            .components_mut()
            .map_err(world)?
            .get_mut(ComponentSlot::Defender)
        {
            Some(Component::Defender(state)) => Ok(state),
            _ => Err(Error::Undefined(format!("{id:#x} has no Defender component"))),
        }
    }
    fn kingdom(&self, id: Id) -> Result<&SpecialShell> {
        let entity = self.actors.entity(self.key(id)?).map_err(world)?;
        match &entity.source {
            Source::Special(s) if matches!(s.source, SpecialSource::Kingdom(_)) => s
                .derived
                .as_ref()
                .ok_or_else(|| Error::World(format!("Kingdom {id:#x} has no derived state"))),
            _ => Err(Error::Undefined(format!("{id:#x} is not a Kingdom"))),
        }
    }
    fn kingdom_mut(&mut self, id: Id) -> Result<&mut SpecialShell> {
        let key = self.key(id)?;
        let entity = self.actors.entity_mut(key).map_err(world)?;
        match &mut entity.source {
            Source::Special(s) if matches!(s.source, SpecialSource::Kingdom(_)) => s
                .derived
                .as_mut()
                .ok_or_else(|| Error::World(format!("Kingdom {id:#x} has no derived state"))),
            _ => Err(Error::Undefined(format!("{id:#x} is not a Kingdom"))),
        }
    }
    fn target(&self, at: Updatable) -> Result<Target> {
        let key = self.key(at.actor)?;
        if self.component(at.actor, at.slot)?.is_none() {
            return Err(Error::World(format!(
                "{:#x} has no {:?} component Updatable",
                at.actor, at.slot
            )));
        }
        Ok(Target::Component {
            object: key,
            slot: at.slot.index() as u8,
        })
    }
    /// The component's Updatable; None when it has no queue identity yet
    /// (then it has no scheduled entry).
    fn existing_owner(&self, at: Updatable) -> Result<Option<Owner>> {
        match self.owners.get(self.target(at)?) {
            Ok(owner) => Ok(Some(owner)),
            Err(scheduler_owners::Error::Missing) => Ok(None),
            Err(error) => Err(world(error)),
        }
    }
    /// The same component Updatable identity the saved table bound, or its
    /// first queue identity (the instance itself was constructed by Load).
    fn owner(&mut self, at: Updatable) -> Result<Owner> {
        let target = self.target(at)?;
        match self.owners.get(target) {
            Ok(owner) => Ok(owner),
            Err(scheduler_owners::Error::Missing) => self.owners.register(target).map_err(world),
            Err(error) => Err(world(error)),
        }
    }
    fn display_name(&self, definition: &str) -> Result<Vec<u16>> {
        let text = self.definition_text;
        self.catalog
            .native_display_name(definition, &mut |key| {
                text.lookup(key)
                    .map_err(|e| aa_rules::native::Error::Unsupported(e.to_string()))
            })
            .map_err(world)
    }
    fn single(&self, item: &RecruitItem) -> Result<&aa_economy::tick::RecruitSingle> {
        self.kingdom(item.kingdom)?
            .recruit_items
            .singles
            .get(item.index)
            .ok_or_else(|| Error::World(format!("recruit item {} is not live", item.index)))
    }
    fn single_mut(&mut self, item: &RecruitItem) -> Result<&mut aa_economy::tick::RecruitSingle> {
        self.kingdom_mut(item.kingdom)?
            .recruit_items
            .singles
            .get_mut(item.index)
            .ok_or_else(|| Error::World(format!("recruit item {} is not live", item.index)))
    }
}

impl post_load::PostLoadWorld for World<'_> {
    /// Denizen command objects (`51B580`): the Denizen decoder refuses every
    /// nonempty saved `+40` list, so no staged Denizen holds one.
    type Command = Infallible;
    type RecruitItem = RecruitItem;
    /// Canonical ThingData IDS.
    type Definition = String;
    type Layout = Infallible;

    fn highest_slot(&self) -> Result<i32> {
        Ok(self.actors.identities().highest_slot())
    }
    fn object_in_slot(&self, slot: u32) -> Result<Option<(Id, PostLoadClass)>> {
        let slot = u16::try_from(slot)
            .map_err(|_| Error::Undefined(format!("GTID slot {slot} beyond the table")))?;
        let identities = self.actors.identities();
        let id = ActorId::from_raw((u32::from(identities.generation_at(slot)) << 16) | u32::from(slot));
        let Some(entry) = identities.get(id) else {
            return Ok(None);
        };
        let entity = self.actors.entity(entry.value).map_err(world)?;
        Ok(Some((entry.id.raw(), object_class(entity)?)))
    }
    fn live(&self, id: Id) -> Result<bool> {
        Ok(self.actors.key(ActorId::from_raw(id)).is_some())
    }
    fn object_class(&self, id: Id) -> Result<PostLoadClass> {
        object_class(self.actors.entity(self.key(id)?).map_err(world)?)
    }
    fn object_flags(&self, id: Id) -> Result<u32> {
        Ok(self.actors.staged_placement(self.key(id)?).map_err(world)?.flags)
    }
    fn required_object(&self, object: Id) -> Result<Id> {
        Ok(self
            .object(object)?
            .object
            .as_ref()
            .ok_or_else(|| Error::Undefined(format!("{object:#x} has no GObject fields")))?
            .words_50_to_54[0])
    }
    fn definition_name(&self, object: Id) -> Result<Vec<u16>> {
        let id = self.object(object)?.definition.proof.id.clone();
        self.display_name(&id)
    }
    fn log_required_not_loaded(&mut self, entry: RequiredObjectLog) -> Result<()> {
        let text = post_load::REQUIRED_NOT_LOADED
            .replacen("%d", &(entry.required as i32).to_string(), 1)
            .replacen("%d", &(entry.object as i32).to_string(), 1)
            .replacen("%s", &String::from_utf16_lossy(&entry.name), 1);
        self.messages.push(PostLoadMessage {
            function: 0x404ed0,
            line: Some(post_load::POST_LOAD_LINE),
            text,
        });
        Ok(())
    }
    fn clear_required_object(&mut self, object: Id) -> Result<()> {
        self.object_mut(object)?
            .object
            .as_mut()
            .ok_or_else(|| Error::Undefined(format!("{object:#x} has no GObject fields")))?
            .words_50_to_54[0] = 0;
        Ok(())
    }
    fn definition_flags_1c(&self, _object: Id) -> Result<u32> {
        Err(Error::Unsupported("ThingData +1C flags have no proven staged binding"))
    }
    fn set_blocking(&mut self, _object: Id, _blocking: bool) -> Result<()> {
        Err(Error::Unsupported(
            "virtual +50 (GActor 4D4A10 / GObject 4EE840) needs the spatial runtime",
        ))
    }
    fn set_object_flags(&mut self, object: Id, flags: u32) -> Result<()> {
        let key = self.key(object)?;
        self.actors.staged_placement_mut(key).map_err(world)?.flags = flags;
        Ok(())
    }
    fn world_kingdoms(&self) -> Result<Vec<Option<Id>>> {
        Err(Error::Unsupported("GWorld +154 kingdom vector is not staged"))
    }
    fn kingdom_word_190(&self, kingdom: Id) -> Result<u32> {
        self.kingdom(kingdom)?
            .words
            .get(&0x190)
            .copied()
            .ok_or_else(|| Error::World("Kingdom +190 was not initialized".into()))
    }
    fn kingdom_index(&self, kingdom: Id) -> Result<u32> {
        self.kingdom(kingdom)?
            .words
            .get(&0x18c)
            .copied()
            .ok_or_else(|| Error::World("Kingdom +18C was not initialized".into()))
    }
    fn memory_count(&self, object: Id) -> Result<i32> {
        Ok(self
            .object(object)?
            .object
            .as_ref()
            .ok_or_else(|| Error::Undefined(format!("{object:#x} has no GObject fields")))?
            .memory.count())
    }
    fn memory_record(&self, _object: Id, _index: u32) -> Result<Option<u32>> {
        Err(Error::Unsupported("Object +5C detection memory records are not staged"))
    }
    fn ui_agent_active(&self) -> Result<bool> {
        Err(Error::Unsupported("UI agent [B36528] is not part of the staged load"))
    }
    fn ui_memory_record(&mut self, _record: Option<u32>, _kingdom: Id, _after: bool) -> Result<()> {
        Err(Error::Unsupported("UI agent 79DEB0/79E7B0 is not part of the staged load"))
    }
    fn refresh_memory_record(&mut self, _object: Id, _record: u32) -> Result<()> {
        Err(Error::Unsupported("4EC4A0 detection memory refresh is not staged"))
    }
    fn settlement_space(&self, _object: Id) -> Result<bool> {
        Err(Error::Unsupported("ThingData virtual +24 (462E50) has no staged binding"))
    }
    fn tile_size_bits(&self) -> Result<u32> {
        Err(Error::Unsupported("[GWorld+30]+8 terrain is not part of the staged load"))
    }
    fn thing_manager_74_bits(&self) -> Result<u32> {
        Err(Error::Unsupported("[B39F58]+74 is not part of the staged load"))
    }
    fn object_position(&self, object: Id) -> Result<[u32; 2]> {
        let p = self.actors.staged_placement(self.key(object)?).map_err(world)?;
        Ok([p.position_bits[0], p.position_bits[1]])
    }
    fn zone_count(&self, _actor: Id) -> Result<i32> {
        Err(Error::Unsupported("Actor +148 zone vector is not staged"))
    }
    fn move_zone(&mut self, _actor: Id, _index: u32, _xy: [u32; 2], _radius_bits: u32) -> Result<()> {
        Err(Error::Unsupported("515F40 zone move needs the zone runtime"))
    }
    fn precision(&self) -> Precision {
        self.precision
    }
    fn has_component(&self, actor: Id, slot: ComponentSlot) -> Result<bool> {
        Ok(self.component(actor, slot)?.is_some())
    }
    /// The staged Actor has no `+118/+120/+128` storage: Actor Load requires
    /// the saved lists empty (actor_finish) and no ported Load links into them.
    fn actor_list(&self, actor: Id, _list: ActorList) -> Result<Vec<Id>> {
        self.actor_words(actor)?;
        Ok(Vec::new())
    }
    fn remove_actor_list_node(&mut self, actor: Id, list: ActorList, _index: usize) -> Result<()> {
        Err(Error::World(format!(
            "{actor:#x} {list:?} has no staged node to unlink"
        )))
    }
    fn actor_kingdom(&self, actor: Id) -> Result<Option<Id>> {
        let word = *self
            .actor_words(actor)?
            .get(&0xe8)
            .ok_or_else(|| Error::World(format!("{actor:#x} +E8 was not initialized")))?;
        Ok((word != 0).then_some(word))
    }
    fn element_organization(&self, actor: Id) -> Result<Option<Id>> {
        match self.component(actor, ComponentSlot::Element)? {
            None => Ok(None),
            Some(Component::Element(state)) => Ok(state.organization.map(ActorId::raw)),
            Some(_) => Err(Error::World("Element slot holds another component".into())),
        }
    }
    fn has_scheduled(&mut self, at: Updatable, queue: u8, data: u32) -> Result<bool> {
        Ok(self
            .existing_owner(at)?
            .is_some_and(|owner| self.queues.has_from_owner_head(owner, queue, data)))
    }
    fn has_any_scheduled(&self, at: Updatable) -> Result<bool> {
        Ok(self
            .existing_owner(at)?
            .is_some_and(|owner| self.queues.has_owner(owner)))
    }
    fn add_periodic(&mut self, at: Updatable, queue: u8, data: u32) -> Result<()> {
        let owner = self.owner(at)?;
        self.queues
            .add_periodic(queue, owner, data, self.rng)
            .map_err(world)?;
        Ok(())
    }
    fn add_after(&mut self, at: Updatable, queue: u8, data: u32, delay_bits: u32) -> Result<()> {
        let owner = self.owner(at)?;
        self.queues
            .add_after(queue, owner, data, delay_bits)
            .map_err(world)?;
        Ok(())
    }
    fn remove_scheduled(&mut self, at: Updatable, queue: u8, data: u32) -> Result<u32> {
        match self.existing_owner(at)? {
            Some(owner) => self.queues.remove_matching(owner, queue, data).map_err(world),
            None => Ok(0),
        }
    }
    fn cancel_scheduled(&mut self, at: Updatable) -> Result<()> {
        match self.existing_owner(at)? {
            Some(owner) => self.queues.cancel_owner(owner).map_err(world),
            None => Ok(()),
        }
    }
    fn random(&mut self) -> Result<u32> {
        Ok(self.rng.next_u32())
    }
    fn unit_supply_enabled(&self) -> Result<bool> {
        Ok(self.settings.unit_supply_enabled().map_err(world)?.0)
    }
    fn body_shortfall_rate_bits(&self, actor: Id) -> Result<u32> {
        let id = self.object(actor)?.definition.proof.id.clone();
        let ground = self
            .catalog
            .native_ground_definition(&id, self.precision)
            .map_err(world)?;
        Ok(ground
            .body_shortfall_damage_rate
            .as_ref()
            .ok_or_else(|| Error::World(format!("{id} has no Body unit_shortfall_damage_rate")))?
            .bits)
    }
    fn word_16c(&self, actor: Id) -> Result<Id> {
        self.actor_words(actor)?
            .get(&0x16c)
            .copied()
            .ok_or_else(|| Error::World(format!("{actor:#x} +16C was not restored")))
    }
    fn world_version(&self) -> Result<u32> {
        Ok(self.world_version)
    }
    fn wall_flag_42c(&self, _actor: Id) -> Result<bool> {
        Err(Error::Unsupported("ThingData +42C (Wall) has no definition producer"))
    }
    fn denizen_commands(&self, actor: Id) -> Result<Vec<Option<Infallible>>> {
        match self.component(actor, ComponentSlot::Denizen)? {
            Some(Component::Denizen(_)) => Ok(Vec::new()),
            _ => Err(Error::Undefined(format!("{actor:#x} has no Denizen component"))),
        }
    }
    fn command_members(&self, command: &Infallible) -> Result<Vec<Id>> {
        match *command {}
    }
    fn set_mind_command(&mut self, _member: Id, command: &Infallible) -> Result<()> {
        match *command {}
    }
    fn defender_entries(&self, actor: Id) -> Result<Vec<[Id; 2]>> {
        Ok(self.defender(actor)?.entries.clone())
    }
    fn set_defender_entry_value(&mut self, actor: Id, index: usize, value: Id) -> Result<()> {
        let entry = self
            .defender_mut(actor)?
            .entries
            .get_mut(index)
            .ok_or_else(|| Error::World(format!("Defender entry {index} is not live")))?;
        entry[1] = value;
        Ok(())
    }
    fn remove_defender_entry(&mut self, actor: Id, index: usize) -> Result<()> {
        let entries = &mut self.defender_mut(actor)?.entries;
        if index >= entries.len() {
            return Err(Error::World(format!("Defender entry {index} is not live")));
        }
        entries.remove(index);
        Ok(())
    }
    fn defender_word_1c(&self, actor: Id) -> Result<u32> {
        Ok(self.defender(actor)?.word_1c)
    }
    fn defender_ability_count(&self, actor: Id) -> Result<usize> {
        Ok(self.defender(actor)?.abilities.len())
    }
    fn ability_target(&self, actor: Id, index: usize) -> Result<Id> {
        let handle = *self
            .defender(actor)?
            .abilities
            .get(index)
            .ok_or_else(|| Error::World(format!("Defender Ability {index} is not live")))?;
        Ok(self
            .actors
            .abilities
            .get(&handle)
            .ok_or_else(|| Error::World(format!("Ability handle {handle} is not in the arena")))?
            .word14)
    }
    fn set_ability_target(&mut self, actor: Id, index: usize, target: Id) -> Result<()> {
        let handle = *self
            .defender(actor)?
            .abilities
            .get(index)
            .ok_or_else(|| Error::World(format!("Defender Ability {index} is not live")))?;
        self.actors
            .abilities
            .get_mut(&handle)
            .ok_or_else(|| Error::World(format!("Ability handle {handle} is not in the arena")))?
            .word14 = target;
        Ok(())
    }
    fn hq_release_items(&mut self, actor: Id) -> Result<()> {
        match self.object_mut(actor)?.components_mut().map_err(world)?.get_mut(ComponentSlot::Hq) {
            Some(Component::Hq(state)) => { state.pending.clear(); Ok(()) },
            _ => Err(Error::Undefined(format!("{actor:#x} has no HQ component"))),
        }
    }
    fn hq_index(&self, actor: Id) -> Result<i32> {
        match self.component(actor, ComponentSlot::Hq)? {
            Some(Component::Hq(state)) => Ok(state.badge_index),
            _ => Err(Error::Undefined(format!("{actor:#x} has no HQ component"))),
        }
    }
    fn set_hq_index(&mut self, actor: Id, index: i32) -> Result<()> {
        match self.object_mut(actor)?.components_mut().map_err(world)?.get_mut(ComponentSlot::Hq) {
            Some(Component::Hq(state)) => { state.badge_index=index; Ok(()) },
            _ => Err(Error::Undefined(format!("{actor:#x} has no HQ component"))),
        }
    }
    fn kingdom_hq_slots(&self, kingdom: Id) -> Result<Option<i32>> {
        let shell = self.kingdom(kingdom)?;
        let nation = match shell.references.get(&0x234) {
            Some(nation) => nation.as_ref(),
            None if shell.words.get(&0x234) == Some(&0) => None,
            None => return Err(Error::World("Kingdom +234 was not initialized".into())),
        };
        let Some(nation) = nation else {
            return Ok(None);
        };
        let count = self
            .catalog
            .native_nation_badges(&nation.id, self.precision)
            .and_then(|badges| badges.count())
            .map_err(world)?;
        Ok(Some(count as i32))
    }
    fn kingdom_hq_taken(&self, kingdom: Id) -> Result<Vec<i32>> {
        Ok(self
            .kingdom(kingdom)?
            .nation_badge_indices
            .iter()
            .map(|v| *v as i32)
            .collect())
    }
    fn push_kingdom_hq_taken(&mut self, kingdom: Id, index: i32) -> Result<()> {
        let shell = self.kingdom_mut(kingdom)?;
        shell.nation_badge_indices.insert(0, index as u32);
        shell.words.remove(&0x36c);
        shell.words.remove(&0x370);
        Ok(())
    }
    fn kingdom_group_count(&self, kingdom: Id) -> Result<i32> {
        let groups = self
            .kingdom(kingdom)?
            .vectors
            .get(&0x34c)
            .ok_or_else(|| Error::World("Kingdom +34C was not constructed".into()))?;
        i32::try_from(groups.values.len()).map_err(world)
    }
    fn kingdom_group_members(&self, _kingdom: Id, _group: usize) -> Result<Vec<Id>> {
        Err(Error::Unsupported("Kingdom +34C group objects are not staged"))
    }
    fn remove_kingdom_group_member(&mut self, _kingdom: Id, _group: usize, _index: usize) -> Result<()> {
        Err(Error::Unsupported("Kingdom +34C group objects are not staged"))
    }
    fn remove_kingdom_group(&mut self, _kingdom: Id, _group: usize) -> Result<()> {
        Err(Error::Unsupported("Kingdom +34C group objects are not staged"))
    }
    fn recruit_item_count(&self, kingdom: Id) -> Result<i32> {
        i32::try_from(self.kingdom(kingdom)?.recruit_items.singles.len()).map_err(world)
    }
    fn recruit_item_at(&self, kingdom: Id, index: usize) -> Result<Option<RecruitItem>> {
        let count = self.kingdom(kingdom)?.recruit_items.singles.len();
        Ok((index < count).then_some(RecruitItem { kingdom, index }))
    }
    fn recruit_item_byte_8(&self, item: &RecruitItem) -> Result<u8> {
        Ok(u8::from(self.single(item)?.available))
    }
    fn recruit_item_organization(&self, item: &RecruitItem) -> Result<Id> {
        Ok(self.single(item)?.organization)
    }
    fn set_recruit_item_organization(&mut self, item: &RecruitItem, organization: Id) -> Result<()> {
        self.single_mut(item)?.organization = organization;
        Ok(())
    }
    fn recruit_item_name(&self, item: &RecruitItem) -> Result<Vec<u16>> {
        let token = self.single(item)?.definition;
        let entry = self.definitions.entry(token).map_err(world)?;
        if entry.key.domain != "actordefinition" {
            return Err(Error::World("recruit item key is not an ActorDefinition".into()));
        }
        let name = entry.key.name.clone();
        self.display_name(&name)
    }
    fn set_recruit_item_available(&mut self, item: &RecruitItem) -> Result<()> {
        self.single_mut(item)?.available = true;
        Ok(())
    }
    fn increment_kingdom_224(&mut self, kingdom: Id) -> Result<()> {
        let table = &mut self.kingdom_mut(kingdom)?.recruit_items;
        table.changes = table.changes.wrapping_add(1);
        Ok(())
    }
    fn map_warning(&mut self, format: &'static str, name: Vec<u16>) -> Result<()> {
        self.messages.push(PostLoadMessage {
            function: 0x5a1040,
            line: None,
            text: format.replacen("%s", &String::from_utf16_lossy(&name), 1),
        });
        Ok(())
    }
    // Settlement (slot 26 has no staged component variant, so has_component
    // never reaches 586BC0) and GGroup (class 8 records are not restored).
    fn settlement_count(&self, _actor: Id) -> Result<i32> {
        Err(Error::Unsupported("Settlement component is not staged"))
    }
    fn settlement_entry(&self, _actor: Id, _index: usize) -> Result<post_load::SettlementEntry> {
        Err(Error::Unsupported("ThingData +47C settlement entries are not staged"))
    }
    fn settlement_definition(&self, _actor: Id, _kind: u32) -> Result<Option<String>> {
        Err(Error::Unsupported("45D7C0 settlement definitions are not staged"))
    }
    fn object_z(&self, object: Id) -> Result<u32> {
        Ok(self
            .actors
            .staged_placement(self.key(object)?)
            .map_err(world)?
            .position_bits[2])
    }
    fn create_gthing(&mut self, _params: ActorParams<String, Infallible>) -> Result<Option<Id>> {
        Err(Error::Unsupported("4F9C10 creation during PostLoad is not bound"))
    }
    fn set_settlement_thing(&mut self, _actor: Id, _index: usize, _id: Id) -> Result<()> {
        Err(Error::Unsupported("Settlement component is not staged"))
    }
    fn group_target(&self, _group: Id) -> Result<Option<post_load::GroupTarget>> {
        Err(Error::Unsupported("GGroup +1A4 is not staged"))
    }
    fn set_group_target(&mut self, _group: Id, _target: post_load::GroupTarget) -> Result<()> {
        Err(Error::Unsupported("GGroup +1A4 is not staged"))
    }
    fn group_radius_bits(&self, _group: Id) -> Result<u32> {
        Err(Error::Unsupported("GGroup +1B4 is not staged"))
    }
    fn clamp_position(&mut self, _xy: [u32; 2]) -> Result<[u32; 2]> {
        Err(Error::Unsupported("552640 terrain clamp is not part of the staged load"))
    }
}
