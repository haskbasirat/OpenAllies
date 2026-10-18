use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    property_inputs::PropertyModifierInputs,
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    targets::parse_flag_word,
    tgi::Limits,
};
fn file(path: &str, text: String) -> ResolvedFile {
    ResolvedFile {
        logical_path: path.into(),
        origin: Origin::V14Loose,
        sha256: sha256(text.as_bytes()),
        bytes: text.into_bytes(),
    }
}
#[test]
fn original_property_target_words_and_resistance_stores_are_exact() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-property-definitions.json"
    ))
    .unwrap();
    for row in fixture["target_flag_cases"].as_array().unwrap() {
        let out = parse_flag_word(
            row["raw"].as_str().unwrap(),
            row["initial"].as_u64().unwrap() as u32,
            |name| Ok(row["registry"][name].as_u64().unwrap() as u32),
        )
        .unwrap();
        assert_eq!(out as u64, row["output"].as_u64().unwrap());
    }
    for row in fixture["resistance_cases"].as_array().unwrap() {
        let bits = row["amount"].as_u64().unwrap() as u32;
        let amount = f32::from_bits(bits);
        let pc = serde_json::from_value(row["precision"].clone()).unwrap();
        let source = format!(
            "[Property]{{\nstring IDS=test\nflag is_technology=false\nstring enabled_target_flags=NULL\nstring required_properties=NULL\nfixed performer_cv_bias=0\nflag pass_down_recruit=false\nflag pass_down_denizen=false\nflag pass_down_members=true\nbool is_role_property=false\n[Modifier]{{\nstring type=RESIST_DAMAGE\nstring damage_type=damage\nfixed amount={amount}\n}}\n}}\n"
        );
        let catalog = ActorCatalog::from_files(
            vec![
                file("property.tgi", source),
                file(
                    "Game/damage_types.tgi",
                    "[DamageType]{string IDS=damage}\n".into(),
                ),
            ],
            Limits::default(),
        );
        let out = catalog.native_shared_property_inputs("TEST", pc).unwrap();
        let [
            PropertyModifierInputs::ResistDamage {
                amount,
                damage_type,
                damage_type_index,
                ..
            },
        ] = out.modifiers.as_slice()
        else {
            panic!()
        };
        assert_eq!(
            amount.fixed_bits().unwrap() as u64,
            row["output"]["amount"].as_u64().unwrap()
        );
        assert_eq!(damage_type.id, "damage");
        assert_eq!(*damage_type_index, 0);
        assert_eq!(out.target_flags, 0);
    }
}
#[test]
#[ignore = "requires original/v14 files via AA_GAME_ROOT"]
fn original_initial_property_corpus_and_camera_inputs_are_complete_within_scope() {
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
    let pc = Precision::Double53;
    let mut ids = std::collections::BTreeSet::new();
    for id in [
        "usa_infantry",
        "gbr_infantry",
        "ger_infantry",
        "rus_infantry",
        "jpn_infantry",
        "regiment_usa_infantry",
    ] {
        for prop in catalog
            .native_actor_init_inputs(id, pc)
            .unwrap()
            .initial_properties
        {
            ids.insert(prop.id);
        }
    }
    assert_eq!(ids.len(), 14);
    let mut out = Vec::new();
    for id in ids {
        let property = catalog.native_shared_property_inputs(&id, pc).unwrap();
        assert!(property.fx_sources.values().all(Vec::is_empty));
        if id == "infantry" {
            assert_eq!(property.target_flags, 1 << 20);
            let [
                PropertyModifierInputs::ResistDamage {
                    damage_type,
                    damage_type_index,
                    amount,
                    ..
                },
            ] = property.modifiers.as_slice()
            else {
                panic!()
            };
            assert_eq!(damage_type.id, "armor_piercing");
            assert_eq!(*damage_type_index, 3);
            assert_eq!(amount.fixed_bits().unwrap(), 0x3e4ccccd);
        } else {
            assert!(property.modifiers.is_empty());
            assert_eq!(
                property.target_flags,
                if id == "org_infantry" { 1 << 31 } else { 0 }
            );
        }
        out.push(property);
    }
    let camera = catalog.native_camera_inputs("CAMERA", pc).unwrap();
    assert_eq!(camera.id, "camera");
    assert_eq!(
        catalog.native_final_flags("camera", pc).unwrap(),
        catalog.native_base_flags("camera", false, pc).unwrap().0
    );
    catalog.native_object_definition("camera", pc).unwrap();
    println!("{}", serde_json::json!({"properties":out,"camera":camera}));
}
