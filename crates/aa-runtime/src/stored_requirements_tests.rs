use super::*;

#[test]
#[ignore = "requires original AA_GAME_ROOT sources; no World/Actor allocation is synthesized"]
fn actual_element_requirements_use_same_moved_bank_and_registered_properties() {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let definitions = ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&definitions);
    catalog.settings = Some(
        crate::settings::Settings::load_v14(&mut files, aa_rules::tgi::Limits::default()).unwrap(),
    );
    let receipt = crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(
        &catalog, &mut files,
    )
    .unwrap();
    crate::combat_source_completion::install(
        &catalog,
        &mut files,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    let mut bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    assert!(catalog.definition_bank.borrow().combat_source().is_err());
    let pc = bank.combat_source().unwrap().source_precision();
    let mut store = crate::actor_world::ActorStore::new();
    let property_owner = &store.property_allocations as *const _ as usize;
    let (_, mut access) = store.split_minds();
    for (id, wanted_structures, wanted_properties) in [
        ("ger_truck_infantry", vec![], vec![]),
        ("ger_truck_engineer", vec![], vec![]),
        ("ger_truck_airborne", vec![], vec!["air_support"]),
        (
            "ger_truck_mechanized",
            vec!["ger_hq_infantry"],
            vec!["mechhqproduction"],
        ),
        (
            "ger_truck_armor",
            vec!["ger_hq_mechanized"],
            vec!["armorhqproduction"],
        ),
    ] {
        let fields = requirements(&catalog, &mut access, &mut bank, id, pc).unwrap();
        let names = |tokens: &[u32]| {
            tokens
                .iter()
                .map(|&v| bank.entry(v).unwrap().key.name.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&fields.actors), wanted_structures, "{id}");
        assert_eq!(names(&fields.properties), wanted_properties, "{id}");
        assert!(!fields.any_property && !fields.any_actor);
        assert!(hide_if_missing_properties(&catalog, &bank, id, pc).unwrap());
    }
    assert_eq!(
        &*access.property_allocations as *const _ as usize,
        property_owner
    );
    let foreign = if pc == Precision::Single24 {
        Precision::Double53
    } else {
        Precision::Single24
    };
    assert!(hide_if_missing_properties(&catalog, &bank, "ger_truck_infantry", foreign).is_err());
    //Every positive required Property source is a registered/shared type. Its
    //virtual18 remains true even when its independently loaded+5C is false.
    let fields =
        requirements(&catalog, &mut access, &mut bank, "ger_truck_mechanized", pc).unwrap();
    let definition = fields.properties[0];
    let mut world = Bound {
        catalog: &catalog,
        actors: &mut access,
        bank: &mut bank,
        pc,
    };
    assert!(RequirementWorld::is_technology(&mut world, definition).unwrap());
    assert!(world.actors.property_allocations.instances().is_empty());
}
