use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    native_mobile_deployment::{Definition, NativeMobileDeploymentData},
    sources::{SourceLimits, V14Sources},
};
use serde_json::Value;

fn precision(v: &Value) -> Precision {
    match v.as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Extended64" => Precision::Extended64,
        _ => Precision::Double53,
    }
}

#[test]
fn native_aliases_keep_registered_owner_identity_and_reverse_callback_order() {
    use aa_rules::{
        actors::native_load::{NativeLoadReport, load_files},
        sources::{Origin, ResolvedFile, sha256},
        tgi::Limits,
    };
    let text = r#"
[Template A]
{
 string IDS
 string category = ACTOR
 [BodyComponent] {}
 [MobileDeploymentComponent]
 {
  string unpack_to_actor_ids = null
  fixed pack_health_min = 0.25
  point2 packed_position_offset = 0,0
 }
 [Size]
 {
  fixed height = 1
  fixed scale_min = 1
  fixed scale_max = 1
  string blocking_shape = POINT
  string soft_shape = CIRCLE
  fixed soft_radius = 1
  string target_shape = null
 }
}
[Thing template=A]
{
 IDS = first
 [MobileDeploymentComponent]
 {
  unpack_to_actor_ids = target_alias
  pack_health_min = 0.75
 }
}
[Thing template=A]
{
 IDS = second
 [MobileDeploymentComponent]
 {
  unpack_to_actor_ids = target
 }
}
[Thing template=A]
{
 IDS = target
}
[Translate]
{
 node = Thing
 ids_old = first
 ids_new = second
}
[Translate]
{
 node = Thing
 ids_old = target_alias
 ids_new = target
}
"#;
    let bytes = text.as_bytes().to_vec();
    let catalog = load_files(
        vec![ResolvedFile {
            logical_path: "mobile.tgi".into(),
            origin: Origin::V14Loose,
            sha256: sha256(&bytes),
            bytes,
        }],
        Limits::default(),
        &mut NativeLoadReport::default(),
    );
    assert_eq!(
        catalog.find("first").unwrap().id,
        "second",
        "fixture deliberately shadows first's lookup name"
    );
    let data = catalog
        .native_mobile_deployment_data(Precision::Double53)
        .unwrap();
    assert_eq!(
        data.definition("first").unwrap().id,
        "first",
        "registered owner is not re-looked-up through its alias"
    );
    assert_eq!(
        data.resolution_order,
        vec![
            ("second".into(), Some("target".into())),
            ("first".into(), Some("target".into()))
        ]
    );
    assert_eq!(
        data.definition("target").unwrap().pack_to.as_deref(),
        Some("first")
    );
    assert_eq!(
        data.definition("first").unwrap().pack_health_min_bits,
        0,
        "forward side keeps constructor0 despite authored .75"
    );
    assert_eq!(
        data.definition("target").unwrap().pack_health_min_bits,
        0x3e800000
    );
}
#[test]
fn original_pairing_order_validation_and_partial_writes_replay() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-mobile-actions/native-mobile-pairing.json"
    ))
    .unwrap();
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 49);
    for case in fixture["cases"].as_array().unwrap() {
        let c = &case["input"];
        let mut data = NativeMobileDeploymentData::default();
        for (id, spec) in c["definitions"].as_object().unwrap() {
            data.definitions.insert(
                id.clone(),
                Definition {
                    id: id.clone(),
                    body: spec["body"].as_bool().unwrap(),
                    mobile: spec["mobile"].as_bool().unwrap(),
                    element: spec["element"].as_bool().unwrap(),
                    packed_position_offset_bits: [
                        spec["offset"][0].as_u64().unwrap() as u32,
                        spec["offset"][1].as_u64().unwrap() as u32,
                    ],
                    soft_radius_bits: Some(spec["radius"].as_u64().unwrap() as u32),
                    pack_health_min_bits: 0,
                    unpack_to: None,
                    pack_to: None,
                    sources: Vec::new(),
                },
            );
        }
        let mut error = None;
        for pair in c["links"].as_array().unwrap().iter().rev() {
            let source = pair[0].as_str().unwrap();
            let target = pair[1].as_str().unwrap();
            let target = c["aliases"]
                .get(target)
                .and_then(Value::as_str)
                .unwrap_or(target);
            if let Err(e) = data.resolve_reference(source, Some(target), precision(&c["precision"]))
            {
                error = Some(e.to_string());
                break;
            }
        }
        let expected = &case["output"];
        assert_eq!(
            error.is_some(),
            !expected["error"].is_null(),
            "{}",
            c["label"]
        );
        if let Some(error) = error {
            assert_eq!(
                error,
                format!(
                    "unsupported native definition: {}",
                    expected["error"]["format"].as_str().unwrap()
                )
            );
        }
        for (id, d) in &data.definitions {
            assert_eq!(
                serde_json::to_value(&d.unpack_to).unwrap(),
                expected["definitions"][id]["unpack_to"],
                "{} {id}",
                c["label"]
            );
            assert_eq!(
                serde_json::to_value(&d.pack_to).unwrap(),
                expected["definitions"][id]["pack_to"],
                "{} {id}",
                c["label"]
            );
        }
        let actual_sources: Vec<_> = data
            .resolution_order
            .iter()
            .map(|(s, _)| s.as_str())
            .collect();
        let expected_sources: Vec<_> = expected["calls"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["owner"].as_str().unwrap())
            .collect();
        assert_eq!(actual_sources, expected_sources);
    }
}

#[test]
#[ignore = "requires original/v14 sources via AA_GAME_ROOT"]
fn shipped_mobile_links_and_health_fields_resolve_without_inference() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut source = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let catalog = ActorCatalog::load_v14(&mut source).unwrap();
    let data = catalog
        .native_mobile_deployment_data(Precision::Double53)
        .unwrap();
    assert!(!data.resolution_order.is_empty());
    let mut pairs = 0;
    for definition in data.definitions.values() {
        if let Some(target) = &definition.unpack_to {
            let target = data.definition(target).unwrap();
            assert!(target.body && target.mobile);
            assert_eq!(definition.element, target.element);
            assert_eq!(
                definition.pack_health_min_bits, 0,
                "forward side keeps constructor threshold0"
            );
            pairs += 1;
        }
    }
    println!(
        "{} ActorData definitions; {} MobileDeployment callbacks; {} resolved forward pairs",
        data.definitions.len(),
        data.resolution_order.len(),
        pairs
    );
    if let Some(path) = std::env::var_os("AA_EXPORT_MOBILE_SOURCE") {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        file.write_all(serde_json::to_string_pretty(&data).unwrap().as_bytes())
            .unwrap();
    }
}
