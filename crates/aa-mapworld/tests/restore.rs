use aa_actors::{
    creation::ObjectDefinition,
    loading::{LoadKind, construct_shell},
};
use aa_mapworld::{inventory::ObjectState, restore::restore_object_state};
use aa_simulation::NativeRng;
use serde::Deserialize;
#[derive(Deserialize)]
struct Case {
    precision: aa_numeric::Precision,
    version: u32,
    minimum: u32,
    maximum: u32,
    multiplier: u32,
    variant: i32,
    weights: Vec<u32>,
    total: u32,
    enabled: bool,
    rng_before: u32,
    native: Expected,
}
#[derive(Deserialize)]
struct Expected {
    scale_bits: u32,
    variant: u32,
    word50: u32,
    word54: u32,
    rng_state: u32,
}
#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}
#[test]
fn saved_object_scale_variants_rng_and_words_match_original_load() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-object-load.json"
    ))
    .unwrap();
    assert_eq!(fixture.cases.len(), 144);
    for c in fixture.cases {
        let definition = ObjectDefinition {
            variations_enabled: c.enabled,
            variation_weights_bits: c.weights,
            total_variation_weight_bits: c.total,
            scale_min_bits: c.minimum,
            scale_max_bits: c.maximum,
        };
        let input = ObjectState {
            version: c.version,
            scale_multiplier_bits: c.multiplier,
            variant_index: (c.version >= 1).then_some(c.variant),
            associated_object_id: (c.version >= 2).then_some(0x12345678),
            legacy_word: (c.version >= 3).then_some(65535),
            nested_count: (c.version >= 4).then_some(0),
        };
        let mut object = construct_shell(LoadKind::Object, ()).object.unwrap();
        let mut rng = NativeRng::new(c.rng_before);
        restore_object_state(&mut object, &input, &definition, &mut rng, c.precision).unwrap();
        assert_eq!(object.scale_bits, c.native.scale_bits);
        assert_eq!(object.variant as u32, c.native.variant);
        assert_eq!(object.words_50_to_54[0], c.native.word50);
        assert_eq!(object.words_50_to_54[1], c.native.word54);
        assert_eq!(rng.state, c.native.rng_state);
    }
}
