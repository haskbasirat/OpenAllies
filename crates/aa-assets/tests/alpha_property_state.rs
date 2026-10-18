use aa_assets::{AlphaPropertyState, Material, RawMesh};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/byroredux-20261016/alpha-retention-fixtures.json"
    ))
    .unwrap()
}

#[test]
fn byro_validated_original_alpha_fields_survive_prepare_and_roundtrip() {
    let data = fixture();
    let cases = data["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 95);
    for case in cases {
        let material: Material = serde_json::from_value(case["material"].clone()).unwrap();
        let mesh = RawMesh {
            name: "portable metadata carrier".into(),
            hierarchy: vec![],
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
            uvs: vec![0.0; 6],
            indices: vec![0, 1, 2],
            material,
            skinned_pose: false,
        };
        let prepared = mesh.prepare().unwrap().0;
        let encoded = serde_json::to_value(&prepared.material).unwrap();
        assert_eq!(encoded["alphaPropertyState"], case["expected"]);
        assert_eq!(
            serde_json::from_value::<Material>(encoded).unwrap(),
            prepared.material
        );
        // Raw metadata alone never supplies source-bound rendering state.
        assert!(prepared.material.native_render_state.is_none());
    }
}

#[test]
fn older_payloads_and_absent_authored_alpha_keep_metadata_absent() {
    let data = fixture();
    for case in data["cases"].as_array().unwrap() {
        let old: Material = serde_json::from_value(case["legacy_material"].clone()).unwrap();
        assert!(old.alpha_property_state.is_none());
        assert!(
            serde_json::to_value(old)
                .unwrap()
                .get("alphaPropertyState")
                .is_none()
        );
    }
    let absent: Material = serde_json::from_value(data["absent_material"].clone()).unwrap();
    assert!(absent.alpha_property_state.is_none());
    assert!(Material::default().alpha_property_state.is_none());
}

#[test]
fn authored_alpha_record_requires_both_exact_width_words() {
    let material: Material = serde_json::from_value(json!({
        "alphaPropertyState": {"flags": 65535, "threshold": 255}
    }))
    .unwrap();
    assert_eq!(
        material.alpha_property_state,
        Some(AlphaPropertyState {
            flags: u16::MAX,
            threshold: u8::MAX,
        })
    );
    for raw in [
        json!({"flags": 1}),
        json!({"threshold": 0}),
        json!({"flags": 65536, "threshold": 0}),
        json!({"flags": 1, "threshold": 256}),
        json!({"flags": -1, "threshold": 0}),
    ] {
        assert!(serde_json::from_value::<Material>(json!({"alphaPropertyState": raw})).is_err());
    }
}
