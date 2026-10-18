//! The real aa-runtime library crate, real aa-netcode owner and real Windows
//! inputs meet here. Keeping this external avoids lib-test duplicate trait IDs.
#[path = "../../aa-client/src/windows_reset_os.rs"]
mod windows_reset_os;

#[test]
#[ignore = "requires original Data.rwd and accepted v14 sources via AA_GAME_ROOT"]
fn actual_source_and_windows_os_construct_the_retained_local_player() {
    use aa_netcode::local_bootstrap::{InitialProperties, LocalBootstrap, Notifications, Progress};
    use aa_rules::{
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use aa_runtime::{
        load_tail_world::SessionRead, load_world_globals::SessionPhase, localization::Localization,
    };
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let caption: Vec<u16> = "static_group_wildcard_name".encode_utf16().collect();
    let expected = text.display(&caption).unwrap();
    let mut required_gate = || {
        Err(aa_netcode::Error::Unsupported(
            "actual logger gate not bound in initial disabled scope",
        ))
    };
    let mut os = windows_reset_os::WindowsResetOs::new(&mut required_gate);
    let queue = Notifications::new(0); // explicit interactive test application owner
    let mut owner = LocalBootstrap::from_native_constructors();
    owner
        .reset_fresh(
            &mut os,
            &InitialProperties::from_native_constructor(),
            &queue,
        )
        .unwrap();
    assert_eq!(queue.pop().unwrap().unwrap().kind, 2);
    assert_eq!(owner.last_player_id().unwrap(), 0);
    assert!(!owner.reset_guid_text().is_empty());
    // These are explicit replacement UI/profile inputs, not map Kingdom names.
    let nickname: Vec<u16> = "Local source acceptance".encode_utf16().collect();
    let profile_key: Vec<u16> = "local-bootstrap-source-test".encode_utf16().collect();
    let player = owner
        .create_first_local_player(&nickname, &profile_key, &text)
        .unwrap();
    assert_eq!(owner.progress(), Progress::LocalPlayerCreated);
    assert_eq!(owner.player_status(&player).unwrap(), 0);
    assert_eq!(owner.player_name(&player).unwrap(), nickname);
    assert!(
        SessionRead::player_name_24(&owner, &player)
            .unwrap()
            .is_empty()
    );
    assert!(SessionRead::player_kingdom_28(&owner, &player).is_err());
    assert_eq!(owner.current_phase().unwrap(), 0);
    assert_eq!(owner.application_state().unwrap(), 0);
    assert_eq!(
        owner.player_view_snapshot(&player).unwrap()["group"]["title"],
        serde_json::json!(expected)
    );
    // Actual low-level native765E10 null writer, with no made-up Kingdom.
    // This fixture call does not claim773E30 BeginPlaying was completed.
    owner
        .replace_kingdom_765e10(
            &player,
            None,
            &aa_runtime::actor_world::ActorStore::new(),
            &text,
        )
        .unwrap();
    assert_eq!(
        SessionRead::player_kingdom_28(&owner, &player).unwrap(),
        None
    );
    assert!(owner.selection(&player).unwrap().is_empty());
    assert!(owner.require_host_launch().is_err());
    if let Some(path) = std::env::var_os("AA_LOCAL_BOOTSTRAP_OBSERVATION") {
        let report = serde_json::json!({"passed":true,"scope":"Real Windows tick/GUID, real accepted localization, actual reset and local-player constructor prefix; no Host/launch/Kingdom binding claim",
            "owner":owner.snapshot(),"player_view":owner.player_view_snapshot(&player).unwrap(),
            "caption_source":text.declarations.get("static_group_wildcard_name").unwrap()});
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}

#[test]
#[ignore = "requires accepted actual v14 sources and embedded verified native startup reference via AA_GAME_ROOT"]
fn actual_source_and_windows_os_bind_one_fresh_local_host_prefix() {
    use aa_netcode::{
        lobby::PeerLink,
        local_bootstrap::{InitialProperties, LocalBootstrap, Notifications, Progress},
        local_host::NodeTime,
    };
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use aa_runtime::{
        catalog::Catalog, combat_source_completion as completion,
        combat_source_receipt::VerifiedStartupReceipt, localization::Localization,
        settings::Settings,
    };
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut source = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut source).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(Settings::load_v14(&mut source, Limits::default()).unwrap());
    let text = Localization::load_v14(&mut source, Limits::default()).unwrap();
    let proof = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut source).unwrap();
    completion::install(
        &catalog,
        &mut source,
        proof.context(),
        proof.csv(),
        proof.globals(),
    )
    .unwrap();
    completion::compute(&catalog).unwrap();
    assert_eq!(
        aa_runtime::load_host_settings::admin_ticks_per_second(&catalog).unwrap(),
        0x41800000
    );
    assert!(
        !catalog
            .settings
            .as_ref()
            .unwrap()
            .declarations
            .contains_key("admintickspersecond")
    );
    // Explicit forged legacy row: real registered default has no authored row.
    let mut forged = catalog
        .settings
        .as_ref()
        .unwrap()
        .declarations
        .values()
        .find_map(|rows| rows.first())
        .unwrap()
        .clone();
    forged.field_type = Some("Fixed".into());
    forged.raw = "17".into();
    catalog
        .settings
        .as_mut()
        .unwrap()
        .declarations
        .insert("admintickspersecond".into(), vec![forged]);
    assert!(
        aa_runtime::load_host_settings::admin_ticks_per_second(&catalog)
            .unwrap_err()
            .contains("value/span")
    );
    catalog
        .settings
        .as_mut()
        .unwrap()
        .declarations
        .remove("admintickspersecond");
    let retained = catalog.settings.take();
    assert!(aa_runtime::load_host_settings::admin_ticks_per_second(&catalog).is_err());
    catalog.settings = retained;
    let mut gate = || {
        Err(aa_netcode::Error::Unsupported(
            "unused disabled bootstrap log gate",
        ))
    };
    let mut os = windows_reset_os::WindowsResetOs::new(&mut gate);
    let mut timing = NodeTime::from_native_constructor(
        windows_reset_os::PerformanceTimer::from_native_constructor().unwrap(),
    );
    assert_eq!(timing.cached_seconds_bits(), 0x5368d4a5);
    timing
        .begin_update_prefix(aa_numeric::Precision::Double53)
        .unwrap();
    let mut owner = LocalBootstrap::from_native_constructors();
    owner
        .reset_node_fresh(
            &mut os,
            &InitialProperties::from_native_constructor(),
            &Notifications::new(0),
        )
        .unwrap();
    owner
        .create_first_local_player(
            &"Source Host Prefix".encode_utf16().collect::<Vec<_>>(),
            &[],
            &text,
        )
        .unwrap();
    owner
        .finish_fresh_self_host(&catalog, &mut timing, aa_numeric::Precision::Double53)
        .unwrap();
    assert_eq!(owner.progress(), Progress::LocalHostCreated);
    assert_eq!(owner.host(), PeerLink::Node);
    assert_eq!(owner.host_node().unwrap().clock().words()[0], 0x3d800000);
    assert_eq!(owner.order_node().unwrap().requests.begin(), 0);
    assert_eq!(owner.order_node().unwrap().decrees.begin(), 0);
    assert_eq!(owner.sync_a_cell().borrow().hash, 0);
    assert!(owner.require_host_launch().is_err());
    if let Some(path) = std::env::var_os("AA_LOCAL_HOST_OBSERVATION") {
        let report = serde_json::json!({"passed":true,"scope":"Actual source Bank precision and SVar, real Windows reset/QPC, same retained Node/OrderAdmin/SyncA through75B20E; selected modes/settings/77A950/BeginPlaying/battle not run","owner":owner.snapshot(),"node":owner.host_node().unwrap().snapshot(),"clock_words":owner.host_node().unwrap().clock().words(),"cached_time_58":timing.cached_seconds_bits()});
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file, &report).unwrap();
    }
}
