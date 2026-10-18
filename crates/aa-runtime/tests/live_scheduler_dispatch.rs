//! Original-source ActorStore owners plus explicitly labeled native field probes.
use aa_actors::{
    ActorId,
    components::ComponentSlot,
    creation::{ActorCreationRecord, CreationRecord, ThingDefinition},
};
use aa_body::F32;
use aa_numeric::Precision;
use aa_runtime::{
    actor_world::{
        ActiveComponent, ActorStore, ObjectKey, PlacementState, RuntimeComponent, Spatial,
    },
    catalog::Catalog,
    live_scheduler_dispatch::{Context, dispatch},
    scheduler_owners::{Owners, Target},
};
use aa_simulation::NativeRng;
use aa_world::queues::{Clock, Delivery, Kind, Queues};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Default)]
struct Poses(BTreeMap<ActorId, PlacementState>);
impl Spatial for Poses {
    fn placement(&self, id: ActorId) -> aa_runtime::actor_world::Result<PlacementState> {
        self.0
            .get(&id)
            .copied()
            .ok_or(aa_runtime::actor_world::Error::Missing(
                "test spatial identity",
            ))
    }
    fn publish(&mut self, id: ActorId, p: PlacementState) -> aa_runtime::actor_world::Result<()> {
        assert!(self.0.insert(id, p).is_none());
        Ok(())
    }
}
fn precision(value: &Value) -> Precision {
    match value.as_u64().unwrap() {
        24 => Precision::Single24,
        53 => Precision::Double53,
        64 => Precision::Extended64,
        _ => panic!(),
    }
}
fn queues(pc: Precision) -> (Queues, NativeRng) {
    let mut q = Queues::new(
        Clock {
            begin_bits: 12.25f32.to_bits(),
            end_bits: 12.5f32.to_bits(),
            step_bits: 0.25f32.to_bits(),
        },
        pc,
    )
    .unwrap();
    for (id, count) in [1, 4, 20, 20, 20].into_iter().enumerate() {
        q.initialize(
            id as u8,
            if id < 3 {
                Kind::Periodic
            } else {
                Kind::Deferred
            },
            count,
        )
        .unwrap();
    }
    let mut rng = NativeRng::new(0x12345678);
    q.add_periodic(0, 1, 0, &mut rng).unwrap();
    q.add_periodic(0, 1, 7, &mut rng).unwrap();
    q.add_periodic(0, 2, 0, &mut rng).unwrap();
    q.add_after(3, 1, 0, 1f32.to_bits()).unwrap();
    q.add_after(4, 2, 13, 1f32.to_bits()).unwrap();
    (q, rng)
}
fn native_fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-scheduler-dispatch/native-dispatch.json"
    ))
    .unwrap()
}
fn prepare(catalog: &Catalog<'_>) -> (ActorStore, ObjectKey, ActorId) {
    let mut actors = ActorStore::new();
    let mut rng = NativeRng::new(0x98765432);
    let record = ActorCreationRecord {
        base: CreationRecord {
            definition: ThingDefinition {
                key: "usa_infantry".into(),
                flags: 0,
            },
            position_bits: [0; 3],
            direction_bits: [0, (-1f32).to_bits()],
            name_utf16: vec![],
            saved_id: ActorId::NULL,
        },
        variant: -1,
        owner_word: 0,
        word_30: 0,
        request_nonblocking: false,
    };
    let key = actors
        .construct_new(
            catalog,
            record,
            0,
            &mut rng,
            Precision::Double53,
            Precision::Double53,
        )
        .unwrap();
    let id = actors
        .initialize_object(key, &mut rng, Precision::Double53)
        .unwrap();
    let mut bank = aa_runtime::definitions::Bank::default();
    let input = actors.source_performer(key, catalog, &mut bank).unwrap();
    actors
        .construct_performer(key, &input, Precision::Double53, &mut |store, owner| {
            store.rebuild_construction_actions(owner)
        })
        .unwrap();
    actors
        .construct_ground(
            key,
            ComponentSlot::Properties,
            F32::ZERO,
            Precision::Double53,
        )
        .unwrap();
    actors
        .construct_ground(
            key,
            ComponentSlot::Character,
            F32::ZERO,
            Precision::Double53,
        )
        .unwrap();
    (actors, key, id)
}
fn sources() -> (
    aa_rules::actors::ActorCatalog,
    aa_runtime::settings::Settings,
) {
    use aa_rules::sources::{SourceLimits, V14Sources};
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings =
        aa_runtime::settings::Settings::load_v14(&mut files, aa_rules::tgi::Limits::default())
            .unwrap();
    (
        aa_rules::actors::ActorCatalog::load_v14(&mut files).unwrap(),
        settings,
    )
}
fn perf(actors: &ActorStore, id: ActorId) -> &aa_combat::constructors::OwnedPerformer {
    match actors
        .component(id, ComponentSlot::Performer)
        .unwrap()
        .unwrap()
    {
        RuntimeComponent::Performer(v) => v,
        _ => panic!(),
    }
}
fn perf_mut(actors: &mut ActorStore, id: ActorId) -> &mut aa_combat::constructors::OwnedPerformer {
    match actors
        .component_mut(id, ComponentSlot::Performer)
        .unwrap()
        .unwrap()
    {
        RuntimeComponent::Performer(v) => v,
        _ => panic!(),
    }
}

#[test]
fn native_receipt_distinguishes_three_queue_dispatch_entries() {
    let fixture = native_fixture();
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 972);
    assert_eq!(fixture["noops"].as_array().unwrap().len(), 135);
    let pins = fixture["pins"].as_array().unwrap();
    assert_eq!(
        pins.iter().find(|p| p["name"] == "dispatchers").unwrap()["bytes"],
        "8b01ff601ccccccccccccccccccccccc8b01ff6018"
    );
}

#[test]
#[ignore = "requires original/v14 source via AA_GAME_ROOT"]
fn actual_store_replays_all_native_prefixes_and_noop_leaves() {
    let (source, settings) = sources();
    let mut catalog = Catalog::new(&source);
    catalog.settings = Some(settings);
    let (mut actors, key, id) = prepare(&catalog);
    // Explicit isolated native-class probes on a real source actor, constructed
    // before publication. Infantry does not naturally own all these slots.
    for slot in [
        ComponentSlot::Recruit,
        ComponentSlot::Builder,
        ComponentSlot::MobileDeployment,
        ComponentSlot::Research,
    ] {
        actors
            .construct_ground(key, slot, F32::ZERO, Precision::Double53)
            .unwrap();
    }
    actors
        .object_mut(key)
        .unwrap()
        .components_mut()
        .unwrap()
        .insert(
            ComponentSlot::Hq,
            RuntimeComponent::Hq(Box::new(aa_actors::hq_state::HqState::new(id, 1))),
        );
    let mut poses = Poses::default();
    actors.publish_placement(key, &mut poses).unwrap();
    let allocation = perf(&actors, id) as *const _;
    let mut owners = Owners::new();
    let owner = owners
        .register(Target::Component {
            object: key,
            slot: ComponentSlot::Performer as u8,
        })
        .unwrap();
    assert_eq!(owner, 1);
    assert_eq!(owners.register(Target::Command(9)).unwrap(), 2);
    let mut absent_properties = None;
    let mut absent_character = None;
    let fixture = native_fixture();
    for (ordinal, row) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let c = &row["input"];
        let out = &row["output"];
        let wanted_properties = !c["property_flags"].is_null();
        let wanted_character = c["animator"].as_bool().unwrap();
        for (slot, wanted, held) in [
            (
                ComponentSlot::Properties,
                wanted_properties,
                &mut absent_properties,
            ),
            (
                ComponentSlot::Character,
                wanted_character,
                &mut absent_character,
            ),
        ] {
            let table = actors.object_mut(key).unwrap().components_mut().unwrap();
            if wanted && !table.present(slot) {
                table.insert(slot, held.take().unwrap());
            }
            if !wanted && table.present(slot) {
                *held = table.remove(slot);
            }
        }
        actors.object_mut(key).unwrap().active_component = if c["active"].as_bool().unwrap() {
            ActiveComponent::Slot(ComponentSlot::Performer)
        } else {
            ActiveComponent::None
        };
        poses.0.get_mut(&id).unwrap().flags = c["thing_flags"].as_u64().unwrap() as u32;
        if let Some(RuntimeComponent::Properties(v)) =
            actors.component_mut(id, ComponentSlot::Properties).unwrap()
        {
            v.modifier.flags = c["property_flags"].as_u64().unwrap() as u32;
            // The later field is deliberately opposite; native reads +1C.
            v.modifier_flags = v.modifier.flags ^ 2;
        }
        perf_mut(&mut actors, id).state.stage = c["stage"].as_u64().unwrap() as u32 as i32;
        perf_mut(&mut actors, id).state.turret_index = 17;
        let (mut q, mut rng) = queues(precision(&c["pc"]));
        assert_eq!(
            serde_json::to_value(q.snapshot()).unwrap(),
            row["before_queues"],
            "before row{ordinal}"
        );
        let rng_before = rng;
        let event = Delivery {
            queue: 3,
            owner,
            data: c["event"].as_u64().unwrap() as u32,
            elapsed_bits: None,
        };
        let result = dispatch(
            event,
            &mut Context {
                actors: &mut actors,
                owners: &owners,
                queues: &mut q,
                rng: &mut rng,
                spatial: &poses,
            },
        );
        if let Some(boundary) = out["first_unbound"].as_str() {
            let expected = match boundary {
                "begin" => "57ED20",
                "use_frame" => "57E8C0",
                "end_context" => "4C8300",
                "finish" => "57DC90",
                "animator_cue" => "563AA0",
                _ => panic!(),
            };
            assert!(
                result.unwrap_err().to_string().contains(expected),
                "row{ordinal} boundary{boundary}"
            );
        } else {
            result.unwrap();
        }
        assert_eq!(rng, rng_before, "row{ordinal} RNG");
        assert_eq!(
            perf(&actors, id).state.stage as u32,
            out["stage_bits"],
            "row{ordinal} stage"
        );
        assert_eq!(
            perf(&actors, id).state.turret_index as u32,
            out["turret_index_bits"],
            "row{ordinal} turret"
        );
        assert_eq!(
            serde_json::to_value(q.snapshot()).unwrap(),
            out["queues"],
            "row{ordinal} queues"
        );
        assert_eq!(
            allocation,
            perf(&actors, id) as *const _,
            "owner must stay allocated"
        );
    }
    for row in fixture["noops"].as_array().unwrap() {
        let slot = ComponentSlot::from_index(row["slot"].as_u64().unwrap() as usize).unwrap();
        let target = Target::Component {
            object: key,
            slot: slot as u8,
        };
        let token = owners
            .get(target)
            .unwrap_or_else(|_| owners.register(target).unwrap());
        let (mut q, mut rng) = queues(precision(&row["pc"]));
        let before = q.snapshot();
        let before_rng = rng;
        dispatch(
            Delivery {
                queue: 0,
                owner: token,
                data: row["data"].as_u64().unwrap() as u32,
                elapsed_bits: Some(row["elapsed_bits"].as_u64().unwrap() as u32),
            },
            &mut Context {
                actors: &mut actors,
                owners: &owners,
                queues: &mut q,
                rng: &mut rng,
                spatial: &poses,
            },
        )
        .unwrap();
        assert_eq!(q.snapshot(), before);
        assert_eq!(rng, before_rng);
    }
}

#[test]
#[ignore = "requires original/v14 source via AA_GAME_ROOT"]
fn live_identity_laziness_and_actual_queued_pausing() {
    let (source, settings) = sources();
    let mut catalog = Catalog::new(&source);
    catalog.settings = Some(settings);
    let (mut actors, key, id) = prepare(&catalog);
    let mut poses = Poses::default();
    actors.publish_placement(key, &mut poses).unwrap();
    let mut owners = Owners::new();
    let target = Target::Component {
        object: key,
        slot: ComponentSlot::Performer as u8,
    };
    let owner = owners.register(target).unwrap();
    owners.register(Target::Command(9)).unwrap();
    let (mut q, mut rng) = queues(Precision::Double53);
    let event = Delivery {
        queue: 3,
        owner,
        data: 4,
        elapsed_bits: None,
    };
    let legitimate = actors
        .object_mut(key)
        .unwrap()
        .components_mut()
        .unwrap()
        .remove(ComponentSlot::Properties)
        .unwrap();
    actors
        .object_mut(key)
        .unwrap()
        .components_mut()
        .unwrap()
        .insert(
            ComponentSlot::Properties,
            RuntimeComponent::Builder(Box::new(aa_actors::builder_state::BuilderState::new(id))),
        );
    actors.object_mut(key).unwrap().active_component = ActiveComponent::None;
    dispatch(
        event,
        &mut Context {
            actors: &mut actors,
            owners: &owners,
            queues: &mut q,
            rng: &mut rng,
            spatial: &poses,
        },
    )
    .unwrap();
    actors.object_mut(key).unwrap().active_component =
        ActiveComponent::Slot(ComponentSlot::Performer);
    poses.0.get_mut(&id).unwrap().flags |= 1;
    dispatch(
        event,
        &mut Context {
            actors: &mut actors,
            owners: &owners,
            queues: &mut q,
            rng: &mut rng,
            spatial: &poses,
        },
    )
    .unwrap();
    poses.0.get_mut(&id).unwrap().flags &= !1;
    assert!(
        dispatch(
            event,
            &mut Context {
                actors: &mut actors,
                owners: &owners,
                queues: &mut q,
                rng: &mut rng,
                spatial: &poses
            }
        )
        .unwrap_err()
        .to_string()
        .contains("Properties slot")
    );
    actors
        .object_mut(key)
        .unwrap()
        .components_mut()
        .unwrap()
        .insert(ComponentSlot::Properties, legitimate);
    if let Some(RuntimeComponent::Properties(v)) =
        actors.component_mut(id, ComponentSlot::Properties).unwrap()
    {
        v.modifier.flags |= 2;
    }
    // Real Queues unlinks the event before dispatch; the same owner reschedules.
    q.cancel_owner(1).unwrap();
    q.cancel_owner(2).unwrap();
    q.add_after(3, owner, 0xffffffff, 0).unwrap();
    struct Live<'a> {
        actors: &'a mut ActorStore,
        owners: &'a Owners,
        rng: &'a mut NativeRng,
        poses: &'a Poses,
    }
    impl aa_world::queues::Callbacks for Live<'_> {
        fn deliver(&mut self, q: &mut Queues, event: Delivery) -> aa_world::queues::Result<()> {
            dispatch(
                event,
                &mut Context {
                    actors: self.actors,
                    owners: self.owners,
                    queues: q,
                    rng: self.rng,
                    spatial: self.poses,
                },
            )
        }
    }
    let before_rng = rng;
    q.update(
        3,
        &mut Live {
            actors: &mut actors,
            owners: &owners,
            rng: &mut rng,
            poses: &poses,
        },
    )
    .unwrap();
    assert!(q.has(owner, 3, 0xffffffff));
    assert_eq!(rng, before_rng);
    perf_mut(&mut actors, id).state.owner ^= 0x10000;
    assert!(
        dispatch(
            event,
            &mut Context {
                actors: &mut actors,
                owners: &owners,
                queues: &mut q,
                rng: &mut rng,
                spatial: &poses
            }
        )
        .unwrap_err()
        .to_string()
        .contains("Performer owner")
    );
    perf_mut(&mut actors, id).state.owner = id.raw();
    owners.retire(target, &mut q).unwrap();
    let replacement = owners.register(target).unwrap();
    assert_ne!(replacement, owner);
    assert!(
        dispatch(
            event,
            &mut Context {
                actors: &mut actors,
                owners: &owners,
                queues: &mut q,
                rng: &mut rng,
                spatial: &poses
            }
        )
        .is_err()
    );
    let sun = owners.register(Target::Sun).unwrap();
    assert!(
        dispatch(
            Delivery {
                owner: sun,
                ..event
            },
            &mut Context {
                actors: &mut actors,
                owners: &owners,
                queues: &mut q,
                rng: &mut rng,
                spatial: &poses
            }
        )
        .unwrap_err()
        .to_string()
        .contains("Sun")
    );
}
