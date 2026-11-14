//! Replays research/phase8-movement/native-open-position.json (original
//! 553FA0 FindOpenPositionLine and 554BF0 FindOpenPositionLineThenRadius,
//! with 552F70 and 46E0A0, under Unicorn) against aa_spatial::open_position:
//! the AL result, the output point bits and every clamp / first-hit / radius
//! seam call with its descriptor fields, in order.
use aa_numeric::Precision;
use aa_spatial::{
    Result,
    collision::Shape,
    open_position::{
        OpenLineQuery, OpenLineRadiusWorld, OpenLineWorld, open_line, open_line_then_radius,
    },
};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-movement/native-open-position.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}
fn pair(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
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
    events: Vec<Value>,
    hits: usize,
}
impl OpenLineWorld for Host<'_> {
    fn cell_size(&mut self) -> Result<u32> {
        Ok(u(&self.c["cell"]))
    }
    fn clamp_to_interior(&mut self, point: &mut [u32; 2]) -> Result<()> {
        self.events.push(json!(["clamp", point[0], point[1]]));
        let [lo, hi] = pair(&self.c["box"]);
        for v in point.iter_mut() {
            if f32::from_bits(*v) < f32::from_bits(lo) {
                *v = lo;
            } else if f32::from_bits(*v) > f32::from_bits(hi) {
                *v = hi;
            }
        }
        Ok(())
    }
    fn first_hit(&mut self, q: &OpenLineQuery) -> Result<u32> {
        let s = &self.c["shape"];
        assert_eq!(
            [q.shape.kind, q.shape.radius_bits, q.shape.bounds[0], q.shape.bounds[1], q.shape.bounds[2], q.shape.bounds[3]],
            [u(&s[0]), u(&s[1]), u(&s[2]), u(&s[3]), u(&s[4]), u(&s[5])]
        );
        // Descriptor +54 is 0 and +18 (the snap guard) stays null.
        self.events.push(json!([
            "hit",
            q.position[0],
            q.position[1],
            q.required_actor_flags,
            q.mask,
            q.facing[0],
            q.facing[1],
            0,
            0
        ]));
        let seq = self.c["hits"].as_array().unwrap();
        let r = seq.get(self.hits).map(u).unwrap_or_else(|| u(&self.c["hit_default"]));
        self.hits += 1;
        Ok(r)
    }
}
impl OpenLineRadiusWorld for Host<'_> {
    fn find_open_radius(&mut self, point: &mut [u32; 2], facing: [u32; 2], radius: u32) -> Result<bool> {
        assert_eq!(facing, pair(&self.c["facing"]));
        self.events.push(json!(["radius", point[0], point[1], radius, 0, 0]));
        let ok = self.c["radius_ok"].as_bool().unwrap();
        if ok {
            *point = pair(&self.c["radius_point"]);
        }
        Ok(ok)
    }
}

#[test]
fn open_position_matches_native() {
    let d = fixture();
    let cases = d["cases"].as_array().unwrap();
    assert!(cases.len() >= 2100);
    let mut moved = 0;
    for case in cases {
        let c = &case["input"];
        let want = &case["output"];
        let name = c["name"].as_str().unwrap();
        let p = precision(c["precision"].as_str().unwrap());
        let s = &c["shape"];
        let shape = Shape { kind: u(&s[0]), radius_bits: u(&s[1]), bounds: [u(&s[2]), u(&s[3]), u(&s[4]), u(&s[5])] };
        let water = (u(&c["def_flags"]) >> 21) & 1 != 0;
        let mut point = pair(&c["point"]);
        let facing = pair(&c["facing"]);
        let from = pair(&c["from"]);
        let mut host = Host { c, events: Vec::new(), hits: 0 };
        let r = if c["entry"] == "line" {
            open_line(&mut point, facing, shape, water, from, c["non_blocking"].as_bool().unwrap(), &mut host, p)
        } else {
            open_line_then_radius(&mut point, facing, shape, water, from, u(&c["radius"]), &mut host, p)
        }
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(u32::from(r), u(&want["result"]), "{name} AL");
        assert_eq!(Value::Array(host.events), want["events"], "{name} events");
        assert_eq!(json!(point), want["point"], "{name} point");
        moved += usize::from(point != pair(&c["point"]));
    }
    assert!(moved > 1000, "{moved}");
}
