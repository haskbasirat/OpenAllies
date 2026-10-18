//! Original metagame definitions loaded from the original TGI sources.
//!
//! Template/list inheritance is resolved by the shared aa-rules resolver
//! (`ActorCatalog`); this module ports the metagame-specific native loaders on
//! top of it: which fields are read, in which order, their types, list order,
//! lookups (all ASCII case-insensitive, first match) and the loader
//! validations. Native fatal data errors (`FUN_00410C40` throws) become
//! [`Error::NativeData`] with the native message text.
//!
//! | native | loader |
//! |---|---|
//! | 958AE0 | MGMap territory (type enum 958700, spacing enum 959A50, land/water 9587E0) |
//! | 9583C0 / 9625E0 | territory link add / link load |
//! | 95D930 | MGMap (two passes, offset/scale 9579A0/957A20, alias pairing) |
//! | 95D5C0 | links to alias territories rejected |
//! | 93B250 | MetaGameComponent of an army Thing |
//! | 941240 | MGDivision |
//! | 960510 / 960120 / 95FD70 | MGAI / per-map AI / territory AI |
//! | 949B60 (+9536E0 built-ins) | MGCommand |
//! | 95CCC0 | DynamicCampaign (+95C840 kingdoms, 95C9B0 teams, 95CAD0 events, 95C5D0 ownership) |
//! | 95EB00 / 95E320 / 95E740 | campaign kingdom / liberates_to pass / territory records |
//! | 962290 (+961D90/961B10/962010, qsort 96CE20 with 961790) | shop lists, sorted by cost |
//! | 968000 / 967C10 (+967A40 enum) | events / effects |
//! | 968250 | objective |
//!
//! Not validated here (other subsystems own those registries): textures,
//! cinematics/video paths, VFX, cursors, nation/color/SAI managers, RMC map
//! sizes. Localized fields (`name`, `tooltip`, `message`, ...) are read with the
//! native getter 5B5620 at their native positions and kept as authored keys
//! (the load-time string-table lookup 4B9F40 is not part of this crate).
//! Error precedence between different data managers is not reproduced; within
//! one loader the native field order is.
mod fields;

use crate::battle::DestroyBehavior;
use crate::qsort::msvc_qsort;
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};
use aa_rules::actors::{ActorCatalog, ResolvedNode};
use aa_rules::sources::V14Sources;
use fields::{
    boolean, fixed, fixed_or, int, localized, opt_boolean, opt_point, opt_string, point2_as3, string, subnode,
};
use serde::{Deserialize, Serialize};

const ONE_F32: u32 = 0x3f80_0000;

fn native(message: String) -> Error {
    Error::NativeData(message)
}
fn unknown_ids(ids: &str, manager: &str) -> Error {
    native(format!("Unknown IDS '{ids}' for data manager '{manager}'"))
}
fn eq(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Case-insensitive enum lookup (`FUN_00497220` on a registered table).
fn enum_value<T: Copy>(table: &[(&str, T)], raw: &str) -> Result<T> {
    table
        .iter()
        .find(|(name, _)| eq(name, raw))
        .map(|(_, v)| *v)
        .ok_or_else(|| native(format!("Unknown IDS '{raw}' specified in enum table")))
}

// ---------------------------------------------------------------- settings

/// SVars metagame values read through settings `+0x208` / `+0x20C`
/// (registered by 5065D0).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuickBattleSettings {
    /// `MetaGameQuickBattleTechnologyBonus`, binary32 bits.
    pub technology_bonus_bits: u32,
    /// `MetaGameQuickBattleAttackWinProbability`, binary32 bits.
    pub attack_win_probability_bits: u32,
}

/// A `ResourceList` node read by `4CDB30`: one optional fixed field per
/// resource type, default 0.0. Keyed by resource IDS (aa-rules declaration
/// order; the native resource registry order is not asserted).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceValues {
    pub values: Vec<(String, u32)>,
}

// ---------------------------------------------------------------- map

/// Territory type enum (958700): value, name, flag0, flag1 (flag1 gates links).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerritoryKind {
    Unknown = 0,
    Normal = 1,
    Neutral = 2,
    Background = 3,
    Alias = 4,
}
impl TerritoryKind {
    pub const TABLE: [(&'static str, TerritoryKind); 5] = [
        ("unknown", Self::Unknown),
        ("normal", Self::Normal),
        ("neutral", Self::Neutral),
        ("background", Self::Background),
        ("alias", Self::Alias),
    ];
    /// Payload byte 0 (read by 957E90; runtime meaning unproven).
    pub fn flag0(self) -> bool {
        matches!(self, Self::Normal | Self::Neutral | Self::Alias)
    }
    /// Payload byte 1 (957EB0): the territory's Link children are parsed.
    pub fn has_links(self) -> bool {
        matches!(self, Self::Normal | Self::Alias)
    }
}

/// `actor_spacing_type` enum (959A50).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorSpacing {
    Simple = 0,
    Circle = 1,
    Square = 2,
}
impl ActorSpacing {
    pub const TABLE: [(&'static str, ActorSpacing); 3] =
        [("simple", Self::Simple), ("circle", Self::Circle), ("square", Self::Square)];
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerritoryLink {
    /// Index of the target in [`MapDefinition::territories`].
    pub target: usize,
    /// Read only for NORMAL owners (9625E0); empty otherwise.
    pub rmc_attacker_ids: String,
    /// (x, y, z) binary32 bits after the map offset/scale transform.
    pub intermediate_points: Vec<[u32; 3]>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerritoryDefinition {
    pub ids: String,
    /// Localized `name` (5B5620 key; first field read by 958AE0).
    pub name: String,
    pub kind: TerritoryKind,
    pub object_ids: String,
    /// (x, y, z) binary32 bits after `(raw + offset) * scale` (z: + offset only).
    pub location: [u32; 3],
    pub bounds_min: [u32; 3],
    pub bounds_max: [u32; 3],
    pub actor_spacing_distance_bits: u32,
    pub actor_spacing: ActorSpacing,
    /// `node_land`, `node_water` (9587E0 order).
    pub nodes: [String; 2],
    /// NORMAL only; constructor default -1 otherwise.
    pub minimap_alpha: i32,
    /// NORMAL only; constructor default 0 otherwise.
    pub production: i32,
    pub rmc_ids: String,
    pub rmc_defender_ids: String,
    /// NORMAL/ALIAS only; (0,0,0) appended when none; scaled, not offset.
    pub actor_slots: Vec<[u32; 3]>,
    pub links: Vec<TerritoryLink>,
    /// ALIAS pairing (`+0x44`), set on both territories of an alias link.
    pub alias_partner: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapDefinition {
    pub ids: String,
    /// Localized `name` (47D960).
    pub name: String,
    /// binary32 bits (int field read by the fixed getter, then scaled).
    pub width_bits: u32,
    pub height_bits: u32,
    pub minimap_texture: String,
    pub minimap_blend_amount_bits: u32,
    pub territory_location_offset: [u32; 3],
    pub territory_location_scale_bits: u32,
    pub territories: Vec<TerritoryDefinition>,
}
impl MapDefinition {
    /// `FUN_0095D900`: case-insensitive territory lookup.
    pub fn territory(&self, ids: &str) -> Option<usize> {
        self.territories.iter().position(|t| eq(&t.ids, ids))
    }
}

// ---------------------------------------------------------------- armies / divisions / AI / commands

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DivisionDefinition {
    pub ids: String,
    /// Localized `name` (47D960).
    pub name: String,
    pub icon_location: i32,
    /// Localized `tooltip` and `upgrade_tooltip` (5B5620).
    pub tooltip: String,
    pub upgrade_tooltip: String,
    /// Technology IDS; `None` for an empty authored id (native appends NULL).
    pub technologies: Vec<Option<String>>,
    pub rts_actor_ids: String,
    pub rts_water_actor_ids: String,
    pub upgrade_button_location: i32,
    pub damage_bits: u32,
    /// Indices into [`MetaGameData::divisions`].
    pub prerequisites: Vec<usize>,
}

/// A Thing with a `MetaGameComponent` (an army type), 93B250.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmyDefinition {
    pub ids: String,
    pub movement: i32,
    /// Authored `damage` (`+0x14`, used by the quick-battle strength 93EBD0).
    pub damage_bits: u32,
    /// `+0x18`: damage plus each listed division's damage, binary32 per add.
    pub damage_with_divisions_bits: u32,
    pub division_matching_preference: i32,
    /// Localized `tooltip` of the MetaGameComponent (5B5620).
    pub tooltip: String,
    /// Indices into [`MetaGameData::divisions`].
    pub divisions: Vec<usize>,
    pub capturable: bool,
    pub upgradable: bool,
    pub rts_actor_ids: String,
    pub rts_water_actor_ids: String,
    pub stockpile_bonus: ResourceValues,
    pub water_stockpile_bonus: ResourceValues,
}

/// `95FD70` territory AI record (MGAICapitalTerritoryTemplate / MGAITerritoryTemplate).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerritoryAi {
    /// Map territory index for `Territory` entries; `None` for the four fixed records.
    pub territory: Option<usize>,
    pub additional_importance: i32,
    pub additional_neighbor_proximity_importance_depth: i32,
    pub additional_neighbor_proximity_importance_max: i32,
    pub required_attack_strength_ratio_bits: u32,
    pub desired_defense_threat_ratio_bits: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapAi {
    /// Index into [`MetaGameData::maps`].
    pub map: usize,
    pub default_territory: TerritoryAi,
    pub capital: TerritoryAi,
    pub friendly_capital: TerritoryAi,
    pub enemy_capital: TerritoryAi,
    pub territories: Vec<TerritoryAi>,
}

/// 960510.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiDefinition {
    pub ids: String,
    /// Localized `name` (47D960).
    pub name: String,
    pub additional_attack_occupied_importance: i32,
    pub additional_attack_unoccupied_importance: i32,
    pub additional_defense_importance: i32,
    pub production_importance_multiplier: i32,
    pub no_attack_distance: i32,
    pub max_influence_depth: i32,
    pub additional_border_importance: i32,
    pub additional_border_reduction_importance: i32,
    pub attack_defendability_factor_bits: u32,
    pub actor_distance_influence_multiplier_bits: u32,
    pub actor_distance_strength_multiplier_bits: u32,
    pub goal_cutoff_ratio_bits: u32,
    pub purchase_army_probability_bits: u32,
    pub purchase_tech_probability_bits: u32,
    pub maps: Vec<MapAi>,
}

/// Built-in metagame commands (9536E0), native indices 0..=11; these are the
/// `KKC_MGCommand::Process` (954570) switch values.
pub const MG_COMMANDS: [&str; 12] = [
    "move",
    "attack",
    "startbattle",
    "playbattle",
    "cancelbattle",
    "quickbattle",
    "endturn",
    "purchasetech",
    "purchaseactor",
    "upgradeactor",
    "upgradeactorall",
    "take",
];

/// Authored overrides of a built-in command (949B60).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDefinition {
    pub index: usize,
    /// Localized `name` (47D960) and `tooltip` (5B5620).
    pub name: String,
    pub icon: String,
    pub tooltip: String,
    pub cursor_ids: String,
    pub command_vfx_ids: String,
    /// Present only when authored (`hot_command`); otherwise the built-in default.
    pub hot_command: Option<bool>,
}

// ---------------------------------------------------------------- RMC (battle maps)

/// A kingdom slot of a random-map configuration (template RMCKingdom). Only
/// what the metagame battle code reads; the RMC loader itself (RMCData.cpp) is
/// another subsystem resolved here through the shared aa-rules resolver.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmcKingdom {
    pub ids: String,
    /// `type` (enum 4A1EA0: land 0, water 1), authored text.
    pub kind: String,
    pub independent: bool,
}

/// A metagame battle RMC (root `RMC` nodes referenced by territories).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmcDefinition {
    pub ids: String,
    /// Kingdom slots in resolver list order (template slots first).
    pub kingdoms: Vec<RmcKingdom>,
    /// MapSize list: (width, height) binary32 bits.
    pub map_sizes: Vec<(u32, u32)>,
}

// ---------------------------------------------------------------- campaigns

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShopRow {
    /// Resolved IDS; `None` for an empty technology/division id (native NULL ref).
    pub ids: Option<String>,
    pub cost: i32,
}

/// Shop (962290): each list sorted ascending by cost with the original CRT
/// qsort; `all` holds technologies, actors, divisions (append order) sorted
/// the same way.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shop {
    pub technologies: Vec<ShopRow>,
    pub actors: Vec<ShopRow>,
    pub divisions: Vec<ShopRow>,
    pub all: Vec<ShopRow>,
}

/// Territory record (95E740 + 95E320).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedTerritory {
    /// Map territory index.
    pub territory: usize,
    /// Campaign kingdom index (first IDS match); `None` for an empty value.
    pub liberates_to: Option<usize>,
    /// Actor Thing IDS placed at start, authored order (duplicates allowed).
    pub actors: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignKingdom {
    pub ids: String,
    /// Localized `name` (5B5620; kingdom data `+0x08`).
    pub name: String,
    pub starting_resources: i32,
    pub nation_ids: String,
    /// Map territory index (any territory; ownership is not checked natively).
    pub capital: usize,
    /// Index into [`MetaGameData::ai`]; `None` for an empty id.
    pub mgai: Option<usize>,
    pub color_ids: String,
    pub sai_ids: String,
    pub cinematic_win: String,
    pub nation_selection_screen_background_video: String,
    pub stockpile_bonus: ResourceValues,
    pub water_stockpile_bonus: ResourceValues,
    pub territories: Vec<OwnedTerritory>,
    /// The single required `Objective` (968250).
    pub capital_captures: i32,
    pub shop: Shop,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignTeam {
    pub ids: String,
    /// Campaign kingdom indices (first IDS match), authored order, no dedup.
    pub kingdoms: Vec<usize>,
}

/// Event effect type enum (967A40): value and field flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectKind {
    Unknown = 0,
    AddTech = 1,
    AddActor = 2,
    RemoveTech = 3,
    RemoveActor = 4,
    ChangeTerritory = 5,
}
impl EffectKind {
    pub const TABLE: [(&'static str, EffectKind, u32); 6] = [
        ("unknown", Self::Unknown, 0),
        ("add_tech", Self::AddTech, 0x101),
        ("add_actor", Self::AddActor, 0x11001),
        ("remove_tech", Self::RemoveTech, 0x101),
        ("remove_actor", Self::RemoveActor, 0x1),
        ("change_territory", Self::ChangeTerritory, 0x10011),
    ];
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventEffect {
    pub kind: EffectKind,
    /// Localized `message` (5B5620).
    pub message: String,
    pub kingdom_target: Option<usize>,
    pub kingdom_source: Option<usize>,
    pub territory: Option<usize>,
    pub actor_ids: Option<String>,
    pub tech_ids: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignEvent {
    pub ids: String,
    /// Localized `name` and `message` (5B5620).
    pub name: String,
    pub round: i32,
    pub round_probability_bits: u32,
    pub message: String,
    /// Campaign kingdom index; `None` for an empty id.
    pub kingdom_turn: Option<usize>,
    pub effects: Vec<EventEffect>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Campaign {
    pub ids: String,
    /// Localized `name` (47D960) and `description` (5B5620).
    pub name: String,
    pub description: String,
    /// Index into [`MetaGameData::maps`]; `None` when `map_ids` is empty.
    pub map: Option<usize>,
    pub map_table_ids: String,
    pub initial_year: i32,
    /// Authored month minus one (95CCC0 rejects values outside 0..=11).
    pub initial_month_index: i32,
    pub attacking_destroy_behavior: DestroyBehavior,
    pub defending_destroy_behavior: DestroyBehavior,
    pub territory_friendly_vfx: String,
    pub territory_enemy_vfx: String,
    pub territory_friendly_arrow_vfx: String,
    pub territory_enemy_arrow_vfx: String,
    pub cinematic_intro: String,
    pub kingdoms: Vec<CampaignKingdom>,
    pub teams: Vec<CampaignTeam>,
    pub events: Vec<CampaignEvent>,
    /// Campaign kingdom index.
    pub default_kingdom: usize,
}

/// A Thing reference that could not be checked because its defining source is
/// not parsed by the shared aa-rules TGI parser (see README gaps).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnverifiedReference {
    pub field: String,
    pub ids: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaGameData {
    pub settings: QuickBattleSettings,
    /// Things (any category except METAGAME_ACTOR) the shared resolver could
    /// not resolve; they were not inspected for a MetaGameComponent.
    pub unresolved_things: Vec<String>,
    /// Thing references accepted without verification (defining source unparsed).
    pub unverified_references: Vec<UnverifiedReference>,
    pub maps: Vec<MapDefinition>,
    pub map_tables: Vec<String>,
    pub divisions: Vec<DivisionDefinition>,
    pub armies: Vec<ArmyDefinition>,
    pub ai: Vec<AiDefinition>,
    pub commands: Vec<CommandDefinition>,
    pub campaigns: Vec<Campaign>,
    /// RMCs referenced by map territories (`rmc_ids`), in first-reference order.
    pub rmcs: Vec<RmcDefinition>,
}

// ---------------------------------------------------------------- loader context

struct Ctx<'a> {
    catalog: &'a ActorCatalog,
    precision: Precision,
    resources: &'a [String],
    property_ids: Vec<String>,
    rmc: Vec<(String, Vec<String>, usize)>,
    /// Sources the shared TGI parser rejected (path, bytes).
    opaque: &'a [(String, Vec<u8>)],
    unverified: std::cell::RefCell<Vec<UnverifiedReference>>,
}

enum Thing {
    Missing,
    Unverified,
    Found { registry: u32, kind: u32 },
}

/// `IDS = <ids>` assignment text in a source aa-rules could not parse.
fn ids_text_in(bytes: &[u8], ids: &str) -> bool {
    let text = String::from_utf8_lossy(bytes);
    text.lines().any(|line| {
        let line = line.split(';').next().unwrap_or("").trim();
        let mut parts = line.splitn(2, '=');
        matches!((parts.next(), parts.next()), (Some(k), Some(v)) if eq(k.trim(), "IDS") && eq(v.trim(), ids))
    })
}

impl Ctx<'_> {
    /// Thing lookup `00427BD0` on the object manager. An IDS that is absent
    /// from the catalog but assigned in a source the shared parser rejected is
    /// recorded as unverified instead of being accepted or rejected silently.
    fn thing(&self, field: &str, ids: &str) -> Result<Thing> {
        if self.catalog.find(ids).is_some() {
            let c = self.catalog.native_category(ids)?;
            return Ok(Thing::Found {
                registry: c.registry_index,
                kind: c.native_kind,
            });
        }
        if let Some((path, _)) = self.opaque.iter().find(|(_, b)| ids_text_in(b, ids)) {
            self.unverified.borrow_mut().push(UnverifiedReference {
                field: field.to_owned(),
                ids: ids.to_owned(),
                reason: format!("IDS assigned in {path}, which the shared TGI parser rejects"),
            });
            return Ok(Thing::Unverified);
        }
        Ok(Thing::Missing)
    }
    /// `004BDBA0(ids, category)`: an actor Thing (native kind 2; vtable+0x38,
    /// inferred) of the category registry index (0x2B = any).
    fn actor(&self, field: &str, ids: &str, category: u32) -> Result<bool> {
        Ok(match self.thing(field, ids)? {
            Thing::Found { registry, kind } => kind == 2 && (category == 0x2b || registry == category),
            Thing::Unverified => true,
            Thing::Missing => false,
        })
    }
    fn resource_values(&self, node: &ResolvedNode<'_>) -> Result<ResourceValues> {
        let mut values = Vec::with_capacity(self.resources.len());
        for name in self.resources {
            values.push((name.clone(), fixed_or(node, name, 0, self.precision)?));
        }
        Ok(ResourceValues { values })
    }
    fn property(&self, ids: &str) -> bool {
        self.property_ids.iter().any(|p| eq(p, ids))
    }
}

fn f32_add(a: u32, b: u32, p: Precision) -> Result<u32> {
    Ok(Binary::from_f32_bits(a)?.add(&Binary::from_f32_bits(b)?, p)?.store_f32()?)
}
fn f32_mul(a: u32, b: u32, p: Precision) -> Result<u32> {
    Ok(Binary::from_f32_bits(a)?.mul(&Binary::from_f32_bits(b)?, p)?.store_f32()?)
}

// ---------------------------------------------------------------- map loaders

/// 958AE0 (pass 1). Links are added in pass 2.
fn load_territory(ctx: &Ctx<'_>, node: &ResolvedNode<'_>) -> Result<TerritoryDefinition> {
    let p = ctx.precision;
    let ids = string(node, "ids")?;
    let name = localized(node, "name")?;
    let kind = enum_value(&TerritoryKind::TABLE, &string(node, "type")?)?;
    let location = point2_as3(node, "location", p)?;
    let bounds_min = point2_as3(node, "bounds_min", p)?;
    let bounds_max = point2_as3(node, "bounds_max", p)?;
    let actor_spacing_distance_bits = fixed(node, "actor_spacing_distance", p)?;
    let actor_spacing = enum_value(&ActorSpacing::TABLE, &string(node, "actor_spacing_type")?)?;
    let nodes = [string(node, "node_land")?, string(node, "node_water")?];
    let object_ids = string(node, "object_ids")?;
    match ctx.thing("territory object_ids", &object_ids)? {
        Thing::Missing => {
            return Err(native(format!(
                "Object with IDS '{object_ids}' not found for territory '{ids}'."
            )));
        }
        // vtable+0x30 "is object": native kinds OBJECT(1)/ACTOR(2) (inferred).
        Thing::Found { kind: 0, .. } => {
            return Err(native(format!(
                "Territory '{ids}' expects an object IDS for the object, not a thing IDS ('{object_ids}')."
            )));
        }
        _ => {}
    }
    let (mut minimap_alpha, mut production) = (-1, 0);
    let (mut rmc_ids, mut rmc_defender_ids) = (String::new(), String::new());
    if kind == TerritoryKind::Normal {
        minimap_alpha = int(node, "minimap_alpha")?;
        production = int(node, "production")?;
        rmc_ids = string(node, "rmc_ids")?;
        let Some((_, kingdoms, sizes)) = ctx.rmc.iter().find(|(r, _, _)| eq(r, &rmc_ids)) else {
            return Err(if rmc_ids.is_empty() {
                Error::Unsupported(format!("empty rmc_ids in {ids}: native dereferences NULL"))
            } else {
                unknown_ids(&rmc_ids, "RMC")
            });
        };
        rmc_defender_ids = string(node, "rmc_defender_ids")?;
        // 4A0960: case-sensitive comparison against the RMC kingdoms.
        if !kingdoms.contains(&rmc_defender_ids) {
            return Err(native(format!(
                "RMC defender '{rmc_defender_ids}' is not a valid kingdom in the territory '{ids}'"
            )));
        }
        if *sizes == 0 {
            return Err(native(format!(
                "RMC '{rmc_ids}' has no map sizes, it requires at least 1"
            )));
        }
    }
    let mut actor_slots = Vec::new();
    if matches!(kind, TerritoryKind::Normal | TerritoryKind::Alias) {
        for slot in node.children("ActorSlot")? {
            actor_slots.push(point2_as3(&slot, "location", p)?);
        }
    }
    if actor_slots.is_empty() {
        actor_slots.push([0, 0, 0]);
    }
    Ok(TerritoryDefinition {
        ids,
        name,
        kind,
        object_ids,
        location,
        bounds_min,
        bounds_max,
        actor_spacing_distance_bits,
        actor_spacing,
        nodes,
        minimap_alpha,
        production,
        rmc_ids,
        rmc_defender_ids,
        actor_slots,
        links: Vec::new(),
        alias_partner: None,
    })
}

/// 95D930.
fn load_map(ctx: &Ctx<'_>, node: &ResolvedNode<'_>) -> Result<MapDefinition> {
    let p = ctx.precision;
    let ids = string(node, "ids")?;
    let name = localized(node, "name")?;
    let minimap_texture = string(node, "minimap_texture")?;
    let blend = fixed(node, "minimap_blend_amount", p)?;
    let b = Binary::from_f32_bits(blend)?;
    if b.compare(&Binary::from_f32_bits(ONE_F32)?) == std::cmp::Ordering::Greater
        || b.compare(&Binary::from_u32(0)) == std::cmp::Ordering::Less
    {
        return Err(native("Minimap blend amount must be between 0 and 1".into()));
    }
    let mut width = fixed(node, "width", p)?;
    let mut height = fixed(node, "height", p)?;
    let offset = match opt_point(node, "territory_location_offset", p)? {
        Some(v) if v.len() == 3 => [v[0], v[1], v[2]],
        Some(_) => return Err(Error::Unsupported(format!("{ids} offset is not a point3"))),
        None => [0, 0, 0],
    };
    let mut scale = ONE_F32;
    if node.field("territory_location_scale", p)?.is_some() {
        scale = fixed(node, "territory_location_scale", p)?;
        width = f32_mul(width, scale, p)?;
        height = f32_mul(height, scale, p)?;
    }
    let children = node.children("Territory")?;
    let mut territories = Vec::with_capacity(children.len());
    for child in &children {
        territories.push(load_territory(ctx, child)?);
    }
    // Pass 2: links, alias pairing, then offset and scale.
    for (owner, child) in children.iter().enumerate() {
        let kind = territories[owner].kind;
        if kind.has_links() {
            for link in child.children("Link")? {
                let target_ids = string(&link, "territory_ids")?;
                let Some(target) = territories.iter().position(|t| eq(&t.ids, &target_ids)) else {
                    return Err(native(format!(
                        "Territory link with IDS '{target_ids}' not found."
                    )));
                };
                let mut points = Vec::new();
                for ip in link.children("IntermediatePoint")? {
                    points.push(point2_as3(&ip, "location", p)?);
                }
                let mut rmc_attacker_ids = String::new();
                if kind == TerritoryKind::Normal {
                    rmc_attacker_ids = string(&link, "rmc_attacker_ids")?;
                    let t = &territories[owner];
                    let kingdoms = ctx
                        .rmc
                        .iter()
                        .find(|(r, _, _)| eq(r, &t.rmc_ids))
                        .map(|(_, k, _)| k)
                        .ok_or_else(|| unknown_ids(&t.rmc_ids, "RMC"))?;
                    if !kingdoms.contains(&rmc_attacker_ids) {
                        return Err(native(format!(
                            "RMC attacker '{rmc_attacker_ids}' is not a valid kingdom in the territory '{}'",
                            t.ids
                        )));
                    }
                }
                territories[owner].links.push(TerritoryLink {
                    target,
                    rmc_attacker_ids,
                    intermediate_points: points,
                });
                if kind == TerritoryKind::Alias {
                    if territories[owner].links.len() != 1 {
                        return Err(native(format!(
                            "Territory alias types can only have 1 link: '{}'",
                            territories[owner].ids
                        )));
                    }
                    territories[owner].alias_partner = Some(target);
                    territories[target].alias_partner = Some(owner);
                }
            }
        }
        // 9579A0: location/bounds/link points += offset (x, y, z).
        let t = &mut territories[owner];
        for v in [&mut t.location, &mut t.bounds_min, &mut t.bounds_max] {
            for i in 0..3 {
                v[i] = f32_add(v[i], offset[i], p)?;
            }
        }
        for link in &mut t.links {
            for pt in &mut link.intermediate_points {
                for i in 0..3 {
                    pt[i] = f32_add(pt[i], offset[i], p)?;
                }
            }
        }
        // 957A20: x, y of location/bounds/slots/link points *= scale.
        let scale_xy = |v: &mut [u32; 3]| -> Result<()> {
            for c in v.iter_mut().take(2) {
                *c = f32_mul(*c, scale, p)?;
            }
            Ok(())
        };
        for v in [&mut t.location, &mut t.bounds_min, &mut t.bounds_max] {
            scale_xy(v)?;
        }
        for s in &mut t.actor_slots {
            scale_xy(s)?;
        }
        for link in &mut t.links {
            for pt in &mut link.intermediate_points {
                scale_xy(pt)?;
            }
        }
    }
    // 95D5C0
    for t in &territories {
        for l in &t.links {
            if territories[l.target].kind == TerritoryKind::Alias {
                return Err(native(format!(
                    "Links to alias territories not valid.  {} links to {}.",
                    t.ids, territories[l.target].ids
                )));
            }
        }
    }
    Ok(MapDefinition {
        ids,
        name,
        width_bits: width,
        height_bits: height,
        minimap_texture,
        minimap_blend_amount_bits: blend,
        territory_location_offset: offset,
        territory_location_scale_bits: scale,
        territories,
    })
}

// ---------------------------------------------------------------- divisions / armies / AI / commands

/// 941240 (division prerequisites resolved after all divisions are registered).
fn load_division(ctx: &Ctx<'_>, node: &ResolvedNode<'_>) -> Result<(DivisionDefinition, Vec<String>)> {
    let ids = string(node, "ids")?;
    let name = localized(node, "name")?;
    let icon_location = int(node, "icon_location")?;
    let tooltip = localized(node, "tooltip")?;
    let upgrade_tooltip = localized(node, "upgrade_tooltip")?;
    let mut technologies = Vec::new();
    for t in node.children("Technology")? {
        let tid = string(&t, "ids")?;
        if tid.is_empty() {
            technologies.push(None);
        } else if ctx.property(&tid) {
            technologies.push(Some(tid));
        } else {
            return Err(unknown_ids(&tid, "Property"));
        }
    }
    let rts_actor_ids = string(node, "rts_actor_ids")?;
    let rts_water_actor_ids = string(node, "rts_water_actor_ids")?;
    for r in [&rts_actor_ids, &rts_water_actor_ids] {
        if !r.is_empty() && !ctx.actor("division rts actor", r, 0x2b)? {
            return Err(native(format!(
                "RTS Actor IDS '{r}' is not a valid actor for the MG division data"
            )));
        }
    }
    let upgrade_button_location = int(node, "upgrade_button_location")?;
    let damage_bits = fixed(node, "damage", ctx.precision)?;
    let prereq_ids = node
        .children("DivisionPreReq")?
        .iter()
        .map(|r| string(r, "ids"))
        .collect::<Result<Vec<_>>>()?;
    Ok((
        DivisionDefinition {
            ids,
            name,
            icon_location,
            tooltip,
            upgrade_tooltip,
            technologies,
            rts_actor_ids,
            rts_water_actor_ids,
            upgrade_button_location,
            damage_bits,
            prerequisites: Vec::new(),
        },
        prereq_ids,
    ))
}

/// 93B250.
fn load_army(
    ctx: &Ctx<'_>,
    ids: &str,
    c: &ResolvedNode<'_>,
    divisions: &[DivisionDefinition],
) -> Result<ArmyDefinition> {
    let p = ctx.precision;
    let movement = int(c, "movement")?;
    let damage_bits = fixed(c, "damage", p)?;
    let division_matching_preference = int(c, "division_matching_preference")?;
    let tooltip = localized(c, "tooltip")?;
    let mut total = damage_bits;
    let mut list = Vec::new();
    for d in c.children("Division")? {
        let did = string(&d, "ids")?;
        if did.is_empty() {
            return Err(Error::Unsupported(format!(
                "{ids}: empty division id (native appends NULL and crashes in the damage add)"
            )));
        }
        let index = divisions
            .iter()
            .position(|x| eq(&x.ids, &did))
            .ok_or_else(|| unknown_ids(&did, "MGDivision"))?;
        total = f32_add(total, divisions[index].damage_bits, p)?;
        list.push(index);
    }
    let capturable = boolean(c, "capturable")?;
    let upgradable = boolean(c, "upgradable")?;
    let rts_actor_ids = string(c, "rts_actor_ids")?;
    if !rts_actor_ids.is_empty() && !ctx.actor("army rts_actor_ids", &rts_actor_ids, 0x2b)? {
        return Err(native(format!(
            "RTS Actor IDS '{rts_actor_ids}' is not a valid actor for the MG actor data component"
        )));
    }
    let rts_water_actor_ids = string(c, "rts_water_actor_ids")?;
    if !rts_water_actor_ids.is_empty() && !ctx.actor("army rts_water_actor_ids", &rts_water_actor_ids, 0x2b)? {
        return Err(native(format!(
            "RTS Water Actor IDS '{rts_water_actor_ids}' is not a valid actor for the MG actor data component"
        )));
    }
    let stockpile_bonus = ctx.resource_values(&subnode(c, "StockpileBonus")?)?;
    let water_stockpile_bonus = ctx.resource_values(&subnode(c, "WaterStockpileBonus")?)?;
    Ok(ArmyDefinition {
        ids: ids.to_owned(),
        tooltip,
        movement,
        damage_bits,
        damage_with_divisions_bits: total,
        division_matching_preference,
        divisions: list,
        capturable,
        upgradable,
        rts_actor_ids,
        rts_water_actor_ids,
        stockpile_bonus,
        water_stockpile_bonus,
    })
}

/// 95FD70.
fn load_territory_ai(ctx: &Ctx<'_>, node: &ResolvedNode<'_>, territory: Option<usize>) -> Result<TerritoryAi> {
    Ok(TerritoryAi {
        territory,
        additional_importance: int(node, "additional_importance")?,
        additional_neighbor_proximity_importance_depth: int(node, "additional_neighbor_proximity_importance_depth")?,
        additional_neighbor_proximity_importance_max: int(node, "additional_neighbor_proximity_importance_max")?,
        required_attack_strength_ratio_bits: fixed(node, "required_attack_strength_ratio", ctx.precision)?,
        desired_defense_threat_ratio_bits: fixed(node, "desired_defense_threat_ratio", ctx.precision)?,
    })
}

/// 960510 + 960120.
fn load_ai(ctx: &Ctx<'_>, node: &ResolvedNode<'_>, maps: &[MapDefinition]) -> Result<AiDefinition> {
    let p = ctx.precision;
    let ids = string(node, "ids")?;
    let name = localized(node, "name")?;
    let additional_attack_occupied_importance = int(node, "additional_attack_occupied_importance")?;
    let additional_attack_unoccupied_importance = int(node, "additional_attack_unoccupied_importance")?;
    let additional_defense_importance = int(node, "additional_defense_importance")?;
    let production_importance_multiplier = int(node, "production_importance_multiplier")?;
    let no_attack_distance = int(node, "no_attack_distance")?;
    let max_influence_depth = int(node, "max_influence_depth")?;
    let attack_defendability_factor_bits = fixed(node, "attack_defendability_factor", p)?;
    let actor_distance_influence_multiplier_bits = fixed(node, "actor_distance_influence_multiplier", p)?;
    let actor_distance_strength_multiplier_bits = fixed(node, "actor_distance_strength_multiplier", p)?;
    let goal_cutoff_ratio_bits = fixed(node, "goal_cutoff_ratio", p)?;
    let additional_border_importance = int(node, "additional_border_importance")?;
    let additional_border_reduction_importance = int(node, "additional_border_reduction_importance")?;
    let purchase_army_probability_bits = fixed(node, "purchase_army_probability", p)?;
    let purchase_tech_probability_bits = fixed(node, "purchase_tech_probability", p)?;
    let mut out_maps = Vec::new();
    for m in node.children("Map")? {
        let map_ids = string(&m, "ids")?;
        let Some(map) = maps.iter().position(|x| eq(&x.ids, &map_ids)) else {
            return Err(if map_ids.is_empty() {
                Error::Unsupported(format!("{ids}: empty MGAI map id (native NULL map)"))
            } else {
                unknown_ids(&map_ids, "MGMap")
            });
        };
        let default_territory = load_territory_ai(ctx, &subnode(&m, "DefaultTerritory")?, None)?;
        let capital = load_territory_ai(ctx, &subnode(&m, "Capital")?, None)?;
        let friendly_capital = load_territory_ai(ctx, &subnode(&m, "FriendlyCapital")?, None)?;
        let enemy_capital = load_territory_ai(ctx, &subnode(&m, "EnemyCapital")?, None)?;
        let mut territories = Vec::new();
        for t in m.children("Territory")? {
            let tid = string(&t, "ids")?;
            let index = maps[map]
                .territory(&tid)
                .ok_or_else(|| native(format!("Unknown territory ids: {tid}")))?;
            territories.push(load_territory_ai(ctx, &t, Some(index))?);
        }
        out_maps.push(MapAi {
            map,
            default_territory,
            capital,
            friendly_capital,
            enemy_capital,
            territories,
        });
    }
    Ok(AiDefinition {
        ids,
        name,
        additional_attack_occupied_importance,
        additional_attack_unoccupied_importance,
        additional_defense_importance,
        production_importance_multiplier,
        no_attack_distance,
        max_influence_depth,
        additional_border_importance,
        additional_border_reduction_importance,
        attack_defendability_factor_bits,
        actor_distance_influence_multiplier_bits,
        actor_distance_strength_multiplier_bits,
        goal_cutoff_ratio_bits,
        purchase_army_probability_bits,
        purchase_tech_probability_bits,
        maps: out_maps,
    })
}

/// 949B60.
fn load_command(node: &ResolvedNode<'_>) -> Result<CommandDefinition> {
    let ids = string(node, "ids")?;
    let name = localized(node, "name")?;
    let index = MG_COMMANDS
        .iter()
        .position(|c| eq(c, &ids))
        .ok_or_else(|| native(format!("Unknown metagame command: {ids}")))?;
    let icon = string(node, "icon")?;
    let tooltip = localized(node, "tooltip")?;
    Ok(CommandDefinition {
        index,
        name,
        icon,
        tooltip,
        cursor_ids: string(node, "cursor_ids")?,
        command_vfx_ids: string(node, "command_vfx_ids")?,
        hot_command: opt_boolean(node, "hot_command")?,
    })
}

// ---------------------------------------------------------------- campaign loaders

struct CampaignRefs<'a> {
    ctx: &'a Ctx<'a>,
    map: Option<&'a MapDefinition>,
    divisions: &'a [DivisionDefinition],
    ai: &'a [AiDefinition],
}

fn shop_list(
    refs: &CampaignRefs<'_>,
    node: &ResolvedNode<'_>,
    list: &str,
) -> Result<Vec<ShopRow>> {
    let mut rows = Vec::new();
    for row in node.children(list)? {
        let cost = int(&row, "cost")?;
        let ids = string(&row, "ids")?;
        let resolved = match list {
            "Technology" | "Division" if ids.is_empty() => None,
            "Technology" => {
                if !refs.ctx.property(&ids) {
                    return Err(unknown_ids(&ids, "Property"));
                }
                Some(ids)
            }
            "Division" => {
                if !refs.divisions.iter().any(|d| eq(&d.ids, &ids)) {
                    return Err(unknown_ids(&ids, "MGDivision"));
                }
                Some(ids)
            }
            _ => {
                // 4BDBA0(ids, 0x28): METAGAME_ACTOR registry index 40.
                if ids.is_empty() || !refs.ctx.actor("shop actor", &ids, 0x28)? {
                    return Err(native(format!("MetaGame actor with IDS '{ids}' not found.")));
                }
                Some(ids)
            }
        };
        rows.push(ShopRow { ids: resolved, cost });
    }
    Ok(rows)
}

/// 961790 comparator (ascending, signed cost).
fn by_cost(a: &ShopRow, b: &ShopRow) -> std::cmp::Ordering {
    a.cost.cmp(&b.cost)
}

/// 962290.
fn load_shop(refs: &CampaignRefs<'_>, node: &ResolvedNode<'_>) -> Result<Shop> {
    let technologies = shop_list(refs, node, "Technology")?;
    let actors = shop_list(refs, node, "Actor")?;
    let divisions = shop_list(refs, node, "Division")?;
    let mut all: Vec<ShopRow> = technologies
        .iter()
        .chain(&actors)
        .chain(&divisions)
        .cloned()
        .collect();
    let (mut technologies, mut actors, mut divisions) = (technologies, actors, divisions);
    msvc_qsort(&mut all, by_cost);
    msvc_qsort(&mut actors, by_cost);
    msvc_qsort(&mut divisions, by_cost);
    msvc_qsort(&mut technologies, by_cost);
    Ok(Shop {
        technologies,
        actors,
        divisions,
        all,
    })
}

/// 95EB00 (pass 1 of 95C840) with 95E740, 968250, 962290.
fn load_kingdom(refs: &CampaignRefs<'_>, node: &ResolvedNode<'_>) -> Result<CampaignKingdom> {
    let ids = string(node, "ids")?;
    let name = localized(node, "name")?;
    let starting_resources = int(node, "starting_resources")?;
    let nation_ids = string(node, "nation_ids")?;
    let capital_ids = string(node, "capital_ids")?;
    let Some(map) = refs.map else {
        return Err(Error::Unsupported(format!(
            "{ids}: capital lookup without a campaign map (native reads near NULL+0x18)"
        )));
    };
    let capital = map
        .territory(&capital_ids)
        .ok_or_else(|| native(format!("Capital territory with IDS '{capital_ids}' not found.")))?;
    let mgai_ids = string(node, "mgai_ids")?;
    let mgai = if mgai_ids.is_empty() {
        None
    } else {
        Some(
            refs.ai
                .iter()
                .position(|a| eq(&a.ids, &mgai_ids))
                .ok_or_else(|| unknown_ids(&mgai_ids, "MGAI"))?,
        )
    };
    let color_ids = string(node, "color_ids")?;
    let sai_ids = string(node, "sai_ids")?;
    let cinematic_win = string(node, "cinematic_win")?;
    let video = string(node, "nation_selection_screen_background_video")?;
    let stockpile_bonus = refs.ctx.resource_values(&subnode(node, "StockpileBonus")?)?;
    let water_stockpile_bonus = refs.ctx.resource_values(&subnode(node, "WaterStockpileBonus")?)?;
    let mut territories = Vec::new();
    for t in node.children("Territory")? {
        let tid = string(&t, "ids")?;
        let Some(territory) = map.territory(&tid) else {
            return Err(Error::Unsupported(format!(
                "territory record '{tid}' of {ids} is not on the map (native stores NULL and crashes in 95C5D0)"
            )));
        };
        let mut actors = Vec::new();
        for a in t.children("Actor")? {
            let aid = string(&a, "ids")?;
            match refs.ctx.thing("territory actor", &aid)? {
                Thing::Missing => {
                    return Err(native(format!(
                        "Actor with IDS '{aid}' not found in territory '{tid}'."
                    )));
                }
                Thing::Found { kind, .. } if kind != 2 => {
                    return Err(native(format!(
                        "Non-actor thing found with IDS '{aid}' in territory '{tid}'."
                    )));
                }
                _ => actors.push(aid),
            }
        }
        territories.push(OwnedTerritory {
            territory,
            liberates_to: None,
            actors,
        });
    }
    let capital_captures = int(&subnode(node, "Objective")?, "capital_captures")?;
    let shop = load_shop(refs, &subnode(node, "Shop")?)?;
    Ok(CampaignKingdom {
        ids,
        name,
        starting_resources,
        nation_ids,
        capital,
        mgai,
        color_ids,
        sai_ids,
        cinematic_win,
        nation_selection_screen_background_video: video,
        stockpile_bonus,
        water_stockpile_bonus,
        territories,
        capital_captures,
        shop,
    })
}

/// `FUN_0095C0D0`: first kingdom with the IDS.
fn kingdom_index(kingdoms: &[CampaignKingdom], ids: &str) -> Option<usize> {
    kingdoms.iter().position(|k| eq(&k.ids, ids))
}

/// 968000 + 967C10.
fn load_event(
    refs: &CampaignRefs<'_>,
    campaign_ids: &str,
    kingdoms: &[CampaignKingdom],
    node: &ResolvedNode<'_>,
) -> Result<CampaignEvent> {
    let ids = string(node, "ids")?;
    let name = localized(node, "name")?;
    let round = int(node, "round")?;
    let round_probability_bits = fixed(node, "round_probability", refs.ctx.precision)?;
    let message = localized(node, "message")?;
    let turn_ids = string(node, "kingdom_turn_ids")?;
    let kingdom_turn = kingdom_index(kingdoms, &turn_ids);
    if kingdom_turn.is_none() && !turn_ids.is_empty() {
        return Err(native(format!(
            "Kingdom turn IDS '{turn_ids}' is not a valid kingdom for the dynamic campaign '{campaign_ids}'"
        )));
    }
    let mut effects = Vec::new();
    for e in node.children("Effect")? {
        let type_name = string(&e, "type")?;
        let Some(&(_, kind, flags)) = EffectKind::TABLE.iter().find(|(n, _, _)| eq(n, &type_name)) else {
            return Err(Error::Unsupported(format!(
                "effect type '{type_name}': native stores -1 and reads flags out of bounds"
            )));
        };
        let message = localized(&e, "message")?;
        let check = |word: &str, nibble: u32, found: bool, raw: &str| -> Result<()> {
            if flags & (nibble & 0x22222) != 0 && !found {
                return Err(native(format!("{word} is required and not found '{raw}'")));
            }
            if !raw.is_empty() && !found {
                return Err(native(format!("{word} is optional but bad ids specified '{raw}'")));
            }
            Ok(())
        };
        let mut effect = EventEffect {
            kind,
            message,
            kingdom_target: None,
            kingdom_source: None,
            territory: None,
            actor_ids: None,
            tech_ids: None,
        };
        if flags & 0xf != 0 {
            let raw = string(&e, "kingdom_target_ids")?;
            effect.kingdom_target = kingdom_index(kingdoms, &raw);
            check("Kingdom target", 0xf, effect.kingdom_target.is_some(), &raw)?;
        }
        if flags & 0xf0 != 0 {
            let raw = string(&e, "kingdom_source_ids")?;
            effect.kingdom_source = kingdom_index(kingdoms, &raw);
            check("Kingdom source", 0xf0, effect.kingdom_source.is_some(), &raw)?;
        }
        if flags & 0xf0000 != 0 {
            let raw = string(&e, "territory_ids")?;
            effect.territory = refs.map.and_then(|m| m.territory(&raw));
            check("Territory", 0xf0000, effect.territory.is_some(), &raw)?;
        }
        if flags & 0xf000 != 0 {
            let raw = string(&e, "actor_ids")?;
            let found = !raw.is_empty() && refs.ctx.actor("effect actor_ids", &raw, 0x2b)?;
            check("Actor data", 0xf000, found, &raw)?;
            effect.actor_ids = found.then_some(raw);
        }
        if flags & 0xf00 != 0 {
            let raw = string(&e, "tech_ids")?;
            let found = !raw.is_empty() && refs.ctx.property(&raw);
            check("Technology", 0xf00, found, &raw)?;
            effect.tech_ids = found.then_some(raw);
        }
        effects.push(effect);
    }
    Ok(CampaignEvent {
        ids,
        name,
        message,
        round,
        round_probability_bits,
        kingdom_turn,
        effects,
    })
}

/// 95CCC0.
fn load_campaign(
    ctx: &Ctx<'_>,
    node: &ResolvedNode<'_>,
    maps: &[MapDefinition],
    map_tables: &[String],
    divisions: &[DivisionDefinition],
    ai: &[AiDefinition],
) -> Result<Campaign> {
    let ids = string(node, "ids")?;
    let name = localized(node, "name")?;
    let map_ids = string(node, "map_ids")?;
    let map = if map_ids.is_empty() {
        None
    } else {
        Some(
            maps.iter()
                .position(|m| eq(&m.ids, &map_ids))
                .ok_or_else(|| unknown_ids(&map_ids, "MGMap"))?,
        )
    };
    let map_table_ids = string(node, "map_table_ids")?;
    if !map_table_ids.is_empty() && !map_tables.iter().any(|t| eq(t, &map_table_ids)) {
        return Err(unknown_ids(&map_table_ids, "MGMapTable"));
    }
    let description = localized(node, "description")?;
    let initial_year = int(node, "initial_year")?;
    let initial_month_index = int(node, "initial_month")?.wrapping_sub(1);
    if !(0..=11).contains(&initial_month_index) {
        return Err(native("Invalid initial month for dynamic campaign data".into()));
    }
    let attacking_raw = string(node, "attacking_destroy_behavior")?;
    let defending_raw = string(node, "defending_destroy_behavior")?;
    let attacking_destroy_behavior = enum_value(&DestroyBehavior::TABLE, &attacking_raw)?;
    let defending_destroy_behavior = enum_value(&DestroyBehavior::TABLE, &defending_raw)?;
    let territory_enemy_vfx = string(node, "territory_enemy_vfx")?;
    let territory_friendly_vfx = string(node, "territory_friendly_vfx")?;
    let territory_enemy_arrow_vfx = string(node, "territory_enemy_arrow_vfx")?;
    let territory_friendly_arrow_vfx = string(node, "territory_friendly_arrow_vfx")?;
    let cinematic_intro = string(node, "cinematic_intro")?;
    let refs = CampaignRefs {
        ctx,
        map: map.map(|m| &maps[m]),
        divisions,
        ai,
    };
    // 95C840 pass 1: every kingdom; pass 2: liberates_to (95E320).
    let kingdom_nodes = node.children("Kingdom")?;
    let mut kingdoms = Vec::with_capacity(kingdom_nodes.len());
    for k in &kingdom_nodes {
        kingdoms.push(load_kingdom(&refs, k)?);
    }
    for (i, k) in kingdom_nodes.iter().enumerate() {
        for (j, t) in k.children("Territory")?.iter().enumerate() {
            let tid = string(t, "ids")?;
            let lib = string(t, "liberates_to")?;
            if lib.is_empty() {
                continue;
            }
            let target = kingdom_index(&kingdoms, &lib).ok_or_else(|| {
                native(format!(
                    "Trying to liberate to invalid kingdom with IDS '{lib}' in territory '{tid}'."
                ))
            })?;
            kingdoms[i].territories[j].liberates_to = Some(target);
        }
    }
    // 95C9B0 / 95E540
    let mut teams = Vec::new();
    for t in node.children("Team")? {
        let team_ids = string(&t, "ids")?;
        let mut members = Vec::new();
        for k in t.children("Kingdom")? {
            let kid = string(&k, "ids")?;
            members.push(kingdom_index(&kingdoms, &kid).ok_or_else(|| {
                native(format!("Kingdom with IDS '{kid}' not found for team '{team_ids}'."))
            })?);
        }
        teams.push(CampaignTeam {
            ids: team_ids,
            kingdoms: members,
        });
    }
    // 95CAD0
    let mut events = Vec::new();
    for e in node.children("Event")? {
        events.push(load_event(&refs, &ids, &kingdoms, &e)?);
    }
    let default_raw = string(node, "default_kingdom")?;
    let default_kingdom = kingdom_index(&kingdoms, &default_raw)
        .ok_or_else(|| native(format!("Default kingdom with IDS '{default_raw}' not found.")))?;
    // 95C5D0: kingdom i, record j, kingdom k != i, in that order.
    if let Some(map) = refs.map {
        for (i, a) in kingdoms.iter().enumerate() {
            for rec in &a.territories {
                for (k, b) in kingdoms.iter().enumerate() {
                    if k == i {
                        continue;
                    }
                    let name = &map.territories[rec.territory].ids;
                    if b.territories
                        .iter()
                        .any(|r| eq(&map.territories[r.territory].ids, name))
                    {
                        return Err(native(format!(
                            "2 kingdoms {} and {} cannot own the same territory {}",
                            a.ids, b.ids, name
                        )));
                    }
                }
            }
        }
    }
    Ok(Campaign {
        ids,
        name,
        description,
        map,
        map_table_ids,
        initial_year,
        initial_month_index,
        attacking_destroy_behavior,
        defending_destroy_behavior,
        territory_friendly_vfx,
        territory_enemy_vfx,
        territory_friendly_arrow_vfx,
        territory_enemy_arrow_vfx,
        cinematic_intro,
        kingdoms,
        teams,
        events,
        default_kingdom,
    })
}

// ---------------------------------------------------------------- entry points

fn settings(catalog: &ActorCatalog, precision: Precision) -> Result<QuickBattleSettings> {
    let mut bonus = None;
    let mut probability = None;
    for svars in catalog.resolved_definitions("SVars")? {
        for (name, slot) in [
            ("MetaGameQuickBattleTechnologyBonus", &mut bonus),
            ("MetaGameQuickBattleAttackWinProbability", &mut probability),
        ] {
            if svars.field(name, precision)?.is_some()
                && slot.replace(fixed(&svars, name, precision)?).is_some()
            {
                return Err(Error::Unsupported(format!("SVars override order for {name}")));
            }
        }
    }
    let absent = |n: &str| Error::Unsupported(format!("SVars {n} absent"));
    Ok(QuickBattleSettings {
        technology_bonus_bits: bonus.ok_or_else(|| absent("MetaGameQuickBattleTechnologyBonus"))?,
        attack_win_probability_bits: probability
            .ok_or_else(|| absent("MetaGameQuickBattleAttackWinProbability"))?,
    })
}

/// Data manager registration: duplicate IDS is fatal ("Data IDS already used").
fn unique(kind: &str, ids: &[String]) -> Result<()> {
    for (i, a) in ids.iter().enumerate() {
        if ids[..i].iter().any(|b| eq(a, b)) {
            return Err(native(format!("Data IDS already used: '{a}' ({kind})")));
        }
    }
    Ok(())
}

impl MetaGameData {
    /// Load from the verified original archive plus v14 overlays.
    pub fn load_v14(sources: &mut V14Sources, precision: Precision) -> Result<Self> {
        let catalog = ActorCatalog::load_v14(sources)?;
        let resources = aa_rules::resources::ResourceCatalog::load_v14(sources)
            .map_err(|e| Error::Unsupported(format!("resource catalog: {e}")))?;
        let names: Vec<String> = resources.resources.iter().map(|r| r.id.clone()).collect();
        let mut opaque = Vec::new();
        for d in &catalog.diagnostics {
            if d.message.starts_with("TGI syntax") {
                opaque.push((d.source.path.clone(), sources.read(&d.source.path)?.bytes));
            }
        }
        Self::from_catalog(&catalog, &names, &opaque, precision)
    }

    /// Project the metagame definitions out of a resolved catalog.
    /// `resources`: resource type IDS consulted by every `ResourceList` read.
    /// `opaque`: sources (path, bytes) the shared parser rejected; Thing IDS
    /// assigned there are reported in `unverified_references`.
    /// `precision` only affects percent-suffixed fixed literals (none shipped).
    pub fn from_catalog(
        catalog: &ActorCatalog,
        resources: &[String],
        opaque: &[(String, Vec<u8>)],
        precision: Precision,
    ) -> Result<Self> {
        let mut property_ids = Vec::new();
        for p in catalog.resolved_definitions("Property")? {
            if let Some(ids) = opt_string(&p, "IDS")? {
                property_ids.push(ids);
            }
        }
        let mut rmc = Vec::new();
        let mut rmc_full = Vec::new();
        for r in catalog.resolved_definitions("RMC")? {
            let rmc_ids = string(&r, "IDS")?;
            let mut kingdoms = Vec::new();
            for k in r.children("Kingdom")? {
                kingdoms.push(string(&k, "IDS")?);
            }
            let mut slots = Vec::new();
            for k in r.children("Kingdom")? {
                slots.push(RmcKingdom {
                    ids: string(&k, "IDS")?,
                    kind: opt_string(&k, "type")?.unwrap_or_default(),
                    independent: opt_boolean(&k, "independent")?.unwrap_or(false),
                });
            }
            let mut sizes = Vec::new();
            for ms in r.children("MapSize")? {
                sizes.push((fixed(&ms, "width", precision)?, fixed(&ms, "height", precision)?));
            }
            rmc.push((rmc_ids.clone(), kingdoms, r.children("MapSize")?.len()));
            rmc_full.push(RmcDefinition { ids: rmc_ids, kingdoms: slots, map_sizes: sizes });
        }
        let ctx = Ctx {
            catalog,
            precision,
            resources,
            property_ids,
            rmc,
            opaque,
            unverified: Default::default(),
        };

        let commands = catalog
            .resolved_definitions("MGCommand")?
            .iter()
            .map(load_command)
            .collect::<Result<Vec<_>>>()?;

        let division_nodes = catalog.resolved_definitions("MGDivision")?;
        let mut divisions = Vec::new();
        let mut prereqs = Vec::new();
        for d in &division_nodes {
            let (def, pre) = load_division(&ctx, d)?;
            divisions.push(def);
            prereqs.push(pre);
        }
        unique("MGDivision", &divisions.iter().map(|d| d.ids.clone()).collect::<Vec<_>>())?;
        for (i, pre) in prereqs.iter().enumerate() {
            for p in pre {
                let index = divisions
                    .iter()
                    .position(|d| eq(&d.ids, p))
                    .ok_or_else(|| native(format!("MGDivisionData cannot load prereq division '{p}'")))?;
                divisions[i].prerequisites.push(index);
            }
        }

        // 464BD0 calls 93B250 for every Thing with a (non-optional)
        // MetaGameComponent sub-node. Things the shared resolver cannot resolve
        // are only tolerated when they are not METAGAME_ACTOR; they are counted.
        let mut armies = Vec::new();
        let mut unresolved_things = Vec::new();
        for actor in &catalog.actors {
            let metagame = actor.field("category").and_then(|f| f.raw_text()).is_some_and(|c| eq(c, "METAGAME_ACTOR"));
            let component = match catalog.resolved_actor(&actor.id).map_err(Error::from).and_then(|n| Ok(n.child("MetaGameComponent")?)) {
                Ok(c) => c,
                Err(e) if metagame => return Err(e),
                Err(_) => {
                    unresolved_things.push(actor.id.clone());
                    continue;
                }
            };
            if let Some(component) = component {
                armies.push(load_army(&ctx, &actor.id, &component, &divisions)?);
            } else if metagame {
                return Err(Error::Unsupported(format!("METAGAME_ACTOR {} without MetaGameComponent", actor.id)));
            }
        }

        let map_tables = catalog
            .resolved_definitions("MGMapTable")?
            .iter()
            .map(|t| string(t, "IDS"))
            .collect::<Result<Vec<_>>>()?;
        unique("MGMapTable", &map_tables)?;

        let maps = catalog
            .resolved_definitions("MGMap")?
            .iter()
            .map(|m| load_map(&ctx, m))
            .collect::<Result<Vec<_>>>()?;
        unique("MGMap", &maps.iter().map(|m| m.ids.clone()).collect::<Vec<_>>())?;

        let ai = catalog
            .resolved_definitions("MGAI")?
            .iter()
            .map(|a| load_ai(&ctx, a, &maps))
            .collect::<Result<Vec<_>>>()?;
        unique("MGAI", &ai.iter().map(|a| a.ids.clone()).collect::<Vec<_>>())?;

        let campaigns = catalog
            .resolved_definitions("DynamicCampaign")?
            .iter()
            .map(|c| load_campaign(&ctx, c, &maps, &map_tables, &divisions, &ai))
            .collect::<Result<Vec<_>>>()?;
        unique("DynamicCampaign", &campaigns.iter().map(|c| c.ids.clone()).collect::<Vec<_>>())?;

        let mut rmcs: Vec<RmcDefinition> = Vec::new();
        for m in &maps {
            for t in &m.territories {
                if t.rmc_ids.is_empty() || rmcs.iter().any(|r| eq(&r.ids, &t.rmc_ids)) {
                    continue;
                }
                if let Some(r) = rmc_full.iter().find(|r| eq(&r.ids, &t.rmc_ids)) {
                    rmcs.push(r.clone());
                }
            }
        }
        Ok(Self {
            settings: settings(catalog, precision)?,
            rmcs,
            unresolved_things,
            unverified_references: ctx.unverified.take(),
            maps,
            map_tables,
            divisions,
            armies,
            ai,
            commands,
            campaigns,
        })
    }
}
