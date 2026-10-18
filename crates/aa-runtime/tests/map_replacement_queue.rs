use aa_actors::ActorId;
use aa_runtime::map_replacement_queue::*;
use serde_json::{Value, json};

fn definition(token: u32) -> DefinitionRef {
    DefinitionRef {
        token,
        canonical_ids: format!("definition{token}"),
    }
}
fn normalized(request: RequestRef<'_>) -> Value {
    match request {
        RequestRef::Object(r) => json!({"actor":false,"source":r.source.raw(),
            "definition":r.params.definition.as_ref().map(|d|d.token),
            "position":r.params.position_bits,"direction":r.params.direction_bits,
            "variant":r.params.variant,"requested":r.params.requested_id,
            "name_length":r.params.name_utf16.len(),"properties":[],"properties_alive":true}),
        RequestRef::Actor(r) => {
            let p = &r.params;
            json!({"actor":true,"source":r.source.raw(),"definition":p.definition.token,
                "position":p.position_bits,"direction":p.direction_bits,"variant":p.variant,
                "requested":p.requested_id,"name_length":p.name_utf16.len(),
                "properties":r.properties.iter().map(|p|p.as_ref().map(|p|p.token)).collect::<Vec<_>>(),
                "properties_alive":true,"actor_bytes":[p.byte_50,p.byte_51,p.byte_52],
                "actor_words":{"0x28":p.owner,"0x2c":p.organization_layout.as_ref().map_or(0, |_|panic!("native null layout")),
                    "0x30":p.word_30,"0x34":p.word_34,"0x38":p.mode,"0x3c":p.word_3c,
                    "0x40":p.word_40,"0x44":p.word_44,"0x48":p.word_48,"0x4c":p.word_4c}})
        }
    }
}
#[test]
fn original_capture_growth_alias_copy_and_separate_queue_order_match() {
    let doc: Value = serde_json::from_slice(include_bytes!(
        "../../../research/skirmish-20261016-map-replacement-queue/native-queue.json"
    ))
    .unwrap();
    assert_eq!(doc["cases"].as_array().unwrap().len(), 30);
    for row in doc["cases"].as_array().unwrap() {
        let mut queue = Queue::default();
        let mut warnings = Vec::new();
        for (input, outcome) in row["input"]["requests"]
            .as_array()
            .unwrap()
            .iter()
            .zip(row["outcomes"].as_array().unwrap())
        {
            let snapshot = Snapshot {
                id: ActorId::from_raw(input["source"].as_u64().unwrap() as u32),
                position_bits: serde_json::from_value(input["position"].clone()).unwrap(),
                direction_bits: serde_json::from_value(input["direction"].clone()).unwrap(),
            };
            let target = input["replacement"]
                .as_u64()
                .map(|id| definition(id as u32));
            let result = if input["actor"] == true {
                let properties = input["properties"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|p| {
                        Ok(PropertyInput {
                            kind: p["kind"].as_u64().unwrap() as u32,
                            source: ActorId::from_raw(p["source"].as_u64().unwrap() as u32),
                            definition: p["definition"].as_u64().map(|v| v as u32),
                        })
                    });
                actor_payload(
                    snapshot,
                    input["owner"]
                        .as_u64()
                        .filter(|v| *v != 0)
                        .map(|v| ActorId::from_raw(v as u32)),
                    input["original_definition"].as_u64().unwrap() as u32,
                    target,
                    properties,
                    &mut |id| Ok(definition(id)),
                    |_| Ok(input["required"].as_u64().map(|v| v as u32)),
                    &mut warnings,
                )
                .map(Request::Actor)
            } else {
                Ok(Request::Object(object_payload(snapshot, target)))
            };
            if outcome == "queued" {
                queue.append(result.unwrap());
            } else {
                assert!(result.is_err(), "{}", row["input"]["name"]);
            }
        }
        if let Some(alias) = row["input"].get("alias_append") {
            let kind = if alias["vector"] == 0 {
                Kind::Object
            } else {
                Kind::Actor
            };
            queue
                .append_existing(kind, alias["index"].as_u64().unwrap() as usize)
                .unwrap();
            let source = &queue.actors()[0];
            let copy = queue.actors().last().unwrap();
            assert_eq!(source, copy);
            assert_ne!(source.properties.as_ptr(), copy.properties.as_ptr());
        }
        let actual = [
            queue
                .objects()
                .iter()
                .map(|v| normalized(RequestRef::Object(v)))
                .collect::<Vec<_>>(),
            queue
                .actors()
                .iter()
                .map(|v| normalized(RequestRef::Actor(v)))
                .collect(),
        ];
        assert_eq!(json!(actual), row["queues"], "{}", row["input"]["name"]);
        assert_eq!(
            json!(
                warnings
                    .iter()
                    .map(|w| [w.actor.raw(), w.property.token])
                    .collect::<Vec<_>>()
            ),
            row["warnings"]
        );
        assert_eq!(
            json!(
                queue
                    .native_order()
                    .map(|r| match r {
                        RequestRef::Object(v) => v.source.raw(),
                        RequestRef::Actor(v) => v.source.raw(),
                    })
                    .collect::<Vec<_>>()
            ),
            row["visit_order"]
        );
        assert_eq!(
            json!([queue.objects().len(), queue.actors().len()]),
            row["counts_after_visit"]
        );
    }
}

struct StagedSpatial;
impl aa_runtime::actor_world::Spatial for StagedSpatial {
    fn placement(
        &self,
        _: ActorId,
    ) -> aa_runtime::actor_world::Result<aa_runtime::actor_world::PlacementState> {
        panic!("test actors are staged; no fake published placement")
    }
    fn publish(
        &mut self,
        _: ActorId,
        _: aa_runtime::actor_world::PlacementState,
    ) -> aa_runtime::actor_world::Result<()> {
        panic!("capture must not publish")
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn actual_firefight_capture_reads_current_owners_and_preserves_actor_state() {
    use aa_numeric::Precision;
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use aa_runtime::{
        actor_world::RuntimeComponent, localization::Localization, restoration::Attempt,
        settings::Settings, skirmish_start::Definitions,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut files, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut files, Limits::default()).unwrap();
    let source = ActorCatalog::load_v14(&mut files).unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/DeveloperMaps/(3v3)Firefight.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let attempt = Attempt::load_with_native_mode(
        &map,
        &source,
        settings,
        text,
        Precision::Double53,
        u64::MAX,
        aa_mapworld::restore::RestoreMode::StopOnError,
        Some(1),
    )
    .unwrap();
    assert!(attempt.report.events.iter().all(|e| e.error.is_none()));
    let mut staged = attempt.into_staged().unwrap();
    let definitions =
        Definitions::bind(&source, &mut staged.definitions, Precision::Double53).unwrap();
    let target = staged
        .definitions
        .intern("ActorDefinition", "gbr_corps")
        .unwrap();
    let (id, key) = staged
        .actors
        .identities()
        .iter()
        .find_map(|e| {
            (staged.actors.object(e.value).ok()?.definition.proof.id == "placeholder_corpsunpacked")
                .then_some((e.id, e.value))
        })
        .expect("real starting placeholder");
    let placement = staged.actors.staged_placement(key).unwrap();
    let owner = staged.actors.ordinary_owner(id).unwrap();
    let before = format!("{:?}", staged.actors.entity(key).unwrap());
    let seed = staged.rng.state;
    let mut queue = Queue::default();
    let mut warnings = Vec::new();
    let context = Context {
        actors: &staged.actors,
        spatial: &StagedSpatial,
        definitions: &definitions,
        bank: &staged.definitions,
        source: &source,
        precision: Precision::Double53,
    };
    context
        .capture(&mut queue, key, Some(target), Kind::Actor, &mut warnings)
        .unwrap();
    context
        .capture(&mut queue, key, None, Kind::Object, &mut warnings)
        .unwrap();
    assert_eq!(
        queue.actors()[0].params.position_bits,
        placement.position_bits
    );
    assert_eq!(
        queue.actors()[0].params.direction_bits,
        [placement.facing_bits[0], placement.facing_bits[1]]
    );
    assert_eq!(
        queue.actors()[0].params.owner,
        owner.map_or(0, ActorId::raw)
    );
    assert_eq!(
        queue.actors()[0].params.definition.canonical_ids,
        "gbr_corps"
    );
    assert_eq!(format!("{:?}", staged.actors.entity(key).unwrap()), before);
    assert_eq!(staged.rng.state, seed);
    assert!(queue.objects()[0].params.definition.is_none());
    assert!(aa_runtime::skirmish_start::require_replacement_drain().is_err());

    // Explicit source-derived property fixture on the actual actor owner:
    // Firefight's saved placeholder itself is not claimed to carry these effects.
    let property = source
        .resolved_definitions("Property")
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    let aa_rules::native::TypedValue::String { value: name, .. } =
        property.field("IDS", Precision::Double53).unwrap().unwrap()
    else {
        panic!("Property IDS")
    };
    let token = staged.definitions.intern("Property", &name).unwrap();
    let mut props =
        aa_body::properties::PropertiesState::new(aa_body::credit::Object(id.raw()), 2).unwrap();
    for (index, kind, origin, disabled) in [
        (1, 2, 0, true),
        (2, 2, id.raw(), false),
        (3, 1, 999, false),
        (4, 3, id.raw(), false),
        (5, 4, 999, false),
    ] {
        let handle = aa_body::credit::Object(index);
        let mut instance = aa_body::property_effects::Instance::new(
            handle,
            aa_body::credit::Object(token),
            kind,
            aa_body::F32::ZERO,
            Some(origin),
        );
        instance.disabled = disabled;
        staged.actors.property_allocations.instances_mut().insert(handle, instance);
        props.effects.push(handle);
    }
    staged
        .actors
        .object_mut(key)
        .unwrap()
        .components_mut()
        .unwrap()
        .insert(
            aa_actors::components::ComponentSlot::Properties,
            RuntimeComponent::Properties(Box::new(props)),
        );
    let before = format!("{:?}", staged.actors.property_allocations.instances());
    let context = Context {
        actors: &staged.actors,
        spatial: &StagedSpatial,
        definitions: &definitions,
        bank: &staged.definitions,
        source: &source,
        precision: Precision::Double53,
    };
    context
        .capture(&mut queue, key, Some(target), Kind::Actor, &mut warnings)
        .unwrap();
    assert_eq!(
        queue
            .actors()
            .last()
            .unwrap()
            .properties
            .iter()
            .map(|p| p.as_ref().unwrap().token)
            .collect::<Vec<_>>(),
        [token, token]
    );
    assert_eq!(warnings.len(), 1);
    assert_eq!(format!("{:?}", staged.actors.property_allocations.instances()), before);
    let RuntimeComponent::Properties(state) = staged
        .actors
        .component_mut(id, aa_actors::components::ComponentSlot::Properties)
        .unwrap()
        .unwrap()
    else {
        panic!()
    };
    state.effects.push(aa_body::credit::Object(123456));
    let count = queue.actors().len();
    let context = Context {
        actors: &staged.actors,
        spatial: &StagedSpatial,
        definitions: &definitions,
        bank: &staged.definitions,
        source: &source,
        precision: Precision::Double53,
    };
    let error = context
        .capture(&mut queue, key, Some(target), Kind::Actor, &mut warnings)
        .unwrap_err();
    assert!(error.contains("instance missing"));
    assert_eq!(
        warnings.len(),
        2,
        "earlier diagnostic survives later missing owner"
    );
    assert_eq!(queue.actors().len(), count);
    assert_eq!(staged.rng.state, seed);
}
