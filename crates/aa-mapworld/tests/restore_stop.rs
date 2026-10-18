use aa_mapworld::{
    MapArchive, WorldInventory,
    bootstrap::DefinitionProof,
    restore::{
        GroundDefinitions, RestoreDefinitions, RestoreMode, RestoreStage, RuntimeDefinition,
        restore_records, restore_records_with_mode,
    },
};
use aa_numeric::Precision;
use std::cell::Cell;
struct RejectFirst {
    calls: Cell<usize>,
}
impl RestoreDefinitions for RejectFirst {
    fn resolve(&self, id: &[u16], _: Precision) -> Result<RuntimeDefinition, String> {
        let id = String::from_utf16(id).unwrap();
        if id != "kingdom" {
            return Err("fixture only resolves the original Kingdom definition".into());
        }
        let category = *aa_rules::native::CATEGORIES
            .iter()
            .find(|c| c.name == "KINGDOM")
            .unwrap();
        let proof = DefinitionProof {
            id,
            source: aa_rules::resources::Provenance {
                path: "controlled-test-definition".into(),
                sha256: "fixture".into(),
                span: aa_rules::tgi::Span { start: 0, end: 0 },
            },
            template_chain: vec![],
            category,
            base_flag_inputs: vec![],
        };
        Ok(RuntimeDefinition {
            proof,
            native_class: 4,
            flags: 1 << 29,
            object: Some(aa_actors::creation::ObjectDefinition {
                variations_enabled: false,
                variation_weights_bits: vec![],
                total_variation_weight_bits: 0,
                scale_min_bits: 1f32.to_bits(),
                scale_max_bits: 1f32.to_bits(),
            }),
            component_mask: Some(64),
            ground: GroundDefinitions {
                properties_domain_count: Some(2),
                ..Default::default()
            },
        })
    }
    fn resource_domain_count(&self, _: Precision) -> Result<u32, String> {
        Ok(7)
    }
    fn register_updatable(
        &self,
        _: i32,
        _: &aa_mapworld::restore_components::UpdatableTarget,
    ) -> Result<(), String> {
        self.calls.set(self.calls.get() + 1);
        Err("injected duplicate Updatable rejection".into())
    }
}
#[test]
#[ignore = "requires original tutorial01.RMP"]
fn runtime_stops_before_later_kingdoms_after_registered_load_failure() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../reconstruction/assets/original/Data/Maps/Campaign/tutorial01.RMP");
    let archive = MapArchive::from_bytes(std::fs::read(root).unwrap()).unwrap();
    let inventory = WorldInventory::decode(&archive, 128 << 20).unwrap();
    let defs = RejectFirst {
        calls: Cell::new(0),
    };
    let mut registry = inventory.id_manager.restore_empty_registry().unwrap();
    let mut rng = aa_simulation::NativeRng::new(inventory.world.rng_state);
    let report = restore_records(
        &mut registry,
        &inventory,
        &defs,
        &mut rng,
        Precision::Double53,
    );
    assert_eq!(defs.calls.get(), 1);
    assert_eq!(registry.len(), 1);
    assert_eq!(report.events.len(), 1);
    assert_eq!(report.stopped_before, Some(RestoreStage::Kingdoms));
    assert!(report.events[0].registered);
    assert!(
        report.events[0]
            .error
            .as_ref()
            .unwrap()
            .contains("injected duplicate")
    );
    assert!(report.updatable_registrations.is_empty());
    let defs = RejectFirst {
        calls: Cell::new(0),
    };
    let mut registry = inventory.id_manager.restore_empty_registry().unwrap();
    let mut rng = aa_simulation::NativeRng::new(inventory.world.rng_state);
    let report = restore_records_with_mode(
        &mut registry,
        &inventory,
        &defs,
        &mut rng,
        Precision::Double53,
        RestoreMode::InspectPartial,
    );
    assert_eq!(defs.calls.get(), 3);
    assert_eq!(registry.len(), 3);
    assert!(report.events.len() > 3);
    assert!(!report.runtime_publication_ready);
}
