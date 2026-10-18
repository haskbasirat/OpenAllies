use aa_spatial::moving_state::{self, Actor, Component, Error, Result, World};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Graph {
    actor: u32,
    actors: BTreeMap<u32, Actor>,
    components: BTreeMap<u32, Component>,
    states: BTreeMap<u32, u32>,
}
impl World for Graph {
    fn actor(&self, id: u32) -> Result<Actor> {
        self.actors
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::World(format!("missing actor {id}")))
    }
    fn component(&self, id: u32) -> Result<Component> {
        self.components
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::World(format!("missing component {id}")))
    }
    fn state_kind74(&self, id: u32) -> Result<u32> {
        self.states
            .get(&id)
            .copied()
            .ok_or_else(|| Error::World(format!("missing state {id}")))
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase6-collision/native-moving-fixtures.json"
    ))
    .unwrap()
}
#[test]
fn original_recursive_state_and_ordered_float_equality() {
    let fixture = fixture();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 162);
    for row in cases {
        let graph: Graph = serde_json::from_value(row["input"].clone()).unwrap();
        assert_eq!(
            moving_state::excluded(graph.actor, &graph).unwrap(),
            row["output"].as_bool().unwrap(),
            "{} {}",
            row["input"]["name"],
            row["input"]["precision"]
        );
    }
}
#[test]
fn malformed_graph_returns_error_without_default_clear_state() {
    let mut graph: Graph = serde_json::from_value(fixture()["cases"][0]["input"].clone()).unwrap();
    graph.components.get_mut(&1).unwrap().linked_actor40 = Some(1);
    assert!(matches!(
        moving_state::excluded(1, &graph),
        Err(Error::Cycle(1))
    ));
    graph.components.get_mut(&1).unwrap().linked_actor40 = Some(99);
    assert!(matches!(
        moving_state::excluded(1, &graph),
        Err(Error::World(_))
    ));
    // Restore a real terminal actor whose active component makes it moving.
    graph.components.get_mut(&1).unwrap().linked_actor40 = Some(2);
    graph.actors.get_mut(&2).unwrap().component6c = Some(3);
    graph.actors.get_mut(&2).unwrap().active_component_e4 = Some(3);
    assert!(moving_state::excluded(1, &graph).unwrap());
}
