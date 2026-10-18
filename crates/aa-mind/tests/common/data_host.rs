//! Replay host for the SAIData loader oracles: answers every data-node getter
//! from a case's recorded answer tree and records the calls in the event
//! format of research/phase8-ai/native_data.py.
use aa_mind::{
    Error, Result,
    sai::{
        data::{DataNode, NodeId},
        ego::{EgoServices, ThingSource},
        profile::{DataManagers, Manager},
    },
    x87::binary_from_x87,
};
use aa_numeric::Binary;
use serde_json::{Value, json};

pub struct AnswerHost {
    pub answers: Value,
    pub events: Vec<Value>,
}

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

impl AnswerHost {
    pub fn new(answers: Value) -> Self {
        AnswerHost {
            answers,
            events: vec![],
        }
    }
    fn answer(&self, node: NodeId, getter: &str, key: &str) -> Value {
        self.answers["nodes"][node.to_string()][getter]
            .get(key.to_ascii_lowercase())
            .cloned()
            .unwrap_or_else(|| panic!("unanswered {getter} {key} on node {node}"))
    }
    fn getter(&mut self, op: &str, node: NodeId, name: &str) -> Result<Value> {
        let v = self.answer(node, op, name);
        self.events
            .push(json!({"op": op, "node": node, "name": name, "result": v}));
        if v.get("error").is_some() {
            return Err(Error::World(format!("{op} {name}: {}", v["error"])));
        }
        Ok(v)
    }
}

impl DataNode for AnswerHost {
    fn fixed(&mut self, node: NodeId, name: &str) -> Result<Binary> {
        let v = self.getter("fixed", node, name)?;
        binary_from_x87(
            v["significand"].as_u64().unwrap(),
            v["sign_exponent"].as_u64().unwrap() as u16,
        )
    }
    fn int(&mut self, node: NodeId, name: &str) -> Result<i32> {
        Ok(self.getter("int", node, name)?.as_i64().unwrap() as i32)
    }
    fn boolean(&mut self, node: NodeId, name: &str) -> Result<bool> {
        Ok(self.getter("bool", node, name)?.as_bool().unwrap())
    }
    fn string(&mut self, node: NodeId, name: &str) -> Result<String> {
        Ok(self
            .getter("string", node, name)?
            .as_str()
            .unwrap()
            .to_owned())
    }
    fn localized(&mut self, node: NodeId, name: &str) -> Result<String> {
        Ok(self
            .getter("localized", node, name)?
            .as_str()
            .unwrap()
            .to_owned())
    }
    fn count(&mut self, node: NodeId, name: &str) -> Result<i32> {
        Ok(self.getter("count", node, name)?.as_i64().unwrap() as i32)
    }
    fn item(&mut self, node: NodeId, name: &str, index: i32, count: i32) -> Result<NodeId> {
        let key = format!("{}#{index}", name.to_ascii_lowercase());
        let v = self.answer(node, "item", &key);
        self.events
            .push(json!({"op": "item", "node": node, "name": name,
            "index": index, "count": count, "result": v}));
        if v.get("error").is_some() {
            return Err(Error::World(format!("item {key}: {}", v["error"])));
        }
        Ok(u(&v))
    }
    fn section(&mut self, node: NodeId, name: &str, required: bool) -> Result<Option<NodeId>> {
        let v = self.answer(node, "section", name);
        self.events
            .push(json!({"op": "section", "node": node, "name": name,
            "required": u8::from(required), "result": v}));
        if v.get("error").is_some() {
            return Err(Error::World(format!("section {name}: {}", v["error"])));
        }
        match v.as_u64() {
            Some(id) => Ok(Some(id as u32)),
            None if required => Err(Error::World(format!("SubNode not found. ({name})"))),
            None => Ok(None),
        }
    }
    fn data_error(&mut self, format: &str, args: &[String]) -> Error {
        self.events
            .push(json!({"op": "data_error", "format": format, "args": args}));
        Error::World(format!("data error: {format}"))
    }
}

pub fn manager_key(manager: Manager) -> String {
    match manager {
        Manager::Other(offset) => format!("Other({offset:#x})"),
        other => format!("{other:?}"),
    }
}

impl DataManagers for AnswerHost {
    fn find(&mut self, manager: Manager, ids: &str) -> Result<Option<(u32, i32)>> {
        let kind = manager_key(manager);
        let v = self.answers["managers"][&kind]
            .get(ids.to_ascii_lowercase())
            .cloned()
            .unwrap_or_else(|| panic!("unanswered find {kind} {ids}"));
        self.events
            .push(json!({"op": "find", "manager": kind, "ids": ids, "result": v}));
        Ok(v.as_array()
            .map(|a| (u(&a[0]), a[1].as_i64().unwrap() as i32)))
    }
}

impl EgoServices for AnswerHost {
    fn thing(&mut self, source: ThingSource, ids: &str) -> Result<Option<u32>> {
        let source = format!("{source:?}");
        let v = self.answers["things"][&source]
            .get(ids.to_ascii_lowercase())
            .cloned()
            .unwrap_or_else(|| panic!("unanswered thing {source} {ids}"));
        self.events
            .push(json!({"op": "thing", "source": source, "ids": ids, "result": v}));
        Ok(v.as_u64().map(|h| h as u32))
    }
    fn property(&mut self, ids: &str) -> Result<Option<u32>> {
        let v = self.answers["properties"]
            .get(ids.to_ascii_lowercase())
            .cloned()
            .unwrap_or_else(|| panic!("unanswered property {ids}"));
        self.events
            .push(json!({"op": "property", "ids": ids, "result": v}));
        Ok(v.as_u64().map(|h| h as u32))
    }
    fn registry_count(&mut self, manager: Manager) -> Result<i32> {
        let kind = manager_key(manager);
        Ok(self.answers["counts"][&kind]
            .as_i64()
            .unwrap_or_else(|| panic!("unanswered registry size {kind}")) as i32)
    }
    fn ability_area(&mut self, ability: u32) -> Result<u32> {
        Ok(self.answers["areas"][ability.to_string()]
            .as_u64()
            .unwrap_or_else(|| panic!("unanswered area of {ability:#x}")) as u32)
    }
}

/// Canonical JSON (sorted keys, compact, UTF-8) as the Python producers
/// digest it (`json.dumps(sort_keys=True, separators=(',', ':'),
/// ensure_ascii=False)`).
pub fn canonical(v: &Value, out: &mut String) {
    match v {
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                canonical(x, out);
            }
            out.push(']');
        }
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            out.push('{');
            for (i, k) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).unwrap());
                out.push(':');
                canonical(&m[k], out);
            }
            out.push('}');
        }
        other => out.push_str(&serde_json::to_string(other).unwrap()),
    }
}

pub fn digest(v: &Value) -> String {
    let mut s = String::new();
    canonical(v, &mut s);
    aa_rules::sources::sha256(s.as_bytes())
}

/// Applies `[[path...], value]` answer patches (missing objects are created).
pub fn apply_patches(answers: &mut Value, patches: &Value) {
    for patch in patches.as_array().unwrap() {
        let path = patch[0].as_array().unwrap();
        let mut v = &mut *answers;
        for k in path {
            v = &mut v[k.as_str().unwrap()];
        }
        *v = patch[1].clone();
    }
}
