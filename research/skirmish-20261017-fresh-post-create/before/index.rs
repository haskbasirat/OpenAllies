//! Owned native tile-membership lists and static/dynamic blocker state.
//! Transform changes retain original insertion/removal order. Actor component
//! callbacks and descriptor policy are separate from this authoritative geometry.
use crate::{
    Error, Result,
    collision::{self, Actor, Grid, Shape},
    geometry::{Placement, TileRect, marks_blocker, tile_rectangle},
};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type ActorKey = u32;
pub type NodeKey = u32;

/// Required component/presentation services for ordinary Actor transform
/// commits. The index owns geometry; callbacks must read that authoritative
/// state. The position callback is virtual+C8 (4D0EE0), including its object,
/// component, attachment and property effects; an empty callback is not a full
/// actor implementation. Errors preserve all earlier effects. Callers must not
/// retry a failed operation blindly or remove/reuse `key` during a callback.
pub trait TransformCallbacks {
    fn position_changed(&mut self, index: &mut SpatialIndex, key: ActorKey) -> Result<()>;
    fn world_phase(&self) -> u32;
    fn presentation_available(&self) -> bool;
    fn presentation_position_changed(
        &mut self,
        index: &mut SpatialIndex,
        key: ActorKey,
    ) -> Result<()>;
    fn presentation_facing_changed(
        &mut self,
        index: &mut SpatialIndex,
        key: ActorKey,
    ) -> Result<()>;
}

/// Normalization in 4EE9F0 /4453A0 after 4EEBE0 stores a two-dimensional
/// input and clears facing Z. Zero 2D input reaches native NaN and is rejected
/// by the finite arithmetic API; the 3D flag branch explicitly returns zero.
pub fn normalized_facing(
    input: [u32; 2],
    three_dimensional: bool,
    p: Precision,
) -> Result<[u32; 3]> {
    let x = Binary::from_f32_bits(input[0])?;
    let y = Binary::from_f32_bits(input[1])?;
    let squared = x.mul(&x, p)?.add(&y.mul(&y, p)?, p)?;
    let one = Binary::from_i32(1);
    let epsilon = Binary::from_f32_bits(0x38d1_b717)?;
    let scale = if three_dimensional {
        let length = squared.add(&Binary::from_i32(0), p)?.sqrt(p)?;
        if length.compare(&epsilon).is_gt() {
            one.div(&length, p)?
        } else {
            return Ok([0; 3]);
        }
    } else {
        let delta = squared.sub(&one, p)?;
        let delta = if delta.is_negative_nonzero() {
            delta.negated()
        } else {
            delta
        };
        if !delta.compare(&epsilon).is_gt() {
            return Ok([input[0], input[1], 0]);
        }
        one.div(&squared.sqrt(p)?, p)?
    };
    Ok([
        scale.mul(&x, p)?.store_f32()?,
        scale.mul(&y, p)?.store_f32()?,
        0,
    ])
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Member {
    /// Opaque runtime identity, distinct in meaning from the saved GTID.
    pub key: ActorKey,
    pub reference: u32,
    pub actor: Actor,
    pub height_bits: u32,
    pub facing_z_bits: u32,
    /// Object+44/+48. Original create/load shell initializes both to zero;
    /// position publication computes unclamped native truncate/shift cells.
    pub cached_tile: [i32; 2],
    pub blocking: Shape,
    pub soft: Shape,
    /// Already selected by ordinary virtual+14: explicit target when the
    /// definition+80 override byte is set, otherwise the soft shape.
    pub target: Shape,
    /// Actual virtual+B4 category result (ordinary Actor is true).
    pub actor_category: bool,
    /// Actual instantiated Wall component, actor+90/slot12.
    pub wall_present: bool,
}

#[cfg(test)]
#[path = "index_unused_fields_tests.rs"]
mod unused_fields_tests;

#[cfg(test)]
mod native_tests {
    use super::*;
    use serde_json::{Value, json};

    fn snapshot(index: &mut SpatialIndex) -> Value {
        let mut tiles = Vec::new();
        for y in 0..index.grid.dimensions[1] {
            for x in 0..index.grid.dimensions[0] {
                let tile = index.tile([x, y]).unwrap();
                let actors = index.actors_at([x, y]).unwrap();
                let mut node = tile.first_node;
                let mut last = None;
                while let Some(key) = node {
                    last = node;
                    node = index.node(key).unwrap().next;
                }
                assert_eq!(last, tile.last_node);
                tiles.push(json!({"actors":actors,"flags20":tile.flags20,
                    "primary_reference":tile.primary_reference}));
            }
        }
        let flags: BTreeMap<String, u32> = index
            .members()
            .map(|m| (m.key.to_string(), m.actor.flags))
            .collect();
        json!({"tiles":tiles,"member_flags":flags,"trace":index.take_events()})
    }

    #[test]
    fn original_ordered_membership_sequences() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../research/phase7-spatial/native-index.json"
        ))
        .unwrap();
        let cases = fixture["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 48);
        let mut count = 0;
        for (case_id, case) in cases.iter().enumerate() {
            let mut index = SpatialIndex::new(
                serde_json::from_value(case["grid"].clone()).unwrap(),
                serde_json::from_value(case["flags"].clone()).unwrap(),
                serde_json::from_value(case["precision"].clone()).unwrap(),
            )
            .unwrap();
            index.set_event_capture(true);
            for member in case["members"].as_array().unwrap() {
                index
                    .register(serde_json::from_value(member.clone()).unwrap())
                    .unwrap();
            }
            for (step_id, step) in case["steps"].as_array().unwrap().iter().enumerate() {
                let input = &step["input"];
                let key = input["actor"].as_u64().unwrap() as u32;
                match input["op"].as_str().unwrap() {
                    "insert" => index.insert(key).unwrap(),
                    "remove" => index.remove(key).unwrap(),
                    "retire_reference" => {
                        index.retire_reference(index.member(key).unwrap().reference)
                    }
                    "update" => {
                        let member = index.member(key).unwrap();
                        let old_b = member.placement(false);
                        let old_a = member.placement(true);
                        let position: [u32; 2] =
                            serde_json::from_value(input["position"].clone()).unwrap();
                        let facing = input
                            .get("facing")
                            .map(|v| serde_json::from_value(v.clone()).unwrap())
                            .unwrap_or(member.actor.facing);
                        let mut new_b = old_b.clone();
                        new_b.position = position;
                        new_b.facing = facing;
                        let mut new_a = old_a.clone();
                        new_a.position = position;
                        new_a.facing = facing;
                        index
                            .update_tiles(key, &old_b, &new_b, &old_a, &new_a)
                            .unwrap();
                        // Matches the oracle's explicitly prepared caller state.
                        // This fixture does not claim full position/facing callbacks.
                        let member = index.members.get_mut(&key).unwrap();
                        member.actor.position = position;
                        member.actor.facing = facing;
                    }
                    _ => panic!("unknown native fixture operation"),
                }
                assert_eq!(
                    snapshot(&mut index),
                    step["native"],
                    "case {case_id}, step {step_id}"
                );
                count += 1;
            }
        }
        assert_eq!(count, 432);
    }

    struct Callbacks {
        phase: u32,
        available: bool,
        change_phase: Option<u32>,
        trace: Vec<Value>,
    }
    impl Callbacks {
        fn record(&mut self, action: &str, index: &SpatialIndex, key: ActorKey) -> Result<()> {
            let m = index.member(key)?;
            self.trace.push(json!({"action":action,"actor":key,
                "position":[m.actor.position[0],m.actor.position[1],m.height_bits],
                "facing":[m.actor.facing[0],m.actor.facing[1],m.facing_z_bits]}));
            Ok(())
        }
    }
    impl TransformCallbacks for Callbacks {
        fn position_changed(&mut self, index: &mut SpatialIndex, key: ActorKey) -> Result<()> {
            self.record("position_changed", index, key)?;
            if let Some(phase) = self.change_phase {
                self.phase = phase;
            }
            Ok(())
        }
        fn world_phase(&self) -> u32 {
            self.phase
        }
        fn presentation_available(&self) -> bool {
            self.available
        }
        fn presentation_position_changed(
            &mut self,
            index: &mut SpatialIndex,
            key: ActorKey,
        ) -> Result<()> {
            self.record("presentation_position", index, key)
        }
        fn presentation_facing_changed(
            &mut self,
            index: &mut SpatialIndex,
            key: ActorKey,
        ) -> Result<()> {
            self.record("presentation_facing", index, key)
        }
    }
    #[test]
    fn original_transform_commits_and_required_callback_gates() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../research/phase7-spatial/native-transforms.json"
        ))
        .unwrap();
        let cases = fixture["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 96);
        let mut count = 0;
        for (case_id, case) in cases.iter().enumerate() {
            let mut index = SpatialIndex::new(
                serde_json::from_value(case["grid"].clone()).unwrap(),
                serde_json::from_value(case["flags"].clone()).unwrap(),
                serde_json::from_value(case["precision"].clone()).unwrap(),
            )
            .unwrap();
            index.set_event_capture(true);
            let mut keys = Vec::new();
            for member in case["members"].as_array().unwrap() {
                let member: Member = serde_json::from_value(member.clone()).unwrap();
                keys.push(member.key);
                index.register(member).unwrap();
            }
            if case["publish"].as_bool().unwrap() {
                for key in keys {
                    index.insert(key).unwrap();
                }
            }
            index.take_events();
            let mut callbacks = Callbacks {
                phase: case["phase"].as_u64().unwrap() as u32,
                available: case["available"].as_bool().unwrap(),
                change_phase: serde_json::from_value(case["change_phase"].clone()).unwrap(),
                trace: Vec::new(),
            };
            for (step_id, step) in case["steps"].as_array().unwrap().iter().enumerate() {
                let input = &step["input"];
                let key = input["actor"].as_u64().unwrap() as u32;
                match input["op"].as_str().unwrap() {
                    "position" => index
                        .commit_position(
                            key,
                            serde_json::from_value(input["value"].clone()).unwrap(),
                            &mut callbacks,
                        )
                        .unwrap(),
                    "facing" => index
                        .set_facing(
                            key,
                            serde_json::from_value(input["value"].clone()).unwrap(),
                            &mut callbacks,
                        )
                        .unwrap(),
                    _ => panic!("unknown transform fixture operation"),
                }
                let mut actual = snapshot(&mut index);
                let poses: BTreeMap<String, Value> = index
                    .members()
                    .map(|m| {
                        (
                            m.key.to_string(),
                            json!({
                    "position":[m.actor.position[0],m.actor.position[1],m.height_bits],
                    "facing":[m.actor.facing[0],m.actor.facing[1],m.facing_z_bits]}),
                        )
                    })
                    .collect();
                actual["poses"] = json!(poses);
                actual["callbacks"] = json!(std::mem::take(&mut callbacks.trace));
                actual["world_phase"] = json!(callbacks.phase);
                assert_eq!(actual, step["native"], "case {case_id}, step {step_id}");
                count += 1;
            }
        }
        assert_eq!(count, 288);
    }

    #[test]
    fn transform_errors_preserve_completed_effects_and_do_not_present() {
        struct Failing;
        impl TransformCallbacks for Failing {
            fn position_changed(&mut self, _: &mut SpatialIndex, _: ActorKey) -> Result<()> {
                Err(Error::World("required component service failed".into()))
            }
            fn world_phase(&self) -> u32 {
                panic!("failure stops before presentation gating")
            }
            fn presentation_available(&self) -> bool {
                panic!("failure stops before presentation gating")
            }
            fn presentation_position_changed(
                &mut self,
                _: &mut SpatialIndex,
                _: ActorKey,
            ) -> Result<()> {
                panic!("must not present failed commit")
            }
            fn presentation_facing_changed(
                &mut self,
                _: &mut SpatialIndex,
                _: ActorKey,
            ) -> Result<()> {
                panic!("must not present failed normalization")
            }
        }
        let f: Value = serde_json::from_str(include_str!(
            "../../../research/phase7-spatial/native-index.json"
        ))
        .unwrap();
        let row = &f["cases"][0];
        let mut index = SpatialIndex::new(
            serde_json::from_value(row["grid"].clone()).unwrap(),
            serde_json::from_value(row["flags"].clone()).unwrap(),
            Precision::Double53,
        )
        .unwrap();
        index.set_event_capture(true);
        index
            .register(serde_json::from_value(row["members"][0].clone()).unwrap())
            .unwrap();
        index.insert(1).unwrap();
        index.take_events();
        let position = [11_f32.to_bits(), 9_f32.to_bits(), 3_f32.to_bits()];
        assert!(index.commit_position(1, position, &mut Failing).is_err());
        let m = index.member(1).unwrap();
        assert_eq!(m.actor.position, [position[0], position[1]]);
        assert_eq!(m.height_bits, position[2]);
        assert!(
            !index.take_events().is_empty(),
            "membership already changed before required callback failed"
        );
        assert!(index.set_facing(1, [0, 0], &mut Failing).is_err());
        assert_eq!(
            index.member(1).unwrap().actor.facing,
            [0, 0],
            "native initial facing stores precede invalid 2D normalization"
        );
        assert!(
            index.take_events().is_empty(),
            "normalization failure precedes tile updates"
        );
    }
}
impl Member {
    pub fn placement(&self, blocking: bool) -> Placement {
        Placement {
            shape: if blocking {
                self.blocking.clone()
            } else {
                self.soft.clone()
            },
            facing: self.actor.facing,
            position: self.actor.position,
        }
    }
    pub fn target_placement(&self) -> Placement {
        Placement {
            shape: self.target.clone(),
            facing: self.actor.facing,
            position: self.actor.position,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tile {
    pub flags20: u32,
    pub primary_reference: u32,
    pub first_node: Option<NodeKey>,
    pub last_node: Option<NodeKey>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub actor: ActorKey,
    pub next: Option<NodeKey>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Event {
    Insert {
        cell: [i32; 2],
        actor: ActorKey,
    },
    Remove {
        cell: [i32; 2],
        actor: ActorKey,
    },
    Primary {
        cell: [i32; 2],
        before: u32,
        after: u32,
    },
    Flags {
        cell: [i32; 2],
        before: u32,
        after: u32,
    },
    MemberFlags {
        actor: ActorKey,
        before: u32,
        after: u32,
    },
}

pub struct SpatialIndex {
    grid: Grid,
    precision: Precision,
    tiles: Vec<Tile>,
    members: BTreeMap<ActorKey, Member>,
    /// Slot -> current generation and object. A stale GTID never resolves by
    /// casting its low bits to an object key.
    references: BTreeMap<u16, (u16, ActorKey)>,
    nodes: Vec<Option<Node>>,
    free_nodes: Vec<NodeKey>,
    /// Native GThing+10 intrusive query links. Shared across nested queries;
    /// a fresh source create/load shell has no active traversal mark.
    query_links: BTreeMap<ActorKey, ActorKey>,
    capture_events: bool,
    events: Vec<Event>,
}

/// Temporary field bindings for a pre-staging load operation. The persistent
/// index retains only identity/tile/node ownership between these leases; reads
/// of an unbound member remain explicit errors. No pose/flags survive drop.
pub struct LoadFields<'a> {
    index: &'a mut SpatialIndex,
}
impl LoadFields<'_> {
    pub fn index(&self) -> &SpatialIndex {
        self.index
    }
    pub fn index_mut(&mut self) -> &mut SpatialIndex {
        self.index
    }
}
impl Drop for LoadFields<'_> {
    fn drop(&mut self) {
        self.index.members.clear();
    }
}
impl SpatialIndex {
    /// Bind the generation-bearing identity without supplying fake member
    /// fields. The native identity registry already exists before Object Init.
    pub fn register_load_reference(&mut self, key: ActorKey, reference: u32) -> Result<()> {
        if key == 0 || reference as u16 == 0 {
            return Err(Error::World("null load spatial identity".into()));
        }
        if let Some(&(generation, old_key)) = self.references.get(&(reference as u16)) {
            if generation == (reference >> 16) as u16 && old_key == key {
                return Ok(());
            }
            return Err(Error::World(
                "conflicting load spatial generation/identity".into(),
            ));
        }
        if self.references.values().any(|&(_, old_key)| old_key == key) {
            return Err(Error::World(
                "load spatial key already belongs to another reference".into(),
            ));
        }
        self.references
            .insert(reference as u16, ((reference >> 16) as u16, key));
        Ok(())
    }
    fn validate_load_fields(&self, member: &Member) -> Result<()> {
        if self.resolve_reference(member.reference) != Some(member.key) {
            return Err(Error::World(
                "load fields do not match the registered generation".into(),
            ));
        }
        //5534F0 reads facing only for boxes and never reads Z. A circle's
        //saved facing may contain native NaN bits: retain those raw words.
        //The actual geometry operation validates each numeric input it uses.
        tile_rectangle(&self.grid, &member.placement(false), self.precision)?;
        Ok(())
    }
    /// A synchronous operation may read these actual borrowed-field snapshots,
    /// but must commit its changed fields back before releasing the lease.
    /// The membership kernel has no outward callbacks. All candidates validate
    /// before any field is installed; the guard clears fields on every exit.
    pub fn lease_load_fields(&mut self, fields: Vec<Member>) -> Result<LoadFields<'_>> {
        if !self.members.is_empty() {
            return Err(Error::World(
                "load field lease overlaps persistent/live fields".into(),
            ));
        }
        let mut members = BTreeMap::new();
        for member in fields {
            self.validate_load_fields(&member)?;
            if members.insert(member.key, member).is_some() {
                return Err(Error::World("duplicate load field identity".into()));
            }
        }
        self.members = members;
        Ok(LoadFields { index: self })
    }
    /// One-time staging transfer attaches the actual moved placement fields to
    /// existing membership. It never registers/reorders tile-list nodes again.
    pub fn attach_loaded_fields(&mut self, member: Member) -> Result<()> {
        self.validate_load_fields(&member)?;
        if self.members.contains_key(&member.key) {
            return Err(Error::World("loaded fields already attached".into()));
        }
        self.members.insert(member.key, member);
        Ok(())
    }
    /// Original4EE5C0 for ordinary immutable shape getters. Membership sees
    /// the old actor transform until all updates finish, then virtual+C8 sees
    /// the newly committed XYZ. Presentation gating is read after that callback.
    pub fn commit_position<C: TransformCallbacks>(
        &mut self,
        key: ActorKey,
        position: [u32; 3],
        callbacks: &mut C,
    ) -> Result<()> {
        for bits in position {
            Binary::from_f32_bits(bits)?;
        }
        let member = self.member(key)?;
        if member.actor.flags & (1 << 26) != 0 {
            let old_a = member.placement(true);
            let old_b = member.placement(false);
            let mut new_a = old_a.clone();
            let mut new_b = old_b.clone();
            new_a.position = [position[0], position[1]];
            new_b.position = new_a.position;
            self.update_tiles(key, &old_b, &new_b, &old_a, &new_a)?;
        }
        let member = self
            .members
            .get_mut(&key)
            .ok_or_else(|| Error::World("missing spatial actor".into()))?;
        member.actor.position = [position[0], position[1]];
        member.height_bits = position[2];
        callbacks.position_changed(self, key)?;
        if callbacks.world_phase() != 6 && callbacks.presentation_available() {
            callbacks.presentation_position_changed(self, key)?;
        }
        Ok(())
    }
    /// Original4EEBE0/4EE9F0. Unlike position updates, facing is committed
    /// before membership effects, so primary-reference scans see the new facing.
    pub fn set_facing<C: TransformCallbacks>(
        &mut self,
        key: ActorKey,
        facing: [u32; 2],
        callbacks: &mut C,
    ) -> Result<()> {
        let member = self.member(key)?;
        if member.actor.flags & (1 << 17) != 0 {
            return Ok(());
        }
        let old_a = member.placement(true);
        let old_b = member.placement(false);
        let three_dimensional = member.actor.flags & (1 << 12) != 0;
        let member = self
            .members
            .get_mut(&key)
            .ok_or_else(|| Error::World("missing spatial actor".into()))?;
        member.actor.facing = facing;
        member.facing_z_bits = 0;
        let normalized = normalized_facing(facing, three_dimensional, self.precision)?;
        let member = self
            .members
            .get_mut(&key)
            .ok_or_else(|| Error::World("missing spatial actor".into()))?;
        member.actor.facing = [normalized[0], normalized[1]];
        member.facing_z_bits = normalized[2];
        let new_a = member.placement(true);
        let new_b = member.placement(false);
        if member.actor.flags & (1 << 26) != 0 {
            self.update_tiles(key, &old_b, &new_b, &old_a, &new_a)?;
        }
        if callbacks.world_phase() != 6 && callbacks.presentation_available() {
            callbacks.presentation_facing_changed(self, key)?;
        }
        Ok(())
    }
    pub fn new(grid: Grid, flags: Vec<u32>, precision: Precision) -> Result<Self> {
        let count = grid.dimensions[0]
            .checked_mul(grid.dimensions[1])
            .filter(|n| *n > 0);
        if grid.dimensions.iter().any(|&n| n <= 0) || count.map(|n| n as usize) != Some(flags.len())
        {
            return Err(Error::InvalidGrid);
        }
        // Also validates the actual positive finite cell size.
        collision::probe(&grid, [0, 0], precision).map_err(|e| Error::World(e.to_string()))?;
        let tiles = flags
            .into_iter()
            .map(|flags20| Tile {
                flags20,
                primary_reference: 0,
                first_node: None,
                last_node: None,
            })
            .collect();
        Ok(Self {
            grid,
            precision,
            tiles,
            members: BTreeMap::new(),
            references: BTreeMap::new(),
            nodes: Vec::new(),
            free_nodes: Vec::new(),
            query_links: BTreeMap::new(),
            capture_events: false,
            events: Vec::new(),
        })
    }
    pub fn grid(&self) -> &Grid {
        &self.grid
    }
    pub fn precision(&self) -> Precision {
        self.precision
    }
    pub fn members(&self) -> impl Iterator<Item = &Member> {
        self.members.values()
    }
    pub fn member(&self, key: ActorKey) -> Result<&Member> {
        self.members
            .get(&key)
            .ok_or_else(|| Error::World(format!("missing spatial actor {key}")))
    }

    /// Direct native initialization stores after the same placement owner has
    /// moved here. These are not transform commands and run no move callbacks.
    /// Caller must bind the current creation allocation/generation first.
    pub fn set_initialization_scalar(&mut self,key:ActorKey,offset:u32,value:u32)->Result<()> {
        match offset {
            0x28=>self.member_mut(key)?.height_bits=value,
            0x44=>self.member_mut(key)?.cached_tile[0]=value as i32,
            0x48=>self.member_mut(key)?.cached_tile[1]=value as i32,
            _=>return Err(Error::World("initialization scalar only admits height and cached tile stores".into())),
        }
        Ok(())
    }
    ///Native4D64F0's successful state24 suffix ORs bit12 without moving the
    ///object or changing membership. No arbitrary flag mask is accepted.
    pub fn mark_falling(&mut self,key:ActorKey)->Result<()> {
        self.member_mut(key)?.actor.flags|=0x1000;
        Ok(())
    }
    /// Native World removal4F5490/4F65B0 only ORs dead/retained bits. Those
    /// bits do not change geometry or tile membership; the same published
    /// Thing+8 word remains authoritative. General flag edits require their
    /// own placement/membership operation.
    pub fn or_removal_flags(&mut self, key: ActorKey, bits: u32) -> Result<()> {
        if !matches!(bits, 1 | 2 | 0x81) {
            return Err(Error::World("removal flag write requires native1,2 or81 mask".into()));
        }
        self.member_mut(key)?.actor.flags |= bits;
        Ok(())
    }
    ///4E44E0 removes only registered/counted flags in the same Thing+8 word.
    ///These bits do not alter placement geometry or tile membership.
    pub fn clear_registration_flags(&mut self, key: ActorKey, bits: u32) -> Result<()> {
        if !matches!(bits, 0x08000000 | 0x10000000 | 0x18000000) {
            return Err(Error::World("registration clear requires native bits27/28".into()));
        }
        self.member_mut(key)?.actor.flags &= !bits;
        Ok(())
    }
    pub(crate) fn member_mut(&mut self, key: ActorKey) -> Result<&mut Member> {
        self.members
            .get_mut(&key)
            .ok_or_else(|| Error::World(format!("missing spatial actor {key}")))
    }
    /// Original4EE4C0 after an explicitly evaluated virtual+B0. Ordinary Actor
    /// returns true. Derived virtual implementations must execute first and
    /// supply their actual result; flags are read after that evaluation.
    pub fn refresh_membership(&mut self, key: ActorKey, category_b0: bool) -> Result<()> {
        let flags = self.member(key)?.actor.flags;
        let wanted = category_b0 || flags & 0x4408 != 0;
        let present = flags & (1 << 26) != 0;
        if present && !wanted {
            self.remove(key)?;
        }
        if !present && wanted {
            self.insert(key)?;
        }
        Ok(())
    }
    /// Original4EE930 for ordinary Actor: write request bit3, then refresh via
    /// its actual constant-true+B0. Unchanged requests perform no refresh.
    pub fn set_ordinary_membership_request(
        &mut self,
        key: ActorKey,
        requested: bool,
    ) -> Result<()> {
        let m = self.member_mut(key)?;
        let before = m.actor.flags;
        if (before & 8 != 0) == requested {
            return Ok(());
        }
        m.actor.flags = if requested { before | 8 } else { before & !8 };
        let after = m.actor.flags;
        self.emit(Event::MemberFlags {
            actor: key,
            before,
            after,
        })?;
        self.refresh_membership(key, true)
    }
    pub fn tile(&self, cell: [i32; 2]) -> Result<&Tile> {
        Ok(&self.tiles[self.tile_index(cell)?])
    }
    pub fn node(&self, key: NodeKey) -> Result<&Node> {
        self.nodes
            .get(key as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| Error::World("stale spatial list node".into()))
    }
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
    /// Native-write diagnostics, disabled for normal live simulation. Disabling
    /// discards buffered diagnostics; actual engine callbacks are separate.
    pub fn set_event_capture(&mut self, enabled: bool) {
        self.capture_events = enabled;
        if !enabled {
            self.events.clear();
        }
    }
    pub fn resolve_reference(&self, reference: u32) -> Option<ActorKey> {
        self.references
            .get(&(reference as u16))
            .and_then(|&(generation, key)| (generation == (reference >> 16) as u16).then_some(key))
    }
    /// Current generation-bearing identity, including between load-field
    /// leases. No mutable pose/flag lookup is needed for a native list node.
    pub fn reference_for_key(&self, key: ActorKey) -> Result<u32> {
        let mut matches = self
            .references
            .iter()
            .filter_map(|(&slot, &(generation, candidate))| {
                (candidate == key).then_some((u32::from(generation) << 16) | u32::from(slot))
            });
        let reference = matches
            .next()
            .ok_or_else(|| Error::World("spatial key has no current reference".into()))?;
        if matches.next().is_some() {
            return Err(Error::World(
                "spatial key has duplicate current references".into(),
            ));
        }
        Ok(reference)
    }
    /// Register exact geometry without publishing tile membership. Normal
    /// creation and saved-ID restoration prepare their actual shapes first.
    pub fn register(&mut self, member: Member) -> Result<()> {
        if member.key == 0 || member.reference as u16 == 0 {
            return Err(Error::World(
                "null actor identity/GTID slot cannot be registered".into(),
            ));
        }
        if self.members.contains_key(&member.key)
            || self.references.contains_key(&(member.reference as u16))
        {
            return Err(Error::World("duplicate spatial identity/GTID slot".into()));
        }
        //Use the same native-read domain as the load/attach path: circular
        //geometry does not numerically inspect facing or height words.
        tile_rectangle(&self.grid, &member.placement(false), self.precision)?;
        self.references.insert(
            member.reference as u16,
            ((member.reference >> 16) as u16, member.key),
        );
        self.members.insert(member.key, member);
        Ok(())
    }
    /// Native GTID retirement can precede object storage reclamation. Geometry
    /// remains available until its ordered membership removal has completed.
    pub fn retire_reference(&mut self, reference: u32) {
        if self.resolve_reference(reference).is_some() {
            self.references.remove(&(reference as u16));
        }
    }
    pub fn unregister(&mut self, key: ActorKey) -> Result<Member> {
        if self.query_links.contains_key(&key) {
            return Err(Error::World(
                "cannot reclaim an actor held by a spatial query".into(),
            ));
        }
        if self.nodes.iter().flatten().any(|node| node.actor == key) {
            return Err(Error::World(
                "cannot reclaim an actor still in tile lists".into(),
            ));
        }
        let reference = self.member(key)?.reference;
        self.retire_reference(reference);
        self.members
            .remove(&key)
            .ok_or_else(|| Error::World("missing spatial actor".into()))
    }
    pub fn actors_at(&self, cell: [i32; 2]) -> Result<Vec<ActorKey>> {
        let mut out = Vec::new();
        let mut node = self.tile(cell)?.first_node;
        while let Some(key) = node {
            let n = self.node(key)?;
            out.push(n.actor);
            node = n.next;
        }
        Ok(out)
    }
    pub(crate) fn query_link(&self, key: ActorKey) -> Result<Option<ActorKey>> {
        self.member(key)?;
        Ok(self.query_links.get(&key).copied())
    }
    pub(crate) fn set_query_link(&mut self, key: ActorKey, link: Option<ActorKey>) -> Result<()> {
        self.member(key)?;
        if let Some(link) = link {
            self.query_links.insert(key, link);
        } else {
            self.query_links.remove(&key);
        }
        Ok(())
    }
    fn tile_index(&self, cell: [i32; 2]) -> Result<usize> {
        if (0..2).any(|a| cell[a] < 0 || cell[a] >= self.grid.dimensions[a]) {
            return Err(Error::InvalidGrid);
        }
        Ok(cell[1] as usize * self.grid.dimensions[0] as usize + cell[0] as usize)
    }
    fn emit(&mut self, event: Event) -> Result<()> {
        if !self.capture_events {
            return Ok(());
        }
        self.events
            .try_reserve(1)
            .map_err(|_| Error::World("spatial event allocation failed".into()))?;
        self.events.push(event);
        Ok(())
    }
    fn allocate_node(&mut self, node: Node) -> Result<NodeKey> {
        if let Some(key) = self.free_nodes.pop() {
            self.nodes[key as usize] = Some(node);
            return Ok(key);
        }
        let key = u32::try_from(self.nodes.len())
            .map_err(|_| Error::World("native node identity representation exhausted".into()))?;
        self.nodes
            .try_reserve(1)
            .map_err(|_| Error::World("spatial node allocation failed".into()))?;
        self.nodes.push(Some(node));
        Ok(key)
    }
    fn set_primary(&mut self, cell: [i32; 2], reference: u32) -> Result<()> {
        let i = self.tile_index(cell)?;
        let before = self.tiles[i].primary_reference;
        self.tiles[i].primary_reference = reference;
        self.emit(Event::Primary {
            cell,
            before,
            after: reference,
        })
    }
    fn wall_overlaps(&self, key: ActorKey, cell: [i32; 2]) -> Result<bool> {
        let m = self.member(key)?;
        if !m.actor_category
            || !m.wall_present
            || !marks_blocker(
                m.actor.flags,
                m.blocking.radius_bits,
                self.grid.cell_size_bits,
                self.precision,
            )?
        {
            return Ok(false);
        }
        let probe = collision::probe(&self.grid, cell, self.precision)
            .map_err(|e| Error::World(e.to_string()))?;
        collision::overlaps(&m.blocking, &m.actor, &probe, self.precision)
            .map_err(|e| Error::World(e.to_string()))
    }
    /// Original54F6F0: push-front, preserve first tail, then select a Wall
    /// reference only if the existing generation-bearing handle is unresolved.
    fn insert_cell(&mut self, key: ActorKey, cell: [i32; 2]) -> Result<()> {
        let i = self.tile_index(cell)?;
        let node = self.allocate_node(Node {
            actor: key,
            next: self.tiles[i].first_node,
        })?;
        self.tiles[i].first_node = Some(node);
        if self.tiles[i].last_node.is_none() {
            self.tiles[i].last_node = Some(node);
        }
        self.emit(Event::Insert { cell, actor: key })?;
        if self
            .resolve_reference(self.tiles[i].primary_reference)
            .is_none()
            && self.wall_overlaps(key, cell)?
        {
            self.set_primary(cell, self.member(key)?.reference)?;
        }
        Ok(())
    }
    /// Original4F1430 removes only the first matching actor, repairing the tail.
    ///54F780 then rescans from the surviving head if that actor held the primary.
    fn remove_cell(&mut self, key: ActorKey, cell: [i32; 2]) -> Result<()> {
        let i = self.tile_index(cell)?;
        let mut previous = None;
        let mut cursor = self.tiles[i].first_node;
        while let Some(node_key) = cursor {
            let node = self.node(node_key)?.clone();
            if node.actor == key {
                if let Some(previous) = previous {
                    self.nodes[previous as usize].as_mut().unwrap().next = node.next;
                } else {
                    self.tiles[i].first_node = node.next;
                }
                if self.tiles[i].last_node == Some(node_key) {
                    self.tiles[i].last_node = previous;
                }
                self.nodes[node_key as usize] = None;
                self.free_nodes.push(node_key);
                break;
            }
            previous = cursor;
            cursor = node.next;
        }
        self.emit(Event::Remove { cell, actor: key })?;
        if self.tiles[i].primary_reference == self.member(key)?.reference {
            self.set_primary(cell, 0)?;
            let mut cursor = self.tiles[i].first_node;
            while let Some(node_key) = cursor {
                let node = self.node(node_key)?.clone();
                if self.wall_overlaps(node.actor, cell)? {
                    self.set_primary(cell, self.member(node.actor)?.reference)?;
                    break;
                }
                cursor = node.next;
            }
        }
        Ok(())
    }
    /// Exact unfiltered54F190 branch over authoritative current placements.
    pub fn find_blocker(&self, cell: [i32; 2]) -> Result<Option<ActorKey>> {
        let probe = collision::probe(&self.grid, cell, self.precision)
            .map_err(|e| Error::World(e.to_string()))?;
        let mut cursor = self.tile(cell)?.first_node;
        while let Some(key) = cursor {
            let node = self.node(key)?;
            let member = self.member(node.actor)?;
            if marks_blocker(
                member.actor.flags,
                member.blocking.radius_bits,
                self.grid.cell_size_bits,
                self.precision,
            )? && collision::overlaps(&member.blocking, &member.actor, &probe, self.precision)
                .map_err(|e| Error::World(e.to_string()))?
            {
                return Ok(Some(node.actor));
            }
            cursor = node.next;
        }
        Ok(None)
    }
    fn member_flag(&mut self, key: ActorKey, indexed: bool) -> Result<()> {
        let m = self
            .members
            .get_mut(&key)
            .ok_or_else(|| Error::World("missing spatial actor".into()))?;
        let before = m.actor.flags;
        m.actor.flags = if indexed {
            before | 1 << 26
        } else {
            before & !(1 << 26)
        };
        let after = m.actor.flags;
        self.emit(Event::MemberFlags {
            actor: key,
            before,
            after,
        })
    }
    fn blocker_flag(&mut self, cell: [i32; 2], set: bool) -> Result<()> {
        let i = self.tile_index(cell)?;
        let before = self.tiles[i].flags20;
        self.tiles[i].flags20 = if set { before | 0x400 } else { before & !0x400 };
        self.emit(Event::Flags {
            cell,
            before,
            after: self.tiles[i].flags20,
        })
    }
    fn full_membership(
        &mut self,
        key: ActorKey,
        soft: &Placement,
        blocking: &Placement,
        insert: bool,
    ) -> Result<()> {
        let rect = tile_rectangle(&self.grid, soft, self.precision)?;
        for x in rect.min[0]..=rect.max[0] {
            for y in rect.min[1]..=rect.max[1] {
                if insert {
                    self.insert_cell(key, [x, y])?;
                } else {
                    self.remove_cell(key, [x, y])?;
                }
            }
        }
        self.member_flag(key, insert)?;
        let member = self.member(key)?;
        if marks_blocker(
            member.actor.flags,
            member.blocking.radius_bits,
            self.grid.cell_size_bits,
            self.precision,
        )? {
            let actor = Actor {
                definition: member.actor.definition,
                flags: member.actor.flags,
                position: blocking.position,
                facing: blocking.facing,
            };
            for x in rect.min[0]..=rect.max[0] {
                for y in rect.min[1]..=rect.max[1] {
                    let cell = [x, y];
                    let probe = collision::probe(&self.grid, cell, self.precision)
                        .map_err(|e| Error::World(e.to_string()))?;
                    if collision::overlaps(&blocking.shape, &actor, &probe, self.precision)
                        .map_err(|e| Error::World(e.to_string()))?
                        && (insert || self.find_blocker(cell)?.is_none())
                    {
                        self.blocker_flag(cell, insert)?;
                    }
                }
            }
        }
        Ok(())
    }
    pub fn insert(&mut self, key: ActorKey) -> Result<()> {
        let member = self.member(key)?;
        let soft = member.placement(false);
        let blocking = member.placement(true);
        self.full_membership(key, &soft, &blocking, true)
    }
    pub fn remove(&mut self, key: ActorKey) -> Result<()> {
        let member = self.member(key)?;
        let soft = member.placement(false);
        let blocking = member.placement(true);
        self.full_membership(key, &soft, &blocking, false)
    }
    /// Original5539B0. Member's authoritative transform is deliberately still
    /// its native current transform throughout the membership side effects.
    pub fn update_tiles(
        &mut self,
        key: ActorKey,
        old_soft: &Placement,
        new_soft: &Placement,
        old_blocking: &Placement,
        new_blocking: &Placement,
    ) -> Result<()> {
        let m = self.member(key)?;
        if m.actor.flags & (1 << 26) == 0 {
            return Ok(());
        }
        let old = tile_rectangle(&self.grid, old_soft, self.precision)?;
        let new = tile_rectangle(&self.grid, new_soft, self.precision)?;
        if marks_blocker(
            m.actor.flags,
            m.blocking.radius_bits,
            self.grid.cell_size_bits,
            self.precision,
        )? {
            self.full_membership(key, old_soft, old_blocking, false)?;
            return self.full_membership(key, new_soft, new_blocking, true);
        }
        if old == new {
            return Ok(());
        }
        if !old.overlaps_strict(new) {
            self.full_membership(key, old_soft, old_blocking, false)?;
            return self.full_membership(key, new_soft, new_blocking, true);
        }
        let union: TileRect = old.union(new);
        for y in union.min[1]..=union.max[1] {
            for x in union.min[0]..=union.max[0] {
                let cell = [x, y];
                if old.contains(cell) {
                    if !new.contains(cell) {
                        self.remove_cell(key, cell)?;
                    }
                } else if new.contains(cell) {
                    self.insert_cell(key, cell)?;
                }
            }
        }
        Ok(())
    }
}
