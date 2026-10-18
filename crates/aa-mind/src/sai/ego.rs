//! Original SAI ego data (`[Ego]` definitions, 0x98C-byte object):
//! constructor `4AB9A0` with its reset `4A8E00` and the exploration-goal
//! defaults `4A5C60`, and `KKC_SAIEgoData::LoadData` `4B0560` with every
//! sub-loader it calls (see `ego_lists.rs`).
//!
//! Model: `image` holds the object's scalar bytes exactly as the native
//! object (binary32 bit patterns, integers, bytes, opaque handles). Words
//! that hold native pointers or container headers are zero in `image`; their
//! contents are the typed fields below (`refs` are the DataRefs the loader
//! queues on the data manager's pending list `[B36530]+824`, creation order).
//! Bytes the constructor leaves uninitialized are zero here as well.
//!
//! Object handles (Things, Properties, AbilityDefinition / Formation /
//! Resource definitions) are the opaque words `EgoServices` returns; the
//! native stores and hashes the definition pointers in the same places.
use super::data::{DataNode, NodeId, f32_of, milliseconds, required};
use super::ego_lists as lists;
use super::native_hash::NativeHash;
use super::profile::{DataManagers, Manager, base_load};
use crate::{Result, x87::num};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

pub const EGO_SIZE: usize = 0x98C;
/// Number of goal types (`7BDDD0`) = GoalPriority slots at +4C (0x18 each).
pub const GOAL_SLOTS: usize = 23;
const VTABLE: u32 = 0x00a1_7c3c;
const MINUS_ONE: u32 = 0xbf80_0000;
const ONE: u32 = 0x3f80_0000;

/// Thing DataManager a `4BDBA0(ids, 0x2B)` lookup goes through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThingSource {
    /// `[B39F58]`.
    Global,
    /// `[[B36530]+0C]` (the RecruitPreference loader `4ADF50`).
    Root,
}

/// Data services of the ego loaders beyond the node getters.
pub trait EgoServices: DataNode + DataManagers {
    /// `4BDBA0(ids, 0x2B)`: `427BD0` then the Thing virtual +38 (actor
    /// things only); `None` for anything else.
    fn thing(&mut self, source: ThingSource, ids: &str) -> Result<Option<u32>>;
    /// `463FA0` on the Property registry `[[B36530]+464]`.
    fn property(&mut self, ids: &str) -> Result<Option<u32>>;
    /// Registry size (+28) of the AbilityDefinition (`[[B36530]+468]`) or
    /// Resource (`[B36530]+10`) manager.
    fn registry_count(&mut self, manager: Manager) -> Result<i32>;
    /// `[ability + 38]`: the AbilityDefinition `area` (binary32 bits).
    fn ability_area(&mut self, ability: u32) -> Result<u32>;
}

/// One word of a queued DataRef past its `{vtable, ids, next}` header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefField {
    /// A pointer into this ego (offset).
    Ego(u32),
    Word(u32),
    Byte(u8),
}

/// A DataRef (`473AD0`): resolved later by its virtuals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataRef {
    pub vtable: u32,
    pub ids: String,
    /// Fields from +0C on, in offset order.
    pub fields: Vec<RefField>,
}

/// GoalPriority `ActorPreference` record (0x1C, `4AF500`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorPreference {
    /// +0 Property handle (0 when none / unknown).
    pub property: u32,
    /// +4 Thing handle (0 when none / unknown).
    pub actor: u32,
    /// +8 `value`.
    pub value: u32,
    /// +C `count_value_once_per_company`.
    pub value_once_per_company: u8,
    /// +10 `multiplier`.
    pub multiplier: u32,
    /// +14 `count_multiplier_once_per_goal`, +15 `..._once_per_company`.
    pub multiplier_once_per_goal: u8,
    pub multiplier_once_per_company: u8,
}

/// ActorPriority record (0x14, `4AF8E0`); the +1..+3 padding is not part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThingPriority {
    pub count_all_property_actors: u8,
    pub value: u32,
    pub repeat_penalty: u32,
    pub max_recruited_per_think: i32,
    pub one_time_bonus: u32,
}

/// RecruitPreference match (0xC node `{thing, priority, next}`, `4ADF50`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecruitMatch {
    pub thing: u32,
    pub priority: u32,
}

/// GeneralPowerActorRequirements (0x18, `4AFB60`): +0 max_percent_health,
/// +4..+8 must_be_engaged / must_not_be_engaged / must_be_friendly /
/// must_be_enemy / must_be_owned, +C list of Property handles, +14
/// multiplier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorRequirements {
    pub max_percent_health: u32,
    pub flags: [u8; 5],
    /// Head first.
    pub properties: Vec<u32>,
    pub multiplier: u32,
}

impl Default for ActorRequirements {
    /// Both constructors (`4AA1D0` inline, the category `new` in `4AFD50`).
    fn default() -> Self {
        ActorRequirements {
            max_percent_health: ONE,
            flags: [0; 5],
            properties: Vec::new(),
            multiplier: 0,
        }
    }
}

pub const POWER_SIZE: usize = 0xC8;

/// A general power (0xC8, constructor `4AA1D0`, loader `4AFD50`).
/// `image` +18..+2F (the inline requirements) and the list / vector headers
/// (+4C/+50, +64/+68, +B4..+BC) are zero; their contents are typed fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneralPower {
    pub image: Vec<u8>,
    /// +18 `Location.ActorRequirements`.
    pub location_requirements: ActorRequirements,
    /// +4C `CV.Category` list, head first.
    pub cv_categories: Vec<ActorRequirements>,
    /// +64 `ActorValue.Category` list, head first.
    pub value_categories: Vec<ActorRequirements>,
    /// +B4 vector: one binary32 per Resource registry entry.
    pub resource_amounts: Vec<u32>,
}

impl GeneralPower {
    /// `4AA1D0` (+0 is left for the loader).
    pub fn new() -> Self {
        let mut image = vec![0u8; POWER_SIZE];
        image[0xbc..0xc0].copy_from_slice(&1u32.to_le_bytes());
        GeneralPower {
            image,
            location_requirements: ActorRequirements::default(),
            cv_categories: Vec::new(),
            value_categories: Vec::new(),
            resource_amounts: Vec::new(),
        }
    }
    pub fn set_u32(&mut self, off: usize, v: u32) {
        self.image[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
    pub fn set_u8(&mut self, off: usize, v: u8) {
        self.image[off] = v;
    }
    pub fn u32(&self, off: usize) -> u32 {
        u32::from_le_bytes(self.image[off..off + 4].try_into().unwrap())
    }
}

impl Default for GeneralPower {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ego {
    pub ids: String,
    /// +10 localized name (`47D960`).
    pub name: String,
    pub image: Vec<u8>,
    /// +4C slot +14: GoalPriority ActorPreference list per goal type
    /// (`None` = null pointer), head first.
    pub goal_preferences: Vec<Option<Vec<ActorPreference>>>,
    /// +2A4 `ResourceNeed` records `{short, enough, plenty_value, plenty_need}`.
    pub resource_needs: Vec<[u32; 4]>,
    /// +2B0 `EconomicExcess` `{quantity, bonus}` (sorted by bonus).
    pub economic_excess: Vec<[u32; 2]>,
    /// +2C0 `ComponentValue` `{value, new_construction_only}` by component type.
    pub component_values: Vec<(u32, u8)>,
    /// +374 actor priorities and +384 property priorities.
    pub actor_priorities: NativeHash<ThingPriority>,
    pub property_priorities: NativeHash<ThingPriority>,
    /// +3FC wall strength by settlement level.
    pub wall_strength: Vec<u32>,
    /// +704 RecruitPreference lists (head first) by Thing.
    pub recruit_preferences: NativeHash<Vec<RecruitMatch>>,
    /// +73C SpecificBuildingBonus by Thing.
    pub building_bonuses: NativeHash<u32>,
    /// +830 (17 buckets; not written by the loader).
    pub table_830: NativeHash<u32>,
    /// +884 general powers by AbilityDefinition registry index.
    pub general_powers: Vec<Option<GeneralPower>>,
    /// +910 RAI attack-bombarder flare priority by goal type.
    pub flare_priorities: Vec<u32>,
    /// +92C GoalFormation `{unthreatened, threatened, combat}` by goal type.
    pub goal_formations: NativeHash<[u32; 3]>,
    /// DataRefs queued by the loader, creation order.
    pub refs: Vec<DataRef>,
}

/// `4A8E00` scalar resets (offsets).
const RESET_F32_MINUS_ONE: [u32; 228] = [
    0x18, 0x28, 0x2c, 0x30, 0x34, 0x3c, 0x40, 0x48, 0x274, 0x278, 0x27c, 0x284, 0x288, 0x28c,
    0x290, 0x294, 0x298, 0x29c, 0x2a0, 0x2bc, 0x2cc, 0x2d0, 0x2d4, 0x2d8, 0x2dc, 0x2e0, 0x2e4,
    0x2e8, 0x2ec, 0x2f0, 0x2f4, 0x2f8, 0x2fc, 0x300, 0x304, 0x308, 0x30c, 0x310, 0x314, 0x318,
    0x31c, 0x320, 0x324, 0x328, 0x32c, 0x33c, 0x340, 0x344, 0x348, 0x34c, 0x350, 0x354, 0x358,
    0x35c, 0x360, 0x364, 0x368, 0x36c, 0x370, 0x39c, 0x3a0, 0x3a4, 0x3a8, 0x3ac, 0x3b0, 0x3b4,
    0x3b8, 0x3bc, 0x3c0, 0x3c4, 0x3c8, 0x3cc, 0x3d0, 0x3d8, 0x3e0, 0x3e4, 0x3e8, 0x3f8, 0x420,
    0x424, 0x428, 0x42c, 0x430, 0x434, 0x438, 0x43c, 0x440, 0x444, 0x448, 0x44c, 0x450, 0x454,
    0x458, 0x460, 0x6b0, 0x6b4, 0x6b8, 0x6bc, 0x6c0, 0x6cc, 0x6d0, 0x6d4, 0x6d8, 0x6dc, 0x6e0,
    0x6e4, 0x6f4, 0x6f8, 0x6fc, 0x700, 0x714, 0x718, 0x71c, 0x720, 0x724, 0x728, 0x72c, 0x730,
    0x734, 0x738, 0x74c, 0x750, 0x754, 0x758, 0x75c, 0x760, 0x764, 0x768, 0x76c, 0x774, 0x778,
    0x77c, 0x780, 0x784, 0x788, 0x78c, 0x790, 0x794, 0x798, 0x7a8, 0x7ac, 0x7b0, 0x7b4, 0x7b8,
    0x7bc, 0x7c0, 0x7c4, 0x7c8, 0x7cc, 0x7d0, 0x7d4, 0x7d8, 0x7dc, 0x7e0, 0x7e4, 0x7e8, 0x7ec,
    0x7f0, 0x7f4, 0x7f8, 0x7fc, 0x800, 0x804, 0x808, 0x80c, 0x810, 0x814, 0x818, 0x81c, 0x820,
    0x824, 0x828, 0x82c, 0x840, 0x844, 0x848, 0x84c, 0x850, 0x854, 0x858, 0x85c, 0x860, 0x864,
    0x868, 0x86c, 0x870, 0x874, 0x878, 0x87c, 0x880, 0x890, 0x894, 0x89c, 0x8a4, 0x8a8, 0x8b4,
    0x8b8, 0x8bc, 0x8c0, 0x8c4, 0x8c8, 0x8cc, 0x8d8, 0x8dc, 0x8e0, 0x8e4, 0x8e8, 0x8ec, 0x8f0,
    0x8f4, 0x8f8, 0x8fc, 0x900, 0x904, 0x908, 0x90c, 0x93c, 0x940, 0x944, 0x948, 0x94c, 0x954,
    0x958, 0x95c, 0x964, 0x968, 0x96c, 0x970,
];
const RESET_I32_MINUS_ONE: [u32; 19] = [
    0x38, 0x44, 0x280, 0x334, 0x338, 0x394, 0x398, 0x3ec, 0x3f0, 0x3f4, 0x6e8, 0x6ec, 0x770, 0x898,
    0x8a0, 0x8ac, 0x8b0, 0x8d4, 0x978,
];
const RESET_BYTE_1: [u32; 5] = [0x3d4, 0x3dc, 0x950, 0x960, 0x974];
/// `4A5C60` exploration-goal defaults (relative to +464 / +528 / +5EC).
const EXPLORE_F32_MINUS_ONE: [u32; 39] = [
    0x4, 0x8, 0xc, 0x10, 0x14, 0x18, 0x1c, 0x20, 0x28, 0x2c, 0x30, 0x34, 0x44, 0x48, 0x50, 0x54,
    0x58, 0x5c, 0x60, 0x64, 0x68, 0x70, 0x74, 0x78, 0x7c, 0x80, 0x88, 0x8c, 0x90, 0x94, 0x98, 0xa0,
    0xa4, 0xa8, 0xac, 0xb0, 0xb8, 0xbc, 0xc0,
];
const EXPLORE_I32_MINUS_ONE: [u32; 4] = [0x0, 0x38, 0x40, 0x4c];
pub const EXPLORATION_GOALS: [usize; 3] = [0x464, 0x528, 0x5ec];

impl Ego {
    /// `4AB9A0` + `4A8E00` (+ `4A5C60` x3) for a definition `ids`.
    pub fn new(ids: &str) -> Self {
        let mut e = Ego {
            ids: ids.to_owned(),
            name: String::new(),
            image: vec![0; EGO_SIZE],
            goal_preferences: vec![None; GOAL_SLOTS],
            resource_needs: Vec::new(),
            economic_excess: Vec::new(),
            component_values: Vec::new(),
            actor_priorities: NativeHash::new(7),
            property_priorities: NativeHash::new(7),
            // 4A8E00: push_back(0) on the +3FC vector (766DE0).
            wall_strength: vec![0],
            recruit_preferences: NativeHash::new(0x61),
            building_bonuses: NativeHash::new(0x17),
            table_830: NativeHash::new(0x11),
            general_powers: Vec::new(),
            flare_priorities: Vec::new(),
            goal_formations: NativeHash::new(7),
            refs: Vec::new(),
        };
        e.set_u32(0, VTABLE);
        e.set_i32(0xc, -1);
        // 96D18D(+4C, 0x18, 23, 4A5D60): {bonus 0, multiplier 1, minimum 0,
        // fuzziness -1, inertia -1, list 0}.
        for t in 0..GOAL_SLOTS {
            let s = 0x4c + t * 0x18;
            e.set_u32(s + 4, ONE);
            e.set_u32(s + 0xc, MINUS_ONE);
            e.set_u32(s + 0x10, MINUS_ONE);
        }
        e.set_u32(0x408, MINUS_ONE);
        e.set_u32(0x414, MINUS_ONE);
        for base in EXPLORATION_GOALS {
            for off in EXPLORE_F32_MINUS_ONE {
                e.set_u32(base + off as usize, MINUS_ONE);
            }
            for off in EXPLORE_I32_MINUS_ONE {
                e.set_i32(base + off as usize, -1);
            }
        }
        for off in RESET_F32_MINUS_ONE {
            e.set_u32(off as usize, MINUS_ONE);
        }
        for off in RESET_I32_MINUS_ONE {
            e.set_i32(off as usize, -1);
        }
        for off in RESET_BYTE_1 {
            e.set_u8(off as usize, 1);
        }
        e
    }
    pub fn set_u32(&mut self, off: usize, v: u32) {
        self.image[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
    pub fn set_i32(&mut self, off: usize, v: i32) {
        self.set_u32(off, v as u32);
    }
    pub fn set_u8(&mut self, off: usize, v: u8) {
        self.image[off] = v;
    }
    pub fn u32(&self, off: usize) -> u32 {
        u32::from_le_bytes(self.image[off..off + 4].try_into().unwrap())
    }
    /// `4A5D80` against the static default `{0, 1, 0, -1, -1, null}`
    /// (B3B400): five `fucompp` equalities, then the list pointer.
    pub fn goal_slot_is_default(&self, t: usize) -> bool {
        let s = 0x4c + t * 0x18;
        let eq = |off: usize, bits: u32| f32::from_bits(self.u32(s + off)) == f32::from_bits(bits);
        eq(0, 0)
            && eq(4, ONE)
            && eq(8, 0)
            && eq(0xc, MINUS_ONE)
            && eq(0x10, MINUS_ONE)
            && self.goal_preferences[t].is_none()
    }
}

/// `fstp dword` of a getter value.
fn f32_field(d: &mut impl DataNode, node: NodeId, name: &str) -> Result<u32> {
    f32_of(&d.fixed(node, name)?)
}

fn cmp_f32(v: &Binary, bits: u32) -> Result<Ordering> {
    Ok(v.compare(&num(Binary::from_f32_bits(bits))?))
}

/// The native named-list loop `for (i = 0; i < count && i >= 0; i++)`.
pub(crate) fn for_items<D: DataNode>(
    d: &mut D,
    node: NodeId,
    name: &str,
    mut body: impl FnMut(&mut D, i32, i32) -> Result<()>,
) -> Result<()> {
    let count = d.count(node, name)?;
    let mut i = 0;
    while i < count && i >= 0 {
        body(d, i, count)?;
        i += 1;
    }
    Ok(())
}

/// A Recover fraction: the getter value must not be below 0 and the STORED
/// binary32 must not exceed 1 (`fld dword` reload before `fcomp 1`).
fn stored_fraction(
    d: &mut impl EgoServices,
    e: &mut Ego,
    node: NodeId,
    name: &str,
    off: usize,
    message: &str,
) -> Result<()> {
    let v = d.fixed(node, name)?;
    let stored = f32_of(&v)?;
    e.set_u32(off, stored);
    if cmp_f32(&v, 0)? == Ordering::Less
        || cmp_f32(&num(Binary::from_f32_bits(stored))?, ONE)? == Ordering::Greater
    {
        return Err(d.data_error(message, std::slice::from_ref(&e.ids)));
    }
    Ok(())
}

/// Complete `4B0560` (the diagnostic timing log excluded) on a constructed
/// ego. `p` is the x87 precision control of the loading thread.
pub fn load(d: &mut impl EgoServices, root: NodeId, e: &mut Ego, p: Precision) -> Result<()> {
    let ids = e.ids.clone();
    base_load(d, root, &ids, &mut e.name)?;
    // 47D960 ends with the DataItem +14 "loaded" byte.
    e.set_u8(0x14, 1);
    e.set_u32(0x18, f32_field(d, root, "priority")?);
    let filters = required(d, root, "Filters")?;
    lists::filters(d, filters, e)?;
    let map = required(d, root, "MapAnalysis")?;
    e.set_u32(0x28, f32_field(d, map, "cv_influence_exponent")?);
    e.set_u32(0x2c, f32_field(d, map, "cv_influence_max_travel_time")?);
    e.set_u32(0x30, f32_field(d, map, "border_influence_range")?);
    let ge = required(d, root, "GoalEngine")?;
    let gs = required(d, ge, "GeneralSettings")?;
    e.set_u32(0x34, f32_field(d, gs, "think_frequency")?);
    e.set_i32(0x38, milliseconds(&d.fixed(gs, "reaction_time")?, p)?);
    e.set_u32(0x3c, f32_field(d, gs, "fuzziness")?);
    e.set_u32(0x40, f32_field(d, gs, "goal_inertia")?);
    e.set_i32(0x44, d.int(gs, "immediate_think_on_build_max_settlements")?);
    e.set_u32(0x48, f32_field(d, gs, "immediate_think_on_build_max_time")?);
    for_items(d, ge, "GoalPriority", |d, i, count| {
        let item = d.item(ge, "GoalPriority", i, count)?;
        lists::goal_priority(d, item, e)
    })?;
    let rm = required(d, ge, "ResourceManagement")?;
    e.set_u32(0x274, f32_field(d, rm, "saving_time_limit")?);
    e.set_u32(
        0x278,
        f32_field(d, rm, "settlement_reserved_resources_amt")?,
    );
    e.set_u32(
        0x27c,
        f32_field(d, rm, "settlement_reserved_resources_time")?,
    );
    e.set_i32(
        0x280,
        d.int(rm, "settlement_reserved_resources_max_settlements")?,
    );
    lists::resource_needs(d, rm, e)?;
    // EconomicExcess: push an element (40D770 growth), then its fields.
    for_items(d, ge, "EconomicExcess", |d, i, count| {
        let item = d.item(ge, "EconomicExcess", i, count)?;
        let quantity = f32_field(d, item, "quantity")?;
        let item = d.item(ge, "EconomicExcess", i, count)?;
        let bonus = f32_field(d, item, "bonus")?;
        e.economic_excess.push([quantity, bonus]);
        Ok(())
    })?;
    let av = required(d, ge, "ActorValue")?;
    e.set_u32(0x2bc, f32_field(d, av, "scaling_mult")?);
    // 96CE20(+2B0, count, 8, 4A5C20): ascending by bonus.
    super::crt_qsort::qsort(&mut e.economic_excess, |a, b| {
        let (x, y) = (f32::from_bits(a[1]), f32::from_bits(b[1]));
        Ok(if x == y {
            0
        } else if x < y {
            -1
        } else {
            1
        })
    })?;
    for_items(d, av, "ComponentValue", |d, i, count| {
        let item = d.item(av, "ComponentValue", i, count)?;
        lists::component_value(d, item, e)
    })?;
    e.set_u32(0x2cc, f32_field(d, av, "enemy_influence_mult")?);
    e.set_u32(0x2d0, f32_field(d, av, "influence_exponent")?);
    e.set_u32(0x2d4, f32_field(d, av, "min_influence_factor")?);
    e.set_u32(0x2d8, f32_field(d, av, "military_cv_mult")?);
    e.set_u32(0x2dc, f32_field(d, av, "distance_mult_max")?);
    // distance_mult_min: ST0 against the stored max, then against 0.
    let min = d.fixed(av, "distance_mult_min")?;
    e.set_u32(0x2e0, f32_of(&min)?);
    let max = e.u32(0x2dc);
    if cmp_f32(&min, max)? == Ordering::Greater || cmp_f32(&min, 0)? == Ordering::Less {
        return Err(d.data_error("Invalid actor value ", &[]));
    }
    e.set_u32(0x2e4, f32_field(d, av, "military_value_scaling_mult")?);
    e.set_u32(
        0x2e8,
        f32_field(d, av, "military_value_influence_ratio_min")?,
    );
    e.set_u32(
        0x2ec,
        f32_field(d, av, "military_value_influence_ratio_max")?,
    );
    e.set_u32(
        0x2f0,
        f32_field(d, av, "military_value_influence_ratio_exponent")?,
    );
    e.set_u32(0x2f4, f32_field(d, av, "military_value_radius_mult")?);
    e.set_u32(0x2f8, f32_field(d, av, "military_value_marker_value")?);
    e.set_u32(
        0x2fc,
        f32_field(d, av, "military_value_enemy_traffic_value")?,
    );
    e.set_u32(0x300, f32_field(d, av, "military_value_bottleneck_value")?);
    let off = required(d, ge, "OffensiveCV")?;
    e.set_u32(0x304, f32_field(d, off, "cv_advantage_target")?);
    e.set_u32(0x308, f32_field(d, off, "cv_advantage_max")?);
    e.set_u32(0x30c, f32_field(d, off, "cv_exponent_under_target")?);
    e.set_u32(0x310, f32_field(d, off, "cv_max_mult_over_target")?);
    e.set_u32(0x314, f32_field(d, off, "min_cv_to_assign")?);
    e.set_u32(0x318, f32_field(d, off, "no_primary_supply_bonus")?);
    let def = required(d, ge, "DefensiveCV")?;
    e.set_u32(0x31c, f32_field(d, def, "cv_advantage_target")?);
    e.set_u32(0x320, f32_field(d, def, "cv_advantage_max")?);
    e.set_u32(0x324, f32_field(d, def, "cv_exponent_under_target")?);
    e.set_u32(0x328, f32_field(d, def, "cv_max_mult_over_target")?);
    e.set_u32(0x32c, f32_field(d, def, "min_cv_to_assign")?);
    e.set_u8(0x330, u8::from(d.boolean(def, "count_static_cv")?));
    let dcg = required(d, ge, "DefendConstructGoalCV")?;
    e.set_u8(
        0x331,
        u8::from(d.boolean(dcg, "disable_construct_goal_defense")?),
    );
    let un = required(d, ge, "Unpacking")?;
    e.set_i32(0x334, d.int(un, "adjacent_region_depth")?);
    e.set_i32(0x338, d.int(un, "num_random_regions")?);
    e.set_u32(0x33c, f32_field(d, un, "base_priority")?);
    e.set_u32(0x340, f32_field(d, un, "enemy_static_influence_mult")?);
    e.set_u32(0x344, f32_field(d, un, "enemy_dynamic_influence_mult")?);
    e.set_u32(0x348, f32_field(d, un, "max_friendly_building_bonus")?);
    e.set_u32(
        0x34c,
        f32_field(d, un, "max_friendly_building_bonus_per_building")?,
    );
    e.set_u32(0x350, f32_field(d, un, "max_distance")?);
    e.set_u32(0x354, f32_field(d, un, "distance_exponent")?);
    e.set_u32(0x358, f32_field(d, un, "travel_distance_mult")?);
    e.set_u32(0x35c, f32_field(d, un, "travel_distance_exponent")?);
    e.set_u32(0x360, f32_field(d, un, "fuzzy_factor")?);
    e.set_u32(0x364, f32_field(d, un, "inertia")?);
    let pk = required(d, ge, "Packing")?;
    e.set_u32(0x368, f32_field(d, pk, "pack_out_of_supply_priority")?);
    let tt = required(d, ge, "TravelTime")?;
    e.set_u32(0x36c, f32_field(d, tt, "max_travel_time")?);
    e.set_u32(0x370, f32_field(d, tt, "travel_time_exponent")?);
    for_items(d, ge, "ActorPriority", |d, i, count| {
        let item = d.item(ge, "ActorPriority", i, count)?;
        lists::actor_priority(d, item, e)
    })?;
    e.set_i32(0x394, d.int(ge, "default_max_recruited_per_think")?);
    e.set_i32(0x398, d.int(ge, "build_queue_max_items")?);
    let combat = required(d, ge, "Combat")?;
    let dp = required(d, combat, "DefendPriority")?;
    e.set_u32(0x39c, f32_field(d, dp, "economic_value_exponent")?);
    e.set_u32(0x3a0, f32_field(d, dp, "economic_value_multiplier")?);
    e.set_u32(0x3a4, f32_field(d, dp, "enemy_influence_exponent")?);
    e.set_u32(0x3a8, f32_field(d, dp, "enemy_influence_multiplier")?);
    e.set_u32(
        0x3ac,
        f32_field(d, dp, "friendly_border_influence_exponent")?,
    );
    e.set_u32(
        0x3b0,
        f32_field(d, dp, "friendly_border_influence_multiplier")?,
    );
    e.set_u32(0x3b4, f32_field(d, dp, "enemy_border_influence_exponent")?);
    e.set_u32(
        0x3b8,
        f32_field(d, dp, "enemy_border_influence_multiplier")?,
    );
    e.set_u32(0x3bc, f32_field(d, dp, "enemy_forces_threat_distance")?);
    e.set_u32(0x3c0, f32_field(d, dp, "enemy_forces_bonus")?);
    e.set_u32(0x3c4, f32_field(d, dp, "capture_risk_ratio")?);
    e.set_u32(0x3c8, f32_field(d, dp, "capture_risk_bonus")?);
    let tw = required(d, combat, "Teamwork")?;
    e.set_u32(
        0x3cc,
        f32_field(d, tw, "defend_friendly_kingdom_actor_multiplier")?,
    );
    teamwork(d, e, tw, combat)?;
    e.set_u32(0x3e4, f32_field(d, combat, "congruency_power")?);
    e.set_u32(0x3e8, f32_field(d, combat, "static_defensive_cv_mult")?);
    e.set_i32(0x3ec, d.int(combat, "max_defense_depth")?);
    e.set_i32(0x3f0, d.int(combat, "attack_commit_distance")?);
    e.set_i32(0x3f4, d.int(combat, "linked_region_distance")?);
    e.set_u32(0x3f8, f32_field(d, combat, "penalty_to_builder_cv")?);
    let ws = required(d, combat, "WallStrength")?;
    lists::wall_strength(d, ws, e)?;
    let cag = required(d, combat, "CombinedArmsGeneral")?;
    e.set_u32(0x408, f32_field(d, cag, "base_multiplier")?);
    lists::combined_arms(d, cag, e, 0x408)?;
    let caw = required(d, combat, "CombinedArmsWalls")?;
    e.set_u32(0x414, f32_field(d, caw, "base_multiplier")?);
    lists::combined_arms(d, caw, e, 0x414)?;
    let aa = required(d, combat, "AirAttacks")?;
    e.set_u32(0x420, f32_field(d, aa, "engaged_bonus")?);
    e.set_u32(0x424, f32_field(d, aa, "target_damage_floor")?);
    e.set_u32(0x428, f32_field(d, aa, "target_damage_ceiling")?);
    e.set_u32(0x42c, f32_field(d, aa, "target_damage_bonus")?);
    e.set_u32(0x430, f32_field(d, aa, "support_ground_attack_bonus")?);
    e.set_u32(0x434, f32_field(d, aa, "no_bonus_penalty")?);
    e.set_u32(0x438, f32_field(d, aa, "absolute_min_health")?);
    e.set_u32(0x43c, f32_field(d, aa, "absolute_min_health_engaged_mult")?);
    e.set_u32(
        0x440,
        f32_field(d, aa, "absolute_min_health_unengaged_mult")?,
    );
    e.set_u32(0x444, f32_field(d, aa, "extra_planes_multiplier_max")?);
    e.set_u32(
        0x448,
        f32_field(d, aa, "extra_planes_multiplier_per_plane")?,
    );
    e.set_u32(0x44c, f32_field(d, aa, "resupplying_mult")?);
    e.set_u32(0x450, f32_field(d, aa, "min_time_between_attacks")?);
    let bb = required(d, combat, "Bombardement")?;
    e.set_u32(0x454, f32_field(d, bb, "unengaged_target_multiplier")?);
    // balancing_constant must be > 0 (fst, fcomp 0, test ah,41, jp).
    let bc = d.fixed(bb, "balancing_constant")?;
    e.set_u32(0x458, f32_of(&bc)?);
    if cmp_f32(&bc, 0)? != Ordering::Greater {
        return Err(d.data_error(
            "Bombarding balancing_constant in ego (%s) must be > 0.",
            std::slice::from_ref(&e.ids),
        ));
    }
    let ex = required(d, ge, "Exploration")?;
    e.set_u8(
        0x45c,
        u8::from(d.boolean(ex, "kingdom_center_count_settlement_bldgs")?),
    );
    e.set_u32(0x460, f32_field(d, ex, "kingdom_center_min_time_owned")?);
    for (name, base) in ["CloseGoals", "DeepGoals", "AirReconGoals"]
        .into_iter()
        .zip(EXPLORATION_GOALS)
    {
        let s = required(d, ex, name)?;
        lists::exploration_goals(d, s, e, base)?;
    }
    let cn = required(d, ge, "Construction")?;
    e.set_u32(0x6b0, f32_field(d, cn, "construct_max_multiplier")?);
    e.set_u32(0x6b4, f32_field(d, cn, "construct_in_progress_bonus")?);
    e.set_u32(0x6b8, f32_field(d, cn, "construct_city_defense_max_bonus")?);
    let cde = d.fixed(cn, "construct_city_defense_exponent")?;
    e.set_u32(0x6bc, f32_of(&cde)?);
    if cmp_f32(&cde, 0)? == Ordering::Less {
        return Err(d.data_error(
            "construct_city_defense_exponent in ego (%s) must be >= 0.",
            std::slice::from_ref(&e.ids),
        ));
    }
    e.set_u32(0x6c0, f32_field(d, cn, "construct_supply_defense_bonus")?);
    e.set_u8(0x6c4, u8::from(d.boolean(cn, "construct_in_supply_only")?));
    e.set_u8(0x6c5, u8::from(d.boolean(cn, "construct_in_all_regions")?));
    e.set_u8(
        0x6c6,
        u8::from(d.boolean(cn, "construct_to_defend_research")?),
    );
    e.set_u8(
        0x6c7,
        u8::from(d.boolean(cn, "construct_to_defend_recruiters")?),
    );
    e.set_u8(
        0x6c8,
        u8::from(d.boolean(cn, "construct_to_defend_cities")?),
    );
    let rc = required(d, ge, "Recover")?;
    e.set_u32(0x6cc, f32_field(d, rc, "base_priority_hp")?);
    e.set_u32(0x6d0, f32_field(d, rc, "exponent_hp")?);
    stored_fraction(
        d,
        e,
        rc,
        "max_hp",
        0x6d4,
        "recover max_hp in ego (%s) must be between 0 and 1.",
    )?;
    e.set_u32(0x6d8, f32_field(d, rc, "base_priority_morale")?);
    e.set_u32(0x6dc, f32_field(d, rc, "exponent_morale")?);
    stored_fraction(
        d,
        e,
        rc,
        "max_morale",
        0x6e0,
        "recover max_morale in ego (%s) must be between 0 and 1.",
    )?;
    stored_fraction(
        d,
        e,
        rc,
        "critical_morale",
        0x6e4,
        "recover critical_morale in ego (%s) must be between 0 and 1.",
    )?;
    let rq = required(d, ge, "Recruiting")?;
    e.set_i32(0x6e8, d.int(rq, "max_goals_per_think")?);
    let military = d.int(rq, "num_military_goals")?;
    e.set_i32(0x6ec, military);
    if military <= 0 {
        return Err(d.data_error(
            "num_military_goals in ego (%s) must be > 0.",
            std::slice::from_ref(&e.ids),
        ));
    }
    e.set_u8(
        0x6f0,
        u8::from(d.boolean(rq, "recruit_settlers_when_all_camped")?),
    );
    e.set_u8(
        0x6f1,
        u8::from(d.boolean(rq, "recruit_settlers_when_all_settled")?),
    );
    e.set_u32(0x6f4, f32_field(d, rq, "core_cv_multiplier")?);
    e.set_u32(0x6f8, f32_field(d, rq, "core_cv_ratio_multiplier")?);
    e.set_u32(0x6fc, f32_field(d, rq, "support_cv_multiplier")?);
    e.set_u32(0x700, f32_field(d, rq, "support_cv_ratio_multiplier")?);
    for_items(d, rq, "RecruitPreference", |d, i, count| {
        let item = d.item(rq, "RecruitPreference", i, count)?;
        lists::recruit_preference(d, item, e)
    })?;
    e.set_u32(0x714, f32_field(d, rq, "economic_value_multiplier")?);
    e.set_u32(0x718, f32_field(d, rq, "decreased_income_penalty")?);
    e.set_u32(0x71c, f32_field(d, rq, "insufficient_income_level")?);
    e.set_u32(0x720, f32_field(d, rq, "insufficient_income_penalty")?);
    e.set_u32(0x724, f32_field(d, rq, "matching_builder_bonus")?);
    e.set_u32(0x728, f32_field(d, rq, "mismatching_builder_penalty")?);
    e.set_u32(0x72c, f32_field(d, rq, "empty_slot_value")?);
    e.set_u32(0x730, f32_field(d, rq, "candidate_fuzziness")?);
    e.set_u32(0x734, f32_field(d, rq, "base_value")?);
    e.set_u32(0x738, f32_field(d, rq, "building_bonus")?);
    for_items(d, rq, "SpecificBuildingBonus", |d, i, count| {
        let item = d.item(rq, "SpecificBuildingBonus", i, count)?;
        lists::building_bonus(d, item, e)
    })?;
    for (off, name) in [
        (0x74c, "location_fuzziness"),
        (0x750, "replace_company_cv_mult"),
        (0x754, "recruit_by_air_min_ratio"),
        (0x758, "recruit_by_air_max_ratio"),
        (0x75c, "recruit_by_air_ratio_under_one_multiplier"),
        (0x760, "recruit_by_air_ratio_over_one_multiplier"),
        (0x764, "recruit_by_air_ratio_under_one_exponent"),
        (0x768, "recruit_by_air_ratio_over_one_exponent"),
        (0x76c, "recruit_by_air_fuzziness"),
    ] {
        e.set_u32(off, f32_field(d, rq, name)?);
    }
    e.set_i32(0x770, d.int(rq, "recruit_by_air_target_search_depth")?);
    for (off, name) in [
        (0x774, "recruit_by_air_target_search_min_ratio"),
        (0x778, "recruit_by_air_target_search_max_ratio"),
        (0x77c, "recruit_by_air_target_search_ratio_multiplier"),
        (0x780, "recruit_by_air_target_search_fuzziness"),
        (0x784, "non_combat_landing_min_influence"),
        (0x788, "non_combat_landing_high_risk_multiplier"),
        (0x78c, "landing_location_influence_multiplier"),
        (0x790, "landing_location_influence_ceiling"),
        (0x794, "landing_location_influence_floor"),
        (
            0x798,
            "landing_location_friendly_buildings_distance_modifier",
        ),
    ] {
        e.set_u32(off, f32_field(d, rq, name)?);
    }
    for_items(d, rq, "SpecificRecruitRequest", |d, i, count| {
        let item = d.item(rq, "SpecificRecruitRequest", i, count)?;
        lists::recruit_request(d, item, e)
    })?;
    let at = required(d, ge, "Attach")?;
    e.set_u8(0x7a4, u8::from(d.boolean(at, "attach_when_engaged")?));
    e.set_u32(0x7a8, f32_field(d, at, "cv_multiplier")?);
    let ds = required(d, ge, "Destroy")?;
    e.set_u32(0x7ac, f32_field(d, ds, "base_priority")?);
    e.set_u32(0x7b0, f32_field(d, ds, "fuzzy_factor")?);
    e.set_u32(0x7b4, f32_field(d, ds, "min_stockpilable_income")?);
    e.set_u32(0x7b8, f32_field(d, ds, "min_stockpile")?);
    // min_seconds_solvency * binary32 1/60 (A1B268) at PC, then stored.
    let solvency = d.fixed(ds, "min_seconds_solvency")?;
    e.set_u32(
        0x7bc,
        f32_of(&num(
            solvency.mul(&num(Binary::from_f32_bits(0x3c88_8889))?, p)
        )?)?,
    );
    for (off, name) in [
        (0x7c0, "economic_value_mult"),
        (0x7c4, "sale_base_value"),
        (0x7c8, "sale_price_multiplier"),
        (0x7cc, "sale_cv_mult"),
        (0x7d0, "sale_resource_income_min"),
        (0x7d4, "sale_resource_income_mult"),
        (0x7d8, "sale_resource_income_max"),
        (0x7dc, "denial_sale_min_cv_advantage"),
        (0x7e0, "denial_sale_max_time_owned"),
        (0x7e4, "denial_sale_max_priority"),
        (0x7e8, "denial_sale_exponent"),
        (0x7ec, "denial_sale_cv_mult"),
        (0x7f0, "denial_raze_min_cv_advantage"),
        (0x7f4, "denial_raze_max_time_owned"),
        (0x7f8, "denial_raze_max_priority"),
        (0x7fc, "denial_raze_exponent"),
        (0x800, "denial_raze_cv_mult"),
        (0x804, "se_base_probability"),
        (0x808, "se_influence_mult"),
        (0x80c, "se_exponent"),
        (0x810, "se_influence_floor"),
        (0x814, "se_influence_ceiling"),
    ] {
        e.set_u32(off, f32_field(d, ds, name)?);
    }
    let hr = required(d, ge, "Heroes")?;
    e.set_u32(0x818, f32_field(d, hr, "base_priority")?);
    e.set_u32(0x81c, f32_field(d, hr, "organization_cv_mult")?);
    e.set_u32(0x820, f32_field(d, hr, "hero_cv_multiplier")?);
    e.set_u32(0x824, f32_field(d, hr, "wait_for_out_of_supply_mult")?);
    e.set_u32(0x828, f32_field(d, hr, "move_hero_mult")?);
    let rs = required(d, ge, "Research")?;
    e.set_u32(0x82c, f32_field(d, rs, "base_priority")?);
    // TechPriority: DataRef A1703C {ego, 0, priority}.
    for_items(d, rs, "TechPriority", |d, i, count| {
        let item = d.item(rs, "TechPriority", i, count)?;
        let tech = d.string(item, "tech")?;
        let item = d.item(rs, "TechPriority", i, count)?;
        let priority = f32_field(d, item, "priority")?;
        e.refs.push(DataRef {
            vtable: 0x00a1_703c,
            ids: tech,
            fields: vec![
                RefField::Ego(0),
                RefField::Word(0),
                RefField::Word(priority),
            ],
        });
        Ok(())
    })?;
    let gv = required(d, ge, "Give")?;
    for (off, name) in [
        (0x840, "resource_without_request_base_priority"),
        (0x844, "resource_without_request_my_stockpile_mult"),
        (0x848, "resource_without_request_their_stockpile_mult"),
        (0x84c, "resource_without_request_my_income_mult"),
        (0x850, "resource_without_request_their_income_mult"),
        (0x854, "resource_without_request_give_amount_base_mult"),
        (0x858, "resource_without_request_give_amount_fuzzy_factor"),
        (0x85c, "resource_with_request_base_priority"),
        (0x860, "resource_with_request_amount_mult"),
        (0x864, "actor_base_priority"),
        (0x868, "actor_my_stockpile_mult"),
        (0x86c, "actor_their_stockpile_mult"),
        (0x870, "actor_my_income_mult"),
        (0x874, "actor_their_income_mult"),
        (0x878, "actor_my_building_count_mult"),
        (0x87c, "actor_their_building_count_mult"),
        (0x880, "actor_building_value_mult"),
    ] {
        e.set_u32(off, f32_field(d, gv, name)?);
    }
    let gp = required(d, ge, "GeneralPowers")?;
    lists::general_powers(d, gp, e)?;
    let tc = required(d, ge, "TeamCommands")?;
    e.set_u32(0x890, f32_field(d, tc, "obey_flare_priority")?);
    let fl = required(d, ge, "InternalFlares")?;
    for (off, name) in [
        (0x8c0, "defend_attacked_target_flare_duration"),
        (0x8c4, "defend_attacked_target_flare_priority"),
        (
            0x8c8,
            "attack_region_on_organization_defeated_flare_duration",
        ),
        (
            0x8cc,
            "attack_region_on_organization_defeated_flare_priority",
        ),
    ] {
        e.set_u32(off, f32_field(d, fl, name)?);
    }
    e.set_u8(
        0x8d0,
        u8::from(d.boolean(
            fl,
            "attack_region_on_organization_defeated_flare_is_additive",
        )?),
    );
    e.set_i32(0x8d4, d.int(fl, "occluded_target_search_depth")?);
    for (off, name) in [
        (0x8d8, "occluded_target_flare_duration"),
        (0x8dc, "occluded_target_flare_priority_mult"),
        (0x8e0, "counter_attack_captured_actor_flare_duration"),
        (0x8e4, "counter_attack_captured_actor_flare_priority"),
        (0x8e8, "defend_new_structure_flare_duration"),
        (0x8ec, "defend_new_structure_flare_priority"),
        (0x8f0, "defend_new_settlement_flare_duration"),
        (0x8f4, "defend_new_settlement_flare_priority"),
        (0x8f8, "defend_captured_structure_flare_duration"),
        (0x8fc, "defend_captured_structure_flare_priority"),
        (0x900, "defend_captured_settlement_flare_duration"),
        (0x904, "defend_captured_settlement_flare_priority"),
        (0x908, "attack_bombarder_flare_duration"),
        (0x90c, "attack_bombarder_flare_default_priority"),
    ] {
        e.set_u32(off, f32_field(d, fl, name)?);
    }
    // 4A8700(23, &default): the +910 vector holds 23 copies.
    e.flare_priorities = vec![e.u32(0x90c); GOAL_SLOTS];
    for_items(d, fl, "AttackBombarderGoalPriority", |d, i, count| {
        let item = d.item(fl, "AttackBombarderGoalPriority", i, count)?;
        let goal = d.string(item, "goal")?;
        let t = super::tables::lookup(&super::tables::GOAL_TYPES, &goal).unwrap_or(-1);
        let item = d.item(fl, "AttackBombarderGoalPriority", i, count)?;
        let priority = f32_field(d, item, "priority")?;
        if t < 0 {
            return Err(d.data_error(
                "Invalid goal type (%s) for RAI attack bombarder priority in ego (%s)",
                &[goal, e.ids.clone()],
            ));
        }
        e.flare_priorities[t as usize] = priority;
        Ok(())
    })?;
    let rai = required(d, root, "ReactiveAI")?;
    e.set_u32(0x894, f32_field(d, rai, "think_frequency")?);
    e.set_i32(0x898, milliseconds(&d.fixed(rai, "reaction_time")?, p)?);
    e.set_u32(0x89c, f32_field(d, rai, "unit_retreat_max_health")?);
    e.set_i32(0x8a0, d.int(rai, "unit_retreat_depth")?);
    e.set_u32(0x8a4, f32_field(d, rai, "unit_retreat_cv_ratio")?);
    e.set_u32(0x8a8, f32_field(d, rai, "unit_retreat_probability")?);
    e.set_i32(0x8ac, d.int(rai, "unit_retreat_cycles_til_reconsider")?);
    e.set_i32(0x8b0, d.int(rai, "attack_distance")?);
    e.set_u32(0x8b4, f32_field(d, rai, "min_attack_advantage")?);
    e.set_u32(0x8b8, f32_field(d, rai, "max_attack_advantage")?);
    e.set_u32(
        0x8bc,
        f32_field(d, rai, "threatened_region_influence_ratio")?,
    );
    let stopped = d.string(rai, "stopped_formation")?;
    if !stopped.is_empty() {
        let found = d.find(Manager::Formations, &stopped)?.map_or(0, |(h, _)| h);
        e.set_u32(0x91c, found);
        if found == 0 {
            return Err(d.data_error(
                "Unrecognized stopped formation ids (%s) in ego (%s)",
                &[stopped, e.ids.clone()],
            ));
        }
    }
    let df = required(d, rai, "DefaultFormation")?;
    let mut default = [e.u32(0x920), e.u32(0x924), e.u32(0x928)];
    lists::formations(d, df, e, &mut default, true)?;
    for (k, h) in default.into_iter().enumerate() {
        e.set_u32(0x920 + 4 * k, h);
    }
    for_items(d, rai, "GoalFormation", |d, i, count| {
        let item = d.item(rai, "GoalFormation", i, count)?;
        lists::goal_formation(d, item, e)
    })?;
    let ln = required(d, root, "Learning")?;
    for (off, name) in [
        (0x93c, "learning_factor"),
        (0x944, "recruit_exponent"),
        (0x948, "recruit_floor"),
        (0x94c, "recruit_ceiling"),
        (0x940, "recruit_duration"),
    ] {
        e.set_u32(off, f32_field(d, ln, name)?);
    }
    e.set_u8(
        0x950,
        u8::from(d.boolean(ln, "recruit_scale_over_duration")?),
    );
    e.set_u32(0x958, f32_field(d, ln, "cv_exponent")?);
    e.set_u32(0x95c, f32_field(d, ln, "cv_ceiling")?);
    e.set_u32(0x954, f32_field(d, ln, "cv_duration")?);
    e.set_u8(0x960, u8::from(d.boolean(ln, "cv_scale_over_duration")?));
    e.set_u32(0x968, f32_field(d, ln, "defend_multiplier")?);
    e.set_u32(0x96c, f32_field(d, ln, "defend_exponent")?);
    e.set_u32(0x970, f32_field(d, ln, "defend_ceiling")?);
    e.set_u32(0x964, f32_field(d, ln, "defend_duration")?);
    e.set_u8(
        0x974,
        u8::from(d.boolean(ln, "defend_scale_over_duration")?),
    );
    let depth = d.int(ln, "defend_depth")?;
    e.set_i32(0x978, depth);
    if depth < 0 {
        return Err(d.data_error(
            "the defend_depth for learning in ego (%s) must be >= 0",
            std::slice::from_ref(&e.ids),
        ));
    }
    // SettlementTemplate (A17044 {ego}) and CompanyTemplate (A17040 {ego, 0})
    // references, both children of the root.
    for_items(d, root, "SettlementTemplate", |d, i, count| {
        let item = d.item(root, "SettlementTemplate", i, count)?;
        let ids = d.string(item, "IDS")?;
        e.refs.push(DataRef {
            vtable: 0x00a1_7044,
            ids,
            fields: vec![RefField::Ego(0)],
        });
        Ok(())
    })?;
    for_items(d, root, "CompanyTemplate", |d, i, count| {
        let item = d.item(root, "CompanyTemplate", i, count)?;
        let ids = d.string(item, "IDS")?;
        e.refs.push(DataRef {
            vtable: 0x00a1_7040,
            ids,
            fields: vec![RefField::Ego(0), RefField::Word(0)],
        });
        Ok(())
    })
}

/// Teamwork multipliers (+3D0..+3DC) and the Combat congruency minimum
/// (+3E0), each checked on the unrounded getter value.
fn teamwork(d: &mut impl EgoServices, e: &mut Ego, tw: NodeId, combat: NodeId) -> Result<()> {
    let ids = [e.ids.clone()];
    for (node, name, off, flag, message) in [
        (
            tw,
            "attack_allied_influence_cv_multiplier",
            0x3d0,
            Some((0x3d4, "attack_allied_influence_cv_can_exceed_max")),
            "Invalid attack_allied_influence_cv_multiplier in ego (%s), 0 <= attack_allied_influence_cv_multiplier <= 1.",
        ),
        (
            tw,
            "defend_allied_influence_cv_multiplier",
            0x3d8,
            Some((0x3dc, "defend_allied_influence_cv_can_exceed_max")),
            "Invalid defend_allied_influence_cv_multiplier in ego (%s), 0 <= defend_allied_influence_cv_multiplier <= 1.",
        ),
        (
            combat,
            "congruency_min_multiplier",
            0x3e0,
            None,
            "Invalid congruency_min_multiplier in ego (%s), 0 <= congruency_min_multiplier <= 1.",
        ),
    ] {
        let v = d.fixed(node, name)?;
        e.set_u32(off, f32_of(&v)?);
        if cmp_f32(&v, 0)? == Ordering::Less || cmp_f32(&v, ONE)? == Ordering::Greater {
            return Err(d.data_error(message, &ids));
        }
        if let Some((flag_off, flag_name)) = flag {
            e.set_u8(flag_off, u8::from(d.boolean(tw, flag_name)?));
        }
    }
    Ok(())
}
