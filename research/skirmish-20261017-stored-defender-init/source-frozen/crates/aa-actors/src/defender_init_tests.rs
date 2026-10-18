use super::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
struct Lab<'a> {
    input: &'a Value,
    stop: Option<&'a str>,
    events: Vec<String>,
    counts: BTreeMap<&'static str, usize>,
    range: u32,
    zone: bool,
    count: u32,
    state: u8,
    stopped: Option<String>,
}
impl Lab<'_> {
    fn event(&mut self, label: &'static str) -> Result<()> {
        let count = self.counts.entry(label).or_default();
        let key = format!("{label}:{count}");
        *count += 1;
        if self.stop == Some(&key) {
            self.stopped = Some(key);
            return Err(Error::World(
                "declared native child/instruction stop".into(),
            ));
        }
        self.events.push(key);
        Ok(())
    }
    fn word(&self, key: &str) -> u32 {
        self.input[key].as_u64().unwrap() as u32
    }
    fn flag(&self, key: &str) -> bool {
        self.input[key].as_bool().unwrap()
    }
}
impl World for Lab<'_> {
    fn zone_present(&mut self) -> Result<bool> {
        self.event("zone")?;
        Ok(self.zone)
    }
    fn definition_mask(&mut self) -> Result<u32> {
        self.event("mask")?;
        Ok(self.word("mask"))
    }
    fn range(&mut self, organization: bool) -> Result<u32> {
        self.event("range")?;
        Ok(self.word(if organization { "range2fc" } else { "range218" }))
    }
    fn store_range(&mut self, bits: u32) -> Result<()> {
        self.event("store")?;
        self.range = bits;
        Ok(())
    }
    fn create_zone(&mut self) -> Result<bool> {
        self.event("create")?;
        self.zone = true;
        self.count = 9;
        self.state = 1;
        Ok(true)
    }
    fn aircraft_present(&mut self) -> Result<bool> {
        self.event("aircraft")?;
        Ok(self.flag("aircraft"))
    }
    fn aircraft_zone_flag(&mut self) -> Result<()> {
        self.event("flag")?;
        self.state |= 2;
        Ok(())
    }
    fn position(&mut self) -> Result<[u32; 2]> {
        self.event("position")?;
        Ok([
            self.input["xy"][0].as_u64().unwrap() as u32,
            self.input["xy"][1].as_u64().unwrap() as u32,
        ])
    }
    fn move_zone(&mut self, p: [u32; 2], range: u32) -> Result<()> {
        self.event("move")?;
        assert_eq!(range, self.range);
        assert_eq!(
            p,
            [
                self.input["xy"][0].as_u64().unwrap() as u32,
                self.input["xy"][1].as_u64().unwrap() as u32
            ]
        );
        Ok(())
    }
    fn performer_present(&mut self) -> Result<bool> {
        self.event("performer")?;
        Ok(self.flag("performer"))
    }
    fn refresh_performer(&mut self) -> Result<()> {
        self.event("refresh")
    }
}
#[test]
fn original_v14_all_pc_coordinator_and_partial_stores() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261017-stored-defender-init/native-defender.json");
    let fixture: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert!(!rows.is_empty());
    for row in rows {
        let input = &row["input"];
        let zone = input["zone"].as_bool().unwrap();
        let mut lab = Lab {
            input,
            stop: row["stop"].as_str(),
            events: vec![],
            counts: BTreeMap::new(),
            range: input["old_range"].as_u64().unwrap() as u32,
            zone,
            count: if zone { 9 } else { 0 },
            state: 1,
            stopped: None,
        };
        let result = initialize(&mut lab);
        let actual = json!({"events":lab.events,"range":lab.range,"count":lab.count,"zone":lab.zone,"zone_state":if lab.zone{Some(lab.state)}else{None},"zone_flags":if lab.zone{Some(0x10)}else{None},"stopped":lab.stopped,"result":result.ok().map(u32::from)});
        //566E8C writes AL=1; upper EAX is caller-clobbered native transport.
        //The actual coordinator's ABI consumes only its Boolean low byte.
        let mut expected = row["output"].clone();
        if let Some(eax) = expected["result"].as_u64() {
            expected["result"] = json!(eax & 0xff)
        }
        assert_eq!(actual, expected, "{row}");
    }
}
#[test]
fn successful_existing_zone_reads_no_source_or_range() {
    let input = json!({"zone":true,"aircraft":true,"performer":false,"old_range":0x7fa01234u32});
    let mut lab = Lab {
        input: &input,
        stop: None,
        events: vec![],
        counts: BTreeMap::new(),
        range: 0x7fa01234,
        zone: true,
        count: 9,
        state: 1,
        stopped: None,
    };
    assert_eq!(initialize(&mut lab).unwrap(), true);
    assert_eq!(lab.events, ["zone:0", "performer:0"]);
    assert_eq!(lab.range, 0x7fa01234);
}
