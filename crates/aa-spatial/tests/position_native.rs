use aa_numeric::{Binary, Precision};
use aa_spatial::{
    Result,
    geometry::Placement,
    index::{ActorKey, Member, SpatialIndex},
    position::{
        self, ComponentKey, Components, Diagnostic, HeightEdit, PositionHistory, TerrainPaint,
        World,
    },
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
struct Fixture {
    input: Value,
    components: Components,
    history: PositionHistory,
    organization_xy: [u32; 2],
    frame: i32,
    calls: Vec<Value>,
    fail_cell: bool,
}
impl Fixture {
    fn new(input: &Value) -> Self {
        Self {
            input: input.clone(),
            components: serde_json::from_value(input["components"].clone()).unwrap(),
            history: serde_json::from_value(input["history"].clone()).unwrap(),
            organization_xy: serde_json::from_value(input["organization_xy"].clone()).unwrap(),
            frame: 0,
            calls: Vec::new(),
            fail_cell: false,
        }
    }
}
impl World for Fixture {
    fn components(&self, actor: ActorKey) -> Result<Components> {
        assert_eq!(actor, 1);
        Ok(self.components.clone())
    }
    fn session_mode(&self) -> Result<i32> {
        Ok(self.input["session_mode"].as_i64().unwrap() as i32)
    }
    fn world_word_48(&self) -> Result<u32> {
        Ok(self.input["world_word_48"].as_u64().unwrap() as u32)
    }
    fn height_margin_bits(&self) -> Result<u32> {
        Ok(self.input["margin_bits"].as_u64().unwrap() as u32)
    }
    fn ground_height(
        &mut self,
        _: &mut SpatialIndex,
        position: [u32; 2],
        flags: u32,
    ) -> Result<Binary> {
        self.calls
            .push(json!({"op":"ground_height","position":position,"flags":flags}));
        Ok(Binary::from_f32_bits(
            self.input["ground_height_bits"].as_u64().unwrap() as u32,
        )?)
    }
    fn edit_height(&mut self, _: &mut SpatialIndex, _: ActorKey, edit: HeightEdit) -> Result<()> {
        self.calls.push(json!({"op":"edit_height","edit":edit}));
        Ok(())
    }
    fn clear_doodads(
        &mut self,
        _: &mut SpatialIndex,
        _: ActorKey,
        placement: Placement,
    ) -> Result<()> {
        self.calls
            .push(json!({"op":"clear_doodads","placement":placement}));
        Ok(())
    }
    fn ground_variation(&mut self, _: &mut SpatialIndex, _: ActorKey) -> Result<Option<u32>> {
        self.calls.push(json!({"op":"ground_variation"}));
        Ok(serde_json::from_value(self.input["variation"].clone()).unwrap())
    }
    fn footprint_padding_bits(&self, _: ActorKey) -> Result<u32> {
        Ok(self.input["padding_bits"].as_u64().unwrap() as u32)
    }
    fn paint_terrain(
        &mut self,
        _: &mut SpatialIndex,
        _: ActorKey,
        paint: TerrainPaint,
    ) -> Result<()> {
        self.calls.push(json!({"op":"paint_terrain","paint":paint}));
        Ok(())
    }
    fn cell_changed(&mut self, _: &mut SpatialIndex, _: ActorKey, old: [i32; 2]) -> Result<()> {
        self.calls.push(json!({"op":"cell_changed","old":old}));
        if self.fail_cell {
            Err(aa_spatial::Error::World(
                "required cell-change service unavailable".into(),
            ))
        } else {
            Ok(())
        }
    }
    fn diagnostic(&mut self, _: &mut SpatialIndex, _: ActorKey, value: Diagnostic) -> Result<()> {
        self.calls.push(json!({"op":"diagnostic","value":value}));
        Ok(())
    }
    fn component_owner(&self, component: ComponentKey) -> Result<ActorKey> {
        assert!([10, 12].contains(&component));
        Ok(1)
    }
    fn set_organization_saved_xy(&mut self, component: ComponentKey, xy: [u32; 2]) -> Result<()> {
        self.organization_xy = xy;
        self.calls
            .push(json!({"op":"organization_xy","component":component,"xy":xy}));
        Ok(())
    }
    fn element_position_changed(
        &mut self,
        _: &mut SpatialIndex,
        component: ComponentKey,
    ) -> Result<()> {
        self.calls
            .push(json!({"op":"element","component":component}));
        Ok(())
    }
    fn is_attached(&mut self, _: &mut SpatialIndex, _: ActorKey) -> Result<bool> {
        self.calls.push(json!({"op":"is_attached"}));
        Ok(self.input["attached"].as_bool().unwrap())
    }
    fn attachment_parent(&mut self, _: &mut SpatialIndex, _: ActorKey) -> Result<ActorKey> {
        self.calls.push(json!({"op":"parent"}));
        Ok(2)
    }
    fn settlement_attached_actor(&self, component: ComponentKey) -> Result<Option<ActorKey>> {
        assert_eq!(component, 13);
        Ok(serde_json::from_value(self.input["settlement_actor"].clone()).unwrap())
    }
    fn move_ground(
        &mut self,
        _: &mut SpatialIndex,
        actor: ActorKey,
        position: [u32; 2],
    ) -> Result<()> {
        self.calls
            .push(json!({"op":"move_ground","actor":actor,"position":position}));
        Ok(())
    }
    fn follower_offset(&self, _: ActorKey) -> Result<[u32; 2]> {
        Ok(serde_json::from_value(self.input["follower_offset"].clone()).unwrap())
    }
    fn follower_target(
        &mut self,
        _: &mut SpatialIndex,
        actor: ActorKey,
        position: [u32; 3],
    ) -> Result<()> {
        self.calls
            .push(json!({"op":"follower_target","actor":actor,"position":position}));
        Ok(())
    }
    fn history_mut(&mut self, component: ComponentKey) -> Result<&mut PositionHistory> {
        assert_eq!(component, 14);
        Ok(&mut self.history)
    }
    fn history_interval(&self) -> Result<i32> {
        Ok(self.input["interval"].as_i64().unwrap() as i32)
    }
    fn frame(&self) -> Result<i32> {
        Ok(self.frame)
    }
    fn world_time_bits(&self) -> Result<u32> {
        Ok(self.input["time_bits"].as_u64().unwrap() as u32)
    }
}
fn snapshot(index: &mut SpatialIndex, world: &mut Fixture) -> Value {
    let mut tiles = Vec::new();
    for y in 0..index.grid().dimensions[1] {
        for x in 0..index.grid().dimensions[0] {
            let t = index.tile([x, y]).unwrap();
            tiles.push(json!({"actors":index.actors_at([x,y]).unwrap(),"flags20":t.flags20,"primary_reference":t.primary_reference}));
        }
    }
    let flags: BTreeMap<String, u32> = index
        .members()
        .map(|m| (m.key.to_string(), m.actor.flags))
        .collect();
    json!({"tiles":tiles,"member_flags":flags,"trace":index.take_events(),"calls":std::mem::take(&mut world.calls),
        "cached_tile":index.member(1).unwrap().cached_tile,"organization_xy":world.organization_xy,"history":world.history})
}
#[test]
fn original_actor_position_orchestration_and_concrete_leaves() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-spatial/native-position.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 192);
    let mut count = 0;
    for (case_id, case) in cases.iter().enumerate() {
        let input = &case["input"];
        let precision: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let mut index = SpatialIndex::new(
            serde_json::from_value(input["grid"].clone()).unwrap(),
            serde_json::from_value(input["flags"].clone()).unwrap(),
            precision,
        )
        .unwrap();
        index.set_event_capture(true);
        for value in input["members"].as_array().unwrap() {
            let member: Member = serde_json::from_value(value.clone()).unwrap();
            index.register(member).unwrap();
        }
        if input["publish"].as_bool().unwrap() {
            for key in 1..=3 {
                index.insert(key).unwrap();
            }
        }
        index.take_events();
        let mut world = Fixture::new(input);
        for (step_id, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            world.frame = step["frame"].as_i64().unwrap() as i32;
            position::actor_position_changed(&mut index, 1, &mut world).unwrap();
            assert_eq!(
                snapshot(&mut index, &mut world),
                step["native"],
                "case{case_id} step{step_id}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 576);
}

#[test]
fn required_position_services_fail_after_completed_native_effects() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-spatial/native-position.json"
    ))
    .unwrap();
    let input = &fixture["cases"][1]["input"];
    let mut index = SpatialIndex::new(
        serde_json::from_value(input["grid"].clone()).unwrap(),
        serde_json::from_value(input["flags"].clone()).unwrap(),
        Precision::Single24,
    )
    .unwrap();
    for value in input["members"].as_array().unwrap() {
        index
            .register(serde_json::from_value(value.clone()).unwrap())
            .unwrap();
    }
    let mut world = Fixture::new(input);
    world.fail_cell = true;
    assert!(position::actor_position_changed(&mut index, 1, &mut world).is_err());
    let member = index.member(1).unwrap();
    assert_eq!(member.actor.flags & 8, 0);
    assert_ne!(
        member.actor.flags & (1 << 26),
        0,
        "membership refresh happened first"
    );
    assert_eq!(
        member.cached_tile,
        [2, 2],
        "cached coordinates were committed before124 dispatch"
    );
    assert!(world.calls.iter().any(|v| v["op"] == "cell_changed"));
    assert!(
        !world.calls.iter().any(|v| v["op"] == "diagnostic"),
        "later actor effects must not run"
    );
    let mut history = PositionHistory {
        samples: Vec::new(),
        read_index: 0,
        write_index: 0,
        reset_latch: false,
        last_frame: 3,
    };
    assert!(history.sample(7, 2, 0, [0, 0]).is_err());
    assert_eq!(
        history.last_frame, 7,
        "native frame write precedes ring storage access"
    );
}
