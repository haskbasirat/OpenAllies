//! Fresh creation of a GActor (runtime class 3): the actor-specific work below
//! `KKC_GWorld::CreateGThing` (`4F9C10`) for an A25B28 actor creation block
//! (GACD), and the entry points CreateGActor (`4FA030`) and
//! CreateOrganization (`4FA150`).
//!
//! This is the shared path behind combat member spawn (`57AD30` / `57D950` /
//! `4FBF40`), special-operation summons, trigger reinforcements, economy
//! BuildActor (`563160`) and the trailer of a created actor.
//!
//! Composition (each native function is ported once and reused here):
//! * `4F9C10` control flow: [`world_creation::create`] (oracle
//!   research/phase8-restoration/native-create-gthing.json);
//! * constructor `4D6BB0` over `4EA5E0` / `4EF2A0` ([`creation::construct`],
//!   [`creation::select_variant`]) plus `4EE9F0`'s scene transform and the
//!   nonblocking request `4D4A10(0)` -> `4EE840(0)` (this module);
//! * InitCreate `4D6F10`: [`init_create::initialize`] (oracle
//!   research/phase7-actors/native-init-create.json) over this module's
//!   world adapter: object init `4E9FC0` (identity `4EE460`, scale `49B440`),
//!   assembly `4D21A0` with the factory `4CF480`, requirement validation
//!   `4CFB00` (a missing requirement is the native false return after the
//!   GActor.cpp 0x5E8 log), names `4F4A20` (GWorld::AssignName, one world RNG
//!   draw), numbered names `4E4980`, Character start `563D50` / default stance
//!   `4CFC80`, the trailer through `4FA030` (recursive), the component
//!   InitCreate loop and construction registration `54BFC0`;
//! * InitExisting `4D7230` (this module), including GObject `4EBE50` for an
//!   actor (`4EE700`, `4EFA40`, `4EF6A0` -> `4F3640` with the GACD block,
//!   `4EE4C0`, `4EB6E0`, `4EA150`, `4F1F70`, `4EA3D0`, `4EBC70`) and the
//!   required object `4D1FD0`;
//! * PostCreate `4CF300`: [`world_publication::post_create`];
//! * registration `4F9510` actor branch (`4D62F0`, `4CFEA0`, `4F4C40`),
//!   `50C220`, creation FX `42A590` (this module);
//! * `4FA030` / `4FA150` (this module, [`create_gactor`],
//!   [`create_organization`]).
//!
//! Every other owner's subsystem is a [`World`] method named after the
//! original function. RNG draws (GWorld+D4) in native order: facing (flag bit
//! 15), variant (`49B900` when requested -1 and aspects_as_variations),
//! scale (`49B440`), component constructors (world), name (`4F4A20`), then the
//! trailer's whole creation, then the component InitCreate virtuals (world).
//!
//! Oracle: research/phase8-actor-creation/native-actor-creation.json
//! (native_actor_creation.py, the original code under Unicorn at PC24/53/64),
//! replayed by crates/aa-actors/tests/native_actor_creation.rs.
use crate::components::{ALL_SLOTS, ComponentSlot, StateBits};
use crate::creation::{self, CreationRecord, DirectionTable, ObjectDefinition, ThingDefinition};
use crate::init_create::{self, FactoryKind};
use crate::loading::{self, LoadKind, LoadShell};
use crate::registry::ActorId;
use crate::world_creation::{self, CreateRequest, CreateWorld};
use crate::world_publication;
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use aa_simulation::NativeRng;
use serde::Serialize;
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("actor creation world service failed: {0}")]
    World(String),
    #[error("native arithmetic: {0}")]
    Numeric(String),
    /// The original reads through a null or stale pointer here (access
    /// violation): refused, never turned into success.
    #[error("native null dereference: {0}")]
    NullDereference(&'static str),
    /// The original executes `div`/`idiv` by zero here (#DE).
    #[error("native integer division by zero: {0}")]
    DivideByZero(&'static str),
    #[error("native arithmetic outside the proven domain: {0}")]
    Domain(&'static str),
    #[error("actor creation path not ported: {0}")]
    Unsupported(&'static str),
}
pub type Result<T> = std::result::Result<T, Error>;
impl From<aa_numeric::Error> for Error {
    fn from(e: aa_numeric::Error) -> Self {
        Self::Numeric(e.to_string())
    }
}

/// The actor creation block (vtable A25B28, 0x54 bytes; constructor
/// `4C8C20` + `4C8C70`). Pointer fields are GTIDs (0 = null).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ActorParams<D, L> {
    /// +4 definition.
    pub definition: D,
    /// +8..+10.
    pub position_bits: [u32; 3],
    /// +14 / +18 (`4C8C20`: (0, -1.0f)).
    pub direction_bits: [u32; 2],
    /// +1C instance IDS (empty: none).
    pub name_utf16: Vec<u16>,
    /// +20 variant (`4C8C20`: -1).
    pub variant: u32,
    /// +24 requested GTID (0: allocate).
    pub requested_id: u32,
    /// +28 owner Kingdom GTID (0: none).
    pub owner: u32,
    /// +2C Organization layout.
    pub organization_layout: Option<L>,
    /// +30 -> actor +158 (required object GTID).
    pub word_30: u32,
    /// +34 (`4C8C70`: -1): the 5644F0 city argument of PostCreate.
    pub word_34: u32,
    /// +38 creation mode (1: construction state 0x15; 3: Body first, state
    /// 0x1A; 4: no number registration).
    pub mode: u32,
    /// +3C (compared with +24 by the 4D7230 creation FX gate).
    pub word_3c: u32,
    /// +40 / +44: actor +138 candidates (4D7230).
    pub word_40: u32,
    pub word_44: u32,
    /// +48 parent GTID (trailer creation).
    pub word_48: u32,
    pub word_4c: u32,
    /// +50 (state 0x18), +51 (start Character), +52 (nonblocking).
    pub byte_50: u8,
    pub byte_51: u8,
    pub byte_52: u8,
}
impl<D, L> ActorParams<D, L> {
    /// `4C8C20` + `4C8C70` defaults with the definition at +4.
    pub fn new(definition: D) -> Self {
        Self {
            definition,
            position_bits: [0; 3],
            direction_bits: [0, 0xbf80_0000],
            name_utf16: Vec::new(),
            variant: u32::MAX,
            requested_id: 0,
            owner: 0,
            organization_layout: None,
            word_30: 0,
            word_34: u32::MAX,
            mode: 0,
            word_3c: 0,
            word_40: 0,
            word_44: 0,
            word_48: 0,
            word_4c: 0,
            byte_50: 0,
            byte_51: 0,
            byte_52: 0,
        }
    }
}

/// Definition +38 shape A ("blocking"): +0 kind, +4 radius, +10 / +14
/// maxima (the extents of kinds 2 and 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Shape {
    pub kind: u32,
    pub radius_bits: u32,
    pub extent_10_bits: u32,
    pub extent_14_bits: u32,
}

/// Definition +1AC: the name list of `4F4A20` (`+18` names, `+1C` count).
/// `id` identifies the list in the GWorld +268 assignment vector.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NameList {
    pub id: String,
    pub names: Vec<Vec<u16>>,
}

/// Definition +25C TrackInfo entry (0x24 bytes): +0 kind, +20 length.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct TrackInfo {
    pub kind: u32,
    pub length_bits: u32,
}

/// What the creation reads from an ActorData definition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ActorDefinition<D, L> {
    pub key: D,
    /// +8 IDS.
    pub ids_utf16: Vec<u16>,
    /// +10 display name (`load_warning_thing_doesnt_fit` argument).
    pub display_utf16: Vec<u16>,
    /// `[+18]+1C` runtime class.
    pub runtime_class: u32,
    /// +1C flags.
    pub flags: u32,
    /// +24 (Size.height; 4EBE50 stores scale x height at object +38).
    pub height_bits: u32,
    /// +28 / +2C.
    pub scale_min_bits: u32,
    pub scale_max_bits: u32,
    /// +34 byte (`needs_space_for_settlement_zone`, definition virtual +24).
    pub settlement_space: bool,
    /// +38 shape A.
    pub shape_a: Shape,
    /// +108 / +10C entries' +2C / +128.
    pub variations_enabled: bool,
    pub aspect_weights_bits: Vec<u32>,
    pub total_weight_bits: u32,
    /// +174 component mask.
    pub component_mask: u32,
    /// +1AC.
    pub names: Option<NameList>,
    /// +1B0 `numbered`.
    pub numbered: bool,
    /// +25C / +260 TrackInfo.
    pub tracks: Vec<TrackInfo>,
    /// +268 trailer_actor and +270 TrailerHitch node_offset.
    pub trailer: Option<D>,
    pub node_offset_bits: [u32; 3],
    /// +2F0 Organization layout.
    pub layout: Option<L>,
    /// +4E0 non-null (the required-object rule of 4D1FD0).
    pub required_object: bool,
    /// +5B4 CityComponent zone_radius (45D6A0).
    pub zone_radius_bits: u32,
}

/// A spatial-index descriptor (`4CF210` shape A / `4EA0F0` shape B): facing
/// x, y and position x, y (and a zero word).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Descriptor {
    pub shape: ShapeRef,
    pub facing_bits: [u32; 2],
    pub position_bits: [u32; 2],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ShapeRef {
    /// Object virtual +C (`4EA010`: definition +38).
    A,
    /// Object virtual +10 (`4EA020`: definition +50).
    B,
}

/// The definition action list `42A590` runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum FxList {
    /// Definition +BC (4F9C10 after registration, object virtual B0 true).
    Created,
    /// Definition +E4 (4D7230, not under construction).
    Existing,
}

/// Diagnostics of the creation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Log {
    /// `404ED0` lines of 4F9C10 (instance name used, class refusals).
    Create { line: u32, format: &'static str, argument: String },
    /// `4EFDB0` (404F60, GTID.cpp 0xAB) "KKC_GTIDManager::Add() - no IDs left".
    NoIdsLeft,
    /// `4CFB00` (404ED0, GActor.cpp 0x5E8) "Actor %s component %d requires
    /// component %d which is missing".
    MissingComponent { ids_utf16: Vec<u16>, slot: u32, required: u32 },
    /// `4EBE50` (404EA0, GObject.cpp 0x213) "Static object %s is not thick
    /// enough to block tiles".
    NotThickEnough { ids_utf16: Vec<u16> },
    /// `4EF6A0` (404F60, GThing.cpp 299) "creating thing %s failed (reason:
    /// %s) at %f,%f".
    PlacementFailed { ids_utf16: Vec<u16>, code: u32, reason: &'static str, position_bits: [u32; 2] },
    /// `4EF6A0` map warning `load_warning_thing_doesnt_fit|sff` (5A1040).
    ThingDoesntFit { display_utf16: Vec<u16>, position_bits: [u32; 2] },
    /// `4D1FD0` (404F60, GActor.cpp 0x151) "%s: failed placement due to dead
    /// required_object".
    RequiredObjectDead { ids_utf16: Vec<u16> },
    /// 0x15A "%s: ignoring failed placement due to 'required object' while
    /// loading".
    RequiredObjectIgnored { ids_utf16: Vec<u16> },
    /// 0x161 "%s: successful placement on required_object %d".
    RequiredObjectPlaced { ids_utf16: Vec<u16>, required: u32 },
    /// `4FA150` (404F60, GWorld.cpp 0x8A0) "org %d failed to create any
    /// elements - destroying".
    NoMembers { gtid: u32 },
}

/// Placement failure reasons of `4EF6A0` (strings A2AB04..A2A978).
pub fn placement_reason(code: u32) -> &'static str {
    match code {
        1 => "collision with tile",
        2 => "collision with thing",
        3 => "collision with ether",
        4 => "invalid structure placement",
        5 => "settlement zone cannot be placed here",
        6 => "independent building in settlement",
        7 => "indestructible object in settlement zone",
        _ => "unknown",
    }
}

/// `[B36528]` (the SAI/UI agent global).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiAgent {
    Absent,
    /// Present; byte +20 (watching).
    Present { watching: bool },
}

/// The actor's stored shell (the same representation as a restored actor).
/// Pointer-valued actor words hold GTIDs (+E8 owner, +138, +158, +168,
/// +16C). `object.associated_handle` is object +38 (scale x height after
/// InitExisting).
pub type Shell<W> = LoadShell<<W as World>::Definition, <W as World>::Component>;

/// Every native call of the actor creation that belongs to another owner's
/// subsystem, named after the original function. A missing service must
/// return an error, never a default.
pub trait World {
    type Definition: Clone + PartialEq + std::fmt::Debug;
    type Layout: Clone + std::fmt::Debug;
    /// The payload of an actor component slot (+60[slot]).
    type Component;
    /// A created actor (stable from allocation on).
    type Actor: Copy + PartialEq + std::fmt::Debug;

    // ---- definitions and GWorld facts
    fn definition(&self, key: &Self::Definition) -> Result<ActorDefinition<Self::Definition, Self::Layout>>;
    /// GWorld +D4 (`4F02B0`): the single authoritative stream.
    fn rng(&mut self) -> &mut NativeRng;
    /// GWorld +E0.
    fn begin_time_bits(&self) -> u32;
    /// `[B39AF8]`.
    fn directions(&self) -> DirectionTable;
    /// `[GWorld+30]+8` / `+4`.
    fn tile_size_bits(&self) -> u32;
    fn tile_shift(&self) -> u32;
    /// `[GWorld+28]+A8` (map mode; `4F0460` is "non-zero").
    fn map_mode(&self) -> u32;
    /// `50C270` / `50C220` gate: GWorld+D8 != 6 and `[GWorld+130]+4` != 0.
    fn scene_present(&self) -> bool;
    /// `[B36528]`.
    fn ui_agent(&self) -> UiAgent;
    /// `[B36530]+2E8` registered actor states (4D62F0).
    fn registered_state_count(&self) -> u32;
    /// `[[[B36530]+464]+24]`: the global property handle of 4D7230 (None: 0).
    fn global_property(&self) -> Option<u32>;
    /// `[[B39AB0]+1C]` (binary32 bits) and `[[B39AB0]+28]` (signed): the
    /// TrackInfo period settings of 4D7230.
    fn track_settings(&self) -> Result<(u32, i32)>;
    /// `[B39F58]+74` (binary32 bits): the zone radius term of `4BB000`.
    fn thing_manager_74_bits(&self) -> Result<u32>;
    /// Assertion latches B3CC40 / B3CC42 / B3CC43 / B3CC44.
    fn latch(&self, address: u32) -> bool;
    fn set_latch(&mut self, address: u32, value: bool);
    /// `855D30(expression, file, line)`: the new latch value.
    fn assertion_dialog(&mut self, expression: &'static str, line: u32) -> Result<bool>;
    fn log(&mut self, entry: Log) -> Result<()>;

    // ---- objects and identity
    /// `6F7810(0x180)`: store the constructed GActor shell.
    fn allocate(&mut self, shell: Shell<Self>) -> Result<Self::Actor>;
    fn shell(&self, actor: Self::Actor) -> Result<&Shell<Self>>;
    fn shell_mut(&mut self, actor: Self::Actor) -> Result<&mut Shell<Self>>;
    /// Object +14 (0 before `4EE460`).
    fn gtid(&self, actor: Self::Actor) -> Result<u32>;
    /// `4EFB70` + `4F0190`: register the requested GTID for the actor.
    fn register_requested_id(&mut self, actor: Self::Actor, id: u32) -> Result<()>;
    /// `4EFDB0`: allocate a GTID for the actor; None when 0x8000 are live.
    fn allocate_id(&mut self, actor: Self::Actor) -> Result<Option<u32>>;
    /// IDMGR `[B364C0]`: generation of `id` matches and its slot is non-null
    /// (`401060`).
    fn gtid_live(&self, id: u32) -> Result<bool>;
    /// The live actor of a GTID (None: not live or not an actor of this world).
    fn resolve_actor(&self, id: u32) -> Result<Option<Self::Actor>>;
    /// `4F5490`: destroy an actor whose initialization failed.
    fn destroy(&mut self, actor: Self::Actor) -> Result<()>;
    /// `43B7D0` (scene transform through `50C270`) / `43D160` (scene add
    /// through `50C220`).
    fn scene_transform(&mut self, actor: Self::Actor) -> Result<()>;
    fn scene_add(&mut self, actor: Self::Actor) -> Result<()>;
    /// `42A590(list, actor, 0)` -> `429080`.
    fn creation_fx(&mut self, actor: Self::Actor, list: FxList) -> Result<()>;

    // ---- 4F9C10 actor steps
    /// `4154D0` on GWorld+F0: the entry's +4 GTID for an instance IDS.
    fn instance_name_gtid(&mut self, name: &[u16]) -> Result<Option<u32>>;
    /// `57C450(layout)` on the Organization component.
    fn organization_set_layout(&mut self, actor: Self::Actor, layout: &Self::Layout) -> Result<()>;
    /// `4D49B0(group)` (aa_actors::state::group; a pure read).
    fn state_group(&self, actor: Self::Actor, group: u32) -> Result<Option<u32>>;
    /// `4D64F0(state, enabled)` (aa_actors::state::set); updates the shell's
    /// state words.
    fn set_state(&mut self, actor: Self::Actor, state: u32, enabled: bool) -> Result<()>;

    // ---- components (factory 4CF480 and the component virtuals)
    /// `4CF480`'s constructor of `kind`: insert the component into the
    /// shell's slot table.
    fn construct_component(&mut self, actor: Self::Actor, slot: ComponentSlot, kind: FactoryKind) -> Result<()>;
    /// Component virtual +20 (the required slot; None for -1).
    fn component_requirement(&mut self, actor: Self::Actor, slot: ComponentSlot) -> Result<Option<ComponentSlot>>;
    /// Component virtual +4 (InitCreate).
    fn component_init_create(
        &mut self,
        actor: Self::Actor,
        slot: ComponentSlot,
        params: &ActorParams<Self::Definition, Self::Layout>,
    ) -> Result<bool>;
    /// Component virtual +8 (InitExisting).
    fn component_init_existing(&mut self, actor: Self::Actor, slot: ComponentSlot) -> Result<bool>;
    /// Component virtual +14 (after every InitExisting).
    fn component_vf14(&mut self, actor: Self::Actor, slot: ComponentSlot) -> Result<()>;
    /// Component virtual +C (PostCreate).
    fn component_post_create(
        &mut self,
        actor: Self::Actor,
        slot: ComponentSlot,
        params: &ActorParams<Self::Definition, Self::Layout>,
        flag: u32,
    ) -> Result<()>;
    /// `79EA90(owner)` on `[B36528]` and what follows in `4D21A0`
    /// (`4CED60`, `7B41E0`, `7B3F70`) after newly created components.
    fn ui_components_created(&mut self, actor: Self::Actor, owner: u32) -> Result<()>;

    // ---- InitCreate services
    /// `4D0720`: the Character view: (+4C non-zero, first +48 entry non-null).
    fn character_view(&self, actor: Self::Actor) -> Result<(bool, bool)>;
    /// `563D50` / `563BA0` on the Character component.
    fn character_start(&mut self, actor: Self::Actor) -> Result<()>;
    fn character_default_stance(&mut self, actor: Self::Actor) -> Result<()>;
    /// GWorld +268 (count +26C): the (name list, index) assignments.
    fn name_assignments(&self) -> Result<Vec<(String, i32)>>;
    fn push_name_assignment(&mut self, list: &str, index: i32) -> Result<()>;
    /// `4D7F40(name)`: the actor's map-string name.
    fn set_name(&mut self, actor: Self::Actor, name: &[u16]) -> Result<()>;
    /// `4E4980(actor)` on the owner Kingdom (+35C counters; `4CF1A0` stores
    /// actor +170).
    fn assign_number(&mut self, owner: u32, actor: Self::Actor) -> Result<()>;
    /// Body +10 / +14 of an actor (binary32 bits).
    fn body_health(&self, actor: Self::Actor) -> Result<[u32; 2]>;
    /// `562200(health, maximum)` on the child's Body.
    fn copy_health(&mut self, child: Self::Actor, health: [u32; 2]) -> Result<()>;
    /// `54BFC0(owner, definition)` on `[B36520]`.
    fn register_construction(&mut self, owner: u32, definition: &Self::Definition) -> Result<()>;

    // ---- InitExisting services
    /// `46EE00(&xy, tile size)` on shape A.
    fn snap(&mut self, shape: &Shape, xy: [u32; 2], tile_size_bits: u32) -> Result<[u32; 2]>;
    /// `4F3640(params, actor, actor, 0, 0, 0, 0)`: 0 or a reason code.
    fn placement_test(
        &mut self,
        actor: Self::Actor,
        params: &ActorParams<Self::Definition, Self::Layout>,
    ) -> Result<u32>;
    /// `5536B0(actor, b, a)` / `553830(actor, b, a)`: the original sets /
    /// clears flag bit 26; the world must do the same on the shell.
    fn index_insert(&mut self, actor: Self::Actor, b: Descriptor, a: Descriptor) -> Result<()>;
    fn index_remove(&mut self, actor: Self::Actor, b: Descriptor, a: Descriptor) -> Result<()>;
    /// `4F0E50(&xy, flags)`: the extended ST0 result.
    fn terrain_height(&mut self, xy: [u32; 2], flags: u32) -> Result<Binary>;
    /// `4EB6E0` with flag bit 10: `554350` + `555620` and `4D67F0` on every
    /// actor found.
    fn blocking_notify(&mut self, actor: Self::Actor) -> Result<()>;
    /// `4EA150`, `4F1F70` (flag bit 8: GWorld +8 head insert), `4EBC70`.
    fn tile_footprint(&mut self, actor: Self::Actor) -> Result<()>;
    fn world_list_insert(&mut self, actor: Self::Actor) -> Result<()>;
    fn visibility_refresh(&mut self, actor: Self::Actor) -> Result<()>;
    /// `555830(definition, x, y)` on the GMap: the found object's GTID.
    fn find_required_object(&mut self, definition: &Self::Definition, xy: [u32; 2]) -> Result<Option<u32>>;
    /// `46B000(&required+20, &actor+20)` (true: blocked).
    fn required_placement_blocked(&mut self, required: u32, actor: Self::Actor) -> Result<bool>;
    /// Required object virtual +128(actor).
    fn required_object_attach(&mut self, required: u32, actor: Self::Actor) -> Result<()>;
    /// `4D4CF0`, `4D1350`, `4D67F0`, `4D4130` on the actor.
    fn refresh_4d4cf0(&mut self, actor: Self::Actor) -> Result<()>;
    fn refresh_4d1350(&mut self, actor: Self::Actor) -> Result<()>;
    fn refresh_4d67f0(&mut self, actor: Self::Actor) -> Result<()>;
    fn refresh_4d4130(&mut self, actor: Self::Actor) -> Result<()>;
    /// `5035E0(actor, property)` / `5817A0(property, 0, actor)` on +78.
    fn property_applies(&mut self, actor: Self::Actor, property: u32) -> Result<bool>;
    fn properties_add(&mut self, actor: Self::Actor, property: u32) -> Result<()>;
    /// `5788B0` (+7C), `585650` (+98), `57E430` (+74), `569E40` (+88).
    fn organization_refresh(&mut self, actor: Self::Actor) -> Result<()>;
    fn settlement_refresh(&mut self, actor: Self::Actor) -> Result<()>;
    fn performer_refresh(&mut self, actor: Self::Actor) -> Result<()>;
    fn denizen_refresh(&mut self, actor: Self::Actor) -> Result<()>;
    /// `4D69A0(index)` (KKC_GActor::CreateZone, aa_zones::component::
    /// create_zone): true.
    fn create_zone(&mut self, actor: Self::Actor, index: u32) -> Result<bool>;
    /// `515F40(x, y, radius)` on zone slot `index` (None: the native null
    /// `this` when the vector holds `index` or fewer entries).
    fn move_zone(&mut self, actor: Self::Actor, index: u32, xy: [u32; 2], radius_bits: u32) -> Result<()>;
    /// Actor +14C (zone vector size, signed).
    fn zone_count(&self, actor: Self::Actor) -> Result<i32>;
    /// `7A0C20(actor)` on `[B36528]`.
    fn sai_notify(&mut self, actor: Self::Actor) -> Result<()>;
    /// `6F7810(0x20)` + `50AB10(actor, period)` -> actor +144.
    fn track_updatable(&mut self, actor: Self::Actor, period: i32) -> Result<()>;

    // ---- PostCreate and registration
    /// `4EB500(params)`.
    fn ground_paint(&mut self, actor: Self::Actor, params: &ActorParams<Self::Definition, Self::Layout>) -> Result<()>;
    /// `4E6180(actor)` on the owner Kingdom (aa_economy::kingdom_registry::add_actor).
    fn kingdom_add_actor(&mut self, owner: u32, actor: Self::Actor) -> Result<()>;
    /// `5644F0(actor, params+34, 0)`.
    fn city_create(&mut self, actor: Self::Actor, word_34: u32) -> Result<()>;
    /// `5C9180` (remove the found entry) / `5043F0` + `4F7B70` (insert) on
    /// the instance-name map.
    fn instance_name_remove(&mut self, name: &[u16]) -> Result<()>;
    fn instance_name_insert(&mut self, name: &[u16], gtid: u32) -> Result<()>;
    /// `4D4A90(state descriptor, 0)` for an active registered state.
    fn state_reenter(&mut self, actor: Self::Actor, state: u32) -> Result<()>;
    /// `4CFEA0`: spatial grid registration.
    fn grid_register(&mut self, actor: Self::Actor) -> Result<()>;
    /// The +158 object's definition virtual +24.
    fn object_settlement_space(&self, gtid: u32) -> Result<bool>;
    /// GWorld +260 / +264 available list.
    fn available_contains(&self, gtid: u32) -> Result<bool>;
    fn add_available(&mut self, gtid: u32) -> Result<()>;

    // ---- 4FA150
    /// `[layout+8]+2C`, `4C2BC0(layout)`, `578290(layout, slot, &xy, facing)`.
    fn layout_slot_count(&mut self, layout: &Self::Layout) -> Result<i32>;
    fn layout_first_slot(&mut self, layout: &Self::Layout) -> Result<u32>;
    fn layout_slot_placement(
        &mut self,
        layout: &Self::Layout,
        slot: u32,
        xy: &mut [u32; 2],
        facing_bits: [u32; 2],
    ) -> Result<bool>;
    /// Organization component +18 layout data: +18 leader, +2C slot count,
    /// and `+1C[+28[slot]] != 0`.
    fn organization_leader(&mut self, actor: Self::Actor) -> Result<i32>;
    fn organization_slot_count(&mut self, actor: Self::Actor) -> Result<i32>;
    fn organization_slot_defined(&mut self, actor: Self::Actor, slot: i32) -> Result<bool>;
    /// `57AD30(slot, flag)` (member spawn, `4FBF40`) / `57A730` (members).
    fn spawn_member(&mut self, actor: Self::Actor, slot: i32, flag: u8) -> Result<()>;
    fn member_count(&mut self, actor: Self::Actor) -> Result<u32>;
}

// ---------------------------------------------------------------- helpers


fn f32b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
/// `_ftol2` (96CC4C): truncation toward zero; out-of-range is refused.
fn ftol(value: &Binary) -> Result<i32> {
    value.truncate_i32().map_err(|_| Error::Domain("_ftol2 of a value outside i32"))
}
fn word(shell: &LoadShell<impl Sized, impl Sized>, offset: u32) -> u32 {
    shell.actor.as_ref().and_then(|a| a.words.get(&offset).copied()).unwrap_or(0)
}
fn set_word<D, C>(shell: &mut LoadShell<D, C>, offset: u32, value: u32) -> Result<()> {
    shell
        .actor
        .as_mut()
        .ok_or(Error::Unsupported("GActor field on a non-actor shell"))?
        .words
        .insert(offset, value);
    Ok(())
}
fn state_bits<W: World>(world: &W, actor: W::Actor) -> Result<StateBits> {
    Ok(world
        .shell(actor)?
        .actor
        .as_ref()
        .ok_or(Error::Unsupported("GActor state on a non-actor shell"))?
        .components
        .state)
}
fn has<W: World>(world: &W, actor: W::Actor, slot: ComponentSlot) -> Result<bool> {
    Ok(world
        .shell(actor)?
        .actor
        .as_ref()
        .is_some_and(|a| a.components.present(slot)))
}
fn object_definition<D, L>(def: &ActorDefinition<D, L>) -> ObjectDefinition {
    ObjectDefinition {
        variations_enabled: def.variations_enabled,
        variation_weights_bits: def.aspect_weights_bits.clone(),
        total_variation_weight_bits: def.total_weight_bits,
        scale_min_bits: def.scale_min_bits,
        scale_max_bits: def.scale_max_bits,
    }
}
fn descriptors<D, C>(shell: &LoadShell<D, C>) -> (Descriptor, Descriptor) {
    let d = |shape| Descriptor {
        shape,
        facing_bits: [shell.thing.facing_bits[0], shell.thing.facing_bits[1]],
        position_bits: [shell.thing.position_bits[0], shell.thing.position_bits[1]],
    };
    (d(ShapeRef::B), d(ShapeRef::A))
}

// ------------------------------------------------------------ 4D6BB0 ctor
/// `6F7810(0x180)` + `4D6BB0(params)`: GThing `4EF2A0` (facing draw / the
/// direction check that rewrites params +14/+18), `4EE9F0`'s scene transform,
/// GObject `4EA5E0` (variant draw), the GActor fields (+E8 owner when flag
/// bit 23, else params +28 = 0; +158 = params +30), then `4D4A10(0)` when
/// params +52 is set.
pub fn construct<W: World>(
    params: &mut ActorParams<W::Definition, W::Layout>,
    def: &ActorDefinition<W::Definition, W::Layout>,
    world: &mut W,
    pc: Precision,
) -> Result<W::Actor> {
    let shell = loading::construct_shell_with_components::<W::Definition, W::Component>(
        LoadKind::Actor,
        def.key.clone(),
    );
    let actor = world.allocate(shell)?;
    let mut record = CreationRecord {
        definition: ThingDefinition { key: def.key.clone(), flags: def.flags },
        position_bits: params.position_bits,
        direction_bits: params.direction_bits,
        name_utf16: params.name_utf16.clone(),
        saved_id: ActorId::from_raw(params.requested_id),
    };
    let begin = world.begin_time_bits();
    let directions = world.directions();
    let thing = creation::construct(&mut record, begin, world.rng(), directions, pc)?;
    params.direction_bits = record.direction_bits;
    {
        let s = world.shell_mut(actor)?;
        s.thing.flags = thing.flags;
        s.thing.word_c = thing.word_c;
        s.thing.word_10 = thing.word_10;
        s.thing.name_utf16 = thing.name_utf16;
        s.thing.word_1c = Some(thing.created_at_bits);
        s.thing.position_bits = thing.position_bits;
        s.thing.facing_bits = thing.facing_bits;
    }
    if thing.flags & (1 << 26) != 0 {
        return Err(Error::Unsupported(
            "4EE9F0 index move during construction (flag bit 26 is never copied from a definition)",
        ));
    }
    if world.scene_present() {
        world.scene_transform(actor)?;
    }
    if def.variations_enabled {
        let variant = if params.variant == u32::MAX {
            creation::select_variant(&object_definition(def), world.rng(), pc)
                .map_err(|_| Error::Domain("49B900 ran past its aspect list"))?
        } else {
            params.variant as i32
        };
        world
            .shell_mut(actor)?
            .object
            .as_mut()
            .ok_or(Error::Unsupported("GObject fields on a non-object shell"))?
            .variant = variant;
    }
    let owner = if thing.flags & (1 << 23) == 0 {
        params.owner = 0;
        0
    } else {
        params.owner
    };
    {
        let s = world.shell_mut(actor)?;
        set_word(s, 0xe8, owner)?;
        set_word(s, 0x158, params.word_30)?;
    }
    if params.byte_52 != 0 {
        set_blocking(actor, false, world)?;
    }
    Ok(actor)
}

/// GActor virtual +50 `4D4A10(flag)`: `4EE840(flag)` (flag bit 10 change,
/// index membership `4EE4C0`), then the same on the live +168 trailer.
pub fn set_blocking<W: World>(actor: W::Actor, blocking: bool, world: &mut W) -> Result<bool> {
    if !object_set_blocking(actor, blocking, world)? {
        return Ok(false);
    }
    let linked = word(world.shell(actor)?, 0x168);
    if world.gtid_live(linked)? {
        let child = world
            .resolve_actor(linked)?
            .ok_or(Error::Unsupported("4D4A10 on a live +168 object that is not an actor of this world"))?;
        set_blocking(child, blocking, world)?;
    }
    Ok(true)
}

/// GObject virtual +50 `4EE840(flag)`.
fn object_set_blocking<W: World>(actor: W::Actor, blocking: bool, world: &mut W) -> Result<bool> {
    let flags = world.shell(actor)?.thing.flags;
    if (flags >> 10) & 1 == u32::from(blocking) {
        return Ok(false);
    }
    if flags & (1 << 26) != 0 {
        let (b, a) = descriptors(world.shell(actor)?);
        world.index_remove(actor, b, a)?;
    }
    let s = world.shell_mut(actor)?;
    if blocking {
        s.thing.flags |= 0x400;
    } else {
        s.thing.flags &= !0x400;
    }
    index_membership(actor, world)?;
    Ok(true)
}

/// `4EE4C0`: GActor virtual B0 is `6A71D0` (true), so the actor belongs in
/// the index; insert when flag bit 26 is clear.
fn index_membership<W: World>(actor: W::Actor, world: &mut W) -> Result<()> {
    let flags = world.shell(actor)?.thing.flags;
    let wanted = true || flags & 0x4408 != 0;
    let present = flags & (1 << 26) != 0;
    let (b, a) = descriptors(world.shell(actor)?);
    if present && !wanted {
        world.index_remove(actor, b, a)?;
    }
    if !present && wanted {
        world.index_insert(actor, b, a)?;
    }
    Ok(())
}

// -------------------------------------------------------- 4E9FC0 / 4EE460
/// GObject InitCreate `4E9FC0`: identity `4EE460` (requested GTID: `4EFB70`
/// + `4F0190`; none: `4EFDB0`, "no IDs left" and false when 0x8000 are
/// live), then the random scale `49B440` (object +3C).
pub fn object_init_create<W: World>(
    actor: W::Actor,
    params: &ActorParams<W::Definition, W::Layout>,
    def: &ActorDefinition<W::Definition, W::Layout>,
    world: &mut W,
    pc: Precision,
) -> Result<bool> {
    if params.requested_id != 0 {
        world.register_requested_id(actor, params.requested_id)?;
    } else if world.allocate_id(actor)?.is_none() {
        world.log(Log::NoIdsLeft)?;
        return Ok(false);
    }
    let scale = creation::random_scale(&object_definition(def), world.rng(), pc)
        .map_err(|_| Error::Domain("49B440 random scale"))?;
    world
        .shell_mut(actor)?
        .object
        .as_mut()
        .ok_or(Error::Unsupported("GObject fields on a non-object shell"))?
        .scale_bits = scale;
    Ok(true)
}

// ------------------------------------------------------------- 4F4A20
/// `4F4A20` (GWorld::AssignName): one world RNG draw modulo the list count
/// (unsigned; a zero count is the native #DE), the first index not yet
/// assigned to this list (restarting the scan after every hit: the first hit
/// restarts at 0, later hits at index + 1), the (list, index) pair appended
/// to GWorld +268 when an index below the count was found, then `4D7F40`
/// with that name (or the drawn one when every index is taken).
pub fn assign_name<W: World>(actor: W::Actor, list: &NameList, world: &mut W) -> Result<()> {
    let count = u32::try_from(list.names.len()).map_err(|_| Error::Domain("name list count above u32"))?;
    let drawn = world.rng().next_u32();
    if count == 0 {
        return Err(Error::DivideByZero("4F4A20 rng % name count"));
    }
    let drawn = drawn % count;
    let assigned = world.name_assignments()?;
    let mut index = drawn as i32;
    let mut found = false;
    let mut next = drawn as i32;
    let mut i: i64 = 0;
    while i < assigned.len() as i64 {
        if count as i32 <= index {
            break;
        }
        let (id, taken) = &assigned[i as usize];
        if *id == list.id && *taken == index {
            if found {
                next = index.wrapping_add(1);
            } else {
                next = 0;
                found = true;
            }
            i = -1;
            index = next;
        }
        i += 1;
    }
    let mut chosen = drawn as i32;
    if index < count as i32 {
        world.push_name_assignment(&list.id, index)?;
        chosen = next;
    }
    let name = list
        .names
        .get(chosen as u32 as usize)
        .ok_or(Error::NullDereference("4D7F40 name entry past the list"))?
        .clone();
    world.set_name(actor, &name)
}

// ------------------------------------------------------------ InitCreate
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handle<A> {
    Actor(A),
    Gtid(u32),
}

struct InitAdapter<'a, W: World> {
    world: &'a mut W,
    params: &'a ActorParams<W::Definition, W::Layout>,
    def: &'a ActorDefinition<W::Definition, W::Layout>,
    error: Option<Error>,
}
impl<W: World> InitAdapter<'_, W> {
    fn fail(&mut self, e: Error) -> init_create::Error {
        let message = e.to_string();
        self.error.get_or_insert(e);
        init_create::Error::World(message)
    }
    fn actor(&mut self, h: Handle<W::Actor>) -> init_create::Result<W::Actor> {
        match h {
            Handle::Actor(a) => Ok(a),
            Handle::Gtid(_) => Err(self.fail(Error::Unsupported("actor service on a GTID handle"))),
        }
    }
    fn gtid(&mut self, h: Handle<W::Actor>) -> init_create::Result<u32> {
        match h {
            Handle::Gtid(g) => Ok(g),
            Handle::Actor(a) => self.world.gtid(a).map_err(|e| self.fail(e)),
        }
    }
}
type IR<T> = init_create::Result<T>;
macro_rules! w {
    ($self:ident, $e:expr) => {{
        let r = $e;
        r.map_err(|e| $self.fail(e))
    }};
}

impl<W: World> init_create::World for InitAdapter<'_, W> {
    type Actor = Handle<W::Actor>;
    type Definition = W::Definition;
    type Record = ActorParams<W::Definition, W::Layout>;
    fn definition(&self, _: Self::Actor) -> IR<init_create::Definition<W::Definition>> {
        let def = self.def;
        let anchor = def.node_offset_bits.map(F32);
        let linked = match &def.trailer {
            Some(t) => {
                let child = self.world.definition(t).map_err(|e| init_create::Error::World(e.to_string()))?;
                Some(init_create::LinkedDefinition { definition: t.clone(), anchor: child.node_offset_bits.map(F32) })
            }
            None => None,
        };
        Ok(init_create::Definition {
            identity: def.key.clone(),
            component_mask: def.component_mask,
            number_registration: def.names.as_ref().map(|_| def.key.clone()),
            automatic_number: def.numbered,
            anchor,
            linked,
        })
    }
    fn initialize_object(&mut self, a: Self::Actor, _: &Self::Record, pc: Precision) -> IR<bool> {
        let a = self.actor(a)?;
        w!(self, object_init_create(a, self.params, self.def, self.world, pc))
    }
    fn state_group(&self, a: Self::Actor, group: u32) -> IR<Option<u32>> {
        match a {
            Handle::Actor(a) => self.world.state_group(a, group).map_err(|e| init_create::Error::World(e.to_string())),
            Handle::Gtid(_) => Err(init_create::Error::Unsupported("state group of a GTID handle")),
        }
    }
    fn set_state(&mut self, a: Self::Actor, state: u32, enabled: bool) -> IR<()> {
        let a = self.actor(a)?;
        w!(self, self.world.set_state(a, state, enabled))
    }
    fn state_bits(&self, a: Self::Actor) -> IR<StateBits> {
        match a {
            Handle::Actor(a) => state_bits(&*self.world, a).map_err(|e| init_create::Error::World(e.to_string())),
            Handle::Gtid(_) => Err(init_create::Error::Unsupported("state of a GTID handle")),
        }
    }
    fn has_component(&self, a: Self::Actor, slot: ComponentSlot) -> IR<bool> {
        match a {
            Handle::Actor(a) => has(&*self.world, a, slot).map_err(|e| init_create::Error::World(e.to_string())),
            Handle::Gtid(_) => Err(init_create::Error::Unsupported("components of a GTID handle")),
        }
    }
    fn construct_component(&mut self, a: Self::Actor, slot: ComponentSlot, kind: FactoryKind, _: Precision) -> IR<()> {
        let a = self.actor(a)?;
        w!(self, self.world.construct_component(a, slot, kind))
    }
    fn component_requirement(&mut self, a: Self::Actor, slot: ComponentSlot) -> IR<Option<ComponentSlot>> {
        let a = self.actor(a)?;
        w!(self, self.world.component_requirement(a, slot))
    }
    fn city_allowed(&mut self, _: Self::Actor) -> IR<bool> {
        // 4D2100(0x1D) -> 45D6A0: ActorData +5B4 > 0.0 (fcomp, C0|C3 false).
        w!(self, zone_radius_positive(self.def))
    }
    fn construction_ready(&mut self, _: Self::Actor) -> IR<()> {
        // 4B2830 is `ret`.
        Ok(())
    }
    fn register_number(&mut self, a: Self::Actor, _: &W::Definition) -> IR<()> {
        let a = self.actor(a)?;
        let list = self.def.names.clone().ok_or(init_create::Error::Unsupported("+1AC without a name list"))?;
        w!(self, assign_name(a, &list, self.world))
    }
    fn stored_owner(&self, a: Self::Actor) -> IR<Option<Self::Actor>> {
        match a {
            Handle::Actor(a) => {
                let owner = self.world.shell(a).map(|s| word(s, 0xe8)).map_err(|e| init_create::Error::World(e.to_string()))?;
                Ok((owner != 0).then_some(Handle::Gtid(owner)))
            }
            Handle::Gtid(_) => Err(init_create::Error::Unsupported("+E8 of a GTID handle")),
        }
    }
    fn owner(&mut self, a: Self::Actor) -> IR<Option<Self::Actor>> {
        // GActor virtual +108 is 4D1E80: the +E8 word.
        self.stored_owner(a)
    }
    fn assembly_completed(&mut self, a: Self::Actor, _: &[ComponentSlot]) -> IR<()> {
        let a = self.actor(a)?;
        let owner = w!(self, self.world.shell(a).map(|s| word(s, 0xe8)))?;
        if owner == 0 || !matches!(self.world.ui_agent(), UiAgent::Present { watching: true }) {
            return Ok(());
        }
        w!(self, self.world.ui_components_created(a, owner))
    }
    fn assign_owner_number(&mut self, a: Self::Actor, owner: Self::Actor) -> IR<()> {
        let a = self.actor(a)?;
        let owner = self.gtid(owner)?;
        w!(self, self.world.assign_number(owner, a))
    }
    fn character_has_start(&self, a: Self::Actor) -> IR<bool> {
        match a {
            Handle::Actor(a) => {
                let (count, first) = self.world.character_view(a).map_err(|e| init_create::Error::World(e.to_string()))?;
                Ok(count && first)
            }
            Handle::Gtid(_) => Err(init_create::Error::Unsupported("Character of a GTID handle")),
        }
    }
    fn start_character(&mut self, a: Self::Actor) -> IR<()> {
        let a = self.actor(a)?;
        w!(self, self.world.character_start(a))
    }
    fn default_stance(&mut self, a: Self::Actor) -> IR<()> {
        let a = self.actor(a)?;
        // 4CFC80: Character -> 563BA0; else the +E4 object (null for a fresh
        // actor: the constructor zeroes +E4).
        if w!(self, has(&*self.world, a, ComponentSlot::Character))? {
            return w!(self, self.world.character_default_stance(a));
        }
        let e4 = w!(self, self.world.shell(a).map(|s| word(s, 0xe4)))?;
        if e4 != 0 {
            return Err(self.fail(Error::Unsupported("4CFC80 on a non-null +E4 object")));
        }
        Ok(())
    }
    fn position(&self, a: Self::Actor) -> IR<[F32; 3]> {
        match a {
            Handle::Actor(a) => Ok(self
                .world
                .shell(a)
                .map_err(|e| init_create::Error::World(e.to_string()))?
                .thing
                .position_bits
                .map(F32)),
            Handle::Gtid(_) => Err(init_create::Error::Unsupported("position of a GTID handle")),
        }
    }
    fn facing_xy(&self, a: Self::Actor) -> IR<[F32; 2]> {
        match a {
            Handle::Actor(a) => {
                let f = self.world.shell(a).map_err(|e| init_create::Error::World(e.to_string()))?.thing.facing_bits;
                Ok([F32(f[0]), F32(f[1])])
            }
            Handle::Gtid(_) => Err(init_create::Error::Unsupported("facing of a GTID handle")),
        }
    }
    fn actor_id(&self, a: Self::Actor) -> IR<ActorId> {
        match a {
            Handle::Actor(a) => {
                Ok(ActorId::from_raw(self.world.gtid(a).map_err(|e| init_create::Error::World(e.to_string()))?))
            }
            Handle::Gtid(g) => Ok(ActorId::from_raw(g)),
        }
    }
    fn create_linked(
        &mut self,
        request: init_create::LinkedRequest<Self::Actor, W::Definition>,
        pc: Precision,
    ) -> IR<Option<Self::Actor>> {
        // 4C8C70 block: +4 trailer definition, +8 position (4BE740), +14/+18
        // the parent's facing, +28 owner (virtual +108), +48 parent GTID,
        // +52 nonblocking; then 4FA030.
        let owner = match request.owner {
            Some(o) => self.gtid(o)?,
            None => 0,
        };
        let mut params = ActorParams::new(request.definition);
        params.position_bits = request.position.map(|v| v.0);
        params.direction_bits = request.facing_xy.map(|v| v.0);
        params.owner = owner;
        params.word_48 = request.parent_id.raw();
        params.byte_52 = u8::from(request.nonblocking);
        let created = w!(self, create_gactor(&mut params, self.world, pc))?;
        Ok(created.map(Handle::Actor))
    }
    fn set_linked_id(&mut self, a: Self::Actor, child: ActorId) -> IR<()> {
        let a = self.actor(a)?;
        w!(self, self.world.shell_mut(a).and_then(|s| set_word(s, 0x168, child.raw())))
    }
    fn resolve_id(&self, id: ActorId) -> IR<Option<Self::Actor>> {
        if !self.world.gtid_live(id.raw()).map_err(|e| init_create::Error::World(e.to_string()))? {
            return Ok(None);
        }
        match self.world.resolve_actor(id.raw()).map_err(|e| init_create::Error::World(e.to_string()))? {
            Some(a) => Ok(Some(Handle::Actor(a))),
            None => Err(init_create::Error::Unsupported("live linked GTID that is not an actor of this world")),
        }
    }
    fn set_parent_id(&mut self, child: Self::Actor, parent: ActorId) -> IR<()> {
        let child = self.actor(child)?;
        w!(self, self.world.shell_mut(child).and_then(|s| set_word(s, 0x16c, parent.raw())))
    }
    fn copy_linked_health(&mut self, parent: Self::Actor, child: Self::Actor, _: Precision) -> IR<()> {
        let parent = self.actor(parent)?;
        let child = self.actor(child)?;
        let health = w!(self, self.world.body_health(parent))?;
        w!(self, self.world.copy_health(child, health))
    }
    fn initialize_component(&mut self, a: Self::Actor, slot: ComponentSlot, record: &Self::Record, _: Precision) -> IR<bool> {
        let a = self.actor(a)?;
        w!(self, self.world.component_init_create(a, slot, record))
    }
    fn register_construction(&mut self, _: Self::Actor, owner: Option<Self::Actor>, definition: &W::Definition) -> IR<()> {
        let owner = match owner {
            Some(o) => self.gtid(o)?,
            None => 0,
        };
        w!(self, self.world.register_construction(owner, definition))
    }
}

/// `45D6A0`: ActorData +5B4 > 0.0 (`fcomp`; equal, less and unordered are
/// false).
fn zone_radius_positive<D, L>(def: &ActorDefinition<D, L>) -> Result<bool> {
    let bits = def.zone_radius_bits;
    if f32::from_bits(bits).is_nan() {
        return Ok(false);
    }
    if f32::from_bits(bits).is_infinite() {
        return Ok(bits & 0x8000_0000 == 0);
    }
    Ok(f32b(bits)?.compare(&Binary::from_u32(0)) == Ordering::Greater)
}


/// InitCreate `4D6F10`: [`init_create::initialize`] over this module's
/// world. A requirement missing after the assembly (`4CFB00`) is the native
/// false return after its log.
pub fn init_create<W: World>(
    actor: W::Actor,
    params: &ActorParams<W::Definition, W::Layout>,
    def: &ActorDefinition<W::Definition, W::Layout>,
    world: &mut W,
    pc: Precision,
) -> Result<bool> {
    let mut adapter = InitAdapter { world, params, def, error: None };
    let options = init_create::Options {
        reason: params.mode,
        start_character: params.byte_51 != 0,
        nonblocking: params.byte_52 != 0,
    };
    let result = init_create::initialize(Handle::Actor(actor), params, options, &mut adapter, pc);
    if let Some(e) = adapter.error.take() {
        return Err(e);
    }
    match result {
        Ok(v) => Ok(v),
        // 4CFB00: 404ED0 (GActor.cpp 0x5E8), the new-component list is freed
        // (763B90) and 4D21A0 returns false: InitCreate fails natively.
        Err(init_create::Error::MissingComponent { slot, required }) => {
            adapter.world.log(Log::MissingComponent {
                ids_utf16: def.ids_utf16.clone(),
                slot: slot.index() as u32,
                required: required.index() as u32,
            })?;
            Ok(false)
        }
        Err(e) => Err(Error::World(e.to_string())),
    }
}


// Field-view adapter retains the old public World API and source definitions.
// It never constructs, clones or moves a LoadShell.
struct ExistingFields<'a,W:World>{world:&'a mut W,def:&'a ActorDefinition<W::Definition,W::Layout>}
impl<W:World> crate::actor_init_existing::World<W::Definition,W::Layout> for ExistingFields<'_,W>{
 type Actor=W::Actor;
 fn word(&self,a:W::Actor,off:u32)->Result<u32>{let s=self.world.shell(a)?;Ok(match off{
  8=>s.thing.flags,0x20|0x24|0x28=>s.thing.position_bits[((off-0x20)/4)as usize],0x2c|0x30=>s.thing.facing_bits[((off-0x2c)/4)as usize],
  0x38=>s.object.as_ref().ok_or(Error::Unsupported("GObject fields"))?.associated_handle,
  0x3c=>s.object.as_ref().ok_or(Error::Unsupported("GObject fields"))?.scale_bits,
  0x44=>s.object.as_ref().ok_or(Error::Unsupported("GObject fields"))?.word_44,
  0x48=>s.object.as_ref().ok_or(Error::Unsupported("GObject fields"))?.word_48,
  0x50=>s.object.as_ref().map_or(0,|o|o.words_50_to_54[0]),_=>word(s,off),})}
 fn set_word(&mut self,a:W::Actor,off:u32,v:u32)->Result<()>{let s=self.world.shell_mut(a)?;match off{
  0x20|0x24|0x28=>s.thing.position_bits[((off-0x20)/4)as usize]=v,
  0x38=>s.object.as_mut().ok_or(Error::Unsupported("GObject fields"))?.associated_handle=v,
  0x44=>s.object.as_mut().ok_or(Error::Unsupported("GObject fields"))?.word_44=v,
  0x48=>s.object.as_mut().ok_or(Error::Unsupported("GObject fields"))?.word_48=v,
  _=>set_word(s,off,v)?,}Ok(())}
 fn clear_byte_4c(&mut self,a:W::Actor)->Result<()>{self.world.shell_mut(a)?.object.as_mut().ok_or(Error::Unsupported("GObject fields on a non-object shell"))?.byte_4c=0;Ok(())}
 fn definition_height_bits(&self,_:W::Actor)->Result<u32>{Ok(self.def.height_bits)}
 fn shape_a(&self,_:W::Actor)->Result<Shape>{Ok(self.def.shape_a)}
 fn tile_size_bits(&self)->Result<u32>{Ok(self.world.tile_size_bits())}
 fn tile_shift(&self)->Result<u32>{Ok(self.world.tile_shift())}
 fn diagnostic(&mut self,_:W::Actor,d:crate::actor_init_existing::Diagnostic)->Result<()>{
  use crate::actor_init_existing::Diagnostic as D;
  self.world.log(match d{
   D::Thin=>Log::NotThickEnough{ids_utf16:self.def.ids_utf16.clone()},
   D::Placement{code,xy}=>Log::PlacementFailed{ids_utf16:self.def.ids_utf16.clone(),code,reason:placement_reason(code),position_bits:xy},
   D::DoesntFit{xy}=>Log::ThingDoesntFit{display_utf16:self.def.display_utf16.clone(),position_bits:xy},
   D::RequiredDead=>Log::RequiredObjectDead{ids_utf16:self.def.ids_utf16.clone()},
   D::RequiredIgnored=>Log::RequiredObjectIgnored{ids_utf16:self.def.ids_utf16.clone()},
   D::RequiredPlaced(required)=>Log::RequiredObjectPlaced{ids_utf16:self.def.ids_utf16.clone(),required},})
 }
 fn snap(&mut self,_:W::Actor,shape:&Shape,xy:[u32;2],tile:u32)->Result<[u32;2]>{self.world.snap(shape,xy,tile)}
 fn placement_test(&mut self,a:W::Actor,p:&ActorParams<W::Definition,W::Layout>)->Result<u32>{self.world.placement_test(a,p)}
 fn map_mode(&self)->Result<u32>{Ok(self.world.map_mode())}
 fn spatial_membership(&mut self,a:W::Actor)->Result<()>{index_membership(a,self.world)}
 fn terrain_height(&mut self,xy:[u32;2],f:u32)->Result<Binary>{self.world.terrain_height(xy,f)}
 fn memory_refresh(&mut self,a:W::Actor)->Result<()>{if self.world.shell(a)?.object.as_ref().map_or(0,|o|o.memory.count())>0{Err(Error::Unsupported("4EB490/4EBB80 for an object with +58 > 0"))}else{Ok(())}}
 fn required_object_enabled(&self,_:W::Actor)->Result<bool>{Ok(self.def.required_object)}
 fn find_required_object(&mut self,_:W::Actor,xy:[u32;2])->Result<Option<u32>>{self.world.find_required_object(&self.def.key,xy)}
 fn has_component(&self,a:W::Actor,s:ComponentSlot)->Result<bool>{has(self.world,a,s)}
 fn component_after_init(&mut self,a:W::Actor,s:ComponentSlot)->Result<()>{self.world.component_vf14(a,s)}
 fn global_property(&self)->Result<Option<u32>>{Ok(self.world.global_property())}
 fn needs_settlement_zone(&self,_:W::Actor)->Result<bool>{Ok(self.def.settlement_space)}
 fn create_zone2(&mut self,a:W::Actor)->Result<bool>{self.world.create_zone(a,2)}
 fn discovery_radius(&self,pc:Precision)->Result<u32>{zone_radius(self.world,pc)}
 fn move_zone2(&mut self,a:W::Actor,xy:[u32;2],r:u32)->Result<()>{self.world.move_zone(a,2,xy,r)}
 fn sai_present(&self)->Result<bool>{Ok(self.world.ui_agent()!=UiAgent::Absent)}
 fn tracks(&self,_:W::Actor)->Result<Vec<TrackInfo>>{Ok(self.def.tracks.clone())}
 fn state_low(&self,a:W::Actor)->Result<u32>{Ok(state_bits(self.world,a)?.low)}
 fn existing_fx(&mut self,a:W::Actor)->Result<()>{self.world.creation_fx(a,FxList::Existing)}
fn blocking_notify(&mut self,a:W::Actor)->Result<()>{self.world.blocking_notify(a)}
fn tile_footprint(&mut self,a:W::Actor)->Result<()>{self.world.tile_footprint(a)}
fn world_list_insert(&mut self,a:W::Actor)->Result<()>{self.world.world_list_insert(a)}
fn visibility_refresh(&mut self,a:W::Actor)->Result<()>{self.world.visibility_refresh(a)}
fn gtid_live(&self,id:u32)->Result<bool>{self.world.gtid_live(id)}
fn required_placement_blocked(&mut self,r:u32,a:W::Actor)->Result<bool>{self.world.required_placement_blocked(r,a)}
fn required_object_attach(&mut self,r:u32,a:W::Actor)->Result<()>{self.world.required_object_attach(r,a)}
fn component_init_existing(&mut self,a:W::Actor,s:ComponentSlot)->Result<bool>{self.world.component_init_existing(a,s)}
fn property_applies(&mut self,a:W::Actor,p:u32)->Result<bool>{self.world.property_applies(a,p)}
fn properties_add(&mut self,a:W::Actor,p:u32)->Result<()>{self.world.properties_add(a,p)}
fn zone_count(&self,a:W::Actor)->Result<i32>{self.world.zone_count(a)}
fn sai_notify(&mut self,a:W::Actor)->Result<()>{self.world.sai_notify(a)}
fn track_settings(&self)->Result<(u32,i32)>{self.world.track_settings()}
fn track_updatable(&mut self,a:W::Actor,p:i32)->Result<()>{self.world.track_updatable(a,p)}
fn refresh_4d4cf0(&mut self,a:W::Actor)->Result<()>{self.world.refresh_4d4cf0(a)}
fn refresh_4d1350(&mut self,a:W::Actor)->Result<()>{self.world.refresh_4d1350(a)}
fn refresh_4d67f0(&mut self,a:W::Actor)->Result<()>{self.world.refresh_4d67f0(a)}
fn refresh_4d4130(&mut self,a:W::Actor)->Result<()>{self.world.refresh_4d4130(a)}
fn organization_refresh(&mut self,a:W::Actor)->Result<()>{self.world.organization_refresh(a)}
fn settlement_refresh(&mut self,a:W::Actor)->Result<()>{self.world.settlement_refresh(a)}
fn performer_refresh(&mut self,a:W::Actor)->Result<()>{self.world.performer_refresh(a)}
fn denizen_refresh(&mut self,a:W::Actor)->Result<()>{self.world.denizen_refresh(a)}
}
// ---------------------------------------------------------- InitExisting
/// GObject InitExisting `4EBE50` for an actor (GACD block): object +4C byte
/// = 0, +38 = f32(+3C x definition +24), the thickness warning (`4EE700`),
/// placement `4EFA40`, blocking notification `4EB6E0`, tile footprint
/// `4EA150`, tile coordinates +44 / +48, world list `4F1F70`, virtual 8C
/// (`4EA3D0`), visibility `4EBC70`.
pub fn object_init_existing<W: World>(
    actor: W::Actor,
    params: &mut ActorParams<W::Definition, W::Layout>,
    def: &ActorDefinition<W::Definition, W::Layout>,
    world: &mut W,
    pc: Precision,
) -> Result<bool> {
    crate::actor_init_existing::object_initialize(actor,params,&mut ExistingFields{world,def},pc)
}

/// `4D1FD0` (KKC_GActor::InitRequiredObject).
pub fn init_required_object<W: World>(
    actor: W::Actor,
    def: &ActorDefinition<W::Definition, W::Layout>,
    world: &mut W,
) -> Result<bool> {
    crate::actor_init_existing::required_object_initialize(actor,&mut ExistingFields{world,def})
}

/// `4BB000` ([`crate::component_virtuals::discovery_zone_radius`]).
fn zone_radius<W: World>(world: &W, pc: Precision) -> Result<u32> {
    Ok(crate::component_virtuals::discovery_zone_radius(
        world.tile_size_bits(),
        world.thing_manager_74_bits()?,
        pc,
    )?)
}

/// The TrackInfo period of `4D7230`: the largest length of the kind-1
/// entries (`fcomp` ordered; starts at +0.0), `ftol(floor(f64(setting x
/// largest + 0.5)))` (CRT floor 96D5E0, SSE2 path) divided (`idiv`) by the
/// divisor setting, plus 1. None when no entry has kind 1 (no updatable;
/// the division still executes natively).
pub fn track_period(tracks: &[TrackInfo], scale_bits: u32, divisor: i32, pc: Precision) -> Result<Option<i32>> {
    if tracks.is_empty() {
        return Ok(None);
    }
    let mut any = false;
    let mut largest = 0u32;
    for t in tracks {
        if t.kind == 1 {
            any = true;
            let v = f32::from_bits(t.length_bits);
            let current = f32::from_bits(largest);
            if !v.is_nan() && current < v {
                largest = t.length_bits;
            }
        }
    }
    let product = f32b(scale_bits)?.mul(&f32b(largest)?, pc)?.add(&f32b(0x3f00_0000)?, pc)?;
    let stored = aa_numeric::crt_pow::store_binary64(&product)
        .map_err(|_| Error::Domain("fstp qword of the TrackInfo product"))?;
    let floored = f64::from_bits(stored).floor();
    if !(floored >= i32::MIN as f64 && floored < 2_147_483_648.0) {
        return Err(Error::Domain("_ftol2 of the TrackInfo period outside i32"));
    }
    let value = floored as i32;
    if divisor == 0 {
        return Err(Error::DivideByZero("4D7230 TrackInfo idiv"));
    }
    let quotient = value
        .checked_div(divisor)
        .ok_or(Error::Domain("4D7230 TrackInfo idiv overflow (#DE)"))?;
    Ok(any.then_some(quotient.wrapping_add(1)))
}

/// InitExisting `4D7230(params)` for a created actor (the PostLoad caller
/// with a null block is not this path).
pub fn init_existing<W: World>(
    actor: W::Actor,
    params: &mut ActorParams<W::Definition, W::Layout>,
    def: &ActorDefinition<W::Definition, W::Layout>,
    world: &mut W,
    pc: Precision,
) -> Result<bool> {
    crate::actor_init_existing::initialize(actor,params,&mut ExistingFields{world,def},pc)
}

// ------------------------------------------------------------ PostCreate
struct PostAdapter<'a, W: World> {
    world: &'a mut W,
    actor: W::Actor,
    def: &'a ActorDefinition<W::Definition, W::Layout>,
    error: Option<Error>,
}
impl<W: World> PostAdapter<'_, W> {
    fn fail(&mut self, e: Error) -> world_publication::Error {
        let message = e.to_string();
        self.error.get_or_insert(e);
        world_publication::Error::World(message)
    }
    fn check(&self, id: ActorId) -> world_publication::Result<()> {
        let gtid = self.world.gtid(self.actor).map_err(|e| world_publication::Error::World(e.to_string()))?;
        if gtid == id.raw() {
            Ok(())
        } else {
            Err(world_publication::Error::Unsupported("PostCreate callback on another actor"))
        }
    }
}
type PR<T> = world_publication::Result<T>;
impl<W: World> world_publication::CreateWorld for PostAdapter<'_, W> {
    type Record = ActorParams<W::Definition, W::Layout>;
    fn has_component(&self, actor: ActorId, slot: ComponentSlot) -> PR<bool> {
        self.check(actor)?;
        has(&*self.world, self.actor, slot).map_err(|e| world_publication::Error::World(e.to_string()))
    }
    fn map_mode_a8(&self) -> PR<u32> {
        Ok(self.world.map_mode())
    }
    fn object_ground_post_create(&mut self, actor: ActorId, record: &Self::Record) -> PR<()> {
        self.check(actor)?;
        let r = self.world.ground_paint(self.actor, record);
        r.map_err(|e| self.fail(e))
    }
    fn component_post_create(&mut self, actor: ActorId, slot: ComponentSlot, record: &Self::Record, flag: u32) -> PR<()> {
        self.check(actor)?;
        let r = self.world.component_post_create(self.actor, slot, record, flag);
        r.map_err(|e| self.fail(e))
    }
    fn stored_owner(&self, actor: ActorId) -> PR<Option<ActorId>> {
        self.check(actor)?;
        let owner = self
            .world
            .shell(self.actor)
            .map(|s| word(s, 0xe8))
            .map_err(|e| world_publication::Error::World(e.to_string()))?;
        Ok((owner != 0).then_some(ActorId::from_raw(owner)))
    }
    fn owner_post_create_4e6180(&mut self, owner: ActorId, actor: ActorId) -> PR<()> {
        self.check(actor)?;
        let r = self.world.kingdom_add_actor(owner.raw(), self.actor);
        r.map_err(|e| self.fail(e))
    }
    fn definition_mask(&self, actor: ActorId) -> PR<u32> {
        self.check(actor)?;
        Ok(self.def.component_mask)
    }
    fn city_predicate_45d6a0(&mut self, actor: ActorId) -> PR<bool> {
        self.check(actor)?;
        let r = zone_radius_positive(self.def);
        r.map_err(|e| self.fail(e))
    }
    fn city_argument_34(&self, record: &Self::Record) -> PR<u32> {
        Ok(record.word_34)
    }
    fn city_post_create_5644f0(&mut self, actor: ActorId, argument: u32) -> PR<()> {
        self.check(actor)?;
        let r = self.world.city_create(self.actor, argument);
        r.map_err(|e| self.fail(e))
    }
}

/// PostCreate `4CF300(params, flag)`: [`world_publication::post_create`].
pub fn post_create<W: World>(
    actor: W::Actor,
    params: &ActorParams<W::Definition, W::Layout>,
    def: &ActorDefinition<W::Definition, W::Layout>,
    flag: u32,
    world: &mut W,
) -> Result<()> {
    let id = ActorId::from_raw(world.gtid(actor)?);
    let mut adapter = PostAdapter { world, actor, def, error: None };
    let r = world_publication::post_create(&mut adapter, id, params, flag);
    if let Some(e) = adapter.error.take() {
        return Err(e);
    }
    r.map_err(|e| Error::World(e.to_string()))
}

// ---------------------------------------------------------- registration
/// `4F9510` for a GActor: flag bit 31 cleared; a non-empty name replaces its
/// instance-map entry (`4154D0` found -> `5C9180`; then `5043F0` +
/// `4F7B70`); virtuals BC / C0 / AC are false and B4 is true: `4D62F0`
/// (every registered state active on the actor re-entered through `4D4A90`,
/// then `4CFEA0`); then `4F4C40(actor, 1)`.
pub fn register<W: World>(
    actor: W::Actor,
    def: &ActorDefinition<W::Definition, W::Layout>,
    world: &mut W,
) -> Result<()> {
    world.shell_mut(actor)?.thing.flags &= 0x7fff_ffff;
    let name = world.shell(actor)?.thing.name_utf16.clone();
    if !name.is_empty() {
        if world.instance_name_gtid(&name)?.is_some() {
            world.instance_name_remove(&name)?;
        }
        let gtid = world.gtid(actor)?;
        world.instance_name_insert(&name, gtid)?;
    }
    struct Registration<'a,W:World>{world:&'a mut W, actor:W::Actor}
    impl<W:World> crate::actor_registration::World for Registration<'_,W>{
        fn registered_state_count(&mut self)->Result<i32>{Ok(self.world.registered_state_count() as i32)}
        fn state_bits(&mut self)->Result<crate::components::StateBits>{state_bits(self.world,self.actor)}
        fn state_reenter(&mut self,index:u32)->Result<()>{self.world.state_reenter(self.actor,index)}
        fn grid_register(&mut self)->Result<()>{self.world.grid_register(self.actor)}
    }
    crate::actor_registration::register(&mut Registration{world,actor})?;
    // 4F4C40(actor, 1): virtual B0 (true) and definition virtual +24 (+34);
    // else virtual B4 (true) and the City component, unless the live +158
    // object's definition virtual +24 is set.
    let insert = if def.settlement_space {
        true
    } else if !has(world, actor, ComponentSlot::City)? {
        false
    } else {
        let required = word(world.shell(actor)?, 0x158);
        !(world.gtid_live(required)? && world.object_settlement_space(required)?)
    };
    if insert {
        let gtid = world.gtid(actor)?;
        if !world.latch(0xb3cc40) && world.available_contains(gtid)? {
            let latch = world.assertion_dialog("!m_available_cities.Contains(_thing)", 0x1403)?;
            world.set_latch(0xb3cc40, latch);
        }
        world.add_available(gtid)?;
    }
    Ok(())
}

// ------------------------------------------------------------ 4F9C10
struct CreateAdapter<'a, W: World> {
    world: &'a mut W,
    pc: Precision,
    params: ActorParams<W::Definition, W::Layout>,
    def: ActorDefinition<W::Definition, W::Layout>,
    error: Option<Error>,
}
impl<W: World> CreateAdapter<'_, W> {
    fn fail(&mut self, e: Error) -> world_creation::Error {
        let message = e.to_string();
        self.error.get_or_insert(e);
        world_creation::Error::World(message)
    }
}
type CR<T> = world_creation::Result<T>;
impl<W: World> CreateWorld<W::Layout, ()> for CreateAdapter<'_, W> {
    type Object = W::Actor;
    fn instance_name_gtid(&mut self, name: &[u16]) -> CR<Option<u32>> {
        let r = self.world.instance_name_gtid(name);
        r.map_err(|e| self.fail(e))
    }
    fn gtid_live(&self, gtid: u32) -> CR<bool> {
        self.world.gtid_live(gtid).map_err(|e| world_creation::Error::World(e.to_string()))
    }
    fn log(&mut self, line: u32, format: &'static str, argument: &str) -> CR<()> {
        let r = self.world.log(Log::Create { line, format, argument: argument.to_owned() });
        r.map_err(|e| self.fail(e))
    }
    fn construct(&mut self, class: u32, _: u32, _: &CreateRequest<W::Layout, ()>) -> CR<Option<W::Actor>> {
        if class != 3 {
            return Err(self.fail(Error::Unsupported(
                "4F9C10 class constructor other than 3 (classes 2/9: aa-mapworld fresh_creation)",
            )));
        }
        let mut params = self.params.clone();
        let r = construct(&mut params, &self.def, self.world, self.pc);
        self.params = params;
        r.map(Some).map_err(|e| self.fail(e))
    }
    fn set_state(&mut self, object: W::Actor, state: u8, enabled: bool) -> CR<()> {
        let r = self.world.set_state(object, u32::from(state), enabled);
        r.map_err(|e| self.fail(e))
    }
    fn create_component(&mut self, object: W::Actor, slot: ComponentSlot) -> CR<()> {
        let r = match init_create::factory_kind(slot, self.def.component_mask) {
            Some(kind) => self.world.construct_component(object, slot, kind),
            None => Ok(()),
        };
        r.map_err(|e| self.fail(e))
    }
    fn definition_flags(&self, _: W::Actor) -> CR<u32> {
        Ok(self.def.component_mask)
    }
    fn construction_assert_latch(&self) -> CR<bool> {
        Ok(self.world.latch(0xb3cc42))
    }
    fn set_construction_assert_latch(&mut self, value: bool) -> CR<()> {
        self.world.set_latch(0xb3cc42, value);
        Ok(())
    }
    fn state_active(&mut self, object: W::Actor, state: u8) -> CR<bool> {
        // 432E90: (1 << state) & the 64-bit +100/+104 word.
        let bits = match state_bits(&*self.world, object) {
            Ok(b) => b,
            Err(e) => return Err(self.fail(e)),
        };
        let word = u64::from(bits.low) | (u64::from(bits.high) << 32);
        Ok(word & 1u64.wrapping_shl(u32::from(state) & 63) != 0)
    }
    fn assertion_dialog(&mut self, expression: &'static str, line: u32) -> CR<bool> {
        let r = self.world.assertion_dialog(expression, line);
        r.map_err(|e| self.fail(e))
    }
    fn set_organization_layout(&mut self, object: W::Actor, layout: &W::Layout) -> CR<()> {
        let r = self.world.organization_set_layout(object, layout);
        r.map_err(|e| self.fail(e))
    }
    fn init_create(&mut self, object: W::Actor, _: &CreateRequest<W::Layout, ()>) -> CR<bool> {
        let params = self.params.clone();
        let r = init_create(object, &params, &self.def, self.world, self.pc);
        r.map_err(|e| self.fail(e))
    }
    fn init_existing(&mut self, object: W::Actor, _: &CreateRequest<W::Layout, ()>) -> CR<bool> {
        let mut params = self.params.clone();
        let r = init_existing(object, &mut params, &self.def, self.world, self.pc);
        self.params = params;
        r.map_err(|e| self.fail(e))
    }
    fn post_create(&mut self, object: W::Actor, _: &CreateRequest<W::Layout, ()>, flag: u32) -> CR<()> {
        let params = self.params.clone();
        let r = post_create(object, &params, &self.def, flag, self.world);
        r.map_err(|e| self.fail(e))
    }
    fn register(&mut self, object: W::Actor) -> CR<()> {
        let r = register(object, &self.def, self.world);
        r.map_err(|e| self.fail(e))
    }
    fn register_updatable(&mut self, object: W::Actor) -> CR<()> {
        if self.world.scene_present() {
            let r = self.world.scene_add(object);
            r.map_err(|e| self.fail(e))?;
        }
        Ok(())
    }
    fn fires_creation_fx(&mut self, _: W::Actor) -> CR<bool> {
        // GActor virtual B0: 6A71D0.
        Ok(true)
    }
    fn creation_fx(&mut self, object: W::Actor) -> CR<()> {
        let r = self.world.creation_fx(object, FxList::Created);
        r.map_err(|e| self.fail(e))
    }
    fn destroy(&mut self, object: W::Actor) -> CR<()> {
        let r = self.world.destroy(object);
        r.map_err(|e| self.fail(e))
    }
}

/// `4F9C10` (KKC_GWorld::CreateGThing) on an A25B28 actor block. `Ok(None)`
/// where the original returns null; `params` receives the block's native
/// write-backs (direction check, owner cleared without flag bit 23,
/// position / facing of `4EFA40`).
pub fn create<W: World>(
    params: &mut ActorParams<W::Definition, W::Layout>,
    world: &mut W,
    pc: Precision,
) -> Result<Option<W::Actor>> {
    let def = world.definition(&params.definition)?;
    let request = CreateRequest {
        class: def.runtime_class,
        definition_ids: String::from_utf16_lossy(&def.ids_utf16),
        instance_name: params.name_utf16.clone(),
        actor_data: true,
        kingdom_data: false,
        organization_layout: params.organization_layout.clone(),
        mode: params.mode,
        byte_50: params.byte_50 != 0,
        payload: (),
    };
    let mut adapter = CreateAdapter { world, pc, params: params.clone(), def, error: None };
    let result = world_creation::create(&request, &mut adapter);
    *params = adapter.params.clone();
    if let Some(e) = adapter.error.take() {
        return Err(e);
    }
    match result {
        Ok(v) => Ok(v),
        Err(world_creation::Error::NullDereference(what)) => Err(Error::NullDereference(what)),
        Err(world_creation::Error::World(what)) => Err(Error::World(what)),
    }
}

/// CreateGActor `4FA030`: `4F9C10`, then the IsGActor assertion (latch
/// B3CC43, GWorld.cpp 0x85E). Every object this module creates is a GActor
/// (virtual B4 `6A71D0`), so the dialog is unreachable here.
pub fn create_gactor<W: World>(
    params: &mut ActorParams<W::Definition, W::Layout>,
    world: &mut W,
    pc: Precision,
) -> Result<Option<W::Actor>> {
    create(params, world, pc)
}

/// CreateOrganization `4FA150`: for an ActorData definition with an
/// Organization (+174 bit 7), the layout's first slot must pass `578290`
/// (its position becomes the block position (x, y, 0)); `4F9C10`; the
/// IsGActor assertion (B3CC44, 0x880, unreachable here); the leader slot
/// spawned first with flag 0, then every other defined slot with params
/// +51 (`57AD30`); no member (`57A730` == 0) logs 0x8A0 and destroys.
pub fn create_organization<W: World>(
    params: &mut ActorParams<W::Definition, W::Layout>,
    world: &mut W,
    pc: Precision,
) -> Result<Option<W::Actor>> {
    let def = world.definition(&params.definition)?;
    // Definition virtual 38 (IsActorData: class 3 definitions) and +174 bit 7.
    if def.runtime_class == 3 && def.component_mask & (1 << 7) != 0 {
        let Some(layout) = params.organization_layout.clone().or_else(|| def.layout.clone()) else {
            return Err(Error::NullDereference("4FA150: [layout+8] of a null layout"));
        };
        if world.layout_slot_count(&layout)? > 0 {
            let slot = world.layout_first_slot(&layout)?;
            let mut xy = [params.position_bits[0], params.position_bits[1]];
            if !world.layout_slot_placement(&layout, slot, &mut xy, params.direction_bits)? {
                return Ok(None);
            }
            // 4BE740: (x, y, 0).
            params.position_bits = [xy[0], xy[1], 0];
        }
    }
    let Some(actor) = create(params, world, pc)? else {
        return Ok(None);
    };
    if !has(world, actor, ComponentSlot::Organization)? {
        return Ok(Some(actor));
    }
    let leader = world.organization_leader(actor)?;
    if leader >= 0 && world.organization_slot_defined(actor, leader)? {
        world.spawn_member(actor, leader, 0)?;
    }
    let mut slot = 0;
    while slot < world.organization_slot_count(actor)? {
        if slot != leader && world.organization_slot_defined(actor, slot)? {
            world.spawn_member(actor, slot, params.byte_51)?;
        }
        slot += 1;
    }
    if world.member_count(actor)? == 0 {
        let gtid = world.gtid(actor)?;
        world.log(Log::NoMembers { gtid })?;
        world.destroy(actor)?;
        return Ok(None);
    }
    Ok(Some(actor))
}
