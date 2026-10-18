use aa_body::F32;
use aa_combat::{
    combat_mean_coordinator::{self as coordinator, Field, World},
    Error, Result,
};
use aa_numeric::{Binary, Precision};
use serde_json::{json, Value};
use std::collections::BTreeMap;

struct Owner {
    input: Value,
    means: [u32; 8],
    guard: u8,
    trace: Vec<Value>,
    calls: BTreeMap<String, u32>,
    next: Vec<Option<u32>>,
    ability_refs: BTreeMap<u32, u32>,
    slots: BTreeMap<u32, u32>,
}
fn bits(value: &Value) -> u32 {
    value.as_u64().unwrap() as u32
}
impl Owner {
    fn new(input: Value) -> Self {
        let count = input["things"].as_array().unwrap().len();
        let mut ability_refs = BTreeMap::new();
        for (i, t) in input["things"].as_array().unwrap().iter().enumerate() {
            for j in 0..t["abilities"].as_array().unwrap().len() {
                ability_refs.insert(
                    1000 + i as u32 * 16 + j as u32,
                    2000 + i as u32 * 16 + j as u32,
                );
            }
        }
        Self {
            means: std::array::from_fn(|i| bits(&input["initial_means"][i])),
            guard: bits(&input["guard"]) as u8,
            next: (0..count)
                .map(|i| {
                    if i + 1 < count {
                        Some(i as u32 + 1)
                    } else {
                        None
                    }
                })
                .collect(),
            input,
            trace: Vec::new(),
            calls: BTreeMap::new(),
            slots: ability_refs.keys().map(|&key| (key, key)).collect(),
            ability_refs,
        }
    }
    fn event(&mut self, event: Value, key: Option<String>) -> Result<Option<u64>> {
        self.trace.push(event);
        let Some(key) = key else { return Ok(None) };
        let nth = self.calls.entry(key.clone()).or_default();
        *nth += 1;
        let nth = *nth;
        let action = self.input["actions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["key"] == key && a["nth"] == nth)
            .cloned()
            .unwrap_or(json!({}));
        if let Some(changes) = action["set"].as_object() {
            for (field, value) in changes {
                let (kind, ident) = field.split_once(':').unwrap();
                let ident = ident.parse::<usize>().unwrap();
                match kind {
                    "mean" => self.means[(ident - 0x74) / 4] = bits(value),
                    "flags" => self.input["things"][ident]["flags"] = value.clone(),
                    "reload" => {
                        self.input["things"][(ident - 1000) / 16]["abilities"]
                            [(ident - 1000) % 16]["reload"] = value.clone()
                    }
                    "count" => self.input["things"][ident]["ability_count"] = value.clone(),
                    "next" => {
                        self.next[ident] = value
                            .as_u64()
                            .filter(|&v| v < self.next.len() as u64)
                            .map(|v| v as u32)
                    }
                    "ability_ref" => {
                        self.ability_refs.insert(ident as u32, bits(value));
                    }
                    "slot" => {
                        self.slots.insert(ident as u32, bits(value));
                    }
                    _ => panic!("unsupported action {kind}"),
                }
            }
        }
        if let Some(error) = action["error"].as_str() {
            return Err(Error::World(error.to_owned()));
        }
        Ok(action["return_f64"].as_u64())
    }
    fn getter(&mut self, name: &str, ident: u32) -> Result<Binary> {
        if let Some(value) = self.event(
            json!(["get", name, ident]),
            Some(format!("get:{name}:{ident}")),
        )? {
            return Ok(Binary::from_f64_bits(value)?);
        }
        let value = match name {
            "extent" | "defense" | "movement" | "health" => {
                bits(&self.input["things"][ident as usize][name])
            }
            "power" => bits(
                &self.input["things"][((ident - 2000) / 16) as usize]["abilities"]
                    [((ident - 2000) % 16) as usize]["power"],
            ),
            "animation" | "recharge" => self.input["things"][((ident - 1000) / 16) as usize]
                ["abilities"][((ident - 1000) % 16) as usize][name]
                .as_u64()
                .unwrap_or(0) as u32,
            _ => panic!("getter {name}"),
        };
        Ok(Binary::from_f32_bits(value)?)
    }
}
impl World for Owner {
    fn first_link(&mut self) -> Result<Option<u32>> {
        let value = if self.next.is_empty() { None } else { Some(0) };
        self.event(json!(["first", value]), None)?;
        Ok(value)
    }
    fn next_link(&mut self, link: u32) -> Result<Option<u32>> {
        let value = self.next[link as usize];
        self.event(json!(["next", link, value]), None)?;
        Ok(value)
    }
    fn link_definition(&mut self, link: u32) -> Result<u32> {
        self.event(json!(["definition", link, link]), None)?;
        Ok(link)
    }
    fn flags_174(&mut self, definition: u32) -> Result<u32> {
        let value = bits(&self.input["things"][definition as usize]["flags"]);
        self.event(json!(["flags", definition, value]), None)?;
        Ok(value)
    }
    fn extent(&mut self, definition: u32) -> Result<Binary> {
        self.getter("extent", definition)
    }
    fn ability_count(&mut self, definition: u32) -> Result<i32> {
        let t = &self.input["things"][definition as usize];
        let value = t["ability_count"]
            .as_i64()
            .unwrap_or(t["abilities"].as_array().unwrap().len() as i64) as i32;
        self.event(json!(["count", definition, value]), None)?;
        Ok(value)
    }
    fn ability_at(&mut self, definition: u32, index: i32) -> Result<u32> {
        let local = self.slots[&(1000 + definition * 16 + index as u32)];
        self.event(json!(["ability_at", definition, index, local]), None)?;
        Ok(local)
    }
    fn ability_definition(&mut self, local: u32) -> Result<u32> {
        let ability = self.ability_refs[&local];
        self.event(json!(["ability_definition", local, ability]), None)?;
        Ok(ability)
    }
    fn base_power_64(&mut self, ability: u32) -> Result<F32> {
        let value = bits(
            &self.input["things"][((ability - 2000) / 16) as usize]["abilities"]
                [((ability - 2000) % 16) as usize]["power"],
        );
        self.event(
            json!(["base_power", ability, value]),
            Some(format!("base_power:{ability}")),
        )?;
        Ok(F32(value))
    }
    fn power(&mut self, ability: u32) -> Result<Binary> {
        self.getter("power", ability)
    }
    fn reload_04(&mut self, local: u32) -> Result<F32> {
        let value = bits(
            &self.input["things"][((local - 1000) / 16) as usize]["abilities"]
                [((local - 1000) % 16) as usize]["reload"],
        );
        self.event(json!(["reload", local, value]), None)?;
        Ok(F32(value))
    }
    fn first_animation_length(&mut self, local: u32) -> Result<Binary> {
        self.getter("animation", local)
    }
    fn recharge(&mut self, local: u32) -> Result<Binary> {
        self.getter("recharge", local)
    }
    fn defense(&mut self, definition: u32) -> Result<Binary> {
        self.getter("defense", definition)
    }
    fn movement(&mut self, definition: u32) -> Result<Binary> {
        self.getter("movement", definition)
    }
    fn health(&mut self, definition: u32) -> Result<Binary> {
        self.getter("health", definition)
    }
    fn discard_fraction(&mut self) -> Result<F32> {
        let value = bits(&self.input["discard"]);
        self.event(json!(["discard", value]), None)?;
        Ok(F32(value))
    }
    fn read_mean(&mut self, field: Field) -> Result<F32> {
        let value = self.means[(field as usize - 0x74) / 4];
        self.event(json!(["read", field as u32, value]), None)?;
        Ok(F32(value))
    }
    fn write_mean(&mut self, field: Field, value: F32) -> Result<()> {
        self.event(json!(["write", field as u32, value.0]), None)?;
        self.means[(field as usize - 0x74) / 4] = value.0;
        Ok(())
    }
    fn assertion_guard(&mut self) -> Result<u8> {
        self.event(json!(["guard", self.guard]), None)?;
        Ok(self.guard)
    }
    fn assertion(&mut self) -> Result<u8> {
        self.event(json!(["assert"]), Some("assert".to_owned()))?;
        Ok(bits(&self.input["assert_result"]) as u8)
    }
    fn write_assertion_guard(&mut self, value: u8) -> Result<()> {
        self.event(json!(["guard_write", value]), None)?;
        self.guard = value;
        Ok(())
    }
    fn log_mean(&mut self, field: Field, line: u32, value: u64) -> Result<()> {
        self.event(
            json!(["log", field as u32, line, format!("{value:016x}")]),
            Some(format!("log:{}", field as u32)),
        )?;
        Ok(())
    }
}
fn fixture() -> Value {
    serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/skirmish-20261016-combat-means/native-means.json"),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn native_producer_preserves_shared_reads_stores_callbacks_and_failure_prefixes() {
    let fixture = fixture();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 210);
    for row in rows {
        let pc = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => panic!("PC"),
        };
        let mut owner = Owner::new(row["input"].clone());
        let result = coordinator::compute(&mut owner, pc);
        assert_eq!(
            result.is_err(),
            !row["stopped"].is_null(),
            "{} {} {}: {result:?}",
            row["image"],
            row["precision"],
            row["input"]["name"]
        );
        assert_eq!(
            json!(owner.trace),
            row["trace"],
            "{} {} {}",
            row["image"],
            row["precision"],
            row["input"]["name"]
        );
        assert_eq!(json!(owner.means), row["means"]);
        assert_eq!(json!(owner.guard), row["guard"]);
    }
}
#[test]
fn real_registered_subset_retains_bound_input_provenance_and_native_filtered_order() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-combat-value-source/actual-source-inputs.json");
    let source: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let fixture = fixture();
    let mut checked = 0;
    for row in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["input"]["name"] == "registered-source-subset")
    {
        let selected = source["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["precision"] == row["precision"])
            .collect::<Vec<_>>();
        let ordered = source["actor_manager_order"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|id| selected.iter().copied().find(|r| r["id"] == *id))
            .collect::<Vec<_>>();
        assert_eq!(ordered.len(), 10);
        for (index, actual) in ordered.iter().enumerate() {
            let bound = &row["input"]["things"][index];
            let provenance = &row["input"]["source_subset"]["definitions"][index];
            assert_eq!(provenance["id"], actual["id"]);
            assert_eq!(provenance["source"], actual["source"]);
            assert_eq!(bound["flags"], actual["flags_174"]);
            assert_eq!(
                bound["defense"].as_u64().unwrap(),
                actual["body"][0].as_u64().unwrap_or(0)
            );
            assert_eq!(
                bound["health"].as_u64().unwrap(),
                actual["body"][1].as_u64().unwrap_or(0)
            );
            assert_eq!(
                bound["movement"].as_u64().unwrap(),
                actual["speed_228"].as_u64().unwrap_or(0)
            );
            for (a, b) in bound["abilities"]
                .as_array()
                .unwrap()
                .iter()
                .zip(actual["abilities"].as_array().unwrap())
            {
                for (new, old) in [
                    ("power", "power_64"),
                    ("reload", "value_04"),
                    ("recharge", "recharge_18"),
                    ("animation", "first_entry_10"),
                ] {
                    assert_eq!(a[new], b[old]);
                }
            }
        }
        checked += 1;
    }
    assert_eq!(checked, 6);
}
