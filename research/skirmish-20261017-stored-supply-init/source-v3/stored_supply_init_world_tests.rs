use super::*;

#[test]
#[ignore = "requires AA_GAME_ROOT; actual source component-constructor and disjoint-loan laboratory, not whole Actor Init"]
fn actual_zone_loan_keeps_current_and_other_actor_allocations_on_error_all_pc() {
    use crate::{localization::Localization, settings::Settings};
    use aa_actors::creation::{ActorCreationRecord, CreationRecord, ThingDefinition};
    use aa_body::F32;
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

    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let mut actors = ActorStore::new();
        let mut rng = aa_simulation::NativeRng::new(0x12345678);
        let mut keys = Vec::new();
        for name in ["ger_corps", "usa_corps"] {
            let binding = catalog.actor_bindings(name, Precision::Double53).unwrap();
            assert_ne!(
                binding.definition.component_mask.unwrap() & (1 << Slot::Supply.index()),
                0
            );
            let record = ActorCreationRecord {
                base: CreationRecord {
                    definition: ThingDefinition {
                        key: name.into(),
                        flags: binding.definition.flags,
                    },
                    position_bits: [0; 3],
                    direction_bits: [0, (-1f32).to_bits()],
                    name_utf16: Vec::new(),
                    saved_id: ActorId::NULL,
                },
                variant: 0,
                owner_word: 0,
                word_30: 0,
                request_nonblocking: false,
            };
            let key = actors
                .construct_new_bound(
                    &catalog,
                    binding,
                    record,
                    0,
                    &mut rng,
                    Precision::Double53,
                    pc,
                )
                .unwrap();
            let owner = actors.initialize_object(key, &mut rng, pc).unwrap();
            // Only the actual source-present Supply slot is constructed. No
            // InitExisting, owner-Kingdom registration or zone solve is claimed.
            actors
                .construct_ground(key, Slot::Supply, F32::ZERO, pc)
                .unwrap();
            assert_eq!(
                checked(&catalog, &actors, &bank, key).unwrap(),
                (owner, Precision::Double53)
            );
            keys.push((key, owner));
        }
        let (key, owner) = keys[0];
        let (other_key, other) = keys[1];
        let object_address = actors.object(key).unwrap() as *const _ as usize;
        let other_address = actors.object(other_key).unwrap() as *const _ as usize;
        let supply_address = supply(&actors, owner).unwrap() as *const _ as usize;
        let other_supply_address = supply(&actors, other).unwrap() as *const _ as usize;
        let rng_before = rng.state;
        let flags = actors.staged_placement(key).unwrap().flags;
        let mut identities = crate::load_detection_activation::Identities::new();
        crate::load_detection_activation::create_slot_fields(
            owner,
            flags,
            actors
                .object_mut(key)
                .unwrap()
                .actor
                .as_mut()
                .unwrap()
                .zones_148
                .zones_mut(),
            &mut identities,
            0,
        )
        .unwrap();
        let zone_key = actors
            .object(key)
            .unwrap()
            .actor
            .as_ref()
            .unwrap()
            .zones_148
            .zones()
            .slots[0]
            .as_ref()
            .unwrap()
            .key;
        assert_eq!(identities.allocation(zone_key), Some((owner, 0)));

        let failure = actors
            .with_zone_fields(key, 0, |fields, zone| {
                assert!(fields.present(owner));
                assert!(fields.present(other));
                assert!(fields.same_definition_proof(owner).unwrap());
                assert!(fields.same_definition_proof(other).unwrap());
                assert_eq!(fields.owner, owner);
                assert_eq!(fields.zone_key(owner, 0).unwrap(), zone.key);
                assert_eq!(fields.zone_key(other, 0).unwrap(), 0);
                assert_eq!(fields.definition(owner).unwrap().proof.id, "ger_corps");
                assert_eq!(fields.definition(other).unwrap().proof.id, "usa_corps");
                for id in [owner, other] {
                    let RuntimeComponent::Supply(value) =
                        fields.components(id).unwrap().get(Slot::Supply).unwrap()
                    else {
                        panic!();
                    };
                    assert_eq!(value.header.owner, id);
                }
                let RuntimeComponent::Supply(value) =
                    fields.components.get_mut(Slot::Supply).unwrap()
                else {
                    panic!();
                };
                assert_eq!(&**value as *const _ as usize, supply_address);
                // Labelled storage-failure probe: current lookups must observe the
                // actual prefix even while the selected Zone is exclusively borrowed.
                value.byte_14 = 128;
                zone.set_state_bit(2, true);
                assert_eq!(fields.zone_key(owner, 0).unwrap(), zone.key);
                let RuntimeComponent::Supply(value) =
                    fields.components(owner).unwrap().get(Slot::Supply).unwrap()
                else {
                    panic!();
                };
                assert_eq!(value.byte_14, 128);
                Err::<(), _>("explicit disjoint-loan failure probe")
            })
            .unwrap()
            .unwrap_err();
        assert_eq!(failure, "explicit disjoint-loan failure probe");
        assert_eq!(actors.key(owner), Some(key));
        assert_eq!(actors.key(other), Some(other_key));
        assert_eq!(
            actors.object(key).unwrap() as *const _ as usize,
            object_address
        );
        assert_eq!(
            actors.object(other_key).unwrap() as *const _ as usize,
            other_address
        );
        assert_eq!(
            supply(&actors, owner).unwrap() as *const _ as usize,
            supply_address
        );
        assert_eq!(
            supply(&actors, other).unwrap() as *const _ as usize,
            other_supply_address
        );
        assert_eq!(supply(&actors, owner).unwrap().byte_14, 128);
        assert_eq!(supply(&actors, other).unwrap().byte_14, 0);
        let zone = actors
            .object(key)
            .unwrap()
            .actor
            .as_ref()
            .unwrap()
            .zones_148
            .zones()
            .slots[0]
            .as_ref()
            .unwrap();
        assert_eq!(zone.key, zone_key);
        assert_ne!(zone.state & 4, 0);
        assert_eq!(identities.len(), 1);
        assert_eq!(rng.state, rng_before);
        assert!(actors.with_zone_fields(key, 9, |_, _| ()).is_err());
        assert!(checked(&catalog, &actors, &Bank::default(), key).is_err());
        assert_eq!(supply(&actors, owner).unwrap().byte_14, 128);
        println!(
            "actual ger_corps/usa_corps source53 live{pc:?}: same current/other objects and Supply boxes, current lookup, zone identity and error prefix retained"
        );
    }
}
