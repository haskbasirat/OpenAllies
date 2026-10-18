use aa_numeric::Precision;
use aa_rules::{
    actors::{
        ActorCatalog,
        native_load::{NativeLoadReport, load_files},
    },
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use aa_runtime::{
    catalog::Catalog,
    combat_value_source_context::{
        CsvScope, NUMERIC_SETTINGS, OverrideFile, SourceFpContext, bind_csv_files,
        bind_csv_from_sources, bind_numeric,
    },
    settings::Settings,
};
use serde_json::{Value, json};

fn input(path: &str, text: &str) -> ResolvedFile {
    let bytes = text.as_bytes().to_vec();
    ResolvedFile {
        logical_path: path.into(),
        origin: Origin::V14Loose,
        sha256: sha256(&bytes),
        bytes,
    }
}
fn context(pc: Precision) -> SourceFpContext {
    SourceFpContext::supplied_diagnostic(
        match pc {
            Precision::Single24 => 0x7f,
            Precision::Double53 => 0x27f,
            Precision::Extended64 => 0x37f,
        },
        0,
        0x1f80,
        true,
        "explicit source-PC test fixture, not native startup capture",
    )
    .unwrap()
}
fn pc(row: &Value) -> Precision {
    match row["precision"].as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        _ => panic!("pc"),
    }
}
fn base(changed: Option<(&str, &str)>) -> String {
    let mut text = "[SVars]\n{\n".to_string();
    for (name, _) in NUMERIC_SETTINGS {
        let value = changed
            .filter(|(target, _)| *target == name)
            .map(|(_, v)| v)
            .unwrap_or("1");
        text.push_str(&format!("fixed {name} = {value}\n"));
    }
    text.push_str("}\n");
    text
}
fn source(base: &str, overlay: &str, rule: &str) -> ActorCatalog {
    load_files(
        vec![
            input("Game/SVars.tgi", base),
            input("Game/SVars_AA.tgi", overlay),
            input("Game/test-rules.tgi", rule),
        ],
        Limits::default(),
        &mut NativeLoadReport::default(),
    )
}

#[test]
fn typed_cv_inputs_match_native_overwrites_without_constructor_defaults() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-combat-source-context/native-context.json"
    ))
    .unwrap();
    let mut count = 0;
    for row in doc["numeric_cases"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let states = row["states"].as_array().unwrap();
        let first = states[0]["raw"].as_str().unwrap();
        let base = base(Some((name, first)));
        let overlay = if states.len() > 1 {
            format!(
                "[SVars]\n{{\nfixed {name} = {}\n}}\n",
                states.last().unwrap()["raw"].as_str().unwrap()
            )
        } else {
            "[SVars]\n{\n}\n".into()
        };
        let actors = source(&base, &overlay, "");
        let mut catalog = Catalog::new(&actors);
        catalog.settings = Some(
            Settings::from_files(
                [
                    input("Game/SVars.tgi", &base),
                    input("Game/SVars_AA.tgi", &overlay),
                ],
                Limits::default(),
            )
            .unwrap(),
        );
        let context = context(pc(row));
        let bound = bind_numeric(&catalog, &context).unwrap();
        assert_eq!(
            bound.value(name).unwrap().bits,
            states.last().unwrap()["bits"].as_u64().unwrap() as u32,
            "{row}"
        );
        assert_eq!(bound.values().len(), 10);
        assert!(!bound.context().is_observed_native_startup());
        assert_eq!(
            bound.value(name).unwrap().declarations.len(),
            if states.len() > 1 { 2 } else { 1 }
        );
        bound.formula_settings().unwrap();
        bound.mean_discard().unwrap();
        bound.ranged_minimum().unwrap();
        bound.defense_weight_ranged().unwrap();
        count += 1;
    }
    assert_eq!(count, 180);
}

#[test]
fn csv_scope_and_order_match_native_without_importing_profiles() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-combat-source-context/native-context.json"
    ))
    .unwrap();
    for row in doc["csv_cases"].as_array().unwrap() {
        let states = row["states"].as_array().unwrap();
        let roots = vec![input(
            "UVars.tgi",
            &format!(
                "[Vars]\n{{\nstring CombatValueCSV = {}\n}}\n",
                states[0]["raw"].as_str().unwrap()
            ),
        )];
        let overrides: Vec<_> = states
            .iter()
            .skip(1)
            .enumerate()
            .map(|(i, s)| {
                input(
                    &format!("laboratory-override-{i}.tgi"),
                    &format!(
                        "[Vars]\n{{\nstring CombatValueCSV = {}\n}}\n",
                        s["raw"].as_str().unwrap()
                    ),
                )
            })
            .collect();
        let override_inputs: Vec<_> = overrides
            .iter()
            .zip(states.iter().skip(1))
            .map(|(file, state)| OverrideFile {
                file,
                native_load_flags: state["load_flags"].as_u64().unwrap_or(0) as u32,
            })
            .collect();
        let value = bind_csv_files(
            &roots,
            CsvScope::SuppliedOrderedOverrides {
                files: &override_inputs,
                label: "ordered laboratory documents, no actual profile read",
            },
        )
        .unwrap();
        assert_eq!(
            json!(value.value_utf16()),
            states.last().unwrap()["value_utf16"],
            "{row}"
        );
        let ignored = states
            .iter()
            .skip(1)
            .filter(|s| s["load_flags"] == 3)
            .count();
        assert_eq!(value.assignments().len(), states.len() - ignored);
        assert_eq!(value.ignored_restricted_assignments().len(), ignored);
        assert!(!value.is_observed_native_startup());
        assert_eq!(
            value.require_empty_debug_stream().is_ok(),
            value.value_utf16().is_empty()
        );
    }
    let roots = vec![
        input(
            "UVars_AA.tgi",
            "[Vars]\n{\nstring CombatValueCSV = null\n}\n",
        ),
        input(
            "UVars.tgi",
            "[Vars]\n{\nstring CombatValueCSV = cv.csv\n}\n",
        ),
    ];
    assert!(bind_csv_files(&roots, CsvScope::Unknown).is_err());
    let value = bind_csv_files(
        &roots,
        CsvScope::FreshNoProfile {
            label: "fresh test application",
        },
    )
    .unwrap();
    assert_eq!(value.inputs()[0].path, "UVars.tgi");
    value.require_empty_debug_stream().unwrap();
    let mut invalid = input("UVars.tgi", "[Vars]\n{\nstring CombatValueCSV = null\n}\n");
    invalid.sha256 = "changed".into();
    assert!(
        bind_csv_files(&[invalid], CsvScope::FreshNoProfile { label: "test" })
            .unwrap_err()
            .contains("hash")
    );
    let invalid = input(
        "UVars.tgi",
        "[Vars]\n{\nstring CombatValueCSV = null\nstring CombatValueCSV = other.csv\n}\n",
    );
    assert!(bind_csv_files(&[invalid], CsvScope::FreshNoProfile { label: "test" }).is_err());
    let unquoted_empty = input("UVars.tgi", "[Vars]\n{\nstring CombatValueCSV =\n}\n");
    assert!(
        bind_csv_files(
            &[unquoted_empty],
            CsvScope::FreshNoProfile {
                label: "unsupported empty assignment"
            }
        )
        .is_err()
    );
    let collisions: Vec<_> = ["UVars.tgi", "UVars_AA.tgi", "UVars_B.tgi", "UVars_aa.tgi"]
        .iter()
        .map(|name| input(name, "[Vars]\n{\nstring CombatValueCSV = null\n}\n"))
        .collect();
    assert!(
        bind_csv_files(
            &collisions,
            CsvScope::FreshNoProfile {
                label: "nonadjacent case collision"
            }
        )
        .unwrap_err()
        .contains("collide")
    );
}

#[test]
fn fp_context_and_source_owners_are_explicit() {
    for (cw, mxcsr, sse2) in [
        (0x17f, 0x1f80, true),
        (0x67f, 0x1f80, true),
        (0x27f, 0x9f80, true),
        (0x27f, 0x1fc0, true),
        (0x27f, 0x1f80, false),
        (0x200, 0x1f80, true),
    ] {
        assert!(
            SourceFpContext::supplied_diagnostic(cw, 0, mxcsr, sse2, "unsupported diagnostic")
                .is_err()
        );
    }
    assert!(SourceFpContext::supplied_diagnostic(0x27f, 0, 0x1f80, true, "").is_err());
    let text = base(None);
    let actors = source(&text, "", "");
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(Settings::from_files([input("Game/SVars.tgi", &text)], Limits::default()).unwrap());
    let fp = context(Precision::Double53);
    bind_numeric(&catalog, &fp).unwrap();
    catalog
        .settings
        .as_mut()
        .unwrap()
        .declarations
        .get_mut("combatdamageminimum")
        .unwrap()[0]
        .source
        .sha256 = "changed".into();
    assert!(
        bind_numeric(&catalog, &fp)
            .err()
            .unwrap()
            .contains("source owner/hash")
    );
    catalog.settings =
        Some(Settings::from_files([input("Game/SVars.tgi", &text)], Limits::default()).unwrap());
    catalog
        .settings
        .as_mut()
        .unwrap()
        .declarations
        .get_mut("combatdamageminimum")
        .unwrap()[0]
        .raw = "999".into();
    assert!(
        bind_numeric(&catalog, &fp)
            .err()
            .unwrap()
            .contains("value/span")
    );
    let actors = source(
        &text,
        "",
        "[WorldRule]\n{\nstring IDS = CombatValueRangePower\n}\n",
    );
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(Settings::from_files([input("Game/SVars.tgi", &text)], Limits::default()).unwrap());
    assert!(
        bind_numeric(&catalog, &fp)
            .err()
            .unwrap()
            .contains("WorldRule")
    );
}

#[test]
#[ignore = "requires original/v14 source bytes via AA_GAME_ROOT"]
fn actual_cv_source_inputs_are_bound_in_all_explicit_pcs() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let (actors, report) = aa_rules::actors::native_load::load(&mut sources).unwrap();
    assert_eq!(report.exceptions().count(), 0);
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(Settings::load_v14(&mut sources, Limits::default()).unwrap());
    let mut rows = Vec::new();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let fp = context(pc);
        let values = bind_numeric(&catalog, &fp).unwrap();
        assert_eq!(
            values.formula_settings().unwrap().range_power.0,
            0.15f32.to_bits()
        );
        assert_eq!(
            values.formula_settings().unwrap().offense_weight.0,
            0.5f32.to_bits()
        );
        assert_eq!(values.ranged_minimum().unwrap().0, 12f32.to_bits());
        rows.push(json!({"precision":pc,"numeric":values.values(),"context_observed_native_startup":values.context().is_observed_native_startup()}));
    }
    let csv = bind_csv_from_sources(
        &mut sources,
        CsvScope::FreshNoProfile {
            label: "fresh diagnostic only; original game user/profile state not imported",
        },
    )
    .unwrap();
    csv.require_empty_debug_stream().unwrap();
    println!(
        "actual ten numeric source values bound at all3 explicitPCs;CSV assignments{} from{} rootinputs; no profile or native FP capture",
        csv.assignments().len(),
        csv.inputs().len()
    );
    if let Some(path) = std::env::var_os("AA_CV_SOURCE_CONTEXT_OUT") {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        file.write_all(serde_json::to_string_pretty(&json!({"scope":"actual pinned source inputs under explicit diagnostic PCs/FreshNoProfile only; not completed CV or original application context","numeric":rows,"csv":csv,"world_rule_registrations":actors.native_data_registry("WorldRule").unwrap().entries,"source_load_exceptions":report.exceptions().count()})).unwrap().as_bytes()).unwrap();
    }
}
