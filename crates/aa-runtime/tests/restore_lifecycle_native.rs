//! Bounded fresh managed-load lifecycle: original constructor/task fixtures,
//! inactive7A0C20 replay, source configuration and non-cloned capability moves.
use aa_rules::sources::{Origin, ResolvedFile, SourceLimits, V14Sources};
use aa_runtime::restore_lifecycle::{ScopedManagedLoad, TaskKind};
use serde_json::Value;

fn source(body: &str) -> ResolvedFile {
    let bytes = body.as_bytes().to_vec();
    ResolvedFile {
        logical_path: "UVars.tgi".into(),
        origin: Origin::V14Loose,
        sha256: aa_rules::sources::sha256(&bytes),
        bytes,
    }
}
fn empty() -> ResolvedFile {
    source("[Vars]\n{ string GoalPriorityCSV = null\n}\n")
}

#[test]
fn native_full_constructor_and_managed_empty_world_prefix() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-restore-lifecycle/native-lifecycle.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 54);
    for case in cases {
        let task = match case["task_mode"].as_u64().unwrap() {
            1 => TaskKind::Editor,
            2 => TaskKind::Alternate,
            _ => TaskKind::Normal,
        };
        let scope = ScopedManagedLoad::from_uvars(&empty(), case["clock"].as_u64().unwrap() as u32)
            .unwrap();
        let load = scope.begin(task).unwrap();
        assert_eq!(
            u64::from(load.initialization().private_seed),
            case["constructor"]["seed"]
        );
        assert_eq!(u64::from(load.native_load_mode()), case["load_mode"]);
        assert_eq!(
            load.observer_present().unwrap(),
            case["observer_present"].as_bool().unwrap()
        );
        assert!(!case["observer_active"].as_bool().unwrap());
        assert_eq!(case["events"][1], "0076ab80");
        assert_eq!(case["events"][4], "004f7910");
        assert_eq!(case["events"][5], format!("{:08x}", task.wrapper_address()));
        assert_eq!(load.task(), task);
    }
}

#[test]
fn native_inactive_observer_does_not_require_an_actor() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/resume-20261016-element-attach/observer/inactive-oracle.json"
    ))
    .unwrap();
    let load = ScopedManagedLoad::from_uvars(&empty(), 37)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    assert_eq!(doc["cases"].as_array().unwrap().len(), 42);
    for case in doc["cases"].as_array().unwrap() {
        assert_eq!(case["actor_reads"], 0);
        assert!(case["returned"].as_bool().unwrap());
        load.notify_observer(
            u32::from_str_radix(case["actor_pointer"].as_str().unwrap(), 16).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn unknown_source_and_activation_are_never_defaulted_to_absent() {
    assert!(
        ScopedManagedLoad::default()
            .begin(TaskKind::Normal)
            .unwrap_err()
            .contains("Unknown")
    );
    for text in [
        "[Vars]\n{ bool GoalPriorityCSV = false\n}\n",
        "[Vars]\n{ string GoalPriorityCSV\n}\n",
        "[Vars]\n{}\n",
        "[Vars]\n{ string GoalPriorityCSV = debug.csv\n}\n",
        "[Vars Template=InheritedVars]\n{ string GoalPriorityCSV = null\n}\n",
        "[Vars]\n{ string GoalPriorityCSV = null\nstring GoalPriorityCSV = null\n}\n",
    ] {
        assert!(
            ScopedManagedLoad::from_uvars(&source(text), 0).is_err(),
            "{text}"
        );
    }
    let mut invalid = empty();
    invalid.sha256 = "wrong".into();
    assert!(ScopedManagedLoad::from_uvars(&invalid, 0).is_err());
    let mut load = ScopedManagedLoad::from_uvars(&empty(), 0)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let address = load.synchronization_owner_address();
    // Move the actual capability; there is no Clone or public state setter.
    let transfer = |value| value;
    load = transfer(load);
    assert_eq!(load.synchronization_owner_address(), address);
    assert!(load.observer_present().unwrap());
    assert!(load.request_world_ready().unwrap_err().contains("unbound"));
    assert!(load.observer_present().is_err());
    assert!(load.notify_observer(0).is_err());
}

#[test]
fn startup_leases_same_manager_and_keeps_failed_effects() {
    let mut load=ScopedManagedLoad::from_uvars(&empty(),37).unwrap()
        .begin(TaskKind::Normal).unwrap();
    let signal=load.synchronization_owner_address();
    let expected=load.initialization().private_seed;
    let error=load.with_startup_engine::<()>(|engine| {
        assert!(!engine.ready);
        assert_eq!(engine.rng.state,expected);
        assert!(engine.players.is_empty());
        engine.players.push(Some(19));
        engine.word_6c=23;
        Err("reached required startup child".into())
    }).unwrap_err();
    assert_eq!(error,"reached required startup child");
    assert!(load.observer_active().is_err());
    assert!(load.with_startup_engine(|_|Ok(())).is_err());
    let owner=load.into_startup_owner().unwrap();
    assert_eq!(owner.synchronization_owner_address(),signal);
    assert_eq!(owner.engine().players,vec![Some(19)]);
    assert_eq!(owner.engine().word_6c,23);
    assert!(!owner.engine().ready);
    assert_eq!(owner.engine().rng.state,expected);
    let still_loading=ScopedManagedLoad::from_uvars(&empty(),0).unwrap()
        .begin(TaskKind::Normal).unwrap();
    assert!(still_loading.into_startup_owner().unwrap_err().observer_present().unwrap());
}

#[test]
#[ignore = "requires original AA_GAME_ROOT source archive"]
fn actual_v14_uvars_source_constructs_the_same_inactive_scope() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let scope = ScopedManagedLoad::from_sources(&mut sources, 0x12345678).unwrap();
    let load = scope.begin(TaskKind::Normal).unwrap();
    assert!(load.observer_present().unwrap());
    assert_eq!(load.initialization().uvars_path, "UVars.tgi");
    assert_eq!(load.initialization().elapsed_millis, 0x12345678);
    load.notify_observer(0xdeadbeef).unwrap();
}
