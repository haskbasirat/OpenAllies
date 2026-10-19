use aa_numeric::Precision;
use aa_orders::net::sync::Synchronizer;
use aa_runtime::{
    actor_world::ActorStore,
    clocked::{self, Effect, Parts, Predicate, Runtime},
    live_clocked::*,
    live_registration,
    scheduler_owners::Owners,
    spatial_world::SpatialWorld,
};
use aa_simulation::{FloatBits, NativeClock, NativeRng};
use aa_triggers::{manager::Manager, program::CoreProgram};
use aa_world::{
    ObjectId, World,
    queues::{Clock, Delivery, Queues},
};
use std::cell::Cell;

fn required(leaf: &str) -> aa_world::Error {
    aa_world::Error::Callback(leaf.into())
}

fn empty_spatial() -> SpatialWorld {
    SpatialWorld::new(
        aa_spatial::collision::Grid {
            dimensions: [1, 1],
            coordinate_shift: 0,
            cell_size_bits: 1f32.to_bits(),
        },
        vec![0],
        Precision::Double53,
    )
    .unwrap()
}
fn empty_manager() -> Manager {
    Manager::from_saved(aa_triggers::decode::SavedManager {
        next_id: 0,
        triggers: vec![],
        variables: vec![],
        strings: vec![],
    })
}
fn clock() -> NativeClock {
    NativeClock {
        tick: 0,
        begin: FloatBits(0),
        end: FloatBits(0),
        step: FloatBits(0x3d800000),
    }
}
fn queues(clock: NativeClock) -> Queues {
    aa_world::initialization::ClockPlan::from_tick_rate(16f32.to_bits(), Precision::Double53)
        .unwrap()
        .initialize_queues(clock, Precision::Double53)
        .unwrap()
}

#[test]
fn real_field_owners_and_disabled_logger_preserve_first_tick_error_prefix() {
    // Explicit small owner fixture, not a reconstructed map or readiness proof.
    let mut actors = ActorStore::new();
    let owners = Owners::new();
    let mut spatial = empty_spatial();
    let mut registration = live_registration::State::from_world_constructor(vec![], false);
    let mut manager = empty_manager();
    let mut program = CoreProgram::new(0, 7, Precision::Double53);
    let mut sync = Synchronizer::new(16);
    let mut first = UnboundFirstTick;
    let fog = true;
    let world_fog = false;
    let mut counter = u32::MAX;
    let gate_calls = Cell::new(0);
    let mut gate = || {
        gate_calls.set(gate_calls.get() + 1);
        Err("unbound actual gate".into())
    };
    let services = Services {
        actors: &mut actors,
        owners: &owners,
        spatial: &mut spatial,
        registration: &mut registration,
        //No World+4C Sun owner: opcode 46 keeps its explicit required error.
        sun: None,
        fields: Fields {
            svar_fog_cheat: &fog,
            world_fog_cheat: &world_fog,
            auxiliary: Counter::Present(&mut counter),
        },
        sync: SyncLog {
            owner: &mut sync,
            gate: &mut gate,
        }
        .into(),
        triggers: &mut manager,
        trigger_program: &mut program,
        first_tick: &mut first,
    };
    let mut runtime = Runtime::from_parts(Parts {
        world: World::from_native_constructor(clock(), Precision::Double53),
        queues: queues(clock()),
        rng: NativeRng::new(0x12345678),
        services,
    })
    .unwrap();
    let before = runtime.queues.snapshot();
    let error = runtime.step().unwrap_err().to_string();
    assert!(error.contains("4F96D0"), "{error}");
    assert_eq!(runtime.world.phase, 1);
    assert_eq!(runtime.world.clock, clock());
    assert_eq!(runtime.queues.snapshot(), before);
    assert_eq!(runtime.rng.state, 0x12345678);
    assert!(runtime.stopped());
    assert_eq!(gate_calls.get(), 0);
    assert_eq!(runtime.services.sync.direct_owner().unwrap().hash, -1);
    assert!(runtime.world.object_flags.is_empty());
    assert!(runtime.world.pending100_targets.is_empty());
    assert_eq!(runtime.world.auxiliary_counter, None);
    assert!(runtime.world.events().iter().any(|e| matches!(
        e.operation,
        aa_world::Operation::FogCallback { enabled: true }
    )));
    let (mut state, mut services) = runtime.into_state();
    assert!(state.stopped());
    assert!(matches!(
        state.step(&mut services),
        Err(clocked::Error::Stopped)
    ));
    drop((state, services));
    assert_eq!(counter, 0);
}

#[test]
fn enabled_logger_is_same_owner_and_reads_required_gate_before_phase_or_counter() {
    let mut actors = ActorStore::new();
    let owners = Owners::new();
    let mut spatial = empty_spatial();
    let mut registration = live_registration::State::from_world_constructor(vec![], false);
    let mut manager = empty_manager();
    let mut program = CoreProgram::new(0, 7, Precision::Double53);
    let mut sync = Synchronizer::new(16);
    sync.enabled = true;
    let sync_address = &sync as *const _;
    let mut first = UnboundFirstTick;
    let fog = false;
    let mut counter = 9;
    let calls = Cell::new(0);
    let mut gate = || {
        calls.set(calls.get() + 1);
        Err("actual thread gate required".into())
    };
    let services = Services {
        actors: &mut actors,
        owners: &owners,
        spatial: &mut spatial,
        registration: &mut registration,
        //No World+4C Sun owner: opcode 46 keeps its explicit required error.
        sun: None,
        fields: Fields {
            svar_fog_cheat: &fog,
            world_fog_cheat: &fog,
            auxiliary: Counter::Present(&mut counter),
        },
        sync: SyncLog {
            owner: &mut sync,
            gate: &mut gate,
        }
        .into(),
        triggers: &mut manager,
        trigger_program: &mut program,
        first_tick: &mut first,
    };
    let mut runtime = Runtime::from_parts(Parts {
        world: World::from_native_constructor(clock(), Precision::Double53),
        queues: queues(clock()),
        rng: NativeRng::new(9),
        services,
    })
    .unwrap();
    let error = runtime.step().unwrap_err().to_string();
    assert!(error.contains("actual thread gate required"));
    assert_eq!(calls.get(), 1);
    assert_eq!(runtime.world.phase, 0);
    assert_eq!(runtime.world.clock, clock());
    assert_eq!(
        runtime.services.sync.direct_owner().unwrap() as *const _,
        sync_address
    );
    assert_eq!(runtime.services.sync.direct_owner().unwrap().hash, -1);
    drop(runtime);
    assert_eq!(counter, 9);
}

#[test]
fn move_binding_returns_all_original_parts_on_mismatch_and_keeps_queue_nodes() {
    struct Reject;
    impl clocked::Services for Reject {
        fn effect(
            &mut self,
            _: Effect,
            _: &mut World,
            _: &mut Queues,
            _: &mut NativeRng,
        ) -> aa_world::Result<()> {
            Err(required("fixture reached effect"))
        }
        fn predicate(
            &mut self,
            _: Predicate,
            _: ObjectId,
            _: &mut World,
            _: &mut Queues,
            _: &mut NativeRng,
        ) -> aa_world::Result<bool> {
            Err(required("fixture reached predicate"))
        }
        fn update_component(
            &mut self,
            _: Delivery,
            _: &mut World,
            _: &mut Queues,
            _: &mut NativeRng,
        ) -> aa_world::queues::Result<()> {
            Err(aa_world::queues::Error::Callback(
                "fixture reached queue".into(),
            ))
        }
    }
    let mut q = queues(clock());
    q.add_after(3, 17, 42, 0x3f800000).unwrap();
    q.clock.end_bits = 1;
    let snapshot = q.snapshot();
    let parts = Parts {
        world: World::from_native_constructor(clock(), Precision::Double53),
        queues: q,
        rng: NativeRng::new(123),
        services: Reject,
    };
    let failure = Runtime::from_parts(parts)
        .err()
        .expect("mismatching raw clock rejected");
    assert_eq!(failure.parts.rng.state, 123);
    assert_eq!(failure.parts.world.clock, clock());
    assert_eq!(failure.parts.queues.clock.end_bits, 1);
    assert_eq!(failure.parts.queues.snapshot(), snapshot);
    let mut parts = failure.parts;
    parts.queues.clock.end_bits = 0;
    let runtime = Runtime::from_parts(parts).unwrap();
    assert_eq!(runtime.rng.state, 123);
    assert_eq!(runtime.queues.snapshot(), snapshot);
    assert!(!runtime.stopped());
    let (state, _) = runtime.into_state();
    assert_eq!(state.rng.state, 123);
    assert_eq!(state.queues.snapshot(), snapshot);
    assert!(!state.stopped());
}

#[test]
fn permanent_clocked_state_does_not_reset_failure_when_services_are_reborrowed() {
    struct Fail<'a>(&'a mut u32);
    impl clocked::Services for Fail<'_> {
        fn effect(
            &mut self,
            _: Effect,
            _: &mut World,
            _: &mut Queues,
            _: &mut NativeRng,
        ) -> aa_world::Result<()> {
            *self.0 += 1;
            Err(required("explicit fixture effect"))
        }
        fn predicate(
            &mut self,
            _: Predicate,
            _: ObjectId,
            _: &mut World,
            _: &mut Queues,
            _: &mut NativeRng,
        ) -> aa_world::Result<bool> {
            Err(required("fixture predicate"))
        }
        fn update_component(
            &mut self,
            _: Delivery,
            _: &mut World,
            _: &mut Queues,
            _: &mut NativeRng,
        ) -> aa_world::queues::Result<()> {
            Err(aa_world::queues::Error::Callback("fixture queue".into()))
        }
    }
    let mut state = clocked::ClockedState::from_parts(clocked::StateParts {
        world: World::from_native_constructor(clock(), Precision::Double53),
        queues: queues(clock()),
        rng: NativeRng::new(17),
    })
    .unwrap();
    let mut calls = 0;
    assert!(state.step(&mut Fail(&mut calls)).is_err());
    assert!(state.stopped());
    assert_eq!(calls, 1);
    assert!(matches!(
        state.step(&mut Fail(&mut calls)),
        Err(clocked::Error::Stopped)
    ));
    assert_eq!(calls, 1);
    assert_eq!(state.world.clock, clock());
    assert_eq!(state.rng.state, 17);
}

#[test]
fn later_empty_dispatch_reuses_loaded_manager_program_and_same_sync_log() {
    let mut actors = ActorStore::new();
    let owners = Owners::new();
    let mut spatial = empty_spatial();
    let mut registration = live_registration::State::from_world_constructor(vec![], false);
    let mut manager = empty_manager();
    let manager_address = &manager as *const _;
    let mut program = CoreProgram::new(0xdeadbeef, 7, Precision::Single24);
    let mut sync = Synchronizer::new(16);
    sync.reset(2, 33);
    let mut first = UnboundFirstTick;
    let fog = true;
    let mut counter = 8;
    let mut gate = || Ok(true);
    let mut c = clock();
    c.tick = 2;
    c.begin = FloatBits(0x3d800000);
    c.end = FloatBits(0x3e000000);
    let services = Services {
        actors: &mut actors,
        owners: &owners,
        spatial: &mut spatial,
        registration: &mut registration,
        //No World+4C Sun owner: opcode 46 keeps its explicit required error.
        sun: None,
        fields: Fields {
            svar_fog_cheat: &fog,
            world_fog_cheat: &fog,
            auxiliary: Counter::Present(&mut counter),
        },
        sync: SyncLog {
            owner: &mut sync,
            gate: &mut gate,
        }
        .into(),
        triggers: &mut manager,
        trigger_program: &mut program,
        first_tick: &mut first,
    };
    let mut runtime = Runtime::from_parts(Parts {
        world: World::from_loaded_dispatcher(
            aa_world::DispatcherState::from_native_constructor(),
            c,
            Precision::Double53,
        ),
        queues: queues(c),
        rng: NativeRng::new(123),
        services,
    })
    .unwrap();
    let output = runtime.step().unwrap();
    assert_eq!(output.tick, 3);
    assert_eq!(output.begin.0, c.end.0);
    assert_eq!(runtime.world.phase, 0);
    assert_eq!(runtime.rng.state, 123);
    assert_eq!(runtime.services.triggers as *const _, manager_address);
    assert_eq!(runtime.services.trigger_program.time_bits, c.end.0);
    assert_eq!(
        runtime.services.trigger_program.precision,
        Precision::Double53
    );
    let entry = runtime
        .services
        .sync
        .direct_owner()
        .unwrap()
        .entry(3)
        .unwrap();
    assert_eq!(entry.label, "IsFogCheatEnabled()");
    assert_eq!(entry.value, 1);
    assert_eq!(entry.hash, aa_orders::net::sync::step(33, 1));
    assert!(runtime.world.object_flags.is_empty());
    drop(runtime);
    assert_eq!(counter, 9);
}

#[test]
fn original_and_v14_prefix_matches_all_fog_counter_log_and_precision_inputs() {
    let doc: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-clocked/native-live-prefix.json"
    ))
    .unwrap();
    let rows = doc["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 216);
    for row in rows {
        let input = &row["input"];
        let p = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => panic!(),
        };
        let mut actors = ActorStore::new();
        let owners = Owners::new();
        let mut spatial = empty_spatial();
        let mut registration = live_registration::State::from_world_constructor(vec![], false);
        let mut manager = empty_manager();
        let mut program = CoreProgram::new(0, 7, p);
        let mut sync = Synchronizer::new(16);
        let mode = input["log_mode"].as_str().unwrap();
        sync.enabled = mode != "disabled";
        sync.suspend = i32::from(mode == "suspended");
        let svar = input["svar_fog"].as_bool().unwrap();
        let fog = input["world_fog"].as_bool().unwrap();
        let mut counter = input["counter"].as_u64().map(|v| v as u32);
        let auxiliary = match counter.as_mut() {
            Some(value) => Counter::Present(value),
            None => Counter::Null,
        };
        let mut first = UnboundFirstTick;
        let mut gate = || Err("sync_gate_required".into());
        let services = Services {
            actors: &mut actors,
            owners: &owners,
            spatial: &mut spatial,
            registration: &mut registration,
            //No World+4C Sun owner: opcode 46 keeps its explicit required error.
            sun: None,
            fields: Fields {
                svar_fog_cheat: &svar,
                world_fog_cheat: &fog,
                auxiliary,
            },
            sync: SyncLog {
                owner: &mut sync,
                gate: &mut gate,
            }
            .into(),
            triggers: &mut manager,
            trigger_program: &mut program,
            first_tick: &mut first,
        };
        let q = Queues::new(
            Clock {
                begin_bits: 0,
                end_bits: 0,
                step_bits: clock().step.0,
            },
            p,
        )
        .unwrap();
        let mut runtime = Runtime::from_parts(Parts {
            world: World::from_native_constructor(clock(), p),
            queues: q,
            rng: NativeRng::new(123),
            services,
        })
        .unwrap();
        let error = runtime.step().unwrap_err().to_string();
        let outcome = if error.contains("4F96D0") {
            "first_tick_required"
        } else if error.contains("sync_gate_required") {
            "sync_gate_required"
        } else {
            panic!("{error}")
        };
        let actual = serde_json::json!({"outcome":outcome,"fog":u32::from(svar||fog),"phase":runtime.world.phase,
            "clock":[runtime.world.clock.tick,runtime.world.clock.begin.0,runtime.world.clock.end.0,runtime.world.clock.step.0],
            "sync_hash":runtime.services.sync.direct_owner().unwrap().hash as u32});
        for field in ["outcome", "fog", "phase", "clock", "sync_hash"] {
            assert_eq!(actual[field], row["output"][field], "{row}");
        }
        assert!(runtime.stopped());
        assert_eq!(runtime.rng.state, 123);
        drop(runtime);
        assert_eq!(
            serde_json::json!(counter),
            row["output"]["counter"],
            "{row}"
        );
    }
}

#[test]
fn shared_sync_owner_loan_ends_before_first_tick_and_network_uses_same_log() {
    use std::cell::RefCell;
    struct First<'a> {
        sync: &'a RefCell<Synchronizer>,
        called: &'a Cell<bool>,
    }
    impl FirstTick for First<'_> {
        fn run(&mut self, _: &ActorStore) -> Result<(), String> {
            let _loan = self.sync.try_borrow_mut().map_err(|e| e.to_string())?;
            self.called.set(true);
            Err("explicit first-tick fixture required".into())
        }
    }
    let mut actors = ActorStore::new();
    let owners = Owners::new();
    let mut spatial = empty_spatial();
    let mut registration = live_registration::State::from_world_constructor(vec![], false);
    let mut manager = empty_manager();
    let mut program = CoreProgram::new(0, 7, Precision::Double53);
    let sync = RefCell::new(Synchronizer::new(16));
    sync.borrow_mut().reset(0, 33);
    let called = Cell::new(false);
    let mut first = First {
        sync: &sync,
        called: &called,
    };
    let fog = true;
    let mut gate = || Ok(true);
    let services = Services {
        actors: &mut actors,
        owners: &owners,
        spatial: &mut spatial,
        registration: &mut registration,
        //No World+4C Sun owner: opcode 46 keeps its explicit required error.
        sun: None,
        fields: Fields {
            svar_fog_cheat: &fog,
            world_fog_cheat: &fog,
            auxiliary: Counter::Null,
        },
        sync: SyncBinding::Shared {
            owner: &sync,
            gate: &mut gate,
        },
        triggers: &mut manager,
        trigger_program: &mut program,
        first_tick: &mut first,
    };
    let mut runtime = Runtime::from_parts(Parts {
        world: World::from_native_constructor(clock(), Precision::Double53),
        queues: queues(clock()),
        rng: NativeRng::new(7),
        services,
    })
    .unwrap();
    assert!(
        runtime
            .step()
            .unwrap_err()
            .to_string()
            .contains("explicit first-tick fixture required")
    );
    assert!(called.get());
    let expected = aa_orders::net::sync::step(33, 1);
    assert_eq!(sync.borrow().hash, expected);
    // A separate network callback loans the actual same owner after the world
    // callback. The services object still exists; no long-lived mutable loan.
    sync.borrow_mut()
        .log_int(true, "explicit network fixture", 7);
    assert_eq!(sync.borrow().hash, aa_orders::net::sync::step(expected, 7));
}
