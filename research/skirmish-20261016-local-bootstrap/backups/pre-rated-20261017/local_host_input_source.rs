//! Real accepted rules/text and Windows reset/QPC meet actual retained Host
//! inputs and initial selected Header90 GameSettings copy. This stops before
//!7791F0/77A950; no World/battle or ScenarioFind protocol is claimed.
#[path = "../../aa-client/src/windows_reset_os.rs"]
mod windows_reset_os;

#[test]
#[ignore = "requires original Data.rwd, accepted v14 source and real Windows via AA_GAME_ROOT"]
fn real_source_windows_and_explicit_host_input_use_same_owners() {
    use aa_netcode::{
        lobby::GameOptions,
        local_bootstrap::{
            self, InitialProperties, LocalBootstrap, Notifications,
            launch::{Effects, HostInput, InputStage},
        },
        local_host::NodeTime,
    };
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use aa_runtime::{
        catalog::Catalog, combat_source_completion as completion,
        combat_source_receipt::VerifiedStartupReceipt, load_world_globals::SessionPhase,
        localization::Localization, settings::Settings,
    };
    struct Diagnostics(Vec<u32>);
    impl Effects for Diagnostics {
        fn mode_rejected_counting_down(&mut self) -> aa_netcode::Result<()> {
            panic!("fresh reset countdown is actually zero")
        }
        fn set_new_state_diagnostic(&mut self, state: u32) -> aa_netcode::Result<()> {
            self.0.push(state);
            Ok(())
        }
    }
    impl local_bootstrap::assignment::Effects for Diagnostics {
        fn assigning(
            &mut self,
            _: u32,
            _: &[u16],
            _: &[u16],
            _: &[u16],
            _: &[u16],
        ) -> aa_netcode::Result<()> {
            self.0.push(0x778d10);
            Ok(())
        }
        fn check_nation(&mut self, _: &[u16], _: &[u16]) -> aa_netcode::Result<()> {
            self.0.push(0x777cd0);
            Ok(())
        }
        fn check_faction(&mut self, _: &[u16], _: &[u16]) -> aa_netcode::Result<()> {
            self.0.push(0x777610);
            Ok(())
        }
    }
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
    let mut gate = || {
        Err(aa_netcode::Error::Unsupported(
            "actual disabled logger must not read its unbound gate",
        ))
    };
    let mut os = windows_reset_os::WindowsResetOs::new(&mut gate);
    let mut clock = NodeTime::from_native_constructor(
        windows_reset_os::PerformanceTimer::from_native_constructor().unwrap(),
    );
    clock
        .begin_update_prefix(aa_numeric::Precision::Double53)
        .unwrap();
    let mut owner = LocalBootstrap::from_native_constructors();
    assert!(owner.seed_override_100().is_err());
    let selected_map = source
        .catalog()
        .into_iter()
        .find(|entry| entry.logical_path.to_ascii_lowercase().ends_with(".rmp"))
        .expect("actual accepted source archive must contain an original map")
        .logical_path;
    assert!(!source.read(&selected_map).unwrap().bytes.is_empty());
    let variables = aa_runtime::creator_variables::State::from_fresh_sources(
        &catalog,
        &mut source,
        aa_runtime::load_view_variables::NativeOverrides::ControlledNone,
    )
    .unwrap();
    owner
        .bind_load_creator_constructor(&catalog, &variables)
        .unwrap();
    let map =
        aa_mapworld::MapArchive::from_bytes(source.read(&selected_map).unwrap().bytes).unwrap();
    let streams = aa_runtime::map_resource_streams::Owner::open(map);
    let selected_resource = aa_runtime::resource_creator::SourceResource::load(
        &catalog,
        &streams,
        &selected_map.encode_utf16().collect::<Vec<_>>(),
    )
    .unwrap();
    let creator_identity = owner.selected_load_creator().unwrap() as *const _ as usize;
    let mut state = local_bootstrap::constructor_launch_state();
    state.kind = 12;
    let input = HostInput::from_ui_choices(
        state,
        2,
        2,
        false,
        GameOptions {
            name: "Source acceptance".encode_utf16().collect(),
            password: vec![],
            max_players: 100,
            field_d8: u32::MAX,
            allow_observers: true,
        },
        Some(selected_map.encode_utf16().collect()),
    );
    owner
        .reset_node_fresh_with_host_input(
            input,
            &mut os,
            &InitialProperties::from_native_constructor(),
            &Notifications::new(0),
        )
        .unwrap();
    let player = owner
        .create_first_local_player(
            &"Host input acceptance".encode_utf16().collect::<Vec<_>>(),
            &[],
            &text,
        )
        .unwrap();
    owner
        .finish_fresh_self_host(&catalog, &mut clock, aa_numeric::Precision::Double53)
        .unwrap();
    let admin = owner.order_node().unwrap() as *const _ as usize;
    let mut diagnostics = Diagnostics(Vec::new());
    owner.install_host_launch_fields(&mut diagnostics).unwrap();
    assert_eq!(
        owner.host_input_stage().unwrap(),
        InputStage::FieldsInstalled
    );
    assert_eq!(owner.launch_mode(), 2);
    assert_eq!(owner.launch_game_type(), 2);
    assert_eq!(diagnostics.0, [2]);
    assert_eq!(owner.order_node().unwrap() as *const _ as usize, admin);
    assert_eq!(owner.current_phase().unwrap(), 0);
    assert_eq!(owner.application_state().unwrap(), 0);
    owner.install_host_selected_file(&mut diagnostics).unwrap();
    assert_eq!(owner.host_input_stage().unwrap(), InputStage::FileInstalled);
    assert_eq!(
        owner.launch_file_5c(),
        selected_map.encode_utf16().collect::<Vec<_>>()
    );
    assert_eq!(owner.current_phase().unwrap(), 0);
    assert_eq!(owner.application_state().unwrap(), 0);
    owner
        .install_host_selected_game_settings(&catalog, &selected_resource)
        .unwrap();
    assert_eq!(
        owner.host_input_stage().unwrap(),
        InputStage::SettingsInstalled
    );
    assert_eq!(
        owner.selected_load_creator().unwrap() as *const _ as usize,
        creator_identity
    );
    assert_eq!(
        owner.current_game_settings_68().unwrap().parameters(),
        &selected_resource.header().parameters
    );
    assert_eq!(
        owner.selected_load_creator().unwrap().parameters(),
        owner.current_game_settings_68().unwrap().parameters()
    );
    assert!(
        owner
            .selected_load_creator()
            .unwrap()
            .source_resource()
            .is_none()
    );
    assert_eq!(owner.current_phase().unwrap(), 0);
    assert_eq!(owner.application_state().unwrap(), 0);
    let find = |manager: &str, ids: &[u16]| {
        let n = ids.iter().position(|&u| u == 0).unwrap_or(ids.len());
        if n == 0 {
            return None;
        }
        let text = String::from_utf16(&ids[..n]).unwrap();
        catalog
            .actors
            .native_data_registry(manager)
            .unwrap()
            .find(&text)
            .map(|f| f.ids.encode_utf16().collect::<Vec<_>>())
    };
    let refs:Vec<_>=owner.selected_load_creator().unwrap().parameters().kingdoms.iter().map(|p| {
        serde_json::json!({"nation":find("Nation",&p.nation),"faction":find("Faction",&p.faction),"handicap":find("Handicap",&p.handicap)})
    }).collect();
    let input = serde_json::json!({"resource_ids":selected_resource.ids(),"source":selected_resource.source(),
        "sources":catalog.actors.sources,"parameters":owner.selected_load_creator().unwrap().parameters(),
        "base_parameters":owner.current_game_settings_68().unwrap().parameters(),"references":refs,
        "player":owner.snapshot().players[0],"changed_c8":owner.snapshot().changed[0]});
    owner
        .refresh_host_players_7791f0(&catalog, &mut diagnostics)
        .unwrap();
    assert_eq!(
        owner.host_input_stage().unwrap(),
        InputStage::PlayersRefreshed
    );
    assert_eq!(
        owner.selected_load_creator().unwrap() as *const _ as usize,
        creator_identity
    );
    let state:Vec<_>=owner.selected_load_creator().unwrap().parameters().kingdoms.iter().enumerate().map(|(i,p)|serde_json::json!({
        "nation":find("Nation",&p.nation),"faction":find("Faction",&p.faction),"handicap":find("Handicap",&p.handicap),
        "modified68":owner.selected_load_creator().unwrap().parameter_modified(i).unwrap()})).collect();
    let p = owner.snapshot().players.remove(0);
    let after = serde_json::json!({"parameter_state":state,"player":{"kingdom":p["kingdom"],"flag_2c":p["flag_2c"],
        "flag_38":p["flag_38"],"field_3c":p["field_3c"],"field_40":p["field_40"]},"changed_c8":owner.snapshot().changed[0]});
    let folder = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-local-bootstrap/player-refresh");
    if std::env::var_os("AA_CAPTURE_REFRESH").is_some() {
        //Explicit evidence capture mode; this is not a native-parity pass.
        std::fs::write(
            folder.join("actual-source-input.json"),
            serde_json::to_vec_pretty(&serde_json::json!({"input":input,"rust_after":after}))
                .unwrap(),
        )
        .unwrap();
    } else {
        let native: serde_json::Value = serde_json::from_slice(
            &std::fs::read(folder.join("native-source-refresh.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(native["passed"], true);
        assert_eq!(native["input"], input);
        assert_eq!(native["cases"].as_array().unwrap().len(), 6);
        for row in native["cases"].as_array().unwrap() {
            assert_eq!(row["after"], after);
        }
    }
    assert_eq!(owner.current_phase().unwrap(), 0);
    assert_eq!(owner.application_state().unwrap(), 0);
    {
        let mut loan = owner.runtime_borrow().unwrap();
        assert!(
            *loan.frame.running_2c0 && *loan.frame.has_clock_158 && *loan.frame.server_is_self_2c8
        );
        assert_eq!(*loan.frame.admin_time_28c, 0);
        assert_eq!(*loan.frame.heartbeat_2dc, 0);
        loan.players
            .set_seed_7710e0(0x12345678, &mut diagnostics)
            .unwrap();
        assert_eq!(loan.players.seed_override_100().unwrap(), 0x12345678);
        assert!(loan.players.is_local_player(&player).unwrap());
    }
    assert_eq!(owner.seed_override_100().unwrap(), 0x12345678);
    assert!(owner.require_host_launch().is_err());
}
