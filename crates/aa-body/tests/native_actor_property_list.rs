use aa_body::{
    Error, Result,
    actor_property_list::{self as core, State, World},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[derive(Clone)]
struct Node {
    value: Option<u32>,
    next: Option<Option<usize>>,
    freed: bool,
}
struct Host {
    case: Value,
    head: Vec<Option<usize>>,
    tail: Vec<Option<usize>>,
    source: Vec<Option<usize>>,
    nodes: Vec<Node>,
    trace: Vec<Value>,
    counts: BTreeMap<String, u64>,
}
fn ix(v: &Value) -> Option<usize> {
    v.as_u64().map(|v| v as usize)
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
impl Host {
    fn new(c: &Value) -> Self {
        let count = c["actors"].as_array().unwrap().len();
        let mut h = Self {
            case: c.clone(),
            head: vec![None; count],
            tail: vec![None; count],
            source: vec![None; count],
            nodes: vec![],
            trace: vec![],
            counts: BTreeMap::new(),
        };
        for i in 0..count {
            let (head, tail) = h.list(&c["actors"][i]["list"]);
            h.head[i] = head;
            h.tail[i] = tail;
        }
        for i in 0..count {
            let (head, _) = h.list(&c["actors"][i]["source"]);
            h.source[i] = head;
        }
        h
    }
    fn list(&mut self, v: &Value) -> (Option<usize>, Option<usize>) {
        let values = v.as_array().unwrap();
        let start = self.nodes.len();
        for (i, v) in values.iter().enumerate() {
            self.nodes.push(Node {
                value: Some(u(v)),
                next: Some((i + 1 < values.len()).then_some(start + i + 1)),
                freed: false,
            });
        }
        if values.is_empty() {
            (None, None)
        } else {
            (Some(start), Some(self.nodes.len() - 1))
        }
    }
    fn event(&mut self, k: &str, args: Vec<Value>) -> Result<()> {
        let mut e = vec![json!(k)];
        e.extend(args);
        self.trace.push(json!(e));
        let n = self.counts.entry(k.into()).or_default();
        *n += 1;
        let n = *n;
        if let Some(actions) = self.case["mutations"].as_array() {
            for a in actions.clone() {
                if a["event"] != k || a["occurrence"] != n {
                    continue;
                }
                let v = &a["value"];
                match a["kind"].as_str().unwrap() {
                    "value" => self.nodes[ix(&a["node"]).unwrap()].value = Some(u(v)),
                    "head" => self.head[ix(&a["actor"]).unwrap()] = ix(v),
                    "tail" => self.tail[ix(&a["actor"]).unwrap()] = ix(v),
                    "next" => self.nodes[ix(&a["node"]).unwrap()].next = Some(ix(v)),
                    "count" => {
                        let actor = ix(&a["actor"]).unwrap();
                        let n = u(v) as usize;
                        self.case["actors"][actor]["members"]
                            .as_array_mut()
                            .unwrap()
                            .truncate(n);
                    }
                    "member" => {
                        self.case["actors"][ix(&a["actor"]).unwrap()]["members"]
                            [ix(&a["slot"]).unwrap()] = v.clone()
                    }
                    k => panic!("{k}"),
                }
            }
        }
        if self.case["stop"]["event"] == k && self.case["stop"]["occurrence"] == n {
            return Err(Error::Callback(k.into()));
        }
        Ok(())
    }
    fn output(&self, result: Result<()>) -> Value {
        let outcome = match result {
            Ok(()) => "ok".into(),
            Err(Error::Callback(s)) => s,
            Err(e) => panic!("{e}"),
        };
        json!({"outcome":outcome,"trace":self.trace,"actors":self.head.iter().zip(&self.tail).map(|(h,t)|json!({"head":h,"tail":t})).collect::<Vec<_>>(),"nodes":self.nodes.iter().map(|n|json!({"value":n.value,"next":n.next.flatten(),"next_written":n.next.is_some(),"freed":n.freed})).collect::<Vec<_>>()})
    }
}
impl World for Host {
    type Actor = usize;
    type Node = usize;
    type Definition = usize;
    type Organization = usize;
    type Element = usize;
    type Value = u32;
    fn actor_head(&mut self, a: usize) -> Result<Option<usize>> {
        self.event("actor_head", vec![json!(a)])?;
        Ok(self.head[a])
    }
    fn actor_tail(&mut self, a: usize) -> Result<Option<usize>> {
        self.event("actor_tail", vec![json!(a)])?;
        Ok(self.tail[a])
    }
    fn store_actor_head(&mut self, a: usize, n: Option<usize>) -> Result<()> {
        self.event("store_actor_head", vec![json!(a), json!(n)])?;
        self.head[a] = n;
        Ok(())
    }
    fn store_actor_tail(&mut self, a: usize, n: Option<usize>) -> Result<()> {
        self.event("store_actor_tail", vec![json!(a), json!(n)])?;
        self.tail[a] = n;
        Ok(())
    }
    fn node_value(&mut self, n: usize) -> Result<u32> {
        self.event("node_value", vec![json!(n)])?;
        self.nodes[n]
            .value
            .ok_or_else(|| Error::Callback("unwritten node value".into()))
    }
    fn node_next(&mut self, n: usize) -> Result<Option<usize>> {
        self.event("node_next", vec![json!(n)])?;
        self.nodes[n]
            .next
            .ok_or_else(|| Error::Callback("unwritten node next".into()))
    }
    fn store_node_value(&mut self, n: usize, v: u32) -> Result<()> {
        self.event("store_node_value", vec![json!(n), json!(v)])?;
        self.nodes[n].value = Some(v);
        Ok(())
    }
    fn store_node_next(&mut self, n: usize, v: Option<usize>) -> Result<()> {
        self.event("store_node_next", vec![json!(n), json!(v)])?;
        self.nodes[n].next = Some(v);
        Ok(())
    }
    fn allocate_node(&mut self, a: usize) -> Result<usize> {
        self.event("allocate_node", vec![json!(a)])?;
        let n = self.nodes.len();
        self.nodes.push(Node {
            value: None,
            next: None,
            freed: false,
        });
        Ok(n)
    }
    fn free_node(&mut self, a: usize, n: usize) -> Result<()> {
        self.event("free_node", vec![json!(a), json!(n)])?;
        assert!(!self.nodes[n].freed);
        self.nodes[n].freed = true;
        Ok(())
    }
    fn has_source_component_a8(&mut self, a: usize) -> Result<bool> {
        self.event("has_source_component_a8", vec![json!(a)])?;
        Ok(self.case["actors"][a]["builder"].as_bool().unwrap())
    }
    fn definition(&mut self, a: usize) -> Result<usize> {
        self.event("definition", vec![json!(a)])?;
        Ok(a)
    }
    fn definition_flags(&mut self, d: usize) -> Result<u32> {
        self.event("definition_flags", vec![json!(d)])?;
        Ok(u(&self.case["actors"][d]["flags"]))
    }
    fn definition_head_504(&mut self, d: usize) -> Result<Option<usize>> {
        self.event("definition_head_504", vec![json!(d)])?;
        Ok(self.source[d])
    }
    fn organization(&mut self, a: usize) -> Result<Option<usize>> {
        self.event("organization", vec![json!(a)])?;
        Ok((!self.case["actors"][a]["members"].is_null()).then_some(a))
    }
    fn organization_member_count(&mut self, a: usize) -> Result<i32> {
        self.event("organization_member_count", vec![json!(a)])?;
        Ok(self.case["actors"][a]["members"].as_array().unwrap().len() as i32)
    }
    fn organization_member(&mut self, a: usize, index: i32) -> Result<Option<usize>> {
        self.event("organization_member", vec![json!(a), json!(index)])?;
        Ok(ix(&self.case["actors"][a]["members"][index as usize]))
    }
    fn element(&mut self, a: usize) -> Result<Option<usize>> {
        self.event("element", vec![json!(a)])?;
        Ok(self.case["actors"][a]["element"]
            .as_bool()
            .unwrap()
            .then_some(a))
    }
    fn element_parent(&mut self, a: usize) -> Result<Option<usize>> {
        self.event("element_parent", vec![json!(a)])?;
        Ok(ix(&self.case["actors"][a]["parent"]))
    }
}
#[test]
fn native_actor_list_rebuild_and_parent_all_precisions() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-actor-property-core/native-actor-property-core.json"
    ))
    .unwrap();
    let rows = doc["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 942);
    for row in rows {
        let c = &row["input"];
        let mut world = Host::new(c);
        let actor = ix(&c["actor"]).unwrap();
        let result = if c["operation"] == "refresh" {
            core::refresh(&mut world, actor)
        } else {
            core::refresh_parent(&mut world, actor)
        };
        assert_eq!(
            world.output(result),
            row["output"],
            "{} {}",
            c["name"],
            row["precision"]
        );
    }
}
#[test]
fn authoritative_list_keeps_unwritten_and_freed_identity_explicit() {
    let mut state = State::<String>::new();
    assert_eq!(
        (state.head(), state.tail(), state.allocated_nodes()),
        (None, None, 0)
    );
    let first = state.allocate().unwrap();
    assert!(state.value(first).is_err());
    assert!(state.next(first).is_err());
    state.set_value(first, "canonical-actor".into()).unwrap();
    assert_eq!(state.value(first).unwrap(), "canonical-actor");
    assert!(state.next(first).is_err());
    state.set_next(first, None).unwrap();
    assert_eq!(state.next(first).unwrap(), None);
    state.set_head(Some(first));
    state.set_tail(Some(first));
    state.free(first).unwrap();
    assert_eq!((state.head(), state.tail()), (Some(first), Some(first)));
    assert!(state.value(first).is_err());
    assert!(state.free(first).is_err());
    let second = state.allocate().unwrap();
    assert_ne!(first, second);
    state.set_head(None);
    state.set_tail(None);
}
