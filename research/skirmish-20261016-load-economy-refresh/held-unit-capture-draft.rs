use super::*;
use crate::{
    localization::Localization,
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    saved_economy_values::View,
    settings::Settings,
};
use aa_actors::economy_values::{GateWorld, VectorKind, World as _};
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};

fn economy<'a>(record: &'a Record<'_>) -> &'a aa_actors::ground_states::EconomyState {
    match record
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
fn kingdom_snapshot(record: &Record<'_>, id: ActorId) -> Value {
    let RestoredRecord::Kingdom(k) = &record.registry.get(id).unwrap().value else {
        panic!("stored Kingdom")
    };
    let d = k.runtime.as_ref().unwrap().derived.as_ref().unwrap();
    json!({"raw_provided":d.vectors[&0x1ac].values,"effective_provided":d.vectors[&0x1a0].values,"consumed":d.vectors[&0x1b8].values,"stockpiled":d.vectors[&0x1c4].values})
}
#[derive(Default)]
struct Meta {
    supply: Option<u32>,
    cache: Option<Vec<aa_body::F32>>,
    production: Option<Vec<aa_body::F32>>,
}
// A lib-test and its aa-netcode dev dependency contain distinct aa-runtime
// crate identities. Borrow the actual Session fields across that test-only
// boundary; no cached phase, new scalar owner or constant callback is supplied.
struct SessionBorrow<'a>(&'a aa_netcode::lobby::Session);
impl crate::load_world_globals::SessionPhase for SessionBorrow<'_> {
    fn current_phase(&self) -> Result<u32, String> {
        Ok(self.0.field_f8)
    }
    fn application_state(&self) -> Result<u32, String> {
        Ok(self.0.state)
    }
}
fn graph(
    view: &View<'_, '_, '_, '_, '_>,
    ids: &[ActorId],
    meta: &BTreeMap<ActorId, Meta>,
) -> Value {
    let dense: BTreeMap<_, _> = ids.iter().enumerate().map(|(i, id)| (*id, i + 1)).collect();
    let mut defs = Vec::<String>::new();
    let mut actors = BTreeMap::<String, Value>::new();
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
        let key = dense[id].to_string();
        if a.organization.is_some() {
            let slots = (0..view.layout_slot_count(id.raw()).unwrap())
                .map(|i| {
                    let d = view.layout_slot_definition(id.raw(), i).unwrap();
                    if let Some(v) = &d
                        && !defs.contains(v)
                    {
                        defs.push(v.clone());
                    }
                    d
                })
                .collect::<Vec<_>>();
            let members = (0..view.member_count(id.raw()).unwrap())
                .map(|i| view.member(id.raw(), i).unwrap().map(|v| dense[&v]))
                .collect::<Vec<_>>();
            orgs.insert(key.clone(), json!({"slots":slots,"members":members}));
        }
        let context = view.actor_modifier(*id).unwrap();
        actor_modifiers.insert(key.clone(), json!(context.map(|_| dense[id])));
        if context.is_some() {
            let mut m = view.modifier(id.raw()).unwrap();
            m.production_adjustments = meta[id].production.clone().unwrap();
            modifiers.insert(key.clone(), json!(m));
        }
        if a.transport.is_some() {
            transports.insert(key.clone(),json!({"Cost":view.transport_vector(id.raw(),VectorKind::Cost).unwrap(),"Upkeep":view.transport_vector(id.raw(),VectorKind::Upkeep).unwrap()}));
        }
        if let Some(c) = &meta[id].cache {
            caches.insert(key.clone(), json!(c));
        }
        let mut v = json!(a);
        v["element_organization"] = json!(a.element_organization.map(|v| dense[&v]));
        v["constructor18"] = json!(meta[id].supply);
        for field in ["organization", "economy", "transport"] {
            v[field] = json!((!v[field].is_null()).then_some(dense[id]));
        }
        actors.insert(key, v);
    }
    let mut definitions = BTreeMap::<String, Value>::new();
    let mut i = 0;
    while i < defs.len() {
        let mut d = view.definition(&defs[i]).unwrap();
        d.word17c = u32::from(view.recruit_backref_present(&defs[i]).unwrap());
        d.packed_definition = view.packed_definition(&defs[i]).unwrap();
        if let Some(v) = &d.packed_definition
            && !defs.contains(v)
        {
            defs.push(v.clone());
        }
        let mut v = json!(d);
        v["packed_definition"] = json!(
            d.packed_definition
                .as_ref()
                .map(|s| defs.iter().position(|d| d == s).unwrap() + 1)
        );
        definitions.insert((i + 1).to_string(), v);
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
    json!({"name":"actual_immediate_held_economy","precision":"Double53","actor":1,"definition":1,"context":null,"resource_count":view.resource_count().unwrap(),"actors":actors,"definitions":definitions,"organizations":orgs,"modifiers":modifiers,"actor_modifiers":actor_modifiers,"transports":transports,"caches":caches,"initial_output":[],"cache_members":true,"operation":"Cost","source_actor_ids":ids.iter().map(|i|i.raw()).collect::<Vec<_>>(),"source_definition_ids":defs})
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
            &SessionBorrow(&session),
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
        let mut meta = BTreeMap::new();
        for current in &ids {
            let actor = if *current == id {
                record.shell.actor.as_ref().unwrap()
            } else {
                record
                    .registry
                    .get(*current)
                    .unwrap()
                    .value
                    .loaded()
                    .unwrap()
                    .shell
                    .actor
                    .as_ref()
                    .unwrap()
            };
            let mut m = Meta::default();
            for s in [
                ComponentSlot::Supply,
                ComponentSlot::Economy,
                ComponentSlot::Properties,
            ] {
                match actor.components.get(s) {
                    Some(RestoredComponent::Supply(v)) => m.supply = Some(v.stockpile.0),
                    Some(RestoredComponent::Economy(v)) => {
                        m.cache = Some(v.resources[3].values.clone())
                    }
                    Some(RestoredComponent::Properties(v)) => {
                        m.production = Some(v.modifier.production.clone())
                    }
                    None => {}
                    _ => panic!("actual typed component"),
                }
            }
            meta.insert(*current, m);
        }
        let Some(RestoredComponent::Economy(mut focused)) = record
            .shell
            .actor
            .as_mut()
            .unwrap()
            .components
            .remove(ComponentSlot::Economy)
        else {
            unreachable!()
        };
        let (graph, blocked, provides, upkeep, provided, cost) = {
            let object = record.shell.object.as_ref().unwrap();
            let mut actions = SavedActions {
                catalog: &catalog,
                context: SavedActionContext {
                    inventory: &attempt.inventory,
                    owner: id,
                    owner_actor: record.shell.actor.as_mut().unwrap(),
                    owner_definition: record.definition,
                    owner_thing_flags: record.shell.thing.flags,
                    owner_thing_word_c: record.shell.thing.word_c,
                    owner_object_byte_4c: object.byte_4c,
                    organization: None,
                    registry: record.registry,
                    precision: pc,
                    source_precision: Precision::Single24,
                },
                active_mind: None,
            };
            assert_eq!(actions.source_precision().unwrap(), Precision::Double53);
            let mut view = View::new(&mut actions, &mut focused, &mut [], pc);
            let mut graph = graph(&view, &ids, &meta);
            graph["precision"] = json!(pc);
            let blocked = view.upkeep_blocked(id).unwrap();
            let provides = view.provides(id).unwrap();
            let upkeep = if blocked {
                vec![]
            } else {
                view.cost_or_upkeep(id, VectorKind::Upkeep).unwrap()
            };
            let provided = if provides {
                view.production(id, false).unwrap()
            } else {
                vec![]
            };
            let cost = view.cost_or_upkeep(id, VectorKind::Cost).unwrap();
            (graph, blocked, provides, upkeep, provided, cost)
        };
        record
            .shell
            .actor
            .as_mut()
            .unwrap()
            .components
            .insert(ComponentSlot::Economy, RestoredComponent::Economy(focused));
        refresh(&catalog, &attempt.inventory, &mut record, pc).unwrap();
        assert_eq!(economy(&record) as *const _ as usize, pointer);
        let after = component_snapshot(economy(&record));
        let after_kingdom = kingdom_snapshot(&record, owner);
        assert_eq!(before_kingdom["stockpiled"], after_kingdom["stockpiled"]);
        let definitions = catalog
            .native_resource_economy(Precision::Double53)
            .unwrap();
        rows.push(json!({"precision":pc,"actor":id.raw(),"owner":owner.raw(),"definition":record.definition.proof,"source_pc":"Double53","initial_stage_report":attempt.report,"graph":graph,"input":{"name":format!("actual_held_{}_{}",id.raw(),pc.significand_bits()),"precision":pc,"catalog":definitions.catalog(),"kingdoms":[before_kingdom,before_kingdom],"limits":{"provided_maximum":crate::saved_kingdom_flows::provider_maximum(&catalog,pc).unwrap(),"stockpiled_maximum":resources.stockpiled_resource_max.f32_bits},"flags":before["flags"],"resources":before["resources"],"block_reason":if blocked{"nonpaying_organization"}else{"none"},"parent":false,"owners":[0],"provides":[provides],"upkeep":upkeep,"provided":provided,"cost":cost,"mutations":[]},"output":{"component":after,"kingdom":after_kingdom},"same_economy_allocation":true,"flow_reset_in_test":false}));
        println!(
            "{pc:?}: actual immediate Actor{} Economy and storedKingdom{} real flows",
            id.raw(),
            owner.raw()
        );
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
