use aa_animation::definitions;
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};

fn synthetic_catalog(length: f32, ratio: f32, walk: f32, cues: &[u32]) -> ActorCatalog {
    use aa_rules::{
        sources::{Origin, ResolvedFile, sha256},
        tgi::Limits,
    };
    let mut text = format!(
        r#"
[Template A]
{{
string IDS = attack1
string type = ABILITY
string kf = /Shared/Source.kf
fixed length = {length}
fixed key_ratio = {ratio}
fixed walk_distance = {walk}
flag animation_blend = false
string sister_anim_type = null
fixed weight = 1
fixed damage_threshold = 0
int damage_filter = -1
[list WhenDoAtTime] {{ fixed time }}
}}
[Thing]
{{
IDS = test
[Animation Template=A]
{{
"#
    );
    for &bits in cues {
        text.push_str(&format!(
            "[WhenDoAtTime] {{ time = {} }}\n",
            f32::from_bits(bits)
        ));
    }
    text.push_str("}\n}\n");
    ActorCatalog::from_files(
        vec![ResolvedFile {
            logical_path: "animation-fixture.tgi".into(),
            origin: Origin::V14Loose,
            sha256: sha256(text.as_bytes()),
            bytes: text.into_bytes(),
        }],
        Limits::default(),
    )
}
#[test]
fn native_timing_stores_and_cue_head_order_match_typed_source_projection() {
    let f: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-animation-runtime/native-bindings.json"
    ))
    .unwrap();
    for c in f["stores"].as_array().unwrap() {
        let values: Vec<u32> = serde_json::from_value(c["input"].clone()).unwrap();
        let catalog = synthetic_catalog(
            f32::from_bits(values[0]),
            f32::from_bits(values[1]),
            f32::from_bits(values[2]),
            &[],
        );
        let actor = catalog.resolved_actor("test").unwrap();
        let rows = actor.children("Animation").unwrap();
        let bound = definitions::animation(
            &rows[0],
            serde_json::from_value(c["precision"].clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            serde_json::json!([
                bound.timing.length.0,
                bound.timing.use_fraction.0,
                bound.timing.walk_distance.0
            ]),
            c["fields"]
        );
    }
    for c in f["cues"].as_array().unwrap() {
        let input: Vec<u32> = serde_json::from_value(c["input"].clone()).unwrap();
        let catalog = synthetic_catalog(1.0, 0.25, 2.0, &input);
        let actor = catalog.resolved_actor("test").unwrap();
        let rows = actor.children("Animation").unwrap();
        let bound = definitions::animation(&rows[0], Precision::Double53).unwrap();
        assert_eq!(
            serde_json::to_value(bound.timing.cues).unwrap(),
            c["linked"]
        );
    }
    for kind in f["kinds"].as_array().unwrap() {
        assert_eq!(
            aa_animation::runtime::canonical_looping_type(kind["index"].as_u64().unwrap() as u32)
                .unwrap(),
            kind["looping"].as_bool().unwrap()
        );
    }
    let catalog = synthetic_catalog(1.0, 0.25, 2.0, &[]);
    let actor = catalog.resolved_actor("test").unwrap();
    let rows = actor.children("Animation").unwrap();
    let bound = definitions::animation(&rows[0], Precision::Double53).unwrap();
    for c in f["lookups"].as_array().unwrap() {
        let base = c["source"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                let mut v = bound.clone();
                v.id = row["name"].as_str().unwrap().into();
                v.timing.kind = row["kind"].as_u64().unwrap() as u32;
                v
            })
            .collect();
        let d = definitions::CharacterDefinitions {
            actor_id: "fixture".into(),
            resource_prefix: bound.fields["kf"].clone(),
            base,
            aspect_ids: vec![],
            aspects: Default::default(),
            sources: vec![],
        };
        let name = c["name"].as_str().unwrap();
        let found = if let Some(kind) = c["kind"].as_u64() {
            d.lookup(0, name, kind as u32).unwrap()
        } else {
            d.find(None, name)
        };
        let index = found.map(|v| d.base.iter().position(|row| std::ptr::eq(row, v)).unwrap());
        assert_eq!(serde_json::to_value(index).unwrap(), c["index"]);
    }
}

#[test]
fn typed_source_rejects_unsupported_duration_and_ratio_without_kf_fallback() {
    for (length, ratio) in [(0.0, 0.5), (-1.0, 0.5), (1.0, -0.1), (1.0, 1.1)] {
        let catalog = synthetic_catalog(length, ratio, 1.0, &[]);
        let actor = catalog.resolved_actor("test").unwrap();
        let rows = actor.children("Animation").unwrap();
        assert!(definitions::animation(&rows[0], Precision::Double53).is_err());
    }
}

#[test]
#[ignore = "requires original Data.rwd and accepted v14 sources via AA_GAME_ROOT"]
fn original_character_sets_preserve_authored_timing_and_aspect_fallback() {
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
    let mut rows = Vec::new();
    for id in [
        "usa_infantry",
        "ger_infantry",
        "gbr_infantry",
        "jpn_infantry",
        "rus_infantry",
        "usa_tank_medium",
    ] {
        let definition = definitions::character(&catalog, id, Precision::Double53).unwrap();
        if let Some(d) = &definition {
            assert!(!d.base.is_empty(), "{id}");
            assert_eq!(
                d.for_variant(d.aspect_ids.len() as u32).unwrap().len(),
                d.base.len()
            );
            assert!(d.for_variant(d.set_count() as u32).is_err());
            for set in 0..d.set_count() as u32 {
                let attack = d.lookup(set, "attack1", 4).unwrap().unwrap();
                assert_eq!(attack.timing.length.0, 1.0_f32.to_bits(), "{id} set{set}");
                assert!(attack.timing.use_fraction.0 != 0);
                assert!(d.lookup(set, "attack1", 0).unwrap().is_none());
                assert!(d.lookup(set, "ATTACK1", 4).unwrap().is_some());
            }
            let performer = catalog
                .native_loaded_performer(id, Precision::Double53, Some(d))
                .unwrap()
                .unwrap();
            assert_eq!(performer.completed_ability_count(), 2, "{id}");
            for ability in performer.abilities() {
                assert_eq!(
                    ability.authored.fields["reload_time"].fixed_bits().unwrap(),
                    0,
                    "{id}"
                );
                assert_eq!(ability.animations.len(), d.set_count());
            }
            if id == "usa_infantry" {
                assert_eq!(
                    performer
                        .abilities()
                        .iter()
                        .map(|a| a.recharge_bits)
                        .collect::<Vec<_>>(),
                    [3.0_f32.to_bits(), 15.0_f32.to_bits()]
                );
            }
        }
        rows.push(serde_json::json!({"actor":id,"definition":definition}));
    }
    let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase7-animation-runtime/original-actor-animation-sets.json");
    std::fs::write(output, serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
}
