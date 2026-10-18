//! Sub-loaders of the original ego loader `4B0560` (see `ego.rs`): Filters
//! `4AD980`, GoalPriority `4AF500`, ResourceNeed `4AF230`, ComponentValue
//! `4AAD80`, ActorPriority `4AF8E0`, exploration goals `4A66E0`,
//! RecruitPreference `4ADF50`, SpecificBuildingBonus `4AB090`,
//! SpecificRecruitRequest `4AAEB0` (`4A7C70`), general powers (inline block,
//! `4AFD50`, `4AFB60`), formations `4AEDC0`, GoalFormation `4AF0C0`, and the
//! inline WallStrength and CombinedArms blocks.
use super::data::{NodeId, f32_of, required};
use super::ego::{
    ActorPreference, ActorRequirements, DataRef, Ego, EgoServices, GeneralPower, RecruitMatch,
    RefField, ThingPriority, ThingSource, for_items,
};
use super::profile::Manager;
use super::tables::{COMPONENT_TYPES, GOAL_ACTOR_PROXIMITY, GOAL_TYPES, lookup};
use crate::{Error, Result, x87::num};
use aa_numeric::Binary;
use std::cmp::Ordering;

const ONE: u32 = 0x3f80_0000;
const MINUS_ONE: u32 = 0xbf80_0000;

fn f32_field(d: &mut impl EgoServices, node: NodeId, name: &str) -> Result<u32> {
    f32_of(&d.fixed(node, name)?)
}
fn compare(v: &Binary, bits: u32) -> Result<Ordering> {
    Ok(v.compare(&num(Binary::from_f32_bits(bits))?))
}
fn ego_ids(e: &Ego) -> String {
    e.ids.clone()
}

/// `4AD980` on ego+1C: `instances_max`, then `Item` DataRefs (A17038
/// `{ego+1C, min, max}`) named by `stat`.
pub fn filters(d: &mut impl EgoServices, node: NodeId, e: &mut Ego) -> Result<()> {
    e.set_u32(0x1c, f32_field(d, node, "instances_max")?);
    for_items(d, node, "Item", |d, i, count| {
        let item = d.item(node, "Item", i, count)?;
        let stat = d.string(item, "stat")?;
        let item = d.item(node, "Item", i, count)?;
        let min = f32_field(d, item, "min")?;
        let item = d.item(node, "Item", i, count)?;
        let max = f32_field(d, item, "max")?;
        e.refs.push(DataRef {
            vtable: 0x00a1_7038,
            ids: stat,
            fields: vec![
                RefField::Ego(0x1c),
                RefField::Word(min),
                RefField::Word(max),
            ],
        });
        Ok(())
    })
}

/// `4AF500`: one GoalPriority into the slot of its goal type.
pub fn goal_priority(d: &mut impl EgoServices, node: NodeId, e: &mut Ego) -> Result<()> {
    let goal = d.string(node, "goal_IDS")?;
    let Some(t) = lookup(&GOAL_TYPES, &goal) else {
        return Err(d.data_error("Unrecognized SAI goal type(%s) in goal priority", &[goal]));
    };
    let t = t as usize;
    if !e.goal_slot_is_default(t) {
        return Err(d.data_error("%s assigned duplicate priority", &[goal]));
    }
    let s = 0x4c + t * 0x18;
    e.set_u32(s, f32_field(d, node, "bonus")?);
    e.set_u32(s + 4, f32_field(d, node, "multiplier")?);
    e.set_u32(s + 8, f32_field(d, node, "minimum")?);
    e.set_u32(s + 0xc, f32_field(d, node, "fuzziness")?);
    e.set_u32(s + 0x10, f32_field(d, node, "inertia")?);
    let count = d.count(node, "ActorPreference")?;
    if count > 0 {
        e.goal_preferences[t] = Some(Vec::new());
    }
    let mut i = 0;
    while i < count && i >= 0 {
        let item = d.item(node, "ActorPreference", i, count)?;
        let property = d.string(item, "property_IDS")?;
        let item = d.item(node, "ActorPreference", i, count)?;
        let actor = d.string(item, "actor_IDS")?;
        if actor.is_empty() == property.is_empty() {
            return Err(d.data_error(
                "ActorPreference (in GoalPriority, %s) must specify a property or an actor, but not both.",
                &[ego_ids(e)],
            ));
        }
        // The record (0x1C) is linked at the list head, then filled.
        let mut pref = ActorPreference {
            property: 0,
            actor: 0,
            value: 0,
            value_once_per_company: 1,
            multiplier: ONE,
            multiplier_once_per_goal: 1,
            multiplier_once_per_company: 1,
        };
        if !property.is_empty() {
            pref.property = d.property(&property)?.unwrap_or(0);
        } else {
            pref.actor = d.thing(ThingSource::Global, &actor)?.unwrap_or(0);
        }
        let item = d.item(node, "ActorPreference", i, count)?;
        pref.value = f32_field(d, item, "value")?;
        let item = d.item(node, "ActorPreference", i, count)?;
        pref.value_once_per_company = u8::from(d.boolean(item, "count_value_once_per_company")?);
        let item = d.item(node, "ActorPreference", i, count)?;
        pref.multiplier = f32_field(d, item, "multiplier")?;
        let item = d.item(node, "ActorPreference", i, count)?;
        pref.multiplier_once_per_goal =
            u8::from(d.boolean(item, "count_multiplier_once_per_goal")?);
        let item = d.item(node, "ActorPreference", i, count)?;
        pref.multiplier_once_per_company =
            u8::from(d.boolean(item, "count_multiplier_once_per_company")?);
        e.goal_preferences[t]
            .as_mut()
            .expect("allocated before the loop")
            .insert(0, pref);
        i += 1;
    }
    Ok(())
}

/// `4AF230` on the ResourceManagement section: the default needs, then the
/// `ResourceNeed` records by Resource registry index (new entries -1).
pub fn resource_needs(d: &mut impl EgoServices, node: NodeId, e: &mut Ego) -> Result<()> {
    for (off, name) in [
        (0x284, "stockpilable_short_need"),
        (0x288, "stockpilable_enough_need"),
        (0x28c, "stockpilable_plenty_value"),
        (0x290, "stockpilable_plenty_need"),
        (0x294, "other_short_need"),
        (0x298, "other_enough_need"),
        (0x29c, "other_plenty_value"),
        (0x2a0, "other_plenty_need"),
    ] {
        e.set_u32(off, f32_field(d, node, name)?);
    }
    for_items(d, node, "ResourceNeed", |d, i, count| {
        let item = d.item(node, "ResourceNeed", i, count)?;
        let ids = d.string(item, "resource_IDS")?;
        if ids.is_empty() {
            return Err(d.data_error("Resource Needs must specify a resource.", &[]));
        }
        let Some((_, index)) = d.find(Manager::Resources, &ids)? else {
            return Err(d.data_error(
                "Resource (%s) not found or resources not yet read in",
                &[ids],
            ));
        };
        if index < 0 {
            return Err(Error::World(format!(
                "negative Resource registry index {index}"
            )));
        }
        while e.resource_needs.len() <= index as usize {
            e.resource_needs.push([MINUS_ONE; 4]);
        }
        let mut need = [0u32; 4];
        for (k, name) in ["short_need", "enough_need", "plenty_value", "plenty_need"]
            .into_iter()
            .enumerate()
        {
            need[k] = f32_field(d, item, name)?;
        }
        e.resource_needs[index as usize] = need;
        Ok(())
    })
}

/// `4AAD80`: one ComponentValue (`{value, new_construction_only}` at its
/// component type; growth `4A6C40` constructs `{0, 1}`).
pub fn component_value(d: &mut impl EgoServices, node: NodeId, e: &mut Ego) -> Result<()> {
    let component = d.string(node, "component_IDS")?;
    let value = f32_field(d, node, "value")?;
    let new_only = u8::from(d.boolean(node, "new_construction_only")?);
    let Some(t) = lookup(&COMPONENT_TYPES, &component) else {
        return Err(d.data_error("Component type (%s) not found", &[component]));
    };
    while e.component_values.len() <= t as usize {
        e.component_values.push((0, 1));
    }
    e.component_values[t as usize] = (value, new_only);
    Ok(())
}

/// `4AF8E0`: one ActorPriority into the +374 (Thing) or +384 (Property) table.
pub fn actor_priority(d: &mut impl EgoServices, node: NodeId, e: &mut Ego) -> Result<()> {
    let property = d.string(node, "property_IDS")?;
    let actor = d.string(node, "actor_IDS")?;
    let priority = ThingPriority {
        count_all_property_actors: u8::from(d.boolean(node, "count_all_property_actors")?),
        value: f32_field(d, node, "value")?,
        repeat_penalty: f32_field(d, node, "repeat_penalty")?,
        max_recruited_per_think: d.int(node, "max_recruited_per_think")?,
        one_time_bonus: f32_field(d, node, "one_time_bonus")?,
    };
    if actor.is_empty() == property.is_empty() {
        return Err(d.data_error(
            "Thing priorities must specify a property or a structure, but not both.",
            &[],
        ));
    }
    if !actor.is_empty() {
        let Some(thing) = d.thing(ThingSource::Global, &actor)? else {
            return Err(d.data_error("Unrecognized actor (%s) in ego (%s)", &[actor, ego_ids(e)]));
        };
        if e.actor_priorities.contains(thing)? {
            return Err(d.data_error(
                "Duplicate actor priority (%s) in ego (%s)",
                &[actor, ego_ids(e)],
            ));
        }
        e.actor_priorities.insert(thing, priority)
    } else {
        let Some(handle) = d.property(&property)? else {
            return Err(d.data_error(
                "Unrecognized property (%s) in ego (%s)",
                &[property, ego_ids(e)],
            ));
        };
        if e.property_priorities.contains(handle)? {
            return Err(d.data_error(
                "Duplicate property priority (%s) in ego (%s)",
                &[property, ego_ids(e)],
            ));
        }
        e.property_priorities.insert(handle, priority)
    }
}

/// The inline `SettlementLevel` block of WallStrength: the level and the
/// stored binary32 value must not be negative; new levels are filled with -1
/// and a level already holding a positive value (or 0 above level 0) is a
/// duplicate.
pub fn wall_strength(d: &mut impl EgoServices, node: NodeId, e: &mut Ego) -> Result<()> {
    for_items(d, node, "SettlementLevel", |d, i, count| {
        let item = d.item(node, "SettlementLevel", i, count)?;
        let level = d.int(item, "level")?;
        let item = d.item(node, "SettlementLevel", i, count)?;
        let value = f32_field(d, item, "value")?;
        if level < 0 {
            return Err(d.data_error(
                "Wall strength base level in ego (%s) must be >= 0.",
                &[ego_ids(e)],
            ));
        }
        if f32::from_bits(value) < 0.0 {
            return Err(d.data_error(
                "Wall strength base value in ego (%s) must be >= 0.",
                &[ego_ids(e)],
            ));
        }
        while e.wall_strength.len() <= level as usize {
            e.wall_strength.push(MINUS_ONE);
        }
        let entry = f32::from_bits(e.wall_strength[level as usize]);
        if entry > 0.0 || (entry == 0.0 && level != 0) {
            return Err(d.data_error(
                "Duplicate wall strength base level in ego (%s).",
                &[ego_ids(e)],
            ));
        }
        e.wall_strength[level as usize] = value;
        Ok(())
    })
}

/// The inline `Element` block of CombinedArmsGeneral / CombinedArmsWalls:
/// DataRefs A17048 `{ego+owner, max_percent, max_multiplier}` by `property`.
pub fn combined_arms(
    d: &mut impl EgoServices,
    node: NodeId,
    e: &mut Ego,
    owner: u32,
) -> Result<()> {
    for_items(d, node, "Element", |d, i, count| {
        let item = d.item(node, "Element", i, count)?;
        let property = d.string(item, "property")?;
        let item = d.item(node, "Element", i, count)?;
        let max_percent = f32_field(d, item, "max_percent")?;
        let item = d.item(node, "Element", i, count)?;
        let max_multiplier = f32_field(d, item, "max_multiplier")?;
        e.refs.push(DataRef {
            vtable: 0x00a1_7048,
            ids: property,
            fields: vec![
                RefField::Ego(owner),
                RefField::Word(max_percent),
                RefField::Word(max_multiplier),
            ],
        });
        Ok(())
    })
}

#[derive(Clone, Copy)]
enum Kind {
    Fixed,
    Int,
    Bool,
}

/// `4A66E0` field order (offset in the 0xC4 exploration-goal struct).
const EXPLORATION: &[(usize, Kind, &str)] = &[
    (0x00, Kind::Int, "num_goals"),
    (0x04, Kind::Fixed, "base_priority"),
    (0x08, Kind::Fixed, "repeat_penalty"),
    (0x0c, Kind::Fixed, "min_priority"),
    (0x10, Kind::Fixed, "percent_to_count_region_explored"),
    (0x14, Kind::Fixed, "move_rate_mult"),
    (0x18, Kind::Fixed, "move_rate_exp"),
    (0x1c, Kind::Fixed, "sight_mult"),
    (0x20, Kind::Fixed, "sight_exp"),
    (0x24, Kind::Bool, "subtract_cv"),
    (0x28, Kind::Fixed, "cv_mult"),
    (0x2c, Kind::Fixed, "cv_exp"),
    (0x30, Kind::Fixed, "unit_inertia"),
    (0x34, Kind::Fixed, "intuition_probability"),
    (0x38, Kind::Int, "intuition_distance"),
    (0x3c, Kind::Bool, "intuition_count_mines"),
    (0x3d, Kind::Bool, "intuition_count_settlement_spots"),
    (0x3e, Kind::Bool, "intuition_count_settlements"),
    (0x3f, Kind::Bool, "intuition_count_other_buildings"),
    (0x40, Kind::Int, "num_random_candidates"),
    (0x44, Kind::Fixed, "black_fog_priority"),
    (0x48, Kind::Fixed, "grey_fog_priority"),
    (0x4c, Kind::Int, "fog_search_depth"),
    (0x50, Kind::Fixed, "current_region_bonus"),
    (0x54, Kind::Fixed, "actor_region_bonus"),
    (0x58, Kind::Fixed, "building_region_bonus"),
    (0x5c, Kind::Fixed, "flare_region_bonus"),
    (0x60, Kind::Fixed, "intuition_region_bonus"),
    (0x64, Kind::Fixed, "intuition_region_exponent"),
    (0x68, Kind::Fixed, "random_candidate_bonus"),
    (0x6c, Kind::Bool, "consider_distance_to_kingdom_center"),
    (0x70, Kind::Fixed, "kingdom_center_distance_cap"),
    (0x74, Kind::Fixed, "kingdom_center_distance_max"),
    (0x78, Kind::Fixed, "kingdom_center_distance_exp"),
    (0x7c, Kind::Fixed, "kingdom_center_distance_ceil"),
    (0x80, Kind::Fixed, "kingdom_center_distance_floor"),
    (0x84, Kind::Bool, "kingdom_center_distance_prefer_close"),
    (0x85, Kind::Bool, "consider_distance_to_last_region"),
    (0x88, Kind::Fixed, "last_region_distance_cap"),
    (0x8c, Kind::Fixed, "last_region_distance_max"),
    (0x90, Kind::Fixed, "last_region_distance_exp"),
    (0x94, Kind::Fixed, "last_region_distance_ceil"),
    (0x98, Kind::Fixed, "last_region_distance_floor"),
    (0x9c, Kind::Bool, "last_region_distance_prefer_close"),
    (0x9d, Kind::Bool, "consider_distance_to_actor"),
    (0xa0, Kind::Fixed, "actor_distance_cap"),
    (0xa4, Kind::Fixed, "actor_distance_max"),
    (0xa8, Kind::Fixed, "actor_distance_exp"),
    (0xac, Kind::Fixed, "actor_distance_ceil"),
    (0xb0, Kind::Fixed, "actor_distance_floor"),
    (0xb4, Kind::Bool, "actor_distance_prefer_close"),
    (0xb5, Kind::Bool, "use_time_since_last_scouted"),
    (0xb8, Kind::Fixed, "time_since_last_scouted_max"),
    (0xbc, Kind::Fixed, "time_since_last_scouted_exp"),
    (0xc0, Kind::Fixed, "fuzzy_factor"),
];

/// `4A66E0`: an exploration-goal struct at `base` (+464 / +528 / +5EC).
pub fn exploration_goals(
    d: &mut impl EgoServices,
    node: NodeId,
    e: &mut Ego,
    base: usize,
) -> Result<()> {
    for &(off, kind, name) in EXPLORATION {
        match kind {
            Kind::Fixed => {
                let v = f32_field(d, node, name)?;
                e.set_u32(base + off, v);
            }
            Kind::Int => {
                let v = d.int(node, name)?;
                e.set_i32(base + off, v);
            }
            Kind::Bool => {
                let v = d.boolean(node, name)?;
                e.set_u8(base + off, u8::from(v));
            }
        }
    }
    Ok(())
}

/// `4ADF50`: a RecruitPreference (Things from `[[B36530]+0C]`): the +704
/// list of the preferred Thing gains its `Match` entries at the head.
pub fn recruit_preference(d: &mut impl EgoServices, node: NodeId, e: &mut Ego) -> Result<()> {
    let ids = d.string(node, "unit_ids")?;
    let Some(thing) = d.thing(ThingSource::Root, &ids)? else {
        return Err(d.data_error("Unknown actor IDS or data read out of order (%s).", &[ids]));
    };
    e.recruit_preferences.find_or_insert(thing, Vec::new())?;
    let count = d.count(node, "Match")?;
    if count <= 0 {
        return Err(d.data_error("Actor preference for (%s) has no matches defined.", &[ids]));
    }
    let mut i = 0;
    while i < count && i >= 0 {
        let item = d.item(node, "Match", i, count)?;
        let unit = d.string(item, "unit_ids")?;
        let Some(matched) = d.thing(ThingSource::Root, &unit)? else {
            return Err(d.data_error("Unknown actor IDS or data read out of order (%s).", &[unit]));
        };
        let item = d.item(node, "Match", i, count)?;
        let priority = f32_field(d, item, "priority")?;
        e.recruit_preferences
            .get_mut(thing)?
            .expect("inserted above")
            .insert(
                0,
                RecruitMatch {
                    thing: matched,
                    priority,
                },
            );
        i += 1;
    }
    Ok(())
}

/// `4AB090`: a SpecificBuildingBonus into +73C.
pub fn building_bonus(d: &mut impl EgoServices, node: NodeId, e: &mut Ego) -> Result<()> {
    let ids = d.string(node, "building_ids")?;
    let bonus = f32_field(d, node, "bonus")?;
    if ids.is_empty() {
        return Err(d.data_error("Recruiting building bonuses must specify an IDS.", &[]));
    }
    let Some(thing) = d.thing(ThingSource::Global, &ids)? else {
        return Err(d.data_error("Unrecognized building: %s", &[ids]));
    };
    if e.building_bonuses.contains(thing)? {
        return Err(d.data_error(
            "Duplicate recruiting building bonus for (%s) in ego (%s).",
            &[ids, ego_ids(e)],
        ));
    }
    *e.building_bonuses.find_or_insert(thing, 0)? = bonus;
    Ok(())
}

/// `4AAEB0` -> `4A7C70`: a SpecificRecruitRequest DataRef (A1704C, 0x2C)
/// `{ego, number, base_priority, economic_value_multiplier, repeat_penalty,
/// one_time_bonus, is_recruit, hq_type_not_available_bonus}` named by the
/// recruit (flag 1) or the property (flag 0).
pub fn recruit_request(d: &mut impl EgoServices, node: NodeId, e: &mut Ego) -> Result<()> {
    let recruit = d.string(node, "recruit_ids")?;
    let property = d.string(node, "property_IDS")?;
    let number = d.int(node, "number_to_request")?;
    let base = f32_field(d, node, "base_priority")?;
    let economic = f32_field(d, node, "economic_value_multiplier")?;
    let repeat = f32_field(d, node, "repeat_penalty")?;
    let once = f32_field(d, node, "one_time_bonus")?;
    let hq = f32_field(d, node, "hq_type_not_available_bonus")?;
    if recruit.is_empty() == property.is_empty() {
        return Err(d.data_error(
            "Specific recruit requests must specify a property or a recruit, but not both.",
            &[],
        ));
    }
    let (ids, flag) = if recruit.is_empty() {
        (property, 0)
    } else {
        (recruit, 1)
    };
    e.refs.push(DataRef {
        vtable: 0x00a1_704c,
        ids,
        fields: vec![
            RefField::Ego(0),
            RefField::Word(number as u32),
            RefField::Word(base),
            RefField::Word(economic),
            RefField::Word(repeat),
            RefField::Word(once),
            RefField::Byte(flag),
            RefField::Word(hq),
        ],
    });
    Ok(())
}

/// The inline GeneralPowers block: +884 sized to the AbilityDefinition
/// registry (null-filled), then one `Power` per ability.
pub fn general_powers(d: &mut impl EgoServices, node: NodeId, e: &mut Ego) -> Result<()> {
    let n = d.registry_count(Manager::Abilities)?;
    if n < 0 {
        return Err(Error::World(format!(
            "negative AbilityDefinition registry size {n}"
        )));
    }
    e.general_powers = (0..n).map(|_| None).collect();
    for_items(d, node, "Power", |d, i, count| {
        let item = d.item(node, "Power", i, count)?;
        let ids = d.string(item, "ability_ids")?;
        let index = d.find(Manager::Abilities, &ids)?.map_or(-1, |(_, i)| i);
        if index < 0 || index > e.general_powers.len() as i32 {
            return Err(d.data_error("Bad ability ids (%s) in ego (%s)", &[ids, ego_ids(e)]));
        }
        if index as usize == e.general_powers.len() {
            return Err(Error::World(format!(
                "ability index {index} reads past the +884 vector (native out-of-bounds read)"
            )));
        }
        if e.general_powers[index as usize].is_some() {
            return Err(d.data_error(
                "Duplicate general power (%s) in ego (%s)",
                &[ids, ego_ids(e)],
            ));
        }
        let mut power = GeneralPower::new();
        let item = d.item(node, "Power", i, count)?;
        general_power(d, item, &mut power)?;
        e.general_powers[index as usize] = Some(power);
        Ok(())
    })
}

/// `4AFB60`: GeneralPowerActorRequirements.
pub fn actor_requirements(
    d: &mut impl EgoServices,
    node: NodeId,
    r: &mut ActorRequirements,
) -> Result<()> {
    let health = d.fixed(node, "max_percent_health")?;
    r.max_percent_health = f32_of(&health)?;
    if compare(&health, 0)? != Ordering::Greater || compare(&health, ONE)? == Ordering::Greater {
        return Err(d.data_error(
            "Invalid max_percent_health (%g) in GeneralPowerActorRequirements",
            &[],
        ));
    }
    r.flags[0] = u8::from(d.boolean(node, "must_be_engaged")?);
    r.flags[1] = u8::from(d.boolean(node, "must_not_be_engaged")?);
    if r.flags[0] != 0 && r.flags[1] != 0 {
        return Err(d.data_error(
            "must_be_engaged and must_not_be_engaged both true in GeneralPowerActorRequirements",
            &[],
        ));
    }
    r.flags[2] = u8::from(d.boolean(node, "must_be_friendly")?);
    r.flags[3] = u8::from(d.boolean(node, "must_be_enemy")?);
    r.flags[4] = u8::from(d.boolean(node, "must_be_owned")?);
    let [_, _, friendly, enemy, owned] = r.flags.map(|f| f != 0);
    if friendly && enemy {
        return Err(d.data_error(
            "must_be_friendly and must_be_enemy both true in GeneralPowerActorRequirements",
            &[],
        ));
    }
    if friendly && owned {
        return Err(d.data_error(
            "must_be_friendly and must_be_owned both true in GeneralPowerActorRequirements",
            &[],
        ));
    }
    if !friendly && owned && enemy {
        return Err(d.data_error(
            "must_be_owned and must_be_enemy both true in GeneralPowerActorRequirements",
            &[],
        ));
    }
    for_items(d, node, "RequiredProperty", |d, i, count| {
        let item = d.item(node, "RequiredProperty", i, count)?;
        let ids = d.string(item, "ids")?;
        let Some(property) = d.property(&ids)? else {
            return Err(d.data_error("bad property (%s) in GeneralPowerActorRequirements", &[ids]));
        };
        if r.properties.contains(&property) {
            return Err(d.data_error(
                "duplicate property (%s) in GeneralPowerActorRequirements",
                &[ids],
            ));
        }
        r.properties.insert(0, property);
        Ok(())
    })?;
    r.multiplier = f32_field(d, node, "multiplier")?;
    Ok(())
}

/// A `Category` list: each new requirement (0x18) is linked at the head,
/// then the item is fetched and loaded.
fn categories(
    d: &mut impl EgoServices,
    node: NodeId,
    list: &mut Vec<ActorRequirements>,
) -> Result<()> {
    for_items(d, node, "Category", |d, i, count| {
        list.insert(0, ActorRequirements::default());
        let item = d.item(node, "Category", i, count)?;
        actor_requirements(d, item, &mut list[0])
    })
}

/// `4AFD50`: one general power.
pub fn general_power(d: &mut impl EgoServices, node: NodeId, g: &mut GeneralPower) -> Result<()> {
    let ids = d.string(node, "ability_ids")?;
    let ability = d.find(Manager::Abilities, &ids)?.map_or(0, |(h, _)| h);
    g.set_u32(0, ability);
    if ability == 0 {
        return Err(d.data_error("Bad ability ids (%s)", &[ids]));
    }
    let loc = required(d, node, "Location")?;
    let kind = d.string(loc, "type")?;
    let t = lookup(&GOAL_ACTOR_PROXIMITY, &kind).unwrap_or(-1);
    g.set_u32(4, t as u32);
    if t <= 0 || t >= 4 {
        return Err(d.data_error("Invalid target type in General Power (%s).", &[ids]));
    }
    let placement = required(d, loc, "Placement")?;
    g.set_u32(8, f32_field(d, placement, "radius")?);
    g.set_u32(0xc, f32_field(d, placement, "num_candidates")?);
    let general = required(d, loc, "GeneralRequirements")?;
    g.set_u32(0x10, d.ability_area(ability)?);
    g.set_u8(0x14, u8::from(d.boolean(general, "no_friendly_actors")?));
    g.set_u8(0x15, u8::from(d.boolean(general, "no_enemy_actors")?));
    g.set_u8(0x16, u8::from(d.boolean(general, "no_owned_actors")?));
    g.set_u8(0x17, u8::from(d.boolean(general, "requires_supply")?));
    let ar = required(d, loc, "ActorRequirements")?;
    actor_requirements(d, ar, &mut g.location_requirements)?;
    let pr = required(d, node, "Priority")?;
    g.set_u32(0x30, f32_field(d, pr, "base")?);
    g.set_u32(0x34, f32_field(d, pr, "minimum")?);
    g.set_u32(0x38, f32_field(d, pr, "fuzzy_factor")?);
    let cv = required(d, pr, "CV")?;
    g.set_u32(0x3c, f32_field(d, cv, "minimum")?);
    g.set_u32(0x40, f32_field(d, cv, "multiplier")?);
    g.set_u32(0x44, f32_field(d, cv, "maximum")?);
    g.set_u8(0x48, u8::from(d.boolean(cv, "consider_friendly")?));
    g.set_u8(0x49, u8::from(d.boolean(cv, "consider_enemy")?));
    g.set_u8(0x4a, u8::from(d.boolean(cv, "consider_owned")?));
    categories(d, cv, &mut g.cv_categories)?;
    let av = required(d, pr, "ActorValue")?;
    g.set_u32(0x54, f32_field(d, av, "minimum")?);
    g.set_u32(0x58, f32_field(d, av, "multiplier")?);
    g.set_u32(0x5c, f32_field(d, av, "maximum")?);
    // The native reads these three from the CV section (esi), not ActorValue.
    g.set_u8(0x60, u8::from(d.boolean(cv, "consider_friendly")?));
    g.set_u8(0x61, u8::from(d.boolean(cv, "consider_enemy")?));
    g.set_u8(0x62, u8::from(d.boolean(cv, "consider_owned")?));
    categories(d, av, &mut g.value_categories)?;
    let ei = required(d, pr, "EnemyInfluence")?;
    g.set_u32(0x78, f32_field(d, ei, "minimum")?);
    g.set_u32(0x7c, f32_field(d, ei, "multiplier")?);
    g.set_u32(0x80, f32_field(d, ei, "maximum")?);
    g.set_u32(0x84, f32_field(d, ei, "exponent")?);
    let ir = required(d, pr, "InfluenceRatio")?;
    g.set_u32(0x88, f32_field(d, ir, "minimum")?);
    g.set_u32(0x8c, f32_field(d, ir, "multiplier")?);
    g.set_u32(0x90, f32_field(d, ir, "maximum")?);
    // movzx; fild; fstp dword: 0.0 or 1.0.
    let owned_only = d.boolean(ir, "count_owned_only")?;
    g.set_u32(0x94, if owned_only { ONE } else { 0 });
    g.set_u32(0x98, f32_field(d, ir, "exponent_over_one")?);
    g.set_u32(0x9c, f32_field(d, ir, "exponent_under_one")?);
    let ts = required(d, pr, "TimeSinceExplored")?;
    g.set_u32(0xa0, f32_field(d, ts, "minimum")?);
    g.set_u32(0xa4, f32_field(d, ts, "multiplier")?);
    g.set_u32(0xa8, f32_field(d, ts, "maximum")?);
    let eg = required(d, pr, "EconomyGenerated")?;
    g.set_u32(0xac, f32_field(d, eg, "minimum")?);
    g.set_u32(0xb0, f32_field(d, eg, "multiplier")?);
    // 45F180: resized to the Resource registry size, zero-filled.
    let n = d.registry_count(Manager::Resources)?;
    if n < 0 {
        return Err(Error::World(format!("negative Resource registry size {n}")));
    }
    g.resource_amounts = vec![0; n as usize];
    for_items(d, eg, "Resource", |d, i, count| {
        let item = d.item(eg, "Resource", i, count)?;
        let rid = d.string(item, "ids")?;
        let index = d.find(Manager::Resources, &rid)?.map_or(-1, |(_, i)| i);
        if index < 0 || index >= g.resource_amounts.len() as i32 {
            return Err(d.data_error("Bad resource ids (%s)", &[rid]));
        }
        let item = d.item(eg, "Resource", i, count)?;
        g.resource_amounts[index as usize] = f32_field(d, item, "amount")?;
        Ok(())
    })?;
    let fu = required(d, node, "Followup")?;
    g.set_u32(0xc0, f32_field(d, fu, "attack_flare_priority")?);
    g.set_u32(0xc4, f32_field(d, fu, "attack_flare_duration")?);
    Ok(())
}

/// `4AEDC0(dest, node, all_required)`: unthreatened / threatened / combat
/// Formation handles. Always returns true natively (failures throw).
pub fn formations(
    d: &mut impl EgoServices,
    node: NodeId,
    e: &Ego,
    dest: &mut [u32; 3],
    all_required: bool,
) -> Result<()> {
    let names = [
        d.string(node, "unthreatened_formation")?,
        d.string(node, "threatened_formation")?,
        d.string(node, "combat_formation")?,
    ];
    let missing = |all: bool| {
        if all {
            names.iter().any(|n| n.is_empty())
        } else {
            names.iter().all(|n| n.is_empty())
        }
    };
    if (all_required && missing(true)) || missing(false) {
        return Err(d.data_error(
            "You must specify at least one formation for each goal formation (ego %s)",
            &[ego_ids(e)],
        ));
    }
    let messages = [
        "Unrecognized unthreatened formation ids (%s) in ego (%s)",
        "Unrecognized threatened formation ids (%s) in ego (%s)",
        "Unrecognized combat formation ids (%s) in ego (%s)",
    ];
    for k in 0..3 {
        if names[k].is_empty() {
            continue;
        }
        dest[k] = d
            .find(Manager::Formations, &names[k])?
            .map_or(0, |(h, _)| h);
        if dest[k] == 0 {
            return Err(d.data_error(messages[k], &[names[k].clone(), ego_ids(e)]));
        }
    }
    Ok(())
}

/// `4AF0C0`: a GoalFormation into +92C by goal type.
pub fn goal_formation(d: &mut impl EgoServices, node: NodeId, e: &mut Ego) -> Result<()> {
    let goal = d.string(node, "goal_IDS")?;
    let Some(t) = lookup(&GOAL_TYPES, &goal) else {
        return Err(d.data_error(
            "Unrecognized SAI goal type (%s) in goal formation for ego (%s)",
            &[goal, ego_ids(e)],
        ));
    };
    if e.goal_formations.contains(t as u32)? {
        return Err(d.data_error(
            "Duplicate goal formation (goal = %s) in ego (%s)",
            &[goal, ego_ids(e)],
        ));
    }
    let mut set = [0u32; 3];
    formations(d, node, e, &mut set, false)?;
    e.goal_formations.insert(t as u32, set)
}
