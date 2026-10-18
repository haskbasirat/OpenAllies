use super::*;
use crate::{
    localization::Localization,
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
};
use aa_mapworld::restore::RestoredRecord;
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use aa_simulation::NativeRng;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};

fn evidence(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-saved-leader")
        .join(name)
}
fn outcome(result: &std::result::Result<(), String>) -> &'static str {
    match result {
        Ok(()) => "ok",
        Err(error) if error.contains("required Element is absent") => "missing_element",
        Err(error) if error.contains("Mind virtual +114") => "missing_mind_execution",
        Err(error) => panic!("unexpected adapter boundary: {error}"),
    }
}
fn capture(world: &mut Borrowed<'_, '_, '_>, name: &str, requested: ActorId) -> Value {
    let leader = world.org_word(0x40).unwrap();
    let mut ids: Vec<_> = world
        .context
        .organization
        .members
        .iter()
        .copied()
        .chain([ActorId::from_raw(leader), requested])
        .filter(|id| !id.is_null())
        .collect();
    ids.sort_by_key(|id| id.raw());
    ids.dedup();
    let actors: Vec<_> = ids.into_iter().map(|id| {
        let shell = world.shell(id).unwrap();
        let element = match shell.components.get(ComponentSlot::Element) {
            None => Value::Null,
            Some(RestoredComponent::Element(e)) => json!({"owner":e.header.owner.raw(),"slot":e.slot_index,"attached":e.flag_1c}),
            _ => panic!("Element"),
        };
        json!({"id":id.raw(), "position":[world.actor_position(id,0).unwrap().to_bits(),world.actor_position(id,1).unwrap().to_bits()],"element":element})
    }).collect();
    let formation = world.formation().unwrap().map(|handle| {
        let data = world.catalog.layout_data(world.context.precision).unwrap();
        let lf = data.layout_formation(handle).unwrap();
        json!({"handle":handle,"oriented":world.formation_oriented(handle).unwrap(),
            "offsets":lf.positions.iter().map(|p|p.offset_bits).collect::<Vec<_>>()})
    });
    json!({"name":name,"owner":world.context.owner.raw(),
        "owner_facing":world.context.owner_thing.facing_bits[..2],
        "owner_mind":world.context.owner_actor.components.present(ComponentSlot::Mind),
        "layout":world.context.organization.layout.definition,
        "primary_slot":world.primary_slot().unwrap(),"formation":formation,
        "actors":actors,"members":world.context.organization.members.iter().map(|id|id.raw()).collect::<Vec<_>>(),
        "leader":leader,"requested":requested.raw(),
        "offsets":world.context.organization.offsets_78.iter().map(|p|p.map(|f|f.0)).collect::<Vec<_>>(),
        "rng_state":world.context.rng.state})
}
fn check(
    world: &mut Borrowed<'_, '_, '_>,
    name: &str,
    requested: ActorId,
    oracle: Option<&Value>,
    captured: &mut Vec<Value>,
) -> Value {
    let input = capture(world, name, requested);
    let pc = world.context.precision;
    let count = world.context.native_messages.len();
    let result = aa_movement::borrowed_formation::set_leader(
        world,
        (!requested.is_null()).then_some(requested),
        pc,
    )
    .map_err(|e| e.to_string());
    let output = json!({"outcome":outcome(&result),"leader":world.org_word(0x40).unwrap(),
        "offsets":world.context.organization.offsets_78.iter().map(|p|p.map(|f|f.0)).collect::<Vec<_>>(),
        "rng_state":world.context.rng.state,
        "messages":world.context.native_messages[count..].iter().map(|m|[m.function,m.line]).collect::<Vec<_>>()});
    if let Some(oracle) = oracle {
        let rows: Vec<_> = oracle["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["input"] == input && row["precision"] == format!("{pc:?}"))
            .collect();
        assert_eq!(rows.len(), 2, "original/v14 rows for {name}");
        for row in rows {
            assert_eq!(row["output"], output, "input {input}");
        }
    }
    captured.push(json!({"precision":format!("{pc:?}"),"input":input}));
    output
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source data"]
fn actual_saved_organization_leader_and_retained_offsets_match_native() {
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
    let capture_mode = std::env::var_os("AA_SAVED_LEADER_CAPTURE").is_some();
    let oracle: Option<Value> = (!capture_mode).then(|| {
        serde_json::from_slice(&std::fs::read(evidence("native-saved-leader.json")).unwrap())
            .unwrap()
    });
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
        let Some(RestoredComponent::Organization(mut org)) =
            shell.components.remove(ComponentSlot::Organization)
        else {
            panic!("Organization")
        };
        let address = org.as_ref() as *const _;
        let offsets = org.offsets_78.clone();
        let old = org.words[&0x40];
        let requested = org
            .members
            .iter()
            .copied()
            .find(|id| !id.is_null())
            .unwrap_or(ActorId::NULL);
        for pc in [
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ] {
            org.offsets_78.clone_from(&offsets);
            org.words.insert(0x40, old);
            let mut rng = attempt.rng;
            let mut messages = vec![];
            let mut world = Borrowed {
                catalog: &catalog,
                context: OrganizationLeaderContext {
                    owner,
                    owner_actor: shell,
                    owner_thing: &entity.shell.thing,
                    organization: &mut org,
                    registry: &attempt.registry,
                    rng: &mut rng,
                    native_messages: &mut messages,
                    source_ordinal: 434,
                    precision: pc,
                },
            };
            check(
                &mut world,
                &format!("kursk_{:x}", owner.raw()),
                requested,
                oracle.as_ref(),
                &mut captured,
            );
        }
        assert_eq!(
            org.as_ref() as *const _,
            address,
            "same actual Organization allocation"
        );
        org.offsets_78 = offsets;
        org.words.insert(0x40, old);
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
    // Explicit test substitutions inside actual saved allocations. These are
    // not observed initialized components or completed native Actor Init.
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
    let members: Vec<_> = attempt
        .registry
        .iter()
        .filter_map(|entry| {
            (entry.id != owner && matches!(&entry.value, RestoredRecord::Actor(_)))
                .then_some(entry.id)
        })
        .take(2)
        .collect();
    assert_eq!(members.len(), 2);
    let mut saved_elements = vec![];
    for (slot, member) in members.iter().copied().enumerate() {
        let actor = attempt
            .registry
            .value_mut(member)
            .unwrap()
            .loaded_mut()
            .unwrap()
            .shell
            .actor
            .as_mut()
            .unwrap();
        let mut element = aa_actors::ground_states::ElementState::new(member).unwrap();
        element.slot_index = slot as i32;
        element.flag_1c = true;
        saved_elements.push(actor.components.insert(
            ComponentSlot::Element,
            RestoredComponent::Element(Box::new(element)),
        ));
    }
    let record = std::mem::replace(
        attempt.registry.value_mut(owner).unwrap(),
        RestoredRecord::Loading,
    );
    let RestoredRecord::Actor(mut entity) = record else {
        panic!("Actor")
    };
    let shell = entity.shell.actor.as_mut().unwrap();
    let mind = shell.components.remove(ComponentSlot::Mind);
    let Some(RestoredComponent::Organization(mut org)) =
        shell.components.remove(ComponentSlot::Organization)
    else {
        panic!("Organization")
    };
    let address = org.as_ref() as *const _;
    let old_members = org.members.clone();
    let old_offsets = org.offsets_78.clone();
    let old_words = org.words.clone();
    let old_layout = org.layout.definition.clone();
    assert_eq!(
        catalog
            .layout_data(Precision::Double53)
            .unwrap()
            .layouts
            .iter()
            .find(|l| l.ids == old_layout)
            .unwrap()
            .primary_slot,
        -1
    );
    let data = catalog.layout_data(Precision::Double53).unwrap();
    let oriented = data
        .layouts
        .iter()
        .flat_map(|l| l.formations.iter().map(move |f| (l, f)))
        .find(|(_, f)| {
            f.positions.len() >= 2
                && f.formation
                    .and_then(|i| data.formations.get(i as usize))
                    .is_some_and(|f| f.organized)
        })
        .map(|(l, f)| (l.ids.clone(), f.handle))
        .unwrap();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        for name in [
            "probe_new_leader",
            "probe_same_leader",
            "probe_relative",
            "probe_detached",
            "probe_scatter",
            "probe_rotate",
            "probe_missing_new_element",
        ] {
            org.members = members.clone();
            org.offsets_78 = vec![[F32(3.0f32.to_bits()), F32(4.0f32.to_bits())]; 2];
            org.layout.definition = old_layout.clone();
            org.words.insert(0x74, 0);
            org.words.insert(
                0x40,
                if name == "probe_new_leader" {
                    0
                } else {
                    members[0].raw()
                },
            );
            if name == "probe_scatter" {
                org.members[0] = ActorId::NULL;
            }
            if name == "probe_rotate" {
                org.layout.definition = oriented.0.clone();
                org.words.insert(0x74, oriented.1);
            }
            let first = attempt
                .registry
                .value_mut(members[0])
                .unwrap()
                .loaded_mut()
                .unwrap()
                .shell
                .actor
                .as_mut()
                .unwrap();
            let Some(RestoredComponent::Element(e)) =
                first.components.get_mut(ComponentSlot::Element)
            else {
                panic!("Element")
            };
            e.flag_1c = name != "probe_detached";
            let removed = if name == "probe_missing_new_element" {
                attempt
                    .registry
                    .value_mut(members[1])
                    .unwrap()
                    .loaded_mut()
                    .unwrap()
                    .shell
                    .actor
                    .as_mut()
                    .unwrap()
                    .components
                    .remove(ComponentSlot::Element)
            } else {
                None
            };
            let requested = if name == "probe_new_leader" || name == "probe_same_leader" {
                members[0]
            } else {
                members[1]
            };
            let mut rng = NativeRng::new(0x9182a3b4);
            let mut messages = vec![];
            let mut world = Borrowed {
                catalog,
                context: OrganizationLeaderContext {
                    owner,
                    owner_actor: shell,
                    owner_thing: &entity.shell.thing,
                    organization: &mut org,
                    registry: &attempt.registry,
                    rng: &mut rng,
                    native_messages: &mut messages,
                    source_ordinal: 434,
                    precision: pc,
                },
            };
            let output = check(&mut world, name, requested, oracle, captured);
            assert_eq!(output["leader"], requested.raw());
            assert_eq!(
                output["outcome"],
                if name == "probe_missing_new_element" {
                    "missing_element"
                } else {
                    "ok"
                }
            );
            if name == "probe_scatter" {
                assert_ne!(rng.state, 0x9182a3b4);
            }
            if let Some(element) = removed {
                attempt
                    .registry
                    .value_mut(members[1])
                    .unwrap()
                    .loaded_mut()
                    .unwrap()
                    .shell
                    .actor
                    .as_mut()
                    .unwrap()
                    .components
                    .insert(ComponentSlot::Element, element);
            }
        }
        // Portable identity refusal checks, deliberately outside the native
        // malformed-pointer oracle. The store preceding each missing service
        // stays in the same owned Organization.
        org.words.insert(0x40, 0);
        org.header.owner = members[0];
        let mut rng = attempt.rng;
        let mut messages = vec![];
        let error = set_leader(
            catalog,
            OrganizationLeaderContext {
                owner,
                owner_actor: shell,
                owner_thing: &entity.shell.thing,
                organization: &mut org,
                registry: &attempt.registry,
                rng: &mut rng,
                native_messages: &mut messages,
                source_ordinal: 434,
                precision: pc,
            },
            members[1],
        )
        .unwrap_err();
        assert!(error.contains("owner identity mismatch"));
        assert_eq!(org.words[&0x40], 0);
        org.header.owner = owner;
        let stale = ActorId::from_raw(members[1].raw() ^ 0x0010_0000);
        let error = set_leader(
            catalog,
            OrganizationLeaderContext {
                owner,
                owner_actor: shell,
                owner_thing: &entity.shell.thing,
                organization: &mut org,
                registry: &attempt.registry,
                rng: &mut rng,
                native_messages: &mut messages,
                source_ordinal: 434,
                precision: pc,
            },
            stale,
        )
        .unwrap_err();
        assert!(error.contains("not a current restored Actor"));
        assert_eq!(org.words[&0x40], stale.raw());
        // Source identity is checked at the first actual native definition
        // read, rather than before the native leader store or origin case.
        let alternate = catalog.actors.clone();
        let mut changed = Catalog::new(catalog.actors);
        changed.actors = &alternate;
        org.members = members.clone();
        org.offsets_78 = vec![[F32(3.0f32.to_bits()), F32(4.0f32.to_bits())]; 2];
        org.words.insert(0x40, 0);
        set_leader(
            &changed,
            OrganizationLeaderContext {
                owner,
                owner_actor: shell,
                owner_thing: &entity.shell.thing,
                organization: &mut org,
                registry: &attempt.registry,
                rng: &mut rng,
                native_messages: &mut messages,
                source_ordinal: 434,
                precision: pc,
            },
            members[0],
        )
        .unwrap();
        assert_eq!(org.offsets_78[0], [F32::ZERO; 2]);
        org.offsets_78[0] = [F32(3.0f32.to_bits()), F32(4.0f32.to_bits())];
        let unchanged = org.offsets_78.clone();
        let previous_rng = rng.state;
        let error = set_leader(
            &changed,
            OrganizationLeaderContext {
                owner,
                owner_actor: shell,
                owner_thing: &entity.shell.thing,
                organization: &mut org,
                registry: &attempt.registry,
                rng: &mut rng,
                native_messages: &mut messages,
                source_ordinal: 434,
                precision: pc,
            },
            members[1],
        )
        .unwrap_err();
        assert!(error.contains("source owner changed"));
        assert_eq!(org.words[&0x40], members[1].raw());
        assert_eq!(org.offsets_78, unchanged);
        assert_eq!(rng.state, previous_rng);
    }
    assert_eq!(org.as_ref() as *const _, address);
    org.members = old_members;
    org.offsets_78 = old_offsets;
    org.words = old_words;
    org.layout.definition = old_layout;
    shell.components.insert(
        ComponentSlot::Organization,
        RestoredComponent::Organization(org),
    );
    if let Some(mind) = mind {
        shell.components.insert(ComponentSlot::Mind, mind);
    }
    *attempt.registry.value_mut(owner).unwrap() = RestoredRecord::Actor(entity);
    for (member, old_element) in members.into_iter().zip(saved_elements) {
        let actor = attempt
            .registry
            .value_mut(member)
            .unwrap()
            .loaded_mut()
            .unwrap()
            .shell
            .actor
            .as_mut()
            .unwrap();
        actor.components.remove(ComponentSlot::Element);
        if let Some(element) = old_element {
            actor.components.insert(ComponentSlot::Element, element);
        }
    }
}
