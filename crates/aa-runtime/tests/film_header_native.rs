use aa_mapworld::header::MapHeader;
use aa_runtime::film_header::{self, Result, Sink};
use serde_json::{Value, json};

#[derive(Default)]
struct Stream {
    bytes: Vec<u8>,
    events: Vec<Value>,
    registered: Vec<Vec<u16>>,
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
impl Sink for Stream {
    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.events.push(json!({"write":hex(bytes)}));
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn register_map_string(&mut self, key: &[u16]) -> Result<()> {
        self.events.push(json!({"register":key}));
        self.registered.push(key.to_vec());
        Ok(())
    }
}
#[test]
fn original_v14_all_pc_latest_header_creator_and_key_only_collection_order() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-header.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 9);
    for row in cases {
        let header: MapHeader = serde_json::from_value(row["input"].clone()).unwrap();
        let mut stream = Stream::default();
        film_header::header(&mut stream, &header).unwrap();
        let header_bytes = stream.bytes.len();
        film_header::creator(&mut stream, &header.creator).unwrap();
        assert_eq!(
            json!({"events":stream.events,"payload_hex":hex(&stream.bytes),
            "header_bytes":header_bytes,"registered":stream.registered}),
            row["output"],
            "{}",
            row["pc"]
        );
    }
}
