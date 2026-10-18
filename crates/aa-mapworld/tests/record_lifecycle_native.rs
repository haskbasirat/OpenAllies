use aa_mapworld::record_lifecycle::{self, Result, World};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::BTreeMap;

struct Storage {
    case: Value,
    current: Option<u32>,
    records: BTreeMap<u32, Value>,
    flags: BTreeMap<u32, u32>,
    trace: Vec<Value>,
}
fn word(v: &Value) -> u32 {
    v.as_u64().unwrap().try_into().unwrap()
}
impl Storage {
    fn new(case: &Value) -> Self {
        let records: BTreeMap<_, _> = case["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| (word(&r["id"]), r.clone()))
            .collect();
        let flags = records
            .iter()
            .map(|(id, row)| (*id, word(&row["flags"])))
            .collect();
        Self {
            case: case.clone(),
            current: case["current"].as_u64().map(|v| v as u32),
            records,
            flags,
            trace: vec![],
        }
    }
    fn event(&mut self, row: Value) -> Result<()> {
        self.trace.push(row);
        let step = self.trace.len() as u64;
        if self.case["stop"].as_u64() == Some(step) {
            return Err("stopped".into());
        }
        for change in self.case["mutations"].as_array().unwrap() {
            if change["event"].as_u64() == Some(step) {
                if let Some(out) = change.get("out") {
                    self.current = out.as_u64().map(|v| v as u32);
                }
                if let Some(flags) = change.get("flags") {
                    self.flags.insert(word(&flags["id"]), word(&flags["bits"]));
                }
            }
        }
        Ok(())
    }
}
impl World for Storage {
    type Record = u32;
    fn current_record(&mut self) -> Result<Option<u32>> {
        self.event(json!(["current"]))?;
        Ok(self.current)
    }
    fn flags(&mut self, id: u32) -> Result<u32> {
        self.event(json!(["flags", id]))?;
        Ok(self.flags[&id])
    }
    fn clear_loading_flag(&mut self, id: u32) -> Result<()> {
        self.event(json!(["clear_loading", id]))?;
        *self.flags.get_mut(&id).unwrap() &= 0x7fff_ffff;
        Ok(())
    }
    fn init_null(&mut self, id: u32) -> Result<u8> {
        self.event(json!(["init", id]))?;
        Ok(self.records[&id]["init_return"].as_u64().unwrap() as u8)
    }
    fn editor_insert_out_slot(&mut self) -> Result<()> {
        self.event(json!(["editor"]))
    }
    fn post_create_null_one(&mut self, id: u32) -> Result<()> {
        self.event(json!(["post", id]))
    }
    fn register_record(&mut self, id: u32) -> Result<()> {
        self.event(json!(["register", id]))
    }
    fn object_predicate(&mut self, id: u32) -> Result<u8> {
        self.event(json!(["object", id]))?;
        Ok(self.records[&id]["object_return"].as_u64().unwrap() as u8)
    }
    fn definition_publish_one(&mut self, id: u32) -> Result<()> {
        self.event(json!(["publish", id]))
    }
    fn init_failed_warning(&mut self, id: u32, _: Precision) -> Result<()> {
        let xy = self.records[&id]
            .get("xy")
            .cloned()
            .unwrap_or_else(|| json!([3.75f32.to_bits(), (-4.5f32).to_bits()]));
        self.event(json!([
            "warning",
            id,
            (f32::from_bits(word(&xy[0])) as f64).to_bits(),
            (f32::from_bits(word(&xy[1])) as f64).to_bits()
        ]))
    }
    fn destroy_record(&mut self, id: u32) -> Result<()> {
        self.event(json!(["destroy", id]))
    }
    fn clear_out_slot(&mut self) -> Result<()> {
        self.event(json!(["clear_out"]))?;
        self.current = None;
        Ok(())
    }
}

#[test]
fn per_record_init_postcreate_registration_and_failure_prefix_match_native() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-registration/native-lifecycle.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 636);
    for row in rows {
        let case = &row["input"];
        let pc = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => unreachable!(),
        };
        let mut state = Storage::new(case);
        let result = record_lifecycle::finish(&mut state, pc);
        let outcome = match &result {
            Ok(()) => "ok",
            Err(e) if e == "stopped" => "stopped",
            Err(e) if e.contains("current record became null") => "required_null",
            Err(e) => panic!("unexpected error {e}"),
        };
        let output = json!({"outcome":outcome,"trace":state.trace,"current":state.current,"flags":case["records"].as_array().unwrap().iter().map(|r|json!([r["id"],state.flags[&word(&r["id"])]] )).collect::<Vec<_>>(),"complete_native_epilogue":result.is_ok()});
        assert_eq!(
            output, row["output"],
            "{} / {} / {}",
            case["name"], row["image"], row["precision"]
        );
    }
}
