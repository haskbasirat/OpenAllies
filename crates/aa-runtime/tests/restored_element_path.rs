//! A saved Element Mind with a nonempty +D4 path (native-mind-load.json case
//! synthetic:path_many): aa-mind applies the collection words and hands the
//! head-first nodes to movement_world::Runtime::bind_element_restored.
use aa_mind::{
    Arena, MindId, StateId,
    construct::{self, Kind},
    saved_load,
};
use aa_numeric::Precision;
use aa_runtime::movement_world::Runtime;
use std::collections::BTreeMap;

struct ConstructorWorld;
impl construct::World for ConstructorWorld {
    fn random_word(&mut self) -> aa_mind::Result<u32> {
        Ok(0x12345678)
    }
    fn performer_ability_count(&self, _: u32) -> aa_mind::Result<usize> {
        Ok(0)
    }
    fn set_ability_jitter(&mut self, _: u32, _: usize, _: u32) -> aa_mind::Result<()> {
        panic!("no fixture abilities")
    }
}

#[test]
fn restored_path_nodes_reach_the_move_payload() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-restoration/native-mind-load.json");
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let case = doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["input"]["source"] == "synthetic:path_many")
        .unwrap();
    let hex = case["input"]["payload"].as_str().unwrap();
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let (saved, _) = saved_load::decode(Kind::Element, &bytes).unwrap();
    let constructor = construct::construct(Kind::Element, 7, &mut ConstructorWorld, Precision::Double53).unwrap();
    let ids: BTreeMap<_, _> = constructor
        .states
        .iter()
        .enumerate()
        .map(|(n, s)| (s.offset, StateId(n as u32 + 1)))
        .collect();
    let mut arena = Arena::default();
    let mut installed = constructor.install(MindId(1), &ids, &mut arena).unwrap();
    let applied = saved_load::apply(&saved, &mut installed, &mut arena).unwrap();
    let nodes = applied.element_path.unwrap();
    let native: Vec<Vec<u32>> = serde_json::from_value(case["native"]["path_head_first"].clone()).unwrap();
    assert_eq!(nodes.iter().map(|n| n.words.to_vec()).collect::<Vec<_>>(), native);
    let mut runtime = Runtime::default();
    // Plain binding refuses nothing here (pointer words stay 0) but would
    // drop the nodes; the restored entry carries them.
    let state = runtime.bind_element_restored(&mut installed, nodes.clone()).unwrap();
    let payload = runtime.handle(state).unwrap();
    let payload = payload.read().unwrap();
    assert_eq!(payload.plan.movement.path.nodes, nodes);
    assert!(payload.plan.movement.path.dirty);
    let mut bad = nodes;
    bad.pop();
    let constructor = construct::construct(Kind::Element, 7, &mut ConstructorWorld, Precision::Double53).unwrap();
    let mut arena = Arena::default();
    let mut installed = constructor.install(MindId(1), &ids, &mut arena).unwrap();
    saved_load::apply(&saved, &mut installed, &mut arena).unwrap();
    assert!(Runtime::default().bind_element_restored(&mut installed, bad).is_err() || native.last().map(|n| n[4]) .is_some_and(|k| k != 1 && k != 2));
}
