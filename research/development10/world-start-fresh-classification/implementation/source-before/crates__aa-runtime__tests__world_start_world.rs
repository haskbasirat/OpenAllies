//! Native fixture states installed into real ActorStore/KingdomEconomyLoad.
//! These are adapter integration tests, not claims of a playable map startup.
use aa_actors::{
    ActorId, IdentityRegistry,
    components::ComponentSlot,
    ground_states::{LayoutDefinition, OrganizationState},
    lair_city_state::CityState,
    loading::{LoadKind, construct_shell_with_components},
    owned::Component,
};
use aa_body::F32;
use aa_economy::{
    Catalog as ResourceCatalog, F32Bits, ResourceDefinition, ResourceKind,
    statistics::{KingdomStatistics, RelationStatistics, Statistics},
    statistics_live::DerivedCatalog,
    world_start::WorldStart,
};
use aa_mapworld::{
    ThingState,
    bootstrap::DefinitionProof,
    inventory::{ByteSpan, Placement},
    kingdoms::{KingdomPayload, KingdomRecord},
    restore::{
        LoadCompletion, LoadedEntity, RestoredComponent, RestoredRecord, RuntimeDefinition,
        SourceWorldEntity, WorldEntityRuntime,
    },
    special_state::{NativeVectorState, SpecialShell},
};
use aa_numeric::Precision;
use aa_rules::{
    actors::{
        ActorCatalog,
        native_load::{NativeLoadReport, load_files},
    },
    native::CategoryDescriptor,
    resources::Provenance,
    tgi::{Limits, Span},
};
use aa_runtime::{
    actor_world::{ActorStore, Source},
    restoration::{KingdomEconomyLoad, StatisticsSnapshot, StatisticsSnapshotEntry},
    world_start_world::{World, bind_formulas},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn span() -> ByteSpan {
    ByteSpan {
        offset: 0,
        length: 0,
    }
}
fn words(v: &Value) -> Vec<F32Bits> {
    serde_json::from_value(v.clone()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn id(v: u32) -> ActorId {
    ActorId::from_raw(v)
}
fn pc(v: &Value) -> Precision {
    match v.as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        _ => panic!(),
    }
}
fn empty_source() -> ActorCatalog {
    load_files(vec![], Limits::default(), &mut NativeLoadReport::default())
}
fn definition(label: &str, class: u32) -> RuntimeDefinition {
    RuntimeDefinition {
        proof: DefinitionProof {
            id: label.into(),
            source: Provenance {
                path: "synthetic original world-start oracle state".into(),
                sha256: String::new(),
                span: Span { start: 0, end: 0 },
            },
            template_chain: vec![],
            category: CategoryDescriptor {
                name: "fixture",
                registry_index: 0,
                native_kind: 2,
                runtime_class: class,
                view_type: 0,
            },
            base_flag_inputs: vec![],
        },
        native_class: class,
        flags: 0,
        object: None,
        component_mask: None,
        ground: Default::default(),
    }
}
fn thing() -> ThingState {
    ThingState {
        version: 2,
        name_utf16: vec![],
        word_1c: 0,
        position_bits: [0; 3],
        facing_bits: [0; 3],
        serialized_flags: 0,
        legacy_version_one_word: None,
    }
}
#[allow(clippy::too_many_arguments)] // Explicit native fixture fields.
fn kingdom(
    reg: &mut IdentityRegistry<RestoredRecord>,
    kid: ActorId,
    n: usize,
    word190: u32,
    timer: u32,
    provided: Vec<u32>,
    consumed: Vec<u32>,
    count: u32,
) {
    let def = definition("fixture-kingdom", 4);
    let mut base =
        construct_shell_with_components::<_, RestoredComponent>(LoadKind::Actor, def.proof.clone());
    base.actor.as_mut().unwrap().words.insert(0xe8, 0);
    let mut derived = SpecialShell::kingdom(kid, n as u32).unwrap();
    derived.words.insert(0x190, word190);
    derived.words.insert(0x334, timer);
    // Native+218 is the owned RecruitItemTable vector length, not a scalar
    // shadow. Snapshot construction must read these actual entries.
    derived.words.remove(&0x218);
    //This controlled native-field fixture supplies an inactive SAI branch;
    //the actual production SAI lifecycle owner is not constructed here.
    struct FixtureAi;
    impl aa_economy::kingdom_registry::RecruitAi for FixtureAi {
        fn ai_enabled(&mut self)->Result<bool,aa_economy::EconomyError>{Ok(false)}
        fn ai_listeners(&mut self)->Result<Vec<u32>,aa_economy::EconomyError>{unreachable!("inactive fixture")}
        fn notify_ai(&mut self,_:u32,_:&aa_economy::tick::RecruitSingle,_:bool)->Result<(),aa_economy::EconomyError>{unreachable!("inactive fixture")}
    }
    for i in 0..count {
        let definition = 0x500 + i;
        derived
            .recruit_items
            .add_single(aa_economy::tick::RecruitSingle {
                definition,
                available: false,
                actor: 0,
                organization: 0,
                progress: F32Bits::ZERO,
                regeneration_rate: F32Bits::ZERO,
            },&mut FixtureAi).unwrap();
    }
    for (offset, values) in [(0x1a0, provided), (0x1b8, consumed)] {
        derived.vectors.insert(
            offset,
            NativeVectorState {
                capacity_hint: values.len() as u32,
                values,
            },
        );
    }
    let payload:KingdomPayload=serde_json::from_value(json!({"version":0,"word_188":0,"word_18c":0,"word_190":word190,"display_key_utf16":[],"color_reference_utf16":[],"words_1d0":{"serialized_count":0,"values":[]},"resources_1c4":{"serialized_count":0,"values":[]}})).unwrap();
    let source = KingdomRecord {
        source_span: span(),
        definition_id_utf16: vec![],
        serialized_id: kid.raw(),
        thing: thing(),
        object: None,
        actor: None,
        updatable: None,
        payload,
    };
    reg.register_saved(
        kid,
        true,
        RestoredRecord::Kingdom(Box::new(SourceWorldEntity {
            source,
            completion: LoadCompletion::PostLoaded,
            failure: None,
            runtime: Some(WorldEntityRuntime {
                definition: def,
                base,
                derived: Some(derived),
                actor_tail: None,
                derived_payload_loaded: true,
            }),
        })),
    )
    .unwrap();
}
fn actor(
    reg: &mut IdentityRegistry<RestoredRecord>,
    aid: ActorId,
    active: bool,
    field: u32,
    owner: Option<ActorId>,
    organization: bool,
    city: bool,
) {
    let class = if active { 3 } else { 1 };
    let def = definition("fixture-actor", class);
    let mut shell = construct_shell_with_components::<_, RestoredComponent>(
        if active {
            LoadKind::Actor
        } else {
            LoadKind::Thing
        },
        def.proof.clone(),
    );
    if let Some(a) = &mut shell.actor {
        a.words.insert(0x13c, field);
        a.words.insert(0xe8, owner.map_or(0, ActorId::raw));
        if organization {
            a.components.insert(
                ComponentSlot::Organization,
                Component::Organization(Box::new(
                    OrganizationState::new(
                        aid,
                        LayoutDefinition {
                            definition: "fixture-layout".into(),
                            parameters: vec![],
                            slots: 0,
                        },
                        F32::ZERO,
                        F32::ZERO,
                        [F32::ZERO; 2],
                    )
                    .unwrap(),
                )),
            );
        }
        if city {
            a.components.insert(
                ComponentSlot::City,
                Component::City(Box::new(CityState::new(aid))),
            );
        }
    }
    let source = Placement {
        record_ordinal: 0,
        source_span: span(),
        thing_id_utf16: vec![],
        class_code: class,
        serialized_id: aid.raw(),
        thing: thing(),
        object: None,
        actor: None,
        opaque_tail_span: span(),
        opaque_tail: vec![],
    };
    let record = Box::new(LoadedEntity {
        source,
        definition: def,
        shell,
        completion: LoadCompletion::PostLoaded,
        owner_source_slot: None,
        saved_components: vec![],
        actor_tail: None,
        doodad: None,
        special_object: None,
        failure: None,
        active_component: None,
    });
    reg.register_saved(
        aid,
        active,
        if active {
            RestoredRecord::Actor(record)
        } else {
            RestoredRecord::Thing(record)
        },
    )
    .unwrap();
}
fn state(stats: KingdomStatistics) -> KingdomEconomyLoad {
    KingdomEconomyLoad {
        statistics: stats,
        properties: vec![],
        snapshots: vec![],
    }
}
fn loaded_snapshot(n: usize, marker: u32) -> StatisticsSnapshotEntry {
    StatisticsSnapshotEntry::Loaded(StatisticsSnapshot {
        references_relations: true,
        statistics: Statistics::new(n),
        words_10c: [marker; 10],
        word_134: marker,
        words_138: [marker; 10],
    })
}
fn shell(store: &ActorStore, k: ActorId) -> &SpecialShell {
    match &store.entity(store.key(k).unwrap()).unwrap().source {
        Source::Special(s) => s.derived.as_ref().unwrap(),
        _ => panic!(),
    }
}
fn shell_mut(store: &mut ActorStore, k: ActorId) -> &mut SpecialShell {
    let key = store.key(k).unwrap();
    match &mut store.entity_mut(key).unwrap().source {
        Source::Special(s) => s.derived.as_mut().unwrap(),
        _ => panic!(),
    }
}
fn resources(v: &Value) -> ResourceCatalog {
    ResourceCatalog {
        resources: v
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, r)| ResourceDefinition {
                id: format!("r{i}"),
                kind: match u(&r["kind"]) {
                    0 => ResourceKind::Stockpiled,
                    1 => ResourceKind::Upkeep,
                    2 => ResourceKind::LimitedConsumed,
                    _ => panic!(),
                },
                limited_provider: r["limited_provider"].as_bool().unwrap(),
                purchase_cost: vec![],
            })
            .collect(),
    }
}
fn resource_count(n: usize) -> ResourceCatalog {
    ResourceCatalog {
        resources: (0..n)
            .map(|i| ResourceDefinition {
                id: format!("r{i}"),
                kind: ResourceKind::Stockpiled,
                limited_provider: false,
                purchase_cost: vec![],
            })
            .collect(),
    }
}
fn live_fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-world-start/native-runtime-snapshots.json"
    ))
    .unwrap()
}

#[test]
fn actual_kingdom_snapshots_match_all_132_original_v14_rows_and_preserve_saved_entries() {
    let source = empty_source();
    let doc = live_fixture();
    let mut count = 0;
    for f in doc["fixtures"].as_array().unwrap() {
        let c = &f["case"];
        let res = resources(&c["resources"]);
        let n = res.resources.len();
        let kid = id(100);
        let mut reg = IdentityRegistry::new();
        kingdom(
            &mut reg,
            kid,
            n,
            0,
            0,
            words(&c["provided"]).iter().map(|x| x.0).collect(),
            words(&c["consumed"]).iter().map(|x| x.0).collect(),
            c["count"].as_i64().unwrap() as i32 as u32,
        );
        let mut store = ActorStore::from_restored(reg).unwrap();
        let stats = Statistics {
            production: words(&c["production"]),
            scores: words(&c["scores"]),
        };
        let mut owned = state(KingdomStatistics {
            active: Some(stats.clone()),
            loaded: None,
            relations: (!c["relation_identity"].is_null()).then(RelationStatistics::default),
            loaded_relations: None,
        });
        owned.snapshots = vec![loaded_snapshot(n, 11), loaded_snapshot(n, 22)];
        let mut economy = BTreeMap::from([(kid.raw(), owned)]);
        let formulas = DerivedCatalog {
            rows: serde_json::from_value(c["rows"].clone()).unwrap(),
        };
        let mut started = true;
        World {
            actors: &mut store,
            economy: &mut economy,
            started: &mut started,
            major_kingdoms: &[Some(kid)],
            source: &source,
            resources: &res,
            formulas: &formulas,
            source_precision: pc(&f["precision"]),
            precision: pc(&f["precision"]),
        }
        .statistics_snapshot(kid.raw() as usize)
        .unwrap();
        let entry = &economy[&kid.raw()];
        assert_eq!(
            entry.statistics.active.as_ref().unwrap(),
            &stats,
            "snapshot must not mutate authoritative counters"
        );
        // 45F240 appends after the existing tail; saved snapshots retain age order.
        let StatisticsSnapshotEntry::Live(s) = &entry.snapshots[2] else {
            panic!()
        };
        let mut expected = f["expected"].clone();
        expected["relation_identity"] = if c["relation_identity"].is_null() {
            Value::Null
        } else {
            json!(kid.raw())
        };
        assert_eq!(
            json!({"production":s.statistics.production,"scores":s.statistics.scores,"resource_levels":s.resource_levels,"kingdom_count":s.kingdom_count,"derived":s.derived,"relation_identity":s.relation_identity}),
            expected,
            "{}",
            c["name"]
        );
        for (i, marker) in [(0, 11), (1, 22)] {
            let StatisticsSnapshotEntry::Loaded(s) = &entry.snapshots[i] else {
                panic!()
            };
            assert_eq!(s.word_134, marker);
            assert_eq!(s.words_138, [marker; 10]);
        }
        let first = s.statistics.scores[0];
        economy
            .get_mut(&kid.raw())
            .unwrap()
            .statistics
            .active
            .as_mut()
            .unwrap()
            .scores[0] = F32Bits(0x4120_0000);
        let StatisticsSnapshotEntry::Live(s) = &economy[&kid.raw()].snapshots[2] else {
            panic!()
        };
        assert_eq!(s.statistics.scores[0], first);
        count += 1;
    }
    assert_eq!(count, 132);
}

#[test]
fn organization_city_tracking_mutates_authoritative_actor_and_statistics_against_native_rows() {
    let source = empty_source();
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-world-start/native-runtime-tracking.json"
    ))
    .unwrap();
    let resources = resource_count(0);
    let formulas = DerivedCatalog { rows: vec![] };
    let mut count = 0;
    for f in doc["fixtures"].as_array().unwrap() {
        let c = &f["case"];
        let facts = &c["facts"];
        // These branches are representable by the actual owned runtime today.
        if c["op"] != "track"
            || facts["structure_94"] == true
            || facts["construction_9c"] == true
            || facts["component_d8"] == true
            || !facts["component_98_level"].is_null()
        {
            continue;
        }
        let mut reg = IdentityRegistry::new();
        let kid = id(100);
        let aid = id(1);
        kingdom(&mut reg, kid, 0, 0, 0, vec![], vec![], 0);
        actor(
            &mut reg,
            aid,
            true,
            u(&c["field"]),
            Some(kid),
            facts["component_7c"].as_bool().unwrap(),
            facts["component_d4"].as_bool().unwrap(),
        );
        let mut store = ActorStore::from_restored(reg).unwrap();
        let mut statistics = KingdomStatistics::default();
        statistics.active = Some(Statistics {
            production: vec![],
            scores: words(&c["scores"]),
        });
        let mut economy = BTreeMap::from([(kid.raw(), state(statistics))]);
        let mut started = c["started"].as_bool().unwrap();
        let before = economy[&kid.raw()]
            .statistics
            .active
            .as_ref()
            .unwrap()
            .scores
            .as_ptr();
        let mut w = World {
            actors: &mut store,
            economy: &mut economy,
            started: &mut started,
            major_kingdoms: &[Some(kid)],
            source: &source,
            resources: &resources,
            formulas: &formulas,
            source_precision: pc(&f["precision"]),
            precision: pc(&f["precision"]),
        };
        w.statistics_track(kid.raw() as usize, aid.raw()).unwrap();
        assert_eq!(
            json!(w.field_13c(aid.raw()).unwrap()),
            f["expected"]["field"]
        );
        assert_eq!(
            json!(
                economy[&kid.raw()]
                    .statistics
                    .active
                    .as_ref()
                    .unwrap()
                    .scores
            ),
            f["expected"]["scores"]
        );
        assert_eq!(
            before,
            economy[&kid.raw()]
                .statistics
                .active
                .as_ref()
                .unwrap()
                .scores
                .as_ptr(),
            "must mutate the live allocation"
        );
        count += 1;
    }
    assert_eq!(count, 240);
    eprintln!("tracking runtime native rows: {count}");
}

#[test]
fn owned_runtime_world_start_matches_360_native_coordinator_rows() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../aa-economy/tests/fixtures/native-world-start-fixtures.json"
    ))
    .unwrap();
    let source = empty_source();
    let res = resource_count(3);
    let formulas = DerivedCatalog { rows: vec![] };
    let mut count = 0;
    for f in doc["fixtures"].as_array().unwrap() {
        let c = &f["case"];
        let mut reg = IdentityRegistry::new();
        let mut economy = BTreeMap::new();
        let mut major = vec![];
        for (i, k) in c["kingdoms"].as_array().unwrap().iter().enumerate() {
            if k.is_null() {
                major.push(None);
                continue;
            }
            let kid = id(100 + i as u32);
            major.push(Some(kid));
            kingdom(
                &mut reg,
                kid,
                3,
                u(&k["word_190"]),
                u(&k["timer"]),
                vec![0; 3],
                vec![0; 3],
                0,
            );
            let stat = |v: &Value| {
                (!v.is_null()).then(|| Statistics {
                    production: vec![F32Bits::ZERO; 3],
                    scores: words(v),
                })
            };
            let mut owned = state(KingdomStatistics {
                active: stat(&k["active"]),
                loaded: stat(&k["loaded"]),
                relations: (!k["relations"].is_null()).then(RelationStatistics::default),
                loaded_relations: (!k["loaded_relations"].is_null())
                    .then(RelationStatistics::default),
            });
            if k["has_snapshots"] == true {
                owned.snapshots.push(loaded_snapshot(3, 999));
            }
            economy.insert(kid.raw(), owned);
        }
        for (j, o) in c["objects"].as_array().unwrap().iter().enumerate() {
            if o.is_null() {
                continue;
            }
            actor(
                &mut reg,
                id(j as u32 + 1),
                o["active"].as_bool().unwrap(),
                u(&o["field_13c"]),
                o["owner"].as_u64().map(|k| id(100 + k as u32)),
                false,
                false,
            );
        }
        let mut store = ActorStore::from_restored(reg).unwrap();
        let mut started = c["started"].as_bool().unwrap();
        {
            let mut w = World {
                actors: &mut store,
                economy: &mut economy,
                started: &mut started,
                major_kingdoms: &major,
                source: &source,
                resources: &res,
                formulas: &formulas,
                source_precision: pc(&f["precision"]),
            precision: pc(&f["precision"]),
            };
            w.start().unwrap();
            for (j, o) in c["objects"].as_array().unwrap().iter().enumerate() {
                if o["active"] == true {
                    assert_eq!(
                        json!(w.field_13c(j as u32 + 1).unwrap()),
                        f["expected"]["fields_13c"][j]
                    );
                }
            }
            w.start().unwrap();
        }
        assert_eq!(json!(started), f["expected"]["started"]);
        for (i, k) in major.iter().enumerate() {
            let Some(k) = k else { continue };
            let e = &f["expected"]["kingdoms"][i];
            assert_eq!(json!(shell(&store, *k).words[&0x334]), e["timer"]);
            assert_eq!(
                json!(!economy[&k.raw()].snapshots.is_empty()),
                e["has_snapshots"]
            );
            if c["started"] == false {
                assert_eq!(
                    json!(economy[&k.raw()].statistics.active.as_ref().unwrap().scores),
                    e["active_scores"]
                );
                assert!(economy[&k.raw()].statistics.loaded.is_none());
                assert!(economy[&k.raw()].statistics.loaded_relations.is_none());
            }
            if c["kingdoms"][i]["has_snapshots"] == true {
                assert!(
                    matches!(&economy[&k.raw()].snapshots[..],[StatisticsSnapshotEntry::Loaded(s)]if s.word_134==999)
                );
            }
        }
        count += 1;
    }
    assert_eq!(count, 360);
}

#[test]
fn missing_owner_fields_stop_at_native_order_and_keep_started_guard() {
    let source = empty_source();
    let resources = resource_count(1);
    let formulas = DerivedCatalog { rows: vec![] };
    let kid = id(100);
    let mut reg = IdentityRegistry::new();
    kingdom(&mut reg, kid, 1, 0, 0x4140_0000, vec![0], vec![0], 0);
    for i in [3, 1, 2] {
        actor(&mut reg, id(i), true, 7, Some(kid), true, false);
    }
    let mut store = ActorStore::from_restored(reg).unwrap();
    let key = store.key(id(2)).unwrap();
    store
        .object_mut(key)
        .unwrap()
        .actor
        .as_mut()
        .unwrap()
        .words
        .remove(&0x13c);
    let mut economy = BTreeMap::from([(kid.raw(), state(KingdomStatistics::default()))]);
    let mut started = false;
    {
        let mut w = World {
            actors: &mut store,
            economy: &mut economy,
            started: &mut started,
            major_kingdoms: &[Some(kid)],
            source: &source,
            resources: &resources,
            formulas: &formulas,
            source_precision: Precision::Double53,
        precision: Precision::Double53,
        };
        assert!(w.start().unwrap_err().to_string().contains("13C"));
        assert_eq!(w.field_13c(1).unwrap(), 47);
        assert_eq!(w.field_13c(3).unwrap(), 7);
        w.start().unwrap();
    }
    assert!(started);
    assert_eq!(
        economy[&kid.raw()]
            .statistics
            .active
            .as_ref()
            .unwrap()
            .scores[47],
        F32Bits::from_f32(1.)
    );
    assert!(economy[&kid.raw()].snapshots.is_empty());
    assert_eq!(shell(&store, kid).words[&0x334], 0x4140_0000);
    // A separate explicit snapshot error does not publish a placeholder.
    shell_mut(&mut store, kid).vectors.remove(&0x1a0);
    let mut w = World {
        actors: &mut store,
        economy: &mut economy,
        started: &mut started,
        major_kingdoms: &[Some(kid)],
        source: &source,
        resources: &resources,
        formulas: &formulas,
        source_precision: Precision::Double53,
        precision: Precision::Double53,
    };
    assert!(w.statistics_snapshot(kid.raw() as usize).is_err());
    assert!(economy[&kid.raw()].snapshots.is_empty());
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn installed_source_four_formulas_bind_only_matching_resource_registry() {
    use aa_rules::{
        resources::ResourceCatalog as SourceResources,
        sources::{SourceLimits, V14Sources},
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
    let source_resources = SourceResources::load_v14(&mut sources).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let defs = aa_runtime::kingdom_economy::Definitions::bind(
        &source_resources,
        &actors,
        Precision::Double53,
    )
    .unwrap();
    let bound = bind_formulas(&actors, defs.catalog(), Precision::Double53).unwrap();
    assert_eq!(bound.rows.len(), 4);
    let native = actors
        .native_statistics_catalog(Precision::Double53)
        .unwrap();
    assert_eq!(native.definitions.len(), 80);
    for (row, original) in bound.rows.iter().zip(&native.derived) {
        assert_eq!(
            row.iter()
                .map(|t| (t.index, t.weight.0))
                .collect::<Vec<_>>(),
            original
                .terms
                .iter()
                .map(|t| (t.stat_index as usize, t.weight_bits))
                .collect::<Vec<_>>()
        );
    }
    let mut wrong = defs.catalog().clone();
    wrong.resources.swap(0, 1);
    assert!(bind_formulas(&actors, &wrong, Precision::Double53).is_err());
    wrong = defs.catalog().clone();
    wrong.resources[0].id.push_str("-different");
    assert!(bind_formulas(&actors, &wrong, Precision::Double53).is_err());
}

#[test]
fn explicit_major_kingdom_order_precedes_actor_scan_and_keeps_partial_promotion() {
    let source = empty_source();
    let resources = resource_count(0);
    let formulas = DerivedCatalog { rows: vec![] };
    let mut registry = IdentityRegistry::new();
    let mut economy = BTreeMap::new();
    for raw in [100, 101] {
        kingdom(&mut registry, id(raw), 0, 0, 0, vec![], vec![], 0);
        let mut loaded = Statistics::new(0);
        loaded.scores[0] = F32Bits::from_f32(raw as f32);
        economy.insert(
            raw,
            state(KingdomStatistics {
                loaded: Some(loaded),
                ..Default::default()
            }),
        );
    }
    let mut store = ActorStore::from_restored(registry).unwrap();
    let mut started = false;
    // The stale middle identity must stop the explicit world+154 order. This
    // host error is not a claim of emulating a native invalid-pointer crash.
    let major = [Some(id(101)), None, Some(id(999)), Some(id(100))];
    let mut world = World {
        actors: &mut store,
        economy: &mut economy,
        started: &mut started,
        major_kingdoms: &major,
        source: &source,
        resources: &resources,
        formulas: &formulas,
        source_precision: Precision::Double53,
        precision: Precision::Double53,
    };
    assert!(world.start().unwrap_err().to_string().contains("not live"));
    assert!(started);
    assert!(economy[&101].statistics.loaded.is_none());
    assert_eq!(
        economy[&101].statistics.active.as_ref().unwrap().scores[0],
        F32Bits::from_f32(101.)
    );
    assert!(economy[&100].statistics.active.is_none());
    assert!(economy[&100].statistics.loaded.is_some());
}
