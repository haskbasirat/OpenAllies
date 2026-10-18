use super::*;
use crate::{
    catalog::Catalog, localization::Localization, restoration::Attempt, settings::Settings,
};
use aa_mapworld::restore::{OrganizationAttachContext, RestoredRecord};
use aa_mind::action_mask::World;
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use serde_json::json;

fn parts<'a>(actions: &'a SavedActions<'_, '_, '_>, id: ActorId) -> &'a RestoredMindParts {
    match actions
        .shell(id)
        .unwrap()
        .components
        .get(ComponentSlot::Mind)
        .unwrap()
    {
        RestoredComponent::Mind(value) => value.state.as_any().downcast_ref().unwrap(),
        _ => panic!("Mind slot"),
    }
}

/// The snapshots below are observations, never runtime copies supplied to
/// gameplay. Pointer checks additionally prove the stack/command allocations
/// remain the same, beyond merely equal values.
fn identity(parts: &RestoredMindParts) -> (String, usize, usize, usize) {
    (
        format!("{parts:?}"),
        parts.commands.as_ptr() as usize,
        parts.mind.as_ref().unwrap().stack.as_slices().0.as_ptr() as usize,
        parts
            .mind
            .as_ref()
            .unwrap()
            .pending_commands
            .as_slices()
            .0
            .as_ptr() as usize,
    )
}

fn observation(actions: &mut SavedActions<'_, '_, '_>, id: ActorId) -> serde_json::Value {
    let projection = Projection {
        actions,
        regiment_status: None,
    };
    let actor = projection.actor(id.raw()).unwrap();
    let attached = actor
        .element
        .map(|e| projection.element_organization(e).unwrap())
        .flatten();
    let abilities = actor
        .performer
        .map(|p| projection.abilities(p))
        .transpose()
        .unwrap();
    let properties =
        aa_actors::mobile_deployment_actions::World::properties_flags(projection.actions, id)
            .unwrap();
    let leader = attached.map(|id| {
        let fields = projection.actor(id).unwrap();
        let link = projection.organization_link_d0(id).unwrap();
        json!({"actor":fields,"link":link,"resolved":projection.resolve(link).unwrap()})
    });
    let mobile = actor.componentac.map(|_| {
        json!({"unpack":aa_actors::mobile_deployment_actions::can_unpack(projection.actions,id,true).map_err(|e|e.to_string()),
        "pack":aa_actors::mobile_deployment_actions::can_pack(projection.actions,id,true,projection.actions.context.precision).map_err(|e|e.to_string())})
    });
    json!({"actor":actor,"before":projection.action_bits(id.raw()).unwrap(),"attached":attached,
        "abilities":abilities,"properties":properties,"leader":leader,"mobile":mobile,
        "fog":projection.setting_cache108().map_err(|e|e.to_string())})
}

#[test]
#[ignore = "requires original AA_GAME_ROOT assets"]
fn saved_mind_uses_original_owned_core_and_restores_it_on_error() {
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
    let localization = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            root.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial01.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut attempt = Attempt::load_with_native_mode(
        &map,
        &actors,
        settings,
        localization,
        Precision::Double53,
        u64::MAX,
        aa_mapworld::restore::RestoreMode::StopOnError,
        Some(1),
    )
    .unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(attempt.settings.clone());
    catalog.native_load_mode = attempt.native_load_mode;
    catalog.ability_arena = std::cell::RefCell::new(std::mem::take(&mut attempt.abilities));
    catalog.definition_bank = std::cell::RefCell::new(std::mem::take(&mut attempt.definitions));
    let mut unknown_mode = Catalog::new(&actors);
    unknown_mode.settings = Some(attempt.settings.clone());
    // An actual loading Organization is held outside the registry, matching
    // its native recursive member Load call. No replacement Mind is created.
    let owner = attempt
        .registry
        .iter()
        .find_map(|entry| {
            entry
                .value
                .loaded()?
                .shell
                .actor
                .as_ref()?
                .components
                .present(ComponentSlot::Organization)
                .then_some(entry.id)
        })
        .expect("tutorial01 Organization");
    let taken = std::mem::replace(
        attempt.registry.value_mut(owner).unwrap(),
        RestoredRecord::Loading,
    );
    let RestoredRecord::Actor(mut parent) = taken else {
        panic!("ordinary owner")
    };
    let owner_actor = parent.shell.actor.as_mut().unwrap();
    let Some(RestoredComponent::Organization(mut organization)) =
        owner_actor.components.remove(ComponentSlot::Organization)
    else {
        panic!("Organization")
    };
    let mut actions = SavedActions {
        catalog: &catalog,
        context: OrganizationAttachContext {
            inventory: &attempt.inventory,
            owner,
            owner_actor,
            owner_definition: &parent.definition,
            owner_thing_flags: parent.shell.thing.flags,
            owner_thing_word_c: parent.shell.thing.word_c,
            owner_object_byte_4c: parent.shell.object.as_ref().unwrap().byte_4c,
            organization: &mut organization,
            registry: &mut attempt.registry,
            precision: Precision::Double53,
        }
        .into(),
        active_mind: None,
    };
    let candidates: Vec<_> = actions
        .context
        .registry
        .iter()
        .filter_map(|entry| {
            let actor = entry.value.loaded()?.shell.actor.as_ref()?;
            let RestoredComponent::Mind(mind) = actor.components.get(ComponentSlot::Mind)? else {
                return None;
            };
            let parts = mind.state.as_any().downcast_ref::<RestoredMindParts>()?;
            (parts.installed.kind == Kind::Element).then_some(entry.id)
        })
        .collect();
    assert!(!candidates.is_empty());
    let oracle: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../research/resume-20261016-saved-mind-actions/native-saved-masks.json"
    ))
    .unwrap();
    let mut results = Vec::new();
    let mut native_replays = 0;
    let mut owned_commands = 0;
    for id in candidates {
        owned_commands += parts(&actions, id).commands.len();
        let before = identity(parts(&actions, id));
        let input = observation(&mut actions, id);
        let result = contribute(&mut actions, id);
        assert_eq!(
            before,
            identity(parts(&actions, id)),
            "owned Mind/states/commands changed for {id:?}"
        );
        assert!(actions.active_mind.is_none());
        assert!(flags::World::has_component(&actions, id, ComponentSlot::Mind).unwrap());
        for case in oracle["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|case| case["source_actor"] == id.raw())
        {
            native_replays += 1;
            assert!(
                result.is_ok(),
                "source-observed native case failed: {result:?}"
            );
            assert_eq!(
                flags::World::actor_fields(&actions, id)
                    .unwrap()
                    .action_bits,
                case["output"]["actions"]["101"].as_u64().unwrap()
            );
        }
        results.push(
            json!({"id":id.raw(),"input":input,"result":result.map_err(|e|e.to_string()),
            "actions":flags::World::actor_fields(&actions,id).unwrap().action_bits}),
        );
    }
    assert_eq!(native_replays, 12);
    assert!(
        owned_commands > 0,
        "source case must preserve actual owned saved commands"
    );
    // A genuine callback error also returns the existing component allocation.
    // Remove the required Element only for this negative adapter case, then
    // restore it. Native Element+B4 must fail after its first bit5 store.
    let id = ActorId::from_raw(results[0]["id"].as_u64().unwrap() as u32);
    let before = identity(parts(&actions, id));
    let element = actions
        .shell_mut(id)
        .unwrap()
        .components
        .remove(ComponentSlot::Element)
        .expect("Element");
    let error = contribute(&mut actions, id).unwrap_err().to_string();
    assert!(error.contains("Element component+80"), "{error}");
    assert_eq!(before, identity(parts(&actions, id)));
    assert!(actions.active_mind.is_none());
    let fields = flags::World::actor_fields(&actions, id).unwrap();
    assert_eq!(
        fields.action_bits & (1 << 5) != 0,
        fields.thing_flags & (1 << 29) == 0
    );
    actions
        .shell_mut(id)
        .unwrap()
        .components
        .insert(ComponentSlot::Element, element);
    // No explicit native MapResource mode: stop exactly when FogOfWar is
    // requested; do not inherit creator/header metadata or a defaultmode.
    let fog_actor = results
        .iter()
        .find(|r| {
            !r["input"]["attached"].is_null()
                && r["input"]["actor"]["flags"].as_u64().unwrap() & (1 << 29) == 0
        })
        .map(|r| ActorId::from_raw(r["id"].as_u64().unwrap() as u32))
        .unwrap();
    let before = identity(parts(&actions, fog_actor));
    actions.catalog = &unknown_mode;
    let unknown_mode_error = contribute(&mut actions, fog_actor).unwrap_err().to_string();
    assert!(
        unknown_mode_error.contains("explicit native MapResource load mode"),
        "{unknown_mode_error}"
    );
    assert_eq!(before, identity(parts(&actions, fog_actor)));
    let report = json!({"map":"Campaign/tutorial01.RMP","minds":results,"negative_error":error,
        "unknown_mode_error":unknown_mode_error,"map_source":attempt.inventory.map_source,
        "restore_errors":attempt.report.events.iter().filter_map(|e|e.error.as_ref()).collect::<Vec<_>>()});
    if let Some(path) = std::env::var_os("AA_SAVED_MIND_ACTIONS_OUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    println!("{report}");
}
