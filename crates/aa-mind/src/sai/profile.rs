//! Original SAI profile loader `KKC_SAIData::LoadData` (`4AE3E0`,
//! Source/Data/SAIData.cpp, `[SAI]` definitions in Data/SAI/*.tgi), the
//! DataItem base load `47D960` and the ego link `4AE340`.
//!
//! The profile object (constructor `4AA430`): +8 IDS, +10 name, +1C
//! difficulty (constructor 2), +20 public (1), +24/+28 command delays in ms
//! (0), +2C/+30 command maxima (-1), +34/+38 ego index list, +3C ego
//! fuzziness, +48 nation set, +58 faction set, +68 first nation IDS, +6C
//! first faction IDS. Ego references (`473AD0`, vtable A17054) are queued on
//! the data manager's pending list ([B36530]+824) and resolved later through
//! `4AE340`; the queue order and resolution pass belong to the data manager.
use super::data::{DataNode, NodeId, f32_of, milliseconds};
use super::tables::DIFFICULTY_LEVELS;
use crate::Result;
use aa_numeric::Precision;

/// Data managers addressed by the loaders (offsets in the [B36530] root).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Manager {
    /// Root+5C (`Nation` definitions).
    Nations,
    /// Root+A8 (`Faction` definitions).
    Factions,
    /// Root+56C pointer (`Ego` definitions, used by `4AE340`).
    Egos,
    /// Root+468 pointer (`AbilityDefinition`; the ego general powers).
    Abilities,
    /// Root+474 pointer (`Formation`; the ego formations).
    Formations,
    /// Root+10 (`Resource`; ego resource needs and general powers).
    Resources,
    /// Any other root offset used by the ego loaders.
    Other(u32),
}

/// Data-manager lookups (`48D500`): the found object handle and its
/// registry index (+C), `None` when the IDS is unknown.
pub trait DataManagers {
    fn find(&mut self, manager: Manager, ids: &str) -> Result<Option<(u32, i32)>>;
}

/// Profile fields written by `4AE3E0` (constructor values as defaults).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaiProfile {
    pub ids: String,
    /// +10 localized name (`47D960`).
    pub name: String,
    /// +14 DataItem "loaded" byte (0 from the constructor, 1 after `47D960`).
    pub loaded: bool,
    pub difficulty: i32,
    pub public: bool,
    pub command_max_delay_ms: i32,
    pub command_max_total_delay_ms: i32,
    pub max_ge_commands: i32,
    pub max_rai_commands: i32,
    /// +3C binary32.
    pub ego_fuzziness: u32,
    /// Ego references in creation order (initial_ego first).
    pub ego_refs: Vec<String>,
    /// +34 list: ego registry indices, head first (`4AE340` pushes front).
    pub egos: Vec<i32>,
    /// +48 / +58 sets of found definition handles, insertion order.
    pub nations: Vec<u32>,
    pub factions: Vec<u32>,
    pub first_nation: String,
    pub first_faction: String,
}

impl SaiProfile {
    /// Constructor `4AA430` state for a definition `ids`.
    pub fn new(ids: &str) -> Self {
        SaiProfile {
            ids: ids.to_owned(),
            name: String::new(),
            loaded: false,
            difficulty: 2,
            public: true,
            command_max_delay_ms: 0,
            command_max_total_delay_ms: 0,
            max_ge_commands: -1,
            max_rai_commands: -1,
            ego_fuzziness: 0,
            ego_refs: Vec::new(),
            egos: Vec::new(),
            nations: Vec::new(),
            factions: Vec::new(),
            first_nation: String::new(),
            first_faction: String::new(),
        }
    }
}

/// `47D960`: duplicate-load check on the DataItem name, then `name`
/// (localized); native then sets the DataItem +14 "loaded" byte to 1, which
/// the callers store (`SaiProfile::loaded`, ego image +14).
pub fn base_load(
    data: &mut impl DataNode,
    node: NodeId,
    ids: &str,
    name: &mut String,
) -> Result<()> {
    if !name.is_empty() {
        return Err(data.data_error("loading duplicate node with ids '%s'", &[ids.to_owned()]));
    }
    *name = data.localized(node, "name")?;
    Ok(())
}

/// `497220` on the difficulty map ([B40E64], registered by `7BDCF0`):
/// `__wcsicmp` name match.
pub fn difficulty_value(name: &str) -> Option<i32> {
    DIFFICULTY_LEVELS
        .iter()
        .position(|n| n.eq_ignore_ascii_case(name))
        .map(|i| i as i32)
}

/// One `Nation` / `Faction` list of the profile: list name, manager and the
/// two data-error formats.
struct SetSpec<'a> {
    list: &'a str,
    manager: Manager,
    unknown: &'a str,
    duplicate: &'a str,
}

fn load_set(
    data: &mut (impl DataNode + DataManagers),
    node: NodeId,
    spec: SetSpec<'_>,
    profile_ids: &str,
    set: &mut Vec<u32>,
    first: &mut String,
) -> Result<()> {
    let SetSpec {
        list,
        manager,
        unknown,
        duplicate,
    } = spec;
    let count = data.count(node, list)?;
    if count > 0 {
        let item = data.item(node, list, 0, count)?;
        *first = data.string(item, "IDS")?;
    }
    let mut i = 0;
    while i < count && i >= 0 {
        let item = data.item(node, list, i, count)?;
        let ids = data.string(item, "IDS")?;
        let Some((object, _)) = data.find(manager, &ids)? else {
            return Err(data.data_error(unknown, &[ids, profile_ids.to_owned()]));
        };
        if set.contains(&object) {
            return Err(data.data_error(duplicate, &[ids, profile_ids.to_owned()]));
        }
        set.push(object);
        i += 1;
    }
    Ok(())
}

/// Complete `4AE3E0` (diagnostic timing log excluded) on a constructed
/// profile.
pub fn load(
    data: &mut (impl DataNode + DataManagers),
    node: NodeId,
    profile: &mut SaiProfile,
    p: Precision,
) -> Result<()> {
    base_load(data, node, &profile.ids.clone(), &mut profile.name)?;
    profile.loaded = true;
    let level = data.string(node, "difficulty_level")?;
    profile.difficulty = difficulty_value(&level).unwrap_or(-1);
    if profile.difficulty == -1 {
        return Err(data.data_error("Unrecognized difficulty level: %s", &[level]));
    }
    profile.public = data.boolean(node, "public")?;
    let delay = data.fixed(node, "command_max_delay")?;
    profile.command_max_delay_ms = milliseconds(&delay, p)?;
    let total = data.fixed(node, "command_max_total_delay")?;
    profile.command_max_total_delay_ms = milliseconds(&total, p)?;
    profile.max_ge_commands = data.int(node, "max_ge_commands")?;
    profile.max_rai_commands = data.int(node, "max_rai_commands")?;
    profile.ego_refs.push(data.string(node, "initial_ego")?);
    profile.ego_fuzziness = f32_of(&data.fixed(node, "ego_fuzziness")?)?;
    let count = data.count(node, "Ego")?;
    let mut i = 0;
    while i < count && i >= 0 {
        let item = data.item(node, "Ego", i, count)?;
        profile.ego_refs.push(data.string(item, "IDS")?);
        i += 1;
    }
    let ids = profile.ids.clone();
    load_set(
        data,
        node,
        SetSpec {
            list: "Nation",
            manager: Manager::Nations,
            unknown: "Unrecognized nation (%s) in SAI (%s).",
            duplicate: "Duplicate nation (%s) in SAI (%s).",
        },
        &ids,
        &mut profile.nations,
        &mut profile.first_nation,
    )?;
    load_set(
        data,
        node,
        SetSpec {
            list: "Faction",
            manager: Manager::Factions,
            unknown: "Unrecognized faction (%s) in SAI (%s).",
            duplicate: "Duplicate faction (%s) in SAI (%s).",
        },
        &ids,
        &mut profile.factions,
        &mut profile.first_faction,
    )
}

/// `4AE340` (ego reference resolution): the ego's registry index is pushed
/// on the front of the +34 list; an unknown IDS and a duplicate are data
/// errors.
pub fn link_ego(
    data: &mut (impl DataNode + DataManagers),
    profile: &mut SaiProfile,
    ego_ids: &str,
) -> Result<()> {
    let index = data.find(Manager::Egos, ego_ids)?.map_or(-1, |(_, i)| i);
    if profile.egos.contains(&index) {
        return Err(data.data_error(
            "Duplicate ego (%s) in SAI profile (%s)",
            &[ego_ids.to_owned(), profile.ids.clone()],
        ));
    }
    if index < 0 {
        return Err(data.data_error(
            "Invalid IDS (%s) in SAI profile (%s)",
            &[ego_ids.to_owned(), profile.ids.clone()],
        ));
    }
    profile.egos.insert(0, index);
    Ok(())
}
