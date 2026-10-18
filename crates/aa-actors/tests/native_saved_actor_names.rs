//! Replays research/phase8-saved-components/native-actor-names.json: the
//! original GActor::Load tail 4D971B..4D99C2 (required object, MapString
//! validation, numbered names; CRT _vswprintf/_swscanf/wcscmp executed) and
//! the Kingdom numbered-name counter 4E4980 under Unicorn.
use aa_actors::{
    actor_names::{self as names, NameWorld, NumberedNameCounters, SavedName},
    organization_state::{Arg, LoadError, Message, Result},
};
use serde_json::{Value, json};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../research/phase8-saved-components/native-actor-names.json"
);
const KINGDOM: u32 = 0x4b00;

struct World<'a> {
    c: &'a Value,
    calls: Vec<Value>,
}
fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
impl NameWorld for World<'_> {
    fn gtid(&self) -> Result<u32> {
        Ok(self.c["gtid"].as_u64().unwrap() as u32)
    }
    fn definition_name(&self) -> Result<String> {
        Ok(self.c["def_name"].as_str().unwrap().to_owned())
    }
    fn display_name(&self) -> Result<Vec<u16>> {
        Ok(units(self.c["display"].as_str().unwrap()))
    }
    fn numbered(&self) -> Result<bool> {
        Ok(self.c["numbered"].as_u64().unwrap() != 0)
    }
    fn gtid_live(&self, id: u32) -> Result<bool> {
        Ok(self.c["gtids"].get(id.to_string()).is_some_and(|v| v == "live"))
    }
    fn localize(&mut self, key: &str) -> Result<Option<Vec<u16>>> {
        let text = self.c["localization"].get(key).map(|v| units(v.as_str().unwrap()));
        self.calls.push(json!({"seam": "4B9F40", "key": key, "found": text.is_some()}));
        Ok(text)
    }
    fn strings_from_stream(&mut self) -> Result<bool> {
        let r = self.c["strings_from_stream"].as_bool().unwrap();
        self.calls.push(json!({"seam": "59E520", "result": r}));
        Ok(r)
    }
    fn map_string_exists(&mut self, key: &[u16]) -> Result<bool> {
        let key = String::from_utf16(key).unwrap();
        let found = self.c["map_strings"].as_array().unwrap().iter().any(|k| *k == key);
        self.calls.push(json!({"seam": "4BAA20", "key": key, "create": 0, "found": found}));
        Ok(found)
    }
    fn warn(&mut self, m: Message) -> Result<()> {
        let args: Vec<Value> = m
            .args
            .iter()
            .map(|a| match a {
                Arg::Text(t) => json!(t),
                Arg::Int(v) => json!(v),
            })
            .collect();
        self.calls.push(json!({"seam": "5A1040", "format": m.format, "args": args}));
        Ok(())
    }
    fn owner_kingdom(&mut self) -> Result<Option<u32>> {
        self.calls.push(json!({"seam": "owner+108"}));
        Ok(self.c["kingdom"].as_bool().unwrap().then_some(KINGDOM))
    }
    fn owner_word_e8(&self) -> Result<u32> {
        Ok(if self.c["kingdom"].as_bool().unwrap() { KINGDOM } else { 0 })
    }
    fn assign_number(&mut self, owner_e8: u32) -> Result<u32> {
        assert_eq!(owner_e8, KINGDOM);
        let r = self.c["assign"].as_u64().unwrap() as u32;
        self.calls.push(json!({"seam": "4E4980", "this": "kingdom", "result": r}));
        Ok(r)
    }
}

#[test]
fn actor_name_tail_matches_original() {
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    for case in cases {
        let c = &case["input"];
        let native = &case["native"];
        let name = c["name"].as_str().unwrap();
        let mut key = units(c["key"].as_str().unwrap());
        let mut text = units(c["text"].as_str().unwrap());
        let mut number = c["number"].as_i64().unwrap() as u32;
        let mut w = World { c, calls: Vec::new() };
        let result = names::finish(
            SavedName {
                required_158: c["required_158"].as_u64().unwrap() as u32,
                key: &mut key,
                text: &mut text,
                number: &mut number,
            },
            c["version"].as_u64().unwrap() as u32,
            &mut w,
        );
        match (&native["outcome"], result) {
            (Value::Null, Ok(())) => {}
            (o, Err(LoadError::Warning(m))) if !o.is_null() => {
                assert_eq!(o["kind"], "Warning");
                assert_eq!(o["format"], m.format, "{name}");
                let args: Vec<Value> = m
                    .args
                    .iter()
                    .map(|a| match a {
                        Arg::Text(t) => json!(t),
                        Arg::Int(v) => json!(v),
                    })
                    .collect();
                assert_eq!(o["args"], json!(args), "{name}");
            }
            (o, r) => panic!("{name}: native {o} rust {r:?}"),
        }
        assert_eq!(w.calls, native["calls"].as_array().unwrap().clone(), "{name}");
        assert_eq!(String::from_utf16(&key).unwrap(), native["key"].as_str().unwrap(), "{name}");
        assert_eq!(String::from_utf16(&text).unwrap(), native["text"].as_str().unwrap(), "{name}");
        assert_eq!(u64::from(number), native["number"].as_u64().unwrap(), "{name}");
    }
    assert!(cases.len() >= 90);
    for run in doc["counters"].as_array().unwrap() {
        let mut counters = NumberedNameCounters::new();
        let numbers: Vec<u32> = run["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| counters.assign(&d.as_str().unwrap().to_owned()).unwrap())
            .collect();
        assert_eq!(json!(numbers), run["numbers"]);
    }
}

/// Outside the oracle domain: a char >= 0x100 reaching the swscanf
/// whitespace test is classified by Windows, so the port refuses it; the
/// numbered-name formats outside %d/%s are refused too.
#[test]
fn os_classified_and_foreign_formats_are_refused() {
    let w = |s: &str| -> Vec<u16> { s.encode_utf16().collect() };
    assert!(matches!(names::scan_decimal(&w("\u{3000}7")), Err(LoadError::Unsupported(_))));
    assert_eq!(names::scan_decimal(&w("7\u{3000}")).unwrap(), Some(7));
    assert_eq!(names::scan_decimal(&w("\u{a0}-12x")).unwrap(), Some((-12i32) as u32));
    assert_eq!(names::scan_decimal(&w("-")).unwrap(), None);
    assert!(names::format_numbered(&w("%s %d"), 1, &w("x")).is_err());
    assert_eq!(names::format_numbered(&w("%dth %s"), -10, &w("X")).unwrap(), w("-10th X"));
}
