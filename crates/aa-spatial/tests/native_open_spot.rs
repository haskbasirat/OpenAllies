//! Replays research/phase8-movement/native-open-spot.json (original 554500
//! with 553000, 552250, 46EE00 and the 85DAA0/85DA80 table lookups under
//! Unicorn) against aa_spatial::open_position::open_spot: the AL result, the
//! point and direction bits and every definition-virtual / first-hit seam
//! call with its descriptor fields, in order. The host answers the table
//! lookups with the words the original tables returned in the same run.
use aa_numeric::Precision;
use aa_spatial::{
    Result,
    collision::Shape,
    open_position::{OpenLineQuery, OpenSpotWorld, open_spot},
};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-movement/native-open-spot.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}
fn pair(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
}
fn shape(v: &Value) -> Shape {
    Shape { kind: u(&v[0]), radius_bits: u(&v[1]), bounds: [u(&v[2]), u(&v[3]), u(&v[4]), u(&v[5])] }
}
fn precision(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("precision {other}"),
    }
}

struct Host<'a> {
    c: &'a Value,
    tables: &'a Value,
    events: Vec<Value>,
    hits: usize,
}
impl OpenSpotWorld for Host<'_> {
    fn alternate_shape(&mut self) -> Result<Shape> {
        self.events.push(json!(["v28"]));
        Ok(shape(&self.c["alt_shape"]))
    }
    fn definition_virtual_30(&mut self) -> Result<u32> {
        self.events.push(json!(["v30"]));
        Ok(u(&self.c["v30"]))
    }
    fn definition_byte_1e(&mut self) -> Result<u8> {
        Ok((u(&self.c["def_flags"]) >> 16) as u8)
    }
    fn cell_size(&mut self) -> Result<u32> {
        Ok(u(&self.c["cell"]))
    }
    fn first_hit(&mut self, q: &OpenLineQuery) -> Result<u32> {
        let want = if self.c["alternate"].as_bool().unwrap() { &self.c["alt_shape"] } else { &self.c["shape"] };
        assert_eq!(q.shape, shape(want));
        self.events.push(json!([
            "hit",
            q.position[0],
            q.position[1],
            q.required_actor_flags,
            q.mask,
            q.facing[0],
            q.facing[1],
            0
        ]));
        let seq = self.c["hits"].as_array().unwrap();
        let r = seq.get(self.hits).map(u).unwrap_or_else(|| u(&self.c["hit_default"]));
        self.hits += 1;
        Ok(r)
    }
    fn table_sin(&mut self, angle_bits: u32) -> Result<u32> {
        Ok(u(&self.tables[angle_bits.to_string()][0]))
    }
    fn table_cos(&mut self, angle_bits: u32) -> Result<u32> {
        Ok(u(&self.tables[angle_bits.to_string()][1]))
    }
}

#[test]
fn open_spot_matches_native() {
    let d = fixture();
    let cases = d["cases"].as_array().unwrap();
    assert!(cases.len() >= 1200);
    let mut found = 0;
    for case in cases {
        let c = &case["input"];
        let want = &case["output"];
        let name = c["name"].as_str().unwrap();
        let p = precision(c["precision"].as_str().unwrap());
        let water = (u(&c["def_flags"]) >> 21) & 1 != 0;
        let mut point = pair(&c["point"]);
        let mut direction = pair(&c["direction"]);
        let mut host = Host { c, tables: &want["tables"], events: Vec::new(), hits: 0 };
        let r = open_spot(
            &mut point,
            &mut direction,
            shape(&c["shape"]),
            water,
            u(&c["radius"]),
            u(&c["scale"]),
            c["non_blocking"].as_bool().unwrap(),
            c["alternate"].as_bool().unwrap(),
            &mut host,
            p,
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(u32::from(r), u(&want["result"]), "{name} AL");
        assert_eq!(Value::Array(host.events), want["events"], "{name} events");
        assert_eq!(json!(point), want["point"], "{name} point");
        assert_eq!(json!(direction), want["direction"], "{name} direction");
        found += usize::from(r);
    }
    assert!(found > 900 && found < cases.len(), "{found}");
}
