//! Synthetic owner/order tests. Native tail behavior has its separate636-row
//! original/v14 oracle; these service fixtures are not live game services.
use super::*;
use crate::{
    bootstrap::DefinitionProof,
    inventory::{ByteSpan, Inventory, Placement, ThingState},
    restore::{
        self, GroundDefinitions, LoadCompletion, LoadedEntity, RestoreReport, RuntimeDefinition,
    },
};
use std::cell::RefCell;

fn definition(class: u32) -> RuntimeDefinition {
    let category = *aa_rules::native::CATEGORIES
        .iter()
        .find(|v| v.runtime_class == class)
        .unwrap();
    RuntimeDefinition {
        proof: DefinitionProof {
            id: format!("fixture{class}"),
            source: aa_rules::resources::Provenance {
                path: "synthetic-lifecycle".into(),
                sha256: "fixture".into(),
                span: aa_rules::tgi::Span { start: 0, end: 1 },
            },
            template_chain: vec![],
            category,
            base_flag_inputs: vec![],
        },
        native_class: class,
        flags: 0,
        object: None,
        component_mask: None,
        ground: GroundDefinitions::default(),
    }
}
fn placement(id: u32, ordinal: usize) -> Placement {
    Placement {
        record_ordinal: ordinal,
        source_span: ByteSpan {
            offset: 0,
            length: 0,
        },
        thing_id_utf16: "fixture1".encode_utf16().collect(),
        class_code: 1,
        serialized_id: id,
        thing: ThingState {
            version: 2,
            name_utf16: vec![],
            word_1c: 0x1234,
            position_bits: [0; 3],
            facing_bits: [0; 3],
            serialized_flags: 0,
            legacy_version_one_word: None,
        },
        object: None,
        actor: None,
        opaque_tail_span: ByteSpan {
            offset: 0,
            length: 0,
        },
        opaque_tail: vec![],
    }
}
fn record(id: u32) -> RestoredRecord {
    let definition = definition(1);
    RestoredRecord::Thing(Box::new(LoadedEntity {
        source: placement(id, 0),
        shell: aa_actors::loading::construct_shell_with_components(
            aa_actors::loading::LoadKind::Thing,
            definition.proof.clone(),
        ),
        definition,
        completion: LoadCompletion::AwaitingPostLoad,
        owner_source_slot: None,
        saved_components: vec![],
        actor_tail: None,
        doodad: None,
        special_object: None,
        failure: None,
        active_component: None,
    }))
}
fn inventory() -> WorldInventory {
    let source = crate::SourceIdentity::of(b"synthetic-lifecycle-fixture");
    WorldInventory {
        map_source: source.clone(),
        header: None,
        header_error: None,
        teams: Some(crate::kingdoms::Teams {
            source: source.clone(),
            slots: vec![],
        }),
        teams_error: None,
        team_generation_mismatches: vec![],
        kingdoms: Some(crate::kingdoms::Kingdoms {
            source: source.clone(),
            world_version: 28,
            major_count: 0,
            slots: vec![],
        }),
        kingdoms_error: None,
        kingdom_generation_mismatches: vec![],
        ownership: None,
        world: crate::WorldState {
            source: source.clone(),
            version: 28,
            rng_state: 123,
            word_dc: 0,
            begin_time_bits: 0,
            word_e4: 0,
            delta_bits: 0,
            word_140: 0,
            word_184: Some(0),
            word_194: Some(0),
            word_198: Some(0),
            word_44: Some(0),
            word_48: Some(0),
            trailing: vec![],
        },
        id_manager: crate::IdManagerState {
            source: source.clone(),
            next_slot: 1,
            highest_slot: 8,
            capacity: 65536,
            serialized_generations: vec![0; 8],
        },
        dynamic_things: Inventory {
            map_source: source.clone(),
            data_source: source.clone(),
            index_source: source,
            data_header_word: 0,
            placements: vec![placement(1, 0), placement(2, 1)],
            unsupported: vec![],
        },
        generation_mismatches: vec![],
        opaque_initialization: vec![],
        absent_sections: vec![],
    }
}

#[derive(Clone, Copy)]
enum Post {
    Normal,
    Error,
    Null,
    Replace(u32),
    RemoveIdentity,
}
struct Definitions {
    mode: Mode,
    post: Post,
    init_al: u8,
    destroy_receipt: bool,
    advance_world_version: bool,
    world_version_reads: RefCell<Vec<(usize, u32)>>,
    initialization_owner_addresses: RefCell<Vec<[usize; 5]>>,
    calls: RefCell<Vec<String>>,
}
impl Definitions {
    fn new(mode: Mode) -> Self {
        Self {
            mode,
            post: Post::Normal,
            init_al: 1,
            destroy_receipt: true,
            advance_world_version: false,
            world_version_reads: RefCell::new(vec![]),
            initialization_owner_addresses: RefCell::new(vec![]),
            calls: RefCell::new(vec![]),
        }
    }
    fn log(&self, call: &str, id: ActorId) {
        self.calls.borrow_mut().push(format!("{call}:{}", id.raw()));
    }
    fn require(&self, context: &Context<'_>, id: ActorId) {
        assert_eq!(context.current().unwrap().id(), id);
        assert!(matches!(
            &context.registry.get(id).unwrap().value,
            RestoredRecord::Loading
        ));
    }
}
impl RestoreDefinitions for Definitions {
    fn resolve(&self, id: &[u16], _: Precision) -> Result<RuntimeDefinition> {
        let name = String::from_utf16(id).unwrap();
        Ok(definition(if name == "fixture5" { 5 } else { 1 }))
    }
    fn thing_definition_exists(&self, _: &[u16], _: Precision) -> Result<bool> {
        Ok(true)
    }
    fn localize(&self, key: &[u16]) -> Result<Vec<u16>> {
        Ok(key.to_vec())
    }
    fn record_lifecycle_mode(&self) -> Mode {
        self.mode
    }
    fn lifecycle_init_null(&self, c: &mut Context<'_>, id: ActorId) -> Result<u8> {
        self.require(c, id);
        self.log("init", id);
        if self.advance_world_version {
            let (current, _, triple_version) = c.split_current_with_world_version()?;
            assert_eq!(current.id(), id);
            let version_address = triple_version as *const _ as usize;
            let parts = c.split_initialize_parts()?;
            assert_eq!(parts.held.id(), id);
            assert!(matches!(
                parts.registry.get(id).unwrap().value,
                RestoredRecord::Loading
            ));
            assert_eq!(parts.world_version as *const _ as usize, version_address);
            self.initialization_owner_addresses.borrow_mut().push([
                parts.rng as *const _ as usize,
                parts.messages as *const _ as usize,
                parts.presentation as *const _ as usize,
                parts.inventory as *const _ as usize,
                version_address,
            ]);
            self.world_version_reads
                .borrow_mut()
                .push((version_address, *parts.world_version));
            *parts.world_version = parts
                .world_version
                .checked_add(1)
                .ok_or("fixture world version overflow")?;
            parts.messages.push(NativeMessage {
                source_ordinal: 0,
                function: 0,
                line: 0,
                text: "synthetic mutation of the actual message owner".into(),
            });
        }
        let base = shell_mut(c.current_mut().unwrap().record_mut())?;
        assert!(base.thing.word_1c.is_some(), "the actual Load ran first");
        base.thing.word_1c = Some(0xC000 + id.raw());
        if id.raw() == 1 {
            assert!(
                c.registry.get(ActorId::from_raw(2)).is_none(),
                "next record was not loaded early"
            );
        }
        if id.raw() == 2 {
            assert_eq!(
                shell(&c.registry.get(ActorId::from_raw(1)).unwrap().value)?
                    .thing
                    .position_bits[0],
                77
            );
        }
        Ok(self.init_al)
    }
    fn lifecycle_post_create_null_one(&self, c: &mut Context<'_>, id: ActorId) -> Result<()> {
        self.require(c, id);
        self.log("post", id);
        shell_mut(c.current_mut().unwrap().record_mut())?
            .thing
            .position_bits[0] = 77;
        match self.post {
            Post::Normal => Ok(()),
            Post::Error => Err("injected PostCreate failure".into()),
            Post::Null => c.return_current_to_registry_and_clear(),
            Post::Replace(raw) => c.replace_current_from_registry(ActorId::from_raw(raw)),
            Post::RemoveIdentity => {
                c.registry.remove(id).unwrap();
                Ok(())
            }
        }
    }
    fn lifecycle_register_record(&self, c: &mut Context<'_>, id: ActorId) -> Result<()> {
        self.require(c, id);
        self.log("register", id);
        Ok(())
    }
    fn lifecycle_object_predicate(&self, c: &mut Context<'_>, id: ActorId) -> Result<u8> {
        self.require(c, id);
        self.log("predicate", id);
        Ok(0)
    }
    fn lifecycle_init_failed_warning(
        &self,
        c: &mut Context<'_>,
        id: ActorId,
        _: Precision,
    ) -> Result<()> {
        self.require(c, id);
        self.log("warning", id);
        Ok(())
    }
    fn lifecycle_destroy_record(&self, c: &mut Context<'_>, id: ActorId) -> Result<()> {
        self.require(c, id);
        self.log("destroy", id);
        if self.destroy_receipt {
            c.retire_current_after_destroy(id)?;
        }
        Ok(())
    }
}

fn restore(
    input: &WorldInventory,
    defs: &impl RestoreDefinitions,
) -> (IdentityRegistry<RestoredRecord>, RestoreReport) {
    let mut registry = input.id_manager.restore_empty_registry().unwrap();
    let mut rng = NativeRng::new(123);
    let report =
        restore::restore_records(&mut registry, input, defs, &mut rng, Precision::Double53);
    (registry, report)
}

#[test]
fn dynamic_and_team_records_finish_before_the_next_load_with_one_owner() {
    for teams in [false, true] {
        let mut input = inventory();
        if teams {
            input.dynamic_things.placements.clear();
            input.teams.as_mut().unwrap().slots = (1..=2)
                .map(|id| crate::kingdoms::TeamSlot {
                    source_presence_byte: 1,
                    record: Some(crate::kingdoms::TeamRecord {
                        source_span: ByteSpan {
                            offset: 0,
                            length: 0,
                        },
                        definition_id_utf16: "fixture5".encode_utf16().collect(),
                        serialized_id: id,
                        thing: placement(id, 0).thing,
                        version: 1,
                        slot_index: id - 1,
                        display_key_utf16: vec![],
                        words_4c: vec![],
                        words_58: vec![],
                        string_64_utf16: Some(vec![]),
                    }),
                })
                .collect();
        }
        let defs = Definitions::new(Mode::Immediate);
        let (registry, report) = restore(&input, &defs);
        assert_eq!(
            *defs.calls.borrow(),
            [
                "init:1",
                "post:1",
                "register:1",
                "predicate:1",
                "init:2",
                "post:2",
                "register:2",
                "predicate:2"
            ]
        );
        assert_eq!(registry.len(), 2);
        assert_eq!(report.lifecycle.len(), 2);
        assert!(
            report
                .lifecycle
                .iter()
                .all(|e| e.attempted && e.completed && !e.omitted && e.error.is_none())
        );
        assert!(!report.runtime_publication_ready);
        assert!(report.lifecycle_detached.is_empty());
    }
}

#[test]
fn diagnostic_omission_and_missing_required_init_cannot_be_readiness() {
    let defs = Definitions::new(Mode::DiagnosticOmitted);
    let (_, report) = restore(&inventory(), &defs);
    assert!(defs.calls.borrow().is_empty());
    assert!(
        report
            .lifecycle
            .iter()
            .all(|e| e.omitted && !e.attempted && !e.completed)
    );
    assert!(!report.runtime_publication_ready);
    struct Missing;
    impl RestoreDefinitions for Missing {
        fn resolve(&self, _: &[u16], _: Precision) -> Result<RuntimeDefinition> {
            Ok(definition(1))
        }
        fn thing_definition_exists(&self, _: &[u16], _: Precision) -> Result<bool> {
            Ok(true)
        }
        fn record_lifecycle_mode(&self) -> Mode {
            Mode::Immediate
        }
    }
    let (registry, report) = restore(&inventory(), &Missing);
    assert_eq!(registry.len(), 1);
    assert!(
        report.lifecycle[0]
            .error
            .as_ref()
            .unwrap()
            .contains("Init(NULL) native service is unavailable")
    );
    assert!(!report.lifecycle[0].completed);
    assert!(!report.runtime_publication_ready);
}

#[test]
fn immediate_world_version_is_the_same_authoritative_word_between_records_and_on_error() {
    let input = inventory();
    let mut defs = Definitions::new(Mode::Immediate);
    defs.advance_world_version = true;
    let (_, report) = restore(&input, &defs);
    let reads = defs.world_version_reads.borrow();
    assert_eq!(reads.len(), 2);
    assert_eq!(reads[0].1, 28);
    assert_eq!(reads[1].1, 29);
    assert_eq!(reads[0].0, reads[1].0, "one GWorld+2C owner across records");
    assert_eq!(report.world_version, 30);
    assert_eq!(report.native_messages.len(), 2);
    let addresses = defs.initialization_owner_addresses.borrow();
    assert_eq!(addresses.len(), 2);
    assert_eq!(
        addresses[0], addresses[1],
        "all disjoint parts keep one owner across records"
    );
    drop(addresses);
    assert_eq!(input.world.version, 28, "source inventory stays immutable");
    drop(reads);
    let mut defs = Definitions::new(Mode::Immediate);
    defs.advance_world_version = true;
    defs.post = Post::Error;
    let (_, report) = restore(&input, &defs);
    assert_eq!(
        report.world_version, 29,
        "earlier world write retained after tail error"
    );
    assert_eq!(defs.world_version_reads.borrow().len(), 1);
    assert_eq!(
        report.native_messages.len(),
        1,
        "actual queue mutation retained on error"
    );
}

#[test]
fn lifecycle_error_preserves_actual_prior_stores_and_stops_later_records() {
    let mut defs = Definitions::new(Mode::Immediate);
    defs.post = Post::Error;
    let (registry, report) = restore(&inventory(), &defs);
    assert_eq!(*defs.calls.borrow(), ["init:1", "post:1"]);
    let base = shell(&registry.get(ActorId::from_raw(1)).unwrap().value).unwrap();
    assert_eq!(base.thing.word_1c, Some(0xC001));
    assert_eq!(base.thing.position_bits[0], 77);
    assert!(registry.get(ActorId::from_raw(2)).is_none());
    assert!(report.lifecycle[0].calls[0].completed);
    assert!(!report.lifecycle[0].calls[1].completed);
    assert!(!report.lifecycle[0].completed);
}

#[test]
fn failed_load_never_invokes_the_immediate_tail_and_legacy_checks_remain_required() {
    let mut input = inventory();
    input.dynamic_things.placements[0].opaque_tail = vec![0xFF];
    let defs = Definitions::new(Mode::Immediate);
    let (_, report) = restore(&input, &defs);
    assert!(defs.calls.borrow().is_empty());
    assert!(!report.lifecycle[0].attempted);
    assert!(
        report.lifecycle[0]
            .error
            .as_ref()
            .unwrap()
            .contains("Load did not complete")
    );
    let mut input = inventory();
    input.world.version = 22;
    let (_, report) = restore(&input, &defs);
    assert!(defs.calls.borrow().is_empty());
    assert!(
        report.lifecycle[0]
            .error
            .as_ref()
            .unwrap()
            .contains("pre-v23")
    );
    assert!(!report.lifecycle[0].completed);
}

fn embedded_bytes(id: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    let name: Vec<u16> = "fixture1".encode_utf16().collect();
    bytes.extend_from_slice(&(name.len() as u16).to_le_bytes());
    for unit in name {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    for word in [1, id, 2] {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes.extend_from_slice(&0u16.to_le_bytes());
    for word in [0x1234u32, 0, 0, 0, 0, 0, 0, 0] {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes
}
fn embedded(
    defs: &Definitions,
    replacement: bool,
) -> (
    IdentityRegistry<RestoredRecord>,
    RestoreReport,
    Result<(Option<ActorId>, usize)>,
) {
    let input = inventory();
    let mut registry = input.id_manager.restore_empty_registry().unwrap();
    registry
        .register_saved(ActorId::from_raw(1), false, RestoredRecord::Loading)
        .unwrap();
    if replacement {
        registry
            .register_saved(ActorId::from_raw(4), false, record(4))
            .unwrap();
    }
    let mut report = RestoreReport::default();
    let mut rng = NativeRng::new(123);
    let mut latch = false;
    let mut version = 28;
    let result = restore::load_embedded_record(
        &mut registry,
        &embedded_bytes(3),
        ActorId::from_raw(1),
        defs,
        &mut restore::LoadContext {
            inventory: &input,
            rng: &mut rng,
            pc: Precision::Double53,
            updatable_registrations: &mut report.updatable_registrations,
            presentation_effects: &mut report.presentation_effects,
            skipped: &mut report.skipped,
            native_messages: &mut report.native_messages,
            lifecycle: &mut report.lifecycle,
            lifecycle_detached: &mut report.lifecycle_detached,
            world_version: &mut version,
            assert_latch: &mut latch,
            source_ordinal: 42,
        },
    );
    (registry, report, result)
}

#[test]
fn nested_records_finish_immediately_and_replacements_return_actual_output_identity() {
    let defs = Definitions::new(Mode::Immediate);
    let (registry, report, result) = embedded(&defs, false);
    assert_eq!(
        result.unwrap(),
        (Some(ActorId::from_raw(3)), embedded_bytes(3).len())
    );
    assert_eq!(
        *defs.calls.borrow(),
        ["init:3", "post:3", "register:3", "predicate:3"]
    );
    assert!(report.lifecycle[0].nested && report.lifecycle[0].completed);
    assert_eq!(report.lifecycle[0].source_ordinal, 42);
    assert!(matches!(
        &registry.get(ActorId::from_raw(1)).unwrap().value,
        RestoredRecord::Loading
    ));
    let mut defs = Definitions::new(Mode::Immediate);
    defs.post = Post::Replace(4);
    let (registry, report, result) = embedded(&defs, true);
    assert_eq!(result.unwrap().0, Some(ActorId::from_raw(4)));
    assert_eq!(report.lifecycle[0].saved_id, 3);
    assert_eq!(report.lifecycle[0].output_id, Some(4));
    assert_eq!(
        *defs.calls.borrow(),
        ["init:3", "post:3", "register:4", "predicate:4"]
    );
    assert!(matches!(
        &registry.get(ActorId::from_raw(3)).unwrap().value,
        RestoredRecord::Thing(_)
    ));
    assert!(matches!(
        &registry.get(ActorId::from_raw(4)).unwrap().value,
        RestoredRecord::Thing(_)
    ));
}

#[test]
fn null_postcreate_is_required_dereference_error_and_destroy_requires_receipt() {
    let mut defs = Definitions::new(Mode::Immediate);
    defs.post = Post::Null;
    let (registry, report, result) = embedded(&defs, false);
    assert!(result.unwrap_err().contains("4FB506 -> 4F9510"));
    assert!(registry.get(ActorId::from_raw(3)).is_some());
    assert!(!report.lifecycle[0].completed);
    let mut defs = Definitions::new(Mode::Immediate);
    defs.init_al = 0;
    defs.destroy_receipt = false;
    let (registry, report, result) = embedded(&defs, false);
    assert!(result.unwrap_err().contains("retirement receipt"));
    assert!(registry.get(ActorId::from_raw(3)).is_some());
    assert!(
        !report.lifecycle[0]
            .calls
            .iter()
            .any(|c| c.service == "clear_out_slot_after_destroy")
    );
    let mut defs = Definitions::new(Mode::Immediate);
    defs.init_al = 0;
    let (registry, report, result) = embedded(&defs, false);
    assert_eq!(result.unwrap(), (None, embedded_bytes(3).len()));
    assert!(registry.get(ActorId::from_raw(3)).is_none());
    assert!(report.lifecycle[0].completed);
    assert!(report.lifecycle[0].calls.last().unwrap().completed);
}

#[test]
fn invalid_registry_mutation_retains_the_actual_detached_allocation() {
    let mut defs = Definitions::new(Mode::Immediate);
    defs.post = Post::RemoveIdentity;
    let (registry, report) = restore(&inventory(), &defs);
    assert!(registry.get(ActorId::from_raw(1)).is_none());
    assert_eq!(report.lifecycle_detached.len(), 1);
    assert_eq!(report.lifecycle_detached[0].id().raw(), 1);
    assert_eq!(
        shell(report.lifecycle_detached[0].record())
            .unwrap()
            .thing
            .position_bits[0],
        77
    );
    assert!(!report.lifecycle[0].completed);
    assert!(
        report.lifecycle[0]
            .error
            .as_ref()
            .unwrap()
            .contains("ownership")
    );
}
