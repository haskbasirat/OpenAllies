//! Original tutorial02 checkpoint at native Element attachment. This is a
//! managed partial-record diagnostic. Immediate Init/PostCreate and complete
//! publication are still omitted; no missing Kingdom arrays are synthesized.
use aa_actors::{ActorId, components::ComponentSlot};
use aa_mapworld::restore::{OrganizationAttachContext, RestoredComponent, RestoredRecord};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use aa_runtime::{
    localization::Localization,
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
};
use serde_json::json;
use std::sync::Arc;

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn original_tutorial02_element_attachment_checkpoint() {
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
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    // A new owned application/load interval from actual UVars sources. This
    // constructor clock is a stated diagnostic fixture, not original uptime.
    let inactive_load = ScopedManagedLoad::from_sources(&mut sources, 375)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let observer_address = inactive_load.synchronization_owner_address();
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            root.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut attempt = Attempt::load_with_bindings(
        &map,
        &actors,
        settings,
        text,
        Precision::Double53,
        u64::MAX,
        aa_mapworld::restore::RestoreMode::StopOnError,
        LoadBindings {
            native_load_mode: None,
            inactive_load: Some(inactive_load),
            resources: Some(Arc::clone(&resources)),
        },
    )
    .unwrap();
    let mut records = Vec::new();
    for entry in attempt.registry.iter() {
        let Some(entity) = entry.value.loaded() else {
            continue;
        };
        let Some(actor) = &entity.shell.actor else {
            continue;
        };
        if !actor.components.present(ComponentSlot::Element)
            && !actor.components.present(ComponentSlot::Organization)
        {
            continue;
        }
        let organization = match actor.components.get(ComponentSlot::Organization) {
            Some(RestoredComponent::Organization(org)) => Some(json!({
                "members":org.members.iter().map(|id|id.raw()).collect::<Vec<_>>(),
                "words":org.words,"layout":org.layout.definition,
            })),
            _ => None,
        };
        let element = match actor.components.get(ComponentSlot::Element) {
            Some(RestoredComponent::Element(element)) => Some(json!({
                "organization":element.organization.map(|id|id.raw()),
                "slot":element.slot_index,"attached":element.flag_1c,
                "definition":element.element_definition,
            })),
            _ => None,
        };
        records.push(json!({"id":entry.id.raw(),"definition":entity.definition.proof.id,
            "flags":entity.shell.thing.flags,"state_low":actor.components.state.low,
            "state_high":actor.components.state.high,"words":actor.words,
            "components":actor.components.iter().map(|(slot,_)|format!("{slot:?}")).collect::<Vec<_>>(),
            "organization":organization,"element":element}));
    }
    let mut report = json!({"first_failure":attempt.report.events.iter().find(|event|event.error.is_some()),
        "records":records,"publication_ready":attempt.report.runtime_publication_ready,
        "context":{"managed_fresh_scope":true,"inactive_observer":true,"elapsed_clock_fixture_millis":375,
            "native_load_mode":attempt.native_load_mode,"resource_catalog_bound":true,
            "immediate_Init_PostCreate":"not_executed_by_this_record_diagnostic"}});
    assert!(!attempt.report.runtime_publication_ready);
    assert_eq!(attempt.native_load_mode, Some(1));
    assert_eq!(
        attempt
            .inactive_load
            .as_ref()
            .unwrap()
            .synchronization_owner_address(),
        observer_address
    );
    assert!(Arc::ptr_eq(
        attempt.resource_source.as_ref().unwrap(),
        &resources
    ));
    let first_event = attempt
        .report
        .events
        .iter()
        .find(|event| event.error.is_some());
    // The real HQ slot30 loader now completes this record pass. This still
    // omits immediate Init; in particular it does not initialize Kingdom flows.
    assert!(first_event.is_none(), "{first_event:?}");

    // The historical original-code prefix proves the membership stores.
    // The current production adapter continues through the implemented
    // contributors, gated HQ Economy refresh and managed inactive observer
    // (or stops at a fixture's absent group13). Only those deliberately
    // supplied flags/state/action/membership fields are reset between cases.
    // Later contributor state can cascade; these are not independent native
    // full-attachment fixtures and assert no complete action-mask parity.
    // Its later action mask is not claimed by that older stopped oracle.
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/resume-20261016-element-attach/native-attach-prefix.json"
    ))
    .unwrap();
    let owner = ActorId::from_raw(4099);
    let member = ActorId::from_raw(4100);
    let taken = std::mem::replace(
        attempt.registry.value_mut(owner).unwrap(),
        RestoredRecord::Loading,
    );
    let RestoredRecord::Actor(mut entity) = taken else {
        panic!("parent Actor")
    };
    let actor = entity.shell.actor.as_mut().unwrap();
    let Some(RestoredComponent::Organization(mut org)) =
        actor.components.remove(ComponentSlot::Organization)
    else {
        panic!("active parent Organization")
    };
    assert_eq!(org.members[4], member);
    let mut catalog = aa_runtime::catalog::Catalog::new(&actors);
    catalog.native_load_mode = attempt.native_load_mode;
    catalog
        .bind_resource_catalog(Arc::clone(attempt.resource_source.as_ref().unwrap()))
        .unwrap();
    catalog
        .bind_inactive_load(attempt.inactive_load.as_ref().unwrap())
        .unwrap();
    catalog.settings = Some(attempt.settings.clone());
    catalog.ability_arena = std::cell::RefCell::new(std::mem::take(&mut attempt.abilities));
    catalog.definition_bank = std::cell::RefCell::new(std::mem::take(&mut attempt.definitions));
    let mut outcomes = Vec::new();
    for case in oracle["cases"].as_array().unwrap() {
        let input = &case["input"];
        let expected = &case["output"];
        let target = attempt
            .registry
            .value_mut(member)
            .unwrap()
            .loaded_mut()
            .unwrap();
        target.shell.thing.flags = input["flags"].as_u64().unwrap() as u32;
        let target_actor = target.shell.actor.as_mut().unwrap();
        let state = input["state"].as_u64().unwrap();
        target_actor.components.state.low = state as u32;
        target_actor.components.state.high = (state >> 32) as u32;
        let actions = input["actions"].as_u64().unwrap();
        target_actor.words.insert(0xf0, actions as u32);
        target_actor.words.insert(0xf4, (actions >> 32) as u32);
        let Some(RestoredComponent::Element(element)) =
            target_actor.components.get_mut(ComponentSlot::Element)
        else {
            panic!("member Element")
        };
        assert!(element.element_definition.is_none());
        element.organization = None;
        element.slot_index = -1;
        element.flag_1c = false;
        let result = aa_runtime::saved_organization_actions::attach(
            &catalog,
            OrganizationAttachContext {
                inventory: &attempt.inventory,
                owner,
                owner_actor: actor,
                owner_definition: &entity.definition,
                owner_thing_flags: entity.shell.thing.flags,
                owner_thing_word_c: entity.shell.thing.word_c,
                owner_object_byte_4c: entity.shell.object.as_ref().unwrap().byte_4c,
                organization: &mut org,
                registry: &mut attempt.registry,
                precision: Precision::Double53,
            },
            member,
            input["slot"].as_i64().unwrap() as i32,
        );
        outcomes.push(
            json!({"input":input,"error":result.err(),"native_prefix_membership_compared":true}),
        );
        let target = attempt
            .registry
            .get(member)
            .unwrap()
            .value
            .loaded()
            .unwrap()
            .shell
            .actor
            .as_ref()
            .unwrap();
        let Some(RestoredComponent::Element(element)) =
            target.components.get(ComponentSlot::Element)
        else {
            panic!("Element retained")
        };
        assert_eq!(
            element.organization.unwrap().raw(),
            expected["organization"].as_u64().unwrap() as u32
        );
        assert_eq!(
            element.slot_index as u32,
            expected["slot"].as_u64().unwrap() as u32
        );
        assert_eq!(
            u64::from(element.flag_1c),
            expected["attached"].as_u64().unwrap()
        );
    }
    actor.components.insert(
        ComponentSlot::Organization,
        RestoredComponent::Organization(org),
    );
    *attempt.registry.value_mut(owner).unwrap() = RestoredRecord::Actor(entity);
    report["attachment_probe_outcomes"] = json!(outcomes);
    if let Some(path) = std::env::var_os("AA_ELEMENT_ATTACH_OUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    } else {
        println!("{report}");
    }
    // Retain the historical action-boundary assertions after collecting ALL
    // eight native membership prefixes. A newly reached required service can
    // then be diagnosed from the exact input and partial state, without losing
    // the other seven membership comparisons or silently accepting its error.
    for outcome in report["attachment_probe_outcomes"].as_array().unwrap() {
        let state = outcome["input"]["state"].as_u64().unwrap();
        if state & (1 << 27) != 0 {
            assert!(outcome["error"].is_null(), "{outcome}");
        } else {
            let error = outcome["error"].as_str().expect("missing group13 must refuse");
            assert!(error.contains("582C40 group13 selected -1"), "{outcome}");
        }
    }
}
