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
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};

fn evidence(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-saved-org-combat")
        .join(name)
}
fn sources(root: &std::path::Path) -> V14Sources {
    V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap()
}
fn capture(world: &mut Borrowed<'_, '_, '_>, name: &str) -> Value {
    let mut members = vec![];
    for id in &world.context.organization.members {
        if id.is_null() {
            members.push(Value::Null);
            continue;
        }
        let performer=Borrowed::performer(world,*id).unwrap().map(|p| {
            json!({"owner":p.state.owner,"value":p.word60,"body":world.health(ActorId::from_raw(p.state.owner)).unwrap().map(|v|v.map(|f|f.0))})
        });
        members.push(json!({"id":id.raw(),"performer":performer}));
    }
    let info = aa_mapworld::restore::RestoreDefinitions::organization_layout_info(
        world.catalog,
        &world.context.organization.layout.definition,
        world.context.precision,
    )
    .unwrap();
    let parameters: Vec<_> = world
        .context
        .organization
        .layout
        .parameters
        .iter()
        .map(|p| {
            p.as_deref().map(
                |id| json!({"id":id,"flags":world.parameter_mask(id).unwrap(),"value":Value::Null}),
            )
        })
        .collect();
    json!({"name":name,"owner":world.context.owner.raw(),"layout":world.context.organization.layout.definition,"groups":info.slot_groups,"parameters":parameters,"members":members,"current":world.context.organization.words[&0x68],"maximum":world.context.organization.words[&0x6c]})
}
fn check_native(
    world: &mut Borrowed<'_, '_, '_>,
    name: &str,
    oracle: Option<&Value>,
    captured: &mut Vec<Value>,
) -> Value {
    let pc = world.context.precision;
    let input = capture(world, name);
    let result = aa_combat::organization_combat_value::recompute(world, pc);
    let error = result.as_ref().err().map(ToString::to_string);
    if let Some(error) = &error {
        assert!(error.contains("1E4"), "unexpected source failure: {error}");
    }
    let output = json!({"outcome":if error.is_some(){"required_source_failure"}else{"ok"},"current":world.context.organization.words[&0x68],"maximum":world.context.organization.words[&0x6c]});
    if let Some(oracle) = oracle {
        let rows: Vec<_> = oracle["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["input"] == input && r["precision"] == format!("{pc:?}"))
            .collect();
        assert_eq!(rows.len(), 2, "both original/v14 rows for {name}");
        for row in rows {
            assert_eq!(row["output"], output, "{input}");
        }
    }
    captured.push(json!({"precision":format!("{pc:?}"),"input":input}));
    output
}

/// Explicitly mutated adapter-storage probes on actual saved allocations. The
/// substituted values/membership are test inputs, never completed native Init.
fn storage_probes(
    registry: &mut aa_actors::IdentityRegistry<RestoredRecord>,
    catalog: &Catalog<'_>,
    owner: ActorId,
    oracle: Option<&Value>,
    captured: &mut Vec<Value>,
) {
    let member = registry
        .iter()
        .find_map(|entry| {
            entry.value.loaded()?.shell.actor.as_ref()?;
            (entry.id != owner).then_some(entry.id)
        })
        .expect("actual saved Actor slot for explicitly substituted test Performer");
    assert_ne!(member, owner);
    let bit5 = catalog
        .actors
        .actors
        .iter()
        .find_map(|actor| {
            catalog
                .canonical_component_mask(&actor.id, Precision::Double53)
                .ok()
                .filter(|mask| mask & 0x20 != 0)
                .map(|_| actor.id.clone())
        })
        .expect("canonical bit5 source parameter");
    let bit5_clear = "horse".to_string();
    assert_eq!(
        catalog
            .canonical_component_mask(&bit5_clear, Precision::Double53)
            .unwrap()
            & 0x20,
        0
    );
    let member_shell = registry
        .value_mut(member)
        .unwrap()
        .loaded_mut()
        .unwrap()
        .shell
        .actor
        .as_mut()
        .unwrap();
    // Every value below is an explicit test input. The current partial Kursk
    // load has not reached concrete saved Performers; this is not an Init receipt.
    let saved_performer = member_shell.components.insert(
        ComponentSlot::Performer,
        RestoredComponent::Performer(aa_combat::constructors::OwnedPerformer {
            state: aa_combat::performer::Performer {
                owner: member.raw(),
                stage: 3,
                abilities: vec![],
                mana: F32::ZERO,
                target_id: 0,
                target_position: [F32::ZERO; 2],
                active_ability: None,
                active_index: -1,
                started_at: F32::ZERO,
                animation_length: F32::ZERO,
                event_kind: 0,
                animation: None,
                turret_index: -1,
            },
            maximum_mana: F32::ZERO,
            turrets: vec![],
            turret_update_pending: false,
            word60: 10.0f32.to_bits(),
        }),
    );
    let Some(RestoredComponent::Performer(performer)) =
        member_shell.components.get(ComponentSlot::Performer)
    else {
        unreachable!()
    };
    let performer_slot = performer as *const _;
    let saved_body = member_shell.components.remove(ComponentSlot::Body);
    let record = std::mem::replace(registry.value_mut(owner).unwrap(), RestoredRecord::Loading);
    let RestoredRecord::Actor(mut entity) = record else {
        panic!("Actor")
    };
    let shell = entity.shell.actor.as_mut().unwrap();
    let Some(RestoredComponent::Organization(mut org)) =
        shell.components.remove(ComponentSlot::Organization)
    else {
        panic!("Organization")
    };
    let allocation = org.as_ref() as *const _;
    let original_members = std::mem::take(&mut org.members);
    let original_parameters = org.layout.parameters.clone();
    let original_words = org.words.clone();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        for name in [
            "probe_nonzero_no_body",
            "probe_nonzero_body",
            "probe_late_unbound",
            "probe_null_vacancy",
            "probe_bit5_clear_vacancy",
        ] {
            let member_shell = registry
                .value_mut(member)
                .unwrap()
                .loaded_mut()
                .unwrap()
                .shell
                .actor
                .as_mut()
                .unwrap();
            member_shell.components.remove(ComponentSlot::Body);
            if name != "probe_nonzero_no_body" {
                let mut body = aa_actors::ground_states::BodyState::new(
                    member,
                    F32(4.0f32.to_bits()),
                    F32::ZERO,
                )
                .unwrap();
                body.body.health = F32(3.0f32.to_bits());
                member_shell
                    .components
                    .insert(ComponentSlot::Body, RestoredComponent::Body(Box::new(body)));
            }
            org.members = vec![member];
            org.layout.parameters.clone_from(&original_parameters);
            if name.contains("vacancy") || name == "probe_late_unbound" {
                org.members.push(ActorId::NULL);
                org.layout.parameters[0] = match name {
                    "probe_late_unbound" => Some(bit5.clone()),
                    "probe_bit5_clear_vacancy" => Some(bit5_clear.clone()),
                    _ => None,
                };
            }
            org.words.insert(0x68, 123.0f32.to_bits());
            org.words.insert(0x6c, 456.0f32.to_bits());
            let mut world = Borrowed {
                catalog,
                context: OrganizationCombatContext {
                    owner,
                    owner_actor: shell,
                    organization: &mut org,
                    registry,
                    precision: pc,
                },
            };
            let output = check_native(&mut world, name, oracle, captured);
            assert_eq!(
                output["current"],
                if name == "probe_nonzero_no_body" {
                    10.0f32.to_bits()
                } else {
                    7.5f32.to_bits()
                }
            );
            assert_eq!(output["maximum"], 10.0f32.to_bits());
            assert_eq!(
                output["outcome"],
                if name == "probe_late_unbound" {
                    "required_source_failure"
                } else {
                    "ok"
                }
            );
        }
        // Portable identity errors have no native pointer-fault timing claim.
        let stale = ActorId::from_raw(member.raw() ^ 0x10000);
        assert!(registry.get(stale).is_none());
        org.members = vec![member, stale];
        let result = recompute(
            catalog,
            OrganizationCombatContext {
                owner,
                owner_actor: shell,
                organization: &mut org,
                registry,
                precision: pc,
            },
        );
        assert!(result.unwrap_err().contains("not a current restored Actor"));
        assert_eq!(org.words[&0x68], 7.5f32.to_bits());
        assert_eq!(org.words[&0x6c], 10.0f32.to_bits());
        org.members = vec![member];
        let member_shell = registry
            .value_mut(member)
            .unwrap()
            .loaded_mut()
            .unwrap()
            .shell
            .actor
            .as_mut()
            .unwrap();
        let Some(RestoredComponent::Performer(p)) =
            member_shell.components.get_mut(ComponentSlot::Performer)
        else {
            unreachable!()
        };
        p.state.owner = owner.raw();
        let result = recompute(
            catalog,
            OrganizationCombatContext {
                owner,
                owner_actor: shell,
                organization: &mut org,
                registry,
                precision: pc,
            },
        );
        assert!(result.unwrap_err().contains("Performer belongs"));
        assert_eq!(org.words[&0x68], 0);
        assert_eq!(org.words[&0x6c], 0);
        let member_shell = registry
            .value_mut(member)
            .unwrap()
            .loaded_mut()
            .unwrap()
            .shell
            .actor
            .as_mut()
            .unwrap();
        let Some(RestoredComponent::Performer(p)) =
            member_shell.components.get_mut(ComponentSlot::Performer)
        else {
            unreachable!()
        };
        p.state.owner = member.raw();
        let Some(RestoredComponent::Body(body)) =
            member_shell.components.get_mut(ComponentSlot::Body)
        else {
            unreachable!()
        };
        body.header.owner = owner;
        let result = recompute(
            catalog,
            OrganizationCombatContext {
                owner,
                owner_actor: shell,
                organization: &mut org,
                registry,
                precision: pc,
            },
        );
        assert!(result.unwrap_err().contains("Body belongs"));
        assert_eq!(org.words[&0x68], 0);
        assert_eq!(org.words[&0x6c], 0);
        org.header.owner = member;
        org.words.insert(0x68, 99.0f32.to_bits());
        org.words.insert(0x6c, 98.0f32.to_bits());
        let result = recompute(
            catalog,
            OrganizationCombatContext {
                owner,
                owner_actor: shell,
                organization: &mut org,
                registry,
                precision: pc,
            },
        );
        assert!(result.unwrap_err().contains("active Organization owner"));
        assert_eq!(org.words[&0x68], 99.0f32.to_bits());
        assert_eq!(org.words[&0x6c], 98.0f32.to_bits());
        org.header.owner = owner;
    }
    assert_eq!(org.as_ref() as *const _, allocation);
    org.members = original_members;
    org.layout.parameters = original_parameters;
    org.words = original_words;
    shell.components.insert(
        ComponentSlot::Organization,
        RestoredComponent::Organization(org),
    );
    *registry.value_mut(owner).unwrap() = RestoredRecord::Actor(entity);
    let member_shell = registry
        .value_mut(member)
        .unwrap()
        .loaded_mut()
        .unwrap()
        .shell
        .actor
        .as_mut()
        .unwrap();
    let Some(RestoredComponent::Performer(p)) =
        member_shell.components.get_mut(ComponentSlot::Performer)
    else {
        unreachable!()
    };
    assert_eq!(p as *const _, performer_slot);
    member_shell.components.remove(ComponentSlot::Performer);
    if let Some(performer) = saved_performer {
        member_shell
            .components
            .insert(ComponentSlot::Performer, performer);
    }
    member_shell.components.remove(ComponentSlot::Body);
    if let Some(body) = saved_body {
        member_shell.components.insert(ComponentSlot::Body, body);
    }
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 data"]
fn actual_saved_organization_cells_and_partial_errors_match_native() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut src = sources(&root);
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
    let mut ids: Vec<_> = attempt
        .registry
        .iter()
        .filter_map(|entry| {
            let org = entry
                .value
                .loaded()?
                .shell
                .actor
                .as_ref()?
                .components
                .get(ComponentSlot::Organization)?;
            match org {
                RestoredComponent::Organization(o) if !o.members.is_empty() => Some(entry.id),
                _ => None,
            }
        })
        .collect();
    ids.sort_by_key(|id| {
        let shell = attempt
            .registry
            .get(*id)
            .unwrap()
            .value
            .loaded()
            .unwrap()
            .shell
            .actor
            .as_ref()
            .unwrap();
        let Some(RestoredComponent::Organization(org)) =
            shell.components.get(ComponentSlot::Organization)
        else {
            unreachable!()
        };
        std::cmp::Reverse(org.members.iter().any(|member| {
            attempt
                .registry
                .get(*member)
                .and_then(|e| e.value.loaded())
                .and_then(|e| e.shell.actor.as_ref())
                .and_then(|s| s.components.get(ComponentSlot::Performer))
                .is_some()
        }))
    });
    ids.truncate(6);
    assert_eq!(
        ids.len(),
        6,
        "six actual Kursk partial Organization sources"
    );
    let capture_mode = std::env::var_os("AA_ORG_COMBAT_CAPTURE").is_some();
    let oracle: Option<Value> = (!capture_mode).then(|| {
        serde_json::from_slice(&std::fs::read(evidence("native-saved-org-combat.json")).unwrap())
            .unwrap()
    });
    let catalog = Catalog::new(&actors);
    let mut captured = vec![];
    let probe_owner = attempt
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
        .expect("actual two-slot-same-group source layout for storage probes");
    for id in ids {
        let record = std::mem::replace(
            attempt.registry.value_mut(id).unwrap(),
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
        let allocation = org.as_ref() as *const _;
        for pc in [
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ] {
            let mut world = Borrowed {
                catalog: &catalog,
                context: OrganizationCombatContext {
                    owner: id,
                    owner_actor: shell,
                    organization: &mut org,
                    registry: &attempt.registry,
                    precision: pc,
                },
            };
            check_native(
                &mut world,
                &format!("kursk_{:x}", id.raw()),
                oracle.as_ref(),
                &mut captured,
            );
        }
        assert_eq!(
            org.as_ref() as *const _,
            allocation,
            "actual Organization allocation retained"
        );
        shell.components.insert(
            ComponentSlot::Organization,
            RestoredComponent::Organization(org),
        );
        *attempt.registry.value_mut(id).unwrap() = RestoredRecord::Actor(entity);
    }
    storage_probes(
        &mut attempt.registry,
        &catalog,
        probe_owner,
        oracle.as_ref(),
        &mut captured,
    );
    if capture_mode {
        std::fs::write(
            evidence("actual-saved-inputs.json"),
            serde_json::to_vec_pretty(&captured).unwrap(),
        )
        .unwrap();
    }
}
