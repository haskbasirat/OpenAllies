//! Records the data-node answers the SAIData loaders read from the SHIPPED
//! data (original Data.rwd + accepted v14 overlays, aa-rules native load),
//! for the native oracle research/phase8-ai/native_saidata.py.
//!
//! cargo run -p aa-mind --release --locked --offline --example dump_sai_inputs -- <profiles.json> <egos.json>
//! (from rust-rebuild; reads ../Data.rwd and
//! ../reconstruction/native-game/runtime-expanded-v14).
use aa_mind::{
    Error, Result,
    sai::{
        data::{DataNode, NodeId},
        ego::{self, Ego, EgoServices, ThingSource},
        profile::{self, DataManagers, Manager, SaiProfile},
        rules_data::RulesData,
    },
};
use aa_numeric::{Binary, Precision};
use aa_rules::{
    actors::ActorCatalog,
    native::TypedValue,
    sources::{SourceLimits, V14Sources},
};
use serde_json::{Map, Value, json};
use std::path::PathBuf;

/// Records every answer by node, getter and lower-case name.
struct Recording<'a, 'b> {
    inner: &'b mut RulesData<'a>,
    nodes: Map<String, Value>,
    managers: Map<String, Value>,
    things: Map<String, Value>,
    properties: Map<String, Value>,
    counts: Map<String, Value>,
    areas: Map<String, Value>,
}
impl Recording<'_, '_> {
    fn put(&mut self, node: NodeId, getter: &str, name: &str, value: Value) {
        let n = self
            .nodes
            .entry(node.to_string())
            .or_insert_with(|| json!({}));
        let g = n
            .as_object_mut()
            .unwrap()
            .entry(getter.to_owned())
            .or_insert_with(|| json!({}));
        g[name.to_ascii_lowercase()] = value;
    }
    fn record<T>(
        &mut self,
        node: NodeId,
        getter: &str,
        name: &str,
        result: Result<T>,
        show: impl Fn(&T) -> Value,
    ) -> Result<T> {
        let value = match &result {
            Ok(v) => show(v),
            Err(e) => json!({"error": e.to_string()}),
        };
        self.put(node, getter, name, value);
        result
    }
}
fn x87(v: &Binary) -> Value {
    match v.to_x87_extended_bits() {
        Some(bits) => json!({"significand": (bits as u64), "sign_exponent": ((bits >> 64) as u16)}),
        None => json!({"unrepresentable": true}),
    }
}
impl DataNode for Recording<'_, '_> {
    fn fixed(&mut self, node: NodeId, name: &str) -> Result<Binary> {
        let r = self.inner.fixed(node, name);
        self.record(node, "fixed", name, r, x87)
    }
    fn int(&mut self, node: NodeId, name: &str) -> Result<i32> {
        let r = self.inner.int(node, name);
        self.record(node, "int", name, r, |v| json!(v))
    }
    fn boolean(&mut self, node: NodeId, name: &str) -> Result<bool> {
        let r = self.inner.boolean(node, name);
        self.record(node, "bool", name, r, |v| json!(v))
    }
    fn string(&mut self, node: NodeId, name: &str) -> Result<String> {
        let r = self.inner.string(node, name);
        self.record(node, "string", name, r, |v| json!(v))
    }
    fn localized(&mut self, node: NodeId, name: &str) -> Result<String> {
        let r = self.inner.localized(node, name);
        self.record(node, "localized", name, r, |v| json!(v))
    }
    fn count(&mut self, node: NodeId, name: &str) -> Result<i32> {
        let r = self.inner.count(node, name);
        self.record(node, "count", name, r, |v| json!(v))
    }
    fn item(&mut self, node: NodeId, name: &str, index: i32, count: i32) -> Result<NodeId> {
        let r = self.inner.item(node, name, index, count);
        let key = format!("{}#{index}", name.to_ascii_lowercase());
        self.record(node, "item", &key, r, |v| json!(v))
    }
    fn section(&mut self, node: NodeId, name: &str, required: bool) -> Result<Option<NodeId>> {
        let r = self.inner.section(node, name, required);
        self.record(node, "section", name, r, |v| json!(v))
    }
    fn data_error(&mut self, format: &str, args: &[String]) -> Error {
        self.inner.data_error(format, args)
    }
}
impl DataManagers for Recording<'_, '_> {
    fn find(&mut self, manager: Manager, ids: &str) -> Result<Option<(u32, i32)>> {
        let r = self.inner.find(manager, ids)?;
        let m = self
            .managers
            .entry(format!("{manager:?}"))
            .or_insert_with(|| json!({}));
        m[ids.to_ascii_lowercase()] = json!(r);
        Ok(r)
    }
}

impl EgoServices for Recording<'_, '_> {
    fn thing(&mut self, source: ThingSource, ids: &str) -> Result<Option<u32>> {
        let r = self.inner.thing(source, ids)?;
        let m = self
            .things
            .entry(format!("{source:?}"))
            .or_insert_with(|| json!({}));
        m[ids.to_ascii_lowercase()] = json!(r);
        Ok(r)
    }
    fn property(&mut self, ids: &str) -> Result<Option<u32>> {
        let r = self.inner.property(ids)?;
        self.properties.insert(ids.to_ascii_lowercase(), json!(r));
        Ok(r)
    }
    fn registry_count(&mut self, manager: Manager) -> Result<i32> {
        let r = self.inner.registry_count(manager)?;
        self.counts.insert(format!("{manager:?}"), json!(r));
        Ok(r)
    }
    fn ability_area(&mut self, ability: u32) -> Result<u32> {
        let r = self.inner.ability_area(ability)?;
        self.areas.insert(ability.to_string(), json!(r));
        Ok(r)
    }
}

fn recording<'a, 'b>(inner: &'b mut RulesData<'a>) -> Recording<'a, 'b> {
    Recording {
        inner,
        nodes: Map::new(),
        managers: Map::new(),
        things: Map::new(),
        properties: Map::new(),
        counts: Map::new(),
        areas: Map::new(),
    }
}

fn main() {
    let out = std::env::args().nth(1).expect("profile output path");
    let ego_out = std::env::args().nth(2).expect("ego output path");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut sources = V14Sources::open(
        root.join("../Data.rwd"),
        root.join("../reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .expect("open original data");
    let catalog = ActorCatalog::load_v14(&mut sources).expect("native data load");
    let p = Precision::Double53;
    let mut profiles = Vec::new();
    for node in catalog.resolved_definitions("SAI").expect("SAI roots") {
        let ids = match node.field("IDS", p) {
            Ok(Some(TypedValue::String { value, .. })) => value,
            other => panic!("SAI root without IDS: {other:?}"),
        };
        let mut rules = RulesData::new(&catalog, p);
        let root_id = rules.root(node);
        let mut rec = recording(&mut rules);
        let mut sai = SaiProfile::new(&ids);
        let result = profile::load(&mut rec, root_id, &mut sai, p);
        profiles.push(json!({
            "ids": ids,
            "root": root_id,
            "nodes": rec.nodes,
            "managers": rec.managers,
            "rust_result": match &result { Ok(()) => json!("ok"), Err(e) => json!(e.to_string()) },
            "rust_profile": format!("{sai:?}"),
        }));
    }
    let mut egos = Vec::new();
    for node in catalog.resolved_definitions("Ego").expect("Ego roots") {
        let ids = match node.field("IDS", p) {
            Ok(Some(TypedValue::String { value, .. })) => value,
            other => panic!("Ego root without IDS: {other:?}"),
        };
        let mut rules = RulesData::new(&catalog, p);
        let root_id = rules.root(node);
        let mut rec = recording(&mut rules);
        let mut e = Ego::new(&ids);
        let result = ego::load(&mut rec, root_id, &mut e, p);
        egos.push(json!({
            "ids": ids,
            "root": root_id,
            "nodes": rec.nodes,
            "managers": rec.managers,
            "things": rec.things,
            "properties": rec.properties,
            "counts": rec.counts,
            "areas": rec.areas,
            "rust_result": match &result { Ok(()) => json!("ok"), Err(e) => json!(e.to_string()) },
        }));
    }
    let ego_doc = json!({
        "schema": 1,
        "method": "aa_mind::sai::rules_data over aa_rules::actors::ActorCatalog::load_v14 (original Data.rwd + v14 overlays); answers recorded while running the Rust ego loader (aa_mind::sai::ego::load, port of 4B0560); handles are aa_mind::sai::rules_data registry handles",
        "egos": egos,
    });
    let doc = json!({
        "schema": 1,
        "method": "aa_mind::sai::rules_data over aa_rules::actors::ActorCatalog::load_v14 (original Data.rwd + v14 overlays); answers recorded while running the Rust SAIData loaders",
        "profiles": profiles,
    });
    std::fs::write(&out, serde_json::to_string_pretty(&doc).unwrap() + "\n").expect("write");
    std::fs::write(
        &ego_out,
        serde_json::to_string(&ego_doc).unwrap()
            + "
",
    )
    .expect("write egos");
    let failed: Vec<_> = ego_doc["egos"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["rust_result"] != "ok")
        .map(|e| format!("{}: {}", e["ids"], e["rust_result"]))
        .collect();
    println!(
        "{} SAI profiles, {} egos ({} failed)",
        doc["profiles"].as_array().unwrap().len(),
        ego_doc["egos"].as_array().unwrap().len(),
        failed.len()
    );
    for f in failed {
        println!("  {f}");
    }
}
