//! Source census for the exact ordinary Property identities, not admission.
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
#[test]
#[ignore = "requires AA_GAME_ROOT actual original/v14 sources and explicit new capture path"]
fn actual_property_instance_source_census() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let pc = Precision::Double53;
    let data = actors.native_layout_data(pc).unwrap();
    let domain = actors.native_target_domain(pc).unwrap();
    let roots = actors.resolved_definitions("Property").unwrap();
    let mut rows = Vec::new();
    for property in &data.properties {
        let node = roots
            .iter()
            .find(|node| node.sources() == property.sources)
            .unwrap();
        let mut mods = Vec::new();
        let mut error = None;
        for child in node.children("Modifier").unwrap() {
            match actors.native_property_modifier(&child, &domain, pc) {
                Ok(Some(modifier)) => mods.push(modifier),
                Ok(None) => {}
                Err(e) => {
                    error = Some(e.to_string());
                    break;
                }
            }
        }
        let fields: [(&str, serde_json::Value); 4] = [
            (
                "required_properties",
                serde_json::to_value(node.field("required_properties", pc).unwrap()).unwrap(),
            ),
            (
                "enabled_target_flags",
                serde_json::to_value(node.field("enabled_target_flags", pc).unwrap()).unwrap(),
            ),
            (
                "pass_down_members",
                serde_json::to_value(node.field("pass_down_members", pc).unwrap()).unwrap(),
            ),
            (
                "performer_cv_bias",
                serde_json::to_value(node.field("performer_cv_bias", pc).unwrap()).unwrap(),
            ),
        ];
        let fields: serde_json::Map<String, serde_json::Value> =
            fields.into_iter().map(|(k, v)| (k.into(), v)).collect();
        rows.push(serde_json::json!({"ordinal":property.ordinal,"id":property.ids,"sources":property.sources,
            "reloads":property.reloads,"modifiers":mods,"modifier_error":error,"fields":fields,
            "fx_counts":{"WhenAdded":node.children("WhenAdded").unwrap().len(),"WhenRemoved":node.children("WhenRemoved").unwrap().len(),"WhenResearched":node.children("WhenResearched").unwrap().len()}}));
    }
    let report = serde_json::json!({"scope":"actual Property source census only; no instance/application claim",
        "source_precision":"Double53","source_identities":actors.sources,"properties":rows});
    let path = std::env::var_os("AA_PROPERTY_INSTANCE_SOURCE_CAPTURE").unwrap();
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    serde_json::to_writer_pretty(file, &report).unwrap();
}
use aa_actors::components::ComponentSlot;
use aa_mapworld::restore::{RestoredComponent, RestoredRecord};
use aa_runtime::{
    catalog::Catalog,
    live_object_init::Record,
    load_property_instances::{self as live, State},
};
use serde_json::{Value, json};
fn props<'a>(record: &'a Record<'_>) -> &'a aa_body::properties::PropertiesState {
    let Some(RestoredComponent::Properties(p)) = record
        .shell
        .actor
        .as_ref()
        .unwrap()
        .components
        .get(ComponentSlot::Properties)
    else {
        panic!("actual Properties required")
    };
    p
}
fn snapshot(record: &Record<'_>, state: &State) -> Value {
    json!({"properties":props(record),"instances":state.instances(),"constructors":state.constructors().iter().map(|(h,p)|json!({"handle":h.0,"definition":p.definition.0,"kind":p.kind,"source_id":p.source_id})).collect::<Vec<_>>()})
}
fn operation(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    state: &mut State,
    property: u32,
    phase: &str,
    pc: Precision,
    clock: u32,
    rows: &mut Vec<Value>,
) {
    let def = state
        .definition_for_reference(
            aa_rules::native::layout_data::PropertyRef::from_handle(property).unwrap(),
        )
        .unwrap();
    let actor = record.shell.actor.as_ref().unwrap();
    let organization=match actor.components.get(ComponentSlot::Organization){Some(RestoredComponent::Organization(org))=>Some(org.members.iter().map(|id| {
        if id.is_null(){json!({"id":0,"properties":false,"experience":false})}else {
            let Some(RestoredRecord::Actor(e))=record.registry.get(*id).map(|e|&e.value)else{panic!("actual member allocation required")};let a=e.shell.actor.as_ref().unwrap();
            json!({"id":id.raw(),"properties":a.components.present(ComponentSlot::Properties),"experience":a.components.present(ComponentSlot::Experience)})
        }
    }).collect::<Vec<_>>()),_=>None};
    let input = json!({"owner":record.owner.raw(),"id":record.definition.proof.id,"native_class":record.definition.native_class,
        "organization":organization,"pass_members":state.definitions().keys().map(|h|(h.0.to_string(),state.facts(*h).unwrap().pass_down_members())).collect::<std::collections::BTreeMap<_,_>>(),
        "components":actor.components.iter().map(|(s,_)|s.index()).collect::<Vec<_>>(),"allegiance":actor.words.get(&0xE8),
        "clock":clock,"phase":phase,"property":def.0,"kind":if phase=="duplicate"{7}else{0},"source_id":if phase=="duplicate"{record.owner.raw()}else{0},
        "before":snapshot(record,state),"definitions":state.definitions()});
    let address = props(record) as *const _ as usize;
    let result = live::add(
        catalog,
        record,
        state,
        property,
        if phase == "duplicate" { 7 } else { 0 },
        (phase == "duplicate").then_some(record.owner),
        || {
            if phase == "missing_clock" {
                Err("actual World+E0 clock is unbound".into())
            } else {
                Ok(clock)
            }
        },
        pc,
        &mut live::RequiredEffects,
    );
    assert_eq!(
        props(record) as *const _ as usize,
        address,
        "same real component allocation survives every result"
    );
    if phase == "success" || phase == "duplicate" {
        result.as_ref().unwrap();
    } else {
        assert!(result.is_err(), "{phase} must reach its explicit boundary")
    }
    rows.push(json!({"precision":format!("{pc:?}"),"input":input,"output":snapshot(record,state),"error":result.err()}));
}
#[test]
#[ignore = "requires actual original/v14 sources; default is immutable captured/native replay"]
fn actual_saved_allocations_property_prefixes_all_pc() {
    use aa_mapworld::restore::RestoreMode;
    use aa_rules::{resources::ResourceCatalog, tgi::Limits};
    use aa_runtime::{
        combat_source_completion as completion,
        combat_source_receipt::VerifiedStartupReceipt,
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        restore_lifecycle::{ScopedManagedLoad, TaskKind},
        settings::Settings,
    };
    use std::sync::Arc;
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
    let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let mut attempt = Attempt::load_with_source_setup(
        &map,
        &actors,
        settings.clone(),
        text,
        Precision::Double53,
        u64::MAX,
        RestoreMode::InspectPartial,
        LoadBindings {
            native_load_mode: Some(1),
            inactive_load: Some(inactive),
            resources: Some(resources.clone()),
        },
        |catalog| {
            let reference =
                VerifiedStartupReceipt::verify_embedded_reference(catalog, &mut sources)?;
            completion::install(
                catalog,
                &mut sources,
                reference.context(),
                reference.csv(),
                reference.globals(),
            )?;
            completion::compute(catalog)
        },
    )
    .unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(settings);
    catalog.native_load_mode = Some(1);
    catalog.bind_resource_catalog(resources).unwrap();
    *catalog.definition_bank.borrow_mut() = attempt.definitions;
    *catalog.ability_arena.borrow_mut() = attempt.abilities;
    let mut state = State::from_world_constructor();
    state.bind_registered_sources(&catalog).unwrap();
    assert_eq!(state.definitions().len(), 165);
    let enchantment = actors.native_data_registry("EnchantmentType").unwrap();
    assert_eq!(
        enchantment
            .entries
            .iter()
            .map(|e| e.ids.as_str())
            .collect::<Vec<_>>(),
        vec![
            "ench_type_generic",
            "ench_type_radiation",
            "ench_type_courage",
            "ench_type_fear"
        ]
    );
    assert!(
        state.require_catalog(&Catalog::new(&actors)).is_err(),
        "same source files cannot substitute a foreign unfinished Bank owner"
    );
    let global = live::global(&catalog).unwrap().unwrap();
    let global_def = state
        .definition_for_reference(
            aa_rules::native::layout_data::PropertyRef::from_handle(global).unwrap(),
        )
        .unwrap();
    assert_eq!(state.facts(global_def).unwrap().id(), "mobile");
    let infantry = state
        .definitions()
        .keys()
        .copied()
        .find(|h| state.facts(*h).unwrap().id() == "infantry")
        .unwrap();
    let infantry_ref = state.facts(infantry).unwrap().reference().handle().unwrap();
    let numeric=state.definitions().iter().find(|(_,d)|matches!(d.modifiers.first(),Some(aa_body::property_flags::Modifier::Numeric(m))if m.kind==aa_body::property_effects::NumericKind::Add)).unwrap().0;
    let numeric_ref = state.facts(*numeric).unwrap().reference().handle().unwrap();
    let ids = attempt
        .registry
        .iter()
        .filter_map(|entry| match &entry.value {
            RestoredRecord::Kingdom(k)
                if k.runtime.as_ref().is_some_and(|s| {
                    s.base
                        .actor
                        .as_ref()
                        .is_some_and(|a| a.components.present(ComponentSlot::Properties))
                }) =>
            {
                Some((entry.id, false))
            }
            RestoredRecord::Actor(a)
                if a.shell.actor.as_ref().is_some_and(|a| {
                    a.components.present(ComponentSlot::Properties)
                        && a.components.present(ComponentSlot::Performer)
                        && !a.components.present(ComponentSlot::Organization)
                        && !a.components.present(ComponentSlot::Settlement)
                }) =>
            {
                Some((entry.id, true))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let org_id = attempt
        .registry
        .iter()
        .find_map(|e| match &e.value {
            RestoredRecord::Actor(a)
                if a.shell.actor.as_ref().is_some_and(|a| {
                    a.components.present(ComponentSlot::Properties)
                        && a.components.present(ComponentSlot::Organization)
                        && !a.components.present(ComponentSlot::Performer)
                }) =>
            {
                Some(e.id)
            }
            _ => None,
        })
        .unwrap();
    let mut selected = ids
        .iter()
        .filter(|(_, p)| !*p)
        .take(3)
        .chain(ids.iter().filter(|(_, p)| *p).take(3))
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 6);
    selected.push((org_id, false));
    let clock = attempt.clock.begin.0;
    let mut rows = vec![];
    for (ordinal, (id, performer)) in selected.into_iter().enumerate() {
        let mut held = std::mem::replace(
            attempt.registry.value_mut(id).unwrap(),
            RestoredRecord::Loading,
        );
        let (shell, definition) = match &mut held {
            RestoredRecord::Kingdom(e) => {
                let r = e.runtime.as_mut().unwrap();
                (&mut r.base, &r.definition)
            }
            RestoredRecord::Actor(e) => (&mut e.shell, &e.definition),
            _ => unreachable!(),
        };
        let mut record = Record {
            owner: id,
            shell,
            definition,
            registry: &mut attempt.registry,
        };
        assert!(props(&record).effects.is_empty());
        for pc in [[
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ][ordinal % 3]]
        {
            let applicable = live::applicable(&catalog, &record, &mut state, global).unwrap();
            rows.push(json!({"operation":"applicable","precision":format!("{pc:?}"),"owner":id.raw(),"result":applicable}));
            if id == org_id {
                operation(
                    &catalog,
                    &mut record,
                    &mut state,
                    global,
                    "success",
                    pc,
                    clock,
                    &mut rows,
                );
            } else if !performer {
                operation(
                    &catalog,
                    &mut record,
                    &mut state,
                    global,
                    "missing_clock",
                    pc,
                    clock,
                    &mut rows,
                );
                operation(
                    &catalog,
                    &mut record,
                    &mut state,
                    global,
                    "success",
                    pc,
                    clock,
                    &mut rows,
                );
                operation(
                    &catalog,
                    &mut record,
                    &mut state,
                    infantry_ref,
                    "missing_clamp",
                    pc,
                    clock,
                    &mut rows,
                );
            } else {
                //New actual definitions ensure an active addition reaches the
                //real Performer tail or a required numeric virtual30.
                operation(
                    &catalog,
                    &mut record,
                    &mut state,
                    global,
                    "performer",
                    pc,
                    clock,
                    &mut rows,
                );
                operation(
                    &catalog,
                    &mut record,
                    &mut state,
                    numeric_ref,
                    "numeric",
                    pc,
                    clock,
                    &mut rows,
                );
            }
            operation(
                &catalog,
                &mut record,
                &mut state,
                global,
                "duplicate",
                pc,
                clock,
                &mut rows,
            );
        }
        drop(record);
        *attempt.registry.value_mut(id).unwrap() = held;
    }
    let report = json!({"scope":"actual held tutorial02 restored-prefix allocations; direct property operation probes, not full immediate lifecycle","source_precision":"Double53","sources":actors.sources,"rows":rows});
    if let Some(path) = std::env::var_os("AA_PROPERTY_INSTANCE_INPUT_CAPTURE") {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file, &report).unwrap();
    } else {
        let expected: Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-load-property-instances/actual-inputs-02.json"
        ))
        .unwrap();
        assert_eq!(report, expected);
        let native:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261016-load-property-instances/native-property-instances.json")).unwrap();
        let actual = rows
            .iter()
            .filter(|r| r.get("input").is_some())
            .collect::<Vec<_>>();
        assert_eq!(actual.len(), native["rows"].as_array().unwrap().len());
        for (row, native) in actual.iter().zip(native["rows"].as_array().unwrap()) {
            assert_eq!(row["output"], native["output"]["state"]);
        }
    }
}
