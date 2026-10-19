//! A registered Thing whose IDS a later Translate root redirects keeps its own
//! allocation for data loading (4BDC50 passes 4BCA00's return to vf0
//! LoadData), while 427BD0 Find returns the translation target.
use aa_numeric::Precision;
use aa_rules::{
    actors::{ThingLookup, ThingRef, native_load::{NativeLoadReport, load_files}},
    sources::{Origin, ResolvedFile, sha256},
    tgi::Limits,
};
use serde_json::Value;
use std::path::PathBuf;

fn file(path: &str, text: &str) -> ResolvedFile {
    ResolvedFile {
        logical_path: path.into(),
        origin: Origin::V14Loose,
        sha256: sha256(text.as_bytes()),
        bytes: text.as_bytes().to_vec(),
    }
}
fn thing(ids: &str) -> String {
    format!("[Thing]\r\n{{\r\n\tIDS = {ids}\r\n\tcategory = THING\r\n}}\r\n")
}
const TRANSLATE_TEMPLATE: &str =
    "[template Translate]\r\n{\r\n\tstring node\r\n\tstring ids_old\r\n\tstring ids_new\r\n}\r\n";

/// The recorded original/v14 pointer-selection cases: vf0 LoadData receives
/// the registered allocation and final Find returns the translation target.
#[test]
fn native_load_data_receiver_is_the_registered_allocation() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../research/development10/mapload-aspect-source/registered-source/native-registered-source.json",
    );
    let fixture: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(fixture["source_witness"]["translation"][0], "chaparral_wall");
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 18);
    for case in cases {
        let registered = &case["registration_pointer"];
        assert_eq!(&case["LoadData_ecx"], registered, "{case}");
        assert_eq!(&case["direct_registry_chaparral_wall"], registered, "{case}");
        assert_eq!(
            case["final_Find_chaparral_wall"], case["translated_target_pointer"],
            "{case}"
        );
        assert_ne!(case["final_Find_chaparral_wall"], *registered, "{case}");
    }
}

#[test]
fn registered_lookup_keeps_a_translated_away_allocation() {
    let mut report = NativeLoadReport::default();
    let catalog = load_files(
        vec![
            file("Templates/template_translate.tgi", TRANSLATE_TEMPLATE),
            file("Doodads/Chaparral/chaparral_wall.tgi", &thing("chaparral_wall")),
            file("Doodads/Temperate/temperate_walls.tgi", &thing("temperate_walls")),
            file(
                "translate.tgi",
                "[Translate template=Translate]\r\n{\r\n\tnode = Thing\r\n\tids_old = chaparral_wall\r\n\tids_new = temperate_walls\r\n}\r\n",
            ),
        ],
        Limits::default(),
        &mut report,
    );
    assert_eq!(
        report.thing_translations,
        [("chaparral_wall".to_owned(), "temperate_walls".to_owned())]
    );
    // Registration order is unchanged by the translation.
    let ids: Vec<_> = catalog.actors.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(ids, ["chaparral_wall", "temperate_walls"]);

    let found = catalog.find("chaparral_wall").unwrap();
    assert_eq!(found.id, "temperate_walls");
    assert_eq!(found.source.path, "Doodads/Temperate/temperate_walls.tgi");
    let own = catalog.registered("CHAPARRAL_WALL").unwrap();
    assert_eq!(own.id, "chaparral_wall");
    assert_eq!(own.source.path, "Doodads/Chaparral/chaparral_wall.tgi");

    // Plain &str keeps Find; ThingRef::registered selects the registry slot.
    let by_find = ThingRef::from("chaparral_wall");
    assert_eq!(by_find.lookup, ThingLookup::Find);
    let thing = ThingRef::registered("chaparral_wall");
    assert_eq!(catalog.select(by_find).unwrap().id, "temperate_walls");
    assert_eq!(catalog.select(thing).unwrap().id, "chaparral_wall");
    let own_root = catalog.resolved_actor(thing).unwrap();
    assert_eq!(own_root.sources()[0].path, "Doodads/Chaparral/chaparral_wall.tgi");
    let found_root = catalog.resolved_actor("chaparral_wall").unwrap();
    assert_eq!(found_root.sources()[0].path, "Doodads/Temperate/temperate_walls.tgi");
    // Typed field reads go through the same selection (untyped fixture roots
    // here, so compare the selected source layers instead of typed values).
    let own_layers = catalog.native_node_sources(thing, &[]).unwrap();
    assert_eq!(own_layers[0].path, "Doodads/Chaparral/chaparral_wall.tgi");
    let found_layers = catalog.native_node_sources("chaparral_wall", &[]).unwrap();
    assert_eq!(found_layers[0].path, "Doodads/Temperate/temperate_walls.tgi");
    assert!(catalog.native_field(thing, "category", Precision::Double53).is_err());

    // Unshadowed IDS: both lookups select the same allocation.
    for lookup in [ThingRef::find("temperate_walls"), ThingRef::registered("temperate_walls")] {
        assert_eq!(catalog.select(lookup).unwrap().source, found.source);
    }
    assert!(catalog.registered("missing").is_none());
}
