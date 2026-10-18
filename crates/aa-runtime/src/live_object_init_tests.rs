//! Synthetic owned-storage adapter tests; unreached world services are explicit
//! errors. Native coordinator arithmetic/order has its separate original oracle.
use super::*;
use aa_mapworld::restore::GroundDefinitions;
use serde_json::json;

#[test]
fn actual_original_and_v14_membership_and_blocking_entry_gates_match() {
    let proof: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-map-startup/native-object-init-gates.json"
    ))
    .unwrap();
    assert_eq!(proof["paired_case_count"], 360);
    assert_eq!(proof["native_execution_count"], 720);
    for table in proof["vtables"].as_array().unwrap() {
        assert_eq!(table["returns_true"], true);
        assert_eq!(table["leaf_bytes"], "b001c3");
    }
    for row in proof["cases"].as_array().unwrap() {
        let flags = row["flags"].as_u64().unwrap() as u32;
        let required = match row["entry"].as_str().unwrap() {
            "004ee4c0" => membership_service_required(flags),
            "004eb6e0" => blocking_service_required(flags),
            other => panic!("unexpected entry {other}"),
        };
        assert_eq!(
            required,
            row["output"]["outcome"] == "required_service",
            "{row}"
        );
    }
}

#[test]
fn actual_original_and_v14_memory_eligibility_and_ordered_reads_match() {
    struct Fields<'a> {
        row: &'a serde_json::Value,
        reads: Vec<serde_json::Value>,
    }
    impl Fields<'_> {
        fn read(&mut self, name: &str, size: u32) -> u32 {
            let value = self.row[name].as_u64().unwrap() as u32;
            self.reads.push(json!([name, size, value]));
            value
        }
    }
    impl MemoryFields for Fields<'_> {
        fn phase(&mut self) -> Result<u32> {
            Ok(self.read("simulation_phase", 4))
        }
        fn mask(&mut self) -> Result<u32> {
            Ok(self.read("component_mask", 4))
        }
        fn flags(&mut self) -> Result<u32> {
            Ok(self.read("flags", 4))
        }
        fn announce(&mut self) -> Result<u8> {
            Ok(self.read("announce_byte", 1) as u8)
        }
    }
    let proof: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-map-startup/native-memory-eligibility.json"
    ))
    .unwrap();
    assert_eq!(proof["paired_case_count"], 1296);
    assert_eq!(proof["native_execution_count"], 2592);
    for row in proof["cases"].as_array().unwrap() {
        let mut fields = Fields {
            row,
            reads: Vec::new(),
        };
        let result =
            memory_eligibility(row["native_class"].as_u64().unwrap() as u32, &mut fields).unwrap();
        assert_eq!(json!(result), row["output"]["result"], "{row}");
        assert_eq!(json!(fields.reads), row["output"]["ordered_reads"], "{row}");
    }
}

struct StopPlacement {
    shell_address: usize,
    mode_address: usize,
    calls: usize,
}
impl Services for StopPlacement {
    fn placement_null(
        &mut self,
        record: &mut Record<'_>,
        mode_owner: &Option<u32>,
        _: Precision,
    ) -> Result<u32> {
        assert_eq!(mode_owner as *const _ as usize, self.mode_address);
        assert_eq!(record.shell as *const _ as usize, self.shell_address);
        assert_eq!(record.word(0x38)?, 6.0f32.to_bits());
        assert_eq!(record.shell.object.as_ref().unwrap().byte_4c, 0);
        assert_eq!(
            &record.shell.thing.position_bits[..2],
            &[1.0f32.to_bits(), 3.0f32.to_bits()]
        );
        record.set_word(0x28, 21.0f32.to_bits())?;
        self.calls += 1;
        Err(Error::Unsupported(
            "test deliberately stops at actual placement boundary",
        ))
    }
    fn thickness_warning(&mut self, _: &mut Record<'_>) -> Result<()> {
        unreachable!()
    }
    fn placement_failure(&mut self, _: &mut Record<'_>, _: u32) -> Result<()> {
        unreachable!()
    }
    fn placement_map_warning(&mut self, _: &mut Record<'_>) -> Result<()> {
        unreachable!()
    }
    fn spatial_membership(&mut self, _: &mut Record<'_>) -> Result<()> {
        unreachable!()
    }
    fn blocking_notify(&mut self, _: &mut Record<'_>) -> Result<()> {
        unreachable!()
    }
    fn tile_footprint(&mut self, _: &mut Record<'_>) -> Result<()> {
        unreachable!()
    }
    fn retain_instances(&mut self, _: &mut Record<'_>) -> Result<bool> {
        unreachable!()
    }
    fn clear_instances(&mut self, _: &mut Record<'_>) -> Result<()> {
        unreachable!()
    }
    fn visibility_refresh(&mut self, _: &mut Record<'_>) -> Result<()> {
        unreachable!()
    }
}

#[test]
fn current_shell_stores_and_callback_mutations_survive_required_placement_error() {
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let provenance = aa_rules::resources::Provenance {
            path: "synthetic-owned-fields.tgi".into(),
            sha256: "synthetic".into(),
            span: aa_rules::tgi::Span { start: 0, end: 1 },
        };
        let proof = DefinitionProof {
            id: "synthetic-owned-object".into(),
            source: provenance.clone(),
            template_chain: vec![],
            category: aa_rules::native::CATEGORIES
                .iter()
                .copied()
                .find(|v| v.registry_index == 1)
                .unwrap(),
            base_flag_inputs: vec![],
        };
        let definition = RuntimeDefinition {
            proof: proof.clone(),
            native_class: 2,
            flags: 0,
            object: None,
            component_mask: None,
            ground: GroundDefinitions::default(),
        };
        let source = Source {
            id: proof.id.clone(),
            provenance,
            template_chain: proof.template_chain.clone(),
            category: proof.category,
            native_class: 2,
            source_precision: Precision::Single24,
            height_bits: 3.0f32.to_bits(),
            shape: Shape {
                kind: 0,
                radius_bits: 0,
                extent_10_bits: 0,
                extent_14_bits: 0,
            },
            blocking: aa_spatial::collision::Shape {
                kind: 0,
                radius_bits: 0,
                bounds: [0; 4],
            },
        };
        let mut shell = aa_actors::loading::construct_shell_with_components::<_, RestoredComponent>(
            LoadKind::Object,
            proof,
        );
        shell.thing.flags = 1 << 16;
        shell.thing.position_bits = [1.49f32.to_bits(), 2.51f32.to_bits(), 99.0f32.to_bits()];
        let object = shell.object.as_mut().unwrap();
        object.scale_bits = 2.0f32.to_bits();
        object.byte_4c = 0xA5;
        object.words_50_to_54 = [0x1234, 0x5678];
        object.memory.allocate_slots(9).unwrap();
        let mut host = StopPlacement {
            shell_address: &shell as *const _ as usize,
            mode_address: 0,
            calls: 0,
        };
        let mut registry = IdentityRegistry::new();
        let owner = ActorId::from_raw(1);
        registry
            .register_saved(owner, false, RestoredRecord::Loading)
            .unwrap();
        let base = serde_json::from_value(json!({"width":2,"height":2,"coordinate_shift":1,"cell_size_bits":2.0f32.to_bits(),"index_bias":0,"height_bits":[0,0,0,0]})).unwrap();
        let overlay = serde_json::from_value(json!({"width":2,"height":2,"coordinate_shift":1,"cell_size_bits":2.0f32.to_bits(),"index_bias":0,"height_bits":[0,0,0,0]})).unwrap();
        let terrain = aa_spatial::TerrainGrid {
            width: 2,
            height: 2,
            coordinate_shift: 1,
            cell_size: 2.0f32.to_bits(),
            cells: vec![
                aa_spatial::Cell {
                    overlay_kind: 0,
                    offset_min: 0,
                    offset_max: 0
                };
                4
            ],
        };
        let surface = GroundSurface {
            terrain: &terrain,
            heights: aa_spatial::height_layers::HeightLayers::new(&base, &overlay).unwrap(),
        };
        let mut objects = crate::world_object_list::WorldObjectList::constructed();
        let mut mode = Some(1);
        host.mode_address = &mode as *const _ as usize;
        {
            let record = Record {
                owner,
                shell: &mut shell,
                definition: &definition,
                registry: &mut registry,
            };
            let map = aa_zones::tile::MapGeometry {
                shift: 1,
                tile_size_bits: 2.0f32.to_bits(),
                extent_bits: [4.0f32.to_bits(); 2],
                width: 2,
                height: 2,
            };
            let point = aa_spatial::collision::Shape {
                kind: 0,
                radius_bits: 0,
                bounds: [0; 4],
            };
            assert_eq!(
                occupancy_indices(&record, &source, &map, &point, pc).unwrap(),
                vec![2]
            );
            let nonpoint = aa_spatial::collision::Shape {
                kind: 1,
                radius_bits: 1.0f32.to_bits(),
                bounds: [0; 4],
            };
            assert!(occupancy_indices(&record, &source, &map, &nonpoint, pc).is_err());
            struct MissingPhase;
            impl MemoryServices for MissingPhase {
                fn simulation_phase(&mut self, _: &MemoryRecord<'_>) -> Result<u32> {
                    Err(Error::Unsupported("actual simulation phase has no owner"))
                }
                fn definition_announce_byte(&mut self, _: &MemoryRecord<'_>) -> Result<u8> {
                    unreachable!("phase fails before source byte")
                }
            }
            assert!(
                memory_eligible(&record, &source, &mut MissingPhase)
                    .unwrap_err()
                    .to_string()
                    .contains("actual simulation phase has no owner")
            );
        }
        assert_eq!(source.source_precision(), Precision::Single24);
        let error = initialize(
            Context {
                record: Record {
                    owner,
                    shell: &mut shell,
                    definition: &definition,
                    registry: &mut registry,
                },
                source: &source,
                surface: &surface,
                map_mode: &mut mode,
                objects: &mut objects,
            },
            pc,
            &mut host,
        )
        .unwrap_err();
        assert!(error.to_string().contains("actual placement boundary"));
        assert_eq!(host.calls, 1);
        assert_eq!(
            shell.object.as_ref().unwrap().associated_handle,
            6.0f32.to_bits()
        );
        assert_eq!(
            shell.object.as_ref().unwrap().words_50_to_54,
            [0x1234, 0x5678]
        );
        assert_eq!(shell.object.as_ref().unwrap().memory.count(), 9);
        assert!(shell.object.as_ref().unwrap().memory.allocation_present());
        assert_eq!(shell.thing.position_bits[2], 21.0f32.to_bits());
        assert!(objects.head().is_none(), "list service was not reached");
        let record = Record {
            owner,
            shell: &mut shell,
            definition: &definition,
            registry: &mut registry,
        };
        assert!(
            record.word(0x4c).is_err(),
            "do not synthesize unknown adjacent bytes"
        );
    }
}

#[test]
fn actual_area_camera_doodad_memory_predicate_matches_native_ordered_reads() {
    struct Fields<'a> {
        row: &'a serde_json::Value,
        reads: Vec<serde_json::Value>,
    }
    impl Fields<'_> {
        fn read(&mut self, name: &str, size: u32) -> u32 {
            let value = self.row[name].as_u64().unwrap() as u32;
            self.reads.push(json!([name, size, value]));
            value
        }
    }
    impl MemoryFields for Fields<'_> {
        fn phase(&mut self) -> Result<u32> {
            Ok(self.read("simulation_phase", 4))
        }
        fn mask(&mut self) -> Result<u32> {
            Ok(self.read("component_mask", 4))
        }
        fn flags(&mut self) -> Result<u32> {
            Ok(self.read("flags", 4))
        }
        fn announce(&mut self) -> Result<u8> {
            Ok(self.read("announce_byte", 1) as u8)
        }
    }
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-map-startup/native-special-object-memory.json"
    ))
    .unwrap();
    assert_eq!(fixture["paired_case_count"], 1296);
    assert_eq!(fixture["native_execution_count"], 2592);
    for row in fixture["cases"].as_array().unwrap() {
        let mut fields = Fields { row, reads: vec![] };
        let result =
            memory_eligibility(row["native_class"].as_u64().unwrap() as u32, &mut fields).unwrap();
        assert_eq!(json!(result), row["output"]["result"], "{row}");
        assert_eq!(json!(fields.reads), row["output"]["ordered_reads"], "{row}");
    }
}
