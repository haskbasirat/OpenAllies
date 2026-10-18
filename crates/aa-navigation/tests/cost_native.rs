use aa_navigation::{cost, search::QueueEntry};
use aa_numeric::Precision;
use serde_json::{Value, json};

struct World {
    input: Value,
    events: Vec<Value>,
}
impl World {
    fn bits(&self, key: &str) -> u32 {
        self.input[key].as_u64().unwrap() as u32
    }
    fn flag(&self, key: &str) -> bool {
        self.input[key].as_bool().unwrap()
    }
}
fn pair(v: &Value) -> [i32; 2] {
    std::array::from_fn(|i| v[i].as_i64().unwrap() as i32)
}
impl cost::World for World {
    fn tile_actor(&mut self, cell: [i32; 2]) -> cost::Result<Option<u32>> {
        self.events.push(json!(["tile_actor", cell[0], cell[1]]));
        let id = if cell == pair(&self.input["from"]) {
            self.bits("source_actor")
        } else {
            self.bits("destination_actor")
        };
        Ok((id != 0).then_some(id))
    }
    fn has_wall_component(&self, actor: u32) -> cost::Result<bool> {
        Ok(actor == 1 && self.flag("has_wall"))
    }
    fn outer_actor(&mut self, _actor: u32, state: &mut cost::State) -> cost::Result<Option<u32>> {
        self.events.push(json!(["outer_actor", 1]));
        if let Some(m) = self.input["mutation"].as_object() {
            if let Some(value) = m.get("collision_owner") {
                state.collision_owner = value.as_bool().unwrap().then_some(1);
            }
            if let Some(value) = m.get("maximum_speed_bits") {
                state.maximum_speed_bits = value.as_u64().unwrap() as u32;
            }
            if let Some(value) = m.get("cost_bits") {
                state.current.cost_bits = value.as_u64().unwrap() as u32;
            }
            if let Some(value) = m.get("health_bits").cloned() {
                self.input["health_bits"] = value;
            }
        }
        Ok(self.flag("outer_owner").then_some(1))
    }
    fn pathing_wall_cost_bits(&mut self) -> cost::Result<u32> {
        self.events
            .push(json!(["wall_factor", self.bits("wall_factor_bits")]));
        Ok(self.bits("wall_factor_bits"))
    }
    fn body_health_bits(&mut self, _actor: u32) -> cost::Result<Option<[u32; 2]>> {
        self.events.push(json!(["body_health", 1]));
        Ok(self
            .flag("has_body")
            .then(|| [self.bits("health_bits"), self.bits("maximum_health_bits")]))
    }
    fn prepare_terrain_penalty(&mut self) -> cost::Result<()> {
        self.events.push(json!(["prepare_terrain"]));
        Ok(())
    }
    fn terrain_speed_bits(&mut self, cell: [i32; 2]) -> cost::Result<u32> {
        self.events.push(json!(["terrain_speed", cell[0], cell[1]]));
        Ok(self.bits(if cell == pair(&self.input["from"]) {
            "source_speed_bits"
        } else {
            "destination_speed_bits"
        }))
    }
    fn terrain_change_penalty_bits(&mut self) -> cost::Result<u32> {
        self.events
            .push(json!(["terrain_penalty", self.bits("penalty_bits")]));
        Ok(self.bits("penalty_bits"))
    }
}

#[test]
fn native_wall_terrain_cost_arithmetic_and_virtual_mutation_order() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-pathcost/native-cost-fixtures.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 177);
    for case in cases {
        let input = &case["input"];
        let expected = &case["output"];
        let mut world = World {
            input: input.clone(),
            events: Vec::new(),
        };
        let mut state = cost::State {
            current: QueueEntry {
                cell: pair(&input["from"]),
                depth: 0,
                priority_bits: 0,
                cost_bits: world.bits("cost_bits"),
            },
            maximum_speed_bits: world.bits("maximum_speed_bits"),
            collision_owner: world.flag("collision_owner").then_some(1),
        };
        let precision: Precision = serde_json::from_value(input["precision"].clone()).unwrap();
        let actual = cost::accumulated_cost(
            world.bits("cell_size_bits"),
            pair(&input["to"]),
            &mut state,
            precision,
            &mut world,
        )
        .unwrap();
        assert_eq!(
            actual,
            expected["result_bits"].as_u64().unwrap() as u32,
            "{} {}",
            input["name"],
            input["precision"]
        );
        assert_eq!(
            world.events,
            *expected["events"].as_array().unwrap(),
            "events {} {}",
            input["name"],
            input["precision"]
        );
        assert_eq!(
            json!({"cost_bits":state.current.cost_bits,"maximum_speed_bits":state.maximum_speed_bits,
                         "collision_owner":state.collision_owner.is_some()}),
            expected["state"]
        );
    }
}
