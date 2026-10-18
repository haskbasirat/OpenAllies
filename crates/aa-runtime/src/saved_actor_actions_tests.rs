use super::*;
use crate::saved_organization_actions::SavedActionContext;
use aa_actors::{
    ActorId, IdentityRegistry, ability_flags as flags,
    components::ComponentSlot,
    ground_states::ElementState,
    loading::{LoadKind, construct_shell_with_components},
};
use aa_mapworld::{
    IdManagerState, Inventory, SourceIdentity, WorldState,
    bootstrap::DefinitionProof,
    restore::{GroundDefinitions, RestoredComponent, RestoredRecord, RuntimeDefinition},
};
use serde_json::{Value, json};

fn inventory() -> WorldInventory {
    let source = SourceIdentity::of(b"explicit synthetic action context; no map fields are read");
    WorldInventory {
        map_source: source.clone(),
        header: None,
        header_error: None,
        teams: None,
        teams_error: None,
        team_generation_mismatches: vec![],
        kingdoms: None,
        kingdoms_error: None,
        kingdom_generation_mismatches: vec![],
        ownership: None,
        world: WorldState {
            source: source.clone(),
            version: 27,
            rng_state: 0,
            word_dc: 0,
            begin_time_bits: 0,
            word_e4: 0,
            delta_bits: 0x3d4ccccd,
            word_140: 0,
            word_184: None,
            word_194: None,
            word_198: None,
            word_44: None,
            word_48: None,
            trailing: vec![],
        },
        id_manager: IdManagerState {
            source: source.clone(),
            next_slot: 1,
            highest_slot: 0,
            capacity: 65536,
            serialized_generations: vec![],
        },
        dynamic_things: Inventory {
            map_source: source.clone(),
            data_source: source.clone(),
            index_source: source,
            data_header_word: 0,
            placements: vec![],
            unsupported: vec![],
        },
        generation_mismatches: vec![],
        opaque_initialization: vec![],
        absent_sections: vec![],
    }
}
fn definition() -> RuntimeDefinition {
    RuntimeDefinition {
        proof: DefinitionProof {
            id: "explicit-held-action-fixture".into(),
            source: aa_rules::resources::Provenance {
                path: "synthetic".into(),
                sha256: "synthetic".into(),
                span: aa_rules::tgi::Span { start: 0, end: 1 },
            },
            template_chain: vec![],
            category: *aa_rules::native::CATEGORIES
                .iter()
                .find(|v| v.runtime_class == 3)
                .unwrap(),
            base_flag_inputs: vec![],
        },
        native_class: 3,
        flags: 0,
        object: None,
        component_mask: Some(0),
        ground: GroundDefinitions::default(),
    }
}

#[test]
fn ordinary_held_actor_uses_actual_component_presence_and_commits_native_prefix() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-held-actor-actions/native-held-actor-actions.json"
    ))
    .unwrap();
    let actors =
        aa_rules::actors::ActorCatalog::from_files(vec![], aa_rules::tgi::Limits::default());
    let catalog = Catalog::new(&actors);
    let inventory = inventory();
    let definition = definition();
    for row in doc["cases"].as_array().unwrap() {
        let input = &row["input"];
        let owner = ActorId::from_raw(17);
        let mut shell = construct_shell_with_components::<_, RestoredComponent>(
            LoadKind::Actor,
            definition.proof.clone(),
        );
        shell.thing.flags = input["flags"].as_u64().unwrap() as u32;
        shell.thing.word_c = u32::from(input["renderer"].as_bool().unwrap());
        shell.object.as_mut().unwrap().byte_4c = input["object4c"].as_u64().unwrap() as u8;
        let actor = shell.actor.as_mut().unwrap();
        actor
            .words
            .insert(0xe8, u32::from(input["owner"].as_bool().unwrap()));
        actor
            .words
            .insert(0xf0, input["actions"].as_u64().unwrap() as u32);
        actor
            .words
            .insert(0xf4, (input["actions"].as_u64().unwrap() >> 32) as u32);
        actor.components.state.low = input["state"].as_u64().unwrap() as u32;
        actor.components.state.high = (input["state"].as_u64().unwrap() >> 32) as u32;
        if input["element"].as_bool().unwrap() {
            let mut element = ElementState::new(owner).unwrap();
            element.element_definition = input["definition"]
                .as_bool()
                .unwrap()
                .then(|| "explicit native nonnull ElementData identity".into());
            element.flag_1c = input["ready"].as_bool().unwrap();
            actor.components.insert(
                ComponentSlot::Element,
                RestoredComponent::Element(Box::new(element)),
            );
        }
        let actor_address = actor as *const _ as usize;
        let state = actor.components.state;
        let element_address = actor
            .components
            .get(ComponentSlot::Element)
            .map(|e| e as *const _ as usize);
        let mut registry = IdentityRegistry::new();
        registry
            .register_saved(owner, false, RestoredRecord::Loading)
            .unwrap();
        let mut actions = SavedActions {
            catalog: &catalog,
            context: SavedActionContext {
                inventory: &inventory,
                owner,
                owner_actor: actor,
                owner_definition: &definition,
                owner_thing_flags: shell.thing.flags,
                owner_thing_word_c: shell.thing.word_c,
                owner_object_byte_4c: shell.object.as_ref().unwrap().byte_4c,
                organization: None,
                registry: &mut registry,
                precision: Precision::Double53,
                source_precision: Precision::Double53,
            },
            active_mind: None,
        };
        assert!(
            !flags::World::has_component(&actions, owner, ComponentSlot::Organization).unwrap()
        );
        let result = flags::rebuild(&mut actions, owner);
        let outcome = match result {
            Ok(()) => "ok",
            Err(e) if e.to_string().contains("76C7F0") => "owner_views_required",
            Err(e) if e.to_string().contains("renderer+54") => "renderer_required",
            Err(e) => panic!("unexpected {e} for {row}"),
        };
        assert_eq!(json!(outcome), row["output"]["outcome"], "{row}");
        assert_eq!(
            json!(
                flags::World::actor_fields(&actions, owner)
                    .unwrap()
                    .action_bits
            ),
            row["output"]["actions"],
            "{row}"
        );
        assert_eq!(
            actions.shell(owner).unwrap() as *const _ as usize,
            actor_address
        );
        assert_eq!(actions.shell(owner).unwrap().components.state, state);
        assert_eq!(
            actions
                .component(owner, ComponentSlot::Element)
                .unwrap()
                .map(|e| e as *const _ as usize),
            element_address
        );
        assert!(
            actions
                .context
                .registry
                .get(owner)
                .is_some_and(|e| matches!(e.value, RestoredRecord::Loading))
        );
    }
}

#[test]
fn public_rebuild_rejects_unbound_source_before_mutating_held_actions() {
    let actors =
        aa_rules::actors::ActorCatalog::from_files(vec![], aa_rules::tgi::Limits::default());
    let catalog = Catalog::new(&actors);
    let inventory = inventory();
    let definition = definition();
    let mut shell = construct_shell_with_components::<_, RestoredComponent>(
        LoadKind::Actor,
        definition.proof.clone(),
    );
    shell.actor.as_mut().unwrap().words.insert(0xf0, 0x12345678);
    let owner = ActorId::from_raw(17);
    let mut registry = IdentityRegistry::new();
    registry
        .register_saved(owner, false, RestoredRecord::Loading)
        .unwrap();
    let mut record = Record {
        owner,
        shell: &mut shell,
        definition: &definition,
        registry: &mut registry,
    };
    assert!(rebuild(&catalog, &inventory, &mut record, Precision::Double53).is_err());
    assert_eq!(
        record.shell.actor.as_ref().unwrap().words[&0xf0],
        0x12345678
    );
}
