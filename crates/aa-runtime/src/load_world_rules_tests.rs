use super::*;

#[test]
fn original_native_vector_overlay_and_editable_reset() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-mission-start/native-world-rules.json"
    ))
    .unwrap();
    for row in fixture["cases"].as_array().unwrap() {
        let c = &row["input"];
        // Explicit descriptor fixture inputs to the bounded value kernel;
        // these cannot construct a public source-backed Owner.
        let mut records: Vec<_> = c["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let kind = d["kind"].as_u64().unwrap() as u32;
                let default =
                    d["default"].as_u64().unwrap() as u32 & if kind == 0 { 255 } else { u32::MAX };
                Record {
                    ids: d["ids"].as_str().unwrap().into(),
                    kind,
                    default,
                    editable: d["editable"] == 1,
                    enum_ids: Vec::new(),
                    source: aa_rules::resources::Provenance {
                        path: "explicit-native-fixture".into(),
                        sha256: String::new(),
                        span: aa_rules::tgi::Span { start: 0, end: 0 },
                    },
                    path: vec![i],
                    locked: 0,
                    value: default,
                }
            })
            .collect();
        let projection = |rows: &[Record]| {
            serde_json::json!(
                rows.iter()
                    .map(|r| serde_json::json!({
            "ids":r.ids,"locked":r.locked,"value":r.value}))
                    .collect::<Vec<_>>()
            )
        };
        assert_eq!(projection(&records), row["output"]["defaults"]);
        for saved in c["saved"].as_array().unwrap() {
            let raw = saved["ids"].as_str().unwrap();
            let translated = c["translations"][raw.to_ascii_lowercase()]
                .as_str()
                .unwrap_or(raw);
            if let Some(record) = records
                .iter_mut()
                .find(|r| r.ids.eq_ignore_ascii_case(translated))
            {
                overlay(
                    record,
                    saved["kind"].as_u64().unwrap() as u32,
                    saved["locked"].as_u64().unwrap() as u8,
                    saved["value"].as_u64().unwrap() as u32,
                );
            }
        }
        assert_eq!(projection(&records), row["output"]["loaded"], "{row}");
        reset_editable(&mut records, c["load_mode"].as_u64().unwrap() as u32);
        assert_eq!(projection(&records), row["output"]["current"], "{row}");
        for (name, expected) in row["output"]["find"].as_object().unwrap() {
            assert_eq!(
                serde_json::json!(
                    records
                        .iter()
                        .position(|r| r.ids.eq_ignore_ascii_case(name))
                ),
                *expected
            );
        }
    }
}

#[test]
#[ignore = "requires actual AA_GAME_ROOT source and sealed startup receipt"]
fn actual_owner_keeps_all_source_defaults_saved_order_and_current_mutations() {
    use aa_rules::sources::{SourceLimits, V14Sources};
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let actors = aa_rules::actors::ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(
        crate::settings::Settings::load_v14(&mut files, aa_rules::tgi::Limits::default()).unwrap(),
    );
    let proof = crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(
        &catalog, &mut files,
    )
    .unwrap();
    crate::combat_source_completion::install(
        &catalog,
        &mut files,
        proof.context(),
        proof.csv(),
        proof.globals(),
    )
    .unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    catalog.native_load_mode = Some(1);
    let mut captures = Vec::new();
    for name in [
        "Campaign/tutorial02",
        "DeveloperMaps/(3v3)Firefight",
        "(2v2)Berlin",
    ] {
        let map = aa_mapworld::MapArchive::from_bytes(
            std::fs::read(game.join(format!(
                "reconstruction/assets/original/Data/Maps/{name}.RMP"
            )))
            .unwrap(),
        )
        .unwrap();
        let inventory = WorldInventory::decode(&map, u64::MAX).unwrap();
        let mut owner = Owner::load(&catalog, &inventory).unwrap();
        assert_eq!(owner.record("FogOfWar").unwrap().default_value(), 1);
        assert_eq!(
            owner
                .record("ResourceStockpileDefault")
                .unwrap()
                .default_value(),
            500f32.to_bits()
        );
        assert_eq!(
            owner.record("StartingExperience").unwrap().default_value(),
            200f32.to_bits()
        );
        assert_eq!(owner.record("CityVictoryRatio").unwrap().default_value(), 0);
        assert_eq!(
            owner.record("Sun").unwrap().enum_ids(0).unwrap(),
            "sun_default"
        );
        owner
            .validate_in_bank(&catalog, &catalog.definition_bank.borrow())
            .unwrap();
        let defaults:Vec<_>=owner.records().map(|v|serde_json::json!({"ids":v.ids,"kind":v.kind,
            "default":v.default,"editable":u8::from(v.editable),"enum_ids":v.enum_ids,"value":v.value,"locked":v.locked})).collect();
        let section = inventory
            .opaque_initialization
            .iter()
            .find(|v| v.name == "RULES")
            .unwrap();
        captures.push(
            serde_json::json!({"map":name,"rules_sha256":section.source.sha256,"records":defaults}),
        );
        let ptr = owner.records.as_ptr();
        owner
            .assign_word("cItYvIcToRyRaTiO", 1, 0x80000001)
            .unwrap();
        assert_eq!(ptr, owner.records.as_ptr());
        assert_eq!(
            owner.record("CityVictoryRatio").unwrap().value(),
            0x80000001
        );
        let projected = owner.to_serialized();
        assert_eq!(projected.rules.len(), owner.records.len());
        assert_eq!(
            projected
                .rules
                .iter()
                .find(|v| String::from_utf16(&v.key).unwrap() == "CityVictoryRatio")
                .unwrap()
                .value,
            0x80000001
        );
        assert!(owner.assign_word("CityVictoryRatio", 2, 0).is_err());
    }
    if let Some(path) = std::env::var_os("AA_WORLD_RULES_CAPTURE") {
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&serde_json::json!({"cases":captures})).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn actual_native_bool_writer_first_lookup_raw_byte_and_counter_wrap() {
    let f: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-mission-start/native-world-rule-bool.json"
    ))
    .unwrap();
    for row in f["cases"].as_array().unwrap() {
        //Explicit scalar leaf inputs; no public source-backed owner is minted.
        let c = &row["input"];
        let mut records: Vec<_> = c["records"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, v)| Record {
                ids: v["ids"].as_str().unwrap().into(),
                kind: v["kind"].as_u64().unwrap() as u32,
                default: 0,
                editable: false,
                enum_ids: Vec::new(),
                source: aa_rules::resources::Provenance {
                    path: "explicit-bool-leaf".into(),
                    sha256: String::new(),
                    span: aa_rules::tgi::Span { start: 0, end: 0 },
                },
                path: vec![i],
                locked: 0x66,
                value: v["word"].as_u64().unwrap() as u32,
            })
            .collect();
        let mut changes = c["changes"].as_u64().unwrap() as u32;
        for (value, expected) in c["values"]
            .as_array()
            .unwrap()
            .iter()
            .zip(row["output"].as_array().unwrap())
        {
            assign_bool_record(
                &mut records,
                &mut changes,
                c["name"].as_str().unwrap(),
                value.as_u64().unwrap() as u8,
            );
            assert_eq!(
                serde_json::json!({"changes":changes,"words":records.iter().map(|r|r.value).collect::<Vec<_>>()}),
                *expected,
                "{row}"
            );
            assert!(records.iter().all(|r| r.locked == 0x66));
        }
    }
}
