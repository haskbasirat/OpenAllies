use aa_numeric::Precision;
use aa_rules::{
    actors::{
        ActorCatalog,
        native_load::{NativeLoadReport, load_files},
    },
    sources::{Origin, ResolvedFile, sha256},
    tgi::Limits,
};
use aa_runtime::{
    definitions::Bank,
    skirmish_start::{
        Definitions, Substitution, SubstitutionEntry, WeightedTable, require_replacement_drain,
        substitution,
    },
};
use aa_simulation::NativeRng;
use serde_json::{Value, json};
fn pc(v: &Value) -> Precision {
    serde_json::from_value(v.clone()).unwrap()
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-start-bindings/native-selection.json"
    ))
    .unwrap()
}
#[test]
fn weighted_table_and_shared_rng_match_195_original_v14_cases() {
    let d = fixture();
    let mut n = 0;
    for row in d["weighted"].as_array().unwrap() {
        let c = &row["case"];
        let pc = pc(&row["precision"]);
        let mut table = WeightedTable::default();
        for e in c["input"].as_array().unwrap() {
            table
                .add(
                    e["definition"].as_u64().map(|x| x as u32),
                    e["weight_bits"].as_u64().unwrap() as u32,
                    pc,
                )
                .unwrap();
        }
        assert_eq!(json!(table.entries), row["expected"]["entries"]);
        assert_eq!(json!(table.total_bits), row["expected"]["total_bits"]);
        let mut rng = NativeRng::new(c["seed"].as_u64().unwrap() as u32);
        for expected in row["expected"]["draws"].as_array().unwrap() {
            let selected = table.choose(&mut rng, pc).unwrap();
            assert_eq!(json!({"selected":selected,"state":rng.state}), *expected);
        }
        n += 1;
    }
    assert_eq!(n, 195);
}
#[test]
fn substitution_preserves_native_first_identity_match_and_null_vs_missing() {
    for row in fixture()["substitution"].as_array().unwrap() {
        let entries: Vec<SubstitutionEntry> =
            serde_json::from_value(row["case"]["entries"].clone()).unwrap();
        let value = substitution(&entries, row["case"]["key"].as_u64().unwrap() as u32);
        assert_eq!(
            matches!(value, Substitution::Missing),
            row["expected"]["missing_logged"].as_bool().unwrap()
        );
        let selected = match value {
            Substitution::Missing => None,
            Substitution::Present(v) => v,
        };
        assert_eq!(json!(selected), row["expected"]["replacement"]);
    }
}
fn file(path: &str, text: &str) -> ResolvedFile {
    let bytes: Vec<u8> = text
        .chars()
        .map(|c| u8::try_from(c as u32).unwrap_or(b'?'))
        .collect();
    ResolvedFile {
        logical_path: path.into(),
        origin: Origin::V14Loose,
        sha256: sha256(&bytes),
        bytes,
    }
}
#[test]
fn nation_source_matches_actual_parser_prefix_and_deferred_target_fixup() {
    let d: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-start-bindings/native-nation-pairs.json"
    ))
    .unwrap();
    let thing = "[Template Thing]\n{ string IDS\n string category = ACTOR\n}\n";
    let definitions = [
        "placeholder_corps",
        "placeholder_bunker",
        "gbr_corps",
        "gbr_bunker",
        "other",
    ]
    .iter()
    .map(|id| format!("[Thing template=Thing]\n{{ IDS={id}\n}}\n"))
    .collect::<String>();
    for row in d["cases"].as_array().unwrap() {
        let mut report = NativeLoadReport::default();
        let catalog = load_files(
            vec![
                file("thing-template.tgi", thing),
                file("nation-template.tgi", d["template"].as_str().unwrap()),
                file("things.tgi", &definitions),
                file("nation.tgi", row["source"].as_str().unwrap()),
            ],
            Limits::default(),
            &mut report,
        );
        let mut bank = Bank::default();
        let result = Definitions::bind(&catalog, &mut bank, Precision::Double53);
        if !row["thrown"].is_null() {
            assert!(result.is_err());
            continue;
        }
        let result = result.unwrap();
        let actual=result.nations["fixture"].iter().map(|r|json!({"placeholder":result.things[&r.placeholder].ids,"replacement":r.replacement.map(|id|result.things[&id].ids.clone())})).collect::<Vec<_>>();
        assert_eq!(json!(actual), row["native_rows"]);
    }
}
#[test]
fn unsupported_publication_and_invalid_numeric_inputs_never_report_prepare_success() {
    assert!(require_replacement_drain().is_err());
    let mut table = WeightedTable::default();
    assert!(
        table
            .add(Some(1), 0x7fc0_0000, Precision::Double53)
            .is_err()
    );
    assert!(table.entries.is_empty());
    table
        .add(Some(1), 1f32.to_bits(), Precision::Double53)
        .unwrap();
    table.total_bits = 0x7f80_0000;
    let mut rng = NativeRng::new(1);
    let old = rng.state;
    assert!(table.choose(&mut rng, Precision::Double53).is_err());
    assert_ne!(old, rng.state);
    let mut null = WeightedTable::default();
    null.add(Some(0), 1f32.to_bits(), Precision::Double53)
        .unwrap();
    assert_eq!(null.choose(&mut rng, Precision::Double53).unwrap(), None);
}

#[test]
fn random_source_matches_original_parser_contribution_windows_and_rejects_unproven_timing() {
    let d: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-start-bindings/native-random-source.json"
    ))
    .unwrap();
    let targets = "[Thing template=Thing]\n{ IDS=random_object\ncategory=RANDOM_OBJECT\n}\n[Thing template=Thing]\n{ IDS=random_actor\ncategory=RANDOM_ACTOR\n}\n";
    let translate_template =
        "[Template Translate]\n{ string node\n string ids_old\n string ids_new\n}\n";
    for row in d["cases"].as_array().unwrap() {
        let targets = if row["wrong"] == true {
            targets.replace("RANDOM_OBJECT", "OBJECT")
        } else {
            targets.into()
        };
        let mut files = vec![file("template.tgi", d["template"].as_str().unwrap())];
        if row["missing"] != true {
            files.push(file("targets.tgi", &targets));
        }
        files.push(file("contributors.tgi", row["source"].as_str().unwrap()));
        let source = load_files(files, Limits::default(), &mut NativeLoadReport::default());
        let mut bank = Bank::default();
        let result = Definitions::bind(&source, &mut bank, Precision::Double53);
        if !row["thrown"].is_null() {
            assert!(result.is_err());
            continue;
        }
        let data = result.unwrap();
        let mut tables = serde_json::Map::new();
        for name in ["random_object", "random_actor"] {
            let token = bank.intern("ActorDefinition", name).unwrap();
            let table = &data.random[&token];
            let entries=table.entries.iter().map(|e|json!({"definition":match data.things[&e.definition.unwrap()].ids.as_str(){"one"=>"contributor0","two"=>"contributor1",other=>panic!("{other}")},"weight_bits":e.weight_bits})).collect::<Vec<_>>();
            tables.insert(
                name.into(),
                json!({"entries":entries,"total_bits":table.total_bits}),
            );
        }
        assert_eq!(Value::Object(tables), row["tables"]);
    }
    let source_text = d["cases"][2]["source"].as_str().unwrap();
    let source = load_files(
        vec![
            file("template.tgi", d["template"].as_str().unwrap()),
            file("contributors.tgi", source_text),
            file("targets.tgi", targets),
        ],
        Limits::default(),
        &mut NativeLoadReport::default(),
    );
    assert!(
        Definitions::bind(&source, &mut Bank::default(), Precision::Double53)
            .unwrap_err()
            .contains("not registered")
    );
    let alias = "[Translate template=Translate]\n{ node=Unit\nids_old=random_actor\nids_new=random_actor\n}\n";
    let source = load_files(
        vec![
            file("template.tgi", d["template"].as_str().unwrap()),
            file("translate-template.tgi", translate_template),
            file("targets.tgi", targets),
            file("contributors.tgi", source_text),
            file("alias.tgi", alias),
        ],
        Limits::default(),
        &mut NativeLoadReport::default(),
    );
    assert!(
        Definitions::bind(&source, &mut Bank::default(), Precision::Double53)
            .unwrap_err()
            .contains("translation-at-load-time")
    );
    let alias = "[Translate template=Translate]\n{ node=Thing\nids_old=one\nids_new=two\n}\n";
    let source = load_files(
        vec![
            file("template.tgi", d["template"].as_str().unwrap()),
            file("translate-template.tgi", translate_template),
            file("targets.tgi", targets),
            file("contributors.tgi", source_text),
            file("alias.tgi", alias),
        ],
        Limits::default(),
        &mut NativeLoadReport::default(),
    );
    assert_eq!(source.find("one").unwrap().id, "two");
    let mut bank = Bank::default();
    let data = Definitions::bind(&source, &mut bank, Precision::Double53).unwrap();
    let token = bank.intern("ActorDefinition", "random_actor").unwrap();
    assert_eq!(
        data.random[&token]
            .entries
            .iter()
            .map(|e| data.things[&e.definition.unwrap()].ids.as_str())
            .collect::<Vec<_>>(),
        ["two", "one"]
    );
}
#[test]
#[ignore = "requires AA_GAME_ROOT source assets"]
fn installed_nations_and_random_tables_bind_in_native_registration_order() {
    use aa_rules::sources::{SourceLimits, V14Sources};
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let source = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut bank = Bank::default();
    let data = Definitions::bind(&source, &mut bank, Precision::Double53).unwrap();
    assert_eq!(data.nations.len(), 5);
    for nation in ["gbr", "ger", "jpn", "rus", "usa"] {
        let key = bank
            .intern("ActorDefinition", "placeholder_corpsunpacked")
            .unwrap();
        let Substitution::Present(Some(target)) =
            substitution(&data.nations[&format!("nation_{nation}")], key)
        else {
            panic!()
        };
        assert_eq!(data.things[&target].ids, format!("{nation}_corps"));
    }
    assert!(!data.random.is_empty());
    let active = data
        .random
        .values()
        .filter(|t| !t.entries.is_empty())
        .count();
    assert!(active > 0);
    eprintln!(
        "source: {} Things,{} Nations,{} random tables({active} populated)",
        data.things.len(),
        data.nations.len(),
        data.random.len()
    );
    // Actual owned runtime records with explicit fixture owner/Nation. This
    // checks lookup binding, not player assignment or a real-map mission.
    use aa_actors::{
        ActorId, IdentityRegistry,
        loading::{LoadKind, construct_shell_with_components},
    };
    use aa_mapworld::{
        ThingState,
        bootstrap::DefinitionProof,
        inventory::{ByteSpan, Placement},
        kingdoms::{KingdomPayload, KingdomRecord},
        restore::{
            LoadCompletion, LoadedEntity, NamedDefinition, RestoredComponent, RestoredRecord,
            RuntimeDefinition, SourceWorldEntity, WorldEntityRuntime,
        },
        special_state::SpecialShell,
    };
    use aa_runtime::actor_world::{ActorStore, Source};
    let aid = ActorId::from_raw(1);
    let kid = ActorId::from_raw(100);
    let mut registry = IdentityRegistry::new();
    let span = || ByteSpan {
        offset: 0,
        length: 0,
    };
    let base = || ThingState {
        version: 2,
        name_utf16: vec![],
        word_1c: 0,
        position_bits: [0; 3],
        facing_bits: [0; 3],
        serialized_flags: 0,
        legacy_version_one_word: None,
    };
    let def = |name: &str, class| {
        let found = source.find(name).unwrap();
        RuntimeDefinition {
            proof: DefinitionProof {
                id: found.id.clone(),
                source: found.source.clone(),
                template_chain: vec![],
                category: source.native_category(name).unwrap(),
                base_flag_inputs: vec![],
            },
            native_class: class,
            flags: 0,
            object: None,
            component_mask: None,
            ground: Default::default(),
        }
    };
    let actor_definition = def("placeholder_corpsunpacked", 3);
    let mut actor_shell = construct_shell_with_components::<_, RestoredComponent>(
        LoadKind::Actor,
        actor_definition.proof.clone(),
    );
    actor_shell
        .actor
        .as_mut()
        .unwrap()
        .words
        .insert(0xe8, kid.raw());
    registry
        .register_saved(
            aid,
            true,
            RestoredRecord::Actor(Box::new(LoadedEntity {
                source: Placement {
                    record_ordinal: 0,
                    source_span: span(),
                    thing_id_utf16: vec![],
                    class_code: 3,
                    serialized_id: aid.raw(),
                    thing: base(),
                    object: None,
                    actor: None,
                    opaque_tail_span: span(),
                    opaque_tail: vec![],
                },
                definition: actor_definition,
                shell: actor_shell,
                completion: LoadCompletion::PostLoaded,
                owner_source_slot: None,
                saved_components: vec![],
                actor_tail: None,
                doodad: None,
                special_object: None,
                failure: None,
                active_component: None,
            })),
        )
        .unwrap();
    let kingdom_definition = def("placeholder_corpsunpacked", 4);
    let kingdom_shell = construct_shell_with_components::<_, RestoredComponent>(
        LoadKind::Actor,
        kingdom_definition.proof.clone(),
    );
    let mut derived = SpecialShell::kingdom(kid, 0).unwrap();
    derived.references.insert(
        0x234,
        Some(NamedDefinition {
            id: "nation_gbr".into(),
            index: None,
        }),
    );
    let payload:KingdomPayload=serde_json::from_value(json!({"version":0,"word_188":0,"word_18c":0,"word_190":0,"display_key_utf16":[],"color_reference_utf16":[],"words_1d0":{"serialized_count":0,"values":[]},"resources_1c4":{"serialized_count":0,"values":[]}})).unwrap();
    registry
        .register_saved(
            kid,
            true,
            RestoredRecord::Kingdom(Box::new(SourceWorldEntity {
                source: KingdomRecord {
                    source_span: span(),
                    definition_id_utf16: vec![],
                    serialized_id: kid.raw(),
                    thing: base(),
                    object: None,
                    actor: None,
                    updatable: None,
                    payload,
                },
                completion: LoadCompletion::PostLoaded,
                failure: None,
                runtime: Some(WorldEntityRuntime {
                    definition: kingdom_definition,
                    base: kingdom_shell,
                    derived: Some(derived),
                    actor_tail: None,
                    derived_payload_loaded: true,
                }),
            })),
        )
        .unwrap();
    let mut actors = ActorStore::from_restored(registry).unwrap();
    let placeholder = bank
        .intern("ActorDefinition", "placeholder_corpsunpacked")
        .unwrap();
    let Substitution::Present(Some(target)) = data
        .placeholder_for_actor(&actors, aid, placeholder)
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(data.things[&target].ids, "gbr_corps");
    let wrong = bank
        .intern("ActorDefinition", "placeholder_bunker")
        .unwrap();
    assert!(data.placeholder_for_actor(&actors, aid, wrong).is_err());
    let k = actors.key(kid).unwrap();
    let Source::Special(s) = &mut actors.entity_mut(k).unwrap().source else {
        panic!()
    };
    s.derived.as_mut().unwrap().references.insert(0x234, None);
    assert!(
        data.placeholder_for_actor(&actors, aid, placeholder)
            .unwrap_err()
            .contains("234")
    );
}
