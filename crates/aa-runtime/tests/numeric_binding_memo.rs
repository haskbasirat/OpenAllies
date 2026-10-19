//! `bind_numeric` replays a completed binding only while every input it reads
//! is unchanged: the result after any Settings edit or precision change equals
//! a binding computed by a fresh Catalog.
use aa_numeric::Precision;
use aa_rules::{
    actors::{
        ActorCatalog,
        native_load::{NativeLoadReport, load_files},
    },
    sources::{Origin, ResolvedFile, sha256},
    tgi::Limits,
};
use aa_runtime::{
    catalog::Catalog,
    combat_value_source_context::{NUMERIC_SETTINGS, SourceFpContext, bind_numeric},
    settings::Settings,
};

fn input(path: &str, text: &str) -> ResolvedFile {
    let bytes = text.as_bytes().to_vec();
    ResolvedFile { logical_path: path.into(), origin: Origin::V14Loose, sha256: sha256(&bytes), bytes }
}
fn context(pc: Precision) -> SourceFpContext {
    let control = match pc {
        Precision::Single24 => 0x7f,
        Precision::Double53 => 0x27f,
        Precision::Extended64 => 0x37f,
    };
    SourceFpContext::supplied_diagnostic(control, 0, 0x1f80, true, "memo equivalence fixture").unwrap()
}
fn base() -> String {
    let mut text = "[SVars]\n{\n".to_string();
    for (index, (name, _)) in NUMERIC_SETTINGS.iter().enumerate() {
        text.push_str(&format!("fixed {name} = {}.{}\n", index + 1, index));
    }
    text.push_str("}\n");
    text
}
fn settings(base: &str, overlay: &str) -> Settings {
    Settings::from_files([input("Game/SVars.tgi", base), input("Game/SVars_AA.tgi", overlay)], Limits::default())
        .unwrap()
}
fn outcome(catalog: &Catalog<'_>, pc: Precision) -> String {
    match bind_numeric(catalog, &context(pc)) {
        Ok(bound) => serde_json::to_string(bound.values()).unwrap(),
        Err(error) => format!("error: {error}"),
    }
}

#[test]
fn replayed_numeric_bindings_equal_fresh_bindings_after_every_input_change() {
    let base = base();
    let overlay = "[SVars]\n{\nfixed CombatValueAreaPower = 2.5\n}\n";
    let actors = load_files(
        vec![input("Game/SVars.tgi", &base), input("Game/SVars_AA.tgi", overlay)],
        Limits::default(),
        &mut NativeLoadReport::default(),
    );
    let other = load_files(
        vec![input("Game/SVars.tgi", &base), input("Game/SVars_AA.tgi", "[SVars]\n{\n}\n")],
        Limits::default(),
        &mut NativeLoadReport::default(),
    );
    let fresh = |settings: Option<Settings>, pc| {
        let mut catalog = Catalog::new(&actors);
        catalog.settings = settings;
        outcome(&catalog, pc)
    };
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(settings(&base, overlay));
    let pcs = [Precision::Double53, Precision::Single24, Precision::Extended64, Precision::Double53];
    for pc in pcs {
        let first = outcome(&catalog, pc);
        assert!(!first.starts_with("error"), "{first}");
        assert_eq!(first, outcome(&catalog, pc));
        assert_eq!(first, fresh(Some(settings(&base, overlay)), pc));
    }
    // Edit one bound row in place: the replay must not hide the change.
    let edited = catalog.settings.as_mut().unwrap();
    let key = "combatvaluerangepower";
    edited.declarations.get_mut(key).unwrap()[0].raw = "7".into();
    let mut expected = settings(&base, overlay);
    expected.declarations.get_mut(key).unwrap()[0].raw = "7".into();
    for pc in pcs {
        assert_eq!(outcome(&catalog, pc), fresh(Some(expected.clone()), pc));
    }
    // Remove a row, then remove Settings entirely.
    catalog.settings.as_mut().unwrap().declarations.remove("combatdamageminimum");
    expected.declarations.remove("combatdamageminimum");
    assert_eq!(outcome(&catalog, Precision::Double53), fresh(Some(expected.clone()), Precision::Double53));
    catalog.settings = None;
    assert_eq!(outcome(&catalog, Precision::Double53), fresh(None, Precision::Double53));
    // Restore the original rows: the result is again the fresh binding.
    catalog.settings = Some(settings(&base, overlay));
    for pc in pcs {
        assert_eq!(outcome(&catalog, pc), fresh(Some(settings(&base, overlay)), pc));
    }
    // A different source owner never reuses this Catalog's binding.
    catalog.actors = &other;
    assert!(outcome(&catalog, Precision::Double53).starts_with("error"));
}
