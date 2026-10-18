use aa_mapworld::header::MapHeader;
use aa_runtime::{
    film_header::{Result, Sink},
    film_parameter_serialization::{self as writer, Decoded, Kingdom, ParameterSource, Team},
};
use serde_json::{Value, json};
fn words(v: &Value) -> Vec<u16> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u16)
        .collect()
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|b| format!("{b:02x}")).collect()
}
struct Paired<'a> {
    decoded: Decoded<'a>,
    teams: Vec<Vec<u16>>,
    kingdoms: Vec<(Vec<u16>, Vec<Vec<u16>>)>,
}
impl ParameterSource for Paired<'_> {
    fn team_count(&self) -> usize {
        self.decoded.team_count()
    }
    fn team(&self, i: usize) -> Result<Team<'_>> {
        let mut t = self.decoded.team(i)?;
        t.display.text = Some(&self.teams[i]);
        Ok(t)
    }
    fn kingdom_count(&self) -> usize {
        self.decoded.kingdom_count()
    }
    fn kingdom(&self, i: usize) -> Result<Kingdom<'_>> {
        let mut k = self.decoded.kingdom(i)?;
        k.display.text = Some(&self.kingdoms[i].0);
        for (pair, text) in k.entries.iter_mut().zip(&self.kingdoms[i].1) {
            pair.text = Some(text);
        }
        Ok(k)
    }
    fn resource_count(&self, k: usize) -> Result<usize> {
        self.decoded.resource_count(k)
    }
    fn resource(&self, k: usize, i: usize) -> Result<(Vec<u16>, u32)> {
        self.decoded.resource(k, i)
    }
}
struct Stream {
    bytes: Vec<u8>,
    events: Vec<Value>,
    registered: Vec<Vec<u16>>,
    stop: Option<usize>,
}
impl Sink for Stream {
    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        if self.stop == Some(self.events.len()) {
            return Err("write".into());
        }
        self.events.push(json!({"write":hex(bytes)}));
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn register_map_string(&mut self, _: &[u16]) -> Result<()> {
        panic!("a current pair must not silently fall back to key-only service")
    }
    fn register_localized_pair(&mut self, key: &[u16], text: Option<&[u16]>) -> Result<()> {
        if self.stop == Some(self.events.len()) {
            return Err("register".into());
        }
        self.events
            .push(json!({"register":key,"text":text.expect("actual pair second word")}));
        self.registered.push(key.to_vec());
        Ok(())
    }
}
#[test]
fn original_v14_all_pc_current_pair_transport_and_all_write_prefixes() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-parameter-pairs.json"
    ))
    .unwrap();
    let cases = data["cases"].as_array().unwrap();
    assert!(!cases.is_empty());
    for row in cases {
        let input = &row["input"];
        let header: MapHeader = serde_json::from_value(input["header"].clone()).unwrap();
        let pairs = &input["pairs"];
        let source = Paired {
            decoded: Decoded(&header.parameters),
            teams: pairs["teams"]
                .as_array()
                .unwrap()
                .iter()
                .map(words)
                .collect(),
            kingdoms: pairs["kingdoms"]
                .as_array()
                .unwrap()
                .iter()
                .map(|k| {
                    (
                        words(&k["text"]),
                        k["entries"].as_array().unwrap().iter().map(words).collect(),
                    )
                })
                .collect(),
        };
        let mut stream = Stream {
            bytes: Vec::new(),
            events: Vec::new(),
            registered: Vec::new(),
            stop: input["stop"].as_u64().map(|n| n as usize),
        };
        let result = writer::write(&mut stream, &source);
        assert_eq!(
            json!({"events":stream.events,"payload_hex":hex(&stream.bytes),"registered":stream.registered,"stopped":result.as_ref().err()}),
            row["output"],
            "{}",
            row["pc"]
        );
    }
}
#[test]
fn current_pair_missing_service_retains_written_key_and_refuses_text_loss() {
    struct Legacy {
        bytes: Vec<u8>,
        key_calls: usize,
    }
    impl Sink for Legacy {
        fn write(&mut self, b: &[u8]) -> Result<()> {
            self.bytes.extend_from_slice(b);
            Ok(())
        }
        fn register_map_string(&mut self, _: &[u16]) -> Result<()> {
            self.key_calls += 1;
            Ok(())
        }
    }
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-parameter-pairs.json"
    ))
    .unwrap();
    let input = &fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| !r["input"]["pairs"]["teams"].as_array().unwrap().is_empty())
        .unwrap()["input"];
    let header: MapHeader = serde_json::from_value(input["header"].clone()).unwrap();
    let source = Paired {
        decoded: Decoded(&header.parameters),
        teams: vec![vec![99]],
        kingdoms: Vec::new(),
    };
    let mut sink = Legacy {
        bytes: Vec::new(),
        key_calls: 0,
    };
    let result = writer::write(&mut sink, &source);
    assert!(result.unwrap_err().contains("pair-aware"));
    assert_eq!(sink.key_calls, 0);
    let mut expected = Vec::new();
    expected.extend_from_slice(&13u32.to_le_bytes());
    expected.extend_from_slice(&1u32.to_le_bytes());
    for s in [
        &header.parameters.teams[0].id,
        &header.parameters.teams[0].display_key,
    ] {
        expected.extend_from_slice(&(s.len() as u16).to_le_bytes());
        for u in s {
            expected.extend_from_slice(&u.to_le_bytes());
        }
    }
    assert_eq!(sink.bytes, expected);
}
