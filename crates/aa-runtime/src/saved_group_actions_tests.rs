use super::*;
use crate::{localization::Localization, restoration::Attempt, settings::Settings};
use aa_actors::ability_flags::World;
use aa_numeric::Precision;
use aa_rules::{
    actors::{
        ActorCatalog,
        native_load::{NativeLoadReport, load_files},
    },
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use serde_json::json;

#[test]
fn shared_ability_reference_cannot_silently_select_shadowed_definition() {
    let file = |path: &str, text: &str| {
        let bytes = text.as_bytes().to_vec();
        ResolvedFile {
            logical_path: path.into(),
            origin: Origin::V14Loose,
            sha256: sha256(&bytes),
            bytes,
        }
    };
    let actors = load_files(
        vec![
            file(
                "Templates/template_translate.tgi",
                "[template Translate]\n{\nstring node\nstring ids_old\nstring ids_new\n}\n",
            ),
            file(
                "Game/ability_fixture.tgi",
                "[AbilityDefinition]\n{\nstring IDS = alpha\n}\n[AbilityDefinition]\n{\nstring IDS = beta\n}\n[Translate]\n{\nnode = AbilityDefinition\nids_old = alpha\nids_new = beta\n}\n",
            ),
        ],
        Limits::default(),
        &mut NativeLoadReport::default(),
    );
    let entry = actors
        .native_data_find("AbilityDefinition", "beta")
        .unwrap();
    let mut value = actors
        .native_data_node(entry)
        .unwrap()
        .field("IDS", Precision::Double53)
        .unwrap();
    let catalog = Catalog::new(&actors);
    require_shared_ability_identity(&catalog, &value).unwrap();
    let Some(aa_rules::native::TypedValue::String { value: key, .. }) = &mut value else {
        panic!("typed IDS")
    };
    *key = "alpha".into();
    assert_eq!(
        actors
            .native_data_find("AbilityDefinition", "alpha")
            .unwrap()
            .ids,
        "beta"
    );
    assert!(
        require_shared_ability_identity(&catalog, &value)
            .unwrap_err()
            .to_string()
            .contains("translated shared")
    );
}

#[test]
#[ignore = "requires original/v14 AA_GAME_ROOT assets"]
fn original_kursk_group_abilities_use_saved_layout_and_ordered_source() {
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
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(root.join("reconstruction/assets/original/Data/Maps/(2v2)Kursk.RMP"))
            .unwrap(),
    )
    .unwrap();
    let mut attempt = Attempt::load_with_native_mode(
        &map,
        &actors,
        settings,
        text,
        Precision::Double53,
        u64::MAX,
        aa_mapworld::restore::RestoreMode::StopOnError,
        Some(1),
    )
    .unwrap();
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
        .expect("original Organization");
    let taken = std::mem::replace(
        attempt.registry.value_mut(owner).unwrap(),
        RestoredRecord::Loading,
    );
    let RestoredRecord::Actor(mut entity) = taken else {
        panic!("Organization actor")
    };
    let actor = entity.shell.actor.as_mut().unwrap();
    let Some(RestoredComponent::Organization(mut org)) =
        actor.components.remove(ComponentSlot::Organization)
    else {
        panic!("owned Organization")
    };
    let catalog = Catalog::new(&actors);
    let mut actions = SavedActions {
        catalog: &catalog,
        context: OrganizationAttachContext {
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
        }
        .into(),
        active_mind: None,
    };
    let count = actions.organization_group_count(owner).unwrap();
    assert!(count > 0);
    let mut groups = Vec::new();
    for index in 0..count {
        let locked = actions.organization_group_locked(owner, index).unwrap();
        let flags = if actions.org(owner).unwrap().layout.parameters[index].is_some() {
            Some(
                (0..actions
                    .organization_group_ability_count(owner, index)
                    .unwrap())
                    .map(|ability| {
                        actions
                            .organization_group_ability_flags(owner, index, ability)
                            .unwrap()
                    })
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        };
        groups.push(json!({"locked":locked,"abilities":flags,"parameter":actions.org(owner).unwrap().layout.parameters[index]}));
    }
    let all = flags::has_attack_class(&actions, owner, false).map_err(|e| e.to_string());
    let locked = flags::has_attack_class(&actions, owner, true).unwrap();
    // A real unloaded/absent pointer in an unlocked slot must stay unread.
    let mut nulled = Vec::new();
    for (index, g) in groups.iter().enumerate() {
        if !g["locked"].as_bool().unwrap() {
            nulled.push((
                index,
                actions
                    .context
                    .organization
                    .as_deref_mut()
                    .unwrap()
                    .layout
                    .parameters[index]
                    .take(),
            ));
        }
    }
    assert_eq!(
        flags::has_attack_class(&actions, owner, true).unwrap(),
        locked
    );
    for (index, value) in nulled {
        actions
            .context
            .organization
            .as_deref_mut()
            .unwrap()
            .layout
            .parameters[index] = value;
    }
    // Separate source-derived probe: use an authored artillery layout and
    // parameter list over the same owned Organization storage, then restore
    // the saved layout. This is not claimed as a Kursk saved regiment.
    let data = catalog.layout_data(Precision::Double53).unwrap();
    let authored = data
        .organization_component(
            &actors,
            "regiment_usa_infantry_artillery",
            Precision::Double53,
        )
        .unwrap()
        .unwrap();
    let layout = &data.layouts[authored.layout as usize];
    let prior = std::mem::replace(
        &mut actions.context.organization.as_deref_mut().unwrap().layout,
        aa_actors::ground_states::LayoutDefinition {
            definition: authored.layout_ids,
            parameters: authored.parameters,
            slots: layout.slot_groups.len(),
        },
    );
    let artillery_groups:Vec<_>=(0..actions.organization_group_count(owner).unwrap()).map(|index|{
        let flags:Vec<_>=(0..actions.organization_group_ability_count(owner,index).unwrap())
            .map(|ability|actions.organization_group_ability_flags(owner,index,ability).unwrap()).collect();
        json!({"locked":actions.organization_group_locked(owner,index).unwrap(),"abilities":flags,"parameter":actions.org(owner).unwrap().layout.parameters[index]})
    }).collect();
    let artillery_all = flags::has_attack_class(&actions, owner, false).unwrap();
    let artillery_locked = flags::has_attack_class(&actions, owner, true).unwrap();
    assert!(
        artillery_all,
        "authored mortar AbilityDefinition must retain bombard bit3"
    );
    actions.context.organization.as_deref_mut().unwrap().layout = prior;
    let report = json!({"map":"(2v2)Kursk.RMP","map_source":attempt.inventory.map_source,"owner":owner.raw(),"definition":entity.definition.proof.id,
        "groups":groups,"all":all,"locked":locked,"first_error":attempt.report.events.iter().find_map(|e|e.error.as_ref()),
        "artillery_source_probe":{"scope":"source-derived test layout, not a saved Kursk actor","definition":"regiment_usa_infantry_artillery","groups":artillery_groups,"all":artillery_all,"locked":artillery_locked}});
    if let Some(path) = std::env::var_os("AA_GROUP_ACTIONS_OUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    println!("{report}");
}
