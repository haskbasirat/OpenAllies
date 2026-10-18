use aa_heightfield::Heightfield;
use aa_numeric::Precision;
use aa_spatial::{
    Error, Result,
    collision::{Actor, Grid, Shape},
    index::{ActorKey, Member, SpatialIndex},
    position::{
        self, ActorPositionWorld, Components, Diagnostic, GroundRelocationWorld,
        ObjectPositionWorld, PositionHistory,
    },
};
use serde_json::{Value, json};
struct Probe<'a> {
    input: &'a Value,
    index: SpatialIndex,
    terrain: aa_spatial::TerrainGrid,
    base: Heightfield,
    overlay: Heightfield,
    events: Vec<Value>,
    org: [u32; 2],
    stop: Option<String>,
}
fn n(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
impl<'a> Probe<'a> {
    fn new(c: &'a Value, pc: Precision) -> Self {
        let mut index = SpatialIndex::new(
            Grid {
                dimensions: [8, 8],
                coordinate_shift: 1,
                cell_size_bits: 2f32.to_bits(),
            },
            vec![0x200; 64],
            pc,
        )
        .unwrap();
        let shape = Shape {
            kind: 1,
            radius_bits: 0.25f32.to_bits(),
            bounds: [0; 4],
        };
        let old: [u32; 3] = serde_json::from_value(c["old"].clone()).unwrap();
        let flags = n(&c["flags"]);
        index
            .register(Member {
                key: 1,
                reference: 0x20007,
                actor: Actor {
                    definition: 42,
                    flags: flags & !(1 << 26),
                    position: [old[0], old[1]],
                    facing: [0, (-1f32).to_bits()],
                },
                height_bits: old[2],
                facing_z_bits: 0,
                cached_tile: [2, 2],
                blocking: shape.clone(),
                soft: shape.clone(),
                target: shape,
                actor_category: true,
                wall_present: false,
            })
            .unwrap();
        if flags & (1 << 26) != 0 {
            index.insert(1).unwrap();
        }
        let field = |height| Heightfield {
            width: 8,
            height: 8,
            coordinate_shift: 1,
            cell_size_bits: 2f32.to_bits(),
            index_bias: 0,
            height_bits: vec![height; 64],
        };
        Self {
            input: c,
            index,
            terrain: aa_spatial::TerrainGrid {
                width: 8,
                height: 8,
                coordinate_shift: 1,
                cell_size: 2f32.to_bits(),
                cells: vec![
                    aa_spatial::Cell {
                        overlay_kind: 0,
                        offset_min: 0,
                        offset_max: 0
                    };
                    64
                ],
            },
            base: field(n(&c["height"])),
            overlay: field(0),
            events: vec![],
            org: [(-7f32).to_bits(), (-8f32).to_bits()],
            stop: None,
        }
    }
    fn event(&mut self, v: Value) -> Result<()> {
        let op = v["op"].as_str().unwrap().to_owned();
        self.events.push(v);
        if self.input["stop"] == op {
            self.stop = Some(op);
            Err(Error::World("native required service cutoff".into()))
        } else {
            Ok(())
        }
    }
    fn output(&self) -> Value {
        let m = self.index.member(1).unwrap();
        json!({"outcome":self.stop.as_ref().map(|s|format!("required_{s}")).unwrap_or("returned".into()),"position":[m.actor.position[0],m.actor.position[1],m.height_bits],"flags":m.actor.flags,"cache":m.cached_tile,"org_xy":self.org,"events":self.events})
    }
}
impl ObjectPositionWorld for Probe<'_> {
    fn set_membership_request(&mut self, a: ActorKey, v: bool) -> Result<()> {
        self.index.set_ordinary_membership_request(a, v)
    }
    fn object_effects(&mut self, _: ActorKey) -> Result<()> {
        self.event(json!({"op":"object_effects"}))
    }
    fn cached_tile(&self, a: ActorKey) -> Result<[i32; 2]> {
        Ok(self.index.member(a)?.cached_tile)
    }
    fn position(&self, a: ActorKey) -> Result<[u32; 2]> {
        Ok(self.index.member(a)?.actor.position)
    }
    fn coordinate_shift(&self) -> Result<u32> {
        Ok(self.index.grid().coordinate_shift)
    }
    fn store_cached_tile(&mut self, a: ActorKey, v: [i32; 2]) -> Result<()> {
        self.index.store_position_cached_tile(a, v)
    }
    fn cell_changed(&mut self, _: ActorKey, old: [i32; 2]) -> Result<()> {
        self.event(json!({"op":"cell_changed","old":old}))
    }
}
impl ActorPositionWorld for Probe<'_> {
    fn reference(&self, a: ActorKey) -> Result<u32> {
        Ok(self.index.member(a)?.reference)
    }
    fn flags(&self, a: ActorKey) -> Result<u32> {
        Ok(self.index.member(a)?.actor.flags)
    }
    fn components(&self, _: ActorKey) -> Result<Components> {
        Ok(Components {
            organization: self.input["org"].as_bool().unwrap().then_some(1),
            element: self.input["element"].as_bool().unwrap().then_some(1),
            mover: None,
            settlement: None,
            history: None,
            follower_reference: 0,
        })
    }
    fn session_mode(&self) -> Result<i32> {
        Ok(n(&self.input["session"]) as i32)
    }
    fn diagnostic(&mut self, _: ActorKey, d: Diagnostic) -> Result<()> {
        self.event(json!({"op":"diagnostic","value":d}))
    }
    fn component_owner(&self, c: u32) -> Result<ActorKey> {
        Ok(c)
    }
    fn set_organization_saved_xy(&mut self, _: u32, xy: [u32; 2]) -> Result<()> {
        self.org = xy;
        self.event(json!({"op":"organization_xy","xy":xy}))
    }
    fn element_position_changed(&mut self, _: u32) -> Result<()> {
        self.event(json!({"op":"element"}))
    }
    fn is_attached(&mut self, _: ActorKey) -> Result<bool> {
        Ok(false)
    }
    fn attachment_parent(&mut self, _: ActorKey) -> Result<ActorKey> {
        panic!("native false Structure branch")
    }
    fn settlement_attached_actor(&self, _: u32) -> Result<Option<ActorKey>> {
        panic!("native no Settlement")
    }
    fn move_ground(&mut self, _: ActorKey, _: [u32; 2]) -> Result<()> {
        panic!("native no attachment")
    }
    fn resolve_reference(&self, r: u32) -> Result<Option<ActorKey>> {
        Ok(self.index.resolve_reference(r))
    }
    fn follower_offset(&self, _: ActorKey) -> Result<[u32; 2]> {
        panic!("native null follower")
    }
    fn follower_point(&self, _: ActorKey, _: [u32; 2]) -> Result<[u32; 3]> {
        panic!("native null follower")
    }
    fn follower_target(&mut self, _: ActorKey, _: [u32; 3]) -> Result<()> {
        panic!("native null follower")
    }
    fn history_mut(&mut self, _: u32) -> Result<&mut PositionHistory> {
        panic!("native null History")
    }
    fn history_interval(&self) -> Result<i32> {
        panic!("native null History")
    }
    fn frame(&self) -> Result<i32> {
        panic!("native null History")
    }
    fn world_time_bits(&self) -> Result<u32> {
        panic!("native null History")
    }
}
impl GroundRelocationWorld for Probe<'_> {
    fn move_ground_fields(&mut self, a: ActorKey, point: [u32; 2]) -> Result<()> {
        let m = self.index.member(a)?;
        let xy = m.actor.position;
        let flags = m.actor.flags;
        self.event(json!({"op":"ground_height","position":xy,"flags":if flags&(1<<21)!=0{0x123}else{0x122}}))?;
        if flags & (1 << 26) != 0 {
            self.event(json!({"op":"update_tiles"}))?;
        }
        let surface = aa_spatial::index_world::GroundSurface {
            terrain: &self.terrain,
            heights: aa_spatial::height_layers::HeightLayers::new(&self.base, &self.overlay)?,
        };
        self.index.move_ground_fields(a, point, &surface)
    }
    fn world_phase(&self) -> Result<u32> {
        Ok(n(&self.input["phase"]))
    }
    fn presentation_available(&self) -> Result<bool> {
        Ok(self.input["view"].as_bool().unwrap())
    }
    fn presentation_position_changed(&mut self, _: ActorKey) -> Result<()> {
        self.event(json!({"op":"presentation"}))
    }
}
#[test]
fn combined_ground_position_prefix_and_children_match213_native_rows() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-stored-ground-relocation/native-relocation.json"
    ))
    .unwrap();
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 213);
    for row in rows {
        let pc = match row["pc"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => unreachable!(),
        };
        let mut p = Probe::new(&row["input"], pc);
        let point = serde_json::from_value(row["input"]["point"].clone()).unwrap();
        let result = position::relocate_on_owners(&mut p, 1, point);
        assert_eq!(
            result.is_ok(),
            row["native"]["outcome"] == "returned",
            "{row}"
        );
        assert_eq!(p.output(), row["native"], "{row}");
    }
}
