//! Owned SAI region/Kingdom state from79E170/7BC130 and their concrete leaves.
//! Visibility is live state. Serialized terrain words alone do not supply it.
use crate::{startup::Object, topology::Topology};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid SAI state: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Allocation(#[from] std::collections::TryReserveError),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kingdom {
    pub object: Object,
    /// Actual Kingdom+14 GTID, used by the notification goal.
    pub native_id: u32,
    /// Actual signed Kingdom+18C major slot index.
    pub major_index: i32,
    /// Actual Kingdom+190 category. Zero is a major Kingdom.
    pub category: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileKnowledge {
    pub word14: u32,
    pub word18: u16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibilitySettings {
    /// Evaluated setting behind original globalB39AB0+108.
    pub fog_enabled: bool,
    /// Actual globalA11740, not an inferred player-count constant.
    pub visible_bit_offset: u32,
}
impl TileKnowledge {
    /// 559530: native16-bit result of the x86 masked shift.
    pub fn occupied(self, kingdom: Kingdom) -> bool {
        kingdom.category == 0
            && self.word18 & (1_u32.wrapping_shl(kingdom.major_index as u32) as u16) != 0
    }
    /// 54ED00→54ECB0.
    pub fn explored(self, kingdom: Kingdom) -> bool {
        kingdom.category != 0 || self.word14 & 1_u32.wrapping_shl(kingdom.major_index as u32) != 0
    }
    /// 54F3C0→54EC70, including disabled fog and nonmajor Kingdom branches.
    pub fn visible(self, kingdom: Kingdom, settings: VisibilitySettings) -> bool {
        !settings.fog_enabled
            || kingdom.category != 0
            || self.word14
                & 1_u32.wrapping_shl(
                    (kingdom.major_index as u32).wrapping_add(settings.visible_bit_offset),
                )
                != 0
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionKingdom {
    pub region: u32,
    pub kingdom: Object,
    /// Native+8,+C,+10 wrapping counters.
    pub explored: u32,
    pub occupied: u32,
    pub visible: u32,
    pub word14: u32,
    /// Native+18/+1C linked list in head-to-tail iteration order.
    /// Repeated identities are retained:7BA0D0 does not deduplicate.
    pub seen_records: Vec<Object>,
}
impl RegionKingdom {
    pub fn register_seen(&mut self, record: Object) -> Result<()> {
        self.seen_records.try_reserve(1)?;
        self.seen_records.insert(0, record);
        Ok(())
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionState {
    /// Native sparse +10 array indexed by Kingdom+18C.
    pub kingdoms: Vec<Option<RegionKingdom>>,
}
impl RegionState {
    /// Native7B8FF0. Negative indices would read invalid native memory and are
    /// explicitly rejected instead of silently selecting an unrelated record.
    pub fn kingdom(&self, kingdom: Kingdom) -> Result<Option<&RegionKingdom>> {
        let i = usize::try_from(kingdom.major_index)
            .map_err(|_| Error::Invalid("negative Kingdom major index"))?;
        Ok(self.kingdoms.get(i).and_then(Option::as_ref))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub regions: Vec<RegionState>,
}
impl State {
    pub fn new(region_count: usize) -> Result<Self> {
        let mut regions = Vec::new();
        regions.try_reserve_exact(region_count)?;
        regions.resize_with(region_count, RegionState::default);
        Ok(Self { regions })
    }
    /// Concrete7BC130+7B9240. Publishes the record before counting tiles, as
    /// native does. Invalid bindings stop and retain all prior state changes.
    pub fn register_region_kingdom(
        &mut self,
        region: u32,
        kingdom: Kingdom,
        topology: &Topology,
        tiles: &[TileKnowledge],
        settings: VisibilitySettings,
    ) -> Result<()> {
        let index = usize::try_from(kingdom.major_index)
            .map_err(|_| Error::Invalid("negative Kingdom major index"))?;
        let state = self
            .regions
            .get_mut(region as usize)
            .ok_or(Error::Invalid("region state missing"))?;
        if state.kingdoms.get(index).is_some_and(Option::is_some) {
            return Ok(());
        }
        if index >= state.kingdoms.len() {
            state
                .kingdoms
                .try_reserve(index + 1 - state.kingdoms.len())?;
            state.kingdoms.resize_with(index + 1, || None);
        }
        state.kingdoms[index] = Some(RegionKingdom {
            region,
            kingdom: kingdom.object,
            explored: 0,
            occupied: 0,
            visible: 0,
            word14: 0,
            seen_records: Vec::new(),
        });
        let record = state.kingdoms[index].as_mut().unwrap();
        let rectangles = topology
            .region_rectangles
            .get(region as usize)
            .ok_or(Error::Invalid("region rectangle list missing"))?;
        for rectangle in rectangles {
            let r = topology
                .rectangles
                .get(*rectangle as usize)
                .ok_or(Error::Invalid("rectangle identity missing"))?;
            // 7B9240: X outer, Y inner, inclusive signed-short bounds.
            for x in i32::from(r.bounds[3])..=i32::from(r.bounds[1]) {
                for y in i32::from(r.bounds[0])..=i32::from(r.bounds[2]) {
                    if x < 0 || y < 0 || x >= topology.layout.width || y >= topology.layout.height {
                        return Err(Error::Invalid("rectangle exceeds live tile grid"));
                    }
                    let tile = *tiles
                        .get((y * topology.layout.width + x) as usize)
                        .ok_or(Error::Invalid("live tile knowledge missing"))?;
                    record.occupied = record
                        .occupied
                        .wrapping_add(u32::from(tile.occupied(kingdom)));
                    record.explored = record
                        .explored
                        .wrapping_add(u32::from(tile.explored(kingdom)));
                    record.visible = record
                        .visible
                        .wrapping_add(u32::from(tile.visible(kingdom, settings)));
                }
            }
        }
        Ok(())
    }
    /// Native79E170 region phase, descending region order. AI notifications
    /// follow this operation and must be performed on actual live controllers.
    pub fn register_kingdom_regions(
        &mut self,
        kingdom: Kingdom,
        topology: &Topology,
        tiles: &[TileKnowledge],
        settings: VisibilitySettings,
    ) -> Result<()> {
        for region in (0..topology.regions.len()).rev() {
            self.register_region_kingdom(region as u32, kingdom, topology, tiles, settings)?;
        }
        Ok(())
    }
}

/// Concrete79E300. True selects classification3; false permits ordinary unit
/// classification. These are component presences, not render-model categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorClassFields {
    pub flags8: u32,
    pub component90: bool,
    /// +7C,+80,+84,+88,+94,+98,+A0,+A8,+AC in original probe order.
    pub components: [bool; 9],
}
pub fn classification_three(actor: ActorClassFields) -> bool {
    !(actor.flags8 & (1 << 23) != 0
        && !actor.component90
        && actor.flags8 & (1 << 22) == 0
        && actor.components.into_iter().any(|v| v))
}
///4EA430 field phase after its actual actor virtual+8C result. Returns none
/// for a nonmajor Kingdom without probing the actor's table.
pub fn seen_record(
    kingdom: Option<Kingdom>,
    actor_virtual8c: bool,
    records: &[Option<Object>],
) -> Result<Option<Object>> {
    let Some(kingdom) = kingdom else {
        return Ok(None);
    };
    if kingdom.category != 0 || !actor_virtual8c {
        return Ok(None);
    }
    let i = usize::try_from(kingdom.major_index)
        .map_err(|_| Error::Invalid("negative seen-record major index"))?;
    records
        .get(i)
        .copied()
        .ok_or(Error::Invalid("actor seen-record table too short"))
}

/// Concrete GActor virtual+8C (4EA3D0), proven by vtableA273F0.
pub fn actor_has_seen_records(word58: i32) -> bool {
    word58 > 0
}

/// GActor+58/+5C, allocated by actual4EA3E0. Seen records themselves remain
/// owned live objects; entries preserve null slots and logical identities.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorSeenTable {
    pub count58: i32,
    pub entries: Vec<Option<Object>>,
}
impl ActorSeenTable {
    pub fn create_if_needed(&mut self, major_count: i32) -> Result<()> {
        if actor_has_seen_records(self.count58) {
            return Ok(());
        }
        if major_count < 0 {
            return Err(Error::Invalid("negative live major Kingdom count"));
        }
        self.count58 = major_count;
        let mut entries = Vec::new();
        entries.try_reserve_exact(major_count as usize)?;
        entries.resize(major_count as usize, None);
        self.entries = entries;
        Ok(())
    }
    pub fn get(&self, kingdom: Option<Kingdom>) -> Result<Option<Object>> {
        seen_record(kingdom, actor_has_seen_records(self.count58), &self.entries)
    }
}

/// Concrete GActor4CF120 with its false argument, the branch used by7A0C20.
/// `contained_object_result` is required only when component80+10 is nonnull;
/// it is that actual object's virtual+20 result, not a default stance guess.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorActivity {
    pub flags8: u32,
    pub contained_object_result: Option<bool>,
    pub component6c: bool,
    pub componentac: bool,
    pub componenta0: bool,
    pub component88: bool,
}
pub fn actor_active_for_sai(fields: ActorActivity) -> bool {
    if fields.flags8 & 4 != 0 {
        return false;
    }
    if let Some(result) = fields.contained_object_result {
        return result;
    }
    !fields.component6c || fields.componentac || fields.componenta0 || fields.component88
}

/// SAI goal base (`KKC_SAIGoal`, vtable 0xA9C9B0), constructed by `0x7A66E0`
/// and destroyed by `0x7A5C90`. Field names follow the native meaning used by
/// the goal engine (`0x7A5200` priority finalization, `0x7A3860`
/// affordability, `0x79E170`/`0x7B4830` list placement). Values stay raw
/// binary32 bits; pointer ownership is the owning Rust collection.
///
/// Native writers: status is written by the goal engine (`0x7A5E10`/
/// `0x7A67B0` selection) and by vtable+0x80 `0x7A5600` (nonzero status becomes
/// 1 after `0x7A3D30` releases assignments); `0x7A5200` writes the applied
/// inertia (+0x28) and priority (+0x38), and goal virtual +0x78 writes the
/// class base priority (+0x38); goal subclasses fill the cost array. The
/// fields are public so those ported routines can perform the same writes.
///
/// Serialization keeps the original fixture layout (`category8`, `scalar18`,
/// `words1c_20`, `word24`, `word28`, `scalars30_38`); the cost array data
/// pointer is not reproducible and is written as 0 with the owned values in
/// `costs` (omitted when empty, as after `0x7A66E0`).
///
/// The SAI goal engine in aa-mind (`aa_mind::sai::engine::GoalState`,
/// `aa_mind::sai::assign::Roster`) names the same words: status +0x08,
/// costs +0x1C/+0x20, average actor travel time +0x30 ([`Self::travel`]),
/// current priority +0x34 ([`Self::current_priority`]), base priority +0x38
/// ([`Self::priority`]) and the actor list header +0x0C/+0x10/+0x14
/// ([`Self::assignment_words`], [`Self::actor_count`]). aa-mind depends on
/// this crate, so conversions into those types belong to aa-mind.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "GoalBaseLayout", into = "GoalBaseLayout")]
pub struct GoalBase {
    /// +0x04 owning controller.
    pub controller: Object,
    /// +0x08 status, compared signed by the engine: 0/1 candidate, 2 selected
    /// last think. Also the list index within a controller type bucket.
    pub status: u32,
    /// +0x0C/+0x10/+0x14 actor list header `{head, tail, count}` of
    /// `{actor, next}` nodes (entries released by `0x7A3D30`/`0x7A5C90`), as
    /// raw words. `0x7A66E0` leaves all three 0; head/tail are native node
    /// pointers, so only an empty list (both 0) carries no further state.
    pub assignment_words: [u32; 3],
    /// +0x18 fuzzy amount (constructor binary32 -1.0).
    pub fuzzy: u32,
    /// +0x1C data / +0x20 count: owned per-resource cost array (binary32
    /// bits), iterated from the last index by `0x7A3860`.
    pub costs: Vec<u32>,
    /// +0x24 cost-array capacity word (constructor 1, destructor 0). Native
    /// allocation bookkeeping only; Rust's Vec owns the storage.
    pub cost_capacity_word: u32,
    /// +0x28 applied inertia (binary32 bits).
    pub inertia: u32,
    /// +0x2C byte.
    pub byte2c: bool,
    /// +0x30 average actor travel time (binary32 bits; constructor -1.0).
    pub travel: u32,
    /// +0x34 current (actor/travel adjusted) priority (binary32 bits;
    /// constructor -1.0).
    pub current_priority: u32,
    /// +0x38 base priority, written by goal virtual +0x78 and `0x7A5200`
    /// (binary32 bits; constructor -FLT_MAX).
    pub priority: u32,
}
impl GoalBase {
    /// Complete field initialization of `0x7A66E0`.
    pub fn new(controller: Object) -> Self {
        Self {
            controller,
            status: 1,
            assignment_words: [0; 3],
            fuzzy: 0xbf80_0000,
            costs: Vec::new(),
            cost_capacity_word: 1,
            inertia: 0,
            byte2c: false,
            travel: 0xbf80_0000,
            current_priority: 0xbf80_0000,
            priority: 0xff7f_ffff,
        }
    }
    /// +0x14 actor count, signed as the engine reads it.
    pub fn actor_count(&self) -> i32 {
        self.assignment_words[2] as i32
    }
    /// True when the +0x0C head and +0x10 tail pointers are both null.
    pub fn actor_list_unlinked(&self) -> bool {
        self.assignment_words[0] == 0 && self.assignment_words[1] == 0
    }
    /// Signed view of +0x08 as the engine compares it.
    pub fn status_signed(&self) -> i32 {
        self.status as i32
    }
}
/// Original-layout serialization of `GoalBase` (phase7 fixture shape).
#[derive(Serialize, Deserialize)]
struct GoalBaseLayout {
    controller: Object,
    category8: u32,
    words0c_14: [u32; 3],
    scalar18: u32,
    words1c_20: [u32; 2],
    word24: u32,
    word28: u32,
    byte2c: bool,
    scalars30_38: [u32; 3],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    costs: Vec<u32>,
}
impl TryFrom<GoalBaseLayout> for GoalBase {
    type Error = String;
    fn try_from(w: GoalBaseLayout) -> std::result::Result<Self, String> {
        if w.words1c_20[1] as usize != w.costs.len() || (w.costs.is_empty() && w.words1c_20[0] != 0) {
            return Err("goal cost array count/pointer words do not match owned costs".into());
        }
        Ok(Self {
            controller: w.controller,
            status: w.category8,
            assignment_words: w.words0c_14,
            fuzzy: w.scalar18,
            costs: w.costs,
            cost_capacity_word: w.word24,
            inertia: w.word28,
            byte2c: w.byte2c,
            travel: w.scalars30_38[0],
            current_priority: w.scalars30_38[1],
            priority: w.scalars30_38[2],
        })
    }
}
impl From<GoalBase> for GoalBaseLayout {
    fn from(g: GoalBase) -> Self {
        Self {
            controller: g.controller,
            category8: g.status,
            words0c_14: g.assignment_words,
            scalar18: g.fuzzy,
            words1c_20: [0, g.costs.len() as u32],
            word24: g.cost_capacity_word,
            word28: g.inertia,
            byte2c: g.byte2c,
            scalars30_38: [g.travel, g.current_priority, g.priority],
            costs: g.costs,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomGoal {
    pub base: GoalBase,
    ///7BE920 concrete vtableA9E560 and targetGTID+3C.
    pub target_id: u32,
    pub words40_44: [u32; 2],
}
/// Controller7A5750 and auxiliary7CCDC0 constructor-owned state. This does not
/// itself mark its enclosing AIPlayer+4 initialized:7B4830 has further work.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Controller {
    pub object: Object,
    pub player: Object,
    pub initialized: bool,
    /// Native23 type buckets, each holding3 category lists; no game entity cap.
    pub groups: Vec<[Vec<Object>; 3]>,
    ///7A4580 head insertion. These goals have not yet run virtual initialization.
    pub pending_kingdom_goals: Vec<KingdomGoal>,
    pub words14_1c: [u32; 3],
    pub byte20: bool,
    pub hash_buckets: Vec<Vec<Object>>,
    pub hash_entry_count: u32,
    pub hash_flag34: bool,
    pub auxiliary: Auxiliary,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Auxiliary {
    pub controller: Object,
    pub pending: Vec<Object>,
    pub hash_buckets: Vec<Vec<Object>>,
    pub hash_entry_count: u32,
    pub hash_flag18: bool,
}
impl Controller {
    pub fn new(object: Object, player: Object) -> Result<Self> {
        let mut groups = Vec::new();
        groups.try_reserve_exact(23)?;
        groups.resize_with(23, || std::array::from_fn(|_| Vec::new()));
        let mut hash_buckets = Vec::new();
        hash_buckets.try_reserve_exact(17)?;
        hash_buckets.resize_with(17, Vec::new);
        let mut auxiliary_buckets = Vec::new();
        auxiliary_buckets.try_reserve_exact(17)?;
        auxiliary_buckets.resize_with(17, Vec::new);
        Ok(Self {
            object,
            player,
            initialized: true,
            groups,
            pending_kingdom_goals: Vec::new(),
            words14_1c: [0, 0, 1],
            byte20: false,
            hash_buckets,
            hash_entry_count: 0,
            hash_flag34: true,
            auxiliary: Auxiliary {
                controller: object,
                pending: Vec::new(),
                hash_buckets: auxiliary_buckets,
                hash_entry_count: 0,
                hash_flag18: true,
            },
        })
    }
    /// Actual7A48F0→7BE920→7A4580. A repeated foreign major Kingdom appends
    /// another goal; null/nonmajor/self returns without allocation or mutation.
    pub fn notify_kingdom(
        &mut self,
        own_kingdom: Option<Object>,
        kingdom: Option<Kingdom>,
    ) -> Result<()> {
        let Some(kingdom) = kingdom else {
            return Ok(());
        };
        if kingdom.category != 0 || own_kingdom == Some(kingdom.object) {
            return Ok(());
        }
        self.pending_kingdom_goals.try_reserve(1)?;
        self.pending_kingdom_goals.insert(
            0,
            KingdomGoal {
                base: GoalBase::new(self.object),
                target_id: kingdom.native_id,
                words40_44: [0, 0],
            },
        );
        Ok(())
    }
}
/// The exact two fields accessed by79E170; all other AIPlayer state remains
/// owned by its live runtime. None is a real empty controller slot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiSlot {
    pub kingdom: Option<Object>,
    pub controller: Option<Controller>,
}
impl State {
    /// Complete concrete79E170, including initialized AI-controller notification.
    pub fn register_kingdom(
        &mut self,
        kingdom: Kingdom,
        topology: &Topology,
        tiles: &[TileKnowledge],
        settings: VisibilitySettings,
        slots: &mut [Option<AiSlot>],
    ) -> Result<()> {
        self.register_kingdom_regions(kingdom, topology, tiles, settings)?;
        for slot in slots.iter_mut().flatten() {
            if let Some(controller) = &mut slot.controller {
                controller.notify_kingdom(slot.kingdom, Some(kingdom))?;
            }
        }
        Ok(())
    }
}
