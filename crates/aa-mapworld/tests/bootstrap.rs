use aa_actors::{ActorId, IdentityRegistry};
use aa_mapworld::{
    bootstrap::{LoadedBase, PendingStep, ResolvedDefinition, restore_base},
    inventory::{ByteSpan, ObjectState, Placement, ThingState},
};
#[test]
fn proof_identity_uses_authored_spelling_after_case_insensitive_lookup() {
    let flags = aa_rules::native::THING_FLAGS
        .iter()
        .map(|(name, _)| format!("bool {name}=false\n"))
        .collect::<String>();
    let text =
        format!("[Thing]{{\nIDS=Authored_Object\nstring category=OBJECT\n[Flags]\n{flags}}}\n");
    let catalog = aa_rules::actors::ActorCatalog::from_files(
        vec![aa_rules::sources::ResolvedFile {
            logical_path: "identity-fixture.tgi".into(),
            origin: aa_rules::sources::Origin::V14Loose,
            sha256: aa_rules::sources::sha256(text.as_bytes()),
            bytes: text.as_bytes().to_vec(),
        }],
        aa_rules::tgi::Limits::default(),
    );
    for request in ["authored_object", "AUTHORED_OBJECT", "Authored_Object"] {
        let resolved = aa_mapworld::bootstrap::resolve_definition(
            &catalog,
            &request.encode_utf16().collect::<Vec<_>>(),
            aa_numeric::Precision::Double53,
        )
        .unwrap();
        assert_eq!(resolved.definition.id, "Authored_Object");
        assert_eq!(resolved.definition.source.path, "identity-fixture.tgi");
    }
}
fn record() -> Placement {
    Placement {
        record_ordinal: 0,
        source_span: ByteSpan {
            offset: 8,
            length: 100,
        },
        thing_id_utf16: "source_object".encode_utf16().collect(),
        class_code: 2,
        serialized_id: 0x12340007,
        thing: ThingState {
            version: 2,
            name_utf16: vec![0xd800, 0, 65],
            word_1c: 0x7fc01234,
            position_bits: [0x80000000, 0x7f800000, 0x42aa1234],
            facing_bits: [0x7fc12345, 1, 0xbf800000],
            serialized_flags: 0xffff00ab,
            legacy_version_one_word: None,
        },
        object: Some(ObjectState {
            version: 5,
            scale_multiplier_bits: 0x3f800000,
            variant_index: Some(0),
            associated_object_id: Some(0),
            legacy_word: Some(0),
            nested_count: Some(0),
        }),
        actor: None,
        opaque_tail_span: ByteSpan {
            offset: 108,
            length: 3,
        },
        opaque_tail: vec![1, 2, 3],
    }
}
fn registry() -> IdentityRegistry<LoadedBase<&'static str>> {
    let mut generations = vec![0; 8];
    generations[7] = 0x1234;
    IdentityRegistry::from_load_metadata(20, 8, &generations).unwrap()
}
#[test]
fn restore_uses_saved_identity_and_exact_load_bits_without_new_creation_defaults() {
    let mut registry = registry();
    let input = record();
    let id = restore_base(
        &mut registry,
        input.clone(),
        ResolvedDefinition {
            definition: "verified-definition",
            native_class: 2,
            flag_word: 0x20abcdef,
        },
    )
    .unwrap();
    assert_eq!(id, ActorId::from_raw(input.serialized_id));
    assert_eq!(registry.next_slot(), 20);
    assert_eq!(registry.highest_slot(), 8);
    let loaded = &registry.get(id).unwrap().value;
    assert_eq!(loaded.shell.thing.position_bits, input.thing.position_bits);
    assert_eq!(loaded.shell.thing.facing_bits, input.thing.facing_bits);
    assert_eq!(loaded.shell.thing.name_utf16, input.thing.name_utf16);
    assert_eq!(loaded.shell.thing.word_1c, Some(input.thing.word_1c));
    assert_eq!(
        loaded.shell.thing.flags,
        input.thing.runtime_flags(0x80000000, 0x20abcdef)
    );
    assert_eq!(loaded.source_record, input);
    assert_eq!(
        loaded.shell.object.as_ref().unwrap().scale_bits,
        0,
        "definition-based scale must remain pending, not guessed"
    );
    assert!(
        loaded
            .remaining
            .contains(&PendingStep::ObjectDefinitionScaleAndVariant)
    );
    assert!(!loaded.long_name_cleared);
    assert!(
        restore_base(
            &mut registry,
            record(),
            ResolvedDefinition {
                definition: "duplicate",
                native_class: 2,
                flag_word: 0
            }
        )
        .is_err()
    );
    assert_eq!(registry.len(), 1);
}
#[test]
fn stale_generations_and_definition_class_mismatches_do_not_create_or_reseed() {
    let mut registry = registry();
    assert!(
        restore_base(
            &mut registry,
            record(),
            ResolvedDefinition {
                definition: "wrong-category",
                native_class: 3,
                flag_word: 0
            }
        )
        .is_err()
    );
    assert_eq!(registry.len(), 0);
    let mut stale = record();
    stale.serialized_id ^= 1 << 16;
    assert!(
        restore_base(
            &mut registry,
            stale,
            ResolvedDefinition {
                definition: "wrong-generation",
                native_class: 2,
                flag_word: 0
            }
        )
        .is_err()
    );
    assert_eq!(registry.len(), 0);
    assert_eq!(registry.generation_at(7), 0x1234);
}
