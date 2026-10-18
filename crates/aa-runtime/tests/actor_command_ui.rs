use aa_runtime::actor_command_ui::{self as ui, Fields};
use serde_json::Value;
use std::path::PathBuf;

fn fixture(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-player-orders/command-ui")
        .join(name);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
struct RawFields<'a> {
    input: &'a Value,
    reads: Vec<&'static str>,
}
impl RawFields<'_> {
    fn get(&mut self, name: &'static str) -> ui::Result<u32> {
        self.reads.push(name);
        if self.input["stop"].as_str() == Some(name) {
            return Err(name.into());
        }
        let i = ui::BOOLEAN_FIELDS
            .iter()
            .position(|n| *n == name)
            .unwrap_or(6);
        Ok(self.input["fields"][i].as_u64().unwrap() as u32)
    }
}
impl Fields for RawFields<'_> {
    fn boolean(&mut self, name: &'static str) -> ui::Result<bool> {
        self.get(name).map(|v| v != 0)
    }
    fn integer(&mut self, name: &'static str) -> ui::Result<i32> {
        self.get(name).map(|v| v as i32)
    }
}
#[test]
fn every_native_control_branch_and_getter_stop_retains_exact_stores() {
    let doc = fixture("native-ui-control.json");
    let mut checked = 0;
    for row in doc["rows"].as_array().unwrap() {
        let input = &row["input"];
        let mut fields = RawFields {
            input,
            reads: Vec::new(),
        };
        let mut state = ui::LoadState::default();
        let result = ui::load_fields(
            &mut state,
            &mut fields,
            input["kind"].as_u64().unwrap() as u32,
            input["behavior"].as_u64().unwrap() as u32,
        );
        let status = result.as_ref().err().map_or("complete", String::as_str);
        assert_eq!(status, row["output"]["status"], "{row}");
        let bytes: Vec<u8> = state
            .written_bytes()
            .iter()
            .map(|b| b.map_or(0xcd, u8::from))
            .collect();
        assert_eq!(serde_json::json!(bytes), row["output"]["bytes"], "{row}");
        assert_eq!(
            u64::from(state.written_position().map_or(0xcdcd_cdcd, |v| v as u32)),
            row["output"]["position"],
            "{row}"
        );
        assert_eq!(serde_json::json!(fields.reads), row["output"]["reads"]);
        assert_eq!(state.completed().is_ok(), result.is_ok());
        checked += 1;
    }
    assert_eq!(checked, 648);
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn actual_canonical_source_controls_match_all_39_commands_in_both_native_images() {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use aa_runtime::{
        catalog::Catalog, combat_source_completion as completion,
        combat_source_receipt::VerifiedStartupReceipt, settings::Settings,
    };
    let game = PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let doc = fixture("native-actual-controls.json");
    for file in doc["source_files"].as_array().unwrap() {
        let path = file["path"].as_str().unwrap();
        assert_eq!(sources.read(path).unwrap().sha256, file["sha256"]);
    }
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(settings);
    assert!(ui::Source::bind(&catalog, "move").is_err());
    let receipt =
        VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut sources).unwrap();
    completion::install(
        &catalog,
        &mut sources,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    completion::compute(&catalog).unwrap();
    let mut checked = 0;
    for run in doc["runs"].as_array().unwrap() {
        for row in run["authored"].as_array().unwrap() {
            let ids = row["ids"].as_str().unwrap();
            let source = ui::Source::bind(&catalog, ids).unwrap();
            let definition = source.definition();
            assert_eq!(
                row["before"],
                serde_json::json!([
                    definition.mind_type,
                    definition.component_slot,
                    definition.behavior_flags
                ])
            );
            assert_eq!(row["before"], row["after"]);
            let controls = source.controls();
            let mut native: Vec<u32> = controls.bytes_3c_41().into_iter().map(u32::from).collect();
            native.push(controls.preferred_position() as u32);
            assert_eq!(
                serde_json::json!(native),
                row["ui_flags_3c_to_41_and_position_44"],
                "{ids} {run}"
            );
            assert_eq!(source.inputs().len(), 7);
            assert_eq!(
                source.node_sources().first(),
                Some(source.definition_source())
            );
            assert_eq!(
                source.definition_source().path,
                row["path"].as_str().unwrap()
            );
            assert_eq!(source.source_precision(), aa_numeric::Precision::Double53);
            checked += 1;
        }
    }
    assert_eq!(checked, 234);
    assert!(ui::Source::bind(&catalog, "not_a_command").is_err());
    assert!(ui::Source::bind(&catalog, " move").is_err());
    assert_eq!(
        ui::Source::bind(&catalog, "MOVE").unwrap().definition().id,
        "move"
    );
}
