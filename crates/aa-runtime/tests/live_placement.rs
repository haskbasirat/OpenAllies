//! Actual restored first Kingdom source/Load fields, isolated placement only.
//! Complete prior Team/Kingdom initialization and map startup are not claimed.
use aa_actors::ActorId;
use aa_mapworld::{
    restore::RestoredRecord,
    terrain::{TerrainChunk, TerrainGridLayout},
};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use aa_runtime::{
    catalog::Catalog,
    live_object_init::Record,
    live_placement::{self, MapFacts, Services},
    localization::Localization,
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
};
use aa_spatial::{Error, Result, creation_placement::Context, loaded_object_placement::Descriptor};
use aa_zones::query::ZoneActors;
use serde_json::{Value, json};

struct Unreached;
fn missing<T>() -> Result<T> {
    Err(Error::World(
        "actual first Kingdom unexpectedly reached an unbound service".into(),
    ))
}
impl ZoneActors for Unreached {
    fn owner(&mut self, _: u32) -> aa_zones::Result<u32> {
        panic!("zone owner")
    }
    fn allied(&mut self, _: u32, _: u32) -> aa_zones::Result<bool> {
        panic!("zone allied")
    }
    fn enemy(&mut self, _: u32, _: u32) -> aa_zones::Result<bool> {
        panic!("zone enemy")
    }
    fn zone_slot(&mut self, _: u32, _: u32) -> aa_zones::Result<u32> {
        panic!("zone slot")
    }
    fn word4(&mut self, _: u32) -> aa_zones::Result<u32> {
        panic!("zone word4")
    }
    fn component_b0(&mut self, _: u32) -> aa_zones::Result<u32> {
        panic!("zone component")
    }
    fn reveal_mask(&mut self, _: u32) -> aa_zones::Result<Option<u32>> {
        panic!("zone reveal")
    }
}
impl Services for Unreached {
    fn required_zone(
        &mut self,
        _: &mut Record<'_>,
        _: [u32; 2],
        _: u32,
        _: u32,
        _: Option<ActorId>,
        _: u32,
        _: u32,
        _: Precision,
    ) -> Result<u32> {
        missing()
    }
    fn zone_actor(&mut self, _: u32) -> Result<ActorId> {
        missing()
    }
    fn structure_at(&mut self, _: &mut Record<'_>, _: [u32; 2]) -> Result<Option<ActorId>> {
        missing()
    }
    fn collision(
        &mut self,
        _: &mut Record<'_>,
        _: &mut Context<u32, ActorId>,
        _: u32,
        _: Precision,
    ) -> Result<u32> {
        missing()
    }
    fn footprint(
        &mut self,
        _: &mut Record<'_>,
        _: &Descriptor<u32, ()>,
        _: Precision,
    ) -> Result<u32> {
        missing()
    }
}

#[test]
fn native_source_shape_expansion_all_precisions() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-placement/native-creation-shape.json"
    ))
    .unwrap();
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 72);
    for row in fixture["cases"].as_array().unwrap() {
        let c = &row["input"];
        let shape = &c["shape"];
        let word = |i: usize| shape[i].as_u64().unwrap() as u32;
        let p = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let s = aa_spatial::collision::Shape {
            kind: word(0),
            radius_bits: word(1),
            bounds: [word(2), word(3), word(4), word(5)],
        };
        let o =
            live_placement::expanded_blocking_shape(&s, c["clearance"].as_u64().unwrap() as u32, p)
                .unwrap();
        assert_eq!(
            json!([
                o.kind,
                o.radius_bits,
                o.bounds[0],
                o.bounds[1],
                o.bounds[2],
                o.bounds[3]
            ]),
            row["output"],
            "{row}"
        );
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn actual_first_kingdom_uses_current_shell_and_completed_source_fields() {
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
    let resources =
        std::sync::Arc::new(aa_rules::resources::ResourceCatalog::load_v14(&mut sources).unwrap());
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(settings.clone());
    catalog.replace_current_text(text.clone());
    catalog.definition_text = Some(text.clone());
    let verified =
        aa_runtime::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(
            &catalog,
            &mut sources,
        )
        .unwrap();
    let source_pc = verified.context().precision();
    assert_eq!(source_pc, Precision::Double53);
    aa_runtime::combat_source_completion::install(
        &catalog,
        &mut sources,
        verified.context(),
        verified.csv(),
        verified.globals(),
    )
    .unwrap();
    let fields = actors.native_source_requirements(source_pc).unwrap();
    let wrong_pc_fields = actors
        .native_source_requirements(Precision::Single24)
        .unwrap();
    let mut rows = vec![];
    for map_name in [
        "Campaign/tutorial02.RMP",
        "DeveloperMaps/(3v3)Firefight.RMP",
        "(2v2)Berlin.RMP",
    ] {
        let map = aa_mapworld::MapArchive::from_bytes(
            std::fs::read(
                root.join("reconstruction/assets/original/Data/Maps")
                    .join(map_name),
            )
            .unwrap(),
        )
        .unwrap();
        let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
            .unwrap()
            .begin(TaskKind::Normal)
            .unwrap();
        let mode = inactive.native_load_mode();
        let mut attempt = aa_runtime::restoration::Attempt::load_with_bindings(
            &map,
            &actors,
            settings.clone(),
            text.clone(),
            Precision::Double53,
            u64::MAX,
            aa_mapworld::restore::RestoreMode::InspectPartial,
            aa_runtime::restoration::LoadBindings {
                native_load_mode: Some(mode),
                inactive_load: Some(inactive),
                resources: Some(resources.clone()),
            },
        )
        .unwrap();
        assert!(!attempt.report.runtime_publication_ready);
        let terrain =
            TerrainChunk::decode(&map.decode_chunk("TERRAIN", u64::MAX).unwrap()).unwrap();
        let extents = attempt
            .inventory
            .header
            .as_ref()
            .unwrap()
            .creator
            .world_extents_bits;
        let layout =
            TerrainGridLayout::from_world_extents(extents, terrain.resolution_bits).unwrap();
        let source = attempt
            .inventory
            .kingdoms
            .as_ref()
            .unwrap()
            .slots
            .iter()
            .find_map(|s| s.record.as_ref())
            .unwrap();
        let owner = ActorId::from_raw(source.serialized_id);
        let record = std::mem::replace(
            attempt.registry.value_mut(owner).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Kingdom(mut entity) = record else {
            panic!("Kingdom")
        };
        let runtime = entity.runtime.as_mut().unwrap();
        let source = actors
            .native_object_definition(&runtime.definition.proof.id, Precision::Double53)
            .unwrap();
        let fields_owner = fields.definition(&runtime.definition.proof.id).unwrap();
        assert_eq!(
            fields_owner.token().source(),
            &runtime.definition.proof.source
        );
        let before = json!({"flags":runtime.base.thing.flags,"position":runtime.base.thing.position_bits,"facing":runtime.base.thing.facing_bits,"actor_words":runtime.base.actor.as_ref().unwrap().words});
        for pc in [
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ] {
            let cell = aa_rules::objects::shape(2, &[layout.cell_size_bits], pc)
                .unwrap()
                .words;
            let geometry = aa_spatial::query::QueryGeometry {
                extent_bits: extents,
                cell_shape: aa_spatial::collision::Shape {
                    kind: cell[0],
                    radius_bits: cell[1],
                    bounds: [cell[2], cell[3], cell[4], cell[5]],
                },
            };
            let mut facts = MapFacts {
                map: aa_spatial::gworld_placement::GMapView {
                    width: layout.width,
                    height: layout.height,
                    coordinate_shift: layout.coordinate_shift,
                    cell_size_bits: layout.cell_size_bits,
                    extent_bits: extents,
                },
                geometry: &geometry,
                local_settings_mode: Some(mode),
                settings_mode_owner: None,
                global_settings_mode: live_placement::SettingsMode::Present(mode),
                zones: None,
                source_fields: Some(&fields),
                point_shape: None,
                placement_clearance_bits: None,
            };
            let mut record = Record {
                owner,
                shell: &mut runtime.base,
                definition: &runtime.definition,
                registry: &mut attempt.registry,
            };
            assert_eq!(
                live_placement::check_restored(
                    &catalog,
                    &mut record,
                    &mut facts,
                    &mut Unreached,
                    pc
                )
                .unwrap(),
                0
            );
            assert_eq!(
                json!({"flags":record.shell.thing.flags,"position":record.shell.thing.position_bits,"facing":record.shell.thing.facing_bits,"actor_words":record.shell.actor.as_ref().unwrap().words}),
                before
            );
            rows.push(json!({"map":map_name,"map_source":attempt.inventory.map_source,"precision":format!("{pc:?}"),
                "definition":runtime.definition.proof,"definition_flags":runtime.definition.flags,"component_mask":runtime.definition.component_mask,"source_precision":format!("{source_pc:?}"),
                "runtime_flags":record.shell.thing.flags,"owner_word":record.shell.actor.as_ref().unwrap().words[&0xe8],
                "position":record.shell.thing.position_bits,"facing":record.shell.thing.facing_bits,
                "shape":source.blocking.words,"settlement_space":fields.byte(&runtime.definition.proof.id,0x34).unwrap(),
                "source_word430":fields.word(&runtime.definition.proof.id,0x430).unwrap(),"source_word444":fields.word(&runtime.definition.proof.id,0x444).unwrap(),
                "source_byte1a8":fields.byte(&runtime.definition.proof.id,0x1a8).unwrap(),"source_required":fields.reference(&runtime.definition.proof.id,0x4e0).unwrap(),
                "extents":extents,"cell_size":layout.cell_size_bits,"settings_mode":mode,"result":0}));
        }
        // A missing source owner remains a reached dependency error, not zero.
        let cell = aa_rules::objects::shape(2, &[layout.cell_size_bits], Precision::Double53)
            .unwrap()
            .words;
        let geometry = aa_spatial::query::QueryGeometry {
            extent_bits: extents,
            cell_shape: aa_spatial::collision::Shape {
                kind: cell[0],
                radius_bits: cell[1],
                bounds: [cell[2], cell[3], cell[4], cell[5]],
            },
        };
        let mut facts = MapFacts {
            map: aa_spatial::gworld_placement::GMapView {
                width: layout.width,
                height: layout.height,
                coordinate_shift: layout.coordinate_shift,
                cell_size_bits: layout.cell_size_bits,
                extent_bits: extents,
            },
            geometry: &geometry,
            local_settings_mode: Some(mode),
            settings_mode_owner: None,
            global_settings_mode: live_placement::SettingsMode::Present(mode),
            zones: None,
            source_fields: None,
            point_shape: None,
            placement_clearance_bits: None,
        };
        let mut record = Record {
            owner,
            shell: &mut runtime.base,
            definition: &runtime.definition,
            registry: &mut attempt.registry,
        };
        assert!(
            live_placement::check_restored(
                &catalog,
                &mut record,
                &mut facts,
                &mut Unreached,
                Precision::Double53
            )
            .unwrap_err()
            .to_string()
            .contains("canonical completed source fields")
        );
        facts.source_fields = Some(&wrong_pc_fields);
        assert!(
            live_placement::check_restored(
                &catalog,
                &mut record,
                &mut facts,
                &mut Unreached,
                Precision::Single24
            )
            .unwrap_err()
            .to_string()
            .contains("arena PC differs")
        );
        facts.source_fields = Some(&fields);
        let mut wrong = runtime.definition.clone();
        wrong.component_mask = Some(0);
        let mut record = Record {
            owner,
            shell: &mut runtime.base,
            definition: &wrong,
            registry: &mut attempt.registry,
        };
        assert!(
            live_placement::check_restored(
                &catalog,
                &mut record,
                &mut facts,
                &mut Unreached,
                Precision::Double53
            )
            .unwrap_err()
            .to_string()
            .contains("held component mask differs")
        );
        wrong = runtime.definition.clone();
        wrong.flags ^= 1 << 18;
        let mut record = Record {
            owner,
            shell: &mut runtime.base,
            definition: &wrong,
            registry: &mut attempt.registry,
        };
        assert!(
            live_placement::check_restored(
                &catalog,
                &mut record,
                &mut facts,
                &mut Unreached,
                Precision::Double53
            )
            .unwrap_err()
            .to_string()
            .contains("held definition flags differ")
        );
        // Runtime virtual+B4 must follow the actual owned constructor/storage,
        // not merely a valid source category attached to an unrelated shell.
        for mutation in 0..6 {
            let mut shell = runtime.base.clone();
            match mutation {
                0 => shell.kind = aa_actors::loading::LoadKind::Object,
                1 => shell.object = None,
                2 => shell.actor = None,
                3 => shell.thing.definition.source.sha256.push('0'),
                4 => shell.thing.definition.template_chain.clear(),
                _ => shell.thing.definition.category.registry_index ^= 1,
            }
            let mut malformed = Record {
                owner,
                shell: &mut shell,
                definition: &runtime.definition,
                registry: &mut attempt.registry,
            };
            assert!(
                live_placement::check_restored(
                    &catalog,
                    &mut malformed,
                    &mut facts,
                    &mut Unreached,
                    Precision::Double53,
                )
                .unwrap_err()
                .to_string()
                .contains("B4 current shell/source/storage differs"),
                "mutation {mutation}",
            );
        }
        *attempt.registry.value_mut(owner).unwrap() = RestoredRecord::Kingdom(entity);
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-live-placement/actual-kingdom-inputs.json");
    let captured = json!({"scope":"actual restored first Kingdom shells and completed canonical source dependency fields; isolated placement only, prior lifecycle/world publication not certified","cases":rows});
    if std::env::var_os("AA_WRITE_CAPTURE").is_some() {
        std::fs::write(&path, serde_json::to_string_pretty(&captured).unwrap()).unwrap();
    } else {
        let expected: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(captured, expected);
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn actual_first_team_uses_owned_global_point_without_object_storage() {
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
    let resources =
        std::sync::Arc::new(aa_rules::resources::ResourceCatalog::load_v14(&mut sources).unwrap());
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(settings.clone());
    catalog.replace_current_text(text.clone());
    catalog.definition_text = Some(text.clone());
    let verified =
        aa_runtime::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(
            &catalog,
            &mut sources,
        )
        .unwrap();
    let point = aa_runtime::load_world_globals::PointShape::from_verified_startup_reference(
        &catalog, &verified,
    )
    .unwrap();
    aa_runtime::combat_source_completion::install(
        &catalog,
        &mut sources,
        verified.context(),
        verified.csv(),
        verified.globals(),
    )
    .unwrap();
    let native: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-placement/team-admission/native-team-placement.json"
    )).unwrap();
    assert_eq!(native["cases"].as_array().unwrap().len(), 6);
    for map_name in ["DeveloperMaps/(3v3)Firefight.RMP", "(2v2)Berlin.RMP"] {
        let archive = aa_mapworld::MapArchive::from_bytes(
            std::fs::read(
                root.join("reconstruction/assets/original/Data/Maps")
                    .join(map_name),
            )
            .unwrap(),
        )
        .unwrap();
        let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
            .unwrap()
            .begin(TaskKind::Normal)
            .unwrap();
        let mode = inactive.native_load_mode();
        let mut attempt = aa_runtime::restoration::Attempt::load_with_bindings(
            &archive,
            &actors,
            settings.clone(),
            text.clone(),
            Precision::Double53,
            u64::MAX,
            aa_mapworld::restore::RestoreMode::InspectPartial,
            aa_runtime::restoration::LoadBindings {
                native_load_mode: Some(mode),
                inactive_load: Some(inactive),
                resources: Some(resources.clone()),
            },
        )
        .unwrap();
        let owner = attempt
            .registry
            .iter()
            .find(|e| matches!(e.value, RestoredRecord::Team(_)))
            .unwrap()
            .id;
        let mut held = std::mem::replace(
            attempt.registry.value_mut(owner).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Team(team) = &mut held else {
            unreachable!()
        };
        let runtime = team.runtime.as_mut().unwrap();
        assert_eq!(runtime.definition.native_class, 5);
        assert_eq!(runtime.base.kind, aa_actors::loading::LoadKind::Thing);
        assert!(runtime.base.object.is_none() && runtime.base.actor.is_none());
        let extents = attempt
            .inventory
            .header
            .as_ref()
            .unwrap()
            .creator
            .world_extents_bits;
        let terrain =
            TerrainChunk::decode(&archive.decode_chunk("TERRAIN", u64::MAX).unwrap()).unwrap();
        let layout =
            TerrainGridLayout::from_world_extents(extents, terrain.resolution_bits).unwrap();
        for pc in [
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ] {
            let row = native["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["map"] == map_name && row["precision"] == format!("{pc:?}"))
                .unwrap();
            assert_eq!(
                row["input"]["definition_flags"],
                json!(runtime.definition.flags)
            );
            assert_eq!(
                row["input"]["runtime_flags_before_init"],
                json!(runtime.base.thing.flags)
            );
            assert_eq!(
                row["input"]["position_bits"],
                json!(runtime.base.thing.position_bits)
            );
            assert_eq!(
                row["input"]["facing_bits"],
                json!(runtime.base.thing.facing_bits)
            );
            assert_eq!(row["output"]["result"], 0);
            let cell = aa_rules::objects::shape(2, &[layout.cell_size_bits], pc)
                .unwrap()
                .words;
            let geometry = aa_spatial::query::QueryGeometry {
                extent_bits: extents,
                cell_shape: aa_spatial::collision::Shape {
                    kind: cell[0],
                    radius_bits: cell[1],
                    bounds: [cell[2], cell[3], cell[4], cell[5]],
                },
            };
            let mut facts = MapFacts {
                map: aa_spatial::gworld_placement::GMapView {
                    width: layout.width,
                    height: layout.height,
                    coordinate_shift: layout.coordinate_shift,
                    cell_size_bits: layout.cell_size_bits,
                    extent_bits: extents,
                },
                geometry: &geometry,
                local_settings_mode: Some(mode),
                settings_mode_owner: None,
                global_settings_mode: live_placement::SettingsMode::Unbound,
                zones: None,
                source_fields: None,
                point_shape: None,
                placement_clearance_bits: None,
            };
            let before = json!([
                runtime.base.thing.flags,
                runtime.base.thing.position_bits,
                runtime.base.thing.facing_bits
            ]);
            let mut record = Record {
                owner,
                shell: &mut runtime.base,
                definition: &runtime.definition,
                registry: &mut attempt.registry,
            };
            assert!(
                live_placement::check_restored(
                    &catalog,
                    &mut record,
                    &mut facts,
                    &mut Unreached,
                    pc
                )
                .unwrap_err()
                .to_string()
                .contains("requires the actual global point owner")
            );
            let shape = point.shape();
            assert_eq!(
                row["point_transport_probe"],
                json!([
                    shape.kind,
                    shape.radius_bits,
                    shape.bounds[0],
                    shape.bounds[1],
                    shape.bounds[2],
                    shape.bounds[3],
                ])
            );
            facts.point_shape = Some(&point);
            assert_eq!(
                live_placement::check_restored(
                    &catalog,
                    &mut record,
                    &mut facts,
                    &mut Unreached,
                    pc,
                )
                .unwrap(),
                row["output"]["result"].as_u64().unwrap() as u32
            );
            assert_eq!(
                before,
                json!([
                    record.shell.thing.flags,
                    record.shell.thing.position_bits,
                    record.shell.thing.facing_bits
                ])
            );
        }
        *attempt.registry.value_mut(owner).unwrap() = held;
    }
}
