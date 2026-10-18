//! Actual shared held Economy, same initialized Kingdom and real Session.
use aa_actors::{ActorId, components::ComponentSlot, economy_values::ActorSnapshot};
use aa_mapworld::restore::RestoredComponent;
use aa_mapworld::restore::RestoredRecord;
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use aa_runtime::{
    catalog::Catalog,
    live_object_init::Record,
    load_economy_refresh_world::refresh,
    localization::Localization,
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};

fn economy<'a>(r: &'a Record<'_>) -> &'a aa_actors::ground_states::EconomyState {
    match r
        .shell
        .actor
        .as_ref()
        .unwrap()
        .components
        .get(ComponentSlot::Economy)
        .unwrap()
    {
        RestoredComponent::Economy(v) => v,
        _ => panic!("actual Economy"),
    }
}
fn component_snapshot(c: &aa_actors::ground_states::EconomyState) -> Value {
    json!({"flags":[u8::from(c.flag_10),u8::from(c.flag_11)],"resources":c.resources.iter().map(|v|&v.values).collect::<Vec<_>>()})
}
fn kingdom_snapshot(r: &Record<'_>, id: ActorId) -> Value {
    let RestoredRecord::Kingdom(k) = &r.registry.get(id).unwrap().value else {
        panic!("actual Kingdom")
    };
    let d = k.runtime.as_ref().unwrap().derived.as_ref().unwrap();
    json!({"raw_provided":d.vectors[&0x1ac].values,"effective_provided":d.vectors[&0x1a0].values,"consumed":d.vectors[&0x1b8].values,"stockpiled":d.vectors[&0x1c4].values})
}
// Read-only serializer of actual source/record fields for the native oracle.
// It implements no runtime query callbacks and owns no replacement world.
fn graph(catalog: &Catalog<'_>, r: &Record<'_>, ids: &[ActorId], pc: Precision) -> Value {
    let dense: BTreeMap<_, _> = ids.iter().enumerate().map(|(i, id)| (*id, i + 1)).collect();
    let source_pc = Precision::Double53;
    let mut defs = Vec::<String>::new();
    let mut actors = BTreeMap::<String, Value>::new();
    let mut orgs = BTreeMap::<String, Value>::new();
    let mut modifiers = BTreeMap::<String, Value>::new();
    let mut actor_modifiers = BTreeMap::<String, Value>::new();
    let mut transports = BTreeMap::<String, Value>::new();
    let mut caches = BTreeMap::<String, Value>::new();
    for id in ids {
        let (shell, definition) = if *id == r.owner {
            (&*r.shell, r.definition)
        } else {
            let a = r.registry.get(*id).unwrap().value.loaded().unwrap();
            (&a.shell, &a.definition)
        };
        let a = shell.actor.as_ref().unwrap();
        let key = dense[id].to_string();
        let name = definition.proof.id.clone();
        if !defs.contains(&name) {
            defs.push(name.clone());
        }
        let mut parent = None;
        let mut org = None;
        let mut eco = None;
        let mut supply = None;
        let mut transport = None;
        for slot in [
            ComponentSlot::Element,
            ComponentSlot::Organization,
            ComponentSlot::Economy,
            ComponentSlot::Supply,
            ComponentSlot::Transport,
            ComponentSlot::Properties,
        ] {
            match a.components.get(slot) {
                Some(RestoredComponent::Element(v)) => {
                    assert_eq!(v.header.owner, *id);
                    parent = v.organization;
                }
                Some(RestoredComponent::Organization(v)) => {
                    assert_eq!(v.header.owner, *id);
                    org = Some(id.raw());
                    let data = catalog.layout_data(source_pc).unwrap();
                    let layout = data
                        .layouts
                        .iter()
                        .find(|d| d.ids.eq_ignore_ascii_case(&v.layout.definition))
                        .unwrap();
                    assert!(layout.error.is_none());
                    let slots = layout
                        .slot_groups
                        .iter()
                        .map(|g| {
                            let d = v.layout.parameters[*g as usize].clone();
                            if let Some(s) = &d
                                && !defs.contains(s)
                            {
                                defs.push(s.clone());
                            }
                            d
                        })
                        .collect::<Vec<_>>();
                    let members = v
                        .members
                        .iter()
                        .map(|m| (!m.is_null()).then(|| dense[m]))
                        .collect::<Vec<_>>();
                    orgs.insert(key.clone(), json!({"slots":slots,"members":members}));
                }
                Some(RestoredComponent::Economy(v)) => {
                    assert_eq!(v.header.owner, *id);
                    eco = Some(id.raw());
                    caches.insert(key.clone(), json!(v.resources[3].values));
                }
                Some(RestoredComponent::Supply(v)) => {
                    assert_eq!(v.header.owner, *id);
                    supply = Some(v.stockpile);
                }
                Some(RestoredComponent::Transport(v)) => {
                    assert_eq!(v.owner, *id);
                    transport = Some(id.raw());
                    transports.insert(
                        key.clone(),
                        json!({"Cost":v.cost.values,"Upkeep":v.upkeep.values}),
                    );
                }
                Some(RestoredComponent::Properties(v)) => {
                    assert_eq!(v.owner.0, id.raw());
                    actor_modifiers.insert(key.clone(), json!(dense[id]));
                    modifiers.insert(key.clone(),json!({"flags":v.modifier.flags,"factor4c":v.modifier.scalar(0x4c).unwrap(),"factor60":v.modifier.scalar(0x60).unwrap(),"production_adjustments":v.modifier.production}));
                }
                None => {}
                _ => panic!("actual component type"),
            }
        }
        if !actor_modifiers.contains_key(&key) {
            actor_modifiers.insert(key.clone(), Value::Null);
        }
        let snapshot = ActorSnapshot {
            definition: name,
            flags: shell.thing.flags,
            state_bits: u64::from(a.components.state.low)
                | (u64::from(a.components.state.high) << 32),
            element_organization: parent,
            organization: org,
            economy: eco,
            constructor18: supply,
            transport,
        };
        let mut v = json!(snapshot);
        v["element_organization"] = json!(parent.map(|p| dense[&p]));
        v["constructor18"] = json!(supply);
        for field in ["organization", "economy", "transport"] {
            v[field] = json!((!v[field].is_null()).then_some(dense[id]));
        }
        actors.insert(key, v);
    }
    let mut definitions = BTreeMap::<String, Value>::new();
    let mut i = 0;
    while i < defs.len() {
        let id = &defs[i];
        let b = catalog.actor_bindings(id, source_pc).unwrap();
        let source = catalog.actors.native_economy_inputs(id, source_pc).unwrap();
        let mask = b.definition.component_mask.unwrap();
        let layout = if let Some(o) = &b.definition.ground.organization {
            Some(
                catalog
                    .layout_data(source_pc)
                    .unwrap()
                    .layouts
                    .iter()
                    .find(|d| d.ids.eq_ignore_ascii_case(&o.layout.definition))
                    .unwrap()
                    .primary_slot,
            )
        } else {
            None
        };
        let (supply, cost, upkeep, production) = if let Some(v) = &source.resources {
            (
                v.supply_required.boolean().unwrap(),
                v.cost_bits.clone(),
                v.upkeep_bits.clone(),
                v.production_bits.clone(),
            )
        } else {
            (false, vec![], vec![], vec![])
        };
        let packed = catalog
            .native_mobile_deployment_data(source_pc)
            .unwrap()
            .definition(id)
            .unwrap()
            .pack_to
            .clone();
        let backref = catalog
            .native_recruit_backrefs(source_pc)
            .unwrap()
            .present(id)
            .unwrap();
        if let Some(p) = &packed
            && !defs.contains(p)
        {
            defs.push(p.clone());
        }
        definitions.insert((i+1).to_string(),json!({"flags174":mask,"class178":source.recruitable_word,"word17c":u32::from(backref),"layout_kind18":layout,"supply_required":supply,"packed_definition":packed.as_ref().map(|p|defs.iter().position(|s|s==p).unwrap()+1),"cost":cost,"upkeep":upkeep,"production":production}));
        i += 1;
    }
    for a in actors.values_mut() {
        a["definition"] = json!(
            defs.iter()
                .position(|d| d == a["definition"].as_str().unwrap())
                .unwrap()
                + 1
        );
    }
    for o in orgs.values_mut() {
        o["slots"] = json!(
            o["slots"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v
                    .as_str()
                    .map(|s| defs.iter().position(|d| d == s).unwrap() + 1))
                .collect::<Vec<_>>()
        );
    }
    json!({"name":"actual_immediate_held_economy","precision":pc,"actor":1,"definition":1,"context":null,"resource_count":catalog.actors.native_resource_registry(source_pc).unwrap().ids.len(),"actors":actors,"definitions":definitions,"organizations":orgs,"modifiers":modifiers,"actor_modifiers":actor_modifiers,"transports":transports,"caches":caches,"initial_output":[],"cache_members":true,"operation":"Cost","source_actor_ids":ids.iter().map(|i|i.raw()).collect::<Vec<_>>(),"source_definition_ids":defs})
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn actual_immediate_held_economy_and_stored_kingdom_flows_all_precisions() {
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
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut rows = Vec::new();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
            .unwrap()
            .begin(TaskKind::Normal)
            .unwrap();
        let session = aa_netcode::lobby::Session::default();
        let mut attempt = Attempt::load_immediate(
            &map,
            &actors,
            settings.clone(),
            text.clone(),
            Precision::Double53,
            u64::MAX,
            aa_mapworld::restore::RestoreMode::StopOnError,
            LoadBindings {
                native_load_mode: Some(1),
                inactive_load: Some(inactive),
                resources: Some(Arc::clone(&resources)),
            },
            &mut sources,
            &session,
            false,
        )
        .unwrap();
        let mut catalog = Catalog::new(&actors);
        catalog.settings = Some(attempt.settings.clone());
        catalog.native_load_mode = attempt.native_load_mode;
        catalog
            .bind_resource_catalog(Arc::clone(&resources))
            .unwrap();
        catalog.definition_bank = std::cell::RefCell::new(std::mem::take(&mut attempt.definitions));
        let count = actors
            .native_resource_registry(Precision::Double53)
            .unwrap()
            .ids
            .len();
        let id = attempt
            .registry
            .iter()
            .find_map(|e| {
                let RestoredRecord::Actor(a) = &e.value else {
                    return None;
                };
                let actor = a.shell.actor.as_ref()?;
                if !actor.components.present(ComponentSlot::Economy) {
                    return None;
                }
                if let Some(RestoredComponent::Element(v)) =
                    actor.components.get(ComponentSlot::Element)
                    && v.organization.is_some()
                {
                    return None;
                }
                let owner = ActorId::from_raw(*actor.words.get(&0xe8)?);
                let RestoredRecord::Kingdom(k) = &attempt.registry.get(owner)?.value else {
                    return None;
                };
                (k.runtime
                    .as_ref()?
                    .derived
                    .as_ref()?
                    .vectors
                    .get(&0x1b8)?
                    .values
                    .len()
                    == count)
                    .then_some(e.id)
            })
            .unwrap_or_else(|| {
                panic!(
                    "actual immediate stage has no held Economy/initialized Kingdom: {}",
                    serde_json::to_string(&attempt.report).unwrap()
                )
            });
        let mut held = std::mem::replace(
            attempt.registry.value_mut(id).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Actor(a) = &mut held else {
            unreachable!()
        };
        let mut record = Record {
            owner: id,
            shell: &mut a.shell,
            definition: &a.definition,
            registry: &mut attempt.registry,
        };
        let owner = ActorId::from_raw(record.shell.actor.as_ref().unwrap().words[&0xe8]);
        let before = component_snapshot(economy(&record));
        let before_kingdom = kingdom_snapshot(&record, owner);
        let pointer = economy(&record) as *const _ as usize;
        let mut ids = vec![id];
        ids.extend(
            record
                .registry
                .iter()
                .filter_map(|e| matches!(e.value, RestoredRecord::Actor(_)).then_some(e.id)),
        );
        let graph = graph(&catalog, &record, &ids, pc);
        refresh(&catalog, &attempt.inventory, &mut record, pc).unwrap();
        assert_eq!(economy(&record) as *const _ as usize, pointer);
        let after = component_snapshot(economy(&record));
        let after_kingdom = kingdom_snapshot(&record, owner);
        assert_eq!(before_kingdom["stockpiled"], after_kingdom["stockpiled"]);
        let blocked = !economy(&record).flag_11;
        let provides = economy(&record).flag_10;
        let reason = if !blocked {
            "none"
        } else if record.shell.thing.flags & 1 != 0 {
            "destructed"
        } else {
            "nonpaying_organization"
        };
        let definitions = catalog
            .native_resource_economy(Precision::Double53)
            .unwrap();
        rows.push(json!({"precision":pc,"actor":id.raw(),"owner":owner.raw(),"definition":record.definition.proof,"source_pc":"Double53","initial_stage_report":attempt.report,"graph":graph,"input":{"name":format!("actual_held_{}_{}",id.raw(),pc.significand_bits()),"precision":pc,"catalog":definitions.catalog(),"kingdoms":[before_kingdom,before_kingdom],"limits":{"provided_maximum":aa_runtime::saved_kingdom_flows::provider_maximum(&catalog,pc).unwrap(),"stockpiled_maximum":resources.stockpiled_resource_max.f32_bits},"flags":before["flags"],"resources":before["resources"],"block_reason":reason,"parent":false,"owners":[0],"provides":[provides],"upkeep":if blocked{json!([])}else{after["resources"][1].clone()},"provided":if provides{after["resources"][0].clone()}else{json!([])},"cost":after["resources"][2],"mutations":[]},"output":{"component":after,"kingdom":after_kingdom},"same_economy_allocation":true,"flow_reset_in_test":false}));
        println!(
            "{pc:?}: actual immediate Actor{} Economy and storedKingdom{} real flows",
            id.raw(),
            owner.raw()
        );
    }
    // Replay the native result against this fresh immediate-load allocation.
    // Initial capture is the sole stage allowed before the fixture exists.
    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-load-economy-refresh/native-held-refresh.json");
    if fixture_path.exists() {
        let fixture: Value = serde_json::from_slice(&std::fs::read(fixture_path).unwrap()).unwrap();
        for row in &rows {
            let matches: Vec<_> = fixture["cases"].as_array().unwrap().iter()
                .filter(|native| native["actor"] == row["actor"]
                    && native["precision"] == row["input"]["precision"])
                .collect();
            assert_eq!(matches.len(), 2, "both native refresh entrypoints");
            for native in matches {
                assert_eq!(native["input"], row["input"]);
                assert_eq!(native["output"], row["output"]);
            }
        }
    } else {
        assert!(std::env::var_os("AA_HELD_ECONOMY_CAPTURE").is_some(),
            "native held-refresh fixture is required for replay");
    }
    if let Some(path) = std::env::var_os("AA_HELD_ECONOMY_CAPTURE") {
        let out = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(out, &json!({"cases":rows})).unwrap();
    }
}
