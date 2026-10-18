use super::*;
use aa_actors::{
    IdentityRegistry,
    loading::{LoadKind, construct_shell_with_components},
};
use aa_mapworld::{
    MapArchive,
    restore::{GroundDefinitions, RuntimeDefinition},
};
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};

struct Unreached;
#[test]
fn actual_ordinary_runtime_shape_and_category_getters_are_pinned() {
    let proof: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-required-placement/native-shape-getters.json"
    ))
    .unwrap();
    let rows = proof["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 108);
    for row in rows {
        let class = row["runtime_class"].as_u64().unwrap();
        assert!(matches!(class, 2 | 3 | 4 | 6 | 7 | 9));
        match row["slot"].as_u64().unwrap() {
            0xc => assert_eq!(row["output"]["shape_definition_offset"], 0x38),
            0xb0 => assert_eq!(row["output"]["al"].as_u64().unwrap() != 0, class >= 2),
            0xb4 => assert_eq!(
                row["output"]["al"].as_u64().unwrap() != 0,
                matches!(class, 3 | 4)
            ),
            _ => unreachable!(),
        }
    }
}
impl ZoneActors for Unreached {
    fn owner(&mut self, _: u32) -> aa_zones::Result<u32> {
        unreachable!()
    }
    fn allied(&mut self, _: u32, _: u32) -> aa_zones::Result<bool> {
        unreachable!()
    }
    fn enemy(&mut self, _: u32, _: u32) -> aa_zones::Result<bool> {
        unreachable!()
    }
    fn zone_slot(&mut self, _: u32, _: u32) -> aa_zones::Result<u32> {
        unreachable!()
    }
    fn word4(&mut self, _: u32) -> aa_zones::Result<u32> {
        unreachable!()
    }
    fn component_b0(&mut self, _: u32) -> aa_zones::Result<u32> {
        unreachable!()
    }
    fn reveal_mask(&mut self, _: u32) -> aa_zones::Result<Option<u32>> {
        unreachable!()
    }
}
impl Services for Unreached {
    fn actor_link(&mut self, _: &mut Record<'_>, _: ActorId, _: u32) -> Result<Option<ActorId>> {
        unreachable!()
    }
    fn mobile_4d1120(&mut self, _: &mut Record<'_>, _: ActorId) -> Result<bool> {
        unreachable!()
    }
    fn enemy_140(&mut self, _: &mut Record<'_>, _: ActorId, _: u32) -> Result<bool> {
        unreachable!()
    }
    fn zone_actor(&mut self, _: u32) -> Result<ActorId> {
        unreachable!()
    }
    fn game_mode_e4(&mut self) -> Result<u32> {
        unreachable!()
    }
    fn current_thread_is_simulation(&mut self) -> Result<bool> {
        unreachable!()
    }
    fn global_mode_b28158(&mut self) -> Result<u32> {
        unreachable!()
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 map and source"]
fn actual_map_metadata_and_current_shell_bind_without_a_radius_default() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let settings = crate::settings::Settings::load_v14(&mut files, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let map_path = files
        .catalog()
        .into_iter()
        .filter(|e| {
            e.logical_path.to_ascii_lowercase().starts_with("maps/")
                && e.logical_path.to_ascii_lowercase().ends_with(".rmp")
        })
        .map(|e| e.logical_path)
        .min()
        .unwrap();
    let archive = MapArchive::from_bytes(files.read(&map_path).unwrap().bytes).unwrap();
    let header = aa_mapworld::header::MapHeader::decode(
        &archive.decode_chunk("MAP_HEADER", u64::MAX).unwrap(),
    )
    .unwrap();
    let owner = ActorId::from_raw(1);
    for p in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let terrain =
            BattleTerrain::load_with_sources(&archive, &mut files, &actors, &settings, p).unwrap();
        let layout = terrain.bound.layout;
        let map = GMapView {
            width: layout.width,
            height: layout.height,
            coordinate_shift: layout.coordinate_shift,
            cell_size_bits: layout.cell_size_bits,
            extent_bits: header.creator.world_extents_bits,
        };
        let index = SpatialIndex::new(
            aa_spatial::collision::Grid {
                dimensions: [map.width, map.height],
                coordinate_shift: map.coordinate_shift,
                cell_size_bits: map.cell_size_bits,
            },
            terrain.bound.cells.iter().map(|c| c.flags20).collect(),
            p,
        )
        .unwrap();
        let geometry = QueryGeometry::from_native_extents(&index, map.extent_bits, p).unwrap();
        let zones = ZoneMap::new(aa_zones::tile::MapGeometry {
            width: map.width,
            height: map.height,
            shift: map.coordinate_shift,
            tile_size_bits: map.cell_size_bits,
            extent_bits: map.extent_bits,
        })
        .unwrap();
        let mut catalog = Catalog::new(&actors);
        catalog.settings = Some(settings.clone());
        let source = actors
            .actors
            .iter()
            .find(|a| {
                actors
                    .native_category(&a.id)
                    .is_ok_and(|c| c.native_kind == 2 && c.runtime_class == 3)
            })
            .unwrap();
        let category = actors.native_category(&source.id).unwrap();
        let proof = DefinitionProof {
            id: source.id.clone(),
            source: source.source.clone(),
            template_chain: source.template_chain.clone(),
            category,
            base_flag_inputs: vec![],
        };
        let definition = RuntimeDefinition {
            proof: proof.clone(),
            native_class: category.runtime_class,
            flags: actors.native_final_flags(&source.id, p).unwrap(),
            object: None,
            component_mask: None,
            ground: GroundDefinitions::default(),
        };
        let object_source = actors
            .actors
            .iter()
            .find(|a| {
                actors
                    .native_category(&a.id)
                    .is_ok_and(|c| c.runtime_class == 2)
            })
            .unwrap();
        let object_category = actors.native_category(&object_source.id).unwrap();
        let object_definition = RuntimeDefinition {
            proof: DefinitionProof {
                id: object_source.id.clone(),
                source: object_source.source.clone(),
                template_chain: object_source.template_chain.clone(),
                category: object_category,
                base_flag_inputs: vec![],
            },
            native_class: 2,
            flags: 0,
            object: None,
            component_mask: None,
            ground: GroundDefinitions::default(),
        };
        let mut shell =
            construct_shell_with_components::<_, RestoredComponent>(LoadKind::Actor, proof);
        let mut registry = IdentityRegistry::new();
        registry
            .register_saved(owner, true, RestoredRecord::Loading)
            .unwrap();
        let mut record = Record {
            owner,
            shell: &mut shell,
            definition: &definition,
            registry: &mut registry,
        };
        let facts = MapFacts {
            map,
            geometry: &geometry,
            index: &index,
            zones: &zones,
            terrain: &terrain,
            source_fields: None,
        };
        let mut services = Unreached;
        let mut live = Live {
            catalog: &catalog,
            record: &mut record,
            map: &facts,
            services: &mut services,
        };
        assert!(live.actor_virtual(owner, 0xb0).unwrap());
        assert!(live.actor_virtual(owner, 0xb4).unwrap());
        live.record.shell.kind = LoadKind::Object;
        assert!(
            live.actor_virtual(owner, 0xb4)
                .unwrap_err()
                .to_string()
                .contains("shell/storage class differs")
        );
        live.record.shell.kind = LoadKind::Actor;
        let actor_storage = live.record.shell.actor.take();
        assert!(
            live.actor_virtual(owner, 0xb4)
                .unwrap_err()
                .to_string()
                .contains("shell/storage class differs")
        );
        live.record.shell.actor = actor_storage;
        let held_proof = live.record.shell.thing.definition.clone();
        live.record.shell.thing.definition = object_definition.proof.clone();
        live.record.definition = &object_definition;
        assert!(
            live.actor_virtual(owner, 0xb4)
                .unwrap_err()
                .to_string()
                .contains("shell/storage class differs")
        );
        live.record.shell.thing.definition = held_proof;
        live.record.definition = &definition;
        // Real registry checks generations; a GTID is never a direct pointer.
        assert_eq!(live.resolve_reference(1).unwrap(), Some(owner));
        assert_eq!(live.resolve_reference(0x10001).unwrap(), None);
        assert_eq!(live.resolve_reference(0).unwrap(), None);
        // The shell remains the authority after a reached callback mutates it.
        live.record.shell.thing.flags = 0x12345678;
        assert_eq!(live.actor_flags(owner).unwrap(), 0x12345678);
        live.record.shell.object.as_mut().unwrap().words_50_to_54[0] = 0x10001;
        assert_eq!(live.actor_reference_50(owner).unwrap(), 0x10001);
        let mut witnessed_difference = false;
        for y in 0..map.height {
            for x in 0..map.width {
                let t = (vec![[x, y]], 0);
                let bits = live.tile_terrain_flags_4c(&t).unwrap();
                assert_eq!(
                    bits,
                    terrain
                        .terrain_definition_at(x, y)
                        .unwrap()
                        .passability_bits
                );
                witnessed_difference |=
                    u32::from(bits & 1) != (index.tile([x, y]).unwrap().flags20 & 1);
            }
        }
        assert!(
            witnessed_difference,
            "metadata must not be substituted by runtime tile flags"
        );
        let error = live.manager_radius_74().unwrap_err().to_string();
        assert!(
            error.contains("combat source State is not installed"),
            "{error}"
        );
        let args = Arguments {
            position: [2.0f32.to_bits(); 2],
            definition: None,
            owner: 0,
            other: None,
            argument_7: 0,
            argument_8: 1,
        };
        let error = check(&catalog, &mut record, &facts, &mut services, args, p)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("combat source State is not installed"),
            "{error}"
        );
        let foreign = MapFacts {
            map: GMapView {
                width: map.width + 1,
                ..map
            },
            ..facts
        };
        assert!(
            check(&catalog, &mut record, &foreign, &mut services, args, p)
                .unwrap_err()
                .to_string()
                .contains("owners disagree")
        );
        // Install an explicitly diagnostic source owner at PC53. The private
        // source-read regression may inspect it; public check still refuses its
        // incomplete/unobserved mean cache. Placement p must never replace it.
        let source_pc = Precision::Double53;
        let fp = crate::combat_value_source_context::SourceFpContext::supplied_diagnostic(
            0x27f,
            0,
            0x1f80,
            true,
            "required placement source-PC regression only",
        )
        .unwrap();
        let csv = crate::combat_value_source_context::bind_csv_from_sources(
            &mut files,
            crate::combat_value_source_context::CsvScope::FreshNoProfile {
                label: "required placement regression; no actual profile read",
            },
        )
        .unwrap();
        crate::combat_source_completion::install(
            &catalog,
            &mut files,
            &fp,
            &csv,
            crate::combat_source_completion::Globals {
                assertion_guard: 0,
                ranged_lookup_guard: 0,
                csv_lookup_guard: 0,
                debug_stream_is_closed: true,
            },
        )
        .unwrap();
        let fields = actors.native_source_requirements(source_pc).unwrap();
        let wrong_fields = actors
            .native_source_requirements(Precision::Single24)
            .unwrap();
        let valid_facts = MapFacts {
            source_fields: Some(&fields),
            ..facts
        };
        let wrong_facts = MapFacts {
            source_fields: Some(&wrong_fields),
            ..valid_facts
        };
        let expected = catalog
            .canonical_component_mask(&source.id, source_pc)
            .unwrap();
        let mut valid_definition = definition.clone();
        valid_definition.component_mask = Some(expected);
        let mut bad_mask = valid_definition.clone();
        bad_mask.component_mask = Some(expected ^ 1);
        let mut bad_proof = valid_definition.clone();
        bad_proof.proof.source.sha256 = "changed-source".into();
        record.definition = &valid_definition;
        let mut live = Live {
            catalog: &catalog,
            record: &mut record,
            map: &valid_facts,
            services: &mut services,
        };
        let token = live.token(&source.id).unwrap();
        assert_eq!(live.source_precision().unwrap(), source_pc);
        assert_eq!(
            live.source_fields(&source.id).unwrap().source_precision(),
            source_pc
        );
        assert_eq!(live.definition_word(token, 0x174).unwrap(), expected);
        live.record.definition = &bad_mask;
        assert!(
            live.definition_word(token, 0x174)
                .unwrap_err()
                .to_string()
                .contains("actual source-PC projection")
        );
        live.record.definition = &bad_proof;
        assert!(
            live.definition_word(token, 0x174)
                .unwrap_err()
                .to_string()
                .contains("provenance differs")
        );
        live.record.definition = &valid_definition;
        live.map = &wrong_facts;
        assert!(
            live.source_fields(&source.id)
                .unwrap_err()
                .to_string()
                .contains("source-field precision differs")
        );
        assert!(
            live.manager_radius_74()
                .unwrap_err()
                .to_string()
                .contains("not complete")
        );
        // The load-time index retains true native nodes and generation mapping
        // after mutable field leases drop. Identity reads must not need fields.
        let mut identities = SpatialIndex::new(
            index.grid().clone(),
            terrain.bound.cells.iter().map(|c| c.flags20).collect(),
            p,
        )
        .unwrap();
        identities.register_load_reference(17, owner.raw()).unwrap();
        let source_shape = actors
            .native_object_definition(&source.id, source_pc)
            .unwrap();
        let shape = |s: &aa_rules::objects::Shape| {
            let w = s.words;
            Shape {
                kind: w[0],
                radius_bits: w[1],
                bounds: [w[2], w[3], w[4], w[5]],
            }
        };
        let fields = aa_spatial::index::Member {
            key: 17,
            reference: owner.raw(),
            actor: aa_spatial::collision::Actor {
                definition: token,
                flags: 0,
                position: [0; 2],
                facing: [0, (-1f32).to_bits()],
            },
            height_bits: 0,
            facing_z_bits: 0,
            cached_tile: [0; 2],
            blocking: shape(&source_shape.blocking),
            soft: shape(&source_shape.soft),
            target: shape(&source_shape.target),
            actor_category: true,
            wall_present: false,
        };
        {
            let mut lease = identities.lease_load_fields(vec![fields]).unwrap();
            lease.index_mut().refresh_membership(17, true).unwrap();
        }
        assert!(identities.member(17).is_err());
        let node = identities.tile([0, 0]).unwrap().first_node.unwrap();
        let identity_facts = MapFacts {
            index: &identities,
            ..valid_facts
        };
        live.map = &identity_facts;
        assert_eq!(live.node_actor(node).unwrap(), owner);
        println!(
            "{p:?}: actual{map_path}; all {} metadata cells, owned shell, generations, source-PC53/foreign-PC/current-mask guards and manager refusal verified",
            map.width * map.height
        );
    }
}
