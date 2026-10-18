//! Production public-owner integration. The clock samples here are declared
//!laboratory inputs; exact Windows OS producer tests run separately.
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
use aa_runtime::{
    catalog::Catalog,
    load_view_variables::NativeOverrides,
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    standalone_network::{Clock, TimerAnchor},
};
struct Samples(u32);
impl Clock for Samples {
    fn start_timer(&mut self) -> Result<TimerAnchor, String> {
        self.0 += 1;
        TimerAnchor::from_os_counter(900000000 + i64::from(self.0), 10_000_000)
    }
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source files"]
fn actual_current_manager_field_and_completed_standalone_move_with_same_application() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let catalog = Catalog::new(&actors);
    let mut app = ScopedManagedLoad::from_sources(&mut files, 300).unwrap();
    assert_eq!(app.game_spy_manager_gate().unwrap().current_byte_bc(), 0);
    assert!(!app.game_spy_manager_gate().unwrap().internet_enabled_bc());
    assert!(app.standalone().is_err());
    let mut samples = Samples(0);
    app.select_local_offline(
        &catalog,
        &mut files,
        NativeOverrides::ControlledNone,
        &mut samples,
    )
    .unwrap();
    let current = app.standalone().unwrap();
    current.require_source_owner(&catalog).unwrap();
    assert!(!current.online_manager_present().unwrap());
    assert_eq!(current.browser_period().unwrap(), Some(50));
    assert_eq!(current.query_period_bits().unwrap(), Some(50f32.to_bits()));
    let source_allocation = current.source_files().as_ptr();
    let mut load = app.begin(TaskKind::Normal).unwrap();
    assert_eq!(
        load.standalone().unwrap().source_files().as_ptr(),
        source_allocation
    );
    assert!(!load.game_spy_manager_gate().internet_enabled_bc());
    let result: Result<(), String> = load.with_startup_engine(|_| {
        Err("deliberate stopped startup, never an activation witness".into())
    });
    assert!(result.is_err());
    let startup = load.into_startup_owner().unwrap();
    assert_eq!(
        startup.standalone().unwrap().source_files().as_ptr(),
        source_allocation
    );
    assert_eq!(startup.game_spy_manager_gate().current_byte_bc(), 0);
    assert_eq!(samples.0, 2);
}
