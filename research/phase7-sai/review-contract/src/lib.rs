#![cfg(test)]
use aa_navigation::{
    ai_startup::{self, Inputs},
    sai_state::{self, Kingdom},
    startup::Object,
    topology::{GridLayout, Topology},
};
use serde_json::{Value, json};
#[test]
fn independent_invalid_source_bindings_cannot_return_ready_candidate() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../native-ai.json");
    let doc: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    for row in doc["fixtures"].as_array().unwrap() {
        let c = &row["input"];
        let v = &c["layout"];
        let bytes = c["input_hex"]
            .as_str()
            .unwrap()
            .as_bytes()
            .chunks_exact(2)
            .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
            .collect::<Vec<_>>();
        let (topology, _) = Topology::decode_prefix(
            &bytes,
            GridLayout {
                width: v["width"].as_i64().unwrap() as i32,
                height: v["height"].as_i64().unwrap() as i32,
                coordinate_shift: v["coordinate_shift"].as_u64().unwrap() as u32,
                extent_bits: std::array::from_fn(|i| {
                    (v["extent_i32"][i].as_i64().unwrap() as f32).to_bits()
                }),
            },
            28,
        )
        .unwrap();
        let graph = topology.prepare_graph().unwrap();
        let kingdoms: Vec<Option<Kingdom>> = serde_json::from_value(c["kingdoms"].clone()).unwrap();
        let mut states = sai_state::State::new(graph.nodes.len()).unwrap();
        for kingdom in kingdoms.iter().flatten() {
            states
                .register_kingdom_regions(
                    *kingdom,
                    &topology,
                    &vec![
                        sai_state::TileKnowledge {
                            word14: 0,
                            word18: 0
                        };
                        (topology.layout.width * topology.layout.height) as usize
                    ],
                    sai_state::VisibilitySettings {
                        fog_enabled: false,
                        visible_bit_offset: 16,
                    },
                )
                .unwrap();
        }
        let input = Inputs {
            controller: Object(100),
            player: Object(200),
            player_index: c["player_index"].as_i64().unwrap() as i32,
            registered_ids: serde_json::from_value(c["registered_ids"].clone()).unwrap(),
            kingdoms,
            own_slot: c["own_slot"].as_u64().unwrap() as usize,
            ability_counts: serde_json::from_value(c["ability_counts"].clone()).unwrap(),
            script_count: c["script_count"].as_u64().unwrap() as usize,
            neighbor_depth: c["neighbor_depth"].as_i64().unwrap() as i32,
            recruitment: serde_json::from_value(c["recruitment"].clone()).unwrap(),
            owned_units: serde_json::from_value(c["owned_units"].clone()).unwrap(),
        };
        let mut bad=input.clone();bad.player_index=-1;assert!(ai_startup::initialize(&bad,&graph,&states).is_err());
        let mut bad=input.clone();bad.registered_ids.push(0x10000);assert!(ai_startup::initialize(&bad,&graph,&states).is_err());
        let mut bad=input.clone();bad.own_slot=bad.kingdoms.len();assert!(ai_startup::initialize(&bad,&graph,&states).is_err());
        let wrong_state=sai_state::State::new(graph.nodes.len()+1).unwrap();assert!(ai_startup::initialize(&input,&graph,&wrong_state).is_err());
        let mut bad=input.clone();bad.recruitment=Some(aa_navigation::ai_startup::Recruitment{options:vec![Some(aa_navigation::ai_startup::RecruitmentOption{object:Object(999),definition_type:Some(7)})],enabled_types:vec![]});assert!(ai_startup::initialize(&bad,&graph,&states).is_err());
        let got = ai_startup::initialize(&input, &graph, &states).unwrap();
        let region_players=got.region_players.iter().map(|r|json!({"region":r.region,"player":r.player,"land":r.land,"water":r.water,"word34":r.word34,"arrays":r.arrays.iter().map(Vec::len).collect::<Vec<_>>()})).collect::<Vec<_>>();
        let actual = json!({"ready":got.ready,"script_words":got.script_words,"word10":got.word10,"word14":got.word14,"pending_count":got.controller.pending_kingdom_goals.len(),"groups":got.controller.groups,"goals":got.goals,"region_kingdoms":got.region_kingdoms,"region_players":region_players});
        assert_eq!(actual, row["native"], "{}", c["name"]);
    }
}
