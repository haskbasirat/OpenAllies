//! Actual source/ActorStore/Input loans with an explicitly controlled counting
//! execution fixture. This is not a complete Staged World or real OS worker.
use super::*;
use aa_actors::creation::{ActorCreationRecord, CreationRecord, ThingDefinition};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

struct CountingExecution {
    trace: Rc<RefCell<Vec<&'static str>>>,
    worker: u32,
    application: u32,
}
impl crate::execution_context::ExecutionIds for CountingExecution {
    fn current_os_thread_id(&mut self) -> Result<u32> {
        self.trace.borrow_mut().push("current_os");
        Ok(self.worker)
    }
    fn application_os_thread_id(&self) -> u32 {
        self.trace.borrow_mut().push("application_os");
        self.application
    }
}
fn snapshot(command: &Command) -> Value {
    let words = (8..0x58)
        .step_by(4)
        .map(|offset| {
            let value = match offset {
                0xc => command.kingdom().map_or(0, ActorId::raw),
                0x10 => command.source().map_or(0, ActorId::raw),
                0x20 => command.target().map_or(0, ActorId::raw),
                _ => command.word(offset).expect("actual428480 word"),
            };
            (offset.to_string(), value)
        })
        .collect::<BTreeMap<_, _>>();
    json!({"words":words,"byte58":command.byte(0x58),"flag":0,
        "fxl_definition":command.fxl_definition(),"fxl_parent_is_self":command.fxl_parent_is_self(),"word68":command.word(0x68)})
}
#[test]
#[ignore = "requires AA_GAME_ROOT; actual source and ActorStore laboratory, controlled counting ExecutionIds only"]
fn actual_fresh_actor_inputs_keep_same_graph_clock_and_pending_commands() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let settings = crate::settings::Settings::load_v14(&mut files, Limits::default()).unwrap();
    let actors_source = ActorCatalog::load_v14(&mut files).unwrap();
    let trace = Rc::new(RefCell::new(vec![]));
    let mut execution = CountingExecution {
        trace: trace.clone(),
        worker: 41,
        application: 42,
    };
    let mut catalog = Catalog::new(&actors_source);
    catalog.settings = Some(settings);
    let receipt = crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(
        &catalog, &mut files,
    )
    .unwrap();
    let startup =
        crate::actor_creation_startup::StartupInputs::from_verified_startup_reference(&receipt)
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
    let source_pc = catalog
        .definition_bank
        .borrow()
        .combat_source()
        .unwrap()
        .source_precision();
    let binding = catalog.actor_bindings("ger_corps", source_pc).unwrap();
    catalog.bind_load_execution(&mut execution).unwrap();
    let bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    assert!(catalog.definition_bank.borrow().combat_source().is_err());
    let constructed = Arc::new(
        Graph::compile_in_bank(&catalog, &bank, &binding.definition, ListKind::Constructed)
            .unwrap(),
    );
    let created = Arc::new(
        Graph::compile_in_bank(&catalog, &bank, &binding.definition, ListKind::Created).unwrap(),
    );
    assert_eq!(constructed.roots().len(), 17);
    assert!(created.roots().is_empty());
    assert_eq!(constructed.nodes().len(), 20);
    for &id in constructed.roots() {
        assert_eq!(
            constructed.node_kind(id),
            Some(fx_graph::FxcKind::CreateVfx)
        );
    }
    assert!(
        constructed
            .nodes()
            .iter()
            .any(|node| node.text(0x3c) == Some("talk1"))
    );
    let mut rows = vec![];
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let mut actors = ActorStore::new();
        let mut rng = aa_simulation::NativeRng::new(0x12345678);
        let record = ActorCreationRecord {
            base: CreationRecord {
                definition: ThingDefinition {
                    key: "ger_corps".into(),
                    flags: binding.definition.flags,
                },
                position_bits: [0x3f800001, 0x40000002, 0x40400003],
                direction_bits: [0, 0xbf800000],
                name_utf16: vec![],
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
                binding.clone(),
                record,
                0,
                &mut rng,
                startup.startup_precision(),
                pc,
            )
            .unwrap();
        let owner = actors.initialize_object(key, &mut rng, pc).unwrap();
        assert!(matches!(
            &actors.entity(key).unwrap().source,
            Source::New { .. }
        ));
        let address = actors.object(key).unwrap() as *const _ as usize;
        let rng_before = rng.state;
        //Explicit unused index allocation: this actual object still owns its
        //staged pose, so the real Actors.placement makes no index query.
        let spatial = SpatialWorld::new(
            aa_spatial::collision::Grid {
                dimensions: [1, 1],
                coordinate_shift: 0,
                cell_size_bits: 1f32.to_bits(),
            },
            vec![0],
            pc,
        )
        .unwrap();
        let pose = actors.placement(key, &spatial).unwrap().position_bits;
        let mut clock = aa_simulation::NativeClock::stock();
        clock.advance_fragment_at_precision(pc).unwrap();
        clock.advance_fragment_at_precision(pc).unwrap();
        let mut presentation = fx_manager::State::<Command>::from_native_constructor();
        let manager_address = &presentation as *const _ as usize;
        trace.borrow_mut().clear();
        let empty = fx_graph::construct_existing(
            created.clone(),
            &mut Inputs {
                catalog: &catalog,
                actors: &actors,
                spatial: &spatial,
                clock: &clock,
                key,
                owner,
                presentation: &presentation,
            },
            0,
        )
        .unwrap();
        assert!(empty.is_none());
        assert!(trace.borrow().is_empty());
        assert!(presentation.pending().next().is_none());
        let mut commands = vec![];
        for _ in 0..2 {
            let command = fx_graph::construct_existing(
                constructed.clone(),
                &mut Inputs {
                    catalog: &catalog,
                    actors: &actors,
                    spatial: &spatial,
                    clock: &clock,
                    key,
                    owner,
                    presentation: &presentation,
                },
                0,
            )
            .unwrap()
            .unwrap();
            assert!(Arc::ptr_eq(command.graph(), &constructed));
            assert_eq!(command.source(), Some(owner));
            assert_eq!(command.target(), Some(owner));
            assert_eq!(command.kingdom(), None);
            assert_eq!(command.word(0x38), Some(clock.begin.0));
            assert_eq!(command.byte(0x58), Some(0));
            commands.push(snapshot(&command));
            presentation
                .submit(Some(command), 0, &mut Presentation)
                .unwrap();
        }
        assert_eq!(
            &*trace.borrow(),
            &[
                "current_os",
                "application_os",
                "current_os",
                "application_os"
            ]
        );
        assert_eq!(presentation.pending().count(), 2);
        for item in presentation.pending() {
            assert!(Arc::ptr_eq(item.as_ref().unwrap().graph(), &constructed));
        }
        assert_eq!(&presentation as *const _ as usize, manager_address);
        assert_eq!(actors.object(key).unwrap() as *const _ as usize, address);
        assert_eq!(rng.state, rng_before);
        let input = json!({"kind":"Actor","nonempty":true,"muted":false,"source_flags":0,"flag":0,
            "source_view":false,"thread_id":41,"app_thread":42,"thing_id":owner.raw(),"kingdom_id":0,"position":pose,"global_position":[0,0,0],"world_time":clock.begin.0,"view_time":0xdeadbeefu32});
        rows.push(json!({"source_id":"ger_corps","source_precision":source_pc,"graph_nodes":constructed.nodes().len(),"constructed_roots":constructed.roots().len(),"precision":pc,"input":input,"commands":commands,"execution_fixture_trace":*trace.borrow(),"pending":presentation.pending().count()}));
        //Reached current View and stale E8 identities stay precise errors;
        //neither failure enqueues a synthetic command or resets prior state.
        actors.object_mut(key).unwrap().thing.word_c = 1;
        let error = fx_graph::construct_existing(
            constructed.clone(),
            &mut Inputs {
                catalog: &catalog,
                actors: &actors,
                spatial: &spatial,
                clock: &clock,
                key,
                owner,
                presentation: &presentation,
            },
            0,
        )
        .unwrap_err();
        assert!(error.contains("View54/vfBC"));
        assert_eq!(presentation.pending().count(), 2);
        actors.object_mut(key).unwrap().thing.word_c = 0;
        actors
            .object_mut(key)
            .unwrap()
            .actor
            .as_mut()
            .unwrap()
            .words
            .insert(0xe8, u32::MAX);
        let error = fx_graph::construct_existing(
            constructed.clone(),
            &mut Inputs {
                catalog: &catalog,
                actors: &actors,
                spatial: &spatial,
                clock: &clock,
                key,
                owner,
                presentation: &presentation,
            },
            0,
        )
        .unwrap_err();
        assert!(error.contains("generation absent"));
        assert_eq!(presentation.pending().count(), 2);
    }
    if let Some(path) = std::env::var_os("AA_STORED_CREATION_FX_CAPTURE") {
        std::fs::write(path, serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
        return;
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../research/skirmish-20261017-stored-creation-fx/native-actual-stored-creation-fx.json",
    );
    let proof: Value =
        serde_json::from_slice(&std::fs::read(path).expect(
            "capture once with AA_STORED_CREATION_FX_CAPTURE, then immutable native replay",
        ))
        .unwrap();
    let expected = proof["rows"].as_array().unwrap();
    assert_eq!(rows.len(), expected.len());
    for (actual, native) in rows.iter().zip(expected) {
        assert_eq!(&actual["input"], &native["input"]);
        for command in actual["commands"].as_array().unwrap() {
            assert_eq!(command, &native["output"]["command"]);
        }
    }
}
