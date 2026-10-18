use aa_runtime::world_object_list::{PreviousLink, WorldObjectList};
use serde_json::{Value, json};

fn snapshot(list: &WorldObjectList<u32>) -> Value {
    json!({
        "head": list.head().map(|id| id.ordinal()),
        "nodes": list.nodes().iter().enumerate().map(|(i, node)| json!({
            "node": i + 1,
            "object": node.object,
            "previous_link": match node.previous_link {
                PreviousLink::Head => json!({"head": true}),
                PreviousLink::Next(id) => json!({"next_of": id.ordinal()}),
            },
            "next": node.next.map(|id| id.ordinal()),
        })).collect::<Vec<_>>(),
    })
}

#[test]
fn original_and_v14_object_list_constructor_insertions_and_backlinks_match() {
    let proof: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-map-startup/native-world-object-list.json"
    ))
    .unwrap();
    assert_eq!(proof["paired_case_count"], 36);
    for case in proof["cases"].as_array().unwrap() {
        let mut list = WorldObjectList::constructed();
        assert!(list.head().is_none());
        assert!(list.nodes().is_empty());
        for (input, output) in case["sequence"]
            .as_array()
            .unwrap()
            .iter()
            .zip(case["output"]["steps"].as_array().unwrap())
        {
            let object = input[0].as_u64().unwrap() as u32;
            let flags = input[1].as_u64().unwrap() as u32;
            let inserted = list.insert(object, flags).unwrap();
            assert_eq!(inserted.is_some(), flags & 0x100 != 0);
            if let Some(id) = inserted {
                assert_eq!(list.node(id).unwrap().object, object);
            }
            assert_eq!(snapshot(&list), output["state"], "{case}");
        }
        assert_eq!(
            list.nodes().len(),
            case["output"]["allocations"].as_array().unwrap().len()
        );
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets; source inspection only"]
fn actual_first_map_records_identify_immediate_init_dispatch_inputs() {
    use aa_mapworld::{MapArchive, WorldInventory, restore::RestoreDefinitions};
    use aa_numeric::Precision;
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use aa_runtime::{catalog::Catalog, localization::Localization, settings::Settings};
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(settings);
    catalog.replace_current_text(text.clone());
    catalog.definition_text = Some(text);
    let mut results = Vec::new();
    for name in [
        "Campaign/tutorial02.RMP",
        "DeveloperMaps/(3v3)Firefight.RMP",
        "(2v2)Berlin.RMP",
    ] {
        let map = MapArchive::from_bytes(
            std::fs::read(
                root.join("reconstruction/assets/original/Data/Maps")
                    .join(name),
            )
            .unwrap(),
        )
        .unwrap();
        let input = WorldInventory::decode(&map, u64::MAX).unwrap();
        let mut records = Vec::new();
        let mut inspect =
            |stage,
             source_id: &[u16],
             id,
             thing: &aa_mapworld::ThingState,
             object: Option<&aa_mapworld::inventory::ObjectState>| {
                let resolved = catalog.resolve(source_id, Precision::Double53).unwrap();
                let flags = thing.runtime_flags(0x8000_0000, resolved.flags);
                let geometry_bound = if matches!(resolved.native_class, 2..=4) {
                    for pc in [
                        Precision::Single24,
                        Precision::Double53,
                        Precision::Extended64,
                    ] {
                        catalog
                            .actors
                            .native_object_definition(&resolved.proof.id, pc)
                            .unwrap();
                    }
                    assert!(
                        aa_runtime::live_object_init::Source::bind(&catalog, &resolved).is_err(),
                        "source-only catalog has no completed source Bank owner"
                    );
                    true
                } else {
                    false
                };
                records.push(
                    json!({"stage":stage,"saved_id":id,"definition":resolved.proof.id,
                "native_class":resolved.native_class,"saved_flags":thing.serialized_flags,
                "definition_flags":resolved.flags,"runtime_flags_before_init":flags,
                "diagnostic_definition_geometry_evaluated_all_three_pc":geometry_bound,
                "live_source_owner_bound":false,
                "serialized_object_state":object,
                "bit7_skips_init":flags & 0x80 != 0,
                "position_bits":thing.position_bits,"facing_bits":thing.facing_bits}),
                );
            };
        if let Some(source) = input
            .teams
            .as_ref()
            .and_then(|v| v.slots.iter().find_map(|s| s.record.as_ref()))
        {
            inspect(
                "Teams",
                &source.definition_id_utf16,
                source.serialized_id,
                &source.thing,
                None,
            );
        }
        if let Some(source) = input
            .kingdoms
            .as_ref()
            .and_then(|v| v.slots.iter().find_map(|s| s.record.as_ref()))
        {
            inspect(
                "Kingdoms",
                &source.definition_id_utf16,
                source.serialized_id,
                &source.thing,
                source.object.as_ref(),
            );
        }
        if let Some(source) = input.dynamic_things.placements.first() {
            inspect(
                "DynamicThings",
                &source.thing_id_utf16,
                source.serialized_id,
                &source.thing,
                source.object.as_ref(),
            );
        }
        results.push(json!({"map":name,"map_source":input.map_source,"world_version":input.world.version,
            "world_extents_bits":input.header.as_ref().map(|h| h.creator.world_extents_bits),"records":records}));
    }
    let report = json!({"scope":"Actual source record dispatch inputs only; no Init/PostCreate/publication executed", "maps":results});
    if let Some(output) = std::env::var_os("AA_LIVE_STARTUP_OUT") {
        let path = std::path::PathBuf::from(output);
        assert!(path.is_absolute() && !path.exists());
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    println!("{}", report);
}
