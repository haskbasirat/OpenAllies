use aa_rules::native::{
    COMPONENTS, THING_FLAGS, base_flag_word, finalize_actor_flags, finalize_component_mask,
};
use serde_json::Value;
#[test]
fn original_base_flag_loop_and_component_mask_boundaries_match() {
    let d: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-actors/definition-flags-native.json"
    ))
    .unwrap();
    for row in d["flag_cases"].as_array().unwrap() {
        let values = std::array::from_fn(|i| row["values"][THING_FLAGS[i].0].as_bool().unwrap());
        assert_eq!(
            base_flag_word(
                row["initial"].as_u64().unwrap() as u32,
                &values,
                row["is_actor"].as_bool().unwrap()
            ),
            row["result"].as_u64().unwrap() as u32
        );
    }
    for row in d["component_branches"].as_array().unwrap() {
        let (_, index) = COMPONENTS
            .iter()
            .find(|(name, _)| *name == row["name"].as_str().unwrap())
            .unwrap();
        assert_eq!(1u32 << index, row["bit"].as_u64().unwrap() as u32);
        for c in row["cases"].as_array().unwrap() {
            let expected = c["initial"].as_u64().unwrap() as u32
                | if c["present"].as_bool().unwrap() {
                    1u32 << index
                } else {
                    0
                };
            assert_eq!(expected, c["result"].as_u64().unwrap() as u32);
        }
    }
    for row in d["performer_filters"].as_array().unwrap() {
        assert_eq!(
            finalize_component_mask(
                row["mask"].as_u64().unwrap() as u32,
                row["ability_count"].as_u64().unwrap() as u32
            ),
            row["result"].as_u64().unwrap() as u32
        );
    }
    for row in d["final_actor_flags"].as_array().unwrap() {
        let v = |s: &str| row[s].as_u64().unwrap() as u32;
        assert_eq!(
            finalize_actor_flags(
                v("initial_flags"),
                v("mask"),
                v("speed_bits"),
                v("category_index")
            )
            .unwrap(),
            v("result")
        );
    }
}
