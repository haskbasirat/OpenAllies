use aa_navigation::region_search::*;
use aa_numeric::Precision;
use serde::Deserialize;
use serde_json::{Value, json};
#[derive(Deserialize)]
struct Case {
    name: String,
    precision: Precision,
    ready: bool,
    busy: bool,
    epoch: u8,
    start: u32,
    goal: u32,
    queue_limit: Option<usize>,
    nodes: Vec<RegionNode>,
}
#[test]
fn concrete_region_search_matches_native_queue_routes_and_visit_state() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-region/graph-fixtures.json"
    ))
    .unwrap();
    for row in doc["fixtures"].as_array().unwrap() {
        let c: Case = serde_json::from_value(row["input"].clone()).unwrap();
        let mut graph = RegionGraph {
            nodes: c.nodes,
            epoch: c.epoch,
            ready: c.ready,
        };
        let mut state = State {
            busy: c.busy,
            output_linked: false,
        };
        let mut path = vec![];
        let result = search(
            &mut graph,
            Request {
                start: c.start,
                goal: c.goal,
                precision: c.precision,
                queue_limit: c.queue_limit,
            },
            &mut state,
            &mut path,
        )
        .unwrap_or_else(|e| panic!("{}: {e}", c.name));
        let actual = json!({"status":result.status,"route":path,"popped":result.popped.into_iter().map(QueueEntry::words).collect::<Vec<_>>(),"discovered":result.discovered,"node_states":graph.nodes.iter().map(|n|[n.visit_epoch,n.parent_neighbor]).collect::<Vec<_>>(),"epoch":graph.epoch,"busy":state.busy,"output_linked":state.output_linked,"queue_remaining":result.queue_remaining});
        let mut expected = row["output"].clone();
        // Native ABI is checked by the x86 oracle, not by the Rust port.
        expected.as_object_mut().unwrap().remove("abi_preserved");
        assert_eq!(actual, expected, "{}", c.name);
    }
}

#[test]
fn a_missing_native_goal_chain_is_explicit_and_nontransactional() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-region/graph-fixtures.json"
    ))
    .unwrap();
    let row = doc["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["input"]["name"] == "diamond_Double53")
        .unwrap();
    let c: Case = serde_json::from_value(row["input"].clone()).unwrap();
    let mut graph = RegionGraph {
        nodes: c.nodes,
        epoch: c.epoch,
        ready: c.ready,
    };
    let mut state = State {
        busy: false,
        output_linked: false,
    };
    let mut path = vec![];
    // Native capacity exhaustion invokes its goal finalizer even though the
    // goal has no predecessor yet. Safe Rust exposes that invalid chain.
    let result = search(
        &mut graph,
        Request {
            start: c.start,
            goal: c.goal,
            precision: c.precision,
            queue_limit: Some(1),
        },
        &mut state,
        &mut path,
    );
    assert!(matches!(result, Err(Error::InvalidPredecessor(3))));
    assert_eq!(path, vec![3]);
    assert!(state.busy && state.output_linked);
    assert_eq!(graph.epoch, 1);
}
