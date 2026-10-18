use super::*;
use crate::{localization::Localization, restoration::Attempt, settings::Settings};
use aa_mapworld::restore::{OrganizationAttachContext, RestoreMode, RestoredRecord};
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn evidence() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-saved-economy-values")
}
fn dense_actor(ids: &BTreeMap<ActorId, u32>, id: Option<ActorId>) -> Option<u32> {
    id.map(|id| {
        *ids.get(&id)
            .expect("captured native reference must have real record")
    })
}

// This is a read-only test export of exact adapter facts into the established
// native resource oracle's typed memory input. It does not fabricate actors.
fn capture(view: &View<'_, '_, '_, '_, '_>, ids: &[ActorId]) -> Value {
    let actor_ids: BTreeMap<_, _> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i as u32 + 1))
        .collect();
    let mut defs = Vec::<String>::new();
    let mut snapshots = Vec::new();
    let mut orgs = BTreeMap::<String, Value>::new();
    let mut modifiers = BTreeMap::<String, Value>::new();
    let mut actor_modifiers = BTreeMap::<String, Value>::new();
    let mut transports = BTreeMap::<String, Value>::new();
    let mut caches = BTreeMap::<String, Value>::new();
    for id in ids {
        let a = view.actor(*id).unwrap();
        if !defs.contains(&a.definition) {
            defs.push(a.definition.clone());
        }
        let key = actor_ids[id].to_string();
        if a.organization.is_some() {
            let mut slots = Vec::<Option<String>>::new();
            for i in 0..view.layout_slot_count(id.raw()).unwrap() {
                let d = view.layout_slot_definition(id.raw(), i).unwrap();
                if let Some(d) = &d {
                    if !defs.contains(d) {
                        defs.push(d.clone());
                    }
                }
                slots.push(d);
            }
            let members: Vec<_> = (0..view.member_count(id.raw()).unwrap())
                .map(|i| dense_actor(&actor_ids, view.member(id.raw(), i).unwrap()))
                .collect();
            orgs.insert(key.clone(), json!({"slots":slots,"members":members}));
        }
        let context = view.actor_modifier(*id).unwrap();
        actor_modifiers.insert(key.clone(), json!(context.map(|_| actor_ids[id])));
        if context.is_some() {
            let mut m = view.modifier(id.raw()).unwrap();
            m.production_adjustments = view.properties(*id).unwrap().modifier.production.clone();
            modifiers.insert(key.clone(), serde_json::to_value(m).unwrap());
        }
        if a.transport.is_some() {
            transports.insert(key.clone(),json!({"Cost":view.transport_vector(id.raw(),VectorKind::Cost).unwrap(),"Upkeep":view.transport_vector(id.raw(),VectorKind::Upkeep).unwrap()}));
        }
        if a.economy.is_some() {
            let e = if *id == view.focused.header.owner {
                &*view.focused
            } else if let Some((_, c)) = view.held.iter().find(|(key, _)| key == id) {
                &**c
            } else {
                match view.component(*id, ComponentSlot::Economy).unwrap() {
                    Some(RestoredComponent::Economy(c)) => c,
                    _ => panic!("Economy"),
                }
            };
            caches.insert(key.clone(), json!(e.resources[3].values));
        }
        let supply = match view.component(*id, ComponentSlot::Supply).unwrap() {
            Some(RestoredComponent::Supply(s)) => Some(s.stockpile.0),
            None => None,
            _ => panic!("Supply"),
        };
        snapshots.push((*id, a, supply));
    }
    let mut definitions = BTreeMap::<String, Value>::new();
    let mut i = 0;
    while i < defs.len() {
        let mut d = view.definition(&defs[i]).unwrap();
        d.word17c = u32::from(view.recruit_backref_present(&defs[i]).unwrap());
        d.packed_definition = view.packed_definition(&defs[i]).unwrap();
        if let Some(p) = &d.packed_definition {
            if !defs.contains(p) {
                defs.push(p.clone());
            }
        }
        let mut v = serde_json::to_value(&d).unwrap();
        v["packed_definition"] = json!(
            d.packed_definition
                .as_ref()
                .map(|p| defs.iter().position(|v| v == p).unwrap() + 1)
        );
        definitions.insert((i + 1).to_string(), v);
        i += 1;
    }
    for org in orgs.values_mut() {
        org["slots"] = json!(
            org["slots"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v
                    .as_str()
                    .map(|s| defs.iter().position(|d| d == s).unwrap() + 1))
                .collect::<Vec<_>>()
        );
    }
    let actors: BTreeMap<_, _> = snapshots
        .into_iter()
        .map(|(id, a, supply)| {
            let mut v = serde_json::to_value(&a).unwrap();
            v["definition"] = json!(defs.iter().position(|d| d == &a.definition).unwrap() + 1);
            v["element_organization"] = json!(dense_actor(&actor_ids, a.element_organization));
            for key in ["organization", "economy", "transport"] {
                v[key] = json!((!v[key].is_null()).then_some(actor_ids[&id]));
            }
            v["constructor18"] = json!(supply);
            (actor_ids[&id].to_string(), v)
        })
        .collect();
    json!({"name":"actual_tutorial02","precision":"Double53","actor":1,"definition":1,"context":null,
        "resource_count":view.resource_count().unwrap(),"actors":actors,"definitions":definitions,"organizations":orgs,
        "modifiers":modifiers,"actor_modifiers":actor_modifiers,"transports":transports,"caches":caches,
        "initial_output":[],"cache_members":false,"operation":"Cost","source_actor_ids":ids.iter().map(|i|i.raw()).collect::<Vec<_>>(),"source_definition_ids":defs})
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source and native captured fixture"]
fn actual_saved_values_and_focused_held_cache_owners_match_native() {
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
        std::fs::read(
            root.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
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
        RestoreMode::InspectPartial,
        Some(1),
    )
    .unwrap();
    let ids: Vec<_> = attempt
        .registry
        .iter()
        .filter_map(|e| e.value.loaded()?.shell.actor.as_ref().map(|_| e.id))
        .collect();
    let owner = *ids
        .iter()
        .find(|id| {
            let a = attempt
                .registry
                .get(**id)
                .unwrap()
                .value
                .loaded()
                .unwrap()
                .shell
                .actor
                .as_ref()
                .unwrap();
            a.components.present(ComponentSlot::Organization)
        })
        .expect("real Organization");
    let economy_ids: Vec<_> = ids
        .iter()
        .filter(|id| {
            **id != owner
                && attempt
                    .registry
                    .get(**id)
                    .unwrap()
                    .value
                    .loaded()
                    .unwrap()
                    .shell
                    .actor
                    .as_ref()
                    .unwrap()
                    .components
                    .present(ComponentSlot::Economy)
        })
        .copied()
        .collect();
    let focused_id = economy_ids[0];
    let held_id = economy_ids[1];
    let taken = std::mem::replace(
        attempt.registry.value_mut(owner).unwrap(),
        RestoredRecord::Loading,
    );
    let RestoredRecord::Actor(mut entity) = taken else {
        panic!("actor")
    };
    let shell = entity.shell.actor.as_mut().unwrap();
    let Some(RestoredComponent::Organization(mut org)) =
        shell.components.remove(ComponentSlot::Organization)
    else {
        panic!("org")
    };
    let Some(RestoredComponent::Economy(mut focused)) = attempt
        .registry
        .value_mut(focused_id)
        .unwrap()
        .loaded_mut()
        .unwrap()
        .shell
        .actor
        .as_mut()
        .unwrap()
        .components
        .remove(ComponentSlot::Economy)
    else {
        panic!("focused")
    };
    let Some(RestoredComponent::Economy(mut held)) = attempt
        .registry
        .value_mut(held_id)
        .unwrap()
        .loaded_mut()
        .unwrap()
        .shell
        .actor
        .as_mut()
        .unwrap()
        .components
        .remove(ComponentSlot::Economy)
    else {
        panic!("held")
    };
    let focus_ptr = &*focused as *const _;
    let held_ptr = &*held as *const _;
    let catalog = Catalog::new(&actors);
    let mut actions = SavedActions {
        catalog: &catalog,
        context: OrganizationAttachContext {
            inventory: &attempt.inventory,
            owner,
            owner_actor: shell,
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
    let mut held_refs = [(held_id, &mut *held)];
    let mut view = View::new(
        &mut actions,
        &mut focused,
        &mut held_refs,
        Precision::Double53,
    );
    assert!(view.actor(focused_id).unwrap().economy.is_some());
    assert!(view.actor(held_id).unwrap().economy.is_some());
    let graph = capture(&view, &ids);
    if std::env::var_os("AA_SAVED_ECONOMY_CAPTURE").is_some() {
        std::fs::write(
            evidence().join("actual-saved-input.json"),
            serde_json::to_vec_pretty(&graph).unwrap(),
        )
        .unwrap();
        println!(
            "captured {} real actors, {} definitions",
            ids.len(),
            graph["definitions"].as_object().unwrap().len()
        );
        return;
    }
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(evidence().join("native-saved-values.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        fixture["input"], graph,
        "actual source and saved fields must match native capture"
    );
    for row in fixture["cases"].as_array().unwrap() {
        // Reset only fixture-owned cache inputs between independent native calls.
        for (key, data) in graph["caches"].as_object().unwrap() {
            let id = ids[key.parse::<usize>().unwrap() - 1];
            let words: Vec<_> = data
                .as_array()
                .unwrap()
                .iter()
                .map(|v| F32(v.as_u64().unwrap() as u32))
                .collect();
            view.store_member_production(id.raw(), &words).unwrap();
        }
        let id = ids[row["actor"].as_u64().unwrap() as usize - 1];
        let op = row["operation"].as_str().unwrap();
        let output = &row["output"];
        match op {
            "upkeep_blocked" => {
                assert_eq!(json!(view.upkeep_blocked(id).unwrap()), output["predicate"])
            }
            "eligible" => assert_eq!(json!(view.provides(id).unwrap()), output["predicate"]),
            "supplied" => assert_eq!(json!(view.supplied(id).unwrap()), output["predicate"]),
            "Cost" | "Upkeep" => assert_eq!(
                json!(
                    view.cost_or_upkeep(
                        id,
                        if op == "Cost" {
                            VectorKind::Cost
                        } else {
                            VectorKind::Upkeep
                        }
                    )
                    .unwrap()
                ),
                output["values"],
                "actor {id:?} {op}"
            ),
            "Production" => assert_eq!(
                json!(view.production(id, true).unwrap()),
                output["values"],
                "actor {id:?} {op}"
            ),
            _ => panic!("{op}"),
        }
        for (key, data) in output["caches"].as_object().unwrap() {
            let actual = &view
                .economy_mut(ids[key.parse::<usize>().unwrap() - 1])
                .unwrap()
                .resources[3]
                .values;
            assert_eq!(json!(actual), *data, "cache {key} actor {id:?} {op}");
        }
    }
    // Exercise the two real allocations directly even if the actual graph's
    // parent production walk does not visit one of these nodes.
    let marker = vec![F32(0x3f800001); view.resource_count().unwrap()];
    view.store_member_production(focused_id.raw(), &marker)
        .unwrap();
    view.store_member_production(held_id.raw(), &marker)
        .unwrap();
    assert_eq!(view.focused.resources[3].values, marker);
    assert_eq!(view.held[0].1.resources[3].values, marker);
    assert_eq!(&*view.focused as *const _, focus_ptr);
    assert_eq!(&*view.held[0].1 as *const _, held_ptr);
    println!(
        "compared {} native saved value cases over {} actors",
        fixture["cases"].as_array().unwrap().len(),
        ids.len()
    );
}
