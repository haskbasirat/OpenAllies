use aa_mapworld::header::MapHeader;
use aa_runtime::{
    film_creator,
    film_header::{self, Result, Sink},
};
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
fn original_v14_all_pc_actual_derived_version_base_and_source_resource() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-creator.json"
    ))
    .unwrap();
    let rows = data["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 24);
    for row in rows {
        let header: MapHeader = serde_json::from_value(row["input"].clone()).unwrap();
        let source: Vec<u16> = row["source"]["ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u16)
            .collect();
        let mut stream = Stream::default();
        film_header::header(&mut stream, &header).unwrap();
        let prefix = stream.bytes.len();
        film_header::string(&mut stream, &header.creator.class_name).unwrap();
        film_creator::load_payload(
            &mut stream,
            &header.creator,
            row["source"]["present"]
                .as_bool()
                .unwrap()
                .then_some(source.as_slice()),
        )
        .unwrap();
        assert_eq!(
            json!({"events":stream.events,"payload_hex":hex(&stream.bytes),"header_bytes":prefix,"registered":stream.registered}),
            row["output"],
            "{} {}",
            row["pc"],
            row["source"]["present"]
        );
    }
}
