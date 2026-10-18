use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    commander::EFFECT_KINDS,
    property_inputs::MODIFIER_FLAGS,
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
#[test]
fn native_registered_effect_and_property_flag_domains_are_exact() {
    let effects: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-faction-constructors.json"
    ))
    .unwrap();
    for (i, row) in effects["effect_kinds"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        assert_eq!(row["kind"].as_u64().unwrap() as usize, i);
        assert_eq!(row["name"], EFFECT_KINDS[i]);
    }
    let flags: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-actor-live/native-property-flags.json"
    ))
    .unwrap();
    for (i, row) in flags["registration"].as_array().unwrap().iter().enumerate() {
        assert_eq!(row["index"].as_u64().unwrap() as usize, i);
        assert_eq!(row["name"], MODIFIER_FLAGS[i]);
    }
}
#[test]
fn unsupported_property_modifiers_are_not_silently_discarded() {
    let src = "[Property]{\nstring IDS=sample\nflag is_technology=false\n[Modifier]{string type=ADD}\n}\n";
    let cat = ActorCatalog::from_files(
        vec![ResolvedFile {
            logical_path: "property.tgi".into(),
            origin: Origin::V14Loose,
            sha256: sha256(src.as_bytes()),
            bytes: src.as_bytes().to_vec(),
        }],
        Limits::default(),
    );
    assert!(
        cat.native_shared_flag_property("sample", Precision::Double53)
            .unwrap_err()
            .to_string()
            .contains("cannot discard ADD")
    );
}
#[test]
#[ignore = "requires original/v14 files via AA_GAME_ROOT"]
fn original_commander_badges_and_indispensable_are_source_bound() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let catalog = ActorCatalog::load_v14(&mut sources).unwrap();
    let pc = Precision::Double53;
    let property = catalog
        .native_shared_flag_property("INDISPENSABLE", pc)
        .unwrap();
    assert_eq!(property.id, "indispensable");
    assert!(property.is_technology.boolean().unwrap());
    assert_eq!(property.modifiers.len(), 1);
    assert_eq!(property.modifiers[0].index, 11);
    assert!(property.fx_sources.values().all(Vec::is_empty));
    let mut commanders = Vec::new();
    for (id, count) in [
        ("zhukov_tutorial", 1),
        ("patton_tutorial", 0),
        ("rundstedt", 4),
    ] {
        let out = catalog.native_commander_constructor_inputs(id, pc).unwrap();
        assert_eq!(out.abilities.len(), count);
        // template_faction_aa.tgi experience_max = 5000 (int, fild/fstp).
        assert_eq!(out.experience_max_bits, 5000f32.to_bits(), "{id}");
        assert!(
            out.abilities
                .iter()
                .all(|a| a.faction_range_bits == 0x7f7fffff)
        );
        commanders.push(out);
    }
    let mut badges = Vec::new();
    for node in catalog.resolved_definitions("Nation").unwrap() {
        let Some(aa_rules::native::TypedValue::String { value: id, .. }) =
            node.field("IDS", pc).unwrap()
        else {
            panic!()
        };
        badges.push(catalog.native_nation_badges(&id, pc).unwrap());
    }
    println!(
        "{}",
        serde_json::json!({"property":property,"commanders":commanders,"badges":badges})
    );
    assert!(!badges.is_empty());
}
