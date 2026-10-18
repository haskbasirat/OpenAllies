use aa_numeric::Precision;
use aa_rules::{
    actors::{ActorCatalog, native_load::{NativeLoadReport, load_files}},
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use aa_runtime::catalog::Catalog;

fn file(text: &str) -> ResolvedFile {
    let bytes = text.as_bytes().to_vec();
    ResolvedFile { logical_path: "fixture.tgi".into(), origin: Origin::V14Loose,
        sha256: sha256(&bytes), bytes }
}

#[test]
fn canonical_mask_does_not_force_unrelated_ground_or_character_fields() {
    let actors = ActorCatalog::from_files(vec![file(concat!(
        "[Thing]\n{ string IDS = minimal\n string category = ACTOR\n",
        " [BodyComponent]\n{ fixed health = unsupported_expression\n}\n",
        " [CharacterComponent]\n{ string character = not_registered\n}\n}\n"
    ))], Limits::default());
    let catalog = Catalog::new(&actors);
    for pc in [Precision::Single24, Precision::Double53, Precision::Extended64] {
        let expected = actors.native_component_mask("minimal", None).unwrap().bits;
        assert_eq!(catalog.canonical_component_mask("minimal", pc).unwrap(), expected);
        assert_eq!(expected & 1, 1);
        assert_eq!(expected & (1 << 5), 0);
        // A full Actor binding must inspect fields which this mask read does
        // not require. This is a lazy source-adapter test, not native UB proof.
        assert!(catalog.actor_bindings("minimal", pc).is_err());
    }
}

#[test]
fn canonical_mask_refuses_shadowed_ids_and_changed_source_owners() {
    let actors = load_files(vec![file(concat!(
        "[Thing]\n{ string IDS = first\n string category = ACTOR\n}\n",
        "[Thing]\n{ string IDS = second\n string category = ACTOR\n}\n",
        "[Translate]\n{ string node = Thing\n string ids_old = first\n string ids_new = second\n}\n"
    ))], Limits::default(), &mut NativeLoadReport::default());
    let other = actors.clone();
    let mut catalog = Catalog::new(&actors);
    assert!(catalog.canonical_component_mask("first", Precision::Double53).unwrap_err()
        .contains("shadowed"));
    assert_eq!(catalog.canonical_component_mask("second", Precision::Double53).unwrap(), 0);
    catalog.actors = &other;
    assert!(catalog.canonical_component_mask("second", Precision::Double53).unwrap_err()
        .contains("source owner changed"));
    assert!(catalog.actor_bindings("second", Precision::Double53).is_err());
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 sources"]
fn actual_masks_match_completed_actor_bindings_at_all_precisions() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits { max_member_bytes: 64 * 1024 * 1024 }).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let catalog = Catalog::new(&actors);
    for pc in [Precision::Single24, Precision::Double53, Precision::Extended64] {
        for id in ["usa_infantry", "gbr_infantry", "ger_infantry", "jpn_infantry", "rus_infantry",
                   "regiment_usa_infantry", "usa_hq_infantry", "usa_corps"] {
            let minimal = catalog.canonical_component_mask(id, pc).unwrap();
            let full = catalog.actor_bindings(id, pc).unwrap();
            assert_eq!(Some(minimal), full.definition.component_mask, "{id} {pc:?}");
            assert_eq!(catalog.canonical_component_mask(id, pc).unwrap(), minimal);
        }
    }
}
