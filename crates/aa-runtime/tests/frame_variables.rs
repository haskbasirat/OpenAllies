use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use aa_runtime::{
    catalog::Catalog, combat_source_completion, combat_source_receipt::VerifiedStartupReceipt,
    frame_variables::State, load_view_variables::NativeOverrides, settings::Settings,
};
#[test]
#[ignore = "requires actual AA_GAME_ROOT archives and verified target source"]
fn actual_frame_variables_keep_source_context_and_current_mutation_in_one_owner() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let mut actors = ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(Settings::load_v14(&mut files, Limits::default()).unwrap());
    let receipt = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut files).unwrap();
    combat_source_completion::install(
        &catalog,
        &mut files,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    assert!(State::from_fresh_sources(&catalog, &mut files, NativeOverrides::Unbound).is_err());
    let mut state =
        State::from_fresh_sources(&catalog, &mut files, NativeOverrides::ControlledNone).unwrap();
    assert_eq!(state.auto_pause_byte(), 1);
    assert_eq!(state.render_dump_avi_byte(), 0);
    assert_eq!(state.render_avi_frames_per_tick(), 2);
    assert_eq!(state.async_ticks_per_second_bits(), 0x41800000);
    assert!(!state.source_files().is_empty());
    assert!(!state.source_assignments().is_empty());
    state.validate(&catalog).unwrap();
    state.assign_auto_pause_byte(128);
    state.assign_render_dump_avi_byte(255);
    state.assign_render_avi_frames_per_tick(-7);
    assert_eq!(state.auto_pause_byte(), 128);
    assert_eq!(state.render_dump_avi_byte(), 255);
    assert_eq!(state.render_avi_frames_per_tick(), -7);
    drop(catalog);
    actors.sources[0].sha256 = "changed source universe".into();
    assert!(state.validate(&Catalog::new(&actors)).is_err());
}
