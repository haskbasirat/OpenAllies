use aa_actors::{ActorId, IdentityRegistry, creation::*};
use aa_numeric::Precision;
use aa_simulation::NativeRng;
use serde_json::{Value, json};
#[test]
fn original_actor_ctor_then_object_identity_and_scale_preserve_rng_and_record_effects() {
    let d: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-actors/created-actors-native.json"
    ))
    .unwrap();
    assert_eq!(d["cases"].as_array().unwrap().len(), 81);
    for row in d["cases"].as_array().unwrap() {
        let c = &row["input"];
        let u = |s: &str| c[s].as_u64().unwrap() as u32;
        let mut record = ActorCreationRecord {
            base: CreationRecord {
                definition: ThingDefinition {
                    key: (),
                    flags: u("definition_flags"),
                },
                position_bits: serde_json::from_value(c["position_bits"].clone()).unwrap(),
                direction_bits: serde_json::from_value(c["direction_bits"].clone()).unwrap(),
                name_utf16: serde_json::from_value(c["name_utf16"].clone()).unwrap(),
                saved_id: ActorId::from_raw(u("saved_id")),
            },
            variant: c["variant"].as_i64().unwrap() as i32,
            owner_word: u("owner_word"),
            word_30: u("word_30"),
            request_nonblocking: false,
        };
        let definition = ObjectDefinition {
            variations_enabled: c["variations_enabled"].as_bool().unwrap(),
            variation_weights_bits: serde_json::from_value(c["weights_bits"].clone()).unwrap(),
            total_variation_weight_bits: u("total_weight_bits"),
            scale_min_bits: u("scale_min_bits"),
            scale_max_bits: u("scale_max_bits"),
        };
        let pc: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let table = DirectionTable::original(
            serde_json::from_value(c["startup_precision"].clone()).unwrap(),
        );
        let mut rng = NativeRng::new(u("rng_state"));
        let shell = construct_actor(
            &mut record,
            &definition,
            u("world_begin_bits"),
            &mut rng,
            table,
            pc,
        )
        .unwrap();
        let actor = shell.actor.as_ref().unwrap();
        // 4D6BB0 leaves +108/+10C, +110/+114 and +148/+14C/+150 as their
        // constructor values, which now have one typed owner each (as in
        // native_load_constructors.rs). Only these proven empty constructor
        // states are projected to the raw words of the native snapshot.
        assert!(actor.actor_list_108.head().is_none());
        assert!(actor.actor_list_108.tail().is_none());
        assert_eq!(actor.actor_list_108.allocated_nodes(), 0);
        assert!(actor.overlap_list_110.head().is_none());
        assert!(actor.overlap_list_110.tail().is_none());
        assert_eq!(actor.overlap_list_110.allocated_nodes(), 0);
        assert!(actor.zones_148.is_empty());
        let mut native_words = actor.words.clone();
        for offset in [0x108, 0x10c, 0x110, 0x114, 0x148, 0x14c, 0x150] {
            assert!(!native_words.contains_key(&offset), "{offset:#x} {c}");
            native_words.insert(offset, 0);
        }
        native_words.insert(0x14c, actor.zones_148.len() as u32);
        native_words.insert(0x150, actor.zones_148.native_capacity());
        let actual = json!({"flags":shell.thing.flags,"name_utf16":shell.thing.name_utf16,"created_at_bits":shell.thing.word_1c.unwrap(),"position_bits":shell.thing.position_bits,"facing_bits":shell.thing.facing_bits,"variant":shell.object.as_ref().unwrap().variant as u32,"actor_words":native_words,"components":vec![0u32;31],"state":[actor.components.state.low,actor.components.state.high]});
        assert_eq!(actual, row["constructor"], "{c}");
        assert_eq!(
            record.owner_word,
            row["record_owner_after"].as_u64().unwrap() as u32
        );
        assert_eq!(
            rng.state,
            row["rng_after_constructor"].as_u64().unwrap() as u32
        );
        let mut registry = IdentityRegistry::new();
        let id = initialize_object(
            &mut registry,
            record.base.saved_id,
            shell,
            &definition,
            &mut rng,
            pc,
        )
        .unwrap();
        assert_eq!(id.raw(), row["id"].as_u64().unwrap() as u32);
        assert_eq!(
            registry
                .get(id)
                .unwrap()
                .value
                .object
                .as_ref()
                .unwrap()
                .scale_bits,
            row["scale_bits"].as_u64().unwrap() as u32,
            "{c}"
        );
        assert_eq!(
            rng.state,
            row["rng_after_initialize"].as_u64().unwrap() as u32
        );
    }
}
