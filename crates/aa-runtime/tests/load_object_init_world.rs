//! Actual first saved Kingdom through the composed Object base initializer.
//! Other Session fields are outside this test; its reached F8 field is owned
//! by the real aa-netcode Session and uses the separately proved constructor
//! value. No constant host callback or alternate mutable actor copy is used.
use aa_mapworld::{
    MapArchive,
    restore::{RestoreMode, RestoredRecord},
};
use aa_numeric::{Binary, Precision};
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
    live_placement::SettingsMode,
    load_object_init_world::{self, Context},
    load_spatial::LoadSpatial,
    load_world_globals::{PointShape, SessionPhase},
    load_zone_services,
    localization::Localization,
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
    world_object_list::WorldObjectList,
};
use serde_json::{Value, json};
use std::sync::Arc;

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source and tutorial map"]
fn actual_first_kingdom_object_init_with_real_session_owner() {
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
        let mut mode = Some(inactive.native_load_mode());
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
        let point = PointShape::from_verified_startup_reference(&catalog, &receipt).unwrap();
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
        let mut zones =
            load_zone_services::before_first_record(&terrain.bound, spatial.index(), header)
                .unwrap();
        let mut objects = WorldObjectList::constructed();
        let mut zone_services = load_zone_services::Context::new(&point, &session);
        let mut record = Record {
            owner,
            shell: &mut runtime.base,
            definition: &runtime.definition,
            registry: &mut attempt.registry,
        };
        let original_xy = [
            record.shell.thing.position_bits[0],
            record.shell.thing.position_bits[1],
        ];
        let expected_z = terrain
            .surface()
            .unwrap()
            .sample(original_xy, 0x122, pc)
            .unwrap()
            .store_f32()
            .unwrap();
        let source = actors
            .native_object_definition("kingdom", source_pc)
            .unwrap();
        let expected_38 = Binary::from_f32_bits(record.shell.object.as_ref().unwrap().scale_bits)
            .unwrap()
            .mul(&Binary::from_f32_bits(source.height.bits).unwrap(), pc)
            .unwrap()
            .store_f32()
            .unwrap();
        let mut context = Context {
            terrain: &terrain,
            header,
            spatial: &mut spatial,
            zones: &mut zones,
            objects: &mut objects,
            source_fields: &fields,
            map_mode: &mut mode,
            global_settings_mode: SettingsMode::Unbound,
            placement_clearance_bits: None,
            zone_services: &mut zone_services,
        };
        assert!(
            load_object_init_world::initialize(&catalog, &mut record, &mut context, pc).unwrap()
        );
        assert_eq!(record.shell.thing.flags, 0xA4C46900);
        assert_eq!(&record.shell.thing.position_bits[..2], &original_xy);
        assert_eq!(record.shell.thing.position_bits[2], expected_z);
        let object = record.shell.object.as_ref().unwrap();
        assert_eq!(object.associated_handle, expected_38);
        assert_eq!(object.byte_4c, 0);
        assert_eq!([object.word_44, object.word_48], [0, 0]);
        assert_eq!(object.memory.count(), 0);
        assert_eq!(context.objects.nodes().len(), 1);
        assert_eq!(
            context
                .objects
                .node(context.objects.head().unwrap())
                .unwrap()
                .object,
            owner
        );
        let key = context.spatial.key(owner).unwrap();
        assert_eq!(
            context.spatial.index().resolve_reference(owner.raw()),
            Some(key)
        );
        assert!(
            context.spatial.index().member(key).is_err(),
            "mutable shell fields never persist in spatial metadata"
        );
        assert!(
            context
                .zones
                .tiles
                .iter()
                .all(|t| t.set.is_none() && t.fog == 0 && t.supply == 0)
        );
        rows.push(json!({"precision":pc,"source_precision":terrain.source_precision(),"owner":owner.raw(),
            "result":true,"flags":record.shell.thing.flags,"position":record.shell.thing.position_bits,
            "word38":object.associated_handle,"cached_tile":[object.word_44,object.word_48],
            "world_list_nodes":context.objects.nodes().len(),"spatial_binding_count":context.spatial.bindings().len(),
            "point_reference":point.provenance(),"phase":session.field_f8}));
        *attempt.registry.value_mut(owner).unwrap() = held;
    }
    if let Some(path) = std::env::var_os("AA_OBJECT_WORLD_CAPTURE") {
        std::fs::write(path, serde_json::to_vec_pretty(&json!({
            "scope":"actual first saved Kingdom through composed Object base only; actual Session F8 owner uses proven constructor value, unread Session fields and prior/following whole-record lifecycle not certified",
            "cases":rows,
        })).unwrap()).unwrap();
    }
}
