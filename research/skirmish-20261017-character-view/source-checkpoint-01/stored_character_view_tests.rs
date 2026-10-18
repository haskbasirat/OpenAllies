//! The same parent coordinator against explicit native child contracts.
use super::*;
use serde_json::{Value, json};
struct Fixture {
    input: Value,
    current: i32,
    pending: i32,
    events: Vec<Value>,
}
impl Fixture {
    fn reach(&mut self, label: &str, event: Value) -> Result<()> {
        self.events.push(event);
        if self.input["fail"] == label {
            Err(label.into())
        } else {
            Ok(())
        }
    }
    fn set(&self, index: i32) -> Result<Option<u32>> {
        let sets = self.input["sets"].as_array().unwrap();
        if sets.is_empty() {
            return Ok(None);
        }
        let selected = if index == -1 {
            sets.last().unwrap()
        } else {
            let selected = sets
                .get(usize::try_from(index).map_err(error)?)
                .ok_or("native set index outside explicit array")?;
            if selected.is_null() {
                sets.last().unwrap()
            } else {
                selected
            }
        };
        Ok(selected.as_u64().map(|v| v as u32))
    }
    fn aspect(&self, stance: i32) -> Result<i32> {
        if stance == -1 {
            Ok(self.input["normal"].as_i64().unwrap() as i32)
        } else {
            Ok(
                self.input["aspects"].as_array().unwrap()[usize::try_from(stance).map_err(error)?]
                    .as_i64()
                    .unwrap() as i32,
            )
        }
    }
}
impl World for Fixture {
    type Set = Option<u32>;
    type Animation = u32;
    fn requested_set(&mut self, stance: i32) -> Result<Self::Set> {
        self.set(self.aspect(stance)?)
    }
    fn current_set(&mut self) -> Result<Self::Set> {
        self.set(if self.current == -1 {
            self.input["variant"].as_i64().unwrap() as i32
        } else {
            self.aspect(self.current)?
        })
    }
    fn current_stance(&mut self) -> Result<i32> {
        Ok(self.current)
    }
    fn stance_count(&mut self) -> Result<i32> {
        Ok(self.input["properties"].as_array().unwrap().len() as i32)
    }
    fn invalid_stance(&mut self, _: i32, current: bool) -> Result<()> {
        self.reach("diagnostic", json!(["diagnostic", current]))
    }
    fn stance_property(&mut self, stance: i32) -> Result<Option<PropertyRef>> {
        Ok(
            self.input["properties"][usize::try_from(stance).map_err(error)?]
                .as_u64()
                .map(|v| PropertyRef::from_handle(v as u32).unwrap()),
        )
    }
    fn has_properties(&mut self) -> Result<bool> {
        Ok(self.input["has_properties"].as_bool().unwrap())
    }
    fn remove_property(&mut self, p: PropertyRef) -> Result<()> {
        self.reach("remove", json!(["remove", p.handle().unwrap()]))
    }
    fn applicable(&mut self, p: PropertyRef) -> Result<bool> {
        self.reach("applicable", json!(["applicable", p.handle().unwrap()]))?;
        Ok(self.input["applicable"].as_bool().unwrap())
    }
    fn add_property(&mut self, p: PropertyRef) -> Result<()> {
        self.reach("add", json!(["add", p.handle().unwrap(), 9]))
    }
    fn store_stance(&mut self, stance: i32) -> Result<()> {
        self.current = stance;
        Ok(())
    }
    fn store_pending(&mut self, stance: i32) -> Result<()> {
        self.pending = stance;
        Ok(())
    }
    fn current_animation(&mut self) -> Result<u32> {
        Ok(0)
    }
    fn animation_allows_transition(&mut self, _: u32) -> Result<bool> {
        Ok(matches!(self.input["kind"].as_u64().unwrap(), 0 | 2))
    }
    fn play(&mut self, h: u32) -> Result<()> {
        self.reach("play", json!(["play", h]))
    }
}
#[test]
fn original_v14_character_view_parent_retains_pending_and_property_failure_prefixes() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-character-view/native-character-view.json"
    ))
    .unwrap();
    assert_eq!(doc["rows"].as_array().unwrap().len(), 471);
    for row in doc["rows"].as_array().unwrap() {
        let input = &row["input"];
        let mut world = Fixture {
            input: input.clone(),
            current: input["current"].as_i64().unwrap() as i32,
            pending: input["pending"].as_i64().unwrap() as i32,
            events: Vec::new(),
        };
        let next = input["next"].as_i64().unwrap() as i32;
        let result = if input["mode"] == "change" {
            change(&mut world, next)
        } else {
            apply_stance(&mut world, next)
        };
        let (outcome, result) = match result {
            Ok(v) => ("ok".to_owned(), Some(v)),
            Err(e) => (e, None),
        };
        let actual = json!({"outcome":outcome,"result":result,"current":world.current as u32,"pending":world.pending as u32,"events":world.events});
        assert_eq!(actual, row["output"], "{} / {}", input, row["precision"]);
    }
}
