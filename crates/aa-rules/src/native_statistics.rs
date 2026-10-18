//! StatisticData producer 58E790/4B5340/4B5020/4B5440 and source facts
//! consumed by statistics tracking. No snapshot arithmetic is performed here.
//! Derived terms are returned in native head-list order (reverse of the TGI
//! ordinal iterator), and factors retain the GetFixed binary32 store.
use crate::{
    actors::{ActorCatalog, ResolvedNode},
    native::{Error, Result, TypedValue},
    resources::Provenance,
};
use aa_numeric::Precision;
use serde::Serialize;
use std::collections::BTreeMap;

/// Complete original 58E1A0 constructor order, observed by native execution.
pub const PREDEFINED_STATISTICS: &[&str] = &[
    "TRIBUTE_GIVEN",
    "TRIBUTE_RECEIVED",
    "TECHS_RESEARCHED",
    "ORGANIZATIONS_RECRUITED",
    "ORGANIZATIONS_KILLED",
    "ORGANIZATIONS_LOST",
    "ORGANIZATION_XP_TOTAL_EARNED",
    "ORGANIZATION_XP_MAX",
    "UNITS_PRODUCED",
    "UNITS_KILLED",
    "UNITS_LOST",
    "TECH_UNITS_PRODUCED",
    "TECH_UNITS_KILLED",
    "TECH_UNITS_LOST",
    "HEROES_PRODUCED",
    "HEROES_KILLED",
    "HEROES_LOST",
    "HERO_XP_TOTAL_EARNED",
    "HERO_XP_MAX",
    "STRUCTURES_BUILT",
    "STRUCTURES_UPGRADED",
    "STRUCTURES_KILLED",
    "STRUCTURES_LOST",
    "SETTLEMENTS_BUILT",
    "SETTLEMENTS_UPGRADED",
    "SETTLEMENTS_CAPTURED",
    "SETTLEMENTS_SURRENDERED",
    "TECH_BUILDINGS_BUILT",
    "TECH_BUILDINGS_CAPTURED",
    "TECH_BUILDINGS_SURRENDERED",
    "TECH_BUILDINGS_KILLED",
    "TECH_BUILDINGS_LOST",
    "TECH_BUILDINGS_POSSESSION_TIME",
    "TECH_BUILDINGS_MAX_OWNED",
    "DEFENSE_BUILDINGS_BUILT",
    "DEFENSE_BUILDINGS_UPGRADED",
    "DEFENSE_BUILDINGS_KILLED",
    "DEFENSE_BUILDINGS_LOST",
    "RESOURCE_BUILDINGS_BUILT",
    "RESOURCE_BUILDINGS_UPGRADED",
    "RESOURCE_BUILDINGS_KILLED",
    "RESOURCE_BUILDINGS_LOST",
    "CITIES_CAPTURED",
    "CITIES_SURRENDERED",
    "EXPERIENCE_GAINED",
    "EXPERIENCE_SPENT",
    "FACTION_ABILITIES_USED",
    "ORGANIZATIONS_OWNED",
    "STRUCTURES_OWNED",
    "STRUCTURE_UPGRADES_OWNED",
    "SETTLEMENTS_OWNED",
    "SETTLEMENT_LEVELS_OWNED",
    "TECH_BUILDINGS_OWNED",
    "BUILDINGS_OWNED",
    "BUILDING_UPGRADES_OWNED",
    "CITIES_OWNED",
    "HERO_INVENTORY_SIZE",
    "FAVORITE_UNIT",
    "FAVORITE_HERO",
    "FAVORITE_BUILDING",
    "FAVORITE_ENEMY",
    "FAVORITE_ORGANIZATION",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StatisticTerm {
    pub stat_index: u32,
    pub weight_bits: u32,
}
#[derive(Clone, Debug, Serialize)]
pub struct StatisticDefinition {
    pub ids: String,
    pub index: u32,
    /// +18: INT=0, FIXED=1, STRING=2; constructor leaves unloaded rows 3.
    pub display_type: u32,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct DerivedStatistic {
    pub ids: String,
    pub index: u32,
    /// Original +1C linked list, traversed by 58F420; NOT authored order.
    pub terms: Vec<StatisticTerm>,
}
#[derive(Clone, Debug, Serialize)]
pub struct NativeStatisticsCatalog {
    pub resource_ids: Vec<String>,
    pub production_start: usize,
    pub rate_start: usize,
    pub derived_start: usize,
    pub definitions: Vec<StatisticDefinition>,
    pub derived: Vec<DerivedStatistic>,
}

fn text(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<String> {
    match node.field(name, pc)? {
        Some(TypedValue::String { value, .. }) => Ok(value),
        _ => Err(Error::Unsupported(format!(
            "Statistic {name} requires assigned string"
        ))),
    }
}

impl ActorCatalog {
    /// Fresh original Statistic manager, plus the supplied native-order roots.
    /// Duplicate/reload and Statistic translations are explicit unsupported
    /// cases; never substitute authored root ordinal for a native stat index.
    pub fn native_statistics_catalog(&self, pc: Precision) -> Result<NativeStatisticsCatalog> {
        let resource_registry = self.native_data_registry("Resource").ok_or_else(|| {
            Error::Unsupported("statistics require native-order Resource registry".into())
        })?;
        let resource_ids: Vec<_> = resource_registry
            .entries
            .iter()
            .map(|e| e.ids.clone())
            .collect();
        for entry in &resource_registry.entries {
            self.native_data_node(entry)?; // Reject unmodeled resource reloads.
        }
        for node in self.resolved_definitions("Translate")? {
            if text(&node, "node", pc)?.eq_ignore_ascii_case("Statistic") {
                return Err(Error::Unsupported(
                    "Statistic translation registry not bound".into(),
                ));
            }
        }
        let production_start = PREDEFINED_STATISTICS.len();
        let rate_start = production_start + resource_ids.len();
        let derived_start = rate_start + resource_ids.len();
        let mut definitions: Vec<_> = PREDEFINED_STATISTICS
            .iter()
            .enumerate()
            .map(|(index, ids)| StatisticDefinition {
                ids: (*ids).into(),
                index: index as u32,
                display_type: 3,
                sources: Vec::new(),
            })
            .collect();
        let mut names: BTreeMap<_, _> = definitions
            .iter()
            .map(|r| (r.ids.to_ascii_lowercase(), r.index as usize))
            .collect();
        let mut loaded = vec![false; definitions.len()];
        let mut derived = Vec::new();
        for node in self.resolved_definitions("Statistic")? {
            let ids = text(&node, "ids", pc)?;
            if ids.is_empty() {
                return Err(Error::Unsupported("empty Statistic IDS".into()));
            }
            let index = match names.get(&ids.to_ascii_lowercase()) {
                Some(&i) => i,
                None => {
                    let i = definitions.len();
                    names.insert(ids.to_ascii_lowercase(), i);
                    definitions.push(StatisticDefinition {
                        ids: ids.clone(),
                        index: i as u32,
                        display_type: 3,
                        sources: Vec::new(),
                    });
                    loaded.push(false);
                    i
                }
            };
            if loaded[index] {
                return Err(Error::Unsupported(format!(
                    "Statistic duplicate/reload {ids} is unmodeled"
                )));
            }
            // Base LoadData47D960 calls the localized getter. Resolve storage
            // type, while localization itself is a presentation-only seam.
            node.localized("name")?.ok_or_else(|| {
                Error::Unsupported(format!("Statistic {ids} lacks assigned localized name"))
            })?;
            let display_type = match text(&node, "display_type", pc)?
                .to_ascii_uppercase()
                .as_str()
            {
                "INT" => 0,
                "FIXED" => 1,
                "STRING" => 2,
                _ => {
                    return Err(Error::Unsupported(format!(
                        "unknown Statistic display_type for {ids}"
                    )));
                }
            };
            let kind = text(&node, "stat_type", pc)?.to_ascii_uppercase();
            if (kind == "FAVORITE") != (display_type == 2) {
                return Err(Error::Unsupported(format!(
                    "Statistic {ids} display/type mismatch"
                )));
            }
            definitions[index].display_type = display_type;
            definitions[index].sources = node.sources();
            match kind.as_str() {
                "VARIABLE" | "FAVORITE" if index < production_start => {}
                "PRODUCTION" | "RATE" => {
                    let resource = text(&node, "resource_ids", pc)?;
                    let entry = resource_registry.find(&resource).ok_or_else(|| {
                        Error::Unsupported(format!("unknown Statistic resource {resource}"))
                    })?;
                    let resource_index = resource_registry
                        .entries
                        .iter()
                        .position(|r| std::ptr::eq(r, entry))
                        .expect("registry identity");
                    let start = if kind == "PRODUCTION" {
                        production_start
                    } else {
                        rate_start
                    };
                    if index != start + resource_index {
                        return Err(Error::Unsupported(format!(
                            "Statistic resource row {ids} is out of order"
                        )));
                    }
                }
                "SCORE" => {
                    if index != derived_start + derived.len() {
                        return Err(Error::Unsupported(format!(
                            "Statistic score {ids} is out of order"
                        )));
                    }
                    let mut terms = Vec::new();
                    for child in node.children("Statistic")? {
                        let stat_ids = text(&child, "stat_ids", pc)?;
                        let stat_index =
                            *names.get(&stat_ids.to_ascii_lowercase()).ok_or_else(|| {
                                Error::Unsupported(format!("unknown stat {stat_ids} in {ids}"))
                            })?;
                        if definitions[stat_index].display_type == 2 {
                            return Err(Error::Unsupported(format!(
                                "string stat {stat_ids} in {ids}"
                            )));
                        }
                        let weight_bits = child
                            .field("factor", pc)?
                            .ok_or_else(|| {
                                Error::Unsupported(format!("missing score factor in {ids}"))
                            })?
                            .fixed_bits()?;
                        terms.push(StatisticTerm {
                            stat_index: stat_index as u32,
                            weight_bits,
                        });
                    }
                    if terms.is_empty() {
                        return Err(Error::Unsupported(format!("score {ids} has no Statistics")));
                    }
                    terms.reverse();
                    derived.push(DerivedStatistic {
                        ids: ids.clone(),
                        index: index as u32,
                        terms,
                    });
                }
                _ => {
                    return Err(Error::Unsupported(format!(
                        "unsupported Statistic type/index {kind}/{index}"
                    )));
                }
            }
            loaded[index] = true;
        }
        if definitions.len() < derived_start
            || loaded[production_start..derived_start].iter().any(|&v| !v)
        {
            return Err(Error::Unsupported(
                "incomplete resource Statistic production/rate rows".into(),
            ));
        }
        Ok(NativeStatisticsCatalog {
            resource_ids,
            production_start,
            rate_start,
            derived_start,
            definitions,
            derived,
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct StatisticsTrackingDefinition {
    pub id: String,
    /// +4D0; false is established by the complete Upgrade census, not defaulted.
    pub predecessor_nonempty: bool,
    /// +43C, TechnologyProperty canonical IDS in native head-list order.
    pub technology_property_ids: Vec<String>,
    /// +4E0; canonical required MARKER definition after source validation.
    pub required_object_ids: Option<String>,
    pub sources: Vec<Provenance>,
}

impl ActorCatalog {
    /// 46797F -> 4C4CF0, ordered TechnologyProperty references at +43C.
    pub fn native_statistics_technology_properties(
        &self,
        id: &str,
        pc: Precision,
    ) -> Result<Vec<String>> {
        let actor = self
            .find(id)
            .ok_or_else(|| Error::Unsupported(format!("missing actor {id}")))?;
        if !actor.id.eq_ignore_ascii_case(id) || self.native_category(id)?.native_kind != 2 {
            return Err(Error::Unsupported(
                "statistics require unshadowed canonical ActorData identity".into(),
            ));
        }
        for node in self.resolved_definitions("Translate")? {
            if text(&node, "node", pc)?.eq_ignore_ascii_case("Property") {
                return Err(Error::Unsupported(
                    "Property translation registry not bound".into(),
                ));
            }
        }
        let node = self.resolved_actor(id)?;
        let mut ids = Vec::new();
        if let Some(structure) = node.child("StructureComponent")? {
            for child in structure.children("TechnologyProperty")? {
                let name = text(&child, "property_IDS", pc)?;
                let property = self
                    .native_named_definition("Property", &name, pc)?
                    .ok_or_else(|| {
                        Error::Unsupported(format!("unknown TechnologyProperty {name}"))
                    })?;
                ids.push(text(&property, "IDS", pc)?);
            }
            ids.reverse();
        }
        Ok(ids)
    }
    /// Bounded fresh ActorData facts for 58F010/58EE00. Inherits the existing
    /// collision source validator's explicit nonempty Upgrade/Settlement-wall
    /// restrictions. Does not fabricate facts for unresolved source callbacks.
    pub fn native_statistics_tracking_definition(
        &self,
        id: &str,
        pc: Precision,
    ) -> Result<StatisticsTrackingDefinition> {
        let actor = self
            .find(id)
            .ok_or_else(|| Error::Unsupported(format!("missing actor {id}")))?;
        if !actor.id.eq_ignore_ascii_case(id) {
            return Err(Error::Unsupported(
                "statistics require unshadowed canonical ActorData identity".into(),
            ));
        }
        let collision = self.native_collision_definition(id, pc)?;
        let node = self.resolved_actor(id)?;
        let technology_property_ids = self.native_statistics_technology_properties(id, pc)?;
        Ok(StatisticsTrackingDefinition {
            id: collision.id,
            predecessor_nonempty: false,
            technology_property_ids,
            required_object_ids: collision.required_object.map(|v| v.id),
            sources: node.sources(),
        })
    }
}
