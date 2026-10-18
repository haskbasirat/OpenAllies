use super::*;
use crate::{
    load_spatial::LoadSpatial,
    load_world_globals::{PointShape, SessionPhase},
    load_zone_services,
    world_object_list::WorldObjectList,
};
use aa_actors::ActorId;
use aa_mapworld::restore::RestoredRecord;
use std::sync::Arc;

#[test]
#[ignore = "requires AA_GAME_ROOT and verified capture03"]
fn actual_held_overlap_owner_runs_real_empty_query_and_retains_expiration_before_error() {
    use crate::{
        battle_terrain, combat_source_completion as completion,
        combat_source_receipt::{TargetFiles, VerifiedStartupReceipt},
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        restore_lifecycle::{ScopedManagedLoad, TaskKind},
        settings::Settings,
    };
    use aa_mapworld::restore::RestoreMode;
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let native = game.join("reconstruction/native-game/runtime-expanded-v14");
    let capture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/development10/native-source-capture-03");
    let exe = std::fs::read(native.join("AA.exe")).unwrap();
    let dll = std::fs::read(native.join("D3D9.dll")).unwrap();
    let config = std::fs::read(native.join("dgVoodoo.conf")).unwrap();
    let receipt = std::fs::read(capture.join("capture.json")).unwrap();
    let manifest = std::fs::read(capture.join("runtime-files.json")).unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        &native,
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let mut point = None;
    let mut attempt = Attempt::load_with_source_setup(
        &map,
        &actors,
        settings.clone(),
        text,
        Precision::Double53,
        u64::MAX,
        RestoreMode::InspectPartial,
        LoadBindings {
            native_load_mode: Some(1),
            inactive_load: Some(inactive),
            resources: Some(resources),
        },
        |catalog| {
            let proof = VerifiedStartupReceipt::verify(
                catalog,
                &mut sources,
                TargetFiles {
                    native_exe: &exe,
                    renderer_dll: &dll,
                    renderer_config: &config,
                    capture_receipt: &receipt,
                    file_manifest_receipt: &manifest,
                },
            )?;
            point = Some(PointShape::from_verified_startup_reference(
                catalog, &proof,
            )?);
            completion::install(
                catalog,
                &mut sources,
                proof.context(),
                proof.csv(),
                proof.globals(),
            )?;
            completion::compute(catalog)
        },
    )
    .unwrap();

    let source =
        battle_terrain::Source::bind(&mut sources, &actors, &settings, Precision::Double53)
            .unwrap();
    let terrain = battle_terrain::BattleTerrain::load(&map, &source).unwrap();
    let mut spatial = LoadSpatial::new(&terrain.bound, Precision::Extended64).unwrap();
    let header = attempt.inventory.header.as_ref().unwrap();
    let mut zones =
        load_zone_services::before_first_record(&terrain.bound, spatial.index(), header).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(settings);
    catalog.native_load_mode = Some(1);
    *catalog.definition_bank.borrow_mut() = attempt.definitions;
    let fields = actors
        .native_source_requirements(Precision::Double53)
        .unwrap();
    let owner = ActorId::from_raw(781);
    let mut held = std::mem::replace(
        attempt.registry.value_mut(owner).unwrap(),
        RestoredRecord::Loading,
    );
    let RestoredRecord::Kingdom(kingdom) = &mut held else {
        panic!("real saved Kingdom")
    };
    let runtime = kingdom.runtime.as_mut().unwrap();
    let mut record = Record {
        owner,
        shell: &mut runtime.base,
        definition: &runtime.definition,
        registry: &mut attempt.registry,
    };
    let actual_flags = record.shell.thing.flags;
    assert_ne!(actual_flags & (1 << 29), 0);
    let mut objects = WorldObjectList::constructed();
    let mut map_mode = Some(1);
    struct UnreachedPhase;
    impl SessionPhase for UnreachedPhase {
        fn current_phase(&self) -> std::result::Result<u32, String> {
            panic!("overlap query must not read Session phase")
        }
    }
    let phase = UnreachedPhase;
    let point = point.unwrap();
    let mut zone_services = load_zone_services::Context::new(&point, &phase);
    let mut context = Context {
        terrain: &terrain,
        header,
        spatial: &mut spatial,
        zones: &mut zones,
        objects: &mut objects,
        source_fields: &fields,
        map_mode: &mut map_mode,
        global_settings_mode: crate::live_placement::SettingsMode::Unbound,
        placement_clearance_bits: None,
        zone_services: &mut zone_services,
    };
    assert!(
        !refresh(
            &catalog,
            &mut record,
            &mut context,
            Precision::Extended64,
            || panic!("actual bit29 returns before clock read")
        )
        .unwrap()
    );
    // Explicit synthetic active-flag input on the real saved allocation. This
    // proves adapter ownership and phase ordering, not a military-unit test.
    record.shell.thing.flags = (actual_flags & !(1 << 29)) | (1 << 10);
    context
        .spatial
        .refresh_membership(&catalog, &mut record)
        .unwrap();
    let list = &mut record.shell.actor.as_mut().unwrap().overlap_list_110;
    let future = list
        .prepend(overlap_list::Entry {
            reference: 0x10001,
            expiry_bits: 0x3f800000,
        })
        .unwrap();
    let expired = list
        .prepend(overlap_list::Entry {
            reference: 0x20002,
            expiry_bits: 0,
        })
        .unwrap();
    let shell_address = record.shell as *const _ as usize;
    let mut clock_reads = 0;
    assert!(
        refresh(
            &catalog,
            &mut record,
            &mut context,
            Precision::Extended64,
            || {
                clock_reads += 1;
                Ok(0)
            }
        )
        .unwrap()
    );
    assert_eq!(clock_reads, 1);
    let list = &record.shell.actor.as_ref().unwrap().overlap_list_110;
    assert!(list.node(expired).is_err());
    assert_eq!(list.head(), Some(future));
    assert_eq!(list.tail(), Some(future));
    assert_eq!(list.allocated_nodes(), 1);
    let key = context.spatial.key(owner).unwrap();
    assert!(context.spatial.index().member(key).is_err());
    assert_eq!(record.shell as *const _ as usize, shell_address);
    let expired = record
        .shell
        .actor
        .as_mut()
        .unwrap()
        .overlap_list_110
        .prepend(overlap_list::Entry {
            reference: 0x30003,
            expiry_bits: 0,
        })
        .unwrap();
    // Deliberately wrong live PC fails map-owner validation AFTER expiration.
    assert!(
        refresh(
            &catalog,
            &mut record,
            &mut context,
            Precision::Single24,
            || Ok(0)
        )
        .unwrap_err()
        .to_string()
        .contains("owners disagree")
    );
    assert!(
        record
            .shell
            .actor
            .as_ref()
            .unwrap()
            .overlap_list_110
            .node(expired)
            .is_err()
    );
    assert_eq!(
        record.shell.actor.as_ref().unwrap().overlap_list_110.head(),
        Some(future)
    );
    println!(
        "actual saved Kingdom owner retained; explicit synthetic active flags: real self-only query completed, expiry prefix retained on later map-PC error"
    );
    *attempt.registry.value_mut(owner).unwrap() = held;
}
