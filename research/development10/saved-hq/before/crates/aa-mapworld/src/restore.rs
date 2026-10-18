//! Staged native saved-object restoration. Registration is observable before
//! derived loading; partial records retain exact source data and an explicit
//! completion state. This is not a fresh-create path.
pub use crate::restore_components::{BodyDefinition, GroundDefinitions, RestoredComponent};
pub use crate::restore_lifecycle::Mode as RecordLifecycleMode;
use crate::{
    WorldInventory,
    bootstrap::DefinitionProof,
    inventory::Placement,
    kingdoms::{KingdomRecord, TeamRecord},
};
use aa_actors::{
    ActorId, IdentityRegistry,
    creation::{ObjectDefinition, select_variant},
    loading::{LoadKind, LoadShell, construct_shell_with_components},
};
use aa_numeric::{Binary, Precision};
use aa_simulation::NativeRng;
use serde::Serialize;

#[derive(Clone, Debug)]
pub struct RuntimeDefinition {
    pub proof: DefinitionProof,
    pub native_class: u32,
    pub flags: u32,
    pub object: Option<ObjectDefinition>,
    pub component_mask: Option<u32>,
    pub ground: GroundDefinitions,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedDefinition {
    pub id: String,
    /// Native registry ordinal only when registration order has been proven.
    pub index: Option<u32>,
}
/// The adapter must resolve source IDs through original typed definition rules.
/// A saved record's class is checked afterward and cannot supply this metadata.
pub trait RestoreDefinitions {
    /// Diagnostics retain the explicit omitted tail. Runtime adapters opt in
    /// only when they own the required native lifecycle services below.
    fn record_lifecycle_mode(&self) -> RecordLifecycleMode {
        RecordLifecycleMode::DiagnosticOmitted
    }
    fn lifecycle_init_null(
        &self,
        _context: &mut crate::restore_lifecycle::Context<'_>,
        _record: ActorId,
    ) -> std::result::Result<u8, String> {
        Err("immediate Init(NULL) native service is unavailable".into())
    }
    fn lifecycle_editor_insert_out_slot(
        &self,
        _context: &mut crate::restore_lifecycle::Context<'_>,
    ) -> std::result::Result<(), String> {
        Err("542A00 editor out-slot insertion is unavailable".into())
    }
    fn lifecycle_post_create_null_one(
        &self,
        _context: &mut crate::restore_lifecycle::Context<'_>,
        _record: ActorId,
    ) -> std::result::Result<(), String> {
        Err("immediate PostCreate(NULL,1) native service is unavailable".into())
    }
    fn lifecycle_register_record(
        &self,
        _context: &mut crate::restore_lifecycle::Context<'_>,
        _record: ActorId,
    ) -> std::result::Result<(), String> {
        Err("4F9510 immediate world registration is unavailable".into())
    }
    fn lifecycle_object_predicate(
        &self,
        _context: &mut crate::restore_lifecycle::Context<'_>,
        _record: ActorId,
    ) -> std::result::Result<u8, String> {
        Err("immediate record virtual+B0 is unavailable".into())
    }
    fn lifecycle_definition_publish_one(
        &self,
        _context: &mut crate::restore_lifecycle::Context<'_>,
        _record: ActorId,
    ) -> std::result::Result<(), String> {
        Err("4296B0 immediate definition publication is unavailable".into())
    }
    fn lifecycle_init_failed_warning(
        &self,
        _context: &mut crate::restore_lifecycle::Context<'_>,
        _record: ActorId,
        _precision: Precision,
    ) -> std::result::Result<(), String> {
        Err("5A1040 Init(false) warning services are unavailable".into())
    }
    fn lifecycle_destroy_record(
        &self,
        _context: &mut crate::restore_lifecycle::Context<'_>,
        _record: ActorId,
    ) -> std::result::Result<(), String> {
        Err("4F5490 actual lifecycle destruction services are unavailable".into())
    }
    fn construct_performer(
        &self,
        _owner: PerformerOwner<'_>,
    ) -> std::result::Result<aa_combat::constructors::OwnedPerformer, String> {
        Err("Performer constructor requires the shared Ability arena and actor callback".into())
    }
    fn apply_performer_ability(
        &self,
        _handle: u32,
        _saved: &aa_combat::commander::SavedAbility,
    ) -> std::result::Result<(), String> {
        Err("Performer saved Ability arena is unavailable".into())
    }
    /// Original 567CD0: one 4C8650 Ability per ActorData+21C local ability,
    /// allocated in the shared world Ability arena.
    fn construct_defender(
        &self,
        _owner: DefenderOwner<'_>,
    ) -> std::result::Result<aa_actors::defender_state::DefenderState, String> {
        Err("Defender constructor requires the shared Ability arena".into())
    }
    /// Ability+14 of an arena Ability (read by 567020 below version 2).
    fn ability_word14(&self, _handle: u32) -> std::result::Result<u32, String> {
        Err("Ability arena is unavailable".into())
    }
    fn actor_numbered(
        &self,
        _id_utf16: &[u16],
        _precision: Precision,
    ) -> std::result::Result<bool, String> {
        Err("resolved ActorData numbered flag is unavailable".into())
    }
    fn dynamic_doodad_definition(
        &self,
        _id_utf16: &[u16],
        _precision: Precision,
    ) -> std::result::Result<crate::dynamic_doodad::Definition, String> {
        Err("resolved DynamicDoodad component definition is unavailable".into())
    }
    fn resolve(
        &self,
        id_utf16: &[u16],
        precision: Precision,
    ) -> std::result::Result<RuntimeDefinition, String>;
    /// 427BD0 on the Thing DataManager (translations applied): whether a
    /// saved IDS names any registered Thing definition.
    fn thing_definition_exists(
        &self,
        _id_utf16: &[u16],
        _precision: Precision,
    ) -> std::result::Result<bool, String> {
        Err("native Thing definition lookup 427BD0 is unavailable".into())
    }
    fn resource_domain_count(&self, _precision: Precision) -> std::result::Result<u32, String> {
        Err("missing completed Resource registry count for Kingdom constructor".into())
    }
    fn localize(&self, _key_utf16: &[u16]) -> std::result::Result<Vec<u16>, String> {
        Err("native localization lookup is unavailable".into())
    }
    fn register_updatable(
        &self,
        _serialized_id: i32,
        _target: &crate::restore_components::UpdatableTarget,
    ) -> std::result::Result<(), String> {
        Err("central saved Updatable ID table is unavailable".into())
    }
    fn resolve_named_definition(
        &self,
        _domain: &str,
        _id_utf16: &[u16],
        _precision: Precision,
    ) -> std::result::Result<Option<NamedDefinition>, String> {
        Err("native named-definition lookup is unavailable".into())
    }
    fn stockpile_resource_max_bits(
        &self,
        _precision: Precision,
    ) -> std::result::Result<u32, String> {
        Err("resolved EconomyStockpileResourceMax setting is unavailable".into())
    }
    fn resolve_commander(
        &self,
        _id_utf16: &[u16],
        _precision: Precision,
    ) -> std::result::Result<Option<CommanderDefinition>, String> {
        Err("resolved Commander constructor inputs are unavailable".into())
    }
    /// Portable canonical identity only. This token is not a native pointer,
    /// registry ordinal, or a substitute for the original pointer hash. Zero
    /// represents a genuinely missing definition; unsupported evaluation errors.
    fn actor_definition_token(
        &self,
        _id_utf16: &[u16],
        _precision: Precision,
    ) -> std::result::Result<u32, String> {
        Err("canonical ActorDefinition identity bank is unavailable".into())
    }
    fn nation_badge_count(
        &self,
        _nation: &NamedDefinition,
        _precision: Precision,
    ) -> std::result::Result<u32, String> {
        Err("resolved NationData Badge texture count is unavailable".into())
    }
    /// `[GWorld+284]+0`: the saved Updatable table's count word (50BD26).
    fn updatable_table_bound(&self) -> std::result::Result<i32, String> {
        Err("central saved Updatable ID table is unavailable".into())
    }
    /// Whether the saved Updatable table slot `id` is already bound.
    fn updatable_slot_occupied(&self, _id: i32) -> std::result::Result<bool, String> {
        Err("central saved Updatable ID table is unavailable".into())
    }
    /// 50B120 on the saved Updatable table: an object destroyed during the
    /// load (4F5490) removes its Updatables' entries.
    fn unregister_updatable(
        &self,
        _target: &crate::restore_components::UpdatableTarget,
    ) -> std::result::Result<(), String> {
        Err("central saved Updatable ID table is unavailable".into())
    }
    /// The Ability destructors of a destroyed Performer/Defender (shared arena).
    fn release_abilities(&self, _handles: &[u32]) -> std::result::Result<(), String> {
        Err("Ability arena release is unavailable".into())
    }
    /// Pure decode of a saved Mind (slot 4) for the Mind kind the factory
    /// 4CF480 selects from ActorData+174 (`definition_mask`). Returns the
    /// base-Load Updatable record and the bytes consumed.
    fn decode_saved_mind(
        &self,
        _definition_mask: u32,
        _bytes: &[u8],
    ) -> std::result::Result<(Option<crate::actor_payload::UpdatablePayload>, usize), String> {
        Err("saved Mind decoder is unavailable".into())
    }
    /// 4CF480 case 4 and the concrete constructor (Element 535BA0 draws the
    /// shared world RNG and writes every Performer ability's jitter).
    fn construct_mind(
        &self,
        _owner: MindOwner<'_>,
    ) -> std::result::Result<Option<crate::restore_components::RestoredMind>, String> {
        Err("Mind constructor requires the shared Mind arena".into())
    }
    /// Ground inputs for a restored Character (slot 2): 563670 takes the
    /// first animation of 4D0720 -> 45E690, i.e. ActorData+1BC[set] where
    /// the saved-load constructor 4D6D70 leaves stance -1, so the set comes
    /// from the LOADED Object variant (`-1`/null entry -> the base set).
    fn character_ground(
        &self,
        _definition_id: &str,
        _variant: i32,
        _precision: Precision,
    ) -> std::result::Result<GroundDefinitions, String> {
        Err("resolved Character view set selection is unavailable".into())
    }
    /// The same view set's per-kind arrays (see `CharacterAnimation`).
    fn character_animation(
        &self,
        _definition_id: &str,
        _variant: i32,
        _kind: i32,
        _index: i32,
        _precision: Precision,
    ) -> std::result::Result<Option<String>, String> {
        Err("resolved Character view animation arrays are unavailable".into())
    }
    /// 4B9F40 on the string table [B36544] (map strings included): the
    /// text, or None where the original returns its (empty) default.
    fn localize_optional(&self, _key: &[u16]) -> std::result::Result<Option<Vec<u16>>, String> {
        Err("native string-table lookup is unavailable".into())
    }
    /// ActorData+10: 47D960's GetLocalized("name") (5B5620: stored text, a
    /// `#key` through 4B9F40 with an empty default).
    fn actor_display_name(
        &self,
        _definition_id: &str,
        _precision: Precision,
    ) -> std::result::Result<Vec<u16>, String> {
        Err("ActorData+10 GetLocalized(\"name\") producer is unavailable".into())
    }
    /// 4BDBA0 on the Thing DataManager [B39F58] with class 0x2B
    /// (KKC_ActorData): canonical IDS, or None (the caller throws).
    fn organization_actor_data(
        &self,
        _ids: &[u16],
        _precision: Precision,
    ) -> std::result::Result<Option<String>, String> {
        Err("ActorData reference lookup 4BDBA0 is unavailable".into())
    }
    /// 463FA0 on [B3AB00] (KKC_OrganizationLayoutData): canonical IDS.
    fn organization_layout_data(
        &self,
        _ids: &[u16],
        _precision: Precision,
    ) -> std::result::Result<Option<String>, String> {
        Err("OrganizationLayoutData lookup 463FA0 is unavailable".into())
    }
    /// LayoutData fields read by the Organization/Transport loaders.
    fn organization_layout_info(
        &self,
        _layout: &str,
        _precision: Precision,
    ) -> std::result::Result<aa_actors::organization_state::LayoutInfo, String> {
        Err("OrganizationLayoutData +8..+50 fields are unavailable".into())
    }
    /// LayoutFormation +C -> FormationData +1A (aggressive) and +24
    /// (Property handle). Handles share the LayoutInfo producer's arena.
    fn organization_formation_facts(
        &self,
        _formation: u32,
        _precision: Precision,
    ) -> std::result::Result<(u8, u32), String> {
        Err("formation +C definition facts are unavailable".into())
    }
    /// 45EEF0 ActorData+1D4 membership using the LayoutInfo Property
    /// handles, not unrelated named-definition ordinals.
    fn organization_has_property(
        &self,
        _actor_data: &str,
        _property: u32,
        _precision: Precision,
    ) -> std::result::Result<bool, String> {
        Err("45EEF0 ActorData property test is unavailable".into())
    }
    /// ActorData+2C0 returned by 45D580 with a null modifier context.
    fn organization_morale_contribution(
        &self,
        _actor_data: &str,
        _precision: Precision,
    ) -> std::result::Result<u32, String> {
        Err("ActorData+2C0 morale contribution (45D580) is unavailable".into())
    }
    /// 501C90 uses the actual ModifierAttributeClampAdd setting, not a
    /// constant chosen by the Organization adapter.
    fn organization_modifier_minimum(
        &self,
        _precision: Precision,
    ) -> std::result::Result<u32, String> {
        Err("ModifierAttributeClampAdd setting is unavailable".into())
    }
    /// 5705A0 attachment and its real callbacks while the parent's
    /// Organization is temporarily held by 57D130 rather than its slot.
    fn organization_attach_member(
        &self,
        _context: OrganizationAttachContext<'_>,
        _member: ActorId,
        _slot: i32,
    ) -> std::result::Result<(), String> {
        Err("5705A0 Element attachment services are unavailable".into())
    }
    ///5788B0 over the same in-progress Organization and owned member shells.
    /// Missing source/owner services must preserve completed stores and error.
    fn organization_combat_value(
        &self,
        _context: OrganizationCombatContext<'_>,
    ) -> std::result::Result<(), String> {
        Err("5788B0 combat value over the restore shells is unavailable".into())
    }
    ///4D64F0 operates on the held Actor and actual Thing flags, including
    /// partial stores before a required enter/leave/action callback fails.
    fn actor_set_state(
        &self,
        _context: ActorStateContext<'_>,
        _index: u8,
        _enabled: bool,
    ) -> std::result::Result<(), String> {
        Err("4D64F0 saved Actor state services are unavailable".into())
    }
    ///4D4CF0 rebuilds the actual Actor build-target list and propagates to
    /// Element parents. The active Organization is held outside the registry.
    fn actor_refresh_build_options(
        &self,
        _context: ActorPropertiesContext<'_>,
    ) -> std::result::Result<(), String> {
        Err("4D4CF0 saved Actor build-target services are unavailable".into())
    }
    ///4CF0F0 reaches the actual held owner's Economy570020 when present.
    fn actor_refresh_economy(
        &self,
        _context: OrganizationAttachContext<'_>,
    ) -> std::result::Result<(), String> {
        Err("4CF0F0 saved owner Economy services are unavailable".into())
    }
    ///4D1350 rebuilds the held owner's actual action mask and publishes a
    /// changed mask only through the required native-gated services.
    fn actor_rebuild_actions(
        &self,
        _context: OrganizationAttachContext<'_>,
    ) -> std::result::Result<(), String> {
        Err("4D1350 saved owner action services are unavailable".into())
    }
    ///5784C0 walks the held owner's current Properties list and applies
    /// qualifying entries to the actual newly attached member.
    fn organization_member_properties(
        &self,
        _context: OrganizationAttachContext<'_>,
        _member: ActorId,
    ) -> std::result::Result<(), String> {
        Err("5784C0 saved member Property services are unavailable".into())
    }
    ///45EC60 exposes the member ActorData's authored SupportProperty list.
    fn member_definition_properties(
        &self,
        _canonical_actor: &str,
        _precision: Precision,
    ) -> std::result::Result<Vec<u32>, String> {
        Err("45EC60 saved member source Property list is unavailable".into())
    }
    fn actor_has_applied_property(
        &self,
        _context: ActorPropertyLookupContext<'_>,
        _property: u32,
        _source: ActorId,
    ) -> std::result::Result<bool, String> {
        Err("4CEF80 saved applied Property lookup is unavailable".into())
    }
    ///579A00 mutates the same held Organization and native load RNG.
    fn organization_set_leader(
        &self,
        _context: OrganizationLeaderContext<'_>,
        _member: ActorId,
    ) -> std::result::Result<(), String> {
        Err("579A00 saved Organization leader services are unavailable".into())
    }
    ///57C800 uses actual selected member/owner state. Position interpolation,
    /// relocation, Mind predicates and scheduling remain mandatory when reached.
    fn organization_follow_leader(
        &self,
        _context: OrganizationLeaderContext<'_>,
        _member: ActorId,
    ) -> std::result::Result<(), String> {
        Err("57C800 saved Organization position services are unavailable".into())
    }
    /// Concrete Mind Load over exactly the decoded bytes.
    fn load_saved_mind(
        &self,
        _mind: &mut crate::restore_components::RestoredMind,
        _bytes: &[u8],
    ) -> std::result::Result<(), String> {
        Err("saved Mind Load is unavailable".into())
    }
    /// Context-aware Load; older adapters retain their explicit bounded API.
    fn load_saved_mind_with_context(
        &self,
        mind: &mut crate::restore_components::RestoredMind,
        bytes: &[u8],
        _context: crate::restore_components::MindLoadContext<'_>,
    ) -> std::result::Result<(), String> {
        self.load_saved_mind(mind, bytes)
    }
}
/// organization_state::Definitions over the restore definitions adapter.
pub struct OrganizationDefinitions<'a, R: RestoreDefinitions + ?Sized> {
    pub definitions: &'a R,
    pub precision: Precision,
}
/// Real in-progress restored objects borrowed by the 5705A0 callbacks.
/// No actor/component snapshot is created or published as a live world.
pub struct OrganizationAttachContext<'a> {
    /// The separate RULES chunk is authoritative during GWorld::Load;
    /// MAP_HEADER rules are only creator/header metadata.
    pub inventory: &'a WorldInventory,
    pub owner: ActorId,
    pub owner_actor: &'a mut aa_actors::loading::ActorShell<RestoredComponent>,
    pub owner_definition: &'a RuntimeDefinition,
    pub owner_thing_flags: u32,
    pub owner_thing_word_c: u32,
    pub owner_object_byte_4c: u8,
    pub organization: &'a mut aa_actors::ground_states::OrganizationState<String>,
    pub registry: &'a mut IdentityRegistry<RestoredRecord>,
    pub precision: Precision,
}
/// Borrowed5788B0 view. The active Organization is temporarily outside its
/// owner's component slot; all other members retain generation-checked IDs.
/// This is an observed restore prefix, not proof of omitted Actor Init.
pub struct OrganizationCombatContext<'a> {
    pub owner: ActorId,
    pub owner_actor: &'a aa_actors::loading::ActorShell<RestoredComponent>,
    pub organization: &'a mut aa_actors::ground_states::OrganizationState<String>,
    pub registry: &'a IdentityRegistry<RestoredRecord>,
    pub precision: Precision,
}
/// Source-free state coordinator access. Effects that require further live
/// services remain explicit failures; no initialized-world claim is implied.
pub struct ActorStateContext<'a> {
    pub owner: ActorId,
    pub owner_actor: &'a mut aa_actors::loading::ActorShell<RestoredComponent>,
    pub owner_thing_flags: &'a mut u32,
}
/// Actual held Actor/list storage and its mid-load Organization membership.
/// Other Actors must resolve through the same generation-checked registry.
pub struct ActorPropertiesContext<'a> {
    pub owner: ActorId,
    pub owner_actor: &'a mut aa_actors::loading::ActorShell<RestoredComponent>,
    pub owner_definition: &'a RuntimeDefinition,
    pub organization: Option<&'a aa_actors::ground_states::OrganizationState<String>>,
    pub registry: &'a mut IdentityRegistry<RestoredRecord>,
    pub precision: Precision,
}
/// Read-only4CEF80 over the held owner and generation-checked saved actors.
pub struct ActorPropertyLookupContext<'a> {
    pub owner: ActorId,
    pub owner_actor: &'a aa_actors::loading::ActorShell<RestoredComponent>,
    pub registry: &'a IdentityRegistry<RestoredRecord>,
}
/// Actual in-progress leader/formation storage, owner transform, ordered world
/// RNG and diagnostic sink. The owner remains temporarily outside its registry.
pub struct OrganizationLeaderContext<'a> {
    pub owner: ActorId,
    pub owner_actor: &'a aa_actors::loading::ActorShell<RestoredComponent>,
    pub owner_thing: &'a aa_actors::loading::ThingShell<crate::bootstrap::DefinitionProof>,
    pub organization: &'a mut aa_actors::ground_states::OrganizationState<String>,
    pub registry: &'a IdentityRegistry<RestoredRecord>,
    pub rng: &'a mut NativeRng,
    pub native_messages: &'a mut Vec<NativeMessage>,
    pub source_ordinal: usize,
    pub precision: Precision,
}
fn org_error(error: String) -> aa_actors::organization_state::LoadError {
    aa_actors::organization_state::LoadError::Unsupported(error)
}
impl<R: RestoreDefinitions + ?Sized> aa_actors::organization_state::Definitions<String>
    for OrganizationDefinitions<'_, R>
{
    fn layout_data(
        &mut self,
        ids: &[u16],
    ) -> aa_actors::organization_state::Result<Option<String>> {
        self.definitions
            .organization_layout_data(ids, self.precision)
            .map_err(org_error)
    }
    fn actor_data(&mut self, ids: &[u16]) -> aa_actors::organization_state::Result<Option<String>> {
        self.definitions
            .organization_actor_data(ids, self.precision)
            .map_err(org_error)
    }
    fn layout_info(
        &self,
        layout: &String,
    ) -> aa_actors::organization_state::Result<aa_actors::organization_state::LayoutInfo> {
        self.definitions
            .organization_layout_info(layout, self.precision)
            .map_err(org_error)
    }
    fn name(&self, definition: &String) -> aa_actors::organization_state::Result<String> {
        Ok(definition.clone())
    }
}
pub struct MindOwner<'a> {
    pub id: ActorId,
    /// ActorData component mask (+174) selecting the concrete Mind.
    pub definition_mask: u32,
    /// Components constructed so far (the Performer precedes the Mind).
    pub components: &'a aa_actors::components::Components<RestoredComponent>,
    pub rng: &'a mut NativeRng,
    pub precision: Precision,
}
/// Resolved FactionData for Commander replacement 4E0F00: constructor inputs
/// plus the loaded FactionData+3C `experience_max` cap (48BE55).
#[derive(Clone, Debug)]
pub struct CommanderDefinition {
    pub definition: aa_combat::commander::Definition,
    pub maximum_experience: aa_body::F32,
}
pub struct DefenderOwner<'a> {
    pub id: ActorId,
    pub definition_id: &'a str,
    pub precision: Precision,
}
pub struct PerformerOwner<'a> {
    pub id: ActorId,
    pub definition_id: &'a str,
    pub components: &'a aa_actors::components::Components<RestoredComponent>,
    pub flags: &'a mut u32,
    pub actor_words: &'a mut std::collections::BTreeMap<u32, u32>,
    /// Object+4C (constructor zero; GObject::Load leaves it) for 4CFCC0.
    pub object_byte_4c: u8,
    /// Thing+C renderer link (null until visual creation) for 4CFCC0.
    pub thing_word_c: u32,
    pub precision: Precision,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum LoadCompletion {
    SourceIdentityOnly,
    RegisteredLoadShell,
    ThingLoaded,
    ObjectLoaded,
    ActorPrefixLoaded,
    ComponentsDecoded,
    ComponentsLoaded,
    AwaitingPostLoad,
    /// GWorld::Load's PostLoad pass (4FD3B3) has visited the object.
    PostLoaded,
}
#[derive(Clone, Debug)]
pub struct SourceWorldEntity<T> {
    pub source: T,
    pub completion: LoadCompletion,
    pub runtime: Option<WorldEntityRuntime>,
    pub failure: Option<String>,
}
#[derive(Clone, Debug)]
pub struct WorldEntityRuntime {
    pub definition: RuntimeDefinition,
    pub base: LoadShell<DefinitionProof, RestoredComponent>,
    /// Derived construction follows saved-ID registration in the base ctor.
    pub derived: Option<crate::special_state::SpecialShell>,
    /// Keeps all unresolved owner/property/localized references explicit.
    pub actor_tail: Option<crate::actor_payload::ActorTail>,
    pub derived_payload_loaded: bool,
}
#[derive(Clone, Debug)]
pub struct LoadedEntity {
    pub source: Placement,
    pub definition: RuntimeDefinition,
    pub shell: LoadShell<DefinitionProof, RestoredComponent>,
    pub completion: LoadCompletion,
    pub owner_source_slot: Option<usize>,
    pub saved_components: Vec<DecodedSavedComponent>,
    pub actor_tail: Option<crate::actor_payload::ActorTail>,
    pub doodad: Option<crate::dynamic_doodad::DynamicDoodad>,
    pub special_object: Option<crate::map_objects::SpecialObject>,
    pub failure: Option<String>,
    /// Actor +E4 restored from the saved current-component index (4D9271).
    pub active_component: Option<aa_actors::components::ComponentSlot>,
}
#[derive(Clone, Debug)]
pub struct DecodedSavedComponent {
    pub slot: u8,
    /// Absolute byte span in the original DYNAMIC_THINGS chunk.
    pub source_span: crate::inventory::ByteSpan,
    pub payload: crate::saved_components::SavedComponent,
}
#[derive(Clone, Debug)]
pub enum RestoredRecord {
    Team(Box<SourceWorldEntity<TeamRecord>>),
    Kingdom(Box<SourceWorldEntity<KingdomRecord>>),
    Thing(Box<LoadedEntity>),
    Object(Box<LoadedEntity>),
    Actor(Box<LoadedEntity>),
    /// A record taken out of the registry while its own Load runs, so that
    /// embedded records (4FC200 -> 4FAE80) can register themselves. It is
    /// always replaced before the record loader returns.
    Loading,
}
impl RestoredRecord {
    pub fn loaded(&self) -> Option<&LoadedEntity> {
        match self {
            Self::Thing(v) | Self::Object(v) | Self::Actor(v) => Some(v),
            _ => None,
        }
    }
    pub fn loaded_mut(&mut self) -> Option<&mut LoadedEntity> {
        match self {
            Self::Thing(v) | Self::Object(v) | Self::Actor(v) => Some(v),
            _ => None,
        }
    }
    pub fn completion(&self) -> LoadCompletion {
        match self {
            Self::Team(v) => v.completion,
            Self::Kingdom(v) => v.completion,
            Self::Thing(v) | Self::Object(v) | Self::Actor(v) => v.completion,
            Self::Loading => LoadCompletion::RegisteredLoadShell,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RestoreStage {
    Teams,
    Kingdoms,
    RequiredObjects,
    DynamicThings,
}
#[derive(Clone, Debug, Serialize)]
pub struct RestoreEvent {
    pub stage: RestoreStage,
    pub source_ordinal: usize,
    pub saved_id: Option<u32>,
    pub registered: bool,
    pub completion: Option<LoadCompletion>,
    pub error: Option<String>,
}
#[derive(Debug, Default, Serialize)]
pub struct RestoreReport {
    pub events: Vec<RestoreEvent>,
    pub stopped_before: Option<RestoreStage>,
    pub runtime_publication_ready: bool,
    /// Each successful Load records either its omitted diagnostic tail or the
    /// exact attempted/completed/error state of the immediate native tail.
    #[serde(default)]
    pub lifecycle: Vec<crate::restore_lifecycle::Event>,
    /// Invalid external identity mutation must not discard the actual held
    /// allocation. These records remain explicit unpublishable evidence.
    #[serde(skip)]
    pub lifecycle_detached: Vec<crate::restore_lifecycle::HeldRecord>,
    /// Trace of successful immediate publication into the runtime's central
    /// saved-ID table, in native stage order. This is not the table itself.
    pub updatable_registrations: Vec<crate::restore_components::UpdatableRegistration>,
    /// Native presentation callbacks (FX lists) reached during Load, in
    /// order. Recorded for the presentation layer; not executed here.
    pub presentation_effects: Vec<crate::restore_components::OwnerEffect>,
    /// DYNAMIC_THINGS records for which 4FAE80 yields null; 4FC340 then seeks
    /// to the next index entry (858F50) and the map keeps loading.
    pub skipped: Vec<SkippedRecord>,
    /// Native log/map-warning messages written by Load code that continues.
    pub native_messages: Vec<NativeMessage>,
    /// 4FC600's conversion vector: saved class-2 records whose definition
    /// is now another class (4F26F0/4EA050/4F9840), replaced by fresh
    /// creation at the very end of the map load (class_conversion::
    /// replace_converted, after PostLoad).
    #[serde(skip)]
    pub conversions: crate::class_conversion::ConversionQueue<RuntimeDefinition>,
    /// The records converted instead of loaded (no saved-ID registration).
    pub converted: Vec<ConvertedRecord>,
    /// GWorld `+2C` after the record sections: the saved GWORLD version,
    /// raised by GActor::Load (`ActorPrefix::upgraded_world_version`). The
    /// later 4FC600 sections compare the version read at its start instead.
    pub world_version: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConvertedRecord {
    pub source_ordinal: usize,
    pub saved_id: u32,
    /// Index in `RestoreReport::conversions`.
    pub queue_index: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NativeMessage {
    /// Top-level DYNAMIC_THINGS ordinal whose Load produced the message.
    pub source_ordinal: usize,
    /// 404EA0/404F30 (debug log) or 5A1040 (map warning).
    pub function: u32,
    pub line: u32,
    pub text: String,
}
/// Restore-wide mutable state shared by top-level and embedded record loads.
pub struct LoadContext<'a> {
    pub inventory: &'a WorldInventory,
    pub rng: &'a mut NativeRng,
    pub pc: Precision,
    pub updatable_registrations: &'a mut Vec<crate::restore_components::UpdatableRegistration>,
    pub presentation_effects: &'a mut Vec<crate::restore_components::OwnerEffect>,
    pub skipped: &'a mut Vec<SkippedRecord>,
    pub native_messages: &'a mut Vec<NativeMessage>,
    pub lifecycle: &'a mut Vec<crate::restore_lifecycle::Event>,
    pub lifecycle_detached: &'a mut Vec<crate::restore_lifecycle::HeldRecord>,
    /// GWorld+2C: the GWORLD version; GActor::Load raises 14 to 15 for an
    /// Actor version above 7 (`ActorPrefix::upgraded_world_version`).
    pub world_version: &'a mut u32,
    /// B3CC47, the 4FC200 `thing->IsGActor()` assert latch.
    pub assert_latch: &'a mut bool,
    /// Top-level DYNAMIC_THINGS ordinal of the record being loaded.
    pub source_ordinal: usize,
}
/// 4FAE80's null results before construction: generation (4FAF14), Thing
/// lookup 427BD0, class compare. Saved class 2 with a conversion context
/// (4FC600's vector) is converted instead of skipped.
fn null_record_cause(
    registry: &IdentityRegistry<RestoredRecord>,
    definitions: &impl RestoreDefinitions,
    thing_id: &[u16],
    class_code: u32,
    saved_id: u32,
    pc: Precision,
    conversion_context: bool,
) -> std::result::Result<Option<SkipCause>, String> {
    let id = ActorId::from_raw(saved_id);
    if registry.generation_at(id.slot()) != id.generation() {
        return Ok(Some(SkipCause::GenerationMismatch));
    }
    if !definitions.thing_definition_exists(thing_id, pc)? {
        return Ok(Some(SkipCause::UnknownThing));
    }
    Ok(match definitions.resolve(thing_id, pc) {
        Ok(definition)
            if crate::class_conversion::check_class(
                class_code,
                definition.native_class,
                conversion_context,
            ) == crate::class_conversion::ClassCheck::Skip =>
        {
            Some(SkipCause::ChangedClass {
                definition_class: definition.native_class,
            })
        }
        _ => None,
    })
}
/// 4FAE80 after its null checks: the class constructor (whose saved-ID
/// registration precedes the Load), Actor+138 = `owner_138`, then the virtual
/// Load. Returns (registered, completion, Load result with the subtype tail
/// bytes consumed).
struct LoadedRecordResult {
    output: Option<ActorId>,
    consumed: usize,
}
fn load_record(
    registry: &mut IdentityRegistry<RestoredRecord>,
    source: &Placement,
    owner_138: u32,
    nested: bool,
    definitions: &impl RestoreDefinitions,
    cx: &mut LoadContext<'_>,
) -> (
    bool,
    Option<LoadCompletion>,
    std::result::Result<Option<LoadedRecordResult>, String>,
) {
    let id = ActorId::from_raw(source.serialized_id);
    let pc = cx.pc;
    let registrations_before = cx.updatable_registrations.len();
    let registry_before = registry.len();
    let definition = match definitions.resolve(&source.thing_id_utf16, pc) {
        Ok(v) => v,
        Err(e) => return (false, None, Err(e)),
    };
    if definition.native_class != source.class_code {
        return (
            false,
            None,
            Err(format!(
                "source class {} differs from definition class {}",
                source.class_code, definition.native_class
            )),
        );
    }
    let kind = match definition.native_class {
        1 => LoadKind::Thing,
        2 | 6 | 7 | 9 => LoadKind::Object,
        3 => LoadKind::Actor,
        other => {
            return (
                false,
                None,
                Err(format!(
                    "unported load constructor for native class {other}"
                )),
            );
        }
    };
    let shell = construct_shell_with_components(kind, definition.proof.clone());
    let loaded = LoadedEntity {
        source: source.clone(),
        definition,
        shell,
        completion: LoadCompletion::RegisteredLoadShell,
        owner_source_slot: None,
        saved_components: Vec::new(),
        actor_tail: None,
        doodad: None,
        special_object: None,
        failure: None,
        active_component: None,
    };
    let record = match kind {
        LoadKind::Thing => RestoredRecord::Thing(Box::new(loaded)),
        LoadKind::Object => RestoredRecord::Object(Box::new(loaded)),
        LoadKind::Actor => RestoredRecord::Actor(Box::new(loaded)),
    };
    if let Err(e) = registry.register_saved(id, kind == LoadKind::Actor, record) {
        return (false, None, Err(e.to_string()));
    }
    let registered_owner = source.actor.as_ref().and_then(|actor| {
        registry
            .get(ActorId::from_raw(actor.owner_handle))
            .map(|entry| entry.id)
    });
    let Some(slot) = registry.value_mut(id) else {
        return (true, None, Err("registered entry disappeared".into()));
    };
    let mut record = std::mem::replace(slot, RestoredRecord::Loading);
    let entity = record
        .loaded_mut()
        .expect("the record registered above is a loaded entity");
    let mut owner_effects = Vec::new();
    let result = apply_loaded(
        entity,
        registry,
        definitions,
        cx,
        registered_owner,
        owner_138,
        nested,
        &mut owner_effects,
    );
    let completion = entity.completion;
    if let Err(error) = &result {
        cx.lifecycle.push(crate::restore_lifecycle::load_failed(
            RestoreStage::DynamicThings,
            cx.source_ordinal,
            id,
            nested,
            definitions.record_lifecycle_mode(),
            error,
        ));
    }
    if let Err(error) = &result
        && let Some(cancel) = caught_load_exception(error)
    {
        if let Err(ownership) =
            crate::restore_lifecycle::return_held(id, record, registry, cx.lifecycle_detached)
        {
            return (true, Some(completion), Err(format!("{error}; {ownership}")));
        }
        return (
            true,
            Some(completion),
            destroy_caught(
                registry,
                id,
                nested,
                registry_before,
                registrations_before,
                cancel,
                error,
                &owner_effects,
                definitions,
                cx,
                source,
            )
            .map(|v| {
                v.map(|consumed| LoadedRecordResult {
                    output: Some(id),
                    consumed,
                })
            }),
        );
    }
    // Constructor side effects stay applied even when a later Load stage of
    // this record failed, as the native writes would.
    let loaded_successfully = result.is_ok();
    let effects = apply_owner_effects(
        registry,
        &owner_effects,
        cx.presentation_effects,
        cx.native_messages,
        cx.source_ordinal,
    );
    let result = result.and_then(|consumed| effects.map(|_| consumed));
    if loaded_successfully && let Err(error) = &result {
        cx.lifecycle.push(crate::restore_lifecycle::load_failed(
            RestoreStage::DynamicThings,
            cx.source_ordinal,
            id,
            nested,
            definitions.record_lifecycle_mode(),
            error,
        ));
    }
    if let Some(entity) = record.loaded_mut() {
        entity.failure = result.as_ref().err().cloned();
    }
    let result = match result {
        Ok(consumed) => crate::restore_lifecycle::finish(
            id,
            record,
            definitions,
            crate::restore_lifecycle::Inputs {
                registry,
                inventory: cx.inventory,
                rng: cx.rng,
                native_messages: cx.native_messages,
                presentation_effects: cx.presentation_effects,
                events: cx.lifecycle,
                detached: cx.lifecycle_detached,
                stage: RestoreStage::DynamicThings,
                source_ordinal: cx.source_ordinal,
                nested,
                precision: pc,
                world_version: *cx.world_version,
            },
        )
        .map(|output| Some(LoadedRecordResult { output, consumed })),
        Err(error) => {
            let restored =
                crate::restore_lifecycle::return_held(id, record, registry, cx.lifecycle_detached);
            Err(match restored {
                Ok(()) => error,
                Err(ownership) => format!("{error}; {ownership}"),
            })
        }
    };
    if let Some(entity) = registry.value_mut(id).and_then(RestoredRecord::loaded_mut) {
        entity.failure = result.as_ref().err().cloned();
    }
    (true, Some(completion), result)
}
/// The 4FAE80 catch for a record whose Load threw (4FB410 / 4FB42A): the
/// object is destroyed with 4F5490 and the record yields null. Destruction
/// undoes what the partially loaded object published: its Updatable table
/// entries (50B120 via the component destructors), its arena Abilities
/// (Performer/Defender destructors), a Supply source (dtor 58A990 -> 4E48B0
/// removes it, so the deferred 4E1F00 effect is not applied) and its GTID.
/// Returns Ok(None) (null result) or an error for an unported destroy.
#[allow(clippy::too_many_arguments)]
fn destroy_caught(
    registry: &mut IdentityRegistry<RestoredRecord>,
    id: ActorId,
    nested: bool,
    registry_before: usize,
    registrations_before: usize,
    cancel: bool,
    error: &str,
    owner_effects: &[crate::restore_components::OwnerEffect],
    definitions: &impl RestoreDefinitions,
    cx: &mut LoadContext<'_>,
    source: &Placement,
) -> std::result::Result<Option<usize>, String> {
    if nested {
        return Err(format!(
            "embedded record Load threw ({error}); the enclosing stream position after the catch is unported"
        ));
    }
    if registry.len() != registry_before + 1 {
        return Err(format!(
            "record Load threw ({error}) after embedded records loaded; their destruction is unported"
        ));
    }
    let mut abilities = Vec::new();
    if let Some(actor) = registry
        .get(id)
        .and_then(|entry| entry.value.loaded())
        .and_then(|entity| entity.shell.actor.as_ref())
    {
        for (_, component) in actor.components.iter() {
            match component {
                RestoredComponent::Performer(p) => {
                    abilities.extend(p.state.abilities.iter().copied())
                }
                RestoredComponent::Defender(d) => abilities.extend(d.abilities.iter().copied()),
                _ => {}
            }
        }
    }
    if !abilities.is_empty() {
        definitions.release_abilities(&abilities)?;
    }
    for registration in cx.updatable_registrations.drain(registrations_before..) {
        definitions.unregister_updatable(&registration.target)?;
    }
    for effect in owner_effects {
        match effect {
            crate::restore_components::OwnerEffect::SupplySource { .. } => {}
            crate::restore_components::OwnerEffect::NativeMessage {
                function,
                line,
                text,
            } => cx.native_messages.push(NativeMessage {
                source_ordinal: cx.source_ordinal,
                function: *function,
                line: *line,
                text: text.clone(),
            }),
            crate::restore_components::OwnerEffect::PresentationFx { .. } => {
                cx.presentation_effects.push(effect.clone())
            }
        }
    }
    registry.remove(id).map_err(|e| e.to_string())?;
    let message = error
        .find(LOAD_WARNING_EXCEPTION)
        .map(|at| error[at + LOAD_WARNING_EXCEPTION.len()..].to_owned())
        .unwrap_or_else(|| error.to_owned());
    if !cancel {
        cx.native_messages.push(NativeMessage {
            source_ordinal: cx.source_ordinal,
            function: 0x5a1040,
            line: 0,
            text: format!("load_warning_thing_init_failed_for_reason|sds: {message}"),
        });
    }
    cx.skipped.push(SkippedRecord {
        source_ordinal: cx.source_ordinal,
        saved_id: id.raw(),
        thing_id_utf16: source.thing_id_utf16.clone(),
        class_code: source.class_code,
        cause: SkipCause::LoadException { cancel, message },
    });
    Ok(None)
}
/// 4FC200 -> 4FAE80(stream, &out, owner, 0): a complete record embedded in
/// a component Load (no length prefix, no conversion context). Returns the
/// loaded thing (None where the original returns null, having consumed only
/// the IDS/class/saved-ID prefix) and the bytes consumed.
pub fn load_embedded_record(
    registry: &mut IdentityRegistry<RestoredRecord>,
    bytes: &[u8],
    owner: ActorId,
    definitions: &impl RestoreDefinitions,
    cx: &mut LoadContext<'_>,
) -> std::result::Result<(Option<ActorId>, usize), String> {
    let (thing_id, class_code, saved_id) =
        record_prefix(bytes).ok_or("embedded record prefix is truncated")?;
    let prefix_len = 2 + 2 * thing_id.len() + 8;
    if let Some(cause) = null_record_cause(
        registry,
        definitions,
        &thing_id,
        class_code,
        saved_id,
        cx.pc,
        false,
    )? {
        cx.skipped.push(SkippedRecord {
            source_ordinal: cx.source_ordinal,
            saved_id,
            thing_id_utf16: thing_id,
            class_code,
            cause,
        });
        return Ok((None, prefix_len));
    }
    let source = crate::inventory::decode_embedded_record(bytes, cx.source_ordinal)
        .map_err(|e| e.to_string())?;
    let tail_start = source.opaque_tail_span.offset;
    let (_, _, result) = load_record(registry, &source, owner.raw(), true, definitions, cx);
    let loaded = result?.ok_or("embedded record destroyed by the Load catch")?;
    let tail = loaded.consumed;
    let id = ActorId::from_raw(saved_id);
    if let Some(entity) = registry.value_mut(id).and_then(RestoredRecord::loaded_mut) {
        // The stored source keeps only this record's bytes.
        entity.source.opaque_tail.truncate(tail);
        entity.source.opaque_tail_span.length = tail;
        entity.source.source_span.length = tail_start + tail;
    }
    Ok((loaded.output, tail_start + tail))
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SkippedRecord {
    pub source_ordinal: usize,
    pub saved_id: u32,
    pub thing_id_utf16: Vec<u16>,
    pub class_code: u32,
    pub cause: SkipCause,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum SkipCause {
    /// 4FAF14: the saved ID's generation differs from the IDMGR table; no
    /// message is produced.
    GenerationMismatch,
    /// 427BD0 found no Thing definition (translations applied).
    UnknownThing,
    /// The definition's class differs and the record is not a class-2 Object
    /// with the caller's conversion context.
    ChangedClass { definition_class: u32 },
    /// The record's Load threw an exception 4FAE80 catches (FuncInfo AE6F98,
    /// try states 13..15 around virtual D8): TGC_LoadWarningException (any
    /// 41DFB0 throw: 858A00, 858AD0, 8589B0, Unknown IDS) logs
    /// `load_warning_thing_init_failed_for_reason|sds`; TGC_LoadCancelException
    /// (5A0910) logs nothing. The object is destroyed (4F5490) and yields null.
    LoadException { cancel: bool, message: String },
}
impl SkipCause {
    /// KKC_MapResource::Warning (5A1040) localization key, formatted with the
    /// saved IDS through 4B9F40.
    pub fn native_warning_key(&self) -> Option<&'static str> {
        match self {
            Self::GenerationMismatch => None,
            Self::UnknownThing => Some("load_warning_unknown_thing_ids|s"),
            Self::ChangedClass { .. } => Some("load_warning_changed_gclass|s"),
            Self::LoadException { cancel: false, .. } => {
                Some("load_warning_thing_init_failed_for_reason|sds")
            }
            Self::LoadException { cancel: true, .. } => None,
        }
    }
}
/// Marker of an error that is a native TGC_LoadWarningException (thrown
/// through 41DFB0 during an object's Load).
pub const LOAD_WARNING_EXCEPTION: &str = "TGC_LoadWarningException: ";
pub fn load_warning_exception(message: impl std::fmt::Display) -> String {
    format!("{LOAD_WARNING_EXCEPTION}{message}")
}
/// 4FAE80's catch handlers: Some(cancel) for an exception it catches.
/// 5A0910 (KKC_MapResource::ThrowCancelException, "Map Load Canceled")
/// throws TGC_LoadCancelException; 41DFB0 throws TGC_LoadWarningException.
fn caught_load_exception(error: &str) -> Option<bool> {
    if error.contains("Map Load Canceled:") {
        Some(true)
    } else if error.contains(LOAD_WARNING_EXCEPTION) {
        Some(false)
    } else {
        None
    }
}
/// 4FAE80's first reads: 41D290 IDS, class word, saved ID word.
fn record_prefix(raw: &[u8]) -> Option<(Vec<u16>, u32, u32)> {
    let count = usize::from(u16::from_le_bytes(raw.get(0..2)?.try_into().ok()?));
    let end = 2 + 2 * count;
    let units = raw
        .get(2..end)?
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let class = u32::from_le_bytes(raw.get(end..end + 4)?.try_into().ok()?);
    let id = u32::from_le_bytes(raw.get(end + 4..end + 8)?.try_into().ok()?);
    Some((units, class, id))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestoreMode {
    /// Runtime path: no later source object is touched after any load failure.
    StopOnError,
    /// Diagnostic inventory only; the returned world is never publishable.
    InspectPartial,
}

/// Original4EC7F5..4EC80B stores the averaged definition scale to binary32;
/// 59E300 then multiplies the saved scale word and stores binary32 again.
pub fn restored_scale_bits(
    minimum: u32,
    maximum: u32,
    saved_multiplier: u32,
    pc: Precision,
) -> aa_numeric::Result<u32> {
    let mean = Binary::from_f32_bits(maximum)?
        .add(&Binary::from_f32_bits(minimum)?, pc)?
        .mul(&Binary::from_f32_bits(0x3f000000)?, pc)?
        .store_f32()?;
    Binary::from_f32_bits(saved_multiplier)?
        .mul(&Binary::from_f32_bits(mean)?, pc)?
        .store_f32()
}
/// Original GObject::Load state after decoded Thing and before Actor loading.
/// Editor-only legacy-handle publication and nonzero nested state are separate.
pub fn restore_object_state(
    state: &mut aa_actors::loading::ObjectShell,
    input: &crate::inventory::ObjectState,
    definition: &ObjectDefinition,
    rng: &mut NativeRng,
    pc: Precision,
) -> std::result::Result<(), String> {
    state.scale_bits = restored_scale_bits(
        definition.scale_min_bits,
        definition.scale_max_bits,
        input.scale_multiplier_bits,
        pc,
    )
    .map_err(|e| e.to_string())?;
    if let Some(variant) = input.variant_index {
        let count = i32::try_from(definition.variation_weights_bits.len())
            .map_err(|_| "native aspect count exceeds i32")?;
        state.variant = if variant >= count { -1 } else { variant };
    }
    if state.variant == -1 && definition.variations_enabled {
        state.variant = select_variant(definition, rng, pc).map_err(|e| e.to_string())?;
    }
    // Version>=2 stores +50, not the constructor's +38 associated field.
    if let Some(word) = input.associated_object_id {
        state.words_50_to_5c[0] = word;
    }
    state.words_50_to_5c[1] = 0; //4DD140: reset non-editor+54 handle.
    if input.nested_count.unwrap_or(0) != 0 {
        return Err("unported nested Object payload".into());
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
fn apply_loaded(
    entity: &mut LoadedEntity,
    registry: &mut IdentityRegistry<RestoredRecord>,
    definitions: &impl RestoreDefinitions,
    cx: &mut LoadContext<'_>,
    registered_owner: Option<ActorId>,
    owner_138: u32,
    nested: bool,
    owner_effects: &mut Vec<crate::restore_components::OwnerEffect>,
) -> std::result::Result<usize, String> {
    let inventory = cx.inventory;
    let pc = cx.pc;
    if entity.source.class_code == 9 {
        // The original derived ctor runs after base saved-ID registration,
        // before the Object Load; it only initializes the two empty slots.
        entity.doodad = Some(crate::dynamic_doodad::DynamicDoodad::default());
    }
    entity.special_object = match entity.source.class_code {
        6 => Some(crate::map_objects::SpecialObject::Area(
            crate::map_objects::Area::default(),
        )),
        7 => Some(crate::map_objects::SpecialObject::Camera(
            crate::map_objects::Camera::default(),
        )),
        _ => None,
    };
    let input = &entity.source.thing;
    let thing = &mut entity.shell.thing;
    apply_thing(thing, input, entity.definition.flags);
    entity.completion = LoadCompletion::ThingLoaded;
    if let Some(input) = &entity.source.object {
        let definition = entity
            .definition
            .object
            .as_ref()
            .ok_or("missing resolved Object definition")?;
        let state = entity
            .shell
            .object
            .as_mut()
            .ok_or("missing Object load shell")?;
        restore_object_state(state, input, definition, cx.rng, pc)?;
        entity.completion = LoadCompletion::ObjectLoaded;
    }
    if let Some(input) = &entity.source.actor {
        let actor = entity
            .shell
            .actor
            .as_mut()
            .ok_or("missing Actor load shell")?;
        let owner = if entity.shell.thing.flags & (1 << 23) != 0 {
            registered_owner
        } else {
            None
        };
        actor.words.insert(0xe8, owner.map_or(0, ActorId::raw));
        // 4FB2DF: an IsGActor (vf B4) result gets +138 = 4FAE80's owner
        // argument (0 for DYNAMIC_THINGS, the organization owner when embedded).
        actor.words.insert(0x138, owner_138);
        *cx.world_version = input.upgraded_world_version(*cx.world_version);
        let registered_owner_for_effects = owner;
        if entity.shell.thing.flags & (1 << 23) != 0 && input.owner_handle != 0 && owner.is_none() {
            return Err(format!(
                "Actor owner {:#010x} is not currently registered (native4D8DAA load error)",
                input.owner_handle
            ));
        }
        // Source slot is diagnostic metadata only; the owner stored above came
        // from the actual current generation-checked registry.
        entity.owner_source_slot = owner.and_then(|id| {
            inventory.kingdoms.as_ref().and_then(|kingdoms| {
                kingdoms.slots.iter().position(|slot| {
                    slot.record
                        .as_ref()
                        .is_some_and(|r| r.serialized_id == id.raw())
                })
            })
        });
        let state =
            input.state_after_read([actor.components.state.low, actor.components.state.high]);
        actor.components.state.low = state[0];
        actor.components.state.high = state[1];
        let cleared = aa_actors::state::LOAD_CLEARED_MIND_MASK;
        actor.components.state.low &= !(cleared as u32);
        actor.components.state.high &= !((cleared >> 32) as u32);
        entity.completion = LoadCompletion::ActorPrefixLoaded;
        let definition_mask = entity
            .definition
            .component_mask
            .ok_or("missing resolved Actor component mask")?;
        let object_byte_4c = entity.shell.object.as_ref().map_or(0, |o| o.byte_4c);
        let entity_variant = entity.shell.object.as_ref().map(|o| o.variant);
        let definition_id = entity.definition.proof.id.clone();
        let thing_word_c = entity.shell.thing.word_c;
        let (tail, consumed) = crate::actor_payload::ActorTail::decode_prefix_with_stages(
            &entity.source.opaque_tail,
            input.version,
            0,
            |slot, saved, defaults, offset, bytes| {
                let slot_id = aa_actors::components::ComponentSlot::from_index(slot as usize)
                    .ok_or_else(|| {
                        crate::Error::Format("component slot outside31-entry table".into())
                    })?;
                let action = crate::restore_components::component_load_action(
                    slot_id,
                    saved,
                    defaults,
                    definition_mask,
                );
                let character_ground = if slot == 2
                    && matches!(
                        action,
                        crate::restore_components::ComponentLoadAction::Default
                            | crate::restore_components::ComponentLoadAction::Saved { .. }
                    ) {
                    let variant =
                        entity
                            .shell
                            .object
                            .as_ref()
                            .map(|o| o.variant)
                            .ok_or_else(|| {
                                crate::Error::Format(
                                    "Character needs the loaded Object variant".into(),
                                )
                            })?;
                    Some(
                        definitions
                            .character_ground(&entity.definition.proof.id, variant, pc)
                            .map_err(crate::Error::Format)?,
                    )
                } else {
                    None
                };
                let ground = character_ground
                    .as_ref()
                    .unwrap_or(&entity.definition.ground);
                if !saved {
                    crate::restore_components::apply_component_stage_with_services(
                        &mut actor.components,
                        crate::restore_components::ComponentOwner {
                            owner: registered_owner_for_effects,
                            owner_effects: &mut *owner_effects,
                            register_updatable: &|id, target| {
                                definitions.register_updatable(id, target)
                            },
                            updatable_registrations: &mut *cx.updatable_registrations,
                            flags: &mut entity.shell.thing.flags,
                            world_time_bits: inventory.world.begin_time_bits,
                            position_bits: entity.shell.thing.position_bits,
                            id: ActorId::from_raw(entity.source.serialized_id),
                        },
                        slot_id,
                        action,
                        None,
                        ground,
                        pc,
                        Some(crate::restore_components::PerformerServices {
                            construct: &mut |components, flags| {
                                definitions
                                    .construct_performer(PerformerOwner {
                                        id: ActorId::from_raw(entity.source.serialized_id),
                                        definition_id: &entity.definition.proof.id,
                                        components,
                                        flags,
                                        actor_words: &mut actor.words,
                                        object_byte_4c,
                                        thing_word_c,
                                        precision: pc,
                                    })
                                    .map_err(crate::Error::Format)
                            },
                            apply_ability: &|handle, saved| {
                                definitions.apply_performer_ability(handle, saved)
                            },
                            construct_defender: &mut || {
                                definitions
                                    .construct_defender(DefenderOwner {
                                        id: ActorId::from_raw(entity.source.serialized_id),
                                        definition_id: &entity.definition.proof.id,
                                        precision: pc,
                                    })
                                    .map_err(crate::Error::Format)
                            },
                            ability_word14: &|handle| definitions.ability_word14(handle),
                            construct_mind: &mut |components| {
                                definitions
                                    .construct_mind(MindOwner {
                                        id: ActorId::from_raw(entity.source.serialized_id),
                                        definition_mask,
                                        components,
                                        rng: &mut *cx.rng,
                                        precision: pc,
                                    })
                                    .map_err(crate::Error::Format)
                            },
                            load_mind: &|mind, bytes, context| {
                                definitions.load_saved_mind_with_context(mind, bytes, context)
                            },
                            character_animation: &|kind, index| {
                                let variant = entity_variant
                                    .ok_or("Character needs the loaded Object variant")?;
                                definitions.character_animation(
                                    &definition_id,
                                    variant,
                                    kind,
                                    index,
                                    pc,
                                )
                            },
                            load_transport: &|state, bytes| {
                                state
                                    .load(
                                        bytes,
                                        &mut OrganizationDefinitions {
                                            definitions,
                                            precision: pc,
                                        },
                                    )
                                    .map_err(|e| e.to_string())
                            },
                        }),
                    )?;
                    if slot == 4 {
                        crate::actor_finish::validate_supported_requirements(actor)
                            .map_err(crate::Error::Format)?;
                    }
                    return Ok((0, None));
                }
                if slot == 7 {
                    if action == crate::restore_components::ComponentLoadAction::DefinitionMismatch
                    {
                        return Err(crate::Error::Format(load_warning_exception(
                            "File Warning: Component 7 removed from actor",
                        )));
                    }
                    // 579BD0 construction exactly as an ordinary stage, then
                    // 57D130 directly on the inserted component.
                    crate::restore_components::apply_component_stage(
                        &mut actor.components,
                        crate::restore_components::ComponentOwner {
                            owner: registered_owner_for_effects,
                            owner_effects: &mut *owner_effects,
                            register_updatable: &|id, target| {
                                definitions.register_updatable(id, target)
                            },
                            updatable_registrations: &mut *cx.updatable_registrations,
                            flags: &mut entity.shell.thing.flags,
                            world_time_bits: inventory.world.begin_time_bits,
                            position_bits: entity.shell.thing.position_bits,
                            id: ActorId::from_raw(entity.source.serialized_id),
                        },
                        slot_id,
                        crate::restore_components::ComponentLoadAction::Default,
                        None,
                        ground,
                        pc,
                    )?;
                    let consumed = crate::organization_restore::load_organization(
                        actor,
                        ActorId::from_raw(entity.source.serialized_id),
                        &entity.definition,
                        &mut entity.shell.thing,
                        entity
                            .shell
                            .object
                            .as_ref()
                            .ok_or_else(|| {
                                crate::Error::Format(
                                    "Organization owner has no Object shell".into(),
                                )
                            })?
                            .byte_4c,
                        registry,
                        definitions,
                        cx,
                        bytes,
                    )
                    .map_err(crate::Error::Format)?;
                    entity.saved_components.push(DecodedSavedComponent {
                        slot,
                        source_span: crate::inventory::ByteSpan {
                            offset: entity.source.opaque_tail_span.offset + offset,
                            length: consumed,
                        },
                        payload: crate::saved_components::SavedComponent::Organization(
                            crate::saved_components::OrganizationPayload {
                                bytes: bytes[..consumed].to_vec(),
                            },
                        ),
                    });
                    return Ok((consumed, None));
                }
                let (payload, consumed) = if slot == 24 {
                    // 58C810 has no length prefix: run it on a constructor
                    // copy (58B9A0) to find the consumed bytes; the Load is a
                    // pure stream/registry read. The real component is
                    // constructed and loaded at its stage below.
                    let count =
                        entity
                            .definition
                            .ground
                            .resource_domain_count
                            .ok_or_else(|| {
                                crate::Error::Format(
                                    "Transport requires the Resource domain count".into(),
                                )
                            })?;
                    let mut scratch = aa_actors::transport_state::TransportState::<String>::new(
                        ActorId::from_raw(entity.source.serialized_id),
                        count,
                    );
                    let consumed = scratch
                        .load(
                            bytes,
                            &mut OrganizationDefinitions {
                                definitions,
                                precision: pc,
                            },
                        )
                        .map_err(|e| crate::Error::Format(e.to_string()))?;
                    (
                        crate::saved_components::SavedComponent::Transport(
                            crate::saved_components::TransportPayload {
                                bytes: bytes[..consumed].to_vec(),
                            },
                        ),
                        consumed,
                    )
                } else if slot == 4 {
                    let (updatable, consumed) = definitions
                        .decode_saved_mind(definition_mask, bytes)
                        .map_err(crate::Error::Format)?;
                    (
                        crate::saved_components::SavedComponent::Mind(
                            crate::saved_components::MindPayload {
                                updatable,
                                bytes: bytes[..consumed].to_vec(),
                            },
                        ),
                        consumed,
                    )
                } else {
                    crate::saved_components::SavedComponent::decode_prefix(slot, bytes)?
                };
                let properties =
                    if let crate::saved_components::SavedComponent::Properties(value) = &payload {
                        Some(value.clone())
                    } else {
                        None
                    };
                entity.saved_components.push(DecodedSavedComponent {
                    slot,
                    source_span: crate::inventory::ByteSpan {
                        offset: entity.source.opaque_tail_span.offset + offset,
                        length: consumed,
                    },
                    payload: payload.clone(),
                });
                crate::restore_components::apply_component_stage_with_services(
                    &mut actor.components,
                    crate::restore_components::ComponentOwner {
                        owner: registered_owner_for_effects,
                        owner_effects: &mut *owner_effects,
                        register_updatable: &|id, target| {
                            definitions.register_updatable(id, target)
                        },
                        updatable_registrations: &mut *cx.updatable_registrations,
                        flags: &mut entity.shell.thing.flags,
                        world_time_bits: inventory.world.begin_time_bits,
                        position_bits: entity.shell.thing.position_bits,
                        id: ActorId::from_raw(entity.source.serialized_id),
                    },
                    slot_id,
                    action,
                    Some(&payload),
                    ground,
                    pc,
                    Some(crate::restore_components::PerformerServices {
                        construct: &mut |components, flags| {
                            definitions
                                .construct_performer(PerformerOwner {
                                    id: ActorId::from_raw(entity.source.serialized_id),
                                    definition_id: &entity.definition.proof.id,
                                    components,
                                    flags,
                                    actor_words: &mut actor.words,
                                    object_byte_4c,
                                    thing_word_c,
                                    precision: pc,
                                })
                                .map_err(crate::Error::Format)
                        },
                        apply_ability: &|handle, saved| {
                            definitions.apply_performer_ability(handle, saved)
                        },
                        construct_defender: &mut || {
                            definitions
                                .construct_defender(DefenderOwner {
                                    id: ActorId::from_raw(entity.source.serialized_id),
                                    definition_id: &entity.definition.proof.id,
                                    precision: pc,
                                })
                                .map_err(crate::Error::Format)
                        },
                        ability_word14: &|handle| definitions.ability_word14(handle),
                        construct_mind: &mut |components| {
                            definitions
                                .construct_mind(MindOwner {
                                    id: ActorId::from_raw(entity.source.serialized_id),
                                    definition_mask,
                                    components,
                                    rng: &mut *cx.rng,
                                    precision: pc,
                                })
                                .map_err(crate::Error::Format)
                        },
                        load_mind: &|mind, bytes, context| {
                            definitions.load_saved_mind_with_context(mind, bytes, context)
                        },
                        character_animation: &|kind, index| {
                            let variant = entity_variant
                                .ok_or("Character needs the loaded Object variant")?;
                            definitions.character_animation(
                                &definition_id,
                                variant,
                                kind,
                                index,
                                pc,
                            )
                        },
                        load_transport: &|state, bytes| {
                            state
                                .load(
                                    bytes,
                                    &mut OrganizationDefinitions {
                                        definitions,
                                        precision: pc,
                                    },
                                )
                                .map_err(|e| e.to_string())
                        },
                    }),
                )?;
                if slot == 4 {
                    crate::actor_finish::validate_supported_requirements(actor)
                        .map_err(crate::Error::Format)?;
                }
                Ok((consumed, properties))
            },
        )
        .map_err(|error| error.to_string())?;
        entity.actor_tail = Some(tail);
        if !nested && consumed != entity.source.opaque_tail.len() {
            return Err(format!(
                "unported Actor subtype bytes after offset {consumed}"
            ));
        }
        entity.completion = LoadCompletion::ComponentsLoaded;
        entity.active_component = crate::actor_finish::finish_unlinked_actor(
            &mut entity.shell,
            entity.actor_tail.as_ref().ok_or("Actor tail disappeared")?,
            &entity.source.thing_id_utf16,
            &entity.definition.proof.id,
            definitions,
            pc,
            &mut crate::actor_finish::NameTail {
                id: ActorId::from_raw(entity.source.serialized_id),
                actor_version: input.version,
                registry: &mut *registry,
                messages: &mut *cx.native_messages,
                source_ordinal: cx.source_ordinal,
                world_time_bits: inventory.world.begin_time_bits,
            },
        )?;
        entity.completion = LoadCompletion::AwaitingPostLoad;
        return Ok(consumed);
    }
    if let Some(special) = &mut entity.special_object {
        let consumed = match special {
            crate::map_objects::SpecialObject::Area(area) => area.load(
                &entity.source.opaque_tail,
                *cx.world_version,
                entity.source.serialized_id,
                definitions,
                cx.updatable_registrations,
            ),
            crate::map_objects::SpecialObject::Camera(camera) => {
                camera.load(&entity.source.opaque_tail, definitions)
            }
        }
        .map_err(|e| e.to_string())?;
        if !nested && consumed != entity.source.opaque_tail.len() {
            return Err(format!(
                "unported Area/Camera subtype bytes after offset{consumed}"
            ));
        }
        entity.completion = LoadCompletion::AwaitingPostLoad;
        return Ok(consumed);
    } else if let Some(doodad) = &mut entity.doodad {
        let definition =
            definitions.dynamic_doodad_definition(&entity.source.thing_id_utf16, pc)?;
        let consumed = doodad
            .load(
                &entity.source.opaque_tail,
                &definition,
                entity.source.serialized_id,
                inventory.world.word_48,
                |id, target| definitions.register_updatable(id, target),
                cx.updatable_registrations,
            )
            .map_err(|e| e.to_string())?;
        if !nested && consumed != entity.source.opaque_tail.len() {
            return Err(format!(
                "unported DynamicDoodad subtype bytes after offset{consumed}"
            ));
        }
        entity.completion = LoadCompletion::AwaitingPostLoad;
        return Ok(consumed);
    } else if !nested && !entity.source.opaque_tail.is_empty() {
        return Err(format!(
            "unported subtype tail: {} bytes",
            entity.source.opaque_tail.len()
        ));
    }
    entity.completion = LoadCompletion::AwaitingPostLoad;
    Ok(0)
}
fn apply_owner_effects(
    registry: &mut IdentityRegistry<RestoredRecord>,
    effects: &[crate::restore_components::OwnerEffect],
    presentation: &mut Vec<crate::restore_components::OwnerEffect>,
    messages: &mut Vec<NativeMessage>,
    source_ordinal: usize,
) -> std::result::Result<(), String> {
    for effect in effects {
        match effect {
            crate::restore_components::OwnerEffect::NativeMessage {
                function,
                line,
                text,
            } => messages.push(NativeMessage {
                source_ordinal,
                function: *function,
                line: *line,
                text: text.clone(),
            }),
            crate::restore_components::OwnerEffect::PresentationFx { .. } => {
                presentation.push(effect.clone());
            }
            &crate::restore_components::OwnerEffect::SupplySource { kingdom, actor } => {
                let Some(RestoredRecord::Kingdom(record)) = registry.value_mut(kingdom) else {
                    return Err(format!(
                        "Supply owner {:#010x} is not a restored Kingdom (4E1F00 is a GKingdom method)",
                        kingdom.raw()
                    ));
                };
                record
                    .runtime
                    .as_mut()
                    .and_then(|r| r.derived.as_mut())
                    .ok_or("Supply owner Kingdom has no derived state")?
                    .register_supply_source(actor);
            }
        }
    }
    Ok(())
}
fn apply_thing(
    thing: &mut aa_actors::loading::ThingShell<DefinitionProof>,
    input: &crate::ThingState,
    definition_flags: u32,
) {
    thing.flags = input.runtime_flags(thing.flags, definition_flags);
    thing.word_1c = Some(input.word_1c);
    thing.position_bits = input.position_bits;
    thing.facing_bits = input.facing_bits;
    thing.name_utf16 = if input.name_utf16.len() > 1000 {
        Vec::new()
    } else {
        input.name_utf16.clone()
    };
}
fn restore_world_entity(
    record: &mut RestoredRecord,
    definitions: &impl RestoreDefinitions,
    rng: &mut NativeRng,
    world_time_bits: u32,
    pc: Precision,
    updatable_registrations: &mut Vec<crate::restore_components::UpdatableRegistration>,
    saved_team: Option<ActorId>,
) -> std::result::Result<(), String> {
    match record {
        RestoredRecord::Team(entity) => {
            let source = &entity.source;
            let derived = crate::special_state::SpecialShell::team().map_err(|e| e.to_string())?;
            let runtime = entity.runtime.as_mut().ok_or("Team shell disappeared")?;
            runtime.derived = Some(derived);
            apply_thing(
                &mut runtime.base.thing,
                &source.thing,
                runtime.definition.flags,
            );
            entity.completion = LoadCompletion::ThingLoaded;
            runtime
                .derived
                .as_mut()
                .ok_or("Team derived shell disappeared")?
                .load_team_fields(source, |key| definitions.localize(key))
                .map_err(|e| e.to_string())?;
            runtime.derived_payload_loaded = true;
            entity.completion = LoadCompletion::AwaitingPostLoad;
            Ok(())
        }
        RestoredRecord::Kingdom(entity) => {
            let source = &entity.source;
            let id = ActorId::from_raw(source.serialized_id);
            let derived = crate::special_state::SpecialShell::kingdom(
                id,
                definitions.resource_domain_count(pc)?,
            )
            .map_err(|e| e.to_string())?;
            let runtime = entity.runtime.as_mut().ok_or("Kingdom shell disappeared")?;
            runtime.derived = Some(derived);
            apply_thing(
                &mut runtime.base.thing,
                &source.thing,
                runtime.definition.flags,
            );
            entity.completion = LoadCompletion::ThingLoaded;
            if let Some(input) = &source.object {
                restore_object_state(
                    runtime
                        .base
                        .object
                        .as_mut()
                        .ok_or("missing Kingdom Object shell")?,
                    input,
                    runtime
                        .definition
                        .object
                        .as_ref()
                        .ok_or("missing Kingdom Object definition")?,
                    rng,
                    pc,
                )?;
                entity.completion = LoadCompletion::ObjectLoaded;
            }
            let input = source.actor.as_ref().ok_or(
                "legacy Kingdom lacks Actor source layer; compatibility path remains unported",
            )?;
            let actor = runtime
                .base
                .actor
                .as_mut()
                .ok_or("missing Kingdom Actor shell")?;
            let state = input
                .prefix
                .state_after_read([actor.components.state.low, actor.components.state.high]);
            actor.components.state.low =
                state[0] & !(aa_actors::state::LOAD_CLEARED_MIND_MASK as u32);
            actor.components.state.high =
                state[1] & !((aa_actors::state::LOAD_CLEARED_MIND_MASK >> 32) as u32);
            if runtime.base.thing.flags & (1 << 23) != 0 && input.prefix.owner_handle != 0 {
                return Err(
                    "Kingdom Actor prefix requires unresolved non-null owner binding".into(),
                );
            }
            actor.words.insert(0xe8, 0);
            entity.completion = LoadCompletion::ActorPrefixLoaded;
            let definition_mask = runtime
                .definition
                .component_mask
                .ok_or("missing Kingdom Actor component mask")?;
            let tail = &input.tail;
            let mut kingdom_effects = Vec::new();
            for slot in std::iter::once(15)
                .chain((0..31).filter(|slot| *slot != 15 && *slot != 4))
                .chain(std::iter::once(4))
            {
                let slot_id = aa_actors::components::ComponentSlot::from_index(slot)
                    .ok_or("invalid Kingdom component slot")?;
                let saved = tail.saved_components_mask & (1 << slot) != 0;
                let action = crate::restore_components::component_load_action(
                    slot_id,
                    saved,
                    tail.default_components_mask,
                    definition_mask,
                );
                let payload = if saved {
                    if slot != 6 {
                        return Err(format!(
                            "Kingdom source saved unexpected component slot{slot}"
                        ));
                    }
                    Some(crate::saved_components::SavedComponent::Properties(
                        tail.properties
                            .clone()
                            .ok_or("Kingdom saved Properties payload missing")?,
                    ))
                } else {
                    None
                };
                crate::restore_components::apply_component_stage(
                    &mut actor.components,
                    crate::restore_components::ComponentOwner {
                        owner: None,
                        owner_effects: &mut kingdom_effects,
                        register_updatable: &|id, target| {
                            definitions.register_updatable(id, target)
                        },
                        updatable_registrations,
                        flags: &mut runtime.base.thing.flags,
                        id,
                        world_time_bits,
                        position_bits: runtime.base.thing.position_bits,
                    },
                    slot_id,
                    action,
                    payload.as_ref(),
                    &runtime.definition.ground,
                    pc,
                )
                .map_err(|e| e.to_string())?;
            }
            if !kingdom_effects.is_empty() {
                return Err(
                    "Kingdom component constructors produced unported owner effects".into(),
                );
            }
            runtime.actor_tail = Some(tail.clone());
            entity.completion = LoadCompletion::ComponentsLoaded;
            finish_kingdom_actor(&mut runtime.base, tail, definitions)?;
            if let Some(id) = source
                .updatable
                .as_ref()
                .and_then(|v| v.serialized_id)
                .filter(|id| *id >= 0)
            {
                let target = crate::restore_components::UpdatableTarget::Kingdom {
                    owner: source.serialized_id,
                };
                definitions.register_updatable(id, &target)?;
                updatable_registrations.push(crate::restore_components::UpdatableRegistration {
                    serialized_id: id,
                    target,
                });
            }
            runtime
                .derived
                .as_mut()
                .ok_or("Kingdom derived shell disappeared")?
                .load_kingdom_prefix(&source.payload, definitions, pc)
                .map_err(|e| e.to_string())?;
            runtime
                .derived
                .as_mut()
                .ok_or("Kingdom derived shell disappeared")?
                .load_kingdom_before_commander(&source.payload, saved_team)
                .map_err(|e| e.to_string())?;
            runtime
                .derived
                .as_mut()
                .ok_or("Kingdom derived shell disappeared")?
                .load_commander(&source.payload, source.serialized_id, definitions, pc)
                .map_err(|e| e.to_string())?;
            runtime
                .derived
                .as_mut()
                .ok_or("Kingdom derived shell disappeared")?
                .load_kingdom_before_ai(&source.payload, definitions, pc)
                .map_err(|e| e.to_string())?;
            runtime
                .derived
                .as_mut()
                .ok_or("Kingdom derived shell disappeared")?
                .load_kingdom_ai(&source.payload)
                .map_err(|e| e.to_string())?;
            runtime
                .derived
                .as_mut()
                .ok_or("Kingdom derived shell disappeared")?
                .load_kingdom_definition_counts(&source.payload, definitions, pc)
                .map_err(|e| e.to_string())?;
            runtime
                .derived
                .as_mut()
                .ok_or("Kingdom derived shell disappeared")?
                .load_kingdom_final_fields(&source.payload, definitions, pc)
                .map_err(|e| e.to_string())?;
            runtime.derived_payload_loaded = true;
            entity.completion = LoadCompletion::AwaitingPostLoad;
            Ok(())
        }
        _ => Err("world entity restoration received a dynamic Thing".into()),
    }
}
/// Source Kingdoms have the proven empty-name/no-owner Properties-only Actor
/// tail path. Broader actors keep explicit callbacks rather than skipping them.
pub fn finish_kingdom_actor<D>(
    base: &mut LoadShell<D, RestoredComponent>,
    tail: &crate::actor_payload::ActorTail,
    definitions: &impl RestoreDefinitions,
) -> std::result::Result<(), String> {
    let actor = base
        .actor
        .as_mut()
        .ok_or("Kingdom Actor shell is missing")?;
    for (slot, _) in actor.components.iter() {
        if slot != aa_actors::components::ComponentSlot::Properties {
            return Err(
                "Kingdom requirement validation requires additional component virtuals".into(),
            );
        }
    }
    if tail.current_component != -1 {
        return Err("Kingdom current component selection requires explicit pointer binding".into());
    }
    if let Some(word) = tail.word_158 {
        actor.words.insert(0x158, word);
    }
    if tail.legacy_word_15c.is_some() {
        actor.words.insert(0x15c, u32::MAX);
    }
    if tail
        .word_160_source
        .as_ref()
        .is_some_and(|v| v.presence != 0)
    {
        return Err("Kingdom Actor tail owner-property lookup is unresolved".into());
    }
    if let Some(word) = tail.word_164.as_ref().and_then(|v| v.value) {
        actor.words.insert(0x164, word);
    }
    if let Some(key) = &tail.localized_name_key_utf16 {
        actor.string_dc_utf16 = key.clone();
        actor.string_e0_utf16 = definitions.localize(key)?;
    }
    for (offset, value) in [
        (0x168, tail.word_168),
        (0x16c, tail.word_16c),
        (0x170, tail.word_170),
    ] {
        if let Some(v) = value {
            actor.words.insert(offset, v);
        }
    }
    if tail.lists.as_ref().is_some_and(|l| {
        !l.list_118.values.is_empty()
            || !l.list_120.values.is_empty()
            || !l.list_128.values.is_empty()
            || !l.list_130.values.is_empty()
    }) {
        return Err("nonempty Kingdom Actor linked lists require owned runtime restoration".into());
    }
    if let Some(word) = tail.word_174 {
        actor.words.insert(0x174, word);
    }
    //4D67F0 returns immediately for actual bit29; no spatial query is required.
    if base.thing.flags & (1 << 29) == 0 {
        return Err("Kingdom Actor visibility refresh requires native spatial callbacks".into());
    }
    if actor.words.get(&0x158).copied().unwrap_or(0) != 0 {
        return Err("Kingdom Actor158 saved identity validation requires registry lookup".into());
    }
    if !actor.string_dc_utf16.is_empty() {
        return Err("Kingdom nonempty Actor name requires action-table binding".into());
    }
    //GKingdom virtual108=4D1E80 returns owner+E8. Null owner skips the
    //numbered-name branch independently of its authored definition flag.
    if actor.words.get(&0xe8).copied().unwrap_or(0) != 0 {
        return Err("Kingdom automatic numbering requires actual owner state".into());
    }
    Ok(())
}
fn prepare_world_entity(
    id_utf16: &[u16],
    is_actor: bool,
    definitions: &impl RestoreDefinitions,
    pc: Precision,
) -> std::result::Result<WorldEntityRuntime, String> {
    let definition = definitions.resolve(id_utf16, pc)?;
    let expected = if is_actor { 4 } else { 5 };
    if definition.native_class != expected {
        return Err(format!(
            "source world entity class {expected} differs from resolved definition class {}",
            definition.native_class
        ));
    }
    let base = construct_shell_with_components(
        if is_actor {
            LoadKind::Actor
        } else {
            LoadKind::Thing
        },
        definition.proof.clone(),
    );
    Ok(WorldEntityRuntime {
        definition,
        base,
        derived: None,
        actor_tail: None,
        derived_payload_loaded: false,
    })
}
fn empty_required_objects(inventory: &WorldInventory) -> std::result::Result<(), String> {
    let data = inventory
        .opaque_initialization
        .iter()
        .find(|v| v.name == "USED_REQUIRED_OBJECTS");
    let index = inventory
        .opaque_initialization
        .iter()
        .find(|v| v.name == "USED_REQUIRED_OBJECTS_INDEX");
    match (data, index) {
        (None, None) => Ok(()),
        (Some(data), Some(index))
            if index.bytes.as_slice() == [0u8; 8]
                && data.bytes.len() == 8
                && data.bytes[..4] == [0u8; 4] =>
        {
            Ok(())
        }
        (Some(_), Some(_)) => Err(
            "nonempty or unsupported required-object stage must load before dynamic Things".into(),
        ),
        _ => Err("required-object data/index pair is incomplete".into()),
    }
}
/// Caller supplies the empty registry restored from this map's IDMGR. Team and
/// Kingdom identities are registered in source order, after definition/base
/// preparation and before their derived construction and saved Load work.
/// Unsupported indexed dynamic records do not shift following source ordinals.
pub fn restore_records(
    registry: &mut IdentityRegistry<RestoredRecord>,
    inventory: &WorldInventory,
    definitions: &impl RestoreDefinitions,
    rng: &mut NativeRng,
    pc: Precision,
) -> RestoreReport {
    restore_records_with_mode(
        registry,
        inventory,
        definitions,
        rng,
        pc,
        RestoreMode::StopOnError,
    )
}
pub fn restore_records_with_mode(
    registry: &mut IdentityRegistry<RestoredRecord>,
    inventory: &WorldInventory,
    definitions: &impl RestoreDefinitions,
    rng: &mut NativeRng,
    pc: Precision,
    mode: RestoreMode,
) -> RestoreReport {
    let mut report = RestoreReport {
        world_version: inventory.world.version,
        ..RestoreReport::default()
    };
    macro_rules! source_stage {
        ($source:expr,$stage:expr,$variant:ident,$actor:expr) => {
            if let Some(source) = $source {
                for (slot, value) in source.slots.iter().enumerate() {
                    if let Some(source) = &value.record {
                        let id = source.serialized_id;
                        let runtime = match prepare_world_entity(
                            &source.definition_id_utf16,
                            $actor,
                            definitions,
                            pc,
                        ) {
                            Ok(runtime) => runtime,
                            Err(error) => {
                                report.events.push(RestoreEvent {
                                    stage: $stage,
                                    source_ordinal: slot,
                                    saved_id: Some(id),
                                    registered: false,
                                    completion: None,
                                    error: Some(error),
                                });
                                report.stopped_before = Some($stage);
                                return report;
                            }
                        };
                        let result = registry.register_saved(
                            ActorId::from_raw(id),
                            $actor,
                            RestoredRecord::$variant(Box::new(SourceWorldEntity {
                                source: source.clone(),
                                completion: LoadCompletion::RegisteredLoadShell,
                                runtime: Some(runtime),
                                failure: None,
                            })),
                        );
                        let mut error = result.err().map(|e| e.to_string());
                        let registered = error.is_none();
                        let mut completion =
                            registered.then_some(LoadCompletion::RegisteredLoadShell);
                        if registered {
                            // Read from the authoritative identity registry before
                            // taking its current record mutably. Prior stages are
                            // committed; no saved handle is replaced by slot/name.
                            let saved_team = registry
                                .get(ActorId::from_raw(id))
                                .and_then(|entry| match &entry.value {
                                    RestoredRecord::Kingdom(e) => e.source.payload.team_handle,
                                    _ => None,
                                })
                                .and_then(|raw| {
                                    registry.get(ActorId::from_raw(raw)).map(|entry| entry.id)
                                });
                            if let Some(registry_slot) = registry.value_mut(ActorId::from_raw(id)) {
                                let mut record =
                                    std::mem::replace(registry_slot, RestoredRecord::Loading);
                                error = restore_world_entity(
                                    &mut record,
                                    definitions,
                                    rng,
                                    inventory.world.begin_time_bits,
                                    pc,
                                    &mut report.updatable_registrations,
                                    saved_team,
                                )
                                .err();
                                completion = Some(record.completion());
                                match &mut record {
                                    RestoredRecord::Team(e) => e.failure = error.clone(),
                                    RestoredRecord::Kingdom(e) => e.failure = error.clone(),
                                    _ => {}
                                }
                                if let Some(load_error) = &error {
                                    report.lifecycle.push(crate::restore_lifecycle::load_failed(
                                        $stage,
                                        slot,
                                        ActorId::from_raw(id),
                                        false,
                                        definitions.record_lifecycle_mode(),
                                        load_error,
                                    ));
                                    if let Err(ownership) = crate::restore_lifecycle::return_held(
                                        ActorId::from_raw(id),
                                        record,
                                        registry,
                                        &mut report.lifecycle_detached,
                                    ) {
                                        error = Some(format!("{load_error}; {ownership}"));
                                    }
                                } else {
                                    error = crate::restore_lifecycle::finish(
                                        ActorId::from_raw(id),
                                        record,
                                        definitions,
                                        crate::restore_lifecycle::Inputs {
                                            registry,
                                            inventory,
                                            rng,
                                            native_messages: &mut report.native_messages,
                                            presentation_effects: &mut report.presentation_effects,
                                            events: &mut report.lifecycle,
                                            detached: &mut report.lifecycle_detached,
                                            stage: $stage,
                                            source_ordinal: slot,
                                            nested: false,
                                            precision: pc,
                                            world_version: report.world_version,
                                        },
                                    )
                                    .err();
                                    if let Some(record) = registry.value_mut(ActorId::from_raw(id))
                                    {
                                        match record {
                                            RestoredRecord::Team(e) => e.failure = error.clone(),
                                            RestoredRecord::Kingdom(e) => e.failure = error.clone(),
                                            _ => {}
                                        }
                                    }
                                }
                            } else {
                                error = Some(
                                    "registered source world entity disappeared before Load".into(),
                                );
                            }
                        }
                        let failed = error.is_some();
                        report.events.push(RestoreEvent {
                            stage: $stage,
                            source_ordinal: slot,
                            saved_id: Some(id),
                            registered,
                            completion,
                            error,
                        });
                        if !registered || (failed && mode == RestoreMode::StopOnError) {
                            report.stopped_before = Some($stage);
                            return report;
                        }
                    }
                }
            } else {
                report.stopped_before = Some($stage);
                return report;
            }
        };
    }
    source_stage!(inventory.teams.as_ref(), RestoreStage::Teams, Team, false);
    source_stage!(
        inventory.kingdoms.as_ref(),
        RestoreStage::Kingdoms,
        Kingdom,
        true
    );
    if let Err(error) = empty_required_objects(inventory) {
        report.events.push(RestoreEvent {
            stage: RestoreStage::RequiredObjects,
            source_ordinal: 0,
            saved_id: None,
            registered: false,
            completion: None,
            error: Some(error),
        });
        report.stopped_before = Some(RestoreStage::DynamicThings);
        return report;
    }
    restore_dynamic(registry, inventory, definitions, rng, pc, &mut report, mode);
    report
}
fn restore_dynamic(
    registry: &mut IdentityRegistry<RestoredRecord>,
    inventory: &WorldInventory,
    definitions: &impl RestoreDefinitions,
    rng: &mut NativeRng,
    pc: Precision,
    report: &mut RestoreReport,
    mode: RestoreMode,
) {
    let things = &inventory.dynamic_things;
    // Records in native index order, whether or not their body is decodable.
    let mut order: Vec<(
        usize,
        Option<&crate::inventory::Placement>,
        Option<&crate::inventory::UnsupportedRecord>,
    )> = things
        .placements
        .iter()
        .map(|p| (p.record_ordinal, Some(p), None))
        .chain(
            things
                .unsupported
                .iter()
                .map(|u| (u.record_ordinal, None, Some(u))),
        )
        .collect();
    order.sort_by_key(|(ordinal, _, _)| *ordinal);
    let mut deferred_unsupported = Vec::new();
    let mut assert_latch = false;
    for (ordinal, placement, unsupported) in order {
        // 4FAE80 null results: generation check, then 427BD0, then class.
        let prefix = match (placement, unsupported) {
            (Some(p), _) => Some((p.thing_id_utf16.clone(), p.class_code, p.serialized_id)),
            (None, Some(u)) => record_prefix(&u.raw),
            (None, None) => None,
        };
        if let Some((thing_id, class_code, saved_id)) = prefix {
            let cause = null_record_cause(
                registry,
                definitions,
                &thing_id,
                class_code,
                saved_id,
                pc,
                true,
            )
            .transpose();
            match cause {
                Some(Ok(cause)) => {
                    report.skipped.push(SkippedRecord {
                        source_ordinal: ordinal,
                        saved_id,
                        thing_id_utf16: thing_id,
                        class_code,
                        cause,
                    });
                    continue;
                }
                Some(Err(error)) => {
                    report.events.push(RestoreEvent {
                        stage: RestoreStage::DynamicThings,
                        source_ordinal: ordinal,
                        saved_id: Some(saved_id),
                        registered: false,
                        completion: None,
                        error: Some(error),
                    });
                    if mode == RestoreMode::StopOnError {
                        report.stopped_before = Some(RestoreStage::DynamicThings);
                        return;
                    }
                    continue;
                }
                None => {}
            }
            // 4FAFA8: a saved class-2 record whose definition is now another
            // class is converted (4FC340 always passes the vector), yields
            // null, and is replaced after PostLoad. Its saved ID is not
            // registered; the loop continues at the next index entry.
            if let Ok(definition) = definitions.resolve(&thing_id, pc)
                && crate::class_conversion::check_class(class_code, definition.native_class, true)
                    == crate::class_conversion::ClassCheck::Convert
            {
                let class = definition.native_class;
                let outcome = match (placement, unsupported) {
                    (Some(p), _) => crate::class_conversion::on_placement_class_mismatch(
                        p,
                        definition,
                        class,
                        Some(&mut report.conversions),
                    ),
                    // Decoder-rejected record: the byte path over its bytes.
                    (None, Some(u)) => crate::class_conversion::on_class_mismatch(
                        &u.raw,
                        definition,
                        class,
                        Some(&mut report.conversions),
                    ),
                    (None, None) => unreachable!("one side of each record is present"),
                };
                match outcome {
                    Ok(crate::class_conversion::MismatchOutcome::Converted { index, .. }) => {
                        report.converted.push(ConvertedRecord {
                            source_ordinal: ordinal,
                            saved_id,
                            queue_index: index,
                        });
                        continue;
                    }
                    Ok(crate::class_conversion::MismatchOutcome::Skipped { .. }) => {
                        unreachable!("a conversion vector is always present in 4FC340")
                    }
                    Err(error) => {
                        // BadDataVersion fails the original map load.
                        report.events.push(RestoreEvent {
                            stage: RestoreStage::DynamicThings,
                            source_ordinal: ordinal,
                            saved_id: Some(saved_id),
                            registered: false,
                            completion: None,
                            error: Some(error.to_string()),
                        });
                        if mode == RestoreMode::StopOnError {
                            report.stopped_before = Some(RestoreStage::DynamicThings);
                            return;
                        }
                        continue;
                    }
                }
            }
        }
        let Some(source) = placement else {
            let record = unsupported.expect("one side of each record is present");
            if mode == RestoreMode::StopOnError {
                report.events.push(RestoreEvent {
                    stage: RestoreStage::DynamicThings,
                    source_ordinal: record.record_ordinal,
                    saved_id: None,
                    registered: false,
                    completion: None,
                    error: Some(record.reason.clone()),
                });
                report.stopped_before = Some(RestoreStage::DynamicThings);
                return;
            }
            deferred_unsupported.push(record);
            continue;
        };
        let id = ActorId::from_raw(source.serialized_id);
        let mut event = RestoreEvent {
            stage: RestoreStage::DynamicThings,
            source_ordinal: source.record_ordinal,
            saved_id: Some(id.raw()),
            registered: false,
            completion: None,
            error: None,
        };
        let mut cx = LoadContext {
            inventory,
            rng: &mut *rng,
            pc,
            updatable_registrations: &mut report.updatable_registrations,
            presentation_effects: &mut report.presentation_effects,
            skipped: &mut report.skipped,
            native_messages: &mut report.native_messages,
            lifecycle: &mut report.lifecycle,
            lifecycle_detached: &mut report.lifecycle_detached,
            world_version: &mut report.world_version,
            assert_latch: &mut assert_latch,
            source_ordinal: ordinal,
        };
        let (registered, completion, outcome) =
            load_record(registry, source, 0, false, definitions, &mut cx);
        if matches!(outcome, Ok(None)) {
            // Caught Load exception: destroyed, null, next index entry.
            continue;
        }
        event.registered = registered;
        event.completion = completion;
        event.error = outcome.err();
        let failed = event.error.is_some();
        report.events.push(event);
        if failed && mode == RestoreMode::StopOnError {
            report.stopped_before = Some(RestoreStage::DynamicThings);
            return;
        }
    }
    for record in deferred_unsupported {
        report.events.push(RestoreEvent {
            stage: RestoreStage::DynamicThings,
            source_ordinal: record.record_ordinal,
            saved_id: None,
            registered: false,
            completion: None,
            error: Some(record.reason.clone()),
        });
        if mode == RestoreMode::StopOnError {
            report.stopped_before = Some(RestoreStage::DynamicThings);
            return;
        }
    }
    report
        .events
        .sort_by_key(|event| (event.stage as u8, event.source_ordinal));
}
