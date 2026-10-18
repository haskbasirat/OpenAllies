use super::*;
use crate::body_init_fields::Fields;
use aa_actors::{
    creation::{ActorCreationRecord, CreationRecord, ThingDefinition},
    loading::construct_shell_with_components,
};
use aa_body::F32;
use serde_json::{json, Value};

#[test]
fn shared_held_and_stored_body_fields_match_all216_native_prefixes() {
    let native: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-load-component-init/native-body-prefix.json"
    ))
    .unwrap();
    for row in native["rows"].as_array().unwrap() {
        let c = &row["input"];
        let owner = ActorId::from_raw(1);
        let mut shell =
            construct_shell_with_components::<(), RuntimeComponent>(LoadKind::Actor, ());
        let actor = shell.actor.as_mut().unwrap();
        actor.components.state.low = c["state_low"].as_u64().unwrap() as u32;
        let mut body = BodyState::new(
            owner,
            F32(c["maximum_bits"].as_u64().unwrap() as u32),
            F32::ONE,
        )
        .unwrap();
        body.body.health = F32(c["health_bits"].as_u64().unwrap() as u32);
        actor
            .components
            .insert(Slot::Body, RuntimeComponent::Body(Box::new(body)));
        if !c["construction"].is_null() {
            let mut value = aa_actors::construction_state::ConstructionState::new(
                owner,
                &aa_actors::construction_state::Definition {
                    base_cost_words: None,
                    empty_predecessor_chain: true,
                },
            )
            .unwrap();
            value.definition_28 = c["construction"].as_bool().unwrap().then_some(1);
            actor.components.insert(
                Slot::Construction,
                RuntimeComponent::Construction(Box::new(value)),
            );
        }
        if c["properties"].as_bool().unwrap() {
            actor.components.insert(
                Slot::Properties,
                RuntimeComponent::Properties(Box::new(
                    aa_body::properties::PropertiesState::new(
                        aa_body::credit::Object(owner.raw()),
                        2,
                    )
                    .unwrap(),
                )),
            );
        }
        //The portable shell is only a test host for the stored projection's
        //same component table type. It is not a production conversion path.
        struct Table<'a> {
            actor: &'a mut aa_actors::loading::ActorShell<RuntimeComponent>,
            owner: ActorId,
        }
        impl Fields for Table<'_> {
            fn state_low(&self) -> Result<u32> {
                Ok(self.actor.components.state.low)
            }
            fn health(&self) -> Result<u32> {
                let Some(RuntimeComponent::Body(v)) = self.actor.components.get(Slot::Body) else {
                    panic!()
                };
                Ok(v.body.health.0)
            }
            fn maximum(&self) -> Result<u32> {
                let Some(RuntimeComponent::Body(v)) = self.actor.components.get(Slot::Body) else {
                    panic!()
                };
                Ok(v.body.maximum.0)
            }
            fn construction_target(&self) -> Result<Option<u32>> {
                let Some(RuntimeComponent::Construction(v)) =
                    self.actor.components.get(Slot::Construction)
                else {
                    return Ok(None);
                };
                assert_eq!(v.header.owner, self.owner);
                Ok(if self.actor.components.state.low & (1 << 29) != 0 {
                    v.definition_28
                } else {
                    None
                })
            }
            fn update_maximum(&mut self, _: u32) -> Result<()> {
                Err(error("561D40 required"))
            }
            fn store_health(&mut self, b: u32) -> Result<()> {
                let Some(RuntimeComponent::Body(v)) = self.actor.components.get_mut(Slot::Body)
                else {
                    panic!()
                };
                v.body.health = F32(b);
                Ok(())
            }
            fn set_health(&mut self, _: u32) -> Result<()> {
                Err(error("561E50 required"))
            }
        }
        let mut host = Table {
            actor: shell.actor.as_mut().unwrap(),
            owner,
        };
        let result = crate::body_init_fields::initialize(&mut host);
        let outcome = match result {
            Ok(true) => "complete",
            Ok(false) => panic!(),
            Err(e) if e.to_string().contains("561D40") => "required_561D40",
            Err(e) if e.to_string().contains("561E50") => "required_561E50",
            Err(e) => panic!("{e}"),
        };
        assert_eq!(outcome, row["native"]["outcome"]);
        assert_eq!(json!(host.health().unwrap()), row["native"]["health_bits"]);
        assert_eq!(
            json!(host.maximum().unwrap()),
            row["native"]["maximum_bits"]
        );
    }
}

#[test]
#[ignore = "requires actual AA_GAME_ROOT; explicit source component-constructor laboratory, not whole fresh map Init"]
fn actual_fresh_body_uses_same_box_and_moved_bank_all_pc() {
    use crate::{localization::Localization, settings::Settings};
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use std::sync::Arc;
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut files, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut files, Limits::default()).unwrap();
    let source = ActorCatalog::load_v14(&mut files).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut files).unwrap());
    let mut catalog = Catalog::new(&source);
    catalog.settings = Some(settings);
    catalog.definition_text = Some(text.clone());
    catalog.replace_current_text(text);
    catalog.bind_resource_catalog(resources).unwrap();
    let receipt = crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(
        &catalog, &mut files,
    )
    .unwrap();
    crate::combat_source_completion::install(
        &catalog,
        &mut files,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    let bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    assert!(catalog.definition_bank.borrow().combat_source().is_err());
    assert_eq!(
        bank.combat_source().unwrap().source_precision(),
        Precision::Double53
    );
    let mut rows = vec![];
    for name in ["usa_corps", "gbr_corps", "ger_bunker"] {
        let binding = catalog.actor_bindings(name, Precision::Double53).unwrap();
        let mask = binding.definition.component_mask.unwrap();
        assert_ne!(mask & 1, 0);
        let record = ActorCreationRecord {
            base: CreationRecord {
                definition: ThingDefinition {
                    key: name.into(),
                    flags: binding.definition.flags,
                },
                position_bits: [0; 3],
                direction_bits: [0, (-1f32).to_bits()],
                name_utf16: vec![],
                saved_id: ActorId::NULL,
            },
            variant: 0,
            owner_word: 0,
            word_30: 0,
            request_nonblocking: false,
        };
        let mut actors = ActorStore::new();
        let mut rng = aa_simulation::NativeRng::new(0x12345678);
        let key = actors
            .construct_new_bound(
                &catalog,
                binding,
                record,
                0,
                &mut rng,
                Precision::Double53,
                Precision::Double53,
            )
            .unwrap();
        let owner = actors
            .initialize_object(key, &mut rng, Precision::Double53)
            .unwrap();
        //Only the source-present slots reached by562290 are constructed. This
        //is an explicit component-constructor lab, not completed Actor assembly.
        for slot in [Slot::Body, Slot::Construction, Slot::Properties] {
            if mask & (1 << slot.index()) != 0 {
                actors
                    .construct_ground(key, slot, F32::ZERO, Precision::Double53)
                    .unwrap();
            }
        }
        let before = body(&actors, owner).unwrap().clone();
        let pointer = body(&actors, owner).unwrap() as *const _ as usize;
        let object_pointer = actors.object(key).unwrap() as *const _ as usize;
        let rng_before = rng.state;
        assert_eq!(
            before.body.health, before.body.maximum,
            "genuine native constructor state, never overwritten for test"
        );
        let state = actors.object(key).unwrap().components().unwrap().state.low;
        for pc in [
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ] {
            assert!(init_in_store(&catalog, &mut actors, &bank, key, pc).unwrap());
            after_init(&catalog, &actors, &bank, key).unwrap();
            assert_eq!(body(&actors, owner).unwrap(), &before);
            assert_eq!(body(&actors, owner).unwrap() as *const _ as usize, pointer);
            assert_eq!(
                actors.object(key).unwrap() as *const _ as usize,
                object_pointer
            );
            assert_eq!(
                actors.object(key).unwrap().components().unwrap().state.low,
                state
            );
            assert_eq!(rng.state, rng_before);
            rows.push(json!({"actor":name,"live_precision":pc,"source_precision":"Double53","owner":owner.raw(),"state_low":state,"health_bits":before.body.health.0,"maximum_bits":before.body.maximum.0,"complete":true}));
            //A labelled damaged-field probe, separate from the genuine fresh
            //constructor state above. The native prefix must put max back
            //into the SAME box before refusing its nonconstant health child.
            let Some(RuntimeComponent::Body(value)) =
                actors.component_mut(owner, Slot::Body).unwrap()
            else {
                panic!()
            };
            value.body.health = F32::ZERO;
            let failed = init_in_store(&catalog, &mut actors, &bank, key, pc).unwrap_err();
            assert!(failed.to_string().contains("561E50"));
            assert!(failed
                .to_string()
                .contains("preceding health=max store retained"));
            assert_eq!(body(&actors, owner).unwrap(), &before);
            assert_eq!(body(&actors, owner).unwrap() as *const _ as usize, pointer);
            assert_eq!(rng.state, rng_before);
            rows.push(json!({"actor":name,"probe":"explicit damaged-health input, not natural constructor state","live_precision":pc,"source_precision":"Double53","owner":owner.raw(),"state_low":state,"health_bits":0,"maximum_bits":before.body.maximum.0,"outcome":"required_561E50","stored_health_bits":body(&actors,owner).unwrap().body.health.0}));
        }
        assert!(init_in_store(
            &catalog,
            &mut actors,
            &Bank::default(),
            key,
            Precision::Double53
        )
        .is_err());
        assert_eq!(body(&actors, owner).unwrap(), &before);
        let flags = actors.object(key).unwrap().definition.flags;
        actors.object_mut(key).unwrap().definition.flags ^= 1;
        assert!(init_in_store(&catalog, &mut actors, &bank, key, Precision::Double53).is_err());
        assert_eq!(body(&actors, owner).unwrap(), &before);
        actors.object_mut(key).unwrap().definition.flags = flags;
    }
    if let Some(path) = std::env::var_os("AA_STORED_BODY_INIT_CAPTURE") {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .unwrap();
        f.write_all(serde_json::to_string_pretty(&json!({"scope":"actual source-bound Body/Construction/Properties component-constructor lab, not initialized Staged or full Actor Init","rows":rows})).unwrap().as_bytes()).unwrap();
    }
    println!(
        "{} actual source-bound Body component calls, same boxes and moved Bank",
        rows.len()
    );
}
