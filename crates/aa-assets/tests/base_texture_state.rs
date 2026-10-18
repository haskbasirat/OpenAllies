use aa_assets::{Material, RawMesh};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/public-references-20261016/graphics/material-retention-fixtures.json"
    ))
    .unwrap()
}

fn assert_state(material: &Material, expected: &Value) {
    let encoded = serde_json::to_value(material).unwrap();
    if expected.is_null() {
        assert!(encoded.get("baseTextureState").is_none());
    } else {
        assert_eq!(&encoded["baseTextureState"], expected);
    }
    let decoded: Material = serde_json::from_value(encoded).unwrap();
    assert_eq!(&decoded, material);
}

#[test]
fn original_base_descriptor_survives_decode_prepare_and_roundtrip() {
    // Three original source NIFs supply seven materials, including the bunker
    // NEAREST map, HealthBar BILERP map, and clamped AssemblyLine maps.
    // Geometry here is only a portable carrier, not native-render evidence.
    let fixture = fixture();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 7);
    for case in cases {
        let material: Material = serde_json::from_value(case["material"].clone()).unwrap();
        let mesh = RawMesh {
            name: case["mesh_name"].as_str().unwrap().into(),
            hierarchy: vec![],
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            normals: vec![0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0],
            uvs: vec![0.0; 6],
            indices: vec![0, 1, 2],
            material,
            skinned_pose: false,
        };
        let prepared = mesh.prepare().unwrap().0;
        assert_state(&prepared.material, &case["expected_base_texture_state"]);
    }
    let descriptor_only = &fixture["descriptor_only_probe"];
    let material: Material = serde_json::from_value(descriptor_only["material"].clone()).unwrap();
    assert!(material.texture.is_none());
    assert_state(&material, &descriptor_only["expected_base_texture_state"]);
}

#[test]
fn legacy_materials_do_not_acquire_authored_sampler_defaults() {
    for case in fixture()["cases"].as_array().unwrap() {
        let material: Material = serde_json::from_value(case["legacy_material"].clone()).unwrap();
        assert!(material.base_texture_state.is_none());
        assert_state(&material, &Value::Null);
    }
    assert!(Material::default().base_texture_state.is_none());
}

#[test]
fn descriptor_words_are_retained_without_interpreting_unknown_modes() {
    let words = json!({"clampMode": 37, "filterMode": 91, "uvSet": u32::MAX});
    let material: Material = serde_json::from_value(json!({"baseTextureState": words})).unwrap();
    assert_state(&material, &words);
    // Presence requires the complete authored record; it does not fill holes
    // with a Niflib constructor value or a preview sampler's choice.
    assert!(
        serde_json::from_value::<Material>(json!({
            "baseTextureState": {"clampMode": 0, "filterMode": 1}
        }))
        .is_err()
    );
}
