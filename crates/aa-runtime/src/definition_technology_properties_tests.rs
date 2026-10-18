use super::*;
use aa_numeric::Precision;
use serde_json::{Value, json};

#[test]
fn source_list_head_and_missing_reference_match_both_native_images_all_pc() {
    use aa_rules::{
        actors::native_load::{NativeLoadReport, load_files},
        sources::{Origin, ResolvedFile, sha256},
        tgi::Limits,
    };
    let fixture:Value=serde_json::from_slice(&std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-technology-properties/native-technology-properties.json")).unwrap()).unwrap();
    let template = "[template Thing]\n{ string IDS\n string category = ACTOR\n [optional StructureComponent]\n { [list TechnologyProperty]\n string property_IDS\n }\n}\n";
    let properties = "[Property]\n{ string IDS = first\n}\n[Property]\n{ string IDS = second\n}\n[Property]\n{ string IDS = third\n}\n";
    let file = |path: &str, text: String| ResolvedFile {
        logical_path: path.into(),
        origin: Origin::V14Loose,
        sha256: sha256(text.as_bytes()),
        bytes: text.into_bytes(),
    };
    for row in fixture["rows"].as_array().unwrap() {
        let mut source =
            "[Thing template=Thing]\n{ IDS = subject\n [StructureComponent]\n {\n".to_owned();
        for name in row["names"].as_array().unwrap() {
            source.push_str(&format!(
                "[TechnologyProperty]\n property_IDS = {}\n",
                name.as_str().unwrap()
            ));
        }
        source.push_str("}\n}\n");
        let mut report = NativeLoadReport::default();
        let actors = load_files(
            vec![
                file("template.tgi", template.into()),
                file("properties.tgi", properties.into()),
                file("subject.tgi", source),
            ],
            Limits::default(),
            &mut report,
        );
        let pc = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => unreachable!(),
        };
        let result = actors.native_statistics_technology_properties("subject", pc);
        if row["output"]["throw"].is_null() {
            assert_eq!(json!(result.unwrap()), row["output"]["values"]);
        } else {
            assert!(result.is_err());
        }
        assert_eq!(row["output"]["iterator_list_is_actual_data43c"], true);
        assert_eq!(row["output"]["iterator_starts_at_actual_head"], true);
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 sources and completed source Bank"]
fn actual_saved2697_and_positive_technology_lists_bind_same_canonical_property_bank() {
    use crate::{
        combat_source_completion as completion, combat_source_receipt::VerifiedStartupReceipt,
        settings::Settings,
    };
    use aa_mapworld::restore::RestoreDefinitions;
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = std::sync::Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(settings);
    catalog.bind_resource_catalog(resources).unwrap();
    let receipt =
        VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut sources).unwrap();
    completion::install(
        &catalog,
        &mut sources,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    completion::compute(&catalog).unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let inventory = aa_mapworld::WorldInventory::decode(&map, u64::MAX).unwrap();
    let actual = inventory
        .dynamic_things
        .placements
        .iter()
        .find(|p| p.serialized_id == 2697)
        .unwrap();
    let mut definitions = vec![String::from_utf16(&actual.thing_id_utf16).unwrap()];
    for actor in &actors.actors {
        if actors.native_category(&actor.id).unwrap().native_kind == 2
            && !actors
                .native_statistics_technology_properties(&actor.id, Precision::Double53)
                .unwrap()
                .is_empty()
        {
            definitions.push(actor.id.clone());
        }
    }
    let mut rows = Vec::new();
    for id in definitions {
        let definition = catalog
            .resolve(&id.encode_utf16().collect::<Vec<_>>(), Precision::Double53)
            .unwrap();
        let expected = actors
            .native_statistics_technology_properties(&id, Precision::Double53)
            .unwrap();
        let first = bind(&catalog, &definition).unwrap();
        let second = bind(&catalog, &definition).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), expected.len());
        let bank = catalog.definition_bank.borrow();
        for (token, id) in first.iter().zip(&expected) {
            assert_eq!(bank.token("Property", id), Some(*token));
            let entry = bank.entry(*token).unwrap();
            assert_eq!(entry.key.domain, "property");
            assert_eq!(entry.key.name, id.to_ascii_lowercase());
        }
        rows.push(json!({"id":id,"source":definition.proof.source,"head_first_ids":expected,"tokens":first}));
    }
    assert!(
        rows.len() > 1,
        "actual installed positive source cases must be exercised"
    );
    assert_eq!(
        rows[0]["id"],
        String::from_utf16(&actual.thing_id_utf16).unwrap()
    );
    if let Some(path) = std::env::var_os("AA_CAPTURE_TECHNOLOGY_PROPERTIES") {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file,&json!({"scope":"actual tutorial02 saved2697 plus all nonempty installed ActorData43C source lists; canonical identity only, no property effects","source_precision":"Double53","saved_id":2697,"rows":rows,"source_identities":actors.sources})).unwrap();
    }
}
