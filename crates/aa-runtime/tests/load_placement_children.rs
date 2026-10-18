//! Actual first saved Kingdom through the composed Object base initializer.
//! Other Session fields are outside this test; its reached F8 field is owned
//! by the real aa-netcode Session and uses the separately proved constructor
//! value. No constant host callback or alternate mutable actor copy is used.
use aa_mapworld::{
    MapArchive,
    restore::{RestoreMode, RestoredRecord},
};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use aa_runtime::{
    battle_terrain,
    catalog::Catalog,
    combat_source_completion,
    combat_source_receipt::VerifiedStartupReceipt,
    live_object_init::Record,
    load_spatial::LoadSpatial,
    load_world_globals::{PointShape, SessionPhase},
    load_zone_services,
    localization::Localization,
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
};
use serde_json::{Value, json};
use std::sync::Arc;

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source and tutorial map"]
fn actual_current_shell_collision_before_membership_and_required_manager_guard() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let map = MapArchive::from_bytes(
        std::fs::read(
            root.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let source_pc = Precision::Double53;
    let terrain_source =
        battle_terrain::Source::bind(&mut sources, &actors, &settings, source_pc).unwrap();
    let terrain = battle_terrain::BattleTerrain::load(&map, &terrain_source).unwrap();
    let fields = actors.native_source_requirements(source_pc).unwrap();
    let phase_proof: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-load-world-globals/native-instructions.json"
    ))
    .unwrap();
    let constructor = phase_proof["constructor_store_cases"].as_array().unwrap();
    assert!(!constructor.is_empty());
    assert!(constructor.iter().all(|row| row["after"] == 0));
    // This actual Session owns the reached mutable F8 field. Unread Session
    // fields are not a claim to whole native Session constructor equivalence.
    let session = aa_netcode::lobby::Session {
        field_f8: constructor[0]["after"].as_u64().unwrap() as u32,
        ..Default::default()
    };
    assert_eq!(SessionPhase::current_phase(&session).unwrap(), 0);
    let mut rows = Vec::new();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
            .unwrap()
            .begin(TaskKind::Normal)
            .unwrap();
        let mode = Some(inactive.native_load_mode());
        let mut attempt = Attempt::load_with_bindings(
            &map,
            &actors,
            settings.clone(),
            text.clone(),
            source_pc,
            u64::MAX,
            RestoreMode::InspectPartial,
            LoadBindings {
                native_load_mode: mode,
                inactive_load: Some(inactive),
                resources: Some(Arc::clone(&resources)),
            },
        )
        .unwrap();
        let mut catalog = Catalog::new(&actors);
        catalog.settings = Some(settings.clone());
        catalog.replace_current_text(text.clone());
        catalog.definition_text = Some(text.clone());
        catalog
            .bind_inactive_load(attempt.inactive_load.as_ref().unwrap())
            .unwrap();
        *catalog.definition_bank.borrow_mut() = attempt.definitions;
        let receipt =
            VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut sources).unwrap();
        let _point = PointShape::from_verified_startup_reference(&catalog, &receipt).unwrap();
        combat_source_completion::install(
            &catalog,
            &mut sources,
            receipt.context(),
            receipt.csv(),
            receipt.globals(),
        )
        .unwrap();
        let owner = attempt
            .registry
            .iter()
            .find(|r| matches!(r.value, RestoredRecord::Kingdom(_)))
            .unwrap()
            .id;
        assert_eq!(owner.raw(), 781);
        let mut held = std::mem::replace(
            attempt.registry.value_mut(owner).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Kingdom(kingdom) = &mut held else {
            unreachable!()
        };
        let runtime = kingdom.runtime.as_mut().unwrap();
        assert_eq!(runtime.base.thing.flags, 0xA0C46900);
        assert_eq!(runtime.base.object.as_ref().unwrap().memory.count(), 0);
        let mut spatial = LoadSpatial::new(&terrain.bound, pc).unwrap();
        let header = attempt.inventory.header.as_ref().unwrap();
        let zones =
            load_zone_services::before_first_record(&terrain.bound, spatial.index(), header)
                .unwrap();
        let mut record = Record {
            owner,
            shell: &mut runtime.base,
            definition: &runtime.definition,
            registry: &mut attempt.registry,
        };
        let grid = spatial.index().grid().clone();
        let layout = terrain.bound.layout;
        let extents = layout.extent_i32.map(|n| (n as f32).to_bits());
        let map_view = aa_spatial::gworld_placement::GMapView {
            width: layout.width,
            height: layout.height,
            coordinate_shift: layout.coordinate_shift,
            cell_size_bits: layout.cell_size_bits,
            extent_bits: extents,
        };
        let geometry =
            aa_spatial::query::QueryGeometry::from_native_extents(spatial.index(), extents, pc)
                .unwrap();
        let source = actors
            .native_object_definition("kingdom", source_pc)
            .unwrap();
        let w = source.blocking.words;
        let position = [
            record.shell.thing.position_bits[0],
            record.shell.thing.position_bits[1],
        ];
        let before_flags = record.shell.thing.flags;
        let definition = catalog
            .definition_bank
            .borrow()
            .token("ActorDefinition", "kingdom")
            .unwrap();
        let mut query = aa_spatial::creation_placement::Context {
            placement: aa_spatial::geometry::Placement {
                shape: aa_spatial::collision::Shape {
                    kind: w[0],
                    radius_bits: w[1],
                    bounds: [w[2], w[3], w[4], w[5]],
                },
                facing: [
                    record.shell.thing.facing_bits[0],
                    record.shell.thing.facing_bits[1],
                ],
                position,
            },
            definition: catalog
                .definition_bank
                .borrow()
                .token("ActorDefinition", "kingdom")
                .unwrap(),
            source_actor: None,
            ignore_actor: Some(owner),
            ignore_definition: None,
            collision_owner: 0,
            hostility_context: 0,
            custom_filter: 0,
            ignore_all: false,
            ignore_moving: false,
            required_actor_flags: 0x20000400,
            mask: 0x500,
        };
        let mut children = aa_runtime::load_placement_children::Children {
            catalog: &catalog,
            map: map_view,
            geometry: &geometry,
            terrain: &terrain,
            spatial: &mut spatial,
            zones: &zones,
            fields: &fields,
            phase: &session,
        };
        use aa_runtime::live_placement::Services as _;
        let result = children
            .collision(&mut record, &mut query, definition, pc)
            .unwrap();
        assert!(result <= 3);
        let key = children.spatial.key(owner).unwrap();
        assert!(children.spatial.index().member(key).is_err());
        assert_eq!(record.shell.thing.flags, before_flags);
        assert!(
            children
                .spatial
                .index()
                .actors_at([0, 0])
                .unwrap()
                .is_empty()
        );
        let missing = children
            .required_zone(&mut record, position, query.definition, 0, None, 0, 1, pc)
            .unwrap_err()
            .to_string();
        assert!(missing.contains("complete"), "{missing}");
        rows.push(
            json!({"precision":pc,"source_precision":source_pc,"owner":owner.raw(),
            "grid":grid,"extents":extents,"shape":w,"position":position,
            "facing":query.placement.facing,"definition_flags":record.definition.flags,
            "tile0_flags":children.spatial.index().tile([0,0]).unwrap().flags20,
            "required_actor_flags":query.required_actor_flags,"mask":query.mask,"result":result}),
        );
        let native:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261016-load-placement-children/native-actual-direct-child.json")).unwrap();
        let expected = native["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["input"]["precision"] == serde_json::to_value(pc).unwrap())
            .unwrap();
        assert_eq!(&expected["input"], rows.last().unwrap());
        assert_eq!(expected["output"]["result"], result);
        *attempt.registry.value_mut(owner).unwrap() = held;
    }
    if let Some(path) = std::env::var_os("AA_PLACEMENT_CHILD_CAPTURE") {
        std::fs::write(path,serde_json::to_vec_pretty(&json!({"scope":"actual source/shell query before membership; diagnostic direct child, not a naturally reached lifecycle call","cases":rows})).unwrap()).unwrap();
    }
}
