//! Fresh actual-source Header owner. Time fixtures are explicit native inputs;
//! the final constructor call additionally uses the real Windows clock leaf.
use aa_runtime::{
    film_header_fields::Destination,
    film_header_owner::{Fields, State},
};
use serde_json::{Value, json};
#[path = "../../aa-client/src/windows_film_time.rs"]
mod windows_film_time;

fn fields(value: &Fields) -> Value {
    json!({
        "words": {
            "12":value.words_0c_10()[0],"16":value.words_0c_10()[1],"20":value.word_14(),
            "24":value.local_filetime()[0],"28":value.local_filetime()[1],
            "48":value.extents_bits()[0],"52":value.extents_bits()[1],
            "64":value.campaign_index(),"72":value.entries().len(),
            "80":value.build_bits(),"84":value.numeric_version()
        },
        "byte64":value.byte_64(),
        "strings": {
            "32":value.name().key(),"36":value.name().text(),
            "40":value.description().key(),"44":value.description().text(),
            "56":value.recording_name(),"60":value.campaign_name(),
            "88":value.version_text(),"92":value.application(),"96":value.release()
        }
    })
}
fn compare_native_fields(actual: &Fields, native: &Value) {
    let actual = fields(actual);
    assert_eq!(actual["strings"], native["strings"]);
    assert_eq!(actual["byte64"], native["byte64"]);
    for (key, value) in actual["words"].as_object().unwrap() {
        assert_eq!(value, &native["words"][key], "word{key}")
    }
}
#[test]
#[ignore = "requires actual AA_GAME_ROOT source completion and Windows clock"]
fn actual_fresh_header_keeps_failure_prefix_and_independent_current_owners() {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use aa_runtime::{
        catalog::Catalog, combat_source_completion as completion,
        combat_source_receipt::VerifiedStartupReceipt, film_process_metadata,
        load_view_variables::NativeOverrides,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(aa_runtime::settings::Settings::load_v14(&mut sources, Limits::default()).unwrap());
    catalog.native_load_mode = Some(1);
    let mut process = film_process_metadata::State::from_fresh_sources(
        &catalog,
        &mut sources,
        NativeOverrides::ControlledNone,
    )
    .unwrap();
    let native: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film-header-constructor/native-header-constructor.json"
    ))
    .unwrap();
    let find = |stop: &str, count: u64| {
        native["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                row["precision"] == "Double53"
                    && row["input"]["stop"] == stop
                    && row["input"]["count"] == count
            })
            .unwrap()
    };
    let failed = {
        let bank = catalog.definition_bank.borrow();
        State::from_film_constructor(&catalog, &bank, &mut process, || {
            Err("required actual6FB1F0".into())
        })
        .err()
        .unwrap()
    };
    assert_eq!(failed.error(), "required actual6FB1F0");
    assert!(failed.rules().is_none() && failed.parameters().is_none());
    compare_native_fields(failed.fields(), &find("clock", 0)["output"]);
    assert_eq!(process.constructed_headers(), 1);
    //No source-completed Bank exists yet. This genuine required-source error
    //occurs after time/byte64 stores and does not roll the process count back.
    let explicit_time = [0x12345678, 0x9abcdef0];
    let failed_rules = {
        let bank = catalog.definition_bank.borrow();
        State::from_film_constructor(&catalog, &bank, &mut process, || Ok(explicit_time))
            .err()
            .unwrap()
    };
    assert!(failed_rules.rules().is_none() && failed_rules.parameters().is_none());
    compare_native_fields(failed_rules.fields(), &find("rules", 1)["output"]);
    assert_eq!(process.constructed_headers(), 2);
    let proof = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut sources).unwrap();
    completion::install(
        &catalog,
        &mut sources,
        proof.context(),
        proof.csv(),
        proof.globals(),
    )
    .unwrap();
    completion::compute(&catalog).unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let inventory = aa_mapworld::WorldInventory::decode(&map, u64::MAX).unwrap();
    let current = aa_runtime::load_world_rules::Owner::load(&catalog, &inventory).unwrap();
    let bank = catalog.definition_bank.borrow();
    let mut a =
        State::from_film_constructor(&catalog, &bank, &mut process, || Ok(explicit_time)).unwrap();
    let b =
        State::from_film_constructor(&catalog, &bank, &mut process, || Ok(explicit_time)).unwrap();
    compare_native_fields(a.fields(), &find("none", 1)["output"]);
    assert_eq!(process.constructed_headers(), 4);
    assert!(a.parameters().teams().next().is_none() && a.parameters().kingdom_count() == 0);
    assert!(b.parameters().teams().next().is_none() && b.parameters().kingdom_count() == 0);
    assert!(!std::ptr::eq(a.rules(), b.rules()));
    assert!(!std::ptr::eq(a.parameters(), b.parameters()));
    let a_first = a.rules().records().next().unwrap() as *const _;
    let b_first = b.rules().records().next().unwrap() as *const _;
    assert_ne!(a_first, b_first);
    let untouched_b = b.rules().to_serialized();
    Destination::set_rule_mode(&mut a, 0x17);
    Destination::set_game_type(&mut a, Vec::new()); //Actual nullable raw copy, no Find.
    Destination::set_rule_extra(&mut a, 0x89abcdef);
    current
        .copy_records_into(Destination::rules(&mut a), &catalog, &bank)
        .unwrap();
    let copied = a.rules().to_serialized();
    assert_eq!(copied.mode, 0x17);
    assert!(copied.game_type.is_empty());
    assert_eq!(a.rules().changes(), 0x89abcdef);
    let expected = current.to_serialized();
    assert_eq!(
        serde_json::to_value(&copied.rules).unwrap(),
        serde_json::to_value(&expected.rules).unwrap()
    );
    assert_eq!(
        serde_json::to_value(b.rules().to_serialized()).unwrap(),
        serde_json::to_value(untouched_b).unwrap()
    );
    //The platform sampler executes now, after actual metadata stores. It is
    //not replaced by a saved timestamp or simulation-clock conversion.
    let real = State::from_film_constructor(
        &catalog,
        &bank,
        &mut process,
        windows_film_time::local_file_time,
    )
    .unwrap();
    assert_ne!(real.fields().local_filetime(), [0, 0]);
    assert_eq!(process.constructed_headers(), 5);
    assert_eq!(real.fields().byte_64(), Some(0));
}
