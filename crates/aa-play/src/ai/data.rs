//! The original strategic-AI data: SAI profiles (`Data/SAI/*.tgi`, template
//! `SAI` in `Templates/template_sai.tgi`) and their egos (template `Ego`,
//! specialised by `template_sai_aa*.tgi`), plus the difficulty handicaps
//! (`Game/handicaps.tgi`, `Properties/properties_handicaps.tgi`).
//!
//! Every value is read from the original data through the aa-rules
//! inherited-node resolver (the same template binding the native loaders
//! `4AA430` profile / `4B0560` ego use, ported exactly in `aa_mind::sai`).
//! Here the values are plain `f32` for the behaviour-faithful opponent.
//! Field names follow the TGI keys; the native ego offsets are listed in
//! research/phase8-ai/ego-fields.json.
use aa_numeric::Precision;
use aa_rules::actors::{ActorCatalog, ResolvedNode};
use aa_rules::native::TypedValue;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

const PC: Precision = Precision::Double53;
const CACHE_VERSION: &str = "aa-play-sai-v3";

/// `tables::GOAL_TYPES` order of the native `7BDB70` enum.
pub const GOAL_TYPES: [&str; 23] = [
    "ATTACK_REGION",
    "ATTACK_ACTOR",
    "DEFEND_REGION",
    "DEFEND_ACTOR",
    "AIR_ATTACK",
    "RECOVER",
    "REPAIR",
    "EXPLORE",
    "BUILD",
    "CONSTRUCT",
    "PACK",
    "UNPACK",
    "RECRUIT",
    "UPGRADE",
    "GIVE_RESOURCES",
    "GIVE_ACTOR",
    "DESTROY",
    "ATTACH_HERO",
    "MOVE_HERO",
    "RESEARCH",
    "BOMBARD",
    "GENERAL_POWER",
    "ATTACH",
];
pub const ATTACK_REGION: usize = 0;
pub const DEFEND_REGION: usize = 2;
pub const RECOVER: usize = 5;
pub const EXPLORE: usize = 7;
pub const UNPACK: usize = 11;
pub const RECRUIT: usize = 12;
pub const RESEARCH: usize = 19;
pub const ATTACH: usize = 22;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Scoring statistic, e.g. `money_rate`.
    pub stat: String,
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ActorPreference {
    pub property: Option<String>,
    pub actor: Option<String>,
    pub value: f32,
    pub count_value_once_per_company: bool,
    pub multiplier: f32,
    pub count_multiplier_once_per_goal: bool,
    pub count_multiplier_once_per_company: bool,
}

/// `[list GoalPriority]` (ego +4C + type * 18, loader `4AF500`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoalPriority {
    pub multiplier: f32,
    pub bonus: f32,
    pub minimum: f32,
    /// Negative selects the ego default.
    pub fuzziness: f32,
    pub inertia: f32,
    pub preferences: Vec<ActorPreference>,
}
impl Default for GoalPriority {
    /// The record of a goal type the ego does not list: the native ego
    /// constructor's neutral record (multiplier 1, the template defaults).
    fn default() -> Self {
        Self { multiplier: 1.0, bonus: 0.0, minimum: 0.0, fuzziness: -1.0, inertia: -1.0, preferences: Vec::new() }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResourceNeed {
    pub resource: String,
    pub short_need: f32,
    pub enough_need: f32,
    pub plenty_value: f32,
    pub plenty_need: f32,
}

/// `[list ActorPriority]` (loader `4AF8E0`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ActorPriority {
    pub property: Option<String>,
    pub actor: Option<String>,
    pub count_all_property_actors: bool,
    pub value: f32,
    pub repeat_penalty: f32,
    /// 0 = the ego default_max_recruited_per_think.
    pub max_recruited_per_think: i32,
    pub one_time_bonus: f32,
}

/// `[list SpecificRecruitRequest]` (loader `4AAEB0`/`4A7C70`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RecruitRequest {
    pub recruit: Option<String>,
    pub property: Option<String>,
    pub number_to_request: i32,
    pub base_priority: f32,
    pub economic_value_multiplier: f32,
    pub repeat_penalty: f32,
    pub one_time_bonus: f32,
    pub hq_type_not_available_bonus: f32,
}

/// `[OffensiveCV]` / `[DefensiveCV]` (ego +304.. / +31C..).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CvCurve {
    pub cv_advantage_target: f32,
    pub cv_advantage_max: f32,
    pub cv_exponent_under_target: f32,
    pub cv_max_mult_over_target: f32,
    pub min_cv_to_assign: f32,
    /// Offensive: no_primary_supply_bonus.
    pub no_primary_supply_bonus: f32,
    /// Defensive: count_static_cv.
    pub count_static_cv: bool,
}

/// `[Combat/DefendPriority]` (ego +39C..+3C8, read by `7BD360`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DefendPriority {
    pub economic_value_exponent: f32,
    pub economic_value_multiplier: f32,
    pub enemy_influence_exponent: f32,
    pub enemy_influence_multiplier: f32,
    pub friendly_border_influence_exponent: f32,
    pub friendly_border_influence_multiplier: f32,
    pub enemy_border_influence_exponent: f32,
    pub enemy_border_influence_multiplier: f32,
    pub enemy_forces_threat_distance: f32,
    pub enemy_forces_bonus: f32,
    pub capture_risk_ratio: f32,
    pub capture_risk_bonus: f32,
}

/// `ExplorationTypeData` (CloseGoals / DeepGoals / AirReconGoals).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Exploration {
    pub num_goals: i32,
    pub base_priority: f32,
    pub repeat_penalty: f32,
    pub min_priority: f32,
    pub percent_to_count_region_explored: f32,
    pub move_rate_mult: f32,
    pub move_rate_exp: f32,
    pub sight_mult: f32,
    pub sight_exp: f32,
    pub subtract_cv: bool,
    pub cv_mult: f32,
    pub cv_exp: f32,
    pub unit_inertia: f32,
    pub intuition_probability: f32,
    pub intuition_distance: i32,
    pub intuition_count_mines: bool,
    pub intuition_count_settlement_spots: bool,
    pub intuition_count_settlements: bool,
    pub intuition_count_other_buildings: bool,
    pub num_random_candidates: i32,
    pub black_fog_priority: f32,
    pub grey_fog_priority: f32,
    pub fog_search_depth: i32,
    pub current_region_bonus: f32,
    pub actor_region_bonus: f32,
    pub building_region_bonus: f32,
    pub intuition_region_bonus: f32,
    pub intuition_region_exponent: f32,
    pub random_candidate_bonus: f32,
    pub consider_distance_to_kingdom_center: bool,
    pub kingdom_center_distance_cap: f32,
    pub kingdom_center_distance_max: f32,
    pub kingdom_center_distance_exp: f32,
    pub kingdom_center_distance_ceil: f32,
    pub kingdom_center_distance_floor: f32,
    pub kingdom_center_distance_prefer_close: bool,
    pub consider_distance_to_last_region: bool,
    pub last_region_distance_cap: f32,
    pub last_region_distance_max: f32,
    pub last_region_distance_exp: f32,
    pub last_region_distance_ceil: f32,
    pub last_region_distance_floor: f32,
    pub last_region_distance_prefer_close: bool,
    pub consider_distance_to_actor: bool,
    pub actor_distance_cap: f32,
    pub actor_distance_max: f32,
    pub actor_distance_exp: f32,
    pub actor_distance_ceil: f32,
    pub actor_distance_floor: f32,
    pub actor_distance_prefer_close: bool,
    pub use_time_since_last_scouted: bool,
    pub time_since_last_scouted_max: f32,
    pub time_since_last_scouted_exp: f32,
    pub fuzzy_factor: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Recruiting {
    pub max_goals_per_think: i32,
    pub num_military_goals: i32,
    pub core_cv_multiplier: f32,
    pub core_cv_ratio_multiplier: f32,
    pub support_cv_multiplier: f32,
    pub support_cv_ratio_multiplier: f32,
    pub economic_value_multiplier: f32,
    pub decreased_income_penalty: f32,
    pub insufficient_income_level: f32,
    pub insufficient_income_penalty: f32,
    pub empty_slot_value: f32,
    pub candidate_fuzziness: f32,
    pub matching_builder_bonus: f32,
    pub mismatching_builder_penalty: f32,
    pub base_value: f32,
    pub building_bonus: f32,
    pub location_fuzziness: f32,
    pub replace_company_cv_mult: f32,
    pub requests: Vec<RecruitRequest>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Unpacking {
    pub adjacent_region_depth: i32,
    pub num_random_regions: i32,
    pub base_priority: f32,
    pub enemy_static_influence_mult: f32,
    pub enemy_dynamic_influence_mult: f32,
    pub max_friendly_building_bonus: f32,
    pub max_friendly_building_bonus_per_building: f32,
    pub max_distance: f32,
    pub distance_exponent: f32,
    pub travel_distance_mult: f32,
    pub travel_distance_exponent: f32,
    pub fuzzy_factor: f32,
    pub inertia: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Recover {
    pub base_priority_hp: f32,
    pub exponent_hp: f32,
    pub max_hp: f32,
    pub base_priority_morale: f32,
    pub exponent_morale: f32,
    pub max_morale: f32,
    pub critical_morale: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Flares {
    pub defend_attacked_target_duration: f32,
    pub defend_attacked_target_priority: f32,
    pub attack_region_on_organization_defeated_duration: f32,
    pub attack_region_on_organization_defeated_priority: f32,
    pub counter_attack_captured_actor_duration: f32,
    pub counter_attack_captured_actor_priority: f32,
    pub defend_new_structure_duration: f32,
    pub defend_new_structure_priority: f32,
    pub defend_captured_structure_duration: f32,
    pub defend_captured_structure_priority: f32,
}

/// `[ReactiveAI]` (ego +894..+8BC, think `7B8AC0`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Reactive {
    pub think_frequency: f32,
    pub reaction_time: f32,
    pub unit_retreat_max_health: f32,
    pub unit_retreat_depth: i32,
    pub unit_retreat_cv_ratio: f32,
    pub unit_retreat_probability: f32,
    pub unit_retreat_cycles_til_reconsider: i32,
    pub region_retreat_depth: i32,
    pub region_retreat_cv_ratio: f32,
    pub region_retreat_probability: f32,
    pub region_retreat_cycles_til_reconsider: i32,
    pub halt_retreat_cv_ratio: f32,
    pub attack_distance: i32,
    pub min_attack_advantage: f32,
    pub max_attack_advantage: f32,
    pub threatened_region_influence_ratio: f32,
}

/// One `[Ego]` (native `KKC_SAIEgo`, loader `4B0560`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Ego {
    pub ids: String,
    pub priority: f32,
    pub filters: Vec<Filter>,
    // MapAnalysis
    pub cv_influence_exponent: f32,
    pub cv_influence_max_travel_time: f32,
    pub border_influence_range: f32,
    // GoalEngine/GeneralSettings
    pub think_frequency: f32,
    pub reaction_time: f32,
    pub fuzziness: f32,
    pub goal_inertia: f32,
    pub immediate_think_on_build_max_settlements: i32,
    pub immediate_think_on_build_max_time: f32,
    /// By `GOAL_TYPES` index.
    pub goal_priorities: Vec<GoalPriority>,
    // ResourceManagement
    pub saving_time_limit: f32,
    pub stockpilable_short_need: f32,
    pub stockpilable_enough_need: f32,
    pub stockpilable_plenty_value: f32,
    pub stockpilable_plenty_need: f32,
    pub other_short_need: f32,
    pub other_enough_need: f32,
    pub other_plenty_value: f32,
    pub other_plenty_need: f32,
    pub resource_needs: Vec<ResourceNeed>,
    // ActorValue
    pub scaling_mult: f32,
    pub enemy_influence_mult: f32,
    pub min_influence_factor: f32,
    pub influence_exponent: f32,
    pub military_cv_mult: f32,
    pub military_value_scaling_mult: f32,
    pub distance_mult_min: f32,
    pub distance_mult_max: f32,
    pub offensive: CvCurve,
    pub defensive: CvCurve,
    pub max_travel_time: f32,
    pub travel_time_exponent: f32,
    pub actor_priorities: Vec<ActorPriority>,
    pub default_max_recruited_per_think: i32,
    pub build_queue_max_items: i32,
    // Combat
    pub defend: DefendPriority,
    pub defend_friendly_kingdom_actor_multiplier: f32,
    pub attack_allied_influence_cv_multiplier: f32,
    pub defend_allied_influence_cv_multiplier: f32,
    pub linked_region_distance: i32,
    pub max_defense_depth: i32,
    pub attack_commit_distance: i32,
    pub congruency_min_multiplier: f32,
    pub congruency_power: f32,
    pub penalty_to_builder_cv: f32,
    /// Close, deep, air recon.
    pub exploration: [Exploration; 3],
    pub kingdom_center_min_time_owned: f32,
    pub unpacking: Unpacking,
    pub pack_out_of_supply_priority: f32,
    pub recover: Recover,
    pub recruiting: Recruiting,
    pub flares: Flares,
    pub reactive: Reactive,
    /// `[Research]` (SAIResearchTemplate, ego +82C base and +830 table).
    pub research_base_priority: f32,
    pub tech_priorities: Vec<(String, f32)>,
    /// `[Attach]` (ego +7A4, +7A8).
    pub attach_when_engaged: bool,
    pub attach_cv_multiplier: f32,
}

impl Ego {
    /// `4A9A20`: the TechPriority of a technology, else the base priority.
    pub fn tech_priority(&self, tech: &str) -> f32 {
        self.tech_priorities.iter().find(|(t, _)| t.eq_ignore_ascii_case(tech)).map_or(self.research_base_priority, |(_, p)| *p)
    }
    pub fn goal(&self, kind: usize) -> GoalPriority {
        self.goal_priorities.get(kind).cloned().unwrap_or_default()
    }
}

/// One `[SAI]` profile (native `KKC_SAIData`, loader `4AA430`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Profile {
    pub ids: String,
    /// `difficulty_level` (all shipped profiles: sai_difficulty_level_HARD).
    pub difficulty_level: String,
    pub public: bool,
    pub command_max_delay: f32,
    pub command_max_total_delay: f32,
    pub max_ge_commands: i32,
    pub max_rai_commands: i32,
    pub ego_fuzziness: f32,
    /// Indices into `AiData::egos`; the initial ego first.
    pub egos: Vec<usize>,
    /// "nation_rus"...
    pub nations: Vec<String>,
    /// General (faction) ids, e.g. "zhukov".
    pub factions: Vec<String>,
}

/// A `[Handicap]` and the multiply modifiers of its property.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Handicap {
    pub ids: String,
    pub property: Option<String>,
    /// (attribute, MultiplyModifier amount), e.g. ("RECRUIT_COST", -0.25).
    pub modifiers: Vec<(String, f32)>,
}

/// ElementComponent `required_structures` / `required_properties` of a
/// Thing (every listed structure owned, every property present): the
/// candidate test `4E65E0` of the recruit planner.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Requirement {
    pub structures: Vec<String>,
    pub properties: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AiData {
    pub source_key: String,
    pub profiles: Vec<Profile>,
    pub egos: Vec<Ego>,
    /// In `Game/handicaps.tgi` order (easy, medium, hard).
    pub handicaps: Vec<Handicap>,
    /// Lower-case Thing IDS -> its element requirements (only Things that
    /// have any).
    #[serde(default)]
    pub requirements: std::collections::BTreeMap<String, Requirement>,
}

impl AiData {
    pub fn profile(&self, ids: &str) -> Option<usize> {
        self.profiles.iter().position(|p| p.ids.eq_ignore_ascii_case(ids))
    }
    pub fn ego(&self, ids: &str) -> Option<usize> {
        self.egos.iter().position(|e| e.ids.eq_ignore_ascii_case(ids))
    }

    /// Load from the user's installed original data (Data.rwd + the v14
    /// overlay), cached as JSON next to the GameData cache
    /// (`AA_PLAY_NO_CACHE=1` forces a fresh read).
    pub fn load(game_root: &std::path::Path) -> Result<Self> {
        let key = format!("{CACHE_VERSION}:{}", aa_rules::sources::BASELINE_LEDGER_SHA256);
        let path = std::env::var("AA_PLAY_SAI_CACHE").map(std::path::PathBuf::from).unwrap_or_else(|_| {
            game_root.join("rust-rebuild").join("assets-cache").join("aa-play").join("sai.json")
        });
        let use_cache = std::env::var("AA_PLAY_NO_CACHE").map_or(true, |v| v.is_empty() || v == "0");
        if use_cache
            && let Ok(bytes) = std::fs::read(&path)
            && let Ok(data) = serde_json::from_slice::<AiData>(&bytes)
            && data.source_key == key
        {
            return Ok(data);
        }
        let mut sources = crate::data::Sources::open(game_root)?;
        let catalog = ActorCatalog::load_v14(&mut sources.files).map_err(|e| anyhow!("loading the original rules catalog: {e}"))?;
        let mut data = Self::from_catalog(&catalog)?;
        data.source_key = key;
        if use_cache {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(bytes) = serde_json::to_vec(&data) {
                let _ = std::fs::write(&path, bytes);
            }
        }
        Ok(data)
    }

    pub fn from_catalog(catalog: &ActorCatalog) -> Result<Self> {
        let mut data = AiData::default();
        let egos = catalog.resolved_definitions("Ego").map_err(|e| anyhow!("Ego roots: {e}"))?;
        for node in &egos {
            let n = N { node: node.clone(), path: "Ego".into() };
            let ego = load_ego(&n).with_context(|| format!("ego {:?}", n.s("IDS").ok().flatten()))?;
            data.egos.push(ego);
        }
        let sais = catalog.resolved_definitions("SAI").map_err(|e| anyhow!("SAI roots: {e}"))?;
        for node in &sais {
            let n = N { node: node.clone(), path: "SAI".into() };
            let profile = load_profile(&n, &data.egos).with_context(|| format!("SAI {:?}", n.s("IDS").ok().flatten()))?;
            data.profiles.push(profile);
        }
        for actor in &catalog.actors {
            let Ok(node) = catalog.resolved_actor(aa_rules::actors::ThingRef::registered(&actor.id)) else { continue };
            let n = N { node, path: actor.id.clone() };
            let Ok(element) = n.child("ElementComponent") else { continue };
            let list = |name: &str| -> Vec<String> {
                element.s(name).ok().flatten().map(|v| v.split(',').map(|x| x.trim().to_ascii_lowercase()).filter(|x| !x.is_empty() && x != "null").collect()).unwrap_or_default()
            };
            let r = Requirement { structures: list("required_structures"), properties: list("required_properties") };
            if !r.structures.is_empty() || !r.properties.is_empty() {
                data.requirements.insert(actor.id.to_ascii_lowercase(), r);
            }
        }
        if let Ok(nodes) = catalog.resolved_definitions("Handicap") {
            for node in nodes {
                let n = N { node, path: "Handicap".into() };
                let ids = n.s("IDS")?.unwrap_or_default();
                let property = n.s("property_ids").ok().flatten();
                let mut modifiers = Vec::new();
                if let Some(p) = &property
                    && let Ok(prop) = catalog.resolved_definition("Property", p, PC)
                {
                    let pn = N { node: prop, path: format!("Property {p}") };
                    for m in pn.children("Modifier")? {
                        if let (Some(attribute), Ok(amount)) = (m.s("attribute")?, m.f("amount")) {
                            modifiers.push((attribute, amount));
                        }
                    }
                }
                data.handicaps.push(Handicap { ids, property, modifiers });
            }
        }
        Ok(data)
    }
}

/// A resolved node with its path for error messages.
struct N<'a> {
    node: ResolvedNode<'a>,
    path: String,
}

impl<'a> N<'a> {
    fn value(&self, name: &str) -> Result<Option<TypedValue>> {
        match self.node.field(name, PC) {
            Ok(v) => Ok(v),
            Err(e) => match self.node.plain_string_field(name) {
                Ok(v) => Ok(v),
                Err(_) => Err(anyhow!("{}/{name}: {e}", self.path)),
            },
        }
    }
    fn f(&self, name: &str) -> Result<f32> {
        match self.value(name)? {
            Some(TypedValue::Scalar(s)) => Ok(f32::from_bits(s.bits)),
            Some(TypedValue::Integer { value, .. }) => Ok(value as f32),
            Some(other) => Err(anyhow!("{}/{name} is not a number: {other:?}", self.path)),
            None => Err(anyhow!("{}/{name} is missing", self.path)),
        }
    }
    fn i(&self, name: &str) -> Result<i32> {
        match self.value(name)? {
            Some(TypedValue::Integer { value, .. }) => Ok(value),
            Some(TypedValue::Scalar(s)) => Ok(f32::from_bits(s.bits) as i32),
            Some(other) => Err(anyhow!("{}/{name} is not an integer: {other:?}", self.path)),
            None => Err(anyhow!("{}/{name} is missing", self.path)),
        }
    }
    fn b(&self, name: &str) -> Result<bool> {
        match self.value(name)? {
            Some(TypedValue::Boolean { value, .. }) => Ok(value),
            Some(TypedValue::Integer { value, .. }) => Ok(value != 0),
            Some(TypedValue::String { value, .. }) if value.eq_ignore_ascii_case("true") => Ok(true),
            Some(TypedValue::String { value, .. }) if value.eq_ignore_ascii_case("false") => Ok(false),
            Some(other) => Err(anyhow!("{}/{name} is not a bool: {other:?}", self.path)),
            None => Err(anyhow!("{}/{name} is missing", self.path)),
        }
    }
    /// A string field; `null`/empty is `None`.
    fn s(&self, name: &str) -> Result<Option<String>> {
        match self.value(name)? {
            Some(TypedValue::String { value, .. }) => {
                Ok((!value.is_empty() && !value.eq_ignore_ascii_case("null")).then(|| value.to_ascii_lowercase()))
            }
            Some(other) => Err(anyhow!("{}/{name} is not a string: {other:?}", self.path)),
            None => Ok(None),
        }
    }
    fn child(&self, name: &str) -> Result<N<'a>> {
        let node = self.node.child(name).map_err(|e| anyhow!("{}/{name}: {e}", self.path))?;
        let node = node.ok_or_else(|| anyhow!("{}/{name} is missing", self.path))?;
        Ok(N { node, path: format!("{}/{name}", self.path) })
    }
    fn children(&self, name: &str) -> Result<Vec<N<'a>>> {
        let nodes = self.node.children(name).map_err(|e| anyhow!("{}/{name}: {e}", self.path))?;
        Ok(nodes.into_iter().enumerate().map(|(i, node)| N { node, path: format!("{}/{name}[{i}]", self.path) }).collect())
    }
}

fn load_curve(n: &N<'_>, offensive: bool) -> Result<CvCurve> {
    Ok(CvCurve {
        cv_advantage_target: n.f("cv_advantage_target")?,
        cv_advantage_max: n.f("cv_advantage_max")?,
        cv_exponent_under_target: n.f("cv_exponent_under_target")?,
        cv_max_mult_over_target: n.f("cv_max_mult_over_target")?,
        min_cv_to_assign: n.f("min_cv_to_assign")?,
        no_primary_supply_bonus: if offensive { n.f("no_primary_supply_bonus")? } else { 0.0 },
        count_static_cv: if offensive { false } else { n.b("count_static_cv")? },
    })
}

fn load_exploration(n: &N<'_>) -> Result<Exploration> {
    Ok(Exploration {
        num_goals: n.i("num_goals")?,
        base_priority: n.f("base_priority")?,
        repeat_penalty: n.f("repeat_penalty")?,
        min_priority: n.f("min_priority")?,
        percent_to_count_region_explored: n.f("percent_to_count_region_explored")?,
        move_rate_mult: n.f("move_rate_mult")?,
        move_rate_exp: n.f("move_rate_exp")?,
        sight_mult: n.f("sight_mult")?,
        sight_exp: n.f("sight_exp")?,
        subtract_cv: n.b("subtract_cv")?,
        cv_mult: n.f("cv_mult")?,
        cv_exp: n.f("cv_exp")?,
        unit_inertia: n.f("unit_inertia")?,
        intuition_probability: n.f("intuition_probability")?,
        intuition_distance: n.i("intuition_distance")?,
        intuition_count_mines: n.b("intuition_count_mines")?,
        intuition_count_settlement_spots: n.b("intuition_count_settlement_spots")?,
        intuition_count_settlements: n.b("intuition_count_settlements")?,
        intuition_count_other_buildings: n.b("intuition_count_other_buildings")?,
        num_random_candidates: n.i("num_random_candidates")?,
        black_fog_priority: n.f("black_fog_priority")?,
        grey_fog_priority: n.f("grey_fog_priority")?,
        fog_search_depth: n.i("fog_search_depth")?,
        current_region_bonus: n.f("current_region_bonus")?,
        actor_region_bonus: n.f("actor_region_bonus")?,
        building_region_bonus: n.f("building_region_bonus")?,
        intuition_region_bonus: n.f("intuition_region_bonus")?,
        intuition_region_exponent: n.f("intuition_region_exponent")?,
        random_candidate_bonus: n.f("random_candidate_bonus")?,
        consider_distance_to_kingdom_center: n.b("consider_distance_to_kingdom_center")?,
        kingdom_center_distance_cap: n.f("kingdom_center_distance_cap")?,
        kingdom_center_distance_max: n.f("kingdom_center_distance_max")?,
        kingdom_center_distance_exp: n.f("kingdom_center_distance_exp")?,
        kingdom_center_distance_ceil: n.f("kingdom_center_distance_ceil")?,
        kingdom_center_distance_floor: n.f("kingdom_center_distance_floor")?,
        kingdom_center_distance_prefer_close: n.b("kingdom_center_distance_prefer_close")?,
        consider_distance_to_last_region: n.b("consider_distance_to_last_region")?,
        last_region_distance_cap: n.f("last_region_distance_cap")?,
        last_region_distance_max: n.f("last_region_distance_max")?,
        last_region_distance_exp: n.f("last_region_distance_exp")?,
        last_region_distance_ceil: n.f("last_region_distance_ceil")?,
        last_region_distance_floor: n.f("last_region_distance_floor")?,
        last_region_distance_prefer_close: n.b("last_region_distance_prefer_close")?,
        consider_distance_to_actor: n.b("consider_distance_to_actor")?,
        actor_distance_cap: n.f("actor_distance_cap")?,
        actor_distance_max: n.f("actor_distance_max")?,
        actor_distance_exp: n.f("actor_distance_exp")?,
        actor_distance_ceil: n.f("actor_distance_ceil")?,
        actor_distance_floor: n.f("actor_distance_floor")?,
        actor_distance_prefer_close: n.b("actor_distance_prefer_close")?,
        use_time_since_last_scouted: n.b("use_time_since_last_scouted")?,
        time_since_last_scouted_max: n.f("time_since_last_scouted_max")?,
        time_since_last_scouted_exp: n.f("time_since_last_scouted_exp")?,
        fuzzy_factor: n.f("fuzzy_factor")?,
    })
}

fn load_ego(n: &N<'_>) -> Result<Ego> {
    let mut ego = Ego {
        ids: n.s("IDS")?.ok_or_else(|| anyhow!("ego without IDS"))?,
        priority: n.f("priority")?,
        ..Default::default()
    };
    if let Ok(filters) = n.child("Filters") {
        for item in filters.children("Item")? {
            ego.filters.push(Filter { stat: item.s("stat")?.unwrap_or_default(), min: item.f("min")?, max: item.f("max")? });
        }
    }
    let ma = n.child("MapAnalysis")?;
    ego.cv_influence_exponent = ma.f("cv_influence_exponent")?;
    ego.cv_influence_max_travel_time = ma.f("cv_influence_max_travel_time")?;
    ego.border_influence_range = ma.f("border_influence_range")?;
    let ge = n.child("GoalEngine")?;
    let gs = ge.child("GeneralSettings")?;
    ego.think_frequency = gs.f("think_frequency")?;
    ego.reaction_time = gs.f("reaction_time")?;
    ego.fuzziness = gs.f("fuzziness")?;
    ego.goal_inertia = gs.f("goal_inertia")?;
    ego.immediate_think_on_build_max_settlements = gs.i("immediate_think_on_build_max_settlements")?;
    ego.immediate_think_on_build_max_time = gs.f("immediate_think_on_build_max_time")?;
    ego.goal_priorities = vec![GoalPriority::default(); GOAL_TYPES.len()];
    for gp in ge.children("GoalPriority")? {
        let Some(kind) = gp.s("goal_IDS")? else { continue };
        let Some(index) = GOAL_TYPES.iter().position(|g| g.eq_ignore_ascii_case(&kind)) else {
            return Err(anyhow!("{}: unknown goal type {kind}", gp.path));
        };
        let mut record = GoalPriority {
            multiplier: gp.f("multiplier")?,
            bonus: gp.f("bonus")?,
            minimum: gp.f("minimum")?,
            fuzziness: gp.f("fuzziness")?,
            inertia: gp.f("inertia")?,
            preferences: Vec::new(),
        };
        for ap in gp.children("ActorPreference")? {
            record.preferences.push(ActorPreference {
                property: ap.s("property_IDS")?,
                actor: ap.s("actor_IDS")?,
                value: ap.f("value")?,
                count_value_once_per_company: ap.b("count_value_once_per_company")?,
                multiplier: ap.f("multiplier")?,
                count_multiplier_once_per_goal: ap.b("count_multiplier_once_per_goal")?,
                count_multiplier_once_per_company: ap.b("count_multiplier_once_per_company")?,
            });
        }
        ego.goal_priorities[index] = record;
    }
    let rm = ge.child("ResourceManagement")?;
    ego.saving_time_limit = rm.f("saving_time_limit")?;
    ego.stockpilable_short_need = rm.f("stockpilable_short_need")?;
    ego.stockpilable_enough_need = rm.f("stockpilable_enough_need")?;
    ego.stockpilable_plenty_value = rm.f("stockpilable_plenty_value")?;
    ego.stockpilable_plenty_need = rm.f("stockpilable_plenty_need")?;
    ego.other_short_need = rm.f("other_short_need")?;
    ego.other_enough_need = rm.f("other_enough_need")?;
    ego.other_plenty_value = rm.f("other_plenty_value")?;
    ego.other_plenty_need = rm.f("other_plenty_need")?;
    for need in rm.children("ResourceNeed")? {
        ego.resource_needs.push(ResourceNeed {
            resource: need.s("resource_IDS")?.unwrap_or_default(),
            short_need: need.f("short_need")?,
            enough_need: need.f("enough_need")?,
            plenty_value: need.f("plenty_value")?,
            plenty_need: need.f("plenty_need")?,
        });
    }
    let av = ge.child("ActorValue")?;
    ego.scaling_mult = av.f("scaling_mult")?;
    ego.enemy_influence_mult = av.f("enemy_influence_mult")?;
    ego.min_influence_factor = av.f("min_influence_factor")?;
    ego.influence_exponent = av.f("influence_exponent")?;
    ego.military_cv_mult = av.f("military_cv_mult")?;
    ego.military_value_scaling_mult = av.f("military_value_scaling_mult")?;
    ego.distance_mult_min = av.f("distance_mult_min")?;
    ego.distance_mult_max = av.f("distance_mult_max")?;
    ego.offensive = load_curve(&ge.child("OffensiveCV")?, true)?;
    ego.defensive = load_curve(&ge.child("DefensiveCV")?, false)?;
    let tt = ge.child("TravelTime")?;
    ego.max_travel_time = tt.f("max_travel_time")?;
    ego.travel_time_exponent = tt.f("travel_time_exponent")?;
    for ap in ge.children("ActorPriority")? {
        ego.actor_priorities.push(ActorPriority {
            property: ap.s("property_IDS")?,
            actor: ap.s("actor_IDS")?,
            count_all_property_actors: ap.b("count_all_property_actors")?,
            value: ap.f("value")?,
            repeat_penalty: ap.f("repeat_penalty")?,
            max_recruited_per_think: ap.i("max_recruited_per_think")?,
            one_time_bonus: ap.f("one_time_bonus")?,
        });
    }
    ego.default_max_recruited_per_think = ge.i("default_max_recruited_per_think")?;
    ego.build_queue_max_items = ge.i("build_queue_max_items")?;
    let combat = ge.child("Combat")?;
    let dp = combat.child("DefendPriority")?;
    ego.defend = DefendPriority {
        economic_value_exponent: dp.f("economic_value_exponent")?,
        economic_value_multiplier: dp.f("economic_value_multiplier")?,
        enemy_influence_exponent: dp.f("enemy_influence_exponent")?,
        enemy_influence_multiplier: dp.f("enemy_influence_multiplier")?,
        friendly_border_influence_exponent: dp.f("friendly_border_influence_exponent")?,
        friendly_border_influence_multiplier: dp.f("friendly_border_influence_multiplier")?,
        enemy_border_influence_exponent: dp.f("enemy_border_influence_exponent")?,
        enemy_border_influence_multiplier: dp.f("enemy_border_influence_multiplier")?,
        enemy_forces_threat_distance: dp.f("enemy_forces_threat_distance")?,
        enemy_forces_bonus: dp.f("enemy_forces_bonus")?,
        capture_risk_ratio: dp.f("capture_risk_ratio")?,
        capture_risk_bonus: dp.f("capture_risk_bonus")?,
    };
    let tw = combat.child("Teamwork")?;
    ego.defend_friendly_kingdom_actor_multiplier = tw.f("defend_friendly_kingdom_actor_multiplier")?;
    ego.attack_allied_influence_cv_multiplier = tw.f("attack_allied_influence_cv_multiplier")?;
    ego.defend_allied_influence_cv_multiplier = tw.f("defend_allied_influence_cv_multiplier")?;
    ego.linked_region_distance = combat.i("linked_region_distance")?;
    ego.max_defense_depth = combat.i("max_defense_depth")?;
    ego.attack_commit_distance = combat.i("attack_commit_distance")?;
    ego.congruency_min_multiplier = combat.f("congruency_min_multiplier")?;
    ego.congruency_power = combat.f("congruency_power")?;
    ego.penalty_to_builder_cv = combat.f("penalty_to_builder_cv")?;
    let ex = ge.child("Exploration")?;
    ego.kingdom_center_min_time_owned = ex.f("kingdom_center_min_time_owned")?;
    ego.exploration = [
        load_exploration(&ex.child("CloseGoals")?)?,
        load_exploration(&ex.child("DeepGoals")?)?,
        load_exploration(&ex.child("AirReconGoals")?)?,
    ];
    let un = ge.child("Unpacking")?;
    ego.unpacking = Unpacking {
        adjacent_region_depth: un.i("adjacent_region_depth")?,
        num_random_regions: un.i("num_random_regions")?,
        base_priority: un.f("base_priority")?,
        enemy_static_influence_mult: un.f("enemy_static_influence_mult")?,
        enemy_dynamic_influence_mult: un.f("enemy_dynamic_influence_mult")?,
        max_friendly_building_bonus: un.f("max_friendly_building_bonus")?,
        max_friendly_building_bonus_per_building: un.f("max_friendly_building_bonus_per_building")?,
        max_distance: un.f("max_distance")?,
        distance_exponent: un.f("distance_exponent")?,
        travel_distance_mult: un.f("travel_distance_mult")?,
        travel_distance_exponent: un.f("travel_distance_exponent")?,
        fuzzy_factor: un.f("fuzzy_factor")?,
        inertia: un.f("inertia")?,
    };
    ego.pack_out_of_supply_priority = ge.child("Packing")?.f("pack_out_of_supply_priority")?;
    let rc = ge.child("Recover")?;
    ego.recover = Recover {
        base_priority_hp: rc.f("base_priority_hp")?,
        exponent_hp: rc.f("exponent_hp")?,
        max_hp: rc.f("max_hp")?,
        base_priority_morale: rc.f("base_priority_morale")?,
        exponent_morale: rc.f("exponent_morale")?,
        max_morale: rc.f("max_morale")?,
        critical_morale: rc.f("critical_morale")?,
    };
    let rr = ge.child("Recruiting")?;
    ego.recruiting = Recruiting {
        max_goals_per_think: rr.i("max_goals_per_think")?,
        num_military_goals: rr.i("num_military_goals")?,
        core_cv_multiplier: rr.f("core_cv_multiplier")?,
        core_cv_ratio_multiplier: rr.f("core_cv_ratio_multiplier")?,
        support_cv_multiplier: rr.f("support_cv_multiplier")?,
        support_cv_ratio_multiplier: rr.f("support_cv_ratio_multiplier")?,
        economic_value_multiplier: rr.f("economic_value_multiplier")?,
        decreased_income_penalty: rr.f("decreased_income_penalty")?,
        insufficient_income_level: rr.f("insufficient_income_level")?,
        insufficient_income_penalty: rr.f("insufficient_income_penalty")?,
        empty_slot_value: rr.f("empty_slot_value")?,
        candidate_fuzziness: rr.f("candidate_fuzziness")?,
        matching_builder_bonus: rr.f("matching_builder_bonus")?,
        mismatching_builder_penalty: rr.f("mismatching_builder_penalty")?,
        base_value: rr.f("base_value")?,
        building_bonus: rr.f("building_bonus")?,
        location_fuzziness: rr.f("location_fuzziness")?,
        replace_company_cv_mult: rr.f("replace_company_cv_mult")?,
        requests: Vec::new(),
    };
    for r in rr.children("SpecificRecruitRequest")? {
        ego.recruiting.requests.push(RecruitRequest {
            recruit: r.s("recruit_ids")?,
            property: r.s("property_ids")?,
            number_to_request: r.i("number_to_request")?,
            base_priority: r.f("base_priority")?,
            economic_value_multiplier: r.f("economic_value_multiplier")?,
            repeat_penalty: r.f("repeat_penalty")?,
            one_time_bonus: r.f("one_time_bonus")?,
            hq_type_not_available_bonus: r.f("hq_type_not_available_bonus")?,
        });
    }
    let fl = ge.child("InternalFlares")?;
    ego.flares = Flares {
        defend_attacked_target_duration: fl.f("defend_attacked_target_flare_duration")?,
        defend_attacked_target_priority: fl.f("defend_attacked_target_flare_priority")?,
        attack_region_on_organization_defeated_duration: fl.f("attack_region_on_organization_defeated_flare_duration")?,
        attack_region_on_organization_defeated_priority: fl.f("attack_region_on_organization_defeated_flare_priority")?,
        counter_attack_captured_actor_duration: fl.f("counter_attack_captured_actor_flare_duration")?,
        counter_attack_captured_actor_priority: fl.f("counter_attack_captured_actor_flare_priority")?,
        defend_new_structure_duration: fl.f("defend_new_structure_flare_duration")?,
        defend_new_structure_priority: fl.f("defend_new_structure_flare_priority")?,
        defend_captured_structure_duration: fl.f("defend_captured_structure_flare_duration")?,
        defend_captured_structure_priority: fl.f("defend_captured_structure_flare_priority")?,
    };
    let research = ge.child("Research")?;
    ego.research_base_priority = research.f("base_priority")?;
    for t in research.children("TechPriority")? {
        if let Some(tech) = t.s("tech")? {
            ego.tech_priorities.push((tech, t.f("priority")?));
        }
    }
    let attach = ge.child("Attach")?;
    ego.attach_when_engaged = attach.b("attach_when_engaged")?;
    ego.attach_cv_multiplier = attach.f("cv_multiplier")?;
    let rai = n.child("ReactiveAI")?;
    ego.reactive = Reactive {
        think_frequency: rai.f("think_frequency")?,
        reaction_time: rai.f("reaction_time")?,
        unit_retreat_max_health: rai.f("unit_retreat_max_health")?,
        unit_retreat_depth: rai.i("unit_retreat_depth")?,
        unit_retreat_cv_ratio: rai.f("unit_retreat_cv_ratio")?,
        unit_retreat_probability: rai.f("unit_retreat_probability")?,
        unit_retreat_cycles_til_reconsider: rai.i("unit_retreat_cycles_til_reconsider")?,
        region_retreat_depth: rai.i("region_retreat_depth")?,
        region_retreat_cv_ratio: rai.f("region_retreat_cv_ratio")?,
        region_retreat_probability: rai.f("region_retreat_probability")?,
        region_retreat_cycles_til_reconsider: rai.i("region_retreat_cycles_til_reconsider")?,
        halt_retreat_cv_ratio: rai.f("halt_retreat_cv_ratio")?,
        attack_distance: rai.i("attack_distance")?,
        min_attack_advantage: rai.f("min_attack_advantage")?,
        max_attack_advantage: rai.f("max_attack_advantage")?,
        threatened_region_influence_ratio: rai.f("threatened_region_influence_ratio")?,
    };
    Ok(ego)
}

fn load_profile(n: &N<'_>, egos: &[Ego]) -> Result<Profile> {
    let mut p = Profile {
        ids: n.s("IDS")?.ok_or_else(|| anyhow!("SAI without IDS"))?,
        difficulty_level: n.s("difficulty_level")?.unwrap_or_default(),
        public: n.b("public")?,
        command_max_delay: n.f("command_max_delay")?,
        command_max_total_delay: n.f("command_max_total_delay")?,
        max_ge_commands: n.i("max_ge_commands")?,
        max_rai_commands: n.i("max_rai_commands")?,
        ego_fuzziness: n.f("ego_fuzziness")?,
        ..Default::default()
    };
    let find = |ids: &str| egos.iter().position(|e| e.ids.eq_ignore_ascii_case(ids));
    if let Some(initial) = n.s("initial_ego")? {
        p.egos.push(find(&initial).ok_or_else(|| anyhow!("unknown initial ego {initial}"))?);
    }
    for e in n.children("Ego")? {
        if let Some(ids) = e.s("IDS")? {
            let index = find(&ids).ok_or_else(|| anyhow!("unknown ego {ids}"))?;
            if !p.egos.contains(&index) {
                p.egos.push(index);
            }
        }
    }
    for e in n.children("Nation")? {
        if let Some(ids) = e.s("IDS")? {
            p.nations.push(ids);
        }
    }
    for e in n.children("Faction")? {
        if let Some(ids) = e.s("ids")? {
            p.factions.push(ids);
        }
    }
    Ok(p)
}
