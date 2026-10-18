use super::*;
use serde_json::{Value, json};
struct Probe<'a> {
    input: &'a Value,
    record: Record,
    events: Vec<String>,
    stats: u32,
}
impl Probe<'_> {
    fn enter(&mut self, event: String) -> Result<()> {
        self.events.push(event.clone());
        if self.input["stop"] == event {
            Err(Error::Unsupported("explicit native required entry"))
        } else {
            Ok(())
        }
    }
}
impl World for Probe<'_> {
    fn time(&mut self) -> Result<u32> {
        Ok(self.input["time"].as_u64().unwrap() as u32)
    }
    fn name(&mut self) -> Result<Vec<u16>> {
        self.enter("name".into())?;
        let key = match self.input["class"].as_u64().unwrap() {
            2 | 9 => "display",
            3 => {
                if self.input["key"] == "" {
                    "display"
                } else {
                    "text"
                }
            }
            4 => "kingdom",
            6 => "area",
            7 => "camera",
            _ => unreachable!(),
        };
        Ok(self.input[key].as_str().unwrap().encode_utf16().collect())
    }
    fn position(&mut self) -> Result<[u32; 2]> {
        Ok([
            self.input["position"][0].as_u64().unwrap() as u32,
            self.input["position"][1].as_u64().unwrap() as u32,
        ])
    }
    fn definition(&mut self) -> Result<String> {
        Ok("native-definition".into())
    }
    fn owner(&mut self) -> Result<ActorId> {
        Ok(ActorId::from_raw(
            if matches!(self.input["class"].as_u64().unwrap(), 3 | 4) {
                self.input["owner"].as_u64().unwrap() as u32
            } else {
                0
            },
        ))
    }
    fn aspect(&mut self) -> Result<u32> {
        self.enter("aspect".into())?;
        Ok(self.input["variant"].as_u64().unwrap() as u32)
    }
    fn stat(&mut self, _: u32) -> Result<u32> {
        self.stats += 1;
        self.enter(format!("stat{}", self.stats))?;
        Ok(0)
    }
    fn flags(&mut self) -> Result<u32> {
        Ok(self.input["flags"].as_u64().unwrap() as u32)
    }
    fn record(&mut self) -> Result<&mut Record> {
        Ok(&mut self.record)
    }
}
#[test]
fn native_memory_refresh_virtuals_and_partial_stores() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-store-memory-refresh/native-memory-refresh.json"
    ))
    .unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let mut record = Record::from_native_constructor();
        record.name_utf16 = "old".encode_utf16().collect();
        record.definition = Some("prior".into());
        record.owner = ActorId::from_raw(0xa5a5a5a5);
        record.aspect = 0xa5a5a5a5;
        record.stat_words = [Some(0xa5a5a5a5); 5];
        record.byte_30 = 0xa5;
        record.flags = 0xa5a5a5a5;
        let mut w = Probe {
            input: &row["input"],
            record,
            events: vec![],
            stats: 0,
        };
        let result = refresh_inner(&mut w);
        let expected = &row["native"];
        assert_eq!(result.is_ok(), expected["outcome"] == "complete", "{row}");
        let actual = json!({"time":w.record.timestamp_bits,"name":String::from_utf16(&w.record.name_utf16).unwrap(),"position":w.record.position_bits,"definition":w.record.definition.as_deref()==Some("native-definition"),"definition_raw":if w.record.definition.as_deref()==Some("native-definition"){0}else{0xa5a5a5a5u32},"owner":w.record.owner.raw(),"aspect":w.record.aspect,"stats":w.record.stat_words.map(Option::unwrap),"byte30":w.record.byte_30,"flags":w.record.flags,"events":w.events});
        let mut expected = expected.clone();
        expected.as_object_mut().unwrap().remove("outcome");
        assert_eq!(actual, expected, "{row}");
    }
}
