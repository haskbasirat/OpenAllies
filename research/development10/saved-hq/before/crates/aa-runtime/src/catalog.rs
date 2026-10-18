//! Actual source definitions connected to saved-object restoration. Completed
//! Ability counts are supplied by the native Ability loader, never mesh counts.
use aa_mapworld::{
    bootstrap,
    restore::{BodyDefinition, GroundDefinitions, RestoreDefinitions, RuntimeDefinition},
};
use aa_numeric::Precision;
use aa_rules::actors::ActorCatalog;
use std::{cell::RefCell, collections::BTreeMap, sync::Arc};

#[cfg(test)]
#[path = "catalog_binding_tests.rs"]
mod binding_tests;

#[derive(Clone, Debug)]
pub struct ActorBindings {
    pub definition: RuntimeDefinition,
    pub source_ground: aa_rules::ground::GroundDefinition,
    pub source_economy: Option<aa_rules::economy_values::EconomyInputs>,
    pub character: Option<aa_animation::definitions::CharacterDefinitions>,
    pub performer: Option<aa_rules::abilities::LoadedPerformer>,
    /// x87 precision control these bindings were completed under.
    pub precision: Precision,
}
impl ActorBindings {
    /// Caller supplies the actual native Character view set after Object
    /// variant/Actor state-aspect selection. It is never inferred from a mesh.
    pub fn ground_for_animation_set(&self, set: u32) -> Result<GroundDefinitions, String> {
        let mut ground = self.definition.ground.clone();
        let character = self
            .character
            .as_ref()
            .ok_or("actor has no Character definition")?;
        let effective_set = crate::actor_world::effective_animation_set(character, set)
            .map_err(|e| e.to_string())?;
        ground.character_animations = Some(
            character
                .for_variant(set)
                .map_err(|e| e.to_string())?
                .iter()
                .enumerate()
                .map(|(row, _)| {
                    Some(crate::actor_world::animation_key(
                        &self.definition.proof.id,
                        effective_set,
                        row,
                    ))
                })
                .collect(),
        );
        Ok(ground)
    }
}

pub struct Catalog<'a> {
    pub actors: &'a ActorCatalog,
    /// Immutable owner of newly cached source arenas. Public legacy callers
    /// may replace `actors`; never return an arena made from another owner.
    source_owner: &'a ActorCatalog,
    pub localization: Option<crate::localization::Localization>,
    /// The game text table as it was when the definitions loaded (ThingData
    /// 47D960 runs before any map's MAPSTRINGS are registered).
    pub definition_text: Option<crate::localization::Localization>,
    pub settings: Option<crate::settings::Settings>,
    /// Actual MapResource+A8 passed to GWorld::Load; absent means the caller
    /// has not supplied the mode. This is unrelated to diagnostic RestoreMode.
    pub native_load_mode: Option<u32>,
    /// A borrowed capability owned by the explicit fresh managed load scope.
    /// Standalone record diagnostics leave the observer lifecycle unknown.
    inactive_load: Option<&'a crate::restore_lifecycle::InactiveLoad>,
    pub definition_bank: RefCell<crate::definitions::Bank>,
    pub ability_arena: RefCell<crate::saved_performer::AbilityArena>,
    pub updatables:
        RefCell<Option<crate::updatables::Table<aa_mapworld::restore_components::UpdatableTarget>>>,
    actor_bindings: RefCell<BTreeMap<(String, u64), Arc<ActorBindings>>>,
    component_masks: RefCell<BTreeMap<(String, u64), Result<u32, String>>>,
    /// Native 4C3610/4C34A0 data and pointer replacements shared by all
    /// Organization loads in this catalog. Keep explicit x87 precision in
    /// the key, just as for the completed ActorData bindings.
    layout_data: RefCell<
        BTreeMap<u64, Result<Arc<aa_rules::native::layout_data::NativeLayoutData>, String>>,
    >,
    mobile_deployment_data: RefCell<
        BTreeMap<u64, Result<Arc<aa_rules::native_mobile_deployment::NativeMobileDeploymentData>, String>>,
    >,
    recruit_backrefs: RefCell<BTreeMap<u64, Result<Arc<aa_rules::native::recruit_backrefs::NativeRecruitBackrefs>, String>>>,
    /// Completed Builder source nodes remain one canonical allocation arena
    /// for this Catalog, distinct from each Actor's mutable +108 list.
    builder_data: RefCell<Option<Result<Arc<aa_rules::native_builder_data::NativeBuilderData>, String>>>,
    resource_source: Option<Arc<aa_rules::resources::ResourceCatalog>>,
    resource_economy: RefCell<BTreeMap<u64, Result<Arc<crate::kingdom_economy::Definitions>, String>>>,
    /// Next restore Mind/state identities (ActorStore::new starts both at 1;
    /// staging continues after the highest restored identity).
    mind_identities: std::cell::Cell<(u32, u32)>,
    /// Saved commands have distinct allocations even when their bytes match.
    command_identity: std::cell::Cell<u32>,
}
/// A restored concrete Mind: the installed remainder plus the Mind core and
/// embedded states it owns until staging moves them into the live arena.
#[derive(Clone, Debug)]
pub struct RestoredMindParts {
    pub installed: aa_mind::construct::Installed,
    /// Temporarily None only while the same core is moved into a Load/action
    /// arena; every returned success or error restores this owner.
    pub mind: Option<aa_mind::Mind>,
    pub states: BTreeMap<aa_mind::StateId, aa_mind::State>,
    /// Loaded Element +D4 path nodes (head first) for the typed Move owner.
    pub element_path: Option<Vec<aa_navigation::PathNode>>,
    /// Loaded Regiment v2+ containers and references, moved once at staging.
    pub regiment_owned: Option<aa_mind::regiment_load::Owned>,
    pub precision: Precision,
    /// One owned reference per saved pending/current command until staging.
    pub commands: Vec<(u32, crate::mind_world::PlainCommand)>,
}
impl aa_mapworld::restore_components::RestoredMindState for RestoredMindParts {
    fn clone_box(&self) -> Box<dyn aa_mapworld::restore_components::RestoredMindState> {
        Box::new(self.clone())
    }
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
/// Original 4CF480 case 4 precedence (ActorData+174 bit 7, then 23, then 8).
pub fn mind_kind(definition_mask: u32) -> Option<aa_mind::construct::Kind> {
    use aa_actors::init_create::{FactoryKind, factory_kind};
    match factory_kind(aa_actors::components::ComponentSlot::Mind, definition_mask)? {
        FactoryKind::OrganizationMind => Some(aa_mind::construct::Kind::Regiment),
        FactoryKind::AircraftMind => Some(aa_mind::construct::Kind::Aircraft),
        FactoryKind::ElementMind => Some(aa_mind::construct::Kind::Element),
        FactoryKind::Standard(_) => None,
    }
}
/// Concrete Mind virtual +20 (Element 5B6830, Regiment 7C53E0, Aircraft 55EA50).
pub fn mind_requirement(
    kind: aa_mind::construct::Kind,
) -> Result<aa_actors::components::ComponentSlot, String> {
    use aa_actors::components::ComponentSlot;
    match kind {
        aa_mind::construct::Kind::Element => Ok(ComponentSlot::Element),
        aa_mind::construct::Kind::Regiment => Ok(ComponentSlot::Mind),
        aa_mind::construct::Kind::Aircraft => ComponentSlot::from_index(0x17)
            .ok_or_else(|| "component slot 0x17 is outside the table".into()),
        aa_mind::construct::Kind::Base => Err("the factory constructs no plain base Mind".into()),
    }
}
struct RestoreMindWorld<'a> {
    rng: &'a mut aa_simulation::NativeRng,
    abilities: &'a [u32],
    arena: &'a mut crate::saved_performer::AbilityArena,
}
impl aa_mind::construct::World for RestoreMindWorld<'_> {
    fn random_word(&mut self) -> aa_mind::Result<u32> {
        Ok(self.rng.next_u32())
    }
    fn performer_ability_count(&self, _: u32) -> aa_mind::Result<usize> {
        Ok(self.abilities.len())
    }
    fn set_ability_jitter(&mut self, _: u32, index: usize, bits: u32) -> aa_mind::Result<()> {
        let handle = *self
            .abilities
            .get(index)
            .ok_or_else(|| aa_mind::Error::World("Performer action index changed".into()))?;
        self.arena
            .get_mut(handle)
            .map_err(aa_mind::Error::World)?
            .fuzzy_range = aa_body::F32(bits);
        Ok(())
    }
}
impl<'a> Catalog<'a> {
    pub fn new(actors: &'a ActorCatalog) -> Self {
        Self {
            actors,
            source_owner: actors,
            localization: None,
            definition_text: None,
            settings: None,
            native_load_mode: None,
            inactive_load: None,
            definition_bank: RefCell::new(crate::definitions::Bank::default()),
            ability_arena: RefCell::new(crate::saved_performer::AbilityArena::new()),
            updatables: RefCell::new(None),
            actor_bindings: RefCell::new(BTreeMap::new()),
            component_masks: RefCell::new(BTreeMap::new()),
            layout_data: RefCell::new(BTreeMap::new()),
            mobile_deployment_data: RefCell::new(BTreeMap::new()),
            recruit_backrefs: RefCell::new(BTreeMap::new()),
            builder_data: RefCell::new(None),
            resource_source: None,
            resource_economy: RefCell::new(BTreeMap::new()),
            mind_identities: std::cell::Cell::new((1, 1)),
            command_identity: std::cell::Cell::new(0),
        }
    }
    /// 463FA0's OrganizationLayout manager and its Formation/Property
    /// identities, produced from the native-order source catalog. Cache
    /// failures too: a failed producer cannot become a fresh definition
    /// arena partway through restoring an organization.
    pub fn layout_data(
        &self,
        precision: Precision,
    ) -> Result<Arc<aa_rules::native::layout_data::NativeLayoutData>, String> {
        self.require_source_owner()?;
        let key = precision.significand_bits();
        if let Some(data) = self.layout_data.borrow().get(&key) {
            return data.clone();
        }
        let result = self
            .actors
            .native_layout_data(precision)
            .map(Arc::new)
            .map_err(|error| error.to_string());
        self.layout_data.borrow_mut().insert(key, result.clone());
        result
    }
    /// Original deferred MobileDeployment references are one completed source
    /// arena. Cache success/failure per x87 precision, never per actor callback.
    pub fn native_mobile_deployment_data(
        &self,
        precision: Precision,
    ) -> Result<Arc<aa_rules::native_mobile_deployment::NativeMobileDeploymentData>, String> {
        let key = precision.significand_bits();
        if let Some(value) = self.mobile_deployment_data.borrow().get(&key) {
            return value.clone();
        }
        let value = self.actors.native_mobile_deployment_data(precision)
            .map(Arc::new).map_err(|e| e.to_string());
        self.mobile_deployment_data.borrow_mut().insert(key, value.clone());
        value
    }
    /// Cache the completed460310/460000 reverse-reference producer, retaining
    /// actual canonical identity and x87 source-load precision.
    pub fn native_recruit_backrefs(
        &self,
        precision: Precision,
    ) -> Result<Arc<aa_rules::native::recruit_backrefs::NativeRecruitBackrefs>, String> {
        self.require_source_owner()?;
        let key = precision.significand_bits();
        if let Some(value) = self.recruit_backrefs.borrow().get(&key) {
            return value.clone();
        }
        let value = self.actors.native_recruit_backrefs(precision)
            .map(Arc::new).map_err(|error| error.to_string());
        self.recruit_backrefs.borrow_mut().insert(key, value.clone());
        value
    }
    /// Integer/reference-only Builder4604C0 projection. The shared source arena
    /// has no precision-dependent arithmetic; `precision` follows the restore
    /// caller API but cannot manufacture a completed global source lifecycle.
    /// This proves only the Builder callback subsequence, not all deferred data.
    pub fn native_builder_data(
        &self,
        _precision: Precision,
    ) -> Result<Arc<aa_rules::native_builder_data::NativeBuilderData>, String> {
        self.require_source_owner()?;
        if let Some(value) = self.builder_data.borrow().as_ref() {
            return value.clone();
        }
        let value = self.actors.native_builder_data()
            .map(Arc::new).map_err(|error| error.to_string());
        *self.builder_data.borrow_mut() = Some(value.clone());
        value
    }
    /// Bind immutable definitions already loaded by the proven Resource source
    /// provider. This never initializes or replaces any Kingdom's live arrays.
    pub fn bind_resource_catalog(
        &mut self,
        source: Arc<aa_rules::resources::ResourceCatalog>,
    ) -> Result<(), String> {
        self.require_source_owner()?;
        if self.resource_source.is_some() {
            return Err("Resource source owner is already bound".into());
        }
        self.resource_source = Some(source);
        self.resource_economy.get_mut().clear();
        Ok(())
    }
    pub fn native_resource_economy(
        &self,
        precision: Precision,
    ) -> Result<Arc<crate::kingdom_economy::Definitions>, String> {
        self.require_source_owner()?;
        let key = precision.significand_bits();
        if let Some(value) = self.resource_economy.borrow().get(&key) {
            return value.clone();
        }
        let value = self.resource_source.as_ref()
            .ok_or_else(|| "saved Kingdom Resource source is not bound".to_string())
            .and_then(|source| crate::kingdom_economy::Definitions::bind(source, self.actors, precision)
                .map(Arc::new).map_err(|error| error.to_string()));
        self.resource_economy.borrow_mut().insert(key, value.clone());
        value
    }
    /// Borrow the actual non-Clone managed load owner; this does not initialize
    /// record Init/PostCreate/publication, or activate SAI.
    pub fn bind_inactive_load(
        &mut self,
        load: &'a crate::restore_lifecycle::InactiveLoad,
    ) -> Result<(), String> {
        self.require_source_owner()?;
        load.observer_present()?;
        if self.inactive_load.is_some() {
            return Err("managed observer load owner is already bound".into());
        }
        let mode = load.native_load_mode();
        if self.native_load_mode.is_some_and(|actual| actual != mode) {
            return Err("managed task and MapResource native modes disagree".into());
        }
        self.native_load_mode = Some(mode);
        self.inactive_load = Some(load);
        Ok(())
    }
    pub(crate) fn saved_observer_present(&self) -> Result<bool, String> {
        let load = self.inactive_load
            .ok_or("5705A0 SAI observer lifecycle is not bound to this restore context")?;
        self.require_managed_mode(load)?;
        load.observer_present()
    }
    pub(crate) fn notify_saved_observer(&self, actor: u32) -> Result<(), String> {
        let load = self.inactive_load
            .ok_or("7A0C20 SAI observer notification has no managed lifecycle owner")?;
        self.require_managed_mode(load)?;
        load.notify_observer(actor)
    }
    pub(crate) fn require_source_owner(&self) -> Result<(), String> {
        if !std::ptr::eq(self.actors, self.source_owner) {
            return Err("Catalog Actor source owner changed after construction".into());
        }
        Ok(())
    }
    fn require_managed_mode(&self, load: &crate::restore_lifecycle::InactiveLoad) -> Result<(), String> {
        self.require_source_owner()?;
        if self.native_load_mode != Some(load.native_load_mode()) {
            return Err("managed task and MapResource native modes disagree".into());
        }
        Ok(())
    }
    /// Existing source-field helpers accept lookup IDS, not native object
    /// pointers. A canonical ActorData identity shadowed by another Thing
    /// translation must not silently resolve a second time to another actor.
    pub(crate) fn require_untranslated_actor_identity(&self, id: &str) -> Result<(), String> {
        self.require_source_owner()?;
        match self.actors.find(id) {
            Some(found) if found.id.eq_ignore_ascii_case(id) => Ok(()),
            Some(found) => Err(format!(
                "canonical ActorData {id} is shadowed by Thing translation to {}; registered-identity source resolution is unavailable",
                found.id
            )),
            None => Err(format!("canonical ActorData {id} is not registered")),
        }
    }
    /// Completed ActorData+174 without forcing unrelated ground/resource
    /// projections. The Performer count is the same native loader result used
    /// by actor_bindings; authored component presence alone is insufficient.
    pub fn canonical_component_mask(&self, id: &str, pc: Precision) -> Result<u32, String> {
        self.require_untranslated_actor_identity(id)?;
        let key = (id.to_ascii_lowercase(), pc.significand_bits());
        if let Some(value) = self.component_masks.borrow().get(&key) {
            return value.clone();
        }
        let value = (|| {
            // No completed count is needed for absent Performer or the
            // native Defender/Aircraft exceptions. Do not resolve animations
            // or Ability data on those paths merely to read the mask.
            if let Ok(mask) = self.actors.native_component_mask_at(id, None, pc) {
                return Ok(mask.bits);
            }
            let character = aa_animation::definitions::character(self.actors, id, pc)
                .map_err(|error| error.to_string())?;
            let performer = self.actors.native_loaded_performer(id, pc,
                character.as_ref().map(|value| value as &dyn aa_rules::abilities::AnimationResolver))
                .map_err(|error| error.to_string())?;
            let count = performer.as_ref().map_or(0, |value| value.completed_ability_count());
            self.actors.native_component_mask_at(id, Some(count), pc)
                .map(|value| value.bits).map_err(|error| error.to_string())
        })();
        self.component_masks.borrow_mut().insert(key, value.clone());
        value
    }
    /// Read native ActorData+1E4/+30C/+36C from the one completed source Bank.
    /// A constructor-only identity or an explicitly supplied diagnostic FP
    /// context cannot authorize this read. Canonical source ownership is checked
    /// again here because the public Catalog source reference may be reassigned
    /// by diagnostic callers after construction.
    pub fn completed_actor_combat_values(
        &self,
        id: &str,
    ) -> Result<crate::combat_source_completion::Values, String> {
        self.require_untranslated_actor_identity(id)?;
        let canonical = self.actors.actors.iter()
            .find(|actor| actor.id.eq_ignore_ascii_case(id))
            .ok_or_else(|| format!("completed combat source has no canonical ActorData {id}"))?;
        let bank = self.definition_bank.try_borrow()
            .map_err(|_| "nested completed combat source Bank ownership")?;
        let token = bank.token("ActorDefinition", id)
            .ok_or_else(|| format!("completed combat source has no existing identity for {id}"))?;
        let state = bank.combat_source()?;
        if state.source_identities() != self.actors.sources
            || !state.definition_id(token)?.eq_ignore_ascii_case(&canonical.id)
            || state.definition_source(token)? != &canonical.source
        {
            return Err(format!("completed combat source ownership differs for {id}"));
        }
        state.completed_values(token)
    }
    pub fn object_creation(
        &self,
        id: &str,
        precision: Precision,
    ) -> Result<aa_actors::creation::ObjectDefinition, String> {
        let value = self
            .actors
            .native_creation_definition(id, precision)
            .map_err(|e| e.to_string())?;
        Ok(aa_actors::creation::ObjectDefinition {
            variations_enabled: value.variations_enabled,
            variation_weights_bits: value.variation_weights_bits,
            total_variation_weight_bits: value.total_variation_weight_bits,
            scale_min_bits: value.scale_min.bits,
            scale_max_bits: value.scale_max.bits,
        })
    }
    pub fn commander_constructor(
        &self,
        id: &str,
        pc: Precision,
    ) -> Result<aa_combat::commander::Definition, String> {
        let input = self
            .actors
            .native_commander_constructor_inputs(id, pc)
            .map_err(|e| e.to_string())?;
        let mut bank = self
            .definition_bank
            .try_borrow_mut()
            .map_err(|_| "nested definition registration")?;
        let faction = bank.intern("Faction", &input.faction_id)?;
        let mut abilities = Vec::new();
        for ability in &input.abilities {
            let local_name = format!("{}/ability/{}", input.faction_id, ability.ordinal);
            let local = bank.intern("FactionLocalAbility", &local_name)?;
            let definition = if ability.shared_definition_id.is_some() {
                bank.intern("AbilityDefinition", &ability.definition.id)?
            } else {
                bank.intern("FactionLocalAbilityDefinition", &local_name)?
            };
            let reason = "Commander constructor/save fields are resolved; special-operation definition execution and world effects remain incomplete";
            bank.constructor_only(definition, reason, Some(ability.faction_range_bits))?;
            bank.constructor_only(local, reason, None)?;
            abilities.push(aa_combat::constructors::LocalAbility {
                definition,
                local_definition: local,
                main_power: aa_body::F32(ability.main_power_bits),
            });
        }
        Ok(aa_combat::commander::Definition {
            id: faction,
            abilities,
        })
    }
    /// A completed gameplay definition keeps its actual ability and animation
    /// data. The finalized component count can no longer be injected separately.
    pub fn actor_bindings(&self, id: &str, pc: Precision) -> Result<Arc<ActorBindings>, String> {
        self.require_source_owner()?;
        let id = self
            .actors
            .find(id)
            .ok_or_else(|| format!("missing Actor definition {id}"))?
            .id
            .as_str();
        let key = (id.to_ascii_lowercase(), pc.significand_bits());
        if let Some(value) = self.actor_bindings.borrow().get(&key) {
            return Ok(Arc::clone(value));
        }
        let base =
            bootstrap::resolve_definition(self.actors, &id.encode_utf16().collect::<Vec<_>>(), pc)
                .map_err(|e| e.to_string())?;
        if base.definition.category.native_kind != 2 {
            return Err("Actor binding requires ActorData".into());
        }
        let character =
            aa_animation::definitions::character(self.actors, id, pc).map_err(|e| e.to_string())?;
        let performer = self
            .actors
            .native_loaded_performer(
                id,
                pc,
                character
                    .as_ref()
                    .map(|v| v as &dyn aa_rules::abilities::AnimationResolver),
            )
            .map_err(|e| e.to_string())?;
        let completed_count = performer
            .as_ref()
            .map_or(0, |p| p.completed_ability_count());
        let mask = self
            .actors
            .native_component_mask_at(id, Some(completed_count), pc)
            .map_err(|e| e.to_string())?
            .bits;
        self.component_masks.borrow_mut().insert(key.clone(), Ok(mask));
        let source_ground = self
            .actors
            .native_ground_definition(id, pc)
            .map_err(|e| e.to_string())?;
        let attributes = &source_ground.attributes;
        let ground = GroundDefinitions {
            body: attributes.body.as_ref().map(|b| BodyDefinition {
                maximum_bits: b.health.bits,
                defense_bits: b.defense.bits,
            }),
            mover: attributes
                .mover
                .as_ref()
                .map(|m| {
                    Ok::<_, String>(aa_actors::ground_states::MoverDefinition {
                        speed: aa_body::F32(m.speed_max.bits),
                        acceleration: aa_body::F32(m.acceleration.bits),
                        turn_radius: aa_body::F32(m.turn_radius.bits),
                        turn_type: m.native_turn_type().map_err(|e| e.to_string())? as i32,
                    })
                })
                .transpose()?,
            properties_domain_count: if mask & (1 << 6) != 0 {
                Some(
                    self.actors
                        .native_ability_type_domain(pc)
                        .and_then(|d| d.count())
                        .map_err(|e| e.to_string())?,
                )
            } else {
                None
            },
            resource_domain_count: if mask & (1 << 9) != 0 {
                Some(self.resource_domain_count(pc)? as usize)
            } else {
                None
            },
            detection_radius_bits: source_ground
                .detection
                .as_ref()
                .map(|d| d.finalized_range_bits),
            organization: source_ground
                .organization
                .as_ref()
                .map(|_| {
                    self.require_untranslated_actor_identity(id)?;
                    let data = self.layout_data(pc)?;
                    // 466C4F/460000 bind these pointers while loading the
                    // ActorData. Keep their canonical identities, not the
                    // authored aliases in the older ground projection.
                    let component = data
                        .organization_component(self.actors, id, pc)
                        .map_err(|error| error.to_string())?
                        .ok_or("Organization source component disappeared")?;
                    for parameter in component.parameters.iter().flatten() {
                        self.require_untranslated_actor_identity(parameter)?;
                    }
                    let layout = data
                        .layouts
                        .get(component.layout as usize)
                        .ok_or("Organization default layout identity is unavailable")?;
                    Ok::<_, String>(aa_actors::owned::OrganizationDefinition {
                        layout: aa_actors::ground_states::LayoutDefinition {
                            definition: component.layout_ids,
                            parameters: component.parameters,
                            slots: layout.slot_groups.len(),
                        },
                        morale_breakpoint_bits: component.morale_breakpoint,
                    })
                })
                .transpose()?,
            ..Default::default()
        };
        let source_economy = if mask & (1 << 9) != 0 {
            Some(
                self.actors
                    .native_economy_inputs(id, pc)
                    .map_err(|e| e.to_string())?,
            )
        } else {
            None
        };
        let mut ground = ground;
        if mask & (1 << 10) != 0 {
            // 56B720 slot list from ActorData+35C/+34C (definitions producer).
            let inventory = self
                .actors
                .native_denizen_inventory(id, pc)
                .map_err(|e| e.to_string())?
                .ok_or("Denizen mask lacks DenizenComponent")?;
            let mut health = BTreeMap::new();
            for slot in &inventory.slots {
                if health.contains_key(&slot.target) {
                    continue;
                }
                let bits = self
                    .actors
                    .native_ground_definition(&slot.target, pc)
                    .map_err(|e| e.to_string())?
                    .attributes
                    .body
                    .ok_or_else(|| format!("Denizen Inventory target {} has no Body", slot.target))?
                    .health
                    .bits;
                health.insert(slot.target.clone(), bits);
            }
            ground.denizen = Some(
                inventory
                    .slots
                    .iter()
                    .map(|slot| aa_actors::denizen_state::InventorySlotDefinition {
                        target: slot.target.clone(),
                        worker: slot.worker,
                        militia_or_air_defensive: slot.militia_or_air_defensive,
                    })
                    .collect(),
            );
            ground.denizen_target_health_bits = Some(health);
        }
        if mask & (1 << 11) != 0 {
            let source = self
                .actors
                .native_supply_fields(id, pc)
                .map_err(|e| e.to_string())?
                .ok_or("Supply component mask lacks SupplyComponent")?;
            ground.supply = Some(aa_actors::supply_state::Definition {
                supply_range_bits: source.supply_range_bits,
                is_relay: source.is_relay,
                // Unread for non-relays; 58ABD0 multiplies it only for relays.
                stockpile_max_bits: source.stockpile_max_bits.unwrap_or(0),
            });
        }
        if mask & (1 << 29) != 0 {
            ground.city_zone_radius_bits = Some(
                self.actors
                    .native_city_zone_radius(id, pc)
                    .map_err(|e| e.to_string())?
                    .ok_or("City component mask lacks CityComponent")?,
            );
        }
        if mask & (1 << 21) != 0 {
            ground.research_items = Some(
                self.actors
                    .native_research_property_ids(id, pc)
                    .map_err(|e| e.to_string())?
                    .ok_or("Research component mask lacks ResearchComponent")?,
            );
        }
        if mask & (1 << 15) != 0 {
            let source = self
                .actors
                .native_construction_inputs(id, pc)
                .map_err(|e| e.to_string())?
                .ok_or("Construction component mask lacks its source input")?;
            ground.construction = Some(aa_actors::construction_state::Definition {
                base_cost_words: source.resources.map(|r| r.cost_bits),
                empty_predecessor_chain: source.upgrade_census.explicit_instances.is_empty(),
            });
        }
        let value = Arc::new(ActorBindings {
            definition: RuntimeDefinition {
                proof: base.definition,
                native_class: base.native_class,
                flags: base.flag_word,
                object: Some(self.object_creation(id, pc)?),
                component_mask: Some(mask),
                ground,
            },
            source_ground,
            source_economy,
            character,
            performer,
            precision: pc,
        });
        self.actor_bindings
            .borrow_mut()
            .insert(key, Arc::clone(&value));
        Ok(value)
    }
}
impl Catalog<'_> {
    fn load_saved_mind_bound(
        &self,
        mind: &mut aa_mapworld::restore_components::RestoredMind,
        bytes: &[u8],
        context: Option<aa_mapworld::restore_components::MindLoadContext<'_>>,
    ) -> Result<(), String> {
        let parts = mind
            .state
            .as_any_mut()
            .downcast_mut::<RestoredMindParts>()
            .ok_or("restored Mind was not built by this adapter")?;
        let (saved, consumed) =
            aa_mind::saved_load::decode(parts.installed.kind, bytes).map_err(|e| e.to_string())?;
        if consumed != bytes.len() {
            return Err("saved Mind payload length changed between decode and Load".into());
        }
        if parts.mind.is_none() {
            return Err("restored Mind core is absent".into());
        }
        if !parts.commands.is_empty() || parts.regiment_owned.is_some() {
            return Err("restored Mind already owns saved allocations".into());
        }
        let mut commands = Vec::new();
        let mut bindings = aa_mind::saved_load::CommandBindings::default();
        if let Some(base) = saved.base() {
            for (current, command) in base.commands.iter().map(|v| (false, v))
                .chain(base.current_command.iter().map(|v| (true, v))) {
                let value = crate::saved_commands::bind(self, command, parts.precision)?;
                let id = self.command_identity.get().checked_add(1)
                    .ok_or("saved command identity space exhausted")?;
                self.command_identity.set(id);
                commands.push((id, value));
                if current { bindings.current = Some(id); }
                else { bindings.pending.push(id); }
            }
        }
        let id = parts.installed.mind_id;
        let mut arena = aa_mind::Arena::default();
        arena.minds.insert(id, parts.mind.take().ok_or("restored Mind core is absent")?);
        arena.states = std::mem::take(&mut parts.states);
        let result = match (saved, context) {
            (aa_mind::saved_load::Saved::Regiment(saved), Some(context)) if saved.version >= 2 => {
                crate::saved_regiment_mind::apply(
                    self, saved, &mut parts.installed, &mut arena, &bindings,
                    crate::saved_regiment_mind::LoadContext {
                        owner: context.owner,
                        world_time_bits: context.world_time_bits,
                        organization: context.organization,
                    }, parts.precision,
                ).map(|applied| (None, Some(applied.owned)))
            }
            (saved, _) => aa_mind::saved_load::apply_with_commands(
                &saved, &mut parts.installed, &mut arena, &bindings,
            ).map(|applied| (applied.element_path, None)).map_err(|e| e.to_string()),
        };
        // apply validates before mutating; the arena always returns intact.
        parts.states = arena.states;
        parts.mind = arena
            .minds
            .remove(&id);
        if parts.mind.is_none() {
            return Err("restored Mind core disappeared".into());
        }
        let (element_path, regiment_owned) = result?;
        parts.element_path = element_path;
        parts.regiment_owned = regiment_owned;
        parts.commands = commands;
        Ok(())
    }
}
impl RestoreDefinitions for Catalog<'_> {
    fn decode_saved_mind(
        &self,
        definition_mask: u32,
        bytes: &[u8],
    ) -> Result<(Option<aa_mapworld::actor_payload::UpdatablePayload>, usize), String> {
        let kind = mind_kind(definition_mask).ok_or(
            "saved Mind but factory 4CF480 constructs none (4D8290 would call Load on null)",
        )?;
        let (saved, consumed) =
            aa_mind::saved_load::decode(kind, bytes).map_err(|e| e.to_string())?;
        let updatable = saved
            .updatable()
            .map(|u| aa_mapworld::actor_payload::UpdatablePayload {
                version: u.version,
                serialized_id: u.serialized_id,
            });
        Ok((updatable, consumed))
    }
    fn construct_mind(
        &self,
        owner: aa_mapworld::restore::MindOwner<'_>,
    ) -> Result<Option<aa_mapworld::restore_components::RestoredMind>, String> {
        use aa_actors::components::ComponentSlot;
        let Some(kind) = mind_kind(owner.definition_mask) else {
            return Ok(None);
        };
        let abilities = match owner.components.get(ComponentSlot::Performer) {
            Some(aa_actors::owned::Component::Performer(p)) => p.state.abilities.clone(),
            None => Vec::new(),
            Some(_) => return Err("Performer slot payload mismatch".into()),
        };
        let mut arena = self
            .ability_arena
            .try_borrow_mut()
            .map_err(|_| "nested Ability arena access during Mind construction")?;
        let made = aa_mind::construct::construct(
            kind,
            owner.id.raw(),
            &mut RestoreMindWorld {
                rng: owner.rng,
                abilities: &abilities,
                arena: &mut arena,
            },
            owner.precision,
        )
        .map_err(|e| e.to_string())?;
        let (mut next_mind, mut next_state) = self.mind_identities.get();
        let mind_id = aa_mind::MindId(next_mind);
        next_mind = next_mind
            .checked_add(1)
            .ok_or("Mind identity space exhausted")?;
        let mut bindings = BTreeMap::new();
        for state in &made.states {
            bindings.insert(state.offset, aa_mind::StateId(next_state));
            next_state = next_state
                .checked_add(1)
                .ok_or("state identity space exhausted")?;
        }
        self.mind_identities.set((next_mind, next_state));
        let mut minds = aa_mind::Arena::default();
        let installed = made
            .install(mind_id, &bindings, &mut minds)
            .map_err(|e| e.to_string())?;
        let mind = minds
            .minds
            .remove(&mind_id)
            .ok_or("installed Mind core disappeared")?;
        Ok(Some(aa_mapworld::restore_components::RestoredMind {
            required: mind_requirement(kind)?,
            state: Box::new(RestoredMindParts {
                installed,
                mind: Some(mind),
                states: minds.states,
                element_path: None,
                regiment_owned: None,
                precision: owner.precision,
                commands: Vec::new(),
            }),
        }))
    }
    fn load_saved_mind(
        &self,
        mind: &mut aa_mapworld::restore_components::RestoredMind,
        bytes: &[u8],
    ) -> Result<(), String> {
        self.load_saved_mind_bound(mind, bytes, None)
    }
    fn load_saved_mind_with_context(
        &self,
        mind: &mut aa_mapworld::restore_components::RestoredMind,
        bytes: &[u8],
        context: aa_mapworld::restore_components::MindLoadContext<'_>,
    ) -> Result<(), String> {
        self.load_saved_mind_bound(mind, bytes, Some(context))
    }
    fn construct_performer(
        &self,
        owner: aa_mapworld::restore::PerformerOwner<'_>,
    ) -> Result<aa_combat::constructors::OwnedPerformer, String> {
        let mut definitions = self
            .definition_bank
            .try_borrow_mut()
            .map_err(|_| "nested definition bank construction")?;
        let mut abilities = self
            .ability_arena
            .try_borrow_mut()
            .map_err(|_| "nested Ability arena construction")?;
        crate::saved_performer::construct(
            self,
            &mut definitions,
            &mut abilities,
            owner.id,
            owner.definition_id,
            owner.components,
            owner.flags,
            owner.actor_words,
            crate::saved_performer::SavedShell {
                object_byte_4c: owner.object_byte_4c,
                thing_word_c: owner.thing_word_c,
            },
            owner.precision,
        )
    }
    fn apply_performer_ability(
        &self,
        handle: u32,
        saved: &aa_combat::commander::SavedAbility,
    ) -> Result<(), String> {
        self.ability_arena
            .try_borrow_mut()
            .map_err(|_| "nested saved Ability update")?
            .apply_saved(handle, saved)
    }
    fn construct_defender(
        &self,
        owner: aa_mapworld::restore::DefenderOwner<'_>,
    ) -> Result<aa_actors::defender_state::DefenderState, String> {
        let pc = owner.precision;
        let binding = self.actor_bindings(owner.definition_id, pc)?;
        let id = binding.definition.proof.id.clone();
        let loaded = self
            .actors
            .native_loaded_defender(
                &id,
                pc,
                binding
                    .character
                    .as_ref()
                    .map(|v| v as &dyn aa_rules::abilities::AnimationResolver),
            )
            .map_err(|e| e.to_string())?
            .ok_or("Defender component mask lacks DefenderComponent")?;
        let mut bank = self
            .definition_bank
            .try_borrow_mut()
            .map_err(|_| "nested definition registration")?;
        let mut arena = self
            .ability_arena
            .try_borrow_mut()
            .map_err(|_| "nested Defender Ability construction")?;
        let mut abilities = Vec::new();
        // 567CD0: 4C8650 per ActorData+21C entry, appended in source order.
        for (index, ability) in loaded.abilities().iter().enumerate() {
            let local_name = format!("{id}/defender/{index}");
            let local = bank.intern("ActorLocalAbility", &local_name)?;
            let definition = match &ability.authored.shared_definition_id {
                Some(aa_rules::native::TypedValue::String { value, .. }) => {
                    bank.intern("AbilityDefinition", value)?
                }
                Some(_) => return Err("shared Ability IDS is not a string".into()),
                None => bank.intern("ActorLocalAbilityDefinition", &local_name)?,
            };
            let handle = arena.allocate(aa_combat::ability::Ability::new(
                definition,
                Some(local),
                aa_body::F32(ability.power_bits),
            ))?;
            abilities.push(handle);
        }
        Ok(aa_actors::defender_state::DefenderState::new(
            owner.id, abilities,
        ))
    }
    fn ability_word14(&self, handle: u32) -> Result<u32, String> {
        Ok(self
            .ability_arena
            .try_borrow()
            .map_err(|_| "nested Ability arena read")?
            .get(handle)?
            .word14)
    }
    fn actor_numbered(&self, id_utf16: &[u16], precision: Precision) -> Result<bool, String> {
        let id = String::from_utf16(id_utf16).map_err(|_| "Actor IDS has invalid UTF16")?;
        self.actors
            .native_actor_init_inputs(&id, precision)
            .and_then(|source| source.numbered.boolean())
            .map_err(|e| e.to_string())
    }
    fn dynamic_doodad_definition(
        &self,
        id_utf16: &[u16],
        precision: Precision,
    ) -> Result<aa_mapworld::dynamic_doodad::Definition, String> {
        let id = String::from_utf16(id_utf16).map_err(|_| "Doodad IDS has invalid UTF16")?;
        let source = self
            .actors
            .native_dynamic_doodad(&id, precision)
            .map_err(|e| e.to_string())?;
        Ok(aa_mapworld::dynamic_doodad::Definition {
            owner_id: source.id,
            component_mask: source.component_mask,
        })
    }
    fn nation_badge_count(
        &self,
        nation: &aa_mapworld::restore::NamedDefinition,
        precision: Precision,
    ) -> Result<u32, String> {
        self.actors
            .native_nation_badges(&nation.id, precision)
            .and_then(|value| value.count())
            .map_err(|e| e.to_string())
    }
    fn actor_definition_token(
        &self,
        id_utf16: &[u16],
        precision: Precision,
    ) -> Result<u32, String> {
        // A Find (427BD0, translations applied) without a definition is the
        // native null key (0).
        let found = String::from_utf16(id_utf16)
            .ok()
            .is_some_and(|id| self.actors.find(&id).is_some());
        if !found {
            return Ok(0);
        }
        let resolved = bootstrap::resolve_definition(self.actors, id_utf16, precision)
            .map_err(|e| e.to_string())?;
        if resolved.definition.category.native_kind != 2 {
            return Err("saved count key is not an ActorData definition".into());
        }
        self.definition_bank
            .try_borrow_mut()
            .map_err(|_| "nested definition registration")?
            .intern("ActorDefinition", &resolved.definition.id)
    }
    fn resolve_commander(
        &self,
        id_utf16: &[u16],
        precision: Precision,
    ) -> Result<Option<aa_mapworld::restore::CommanderDefinition>, String> {
        let id = String::from_utf16(id_utf16).map_err(|_| "Commander IDS has invalid UTF16")?;
        let maximum = self
            .actors
            .native_commander_constructor_inputs(&id, precision)
            .map_err(|e| e.to_string())?
            .experience_max_bits;
        Ok(Some(aa_mapworld::restore::CommanderDefinition {
            definition: self.commander_constructor(&id, precision)?,
            maximum_experience: aa_body::F32(maximum),
        }))
    }
    fn stockpile_resource_max_bits(&self, precision: Precision) -> Result<u32, String> {
        self.settings
            .as_ref()
            .ok_or("native SVar source table is unavailable")?
            .unique_fixed("EconomyStockpileResourceMax", precision)
            .map(|v| v.0)
            .map_err(|e| e.to_string())
    }
    fn resolve_named_definition(
        &self,
        domain: &str,
        id_utf16: &[u16],
        precision: Precision,
    ) -> Result<Option<aa_mapworld::restore::NamedDefinition>, String> {
        let id =
            String::from_utf16(id_utf16).map_err(|_| "definition ID contains invalid UTF16")?;
        // 4CDBB0 -> 48D500: an IDS with no declaration is a null Find (the
        // caller owns the native consequence, e.g. 4CDC70 skips a Resource).
        let Some(node) = self
            .actors
            .native_named_definition(domain, &id, precision)
            .map_err(|e| e.to_string())?
        else {
            return Ok(None);
        };
        let Some(aa_rules::native::TypedValue::String {
            value: canonical, ..
        }) = node.field("IDS", precision).map_err(|e| e.to_string())?
        else {
            return Err("named definition lacks typed IDS".into());
        };
        let index = if domain.eq_ignore_ascii_case("Resource") {
            let registry = self
                .actors
                .native_resource_registry(precision)
                .map_err(|e| e.to_string())?;
            Some(
                registry
                    .ids
                    .iter()
                    .position(|v| v.eq_ignore_ascii_case(&canonical))
                    .ok_or("Resource absent from its completed native registry")?
                    as u32,
            )
        } else {
            None
        };
        Ok(Some(aa_mapworld::restore::NamedDefinition {
            id: canonical,
            index,
        }))
    }
    fn register_updatable(
        &self,
        id: i32,
        target: &aa_mapworld::restore_components::UpdatableTarget,
    ) -> Result<(), String> {
        self.updatables
            .try_borrow_mut()
            .map_err(|_| "nested saved Updatable registration")?
            .as_mut()
            .ok_or("saved Updatable table has not been constructed")?
            .register(id, target.clone())
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    fn localize(&self, key_utf16: &[u16]) -> Result<Vec<u16>, String> {
        self.localization
            .as_ref()
            .ok_or("native localization source table is unavailable")?
            .display(key_utf16)
            .map_err(|e| e.to_string())
    }
    fn resource_domain_count(&self, precision: Precision) -> Result<u32, String> {
        self.actors
            .native_resource_domain(precision)
            .and_then(|domain| domain.count())
            .map_err(|e| e.to_string())
    }
    fn character_ground(
        &self,
        definition_id: &str,
        variant: i32,
        precision: Precision,
    ) -> Result<GroundDefinitions, String> {
        let present = self
            .actors
            .native_character_sets_present(definition_id, precision)
            .map_err(|e| e.to_string())?
            .ok_or("Character constructor for an actor without CharacterComponent")?;
        // 4D6D70 stance -1: the stance aspect table is not consulted.
        let set =
            aa_rules::native::character_view::native_character_view_set(&present, &[], -1, variant)
                .map_err(|e| e.to_string())?
                .ok_or("ActorData+1C0 animation set list is null")?;
        let set = u32::try_from(set).map_err(|_| "animation set index exceeds u32")?;
        let bindings = self.actor_bindings(definition_id, precision)?;
        let mut ground = bindings.ground_for_animation_set(set)?;
        // 563670 -> 4DAEF0 takes the first entry of the set's KIND-0 array
        // ([set+C][0] when [set+10] != 0), not the first authored row.
        let keys = ground
            .character_animations
            .take()
            .ok_or("Character view set has no animation list")?;
        let rows = bindings
            .character
            .as_ref()
            .ok_or("actor has no Character definition")?
            .for_variant(set)
            .map_err(|e| e.to_string())?;
        ground.character_animations = Some(
            rows.iter()
                .zip(keys)
                .filter(|(row, _)| row.timing.kind == 0)
                .map(|(_, key)| key)
                .collect(),
        );
        Ok(ground)
    }
    fn character_animation(
        &self,
        definition_id: &str,
        variant: i32,
        kind: i32,
        index: i32,
        precision: Precision,
    ) -> Result<Option<String>, String> {
        let present = self
            .actors
            .native_character_sets_present(definition_id, precision)
            .map_err(|e| e.to_string())?
            .ok_or("Character Load for an actor without CharacterComponent")?;
        let set =
            aa_rules::native::character_view::native_character_view_set(&present, &[], -1, variant)
                .map_err(|e| e.to_string())?
                .ok_or("ActorData+1C0 animation set list is null")?;
        let set = u32::try_from(set).map_err(|_| "animation set index exceeds u32")?;
        // Native per-kind arrays exist for the registered animation types
        // (the seven canonical types of registry 499FF0).
        if !(0..7).contains(&kind) {
            return Err(format!(
                "animation type {kind} outside the native type arrays"
            ));
        }
        let bindings = self.actor_bindings(definition_id, precision)?;
        let ground = bindings.ground_for_animation_set(set)?;
        let keys = ground
            .character_animations
            .ok_or("Character view set has no animation list")?;
        let rows = bindings
            .character
            .as_ref()
            .ok_or("actor has no Character definition")?
            .for_variant(set)
            .map_err(|e| e.to_string())?;
        let of_kind: Vec<_> = rows
            .iter()
            .zip(keys)
            .filter(|(row, _)| row.timing.kind == kind as u32)
            .map(|(_, key)| key)
            .collect();
        if index < 0 {
            return Err(format!(
                "animation index {index} reads before the type array"
            ));
        }
        Ok(of_kind.get(index as usize).cloned().flatten())
    }
    fn updatable_table_bound(&self) -> Result<i32, String> {
        let table = self
            .updatables
            .try_borrow()
            .map_err(|_| "nested Updatable table access")?;
        let table = table
            .as_ref()
            .ok_or("saved Updatable table is unavailable")?;
        i32::try_from(table.len()).map_err(|_| "Updatable table count exceeds i32".into())
    }
    fn updatable_slot_occupied(&self, id: i32) -> Result<bool, String> {
        let table = self
            .updatables
            .try_borrow()
            .map_err(|_| "nested Updatable table access")?;
        let table = table
            .as_ref()
            .ok_or("saved Updatable table is unavailable")?;
        // 50BD2D reads entry `id` (the bound check admits id == count, an
        // out-of-allocation read natively): refused explicitly.
        usize::try_from(id)
            .ok()
            .and_then(|i| table.slots().get(i))
            .map(Option::is_some)
            .ok_or_else(|| format!("Updatable ID {id} reads outside the saved table"))
    }
    fn unregister_updatable(
        &self,
        target: &aa_mapworld::restore_components::UpdatableTarget,
    ) -> Result<(), String> {
        let mut table = self
            .updatables
            .try_borrow_mut()
            .map_err(|_| "nested saved Updatable table access")?;
        let table = table
            .as_mut()
            .ok_or("saved Updatable table is unavailable")?;
        if !table.remove(target) {
            return Err("destroyed object's Updatable is not in the saved table".into());
        }
        Ok(())
    }
    fn release_abilities(&self, handles: &[u32]) -> Result<(), String> {
        let mut arena = self
            .ability_arena
            .try_borrow_mut()
            .map_err(|_| "nested Ability arena access")?;
        for handle in handles {
            arena.release(*handle)?;
        }
        Ok(())
    }
    fn actor_display_name(&self, definition_id: &str, _: Precision) -> Result<Vec<u16>, String> {
        let text = self
            .definition_text
            .as_ref()
            .ok_or("definition-time text table is unavailable")?;
        self.actors
            .native_display_name(definition_id, &mut |key| {
                text.lookup(key)
                    .map_err(|e| aa_rules::native::Error::Unsupported(e.to_string()))
            })
            .map_err(|e| e.to_string())
    }
    fn localize_optional(&self, key: &[u16]) -> Result<Option<Vec<u16>>, String> {
        self.localization
            .as_ref()
            .ok_or("native localization source table is unavailable")?
            .lookup(key)
            .map_err(|e| e.to_string())
    }
    fn organization_actor_data(&self, ids: &[u16], _: Precision) -> Result<Option<String>, String> {
        // 4BDBA0(ids, 0x2B): 427BD0 Find (translations), then the found
        // definition's virtual +38 must be nonzero; class 0x2B accepts any
        // category. KKC_ActorData (native kind 2; vtable A05540, whose +0
        // is LoadData 464BD0) has +38 = 4F0500 (`mov eax,1`); other found
        // categories' +38 is unported and stays an explicit error.
        let Ok(id) = String::from_utf16(ids) else {
            return Ok(None);
        };
        let Some(found) = self.actors.find(&id) else {
            return Ok(None);
        };
        let canonical = found.id.clone();
        let kind = self
            .actors
            .native_category(&canonical)
            .map_err(|e| e.to_string())?
            .native_kind;
        if kind != 2 {
            return Err(format!(
                "4BDBA0 virtual +38 for non-ACTOR definition {canonical} is unported"
            ));
        }
        Ok(Some(canonical))
    }
    fn organization_layout_data(
        &self,
        ids: &[u16],
        precision: Precision,
    ) -> Result<Option<String>, String> {
        // 4C3050 -> 463FA0 on [B3AB00]. Use the type loader's registry,
        // including its translation and partial-load identities, rather
        // than the generic named-definition projection.
        let Ok(ids) = String::from_utf16(ids) else {
            return Ok(None);
        };
        if ids.is_empty() {
            return Ok(None);
        }
        Ok(self
            .layout_data(precision)?
            .find_layout(&ids)
            .map(|layout| layout.ids.clone()))
    }
    fn organization_layout_info(
        &self,
        layout: &str,
        precision: Precision,
    ) -> Result<aa_actors::organization_state::LayoutInfo, String> {
        let data = self.layout_data(precision)?;
        let layout = data
            .layouts
            .iter()
            .find(|data| data.ids.eq_ignore_ascii_case(layout))
            .ok_or_else(|| format!("no OrganizationLayoutData {layout}"))?;
        let fields = data
            .layout_info(layout)
            .map_err(|error| error.to_string())?;
        Ok(aa_actors::organization_state::LayoutInfo {
            name: fields.name,
            primary_slot: fields.primary_slot,
            entries: fields
                .entries
                .into_iter()
                .map(|entry| aa_actors::organization_state::LayoutEntry {
                    property: entry.property,
                    property_name: entry.property_name,
                    required: entry.required,
                    locked: entry.locked,
                    morale_line: entry.morale_line,
                })
                .collect(),
            slot_groups: fields.slot_groups,
            formations: fields.formations,
            formation_44: fields.formation_44,
            formation_48: fields.formation_48,
            property_50: fields.property_50,
        })
    }
    fn organization_formation_facts(
        &self,
        formation: u32,
        precision: Precision,
    ) -> Result<(u8, u32), String> {
        self.layout_data(precision)?
            .formation_facts(formation)
            .map_err(|error| error.to_string())
    }
    fn organization_has_property(
        &self,
        actor_data: &str,
        property: u32,
        precision: Precision,
    ) -> Result<bool, String> {
        self.require_untranslated_actor_identity(actor_data)?;
        self.layout_data(precision)?
            .actor_has_property(self.actors, actor_data, property, precision)
            .map_err(|error| error.to_string())
    }
    fn organization_morale_contribution(
        &self,
        actor_data: &str,
        precision: Precision,
    ) -> Result<u32, String> {
        self.require_untranslated_actor_identity(actor_data)?;
        aa_rules::native::actor_morale::actor_morale_contribution(
            self.actors,
            actor_data,
            precision,
        )
        .map_err(|error| error.to_string())
    }
    fn organization_modifier_minimum(&self, precision: Precision) -> Result<u32, String> {
        self.settings
            .as_ref()
            .ok_or("native settings source is unavailable")?
            .unique_fixed("ModifierAttributeClampAdd", precision)
            .map(|(bits, _)| bits)
            .map_err(|error| error.to_string())
    }
    fn organization_attach_member(
        &self,
        context: aa_mapworld::restore::OrganizationAttachContext<'_>,
        member: aa_actors::ActorId,
        slot: i32,
    ) -> Result<(), String> {
        crate::saved_organization_actions::attach(self, context, member, slot)
    }
    fn organization_combat_value(
        &self,
        context: aa_mapworld::restore::OrganizationCombatContext<'_>,
    ) -> Result<(), String> {
        self.require_source_owner()?;
        crate::saved_organization_combat::recompute(self, context)
    }
    fn actor_set_state(
        &self,
        context: aa_mapworld::restore::ActorStateContext<'_>,
        index: u8,
        enabled: bool,
    ) -> Result<(), String> {
        crate::saved_actor_state::set(context, index, enabled)
    }
    fn organization_set_leader(
        &self,
        context: aa_mapworld::restore::OrganizationLeaderContext<'_>,
        member: aa_actors::ActorId,
    ) -> Result<(), String> {
        crate::saved_organization_leader::set_leader(self, context, member)
    }
    fn actor_refresh_build_options(
        &self,
        context: aa_mapworld::restore::ActorPropertiesContext<'_>,
    ) -> Result<(), String> {
        crate::saved_actor_properties::refresh(self, context)
    }
    fn actor_refresh_economy(
        &self,
        context: aa_mapworld::restore::OrganizationAttachContext<'_>,
    ) -> Result<(), String> {
        crate::saved_owner_economy::refresh(self, context)
    }
    fn actor_rebuild_actions(
        &self,
        context: aa_mapworld::restore::OrganizationAttachContext<'_>,
    ) -> Result<(), String> {
        crate::saved_owner_actions::rebuild(self, context)
    }
    fn organization_member_properties(
        &self,
        context: aa_mapworld::restore::OrganizationAttachContext<'_>,
        member: aa_actors::ActorId,
    ) -> Result<(), String> {
        crate::saved_member_properties::member_added(self, context, member)
    }
    fn member_definition_properties(
        &self,
        canonical_actor: &str,
        precision: Precision,
    ) -> Result<Vec<u32>, String> {
        self.require_untranslated_actor_identity(canonical_actor)?;
        self.actors
            .native_member_property_data(self.layout_data(precision)?.as_ref(), canonical_actor)
            .map_err(|error| error.to_string())?
            .properties
            .into_iter()
            .map(|property| property.handle().map_err(|error| error.to_string()))
            .collect()
    }
    fn actor_has_applied_property(
        &self,
        context: aa_mapworld::restore::ActorPropertyLookupContext<'_>,
        property: u32,
        source: aa_actors::ActorId,
    ) -> Result<bool, String> {
        crate::saved_member_properties::find_actual(crate::saved_member_properties::LookupContext {
            owner: context.owner,
            owner_actor: context.owner_actor,
            registry: context.registry,
        }, property, source)
    }
    fn organization_follow_leader(
        &self,
        context: aa_mapworld::restore::OrganizationLeaderContext<'_>,
        member: aa_actors::ActorId,
    ) -> Result<(), String> {
        crate::saved_organization_position::follow_leader(self, context, member)
    }
    fn thing_definition_exists(&self, id_utf16: &[u16], _: Precision) -> Result<bool, String> {
        // 427BD0 compares wide strings; unpaired surrogates match no IDS.
        Ok(String::from_utf16(id_utf16)
            .ok()
            .is_some_and(|id| self.actors.find(&id).is_some()))
    }
    fn resolve(&self, id_utf16: &[u16], precision: Precision) -> Result<RuntimeDefinition, String> {
        let base = bootstrap::resolve_definition(self.actors, id_utf16, precision)
            .map_err(|e| e.to_string())?;
        let id = &base.definition.id;
        let kind = base.definition.category.native_kind;
        let object = if matches!(kind, 1 | 2 | 20 | 21) {
            Some(self.object_creation(id, precision)?)
        } else {
            None
        };
        if kind == 2 {
            return Ok(self.actor_bindings(id, precision)?.definition.clone());
        }
        let component_mask = None;
        let ground = GroundDefinitions::default();
        Ok(RuntimeDefinition {
            proof: base.definition,
            native_class: base.native_class,
            flags: base.flag_word,
            object,
            component_mask,
            ground,
        })
    }
}
