//! Actual saved allocations with explicit numeric/modifier probes; no battle claim.
use aa_actors::components::ComponentSlot as Slot;
use aa_body::{
    F32,
    property_effects::{NumericKind, NumericModifier},
};
use aa_mapworld::restore::{RestoreMode, RestoredComponent as Component, RestoredRecord};
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
    load_property_instances::{self as properties, Fields, State},
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
use std::sync::Arc;

fn values(catalog: &Catalog<'_>, record: &Record<'_>) -> Value {
    let a = record.shell.actor.as_ref().unwrap();
    let body = match a.components.get(Slot::Body) {
        Some(Component::Body(b)) => json!([b.body.health.0, b.body.maximum.0, b.body.defense.0]),
        _ => Value::Null,
    };
    let supply = match a.components.get(Slot::Supply) {
        Some(Component::Supply(s)) => json!([s.supply_range.0, s.byte_14 as u32]),
        _ => Value::Null,
    };
    let detection = match a.components.get(Slot::Detection) {
        Some(Component::Detection(d)) => json!(d.radius.0),
        _ => Value::Null,
    };
    let element = match a.components.get(Slot::Element) {
        Some(Component::Element(e)) => json!(e.word_20),
        _ => Value::Null,
    };
    let organization = match a.components.get(Slot::Organization) {
        Some(Component::Organization(o)) => json!([
            o.words.get(&0x88).copied(),
            o.words.get(&0xac).copied(),
            o.bytes.get(&0xb0).map(|v| *v as u32),
            o.definition_value_308.0,
            o.words.get(&0x5c).copied(),
            o.words.get(&0x60).copied()
        ]),
        _ => Value::Null,
    };
    let defender = match a.components.get(Slot::Defender) {
        Some(Component::Defender(d)) => json!([d.word_2c, d.byte_30 as u32]),
        _ => Value::Null,
    };
    let mut abilities = Vec::new();
    for slot in [Slot::Performer, Slot::Defender] {
        let handles = match a.components.get(slot) {
            Some(Component::Performer(p)) => p.state.abilities.as_slice(),
            Some(Component::Defender(d)) => d.abilities.as_slice(),
            _ => &[],
        };
        abilities.extend(handles.iter().map(|h|json!({"handle":h,"slot":slot.index(),"power":catalog.ability_arena.borrow().get(*h).unwrap().power.0})));
    }
    json!({"body":body,"supply":supply,"detection":detection,"element":element,"organization":organization,"defender":defender,"abilities":abilities})
}
fn source(catalog: &Catalog<'_>, record: &Record<'_>) -> Value {
    let pc = catalog
        .definition_bank
        .borrow()
        .combat_source()
        .unwrap()
        .source_precision();
    let ground = catalog
        .actors
        .native_ground_definition(&record.definition.proof.id, pc)
        .unwrap();
    let field = |name: &str| {
        catalog
            .actors
            .native_field(&record.definition.proof.id, name, pc)
            .ok()
            .flatten()
            .and_then(|v| v.fixed_bits().ok())
    };
    let mut abilities = vec![];
    for slot in [Slot::Performer, Slot::Defender] {
        let handles = match record.shell.actor.as_ref().unwrap().components.get(slot) {
            Some(Component::Performer(p)) => p.state.abilities.as_slice(),
            Some(Component::Defender(d)) => d.abilities.as_slice(),
            _ => &[],
        };
        for handle in handles {
            let arena = catalog.ability_arena.borrow();
            let ability = arena.get(*handle).unwrap();
            let bank = catalog.definition_bank.borrow();
            let name = bank
                .entry(ability.local_definition.unwrap())
                .unwrap()
                .key
                .name
                .clone();
            drop(bank);
            drop(arena);
            let (id, index, defender) = if let Some((id, index)) = name.rsplit_once("/ability/") {
                (id, index, false)
            } else {
                let (id, index) = name.rsplit_once("/defender/").unwrap();
                (id, index, true)
            };
            let index = index.parse::<usize>().unwrap();
            let binding = catalog.actor_bindings(id, pc).unwrap();
            let defender_source;
            let loaded = if defender {
                defender_source = catalog
                    .actors
                    .native_loaded_defender(
                        id,
                        pc,
                        binding
                            .character
                            .as_ref()
                            .map(|v| v as &dyn aa_rules::abilities::AnimationResolver),
                    )
                    .unwrap();
                &defender_source.as_ref().unwrap().abilities()[index]
            } else {
                &binding.performer.as_ref().unwrap().abilities()[index]
            };
            let local = aa_runtime::combat_value_source::ability_input(loaded).unwrap();
            abilities.push(json!({"handle":handle,"slot":slot.index(),"power":local.definition.power_64.0,"projectile":local.definition.projectile_3c}));
        }
    }
    json!({"body":ground.attributes.body.as_ref().map(|v|[v.health.bits,v.defense.bits]),"supply":record.definition.ground.supply.as_ref().map(|v|v.supply_range_bits),
  "detection":ground.detection.as_ref().map(|v|v.finalized_range_bits),"resupply":field("ElementComponent.resupply_rate"),"morale_breakpoint":field("OrganizationComponent.morale_breakpoint"),
  "org_control":field("OrganizationComponent.control_range"),"defender_control":field("DefenderComponent.control_range"),"mask":record.definition.component_mask,"abilities":abilities})
}
struct Current<'a>(&'a mut aa_runtime::load_view_variables::State);
impl properties::Effects for Current<'_> {
    fn minimum_multiplier(&mut self) -> aa_body::Result<F32> {
        Ok(F32(self.0.modifier_clamp_multiply_bits()))
    }
}
#[test]
#[ignore = "requires actual original/v14 assets and verified source completion"]
fn actual_multiply_cell_and_numeric_consequences_all_pc() {
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
    let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let mut attempt = Attempt::load_with_source_setup(
        &map,
        &actors,
        settings.clone(),
        text,
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
    catalog.settings = Some(settings);
    catalog.native_load_mode = Some(1);
    catalog.bind_resource_catalog(resources).unwrap();
    *catalog.definition_bank.borrow_mut() = attempt.definitions;
    *catalog.ability_arena.borrow_mut() = attempt.abilities;

    let inventory = aa_mapworld::WorldInventory::decode(&map, u64::MAX).unwrap();
    let mut variables = aa_runtime::load_view_variables::State::reset_from_fresh_source(
        &catalog,
        aa_runtime::load_view_variables::NativeOverrides::ControlledNone,
    )
    .unwrap();
    let rules = aa_runtime::load_world_rules::Owner::load(&catalog, &inventory).unwrap();
    variables.publish_loaded_rules(&catalog, &rules).unwrap();
    assert_eq!(variables.modifier_clamp_multiply_bits(), 0.05f32.to_bits());
    assert_eq!(
        variables
            .combat_values()
            .bits(aa_runtime::load_combat_variables::Field::ModifierClampAdd),
        1f32.to_bits()
    );
    let cells: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-property-numeric/native-modifier-variables.json"
    ))
    .unwrap();
    for row in cells["rows"].as_array().unwrap() {
        let current = row["output"]["current"].as_array().unwrap();
        variables.assign_combat_bits(
            aa_runtime::load_combat_variables::Field::ModifierClampAdd,
            current[0].as_u64().unwrap() as u32,
        );
        variables.assign_modifier_clamp_multiply_bits(current[1].as_u64().unwrap() as u32);
        assert_eq!(
            variables
                .combat_values()
                .bits(aa_runtime::load_combat_variables::Field::ModifierClampAdd),
            current[0].as_u64().unwrap() as u32
        );
        assert_eq!(
            variables.modifier_clamp_multiply_bits(),
            current[1].as_u64().unwrap() as u32
        );
    }
    variables.assign_combat_bits(
        aa_runtime::load_combat_variables::Field::ModifierClampAdd,
        1f32.to_bits(),
    );
    variables.assign_modifier_clamp_multiply_bits(0.05f32.to_bits());
    let mut allocations = State::from_world_constructor();
    allocations.bind_registered_sources(&catalog).unwrap();
    let infantry = allocations
        .definitions()
        .keys()
        .copied()
        .find(|h| allocations.facts(*h).unwrap().id() == "infantry")
        .unwrap();
    let property = allocations
        .facts(infantry)
        .unwrap()
        .reference()
        .handle()
        .unwrap();
    let ids = attempt
        .registry
        .iter()
        .filter_map(|e| matches!(e.value, RestoredRecord::Kingdom(_)).then_some(e.id))
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 3);
    let clock = attempt.clock.begin.0;
    let mut provided = vec![];
    for (i, id) in ids.into_iter().enumerate() {
        let mut held = std::mem::replace(
            attempt.registry.value_mut(id).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Kingdom(e) = &mut held else {
            unreachable!()
        };
        let runtime = e.runtime.as_mut().unwrap();
        let mut record = Record {
            owner: id,
            shell: &mut runtime.base,
            definition: &runtime.definition,
            registry: &mut attempt.registry,
        };
        let pc = [
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ][i];
        let minimum = if i == 1 {
            0.9f32.to_bits()
        } else {
            0.05f32.to_bits()
        };
        variables.assign_modifier_clamp_multiply_bits(minimum);
        let before = match record
            .shell
            .actor
            .as_ref()
            .unwrap()
            .components
            .get(Slot::Properties)
        {
            Some(Component::Properties(p)) => serde_json::to_value(p).unwrap(),
            _ => unreachable!(),
        };
        let input = json!({"owner":id.raw(),"id":record.definition.proof.id,"native_class":record.definition.native_class,
    "components":record.shell.actor.as_ref().unwrap().components.iter().map(|(s,_)|s.index()).collect::<Vec<_>>(),"allegiance":record.shell.actor.as_ref().unwrap().words.get(&0xe8),
    "organization":Value::Null,"clock":clock,"phase":"provided_clamp","property":infantry.0,"kind":0,"source_id":0,"minimum_multiply":minimum,
    "pass_members":allocations.definitions().keys().map(|h|(h.0.to_string(),allocations.facts(*h).unwrap().pass_down_members())).collect::<std::collections::BTreeMap<_,_>>(),
    "before":{"properties":before,"instances":allocations.instances(),"constructors":[]},"definitions":allocations.definitions()});
        properties::add(
            &catalog,
            &mut record,
            &mut allocations,
            property,
            0,
            None,
            || Ok(clock),
            pc,
            &mut Current(&mut variables),
        )
        .unwrap();
        let Some(Component::Properties(p)) = record
            .shell
            .actor
            .as_ref()
            .unwrap()
            .components
            .get(Slot::Properties)
        else {
            unreachable!()
        };
        assert_eq!(p.modifier.damage_factors.len(), 1);
        assert_eq!(
            p.modifier.damage_factors[0].factor.0,
            if i == 1 {
                0.9f32.to_bits()
            } else {
                0x3f4c_cccc
            }
        );
        provided.push(json!({"owner":id.raw(),"definition":record.definition.proof.id,"precision":format!("{pc:?}"),"minimum_multiply":minimum,"minimum_add":variables.combat_values().bits(aa_runtime::load_combat_variables::Field::ModifierClampAdd),"before":before,"after":p,"input":input,"output":{"properties":p,"instances":allocations.instances(),"constructors":[]}}));
        drop(record);
        *attempt.registry.value_mut(id).unwrap() = held;
    }
    // Scoped direct callbacks alter the SAME real modifier as explicit probes;
    // no fake property instance or duplicate ActorStore is introduced.
    let probes = [
        (NumericKind::Add, 0, Slot::Performer),
        (NumericKind::Add, 1, Slot::Performer),
        (NumericKind::Add, 2, Slot::Performer),
        (NumericKind::Add, 0, Slot::Defender),
        (NumericKind::Add, 1, Slot::Defender),
        (NumericKind::Add, 2, Slot::Defender),
        (NumericKind::Add, 3, Slot::Body),
        (NumericKind::Add, 5, Slot::Supply),
        (NumericKind::Add, 6, Slot::Detection),
        (NumericKind::Add, 8, Slot::Organization),
        (NumericKind::Multiply, 14, Slot::Element),
        (NumericKind::Multiply, 19, Slot::Body),
        (NumericKind::Multiply, 20, Slot::Defender),
        (NumericKind::Multiply, 20, Slot::Organization),
        (NumericKind::Multiply, 21, Slot::Organization),
        (NumericKind::Multiply, 22, Slot::Organization),
        (NumericKind::Add, 7, Slot::Organization),
        (NumericKind::Multiply, 13, Slot::Mover),
        (NumericKind::Add, 4, Slot::Properties),
        (NumericKind::Add, 25, Slot::Properties),
        (NumericKind::Multiply, 0, Slot::Properties),
        (NumericKind::Add, 5, Slot::Properties),
    ];
    let mut rows = vec![];
    for (kind, index, slot) in probes {
        let id = attempt
            .registry
            .iter()
            .find_map(|e| match &e.value {
                RestoredRecord::Actor(a)
                    if a.shell.actor.as_ref().is_some_and(|a| {
                        a.components.present(Slot::Properties)
                            && a.components.present(slot)
                            && match a.components.get(slot) {
                                Some(Component::Performer(p)) => !p.state.abilities.is_empty(),
                                Some(Component::Defender(d)) => !d.abilities.is_empty(),
                                _ => true,
                            }
                    }) =>
                {
                    Some(e.id)
                }
                _ => None,
            })
            .unwrap();
        let mut held = std::mem::replace(
            attempt.registry.value_mut(id).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Actor(e) = &mut held else {
            unreachable!()
        };
        let record = Record {
            owner: id,
            shell: &mut e.shell,
            definition: &e.definition,
            registry: &mut attempt.registry,
        };
        for pc in [
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ] {
            let before = values(&catalog, &record);
            let definition = source(&catalog, &record);
            let source_pc = catalog
                .definition_bank
                .borrow()
                .combat_source()
                .unwrap()
                .source_precision();
            let actor = record.shell.actor.as_mut().unwrap();
            let (p, components) = actor.components.split_slot(Slot::Properties);
            let Some(Component::Properties(p)) = p else {
                unreachable!()
            };
            if index < 25 {
                p.modifier.scalars[index as usize] = F32(if kind == NumericKind::Add {
                    2.5f32.to_bits()
                } else {
                    1.25f32.to_bits()
                });
            }
            let modifier = p.modifier.scalars.map(|v| v.0);
            let mut fields=Fields::saved(record.definition,pc,id,
                &mut actor.words,components,record.registry);
            let missing_sources = slot == Slot::Properties;
            if missing_sources && index == 5 {
                assert!(!fields.components.present(Slot::Supply));
            }
            let input = json!({"owner":id.raw(),"id":record.definition.proof.id,"kind":kind,"index":index,"source":definition,"source_precision":format!("{source_pc:?}"),"missing_callback_sources":missing_sources,"before":before,"modifier":modifier,"linked":fields.words.get(&0x168),"present":aa_actors::components::ALL_SLOTS.iter().filter(|s|fields.components.present(**s)).map(|s|s.index()).collect::<Vec<_>>(),"clamp_add":variables.combat_values().bits(aa_runtime::load_combat_variables::Field::ModifierClampAdd)});
            let unbound_catalog = Catalog::new(&actors);
            let result = load_property_numeric::changed(
                if missing_sources {
                    &unbound_catalog
                } else {
                    &catalog
                },
                &mut fields,
                &NumericModifier {
                    kind,
                    index,
                    amount: F32::ONE,
                    repair: false,
                },
                p,
                allocations.instances_mut(),
                &mut variables,
            );
            let boundary = if kind == NumericKind::Add && index == 7 {
                assert!(result.unwrap_err().to_string().contains("578300"));
                Some("morale")
            } else if kind == NumericKind::Multiply && index == 13 {
                assert!(result.unwrap_err().to_string().contains("576C70"));
                Some("mover")
            } else {
                assert!(result.is_ok(), "{}: {:?}", index, result.err());
                None
            };
            drop(fields);
            rows.push(json!({"precision":format!("{pc:?}"),"input":input,"output":values(&catalog,&record),"boundary":boundary}));
        }
        drop(record);
        *attempt.registry.value_mut(id).unwrap() = held;
    }
    let report = json!({"scope":"actual saved numeric direct probes and provided current SVar; no full property-effect/battle acceptance","sources":actors.sources,"provided_multiply":provided,"rows":rows});
    if let Some(path) = std::env::var_os("AA_PROPERTY_NUMERIC_CAPTURE") {
        let file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file, &report).unwrap();
    } else {
        let expected: Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-property-numeric/actual-inputs.json"
        ))
        .unwrap();
        assert_eq!(report, expected);
        let native: Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-property-numeric/native-numeric.json"
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
            assert_eq!(actual["input"], proved["input"]);
            assert_eq!(actual["precision"], proved["precision"]);
            assert_eq!(actual["output"], proved["output"]["state"]);
            assert_eq!(actual["boundary"], proved["output"]["boundary"]);
        }
        for actual in report["provided_multiply"].as_array().unwrap() {
            let matches = native["provided"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|proved| proved["input"] == actual["input"])
                .collect::<Vec<_>>();
            assert_eq!(matches.len(), 3);
            for proved in matches {
                assert_eq!(actual["output"], proved["output"]["state"]);
                assert_eq!(proved["output"]["boundary"], Value::Null);
            }
        }
    }
}
