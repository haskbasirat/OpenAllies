//! Explicit laboratory storage inputs for the independently executed515620
//! rejection prefixes. No source loader/Catalog or actual map readiness claim.
use super::*;
use aa_actors::loading::{LoadKind, LoadShell, construct_shell_with_components};
use aa_mapworld::{bootstrap::DefinitionProof, restore::RuntimeDefinition};

fn definition() -> RuntimeDefinition {
    let category = *aa_rules::native::CATEGORIES
        .iter()
        .find(|v| v.native_kind == 2 && v.runtime_class == 3)
        .unwrap();
    RuntimeDefinition {
        proof: DefinitionProof {
            id: "labelled-city-storage-probe".into(),
            source: aa_rules::resources::Provenance {
                path: "labelled-laboratory-input".into(),
                sha256: "not-a-source-receipt".into(),
                span: aa_rules::tgi::Span { start: 0, end: 0 },
            },
            template_chain: vec![],
            category,
            base_flag_inputs: vec![],
        },
        native_class: 3,
        flags: 0,
        object: None,
        component_mask: None,
        ground: Default::default(),
    }
}
fn shell(definition: &RuntimeDefinition) -> LoadShell<DefinitionProof, RestoredComponent> {
    construct_shell_with_components(LoadKind::Actor, definition.proof.clone())
}
fn add_city(
    shell: &mut LoadShell<DefinitionProof, RestoredComponent>,
    owner: ActorId,
    ids: &mut Identities,
    zone_flags: u16,
    component: bool,
) {
    let actor = shell.actor.as_mut().unwrap();
    actor.zones_148.zones_mut().slots = vec![None; 9];
    let key = ids.allocate(owner, 8).unwrap();
    actor.zones_148.zones_mut().slots[8] = Some(Zone::new(key, owner.raw(), zone_flags));
    if component {
        actor.components.insert(
            ComponentSlot::City,
            RestoredComponent::City(Box::new(aa_actors::lair_city_state::CityState::new(owner))),
        );
    }
}

#[test]
fn native_registration_city_prefixes_read_same_held_storage_lazily() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-registration-city-entry/native-city-entry.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 57);
    let selected = [
        "self",
        "null-target",
        "dead",
        "no-owner",
        "zone-type-cleared",
        "city-null",
        "short-vector",
        "null-zone",
    ];
    let mut count = 0;
    for row in fixture["rows"].as_array().unwrap() {
        let input = &row["input"];
        if !selected.contains(&input["name"].as_str().unwrap()) {
            continue;
        }
        let pc = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let owner = ActorId::from_raw(1);
        let target_id = ActorId::from_raw(2);
        let definition = definition();
        let mut city = shell(&definition);
        let mut target = shell(&definition);
        target.thing.flags = u32::from(input["dead"].as_bool().unwrap());
        //Only the no-owner row reaches108. Every other unknown target word
        //stays unbound, so eager reads cannot hide behind constructor zeroes.
        if !input["owner"].as_bool().unwrap() {
            target.actor.as_mut().unwrap().words.insert(0xe8, 0);
        } else {
            target.actor.as_mut().unwrap().words.remove(&0xe8);
        }
        let mut ids = Identities::new();
        add_city(
            &mut city,
            owner,
            &mut ids,
            input["zone_flags"].as_u64().unwrap() as u16,
            input["city"].as_bool().unwrap(),
        );
        if input["count"] == 8 {
            city.actor
                .as_mut()
                .unwrap()
                .zones_148
                .zones_mut()
                .slots
                .truncate(8);
        }
        if input["zone"] == false {
            city.actor.as_mut().unwrap().zones_148.zones_mut().slots[8] = None;
        }
        let mut registry = aa_actors::IdentityRegistry::new();
        registry
            .register_saved(owner, true, RestoredRecord::Loading)
            .unwrap();
        registry
            .register_saved(target_id, true, RestoredRecord::Loading)
            .unwrap();
        let entering = match input["target"].as_str().unwrap() {
            "self" => owner,
            "null" => ActorId::NULL,
            _ => target_id,
        };
        let other = (entering == target_id).then_some(Other {
            owner: target_id,
            shell: &target,
            definition: &definition,
        });
        let city_address = &city as *const _ as usize;
        let result = city_entered_held(
            owner,
            &mut city,
            &definition,
            &mut registry,
            other,
            entering,
            &ids,
            pc,
            &mut RequiredEffects,
        );
        if row["native"]["terminal"] == "null_zone" {
            assert!(
                matches!(result, Err(Error::NativeNull("zone slot 8"))),
                "{row}: {result:?}"
            );
        } else {
            assert!(result.is_ok(), "{row}: {result:?}");
        }
        assert_eq!(&city as *const _ as usize, city_address);
        if let Some(RestoredComponent::City(city)) = city
            .actor
            .as_ref()
            .unwrap()
            .components
            .get(ComponentSlot::City)
        {
            assert!(city.candidates.head().is_none());
            assert!(city.values.is_empty());
        }
        assert!(row["native"]["candidates"].as_array().unwrap().is_empty());
        count += 1;
    }
    assert_eq!(count, 24);
}

#[test]
fn registration_city_refuses_foreign_zone_identity_before_admission() {
    let owner = ActorId::from_raw(1);
    let definition = definition();
    let mut city = shell(&definition);
    let mut actual = Identities::new();
    add_city(&mut city, owner, &mut actual, 0x100, true);
    let mut foreign = Identities::new();
    foreign.allocate(ActorId::from_raw(3), 8).unwrap();
    let mut registry = aa_actors::IdentityRegistry::new();
    registry
        .register_saved(owner, true, RestoredRecord::Loading)
        .unwrap();
    let error = city_entered_held(
        owner,
        &mut city,
        &definition,
        &mut registry,
        None,
        owner,
        &foreign,
        Precision::Double53,
        &mut RequiredEffects,
    )
    .unwrap_err();
    assert!(error.to_string().contains("zone allocation/owner differs"));
    let Some(RestoredComponent::City(city)) = city
        .actor
        .as_ref()
        .unwrap()
        .components
        .get(ComponentSlot::City)
    else {
        panic!()
    };
    assert!(city.candidates.head().is_none());
    assert!(city.header.scheduled_head.is_none());
}
