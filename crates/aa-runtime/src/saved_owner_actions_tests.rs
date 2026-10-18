use super::*;
use crate::{
    localization::Localization,
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
};
use aa_actors::{ActorId, ability_flags::World, components::ComponentSlot};
use aa_mapworld::restore::{RestoredComponent, RestoredRecord};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};

fn evidence(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-owner-actions")
        .join(name)
}
fn mask(world: &SavedActions<'_, '_, '_>, id: ActorId) -> u64 {
    world.actor_fields(id).unwrap().action_bits
}
fn capture(world: &SavedActions<'_, '_, '_>, name: &str) -> Value {
    let owner = world.context.owner;
    let fields = world.actor_fields(owner).unwrap();
    let groups: Vec<_> = (0..world.organization_group_count(owner).unwrap())
        .map(|i| {
            // Capture source inputs for the exact locked-group scan. Unlocked
            // groups never dereference their parameters in578A30(locked_only=1).
            let locked = world.organization_group_locked(owner, i).unwrap();
            let abilities = if locked {
                (0..world.organization_group_ability_count(owner, i).unwrap())
                    .map(|j| world.organization_group_ability_flags(owner, i, j).unwrap())
                    .collect::<Vec<_>>()
            } else {
                vec![]
            };
            json!({"locked":locked,"abilities":abilities})
        })
        .collect();
    let members: Vec<_> = world
        .context
        .organization
        .as_deref()
        .unwrap()
        .members
        .iter()
        .map(|id| (!id.is_null()).then(|| json!({"id":id.raw(),"actions":mask(world,*id)})))
        .collect();
    let element = match world
        .context
        .owner_actor
        .components
        .get(ComponentSlot::Element)
    {
        None => None,
        Some(RestoredComponent::Element(e)) => Some(
            json!({"owner":e.header.owner.raw(),"definition":e.element_definition.is_some(),"parent":e.organization.map(|id|id.raw()),"ready":e.flag_1c}),
        ),
        _ => panic!("Element"),
    };
    let owner_id = world.context.owner_actor.words[&0xe8];
    let kingdom218 = if owner_id == 0 {
        None
    } else {
        Some(world.kingdom_word_218(ActorId::from_raw(owner_id)).unwrap())
    };
    let hqs = if owner_id == 0 {
        vec![]
    } else {
        let Some(RestoredRecord::Kingdom(kingdom)) = world
            .context
            .registry
            .get(ActorId::from_raw(owner_id))
            .map(|entry| &entry.value)
        else {
            panic!("Kingdom")
        };
        kingdom
            .runtime
            .as_ref()
            .unwrap()
            .derived
            .as_ref()
            .unwrap()
            .vectors[&0x2dc]
            .values
            .clone()
    };
    let resupply: Vec<_> = (0..42).map(|i| world.state_resupply(i).ok()).collect();
    json!({"name":name,"owner":owner.raw(),"definition":world.context.owner_definition.proof.id,
        "flags":fields.thing_flags,"state":fields.state_bits,"actions":fields.action_bits,
        "components":world.context.owner_actor.components.iter().map(|(slot,_)|slot as u8).collect::<Vec<_>>(),
        "members":members,"selected":world.context.organization.as_deref().unwrap().words[&0x44],
        "reference_d0":world.context.organization.as_deref().unwrap().words[&0xd0],
        "reference_valid":world.valid_reference(ActorId::from_raw(world.context.organization.as_deref().unwrap().words[&0xd0])).unwrap(),
        "groups":groups,"element":element,"owner_id":owner_id,"kingdom218":kingdom218,"hqs":hqs,
        "resupply":resupply,"object4c":world.context.owner_object_byte_4c,"renderer_link":world.context.owner_thing_word_c,
        "typed_build_list":world.context.owner_actor.actor_list_108.head().is_some()})
}
fn outcome(result: &Result<(), String>) -> &'static str {
    match result {
        Ok(()) => "ok",
        Err(e) if e.contains("76C7F0") => "owner_views_required",
        Err(e) if e.contains("renderer+54") => "renderer_required",
        Err(e) => panic!("unexpected owner boundary: {e}"),
    }
}
fn check(
    mut world: SavedActions<'_, '_, '_>,
    name: &str,
    oracle: Option<&Value>,
    captured: &mut Vec<Value>,
) {
    let pc = world.context.precision;
    let owner = world.context.owner;
    let input = capture(&world, name);
    let before_org = json!(world.context.organization.as_deref().unwrap());
    let list_address = &world.context.owner_actor.actor_list_108 as *const _;
    let result = rebuild(
        world.catalog,
        OrganizationAttachContext {
            inventory: world.context.inventory,
            owner,
            owner_actor: &mut *world.context.owner_actor,
            owner_definition: world.context.owner_definition,
            owner_thing_flags: world.context.owner_thing_flags,
            owner_thing_word_c: world.context.owner_thing_word_c,
            owner_object_byte_4c: world.context.owner_object_byte_4c,
            organization: world.context.organization.as_deref_mut().unwrap(),
            registry: &mut *world.context.registry,
            precision: pc,
        },
    );
    let label = outcome(&result);
    assert_eq!(
        &world.context.owner_actor.actor_list_108 as *const _,
        list_address
    );
    assert_eq!(
        json!(world.context.organization.as_deref().unwrap()),
        before_org
    );
    let output = json!({"outcome":label,"actions":mask(&world,owner),
        "member_actions":world.context.organization.as_deref().unwrap().members.iter().map(|id|(!id.is_null()).then(||mask(&world,*id))).collect::<Vec<_>>()});
    if let Some(oracle) = oracle {
        let rows: Vec<_> = oracle["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["input"] == input && row["precision"] == format!("{pc:?}"))
            .collect();
        assert_eq!(rows.len(), 2, "original/v14 {name}");
        for row in rows {
            assert_eq!(row["output"], output, "input {input}");
        }
    }
    captured.push(json!({"precision":format!("{pc:?}"),"input":input}));
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 data"]
fn actual_saved_owner_action_masks_and_required_publication_match_native() {
    // This is the original, explicitly bounded owner-action fixture: its
    // capture predates Mind/Detection restoration and the remaining recursive
    // member records. Keep that laboratory projection stable as restoration
    // grows. The whole current Actor/Mind path has separate native evidence;
    // this test must not silently extend the old producer's supported inputs.
    let legacy_inputs: Vec<Value> = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-owner-actions/actual-saved-inputs.json"
    ))
    .unwrap();
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut src = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut src, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut src, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut src).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut src).unwrap());
    let token = ScopedManagedLoad::from_sources(&mut src, 0)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(root.join("reconstruction/assets/original/Data/Maps/(2v2)Kursk.RMP"))
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
        aa_mapworld::restore::RestoreMode::InspectPartial,
        LoadBindings {
            native_load_mode: Some(1),
            inactive_load: Some(token),
            resources: Some(Arc::clone(&resources)),
        },
    )
    .unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.native_load_mode = attempt.native_load_mode;
    catalog.settings = Some(attempt.settings.clone());
    catalog.bind_resource_catalog(resources).unwrap();
    catalog
        .bind_inactive_load(attempt.inactive_load.as_ref().unwrap())
        .unwrap();
    let capture_mode = std::env::var_os("AA_OWNER_ACTIONS_CAPTURE").is_some();
    let oracle: Option<Value> = (!capture_mode).then(|| {
        serde_json::from_slice(&std::fs::read(evidence("native-owner-actions.json")).unwrap())
            .unwrap()
    });
    let ids: Vec<_> = attempt
        .registry
        .iter()
        .filter_map(|entry| {
            let actor = entry.value.loaded()?.shell.actor.as_ref()?;
            actor
                .components
                .present(ComponentSlot::Organization)
                .then_some(entry.id)
        })
        .take(6)
        .collect();
    assert_eq!(ids.len(), 6);
    let mut captured = vec![];
    let kingdom = attempt
        .registry
        .iter()
        .find_map(|entry| matches!(&entry.value, RestoredRecord::Kingdom(_)).then_some(entry.id))
        .unwrap();
    for (ordinal, owner) in ids.into_iter().enumerate() {
        let record = std::mem::replace(
            attempt.registry.value_mut(owner).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Actor(mut entity) = record else {
            panic!("Actor")
        };
        let actor = entity.shell.actor.as_mut().unwrap();
        let Some(RestoredComponent::Organization(mut org)) =
            actor.components.remove(ComponentSlot::Organization)
        else {
            panic!("Organization")
        };
        let legacy_name = format!("kursk_{:x}", owner.raw());
        let legacy = &legacy_inputs
            .iter()
            .find(|row| row["input"]["name"] == legacy_name)
            .expect("frozen original owner input")["input"];
        assert_eq!(
            legacy["components"],
            json!([ComponentSlot::Properties as u8])
        );
        let slots: Vec<_> = actor.components.iter().map(|(slot, _)| slot).collect();
        assert_eq!(
            slots,
            vec![
                ComponentSlot::Mind,
                ComponentSlot::Properties,
                ComponentSlot::Detection
            ],
            "newly restored slots require a separately proven whole-context fixture"
        );
        let held_mind = actor.components.remove(ComponentSlot::Mind).unwrap();
        let held_detection = actor.components.remove(ComponentSlot::Detection).unwrap();
        let original_members = org.members.clone();
        let legacy_members = legacy["members"].as_array().unwrap();
        assert_eq!(org.members.len(), legacy_members.len());
        for (member, expected) in org.members.iter_mut().zip(legacy_members) {
            if expected.is_null() {
                *member = ActorId::NULL;
            } else {
                assert_eq!(json!(member.raw()), expected["id"]);
            }
        }
        let original = [actor.words[&0xf0], actor.words[&0xf4], actor.words[&0xe8]];
        let original_state = actor.components.state;
        for pc in [
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ] {
            let names = if ordinal == 0 {
                vec![
                    "actual",
                    "probe_dead",
                    "probe_state26",
                    "probe_state36",
                    "probe_state25",
                    "probe_unchanged",
                    "probe_owner_views",
                    "probe_renderer",
                    "probe_object4c_negative",
                    "probe_empty_kingdom",
                ]
            } else {
                vec!["actual"]
            };
            for name in names {
                actor.words.insert(0xf0, original[0]);
                actor.words.insert(0xf4, original[1]);
                actor.words.insert(0xe8, original[2]);
                actor.components.state = original_state;
                let mut flags = entity.shell.thing.flags;
                let mut word_c = entity.shell.thing.word_c;
                let mut object4c = entity.shell.object.as_ref().unwrap().byte_4c;
                if name != "actual" {
                    actor.words.insert(0xf0, 0x01234567);
                    actor.words.insert(0xf4, 0x12345678);
                }
                match name {
                    "probe_dead"
                    | "probe_owner_views"
                    | "probe_renderer"
                    | "probe_unchanged"
                    | "probe_object4c_negative" => flags |= 1,
                    "probe_state26" => actor.components.state.low |= 1 << 26,
                    "probe_state36" => actor.components.state.high |= 1 << 4,
                    "probe_state25" => actor.components.state.low |= 1 << 25,
                    _ => {}
                }
                if name == "probe_owner_views" {
                    actor.words.insert(0xe8, kingdom.raw());
                    object4c = 1;
                }
                if name == "probe_empty_kingdom" {
                    actor.words.insert(0xe8, kingdom.raw());
                    object4c = 0;
                    word_c = 0;
                }
                if name == "probe_object4c_negative" {
                    actor.words.insert(0xe8, kingdom.raw());
                    object4c = 0x80;
                    word_c = 0;
                }
                if name == "probe_renderer" {
                    actor.words.insert(0xe8, 0);
                    word_c = 1;
                }
                if name == "probe_unchanged" {
                    actor.words.insert(0xf0, 0);
                    actor.words.insert(0xf4, 0);
                    word_c = 1;
                }
                let label = if name == "actual" {
                    format!("kursk_{:x}", owner.raw())
                } else {
                    name.into()
                };
                let world = SavedActions {
                    catalog: &catalog,
                    context: OrganizationAttachContext {
                        inventory: &attempt.inventory,
                        owner,
                        owner_actor: actor,
                        owner_definition: &entity.definition,
                        owner_thing_flags: flags,
                        owner_thing_word_c: word_c,
                        owner_object_byte_4c: object4c,
                        organization: &mut org,
                        registry: &mut attempt.registry,
                        precision: pc,
                    }
                    .into(),
                    active_mind: None,
                };
                check(world, &label, oracle.as_ref(), &mut captured);
            }
        }
        actor.words.insert(0xf0, original[0]);
        actor.words.insert(0xf4, original[1]);
        actor.words.insert(0xe8, original[2]);
        actor.components.state = original_state;
        assert!(
            actor
                .components
                .insert(ComponentSlot::Mind, held_mind)
                .is_none()
        );
        assert!(
            actor
                .components
                .insert(ComponentSlot::Detection, held_detection)
                .is_none()
        );
        for (member, original) in org.members.iter_mut().zip(original_members) {
            *member = original;
        }
        actor.components.insert(
            ComponentSlot::Organization,
            RestoredComponent::Organization(org),
        );
        *attempt.registry.value_mut(owner).unwrap() = RestoredRecord::Actor(entity);
    }
    if capture_mode {
        std::fs::write(
            evidence("actual-saved-inputs.json"),
            serde_json::to_vec_pretty(&captured).unwrap(),
        )
        .unwrap();
    }
}
