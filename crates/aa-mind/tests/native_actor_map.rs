//! Replays research/phase8-ai/native-actor-map.json: original x86 execution
//! of the SAI actor-by-region list upkeep 7A9940, 79E030, 7BD310, 7BD2A0,
//! 7B99F0, 7BC330, 7BC470 and 7ADFF0 on real native lists. Every recorded
//! callback with its arguments, every list's values (head first, appended
//! lists included) and every record +14 after the call must match.
use aa_mind::Result;
use aa_mind::sai::actor_map::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn document() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-ai/native-actor-map.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("json")
}
fn u(v: &Value) -> u32 {
    v.as_u64().expect("u32") as u32
}

struct World<'a> {
    c: &'a Value,
    keys: Vec<String>,
    events: Vec<Value>,
    lists: BTreeMap<String, Vec<u32>>,
    /// List pointer refs (Ref -> list key).
    list_refs: Vec<String>,
    /// Region tables: list keys (None: a null list pointer).
    tables: BTreeMap<(String, &'static str), Vec<Option<String>>>,
    records: BTreeMap<String, u32>,
}

fn table_name(table: Table) -> &'static str {
    if table == Table::At28 { "at28" } else { "at34" }
}

impl<'a> World<'a> {
    fn new(c: &'a Value) -> Self {
        let mut keys = vec![String::new()];
        for group in ["kingdoms", "actors", "owners", "regions"] {
            keys.extend(c[group].as_object().unwrap().keys().cloned());
        }
        keys.extend(c["watchers"].as_array().unwrap().iter().map(|w| w.as_str().unwrap().to_string()));
        let mut lists = BTreeMap::new();
        for (k, o) in c["owners"].as_object().unwrap() {
            lists.insert(format!("owner:{k}"), o["list"].as_array().unwrap().iter().map(u).collect());
        }
        let mut tables = BTreeMap::new();
        for (k, r) in c["regions"].as_object().unwrap() {
            for table in ["at28", "at34"] {
                let mut entries = Vec::new();
                for (i, values) in r[table].as_array().unwrap().iter().enumerate() {
                    if let Some(values) = values.as_array() {
                        let key = format!("{k}:{table}:{i}");
                        lists.insert(key.clone(), values.iter().map(u).collect());
                        entries.push(Some(key));
                    } else {
                        entries.push(None);
                    }
                }
                tables.insert((k.clone(), table), entries);
            }
        }
        let records = c["actors"]
            .as_object()
            .unwrap()
            .iter()
            .filter(|(_, a)| !a["record"].is_null())
            .map(|(k, _)| (k.clone(), 0x1234_5678))
            .collect();
        let list_refs = vec![String::new()];
        Self { c, keys, events: Vec::new(), lists, list_refs, tables, records }
    }
    fn id(&self, key: &str) -> Ref {
        self.keys.iter().position(|k| k == key).unwrap_or_else(|| panic!("{key}")) as Ref
    }
    fn opt(&self, v: &Value) -> Ref {
        v.as_str().map_or(0, |k| self.id(k))
    }
    fn key(&self, r: Ref) -> Value {
        if r == 0 { json!(0) } else { json!(self.keys[r as usize]) }
    }
    fn name(&self, r: Ref) -> String {
        self.keys[r as usize].clone()
    }
    fn list_key(&self, list: List) -> String {
        match list {
            List::Owner(o) => format!("owner:{}", self.name(o)),
            List::Region(r) => self.list_refs[r as usize].clone(),
        }
    }
}

impl ActorMapWorld for World<'_> {
    fn region_of(&mut self, actor: Ref) -> Result<Ref> {
        self.events.push(json!(["region_of", self.key(actor)]));
        Ok(self.opt(&self.c["region_of"][self.name(actor)]))
    }
    fn kingdom(&mut self, actor: Ref) -> Result<Ref> {
        self.events.push(json!(["kingdom", self.key(actor)]));
        Ok(self.opt(&self.c["actors"][self.name(actor)]["kingdom"]))
    }
    fn word_140(&mut self, actor: Ref) -> Result<u32> {
        Ok(u(&self.c["actors"][self.name(actor)]["word_140"]))
    }
    fn handle_of(&mut self, actor: Ref) -> Result<u32> {
        Ok(u(&self.c["actors"][self.name(actor)]["handle"]))
    }
    fn kingdom_index(&mut self, kingdom: Ref) -> Result<i32> {
        Ok(u(&self.c["kingdoms"][self.name(kingdom)]["index"]) as i32)
    }
    fn test_79f390(&mut self, actor: Ref) -> Result<bool> {
        self.events.push(json!(["test_79f390", self.key(actor)]));
        Ok(self.c["test_79f390"].as_bool().unwrap())
    }
    fn record(&mut self, actor: Ref) -> Result<Ref> {
        // The record is an opaque nonzero object; its owner is listed per actor.
        Ok(if self.c["actors"][self.name(actor)]["record"].is_null() { 0 } else { actor })
    }
    fn record_owner(&mut self, record: Ref) -> Result<Ref> {
        Ok(self.opt(&self.c["actors"][self.name(record)]["record"]))
    }
    fn table_count(&mut self, region: Ref, table: Table) -> Result<i32> {
        Ok(self.tables[&(self.name(region), table_name(table))].len() as i32)
    }
    fn table_list(&mut self, region: Ref, table: Table, index: i32) -> Result<Ref> {
        let entry = self.tables[&(self.name(region), table_name(table))][usize::try_from(index).unwrap()].clone();
        let Some(key) = entry else { return Ok(0) };
        self.list_refs.push(key);
        Ok((self.list_refs.len() - 1) as Ref)
    }
    fn watcher_count(&mut self, region: Ref) -> Result<i32> {
        Ok(self.c["regions"][self.name(region)]["watchers"].as_array().unwrap().len() as i32)
    }
    fn watcher(&mut self, region: Ref, index: i32) -> Result<Ref> {
        Ok(self.opt(&self.c["regions"][self.name(region)]["watchers"][usize::try_from(index).unwrap()]))
    }
    fn remove(&mut self, list: List, value: u32) -> Result<bool> {
        let k = self.list_key(list);
        let l = self.lists.get_mut(&k).unwrap();
        Ok(match l.iter().position(|&v| v == value) {
            Some(i) => {
                l.remove(i);
                true
            }
            None => false,
        })
    }
    fn push_front(&mut self, list: List, value: u32) -> Result<()> {
        let k = self.list_key(list);
        self.lists.get_mut(&k).unwrap().insert(0, value);
        Ok(())
    }
    fn watcher_update(&mut self, watcher: Ref, actor: Ref) -> Result<()> {
        self.events.push(json!(["watcher_update", self.key(watcher), self.key(actor)]));
        Ok(())
    }
    fn watcher_added(&mut self, watcher: Ref, actor: Ref) -> Result<()> {
        self.events.push(json!(["watcher_added", self.key(watcher), self.key(actor)]));
        Ok(())
    }
    fn set_record_14(&mut self, record: Ref, bits: u32) -> Result<()> {
        let k = self.name(record);
        *self.records.get_mut(&k).unwrap() = bits;
        Ok(())
    }
    fn append_list(&mut self, region: Ref, table: Table) -> Result<()> {
        let name = self.name(region);
        let entries = self.tables.get_mut(&(name.clone(), table_name(table))).unwrap();
        let key = format!("{name}:{}:{}", table_name(table), entries.len());
        entries.push(Some(key.clone()));
        self.lists.insert(key, Vec::new());
        Ok(())
    }
}

#[test]
fn actor_map_matches_native() {
    let d = document();
    for (k, n) in [("7A9940", 37), ("79E030", 29), ("7BD310", 33), ("7BD2A0", 38), ("7B99F0", 40), ("7ADFF0", 42)] {
        assert_eq!(d["coverage"][k], n, "every instruction of {k} ran");
    }
    // 7BC330 / 7BC470 leave nine instructions each unvisited: capacity-choice
    // branches of the vector growth, the null-actor handle after the +154
    // dereference and the allocation-failure path.
    assert_eq!(d["coverage"]["7BC330"], 107);
    assert_eq!(d["coverage"]["7BC470"], 98);
    let cases = d["cases"].as_array().unwrap();
    assert!(cases.len() >= 1000);
    let mut changed = 0;
    for case in cases {
        let (c, o) = (&case["input"], &case["output"]);
        let name = c["name"].as_str().unwrap();
        let mut w = World::new(c);
        let before = w.lists.clone();
        let a = &c["args"];
        let actor = w.opt(&a[0]);
        let this = w.opt(&c["this"]);
        let flag = |v: &Value| v.as_u64().unwrap() & 0xff != 0;
        match c["operation"].as_str().unwrap() {
            "remove_from_region" => remove_from_region(actor, w.opt(&a[1]), &mut w),
            "refresh" => refresh(actor, flag(&a[1]), &mut w),
            "region_move" => region_move(this, actor, flag(&a[1]), &mut w),
            "remove_28" => remove_28(this, actor, w.opt(&a[1]), &mut w),
            "remove_34" => remove_34(this, actor, w.opt(&a[1]), &mut w),
            "owner_list" => owner_list(this, actor, flag(&a[1]), &mut w),
            "add_28" => add(this, actor, w.opt(&a[1]), Table::At28, &mut w),
            "add_34" => add(this, actor, w.opt(&a[1]), Table::At34, &mut w),
            other => panic!("{other}"),
        }
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(Value::Array(w.events.clone()), o["events"], "{name} events");
        let lists: BTreeMap<String, Vec<u32>> = serde_json::from_value(o["lists"].clone()).unwrap();
        assert_eq!(w.lists, lists, "{name} lists");
        let records: BTreeMap<String, u32> = serde_json::from_value(o["records"].clone()).unwrap();
        assert_eq!(w.records, records, "{name} records");
        changed += usize::from(w.lists != before);
    }
    assert!(changed > 300, "{changed}");
}
