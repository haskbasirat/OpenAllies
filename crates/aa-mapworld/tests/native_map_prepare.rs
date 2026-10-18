//! Replay the original 4FDA70 boundary trace, not guessed replacement creation.
use aa_mapworld::map_prepare::{self, Error, RandomKind, Result, World};
use serde_json::{Value, json};
use std::collections::VecDeque;

struct Replay {
    events: VecDeque<Value>,
    label: String,
}
impl Replay {
    /// Values after the verified argument prefix are actual read/return values
    /// recorded by the native machine. Every expected boundary must be consumed.
    fn call(&mut self, prefix: Value) -> Vec<Value> {
        let actual = self
            .events
            .pop_front()
            .expect("unexpected call after native trace");
        let actual = actual.as_array().unwrap();
        let prefix = prefix.as_array().unwrap();
        assert!(
            actual.len() >= prefix.len(),
            "{}: {prefix:?} vs {actual:?}",
            self.label
        );
        assert_eq!(&actual[..prefix.len()], prefix, "{}", self.label);
        actual[prefix.len()..].to_vec()
    }
}
fn optional(value: &Value) -> Option<u32> {
    if value.is_null() {
        None
    } else {
        Some(value.as_u64().unwrap().try_into().unwrap())
    }
}
fn kind(value: RandomKind) -> &'static str {
    match value {
        RandomKind::Object => "object",
        RandomKind::Actor => "actor",
    }
}
impl World for Replay {
    type Object = u32;
    type Definition = u32;
    type Kingdom = u32;
    type Nation = u32;
    type Batch = ();

    fn begin_replacements(&mut self) -> Result<()> {
        assert!(self.call(json!(["begin"])).is_empty());
        Ok(())
    }
    fn live_count(&mut self) -> Result<i32> {
        Ok(self.call(json!(["count"]))[0].as_u64().unwrap() as u32 as i32)
    }
    fn next_object(&mut self, cursor: &mut u32) -> Result<Option<u32>> {
        let values = self.call(json!(["next", *cursor]));
        *cursor = values[0].as_u64().unwrap() as u32;
        Ok(optional(&values[1]))
    }
    fn definition(&mut self, object: u32) -> Result<u32> {
        Ok(self.call(json!(["definition", object]))[0]
            .as_u64()
            .unwrap() as u32)
    }
    fn category(&mut self, definition: u32) -> Result<u32> {
        Ok(self.call(json!(["category", definition]))[0]
            .as_u64()
            .unwrap() as u32)
    }
    fn owner(&mut self, object: u32) -> Result<Option<u32>> {
        Ok(optional(&self.call(json!(["owner", object]))[0]))
    }
    fn kingdom_word_190(&mut self, kingdom: u32) -> Result<u32> {
        Ok(self.call(json!(["kingdom_word", kingdom]))[0]
            .as_u64()
            .unwrap() as u32)
    }
    fn nation(&mut self, kingdom: u32) -> Result<Option<u32>> {
        Ok(optional(&self.call(json!(["nation", kingdom]))[0]))
    }
    fn substitution(&mut self, nation: Option<u32>, definition: u32) -> Result<Option<u32>> {
        Ok(optional(
            &self.call(json!(["substitute", nation, definition]))[0],
        ))
    }
    fn replacement_is_actor(&mut self, definition: u32) -> Result<bool> {
        Ok(self.call(json!(["is_actor", definition]))[0]
            .as_bool()
            .unwrap())
    }
    fn missing_substitution(&mut self, object: u32) -> Result<()> {
        assert!(self.call(json!(["missing", object])).is_empty());
        Ok(())
    }
    fn random_scope(&mut self, value: RandomKind) -> Result<()> {
        assert!(self.call(json!(["scope", kind(value)])).is_empty());
        Ok(())
    }
    fn rng_word(&mut self) -> Result<u32> {
        Ok(self.call(json!(["rng"]))[0].as_u64().unwrap() as u32)
    }
    fn log_rng(&mut self, channel: u8, seed: u32) -> Result<()> {
        assert!(self.call(json!(["log_rng", channel, seed])).is_empty());
        Ok(())
    }
    fn choose_random(&mut self, definition: u32, value: RandomKind) -> Result<Option<u32>> {
        Ok(optional(
            &self.call(json!(["choose", definition, kind(value)]))[0],
        ))
    }
    fn queue_replacement(
        &mut self,
        _: &mut (),
        object: u32,
        definition: Option<u32>,
        value: RandomKind,
    ) -> Result<()> {
        assert!(
            self.call(json!(["queue", object, definition, kind(value)]))
                .is_empty()
        );
        Ok(())
    }
    fn process_replacements(&mut self, _: &mut ()) -> Result<()> {
        assert!(self.call(json!(["process"])).is_empty());
        Ok(())
    }
    fn finish_replacements(&mut self, _: ()) -> Result<()> {
        assert!(self.call(json!(["finish"])).is_empty());
        Ok(())
    }
}

#[test]
fn placeholder_and_random_traversal_matches_native_4fda70() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-map-prepare/native-map-prepare.json");
    let fixture: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(fixture["function_bytes_identical_to_v14"], true);
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 96);
    let mut null_cases = 0;
    for row in rows {
        let mut world = Replay {
            label: format!("{} cw={}", row["input"]["name"], row["cw"]),
            events: row["trace"].as_array().unwrap().iter().cloned().collect(),
        };
        let result = map_prepare::resolve_placeholders_and_random_things(&mut world);
        if let Some(read) = row["outcome"]["null_owner"].as_u64() {
            assert!(
                matches!(result, Err(Error::NullOwner(got)) if u64::from(got) == read),
                "{}",
                world.label
            );
            null_cases += 1;
        } else {
            result.unwrap();
            assert_eq!(row["outcome"]["success"], true);
        }
        assert!(
            world.events.is_empty(),
            "{}: unconsumed native calls {:?}",
            world.label,
            world.events
        );
    }
    assert_eq!(null_cases, 6);
    assert_eq!(RandomKind::Object.table_offset(), 0x170);
    assert_eq!(RandomKind::Actor.table_offset(), 0x5c0);
    eprintln!(
        "96 native rows: 90 successful traversals, 6 explicit null-owner fault paths; PC24/53/64"
    );
}
