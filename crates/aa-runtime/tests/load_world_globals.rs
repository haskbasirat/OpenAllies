use aa_runtime::load_world_globals::{POINT_REFERENCE_SHA256, PointShape, SessionPhase};
use serde_json::Value;

#[test]
fn startup_reference_covers_every_word_with_calibrated_native_watches() {
    let raw = include_bytes!(
        "../../../research/skirmish-20261016-load-world-globals/point-startup-reference.json"
    );
    assert_eq!(aa_rules::sources::sha256(raw), POINT_REFERENCE_SHA256);
    let proof: Value = serde_json::from_slice(raw).unwrap();
    let captures = proof["captures"].as_array().unwrap();
    assert_eq!(captures.len(), 2);
    let mut covered = std::collections::BTreeSet::new();
    for c in captures {
        assert_eq!(c["native_writes_before_cutoff"], 0);
        assert_eq!(c["calibration_probes"], 4);
        assert_eq!(c["armed_threads"], 62);
        assert_eq!(c["cleanup_succeeded"], true);
        for w in c["word_indices"].as_array().unwrap() {
            covered.insert(w.as_u64().unwrap());
        }
    }
    assert_eq!(
        covered.into_iter().collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4, 5]
    );
}

#[test]
fn a_phase_only_host_cannot_invent_application_state() {
    struct PhaseOnly;
    impl SessionPhase for PhaseOnly {
        fn current_phase(&self) -> Result<u32, String> {
            Ok(6)
        }
    }
    assert_eq!(PhaseOnly.current_phase().unwrap(), 6);
    assert!(
        PhaseOnly
            .application_state()
            .unwrap_err()
            .contains("unbound")
    );
}

#[test]
#[ignore = "requires original Data.rwd and accepted v14 source files via AA_GAME_ROOT"]
fn actual_source_binds_native_startup_point_and_rechecks_destination() {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
    };
    use aa_runtime::{
        catalog::Catalog, combat_source_receipt::VerifiedStartupReceipt, settings::Settings,
    };
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(Settings::load_v14(&mut sources, aa_rules::tgi::Limits::default()).unwrap());
    let receipt =
        VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut sources).unwrap();
    let owner = PointShape::from_verified_startup_reference(&catalog, &receipt).unwrap();
    owner.validate_catalog(&catalog).unwrap();
    let shape = owner.shape();
    assert_eq!(
        [
            shape.kind,
            shape.radius_bits,
            shape.bounds[0],
            shape.bounds[1],
            shape.bounds[2],
            shape.bounds[3]
        ],
        [0; 6]
    );
    assert_eq!(owner.provenance().reference_sha256, POINT_REFERENCE_SHA256);
    for row in catalog
        .settings
        .as_mut()
        .unwrap()
        .declarations
        .get_mut("combatdamageminimum")
        .unwrap()
    {
        row.raw = "2".into();
    }
    assert!(
        PointShape::from_verified_startup_reference(&catalog, &receipt).is_err(),
        "an earlier receipt cannot bless changed destination numeric sources"
    );
    assert!(
        owner.validate_catalog(&catalog).is_err(),
        "an existing owner cannot bless changed destination sources at the actual read boundary"
    );
    let empty = ActorCatalog::from_files(vec![], aa_rules::tgi::Limits::default());
    assert!(PointShape::from_verified_startup_reference(&Catalog::new(&empty), &receipt).is_err());
    assert!(owner.validate_catalog(&Catalog::new(&empty)).is_err());
}
