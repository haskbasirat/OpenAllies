use super::*;
use crate::{
    localization::Localization,
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
};
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
        .join("../../research/skirmish-20261016-saved-actor-properties")
        .join(name)
}
fn values(state: &State<String>) -> Vec<String> {
    let mut result = vec![];
    let mut node = state.head();
    let mut seen = BTreeSet::new();
    while let Some(id) = node {
        assert!(seen.insert(id));
        result.push(state.value(id).unwrap().clone());
        node = state.next(id).unwrap();
    }
    result
}
fn replace(state: &mut State<String>, values: &[&str]) {
    while let Some(node) = state.head() {
        state.set_head(state.next(node).unwrap());
        state.free(node).unwrap();
    }
    state.set_tail(None);
    for value in values.iter().rev() {
        let node = state.allocate().unwrap();
        state.set_value(node, (*value).into()).unwrap();
        state.set_next(node, state.head()).unwrap();
        state.set_head(Some(node));
        if state.tail().is_none() {
            state.set_tail(Some(node));
        }
    }
}
fn graph(world: &mut Borrowed<'_, '_, '_>) -> Vec<ActorId> {
    let mut result = vec![world.context.owner];
    let mut at = 0;
    while at < result.len() {
        let actor = result[at];
        at += 1;
        let mut additional = vec![];
        if world.organization(actor).unwrap().is_some() {
            additional.extend(
                world
                    .org(actor)
                    .unwrap()
                    .members
                    .iter()
                    .copied()
                    .filter(|id| !id.is_null()),
            );
        }
        if let Some(RestoredComponent::Element(element)) = world
            .shell(actor)
            .unwrap()
            .components
            .get(ComponentSlot::Element)
            && let Some(parent) = element.organization
        {
            additional.push(parent);
        }
        for actor in additional {
            if !result.contains(&actor) {
                result.push(actor);
            }
        }
    }
    result
}
fn capture(world: &mut Borrowed<'_, '_, '_>, name: &str) -> (Value, Vec<ActorId>) {
    let ids = graph(world);
    let pc = world.context.precision;
    let source = world.catalog.native_builder_data(pc).unwrap();
    let actors:Vec<_>=ids.iter().map(|id| {
        let state=world.shell(*id).unwrap();let definition=world.source(*id).unwrap();
        let builder=state.components.present(ComponentSlot::Builder);
        let flags=if builder {world.catalog.canonical_component_mask(&definition.proof.id,pc).unwrap()}
            else {definition.component_mask.expect("actual saved ActorData component mask")};
        let source_values=source.definition(&definition.proof.id).unwrap().ids.clone();
        let element=match state.components.get(ComponentSlot::Element) {
            None=>None,Some(RestoredComponent::Element(element))=>Some(element),_=>panic!("Element"),
        };
        let parent=element.and_then(|element|element.organization)
            .map(|parent|ids.iter().position(|id|*id==parent).unwrap());
        let has_org=(*id==world.context.owner && world.context.organization.is_some())
            ||state.components.present(ComponentSlot::Organization);
        let members=has_org.then(|| {
            world.org(*id).unwrap().members.iter().map(|member| {
                (!member.is_null()).then(||ids.iter().position(|id|id==member).unwrap())
            }).collect::<Vec<_>>()
        });
        json!({"gtid":id.raw(),"definition":definition.proof.id,"list":values(&state.actor_list_108),
            "builder":builder,"flags":flags,"source":source_values,"members":members,
            "element":element.is_some(),"parent":parent})
    }).collect();
    (
        json!({"name":name,"operation":"refresh","actor":0,"actors":actors}),
        ids,
    )
}
fn output(world: &Borrowed<'_, '_, '_>, ids: &[ActorId], outcome: &str) -> Value {
    let actors: Vec<_> = ids
        .iter()
        .map(|id| {
            let state = &world.shell(*id).unwrap().actor_list_108;
            json!({"list":values(state),"tail_value":state.tail().map(|id|state.value(id).unwrap()),
            "allocated_nodes":state.allocated_nodes()})
        })
        .collect();
    json!({"outcome":outcome,"actors":actors})
}
fn check(
    world: &mut Borrowed<'_, '_, '_>,
    name: &str,
    oracle: Option<&Value>,
    captured: &mut Vec<Value>,
) {
    let (input, ids) = capture(world, name);
    let pc = world.context.precision;
    let owner = world.context.owner;
    actor_property_list::refresh(world, owner).unwrap();
    let output = output(world, &ids, "ok");
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
#[ignore = "requires AA_GAME_ROOT original/v14 sources"]
fn actual_saved_build_target_lists_and_partial_stores_match_native() {
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
            resources: Some(resources),
        },
    )
    .unwrap();
    let catalog = Catalog::new(&actors);
    let capture_mode = std::env::var_os("AA_ACTOR_PROPERTIES_CAPTURE").is_some();
    let oracle: Option<Value> = (!capture_mode).then(|| {
        serde_json::from_slice(
            &std::fs::read(evidence("native-saved-actor-properties.json")).unwrap(),
        )
        .unwrap()
    });
    let ids: Vec<_> = attempt
        .registry
        .iter()
        .filter_map(|entry| {
            let shell = entry.value.loaded()?.shell.actor.as_ref()?;
            match shell.components.get(ComponentSlot::Organization)? {
                RestoredComponent::Organization(org) if !org.members.is_empty() => Some(entry.id),
                _ => None,
            }
        })
        .take(6)
        .collect();
    assert_eq!(ids.len(), 6);
    let mut captured = vec![];
    for owner in ids {
        let record = std::mem::replace(
            attempt.registry.value_mut(owner).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Actor(mut entity) = record else {
            panic!("Actor")
        };
        let shell = entity.shell.actor.as_mut().unwrap();
        let Some(RestoredComponent::Organization(org)) =
            shell.components.remove(ComponentSlot::Organization)
        else {
            panic!("Organization")
        };
        let list_address = &shell.actor_list_108 as *const _;
        for pc in [
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ] {
            let mut world = Borrowed::new(
                &catalog,
                ActorPropertiesContext {
                    owner,
                    owner_actor: shell,
                    owner_definition: &entity.definition,
                    organization: Some(&org),
                    registry: &mut attempt.registry,
                    precision: pc,
                },
            );
            check(
                &mut world,
                &format!("kursk_{:x}", owner.raw()),
                oracle.as_ref(),
                &mut captured,
            );
        }
        assert_eq!(&shell.actor_list_108 as *const _, list_address);
        shell.components.insert(
            ComponentSlot::Organization,
            RestoredComponent::Organization(org),
        );
        *attempt.registry.value_mut(owner).unwrap() = RestoredRecord::Actor(entity);
    }
    storage_probes(&catalog, &mut attempt, oracle.as_ref(), &mut captured);
    if capture_mode {
        std::fs::write(
            evidence("actual-saved-inputs.json"),
            serde_json::to_vec_pretty(&captured).unwrap(),
        )
        .unwrap();
    }
}

fn storage_probes(
    catalog: &Catalog<'_>,
    attempt: &mut Attempt,
    oracle: Option<&Value>,
    captured: &mut Vec<Value>,
) {
    let owner = attempt
        .registry
        .iter()
        .find_map(|entry| {
            let shell = entry.value.loaded()?.shell.actor.as_ref()?;
            match shell.components.get(ComponentSlot::Organization)? {
                RestoredComponent::Organization(org)
                    if org.layout.definition == "herd_ambient_large" =>
                {
                    Some(entry.id)
                }
                _ => None,
            }
        })
        .unwrap();
    let member = attempt
        .registry
        .iter()
        .find_map(|entry| {
            (entry.id != owner && matches!(&entry.value, RestoredRecord::Actor(_)))
                .then_some(entry.id)
        })
        .unwrap();
    let record = std::mem::replace(
        attempt.registry.value_mut(owner).unwrap(),
        RestoredRecord::Loading,
    );
    let RestoredRecord::Actor(mut entity) = record else {
        panic!("Actor")
    };
    let shell = entity.shell.actor.as_mut().unwrap();
    let Some(RestoredComponent::Organization(mut org)) =
        shell.components.remove(ComponentSlot::Organization)
    else {
        panic!("Organization")
    };
    let previous_members = std::mem::take(&mut org.members);
    org.members = vec![member, ActorId::NULL, member];
    let previous_list = std::mem::replace(&mut shell.actor_list_108, State::new());
    let member_shell = attempt
        .registry
        .value_mut(member)
        .unwrap()
        .loaded_mut()
        .unwrap()
        .shell
        .actor
        .as_mut()
        .unwrap();
    let member_list = std::mem::replace(&mut member_shell.actor_list_108, State::new());
    let list_address = &shell.actor_list_108 as *const _;
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        for name in [
            "probe_clear_member_union",
            "probe_builder_false_mask",
            "probe_builder_source",
            "probe_parent_propagation",
        ] {
            replace(&mut shell.actor_list_108, &["usa_bunker", "usa_bunker_aa"]);
            let member_shell = attempt
                .registry
                .value_mut(member)
                .unwrap()
                .loaded_mut()
                .unwrap()
                .shell
                .actor
                .as_mut()
                .unwrap();
            replace(
                &mut member_shell.actor_list_108,
                &["usa_bunker", "usa_bunker_aa", "usa_bunker"],
            );
            let old_builder = shell.components.remove(ComponentSlot::Builder);
            if name.contains("builder") {
                shell.components.insert(
                    ComponentSlot::Builder,
                    RestoredComponent::Builder(Box::new(
                        aa_actors::builder_state::BuilderState::new(owner),
                    )),
                );
            }
            let mut definition = entity.definition.clone();
            if name == "probe_builder_source" {
                definition.proof.id = "usa_engineer".into();
            }
            let (owner_element, member_element, member_org) = if name == "probe_parent_propagation"
            {
                let mut element = aa_actors::ground_states::ElementState::new(owner).unwrap();
                element.organization = Some(member);
                let owner_element = shell.components.insert(
                    ComponentSlot::Element,
                    RestoredComponent::Element(Box::new(element)),
                );
                let member_shell = attempt
                    .registry
                    .value_mut(member)
                    .unwrap()
                    .loaded_mut()
                    .unwrap()
                    .shell
                    .actor
                    .as_mut()
                    .unwrap();
                let member_element = member_shell.components.remove(ComponentSlot::Element);
                let member_org = member_shell.components.remove(ComponentSlot::Organization);
                (owner_element, member_element, member_org)
            } else {
                (None, None, None)
            };
            let mut world = Borrowed::new(
                catalog,
                ActorPropertiesContext {
                    owner,
                    owner_actor: shell,
                    owner_definition: &definition,
                    organization: Some(&org),
                    registry: &mut attempt.registry,
                    precision: pc,
                },
            );
            check(&mut world, name, oracle, captured);
            if name == "probe_parent_propagation" {
                shell.components.remove(ComponentSlot::Element);
                if let Some(element) = owner_element {
                    shell.components.insert(ComponentSlot::Element, element);
                }
                let member_shell = attempt
                    .registry
                    .value_mut(member)
                    .unwrap()
                    .loaded_mut()
                    .unwrap()
                    .shell
                    .actor
                    .as_mut()
                    .unwrap();
                if let Some(element) = member_element {
                    member_shell
                        .components
                        .insert(ComponentSlot::Element, element);
                }
                if let Some(org) = member_org {
                    member_shell
                        .components
                        .insert(ComponentSlot::Organization, org);
                }
            }
            shell.components.remove(ComponentSlot::Builder);
            if let Some(builder) = old_builder {
                shell.components.insert(ComponentSlot::Builder, builder);
            }
        }
        // Portable missing source/state checks are separate from valid native
        // graph comparisons. Clearing old nodes precedes these boundaries.
        replace(&mut shell.actor_list_108, &["usa_bunker"]);
        let old_builder = shell.components.insert(
            ComponentSlot::Builder,
            RestoredComponent::Builder(Box::new(aa_actors::builder_state::BuilderState::new(
                owner,
            ))),
        );
        let alternate = catalog.actors.clone();
        let mut changed = Catalog::new(catalog.actors);
        changed.actors = &alternate;
        let error = refresh(
            &changed,
            ActorPropertiesContext {
                owner,
                owner_actor: shell,
                owner_definition: &entity.definition,
                organization: Some(&org),
                registry: &mut attempt.registry,
                precision: pc,
            },
        )
        .unwrap_err();
        assert!(error.contains("source owner changed"));
        assert!(values(&shell.actor_list_108).is_empty());
        assert_eq!(shell.actor_list_108.allocated_nodes(), 0);
        shell.components.remove(ComponentSlot::Builder);
        if let Some(builder) = old_builder {
            shell.components.insert(ComponentSlot::Builder, builder);
        }
        let original = org.members[0];
        org.members[0] = ActorId::from_raw(member.raw() ^ 0x0010_0000);
        replace(&mut shell.actor_list_108, &["usa_bunker"]);
        let error = refresh(
            catalog,
            ActorPropertiesContext {
                owner,
                owner_actor: shell,
                owner_definition: &entity.definition,
                organization: Some(&org),
                registry: &mut attempt.registry,
                precision: pc,
            },
        )
        .unwrap_err();
        assert!(error.contains("not a current restored Actor"));
        assert!(values(&shell.actor_list_108).is_empty());
        org.members[0] = original;
        replace(&mut shell.actor_list_108, &["usa_bunker"]);
        let member_shell = attempt
            .registry
            .value_mut(member)
            .unwrap()
            .loaded_mut()
            .unwrap()
            .shell
            .actor
            .as_mut()
            .unwrap();
        replace(
            &mut member_shell.actor_list_108,
            &["usa_bunker", "usa_bunker_aa", "usa_bunker"],
        );
        let mut element = aa_actors::ground_states::ElementState::new(owner).unwrap();
        element.organization = Some(owner);
        let original_element = shell.components.insert(
            ComponentSlot::Element,
            RestoredComponent::Element(Box::new(element)),
        );
        let error = refresh(
            catalog,
            ActorPropertiesContext {
                owner,
                owner_actor: shell,
                owner_definition: &entity.definition,
                organization: Some(&org),
                registry: &mut attempt.registry,
                precision: pc,
            },
        )
        .unwrap_err();
        assert!(error.contains("parent cycle"));
        assert_eq!(
            values(&shell.actor_list_108),
            vec!["usa_bunker_aa", "usa_bunker"]
        );
        shell.components.remove(ComponentSlot::Element);
        if let Some(element) = original_element {
            shell.components.insert(ComponentSlot::Element, element);
        }
    }
    assert_eq!(&shell.actor_list_108 as *const _, list_address);
    shell.actor_list_108 = previous_list;
    org.members = previous_members;
    shell.components.insert(
        ComponentSlot::Organization,
        RestoredComponent::Organization(org),
    );
    *attempt.registry.value_mut(owner).unwrap() = RestoredRecord::Actor(entity);
    attempt
        .registry
        .value_mut(member)
        .unwrap()
        .loaded_mut()
        .unwrap()
        .shell
        .actor
        .as_mut()
        .unwrap()
        .actor_list_108 = member_list;
}
