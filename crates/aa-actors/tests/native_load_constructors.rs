use aa_actors::{
    ActorId, IdentityRegistry,
    components::ALL_SLOTS,
    loading::{LoadKind, construct_shell},
};
use serde_json::{Value, json};
#[test]
fn original_complete_saved_constructors_preserve_distinct_load_initialization() {
    let d: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-actors/load-constructors-native.json"
    ))
    .unwrap();
    for row in d["cases"].as_array().unwrap() {
        let kind = match row["kind"].as_str().unwrap() {
            "Thing" => LoadKind::Thing,
            "Object" => LoadKind::Object,
            "Actor" => LoadKind::Actor,
            _ => unreachable!(),
        };
        let shell = construct_shell(kind, ());
        let native = &row["constructor"];
        assert_eq!(shell.thing.flags, native["flags"].as_u64().unwrap() as u32);
        assert_eq!(shell.thing.word_c, 0);
        assert_eq!(shell.thing.word_10, 0);
        assert!(shell.thing.word_1c.is_none());
        assert_eq!(native["word_1c_untouched"], 0xcdcdcdcdu32);
        assert_eq!(json!(shell.thing.position_bits), native["position_bits"]);
        assert_eq!(json!(shell.thing.facing_bits), native["facing_bits"]);
        assert_eq!(json!(shell.thing.name_utf16), native["name_utf16"]);
        if let Some(o) = &shell.object {
            assert_eq!(
                json!({"associated_handle":o.associated_handle,"scale_bits":o.scale_bits,"variant":o.variant as u32,"word_44":o.word_44,"word_48":o.word_48,"byte_4c":o.byte_4c,"words_50_to_5c":[o.words_50_to_54[0],o.words_50_to_54[1],o.memory.count() as u32,u32::from(o.memory.allocation_present())]}),
                native["object"]
            );
        }
        if let Some(a) = &shell.actor {
            assert!(ALL_SLOTS.iter().all(|s| a.components.get(*s).is_none()));
            // Constructor-null pointer words now have one typed owner. Only
            // this proven empty constructor is projected to raw nulls for the
            // historical native snapshot; live NodeIds are never pointers.
            assert!(a.actor_list_108.head().is_none());
            assert!(a.actor_list_108.tail().is_none());
            assert_eq!(a.actor_list_108.allocated_nodes(), 0);
            assert!(!a.words.contains_key(&0x108));
            assert!(!a.words.contains_key(&0x10c));
            assert!(a.overlap_list_110.head().is_none());
            assert!(a.overlap_list_110.tail().is_none());
            assert_eq!(a.overlap_list_110.allocated_nodes(), 0);
            assert!(!a.words.contains_key(&0x110));
            assert!(!a.words.contains_key(&0x114));
            let mut native_words = a.words.clone();
            native_words.insert(0x108, 0);
            native_words.insert(0x10c, 0);
            native_words.insert(0x110, 0);
            native_words.insert(0x114, 0);
            assert!(a.zones_148.is_empty());
            assert_eq!(a.zones_148.native_capacity(), 9);
            for offset in [0x148, 0x14c, 0x150] {
                assert!(!a.words.contains_key(&offset));
            }
            native_words.insert(0x148, 0);
            native_words.insert(0x14c, 0);
            native_words.insert(0x150, a.zones_148.native_capacity());
            assert_eq!(json!(native_words), native["actor"]["words"]);
            assert_eq!(
                json!([a.components.state.low, a.components.state.high]),
                native["actor"]["state"]
            );
            assert_eq!(json!(a.string_dc_utf16), native["actor"]["string_dc_utf16"]);
            assert_eq!(json!(a.string_e0_utf16), native["actor"]["string_e0_utf16"]);
        }
        let saved = ActorId::from_raw(row["saved_id"].as_u64().unwrap() as u32);
        let mut r = IdentityRegistry::new();
        r.seed_saved_generation(saved).unwrap();
        r.register_saved(saved, kind == LoadKind::Actor, shell)
            .unwrap();
        assert_eq!(
            r.len(),
            row["registry"]["active"].as_u64().unwrap() as usize
        );
        assert_eq!(
            r.highest_slot(),
            row["registry"]["highest_slot"].as_i64().unwrap() as i32
        );
    }
}
