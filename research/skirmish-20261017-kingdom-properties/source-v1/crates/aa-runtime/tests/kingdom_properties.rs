//! Explicit direct grants on real saved owners; not a natural mission grant.
use aa_actors::{ActorId, components::ComponentSlot as Slot};
use aa_body::{F32, property_effects::Instances};
use aa_mapworld::restore::{
    RestoreDefinitions, RestoreMode, RestoredComponent as Component, RestoredRecord,
};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use aa_runtime::{
    catalog::Catalog,
    live_object_init::Record,
    load_kingdom_properties::{self as kingdom, Context, RequiredNotifications},
    load_property_instances::{self as property, State},
    load_property_numeric,
};
use aa_runtime::{
    combat_source_completion as completion,
    combat_source_receipt::VerifiedStartupReceipt,
    localization::Localization,
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};

struct Effects<'a>(&'a mut aa_runtime::load_view_variables::State);
impl property::Effects for Effects<'_> {
    fn minimum_multiplier(&mut self) -> aa_body::Result<F32> {
        Ok(F32(self.0.modifier_clamp_multiply_bits()))
    }
    fn numeric_changed(
        &mut self,
        catalog: &Catalog<'_>,
        fields: &mut property::Fields<'_>,
        modifier: &aa_body::property_effects::NumericModifier,
        p: &mut aa_body::properties::PropertiesState,
        i: &mut Instances,
    ) -> aa_body::Result<()> {
        load_property_numeric::changed(catalog, fields, modifier, p, i, self.0)
    }
}
fn snapshot(record: &Record<'_>, owner: ActorId, state: &State) -> Value {
    let RestoredRecord::Kingdom(k) = &record.registry.get(owner).unwrap().value else {
        unreachable!()
    };
    let d = k.runtime.as_ref().unwrap().derived.as_ref().unwrap();
    let v = &d.vectors[&0x318];
    let actor = record.shell.actor.as_ref().unwrap();
    let Some(Component::Properties(p)) = actor.components.get(Slot::Properties) else {
        unreachable!()
    };
    let Some(Component::Body(body)) = actor.components.get(Slot::Body) else {
        unreachable!()
    };
    json!({"kingdom_records":v.values,"capacity":v.capacity_hint,"actors":d.actor_lists.as_ref().unwrap().by_class[3],"instances":state.instances(),"constructors":state.constructors().iter().map(|(h,p)|(h.0.to_string(),json!({"definition":p.definition,"kind":p.kind,"source_id":p.source_id}))).collect::<BTreeMap<_,_>>(),"properties":p,"body":[body.body.health.0,body.body.maximum.0,body.body.defense.0]})
}
fn profiles(
    record: &Record<'_>,
    owner: ActorId,
    loaded: &aa_runtime::restriction_groups::LoadedRestrictionGroups,
) -> Value {
    let RestoredRecord::Kingdom(k) = &record.registry.get(owner).unwrap().value else {
        unreachable!()
    };
    let d = k.runtime.as_ref().unwrap().derived.as_ref().unwrap();
    let team = match d.identity_references.get(&0x1ec) {
        Some(v) => *v,
        None => {
            assert_eq!(d.words[&0x1ec], 0);
            None
        }
    };
    let team_profile = team.map(|id| {
        let RestoredRecord::Team(t) = &record.registry.get(id).unwrap().value else {
            unreachable!()
        };
        t.runtime
            .as_ref()
            .unwrap()
            .derived
            .as_ref()
            .unwrap()
            .strings_utf16[&0x64]
            .clone()
    });
    json!({"kingdom":d.strings_utf16[&0x348],"team":team_profile,"profiles":loaded.profiles().profiles.iter().map(|(name,p)|json!({"name":name,"technologies":p.technologies})).collect::<Vec<_>>()})
}
#[test]
#[ignore = "requires actual original/v14 assets and source completion"]
fn actual_kingdom_property_allocations_and_effects_all_pc() {
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
    let localization = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let inventory = aa_mapworld::WorldInventory::decode(&map, u64::MAX).unwrap();
    let row = map
        .directory()
        .iter()
        .find(|v| v.name == "RESTRICTION_GROUPS")
        .unwrap();
    let chunk = map
        .decode_chunk("RESTRICTION_GROUPS", row.decoded_bytes)
        .unwrap();
    let mut rows = vec![];
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
            .unwrap()
            .begin(TaskKind::Normal)
            .unwrap();
        let mut attempt = Attempt::load_with_source_setup(
            &map,
            &actors,
            settings.clone(),
            localization.clone(),
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
        catalog.settings = Some(settings.clone());
        *catalog.localization.borrow_mut() = Some(localization.clone());
        catalog.native_load_mode = Some(1);
        catalog.bind_resource_catalog(resources.clone()).unwrap();
        *catalog.definition_bank.borrow_mut() = attempt.definitions;
        *catalog.ability_arena.borrow_mut() = attempt.abilities;
        let mut variables = aa_runtime::load_view_variables::State::reset_from_fresh_source(
            &catalog,
            aa_runtime::load_view_variables::NativeOverrides::ControlledNone,
        )
        .unwrap();
        let rules = aa_runtime::load_world_rules::Owner::load(&catalog, &inventory).unwrap();
        variables.publish_loaded_rules(&catalog, &rules).unwrap();
        let restrictions = aa_runtime::restriction_groups::LoadedRestrictionGroups::load(
            &chunk,
            &catalog,
            aa_mapworld::restriction_groups::LocalizedStorage::KeyOnly,
            Precision::Double53,
        )
        .unwrap();
        let mut state = State::from_world_constructor();
        state.bind_registered_sources(&catalog).unwrap();
        let actor_id = attempt
            .registry
            .iter()
            .find_map(|e| match &e.value {
                RestoredRecord::Actor(a) if a.definition.proof.id == "weurope_farm6x6_01" => {
                    Some(e.id)
                }
                _ => None,
            })
            .unwrap();
        let mut held = std::mem::replace(
            attempt.registry.value_mut(actor_id).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Actor(a) = &mut held else {
            unreachable!()
        };
        let mut record = Record {
            owner: actor_id,
            shell: &mut a.shell,
            definition: &a.definition,
            registry: &mut attempt.registry,
        };
        let owner = ActorId::from_raw(record.shell.actor.as_ref().unwrap().words[&0xe8]);
        assert!(!owner.is_null());
        //Real4E6180 prefix inserts this actual held Actor in its real stored owner.
        //A later unrelated required service is retained as explicit preparation data.
        let preparation = aa_runtime::load_owner_add_actor::add_actor(
            &catalog,
            &inventory,
            &mut record,
            owner,
            pc,
        );
        let snap = snapshot(&record, owner, &state);
        assert_eq!(
            snap["actors"].as_array().unwrap(),
            &vec![json!(actor_id.raw())]
        );
        let clock = attempt.clock.begin.0;
        //Explicitly grant actual intrinsic definitions through the real5817A0 path.
        //This supplies dependency records for subsequent direct technology probes;
        //no invented modifiers, instance handles or fake success callbacks are used.
        for name in ["vehicle", "infantry", "hq"] {
            let key = state
                .definitions()
                .keys()
                .copied()
                .find(|h| state.facts(*h).unwrap().id() == name)
                .unwrap();
            let reference = state.facts(key).unwrap().reference().handle().unwrap();
            property::add(
                &catalog,
                &mut record,
                &mut state,
                reference,
                0,
                None,
                || Ok(clock),
                pc,
                &mut Effects(&mut variables),
            )
            .unwrap();
        }
        let definitions = state.definitions().clone();
        let mut facts = BTreeMap::new();
        let data = catalog.layout_data(Precision::Double53).unwrap();
        for key in definitions.keys() {
            let f = state.facts(*key).unwrap();
            let aa_rules::native::layout_data::PropertyRef::Registered(i) = f.reference() else {
                unreachable!()
            };
            facts.insert(key.0.to_string(),json!({"id":f.id(),"required":f.required(),"technology":data.properties[i as usize].is_technology.unwrap(),"pass_members":f.pass_down_members()}));
        }
        let base = catalog
            .actors
            .native_ground_definition(&record.definition.proof.id, Precision::Double53)
            .unwrap()
            .attributes
            .body
            .unwrap();
        let positive = [
            "assembly_parts",
            "deployment_crates",
            "ger_infantry_supplies",
            "rus_infantry_supplies",
        ];
        let mut grants = vec![];
        for source_name in positive {
            let source = catalog
                .resolve(
                    &source_name.encode_utf16().collect::<Vec<_>>(),
                    Precision::Double53,
                )
                .unwrap();
            for token in
                aa_runtime::definition_technology_properties::bind(&catalog, &source).unwrap()
            {
                grants.push((source_name.to_owned(), token));
            }
        }
        assert_eq!(grants.len(), 4);
        let mobile = state
            .definitions()
            .keys()
            .copied()
            .find(|h| state.facts(*h).unwrap().id() == "mobile")
            .unwrap()
            .0;
        grants.push(("explicit ordinary Property notify probe".into(), mobile));
        for (ordinal, (source_name, definition)) in grants.into_iter().enumerate() {
            for phase in ["first", "duplicate", "missing_clock"] {
                let before = snapshot(&record, owner, &state);
                let notify = ordinal == 4 && phase == "first";
                let input = json!({"definition":definition,"source_actor_definition":source_name,"kind":10,"origin":actor_id.raw(),"notify":notify,"clock":clock,"phase":phase,"owner":owner.raw(),"actor":actor_id.raw(),"actor_definition":record.definition.proof.id,"position":record.shell.thing.position_bits,"components":record.shell.actor.as_ref().unwrap().components.iter().map(|(s,_)|s.index()).collect::<Vec<_>>(),"allegiance":record.shell.actor.as_ref().unwrap().words[&0xe8],"linked":record.shell.actor.as_ref().unwrap().words[&0x168],"base_health":base.health.bits,"base_defense":base.defense.bits,"clamp_add":variables.combat_values().bits(aa_runtime::load_combat_variables::Field::ModifierClampAdd),"clamp_multiply":variables.modifier_clamp_multiply_bits(),"before":before,"definitions":definitions,"facts":facts,"profile_lookup":profiles(&record,owner,&restrictions)});
                let mut now = || {
                    if phase == "missing_clock" {
                        Err("explicit current World+E0 required clock boundary".into())
                    } else {
                        Ok(clock)
                    }
                };
                let result = kingdom::add(
                    &catalog,
                    &mut record,
                    owner,
                    definition,
                    10,
                    Some(actor_id),
                    notify,
                    &mut Context {
                        instances: &mut state,
                        effects: &mut Effects(&mut variables),
                        //Ordinary SharedProperty byte5C=false skips4E1890;
                        //a missing profile owner must remain genuinely lazy.
                        restrictions: if ordinal == 4 {
                            None
                        } else {
                            Some(&restrictions)
                        },
                        current_time: &mut now,
                        notifications: &mut RequiredNotifications,
                    },
                    pc,
                );
                let boundary = if phase == "missing_clock" {
                    assert!(result.unwrap_err().contains("clock"));
                    Some("missing_clock")
                } else if notify {
                    assert!(result.unwrap_err().contains("GameEventManager"));
                    Some("notification")
                } else {
                    let first = result.unwrap();
                    assert_eq!(first, phase == "first" && ordinal != 3);
                    None
                };
                rows.push(json!({"precision":format!("{pc:?}"),"preparation_error":preparation.as_ref().err(),"input":input,"output":snapshot(&record,owner,&state),"boundary":boundary}));
            }
        }
        drop(record);
        *attempt.registry.value_mut(actor_id).unwrap() = held;
    }
    let report = json!({"scope":"actual saved farm/current Kingdom and actual four positive source-list definitions; explicit direct grants and real intrinsic preparation, not naturally reached mission technology","source_precision":"Double53","sources":actors.sources,"rows":rows});
    if let Some(path) = std::env::var_os("AA_KINGDOM_PROPERTY_CAPTURE") {
        let file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file, &report).unwrap();
    } else {
        let expected: Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-kingdom-properties/actual-inputs.json"
        ))
        .unwrap();
        assert_eq!(report, expected);
        let native: Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-kingdom-properties/native-kingdom-properties.json"
        ))
        .unwrap();
        assert_eq!(
            report["rows"].as_array().unwrap().len(),
            native["rows"].as_array().unwrap().len()
        );
        for (actual, proved) in report["rows"]
            .as_array()
            .unwrap()
            .iter()
            .zip(native["rows"].as_array().unwrap())
        {
            assert_eq!(actual["precision"], proved["precision"]);
            assert_eq!(actual["input"], proved["input"]);
            assert_eq!(actual["output"], proved["output"]["state"]);
            assert_eq!(actual["boundary"], proved["output"]["boundary"]);
        }
    }
}
