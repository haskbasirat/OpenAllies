use super::*;
use aa_rules::{actors::ActorCatalog, sources::SourceLimits, tgi::Limits};

#[test]
#[ignore = "actual original/v14 source factory; no native engine, view activation or GPU execution"]
fn actual_source_factory_retains_one_bundle_and_application() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let mut files = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(crate::settings::Settings::load_v14(&mut files, Limits::default()).unwrap());
    let receipt = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut files).unwrap();
    let early = Inputs::from_fresh_sources(
        &catalog,
        &receipt,
        &mut files,
        NativeOverrides::ControlledNone,
    )
    .err()
    .unwrap();
    assert!(early.seeds().is_none() && early.grid().is_none() && early.orientation().is_none());
    crate::combat_source_completion::install(
        &catalog,
        &mut files,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();

    //Reject the second reached public/private binding after preserving the
    //first source write TerrainBlockSize=8 in the same partial owner.
    let declaration = &mut catalog
        .settings
        .as_mut()
        .unwrap()
        .app_variables
        .get_mut("vgridcellsize")
        .unwrap()[0];
    let original = std::mem::replace(&mut declaration.raw, "9".into());
    let partial = Inputs::from_fresh_sources(
        &catalog,
        &receipt,
        &mut files,
        NativeOverrides::ControlledNone,
    )
    .err()
    .unwrap();
    assert!(partial.seeds().is_some());
    let grid = partial.grid().expect("same partial source owner");
    assert_eq!(grid.terrain_block_size(), 8);
    assert!(grid.validate(&catalog).unwrap_err().contains("incomplete"));
    assert!(partial.orientation().is_none());
    catalog
        .settings
        .as_mut()
        .unwrap()
        .app_variables
        .get_mut("vgridcellsize")
        .unwrap()[0]
        .raw = original;

    let inputs = Inputs::from_fresh_sources(
        &catalog,
        &receipt,
        &mut files,
        NativeOverrides::ControlledNone,
    )
    .unwrap();
    let bank = catalog.definition_bank.borrow();
    inputs.validate(&catalog, &bank).unwrap();
    assert_eq!(
        (
            inputs.grid().terrain_block_size(),
            inputs.grid().vgrid_cell_size()
        ),
        (8, 4)
    );
    assert_eq!(
        inputs.orientation().current(&catalog, &bank).unwrap(),
        [0, 0xbf800000, 0]
    );
    let mut globals = inputs.seeds().bind(&catalog).unwrap();
    let grid = crate::view_grid_state::State::<usize>::from_constructor(&mut globals).unwrap();
    assert_eq!(grid.word(0x14).unwrap(), Some(0));
    assert_eq!(grid.word(0x28).unwrap(), None);
    drop(bank);

    let startup =
        crate::actor_creation_startup::StartupInputs::from_verified_startup_reference(&receipt)
            .unwrap();
    let directory = game
        .join("rust-rebuild/qa-output/view-startup-inputs")
        .join(format!("source-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let mut application = crate::world_ready_services::Application::from_fresh_sources(
        &catalog,
        files,
        crate::application_mode::GameAppState::from_native_constructor(),
        startup,
        &directory,
        NativeOverrides::ControlledNone,
    )
    .unwrap();
    assert!(application.view_inputs().is_err());
    application.install_view_inputs(&catalog, inputs).unwrap();
    let duplicate = Inputs::from_fresh_sources(
        &catalog,
        &receipt,
        application.depot_mut().sources_mut(),
        NativeOverrides::ControlledNone,
    )
    .unwrap();
    let rejected = application
        .install_view_inputs(&catalog, duplicate)
        .err()
        .unwrap();
    assert!(rejected.error.contains("already"));
    assert_eq!(rejected.inputs.grid().terrain_block_size(), 8);
    let bank = catalog.definition_bank.borrow();
    {
        let parts = application.view_startup_parts(&catalog, &bank).unwrap();
        parts.inputs.grid_mut().assign_terrain_block_size(32);
        let _same_loading = parts.loading;
    }
    assert_eq!(
        application
            .view_inputs()
            .unwrap()
            .grid()
            .terrain_block_size(),
        32
    );
    application
        .view_inputs()
        .unwrap()
        .validate(&catalog, &bank)
        .unwrap();
    //The existing Parts API remains available without a second owner.
    application.parts(&catalog).unwrap();
}
