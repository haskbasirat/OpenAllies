use aa_numeric::Precision;
use aa_rules::{
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use aa_runtime::settings::Settings;

fn input(path: &str, text: &str) -> ResolvedFile {
    let bytes = text.as_bytes().to_vec();
    ResolvedFile {
        logical_path: path.into(),
        origin: Origin::V14Loose,
        sha256: sha256(&bytes),
        bytes,
    }
}
#[test]
fn source_settings_do_not_guess_override_order_or_field_type() {
    let text = "[SVars]\n{\nfixed GameTicksPerGameSecond = 16\nint NotFixed = 7\n}\n";
    let single = Settings::from_files([input("one.tgi", text)], Limits::default()).unwrap();
    let (bits, source) = single
        .unique_fixed("GameTicksPerGameSecond", Precision::Double53)
        .unwrap();
    assert_eq!(bits, 16f32.to_bits());
    assert_eq!(source.path, "one.tgi");
    assert!(
        single
            .unique_fixed("NotFixed", Precision::Double53)
            .is_err()
    );
    let ambiguous = Settings::from_files(
        [input("one.tgi", text), input("two.tgi", text)],
        Limits::default(),
    )
    .unwrap();
    assert!(
        ambiguous
            .unique_fixed("GameTicksPerGameSecond", Precision::Double53)
            .is_err()
    );
}
#[test]
#[ignore = "requires AA_GAME_ROOT original archive and v14 source overlay"]
fn original_v14_tick_rate_builds_native_world_queue_plan() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let (rate, source) = settings
        .unique_fixed("GameTicksPerGameSecond", Precision::Double53)
        .unwrap();
    assert_eq!(rate, 16f32.to_bits());
    assert_eq!(source.path, "Game/SVars.tgi");
    let (supply, source) = settings.unit_supply_enabled().unwrap();
    assert!(supply);
    assert_eq!(source.path, "Game/SVars_AA.tgi");
    let plan =
        aa_world::initialization::ClockPlan::from_tick_rate(rate, Precision::Double53).unwrap();
    assert_eq!(plan.step_bits, 0.0625f32.to_bits());
    assert_eq!(plan.queues.map(|q| q.bucket_count), [1, 16, 80, 80, 80]);
}
