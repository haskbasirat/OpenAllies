use aa_numeric::Precision;
use aa_spatial::{Error, Result, collision::Shape, range::ActorTransform, soft_range::*};
use serde_json::{Value, json};
use std::cell::{Cell, RefCell};
const OBJECT_BASE: u64 = 1 << 40;
struct Host {
    c: Value,
    events: RefCell<Vec<Value>>,
    read_count: Cell<u32>,
    fail_getter: Option<u64>,
}
impl World for Host {
    type Actor = u64;
    type ShapeHandle = u32;
    fn snapshot_transform(&self, actor: u64) -> Result<ActorTransform> {
        let id = actor - OBJECT_BASE;
        let a = &self.c["actors"][id.to_string()];
        let transform = ActorTransform {
            position: serde_json::from_value(a["position"].clone()).unwrap(),
            facing: serde_json::from_value(a["facing"].clone()).unwrap(),
        };
        self.events.borrow_mut().push(json!({"op":"snapshot","actor":id,"position":transform.position,"facing":transform.facing}));
        Ok(transform)
    }
    fn soft_shape(&mut self, actor: u64) -> Result<u32> {
        let id = actor - OBJECT_BASE;
        self.events
            .borrow_mut()
            .push(json!({"op":"getter","actor":id}));
        if self.fail_getter == Some(id) {
            return Err(Error::World("fixture soft getter failed".into()));
        }
        if self.c["ordinary"] != true {
            for m in self.c["mutations"].as_array().unwrap().clone() {
                if m["on_actor"].as_u64() == Some(id) {
                    let key = m["actor"].as_u64().unwrap().to_string();
                    for field in ["position", "facing"] {
                        if !m[field].is_null() {
                            self.c["actors"][&key][field] = m[field].clone();
                        }
                    }
                    if !m["shape"].is_null() {
                        self.c["shapes"][m["shape_id"].as_u64().unwrap().to_string()] =
                            m["shape"].clone();
                    }
                }
            }
        }
        Ok(self.c["actors"][id.to_string()]["shape"].as_u64().unwrap() as u32)
    }
    fn read_shape(&self, handle: &u32) -> Result<Shape> {
        self.read_count.set(self.read_count.get() + 1);
        Ok(serde_json::from_value(self.c["shapes"][handle.to_string()].clone()).unwrap())
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase7-soft-range/native-soft-range.json"
    ))
    .unwrap()
}
fn host(c: &Value) -> Host {
    Host {
        c: c.clone(),
        events: RefCell::new(vec![]),
        read_count: Cell::new(0),
        fail_getter: None,
    }
}
#[test]
fn actual_source_first_soft_capture_and_native_shape_range_match() {
    let f = fixture();
    let rows = f["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 330);
    for row in rows {
        let c = &row["input"];
        let mut h = host(c);
        let precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let result = source_first_in_range(
            OBJECT_BASE + 1,
            OBJECT_BASE + 2,
            0x3dcccccd,
            precision,
            &mut h,
        )
        .unwrap();
        assert_eq!(
            json!({"result":result,"events":*h.events.borrow()}),
            row["output"],
            "{}",
            c["name"]
        );
        assert_eq!(h.read_count.get(), 2);
    }
}
#[test]
fn actual_mover_source_receiver_fragment_and_branch_witnesses_match() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-soft-range/native-mover-range.json"
    ))
    .unwrap();
    let rows = f["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 336);
    let mut different = 0;
    for row in rows {
        let c = &row["input"];
        let mut h = host(c);
        let precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let result = source_receiver_in_range(
            OBJECT_BASE + 1,
            OBJECT_BASE + 2,
            0x3dcccccd,
            precision,
            &mut h,
        )
        .unwrap();
        assert_eq!(
            json!({"result":result,"events":*h.events.borrow()}),
            row["output"],
            "{}",
            c["name"]
        );
        assert_eq!(h.read_count.get(), 2);
        let arrival = source_first_in_range(
            OBJECT_BASE + 1,
            OBJECT_BASE + 2,
            0x3dcccccd,
            precision,
            &mut host(c),
        )
        .unwrap();
        different += usize::from(result != arrival);
    }
    assert_eq!(
        different,
        f["receiver_differences_against_arrival"].as_u64().unwrap() as usize
    );
    assert!(different >= 6);
}
#[test]
fn second_getter_failure_preserves_first_effect_and_skips_shape_reads() {
    let f = fixture();
    let c = &f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["input"]["name"] == "mutation_1_2_shape_Double53")
        .unwrap()["input"];
    let mut h = host(c);
    let before = h.c["shapes"]["2"].clone();
    h.fail_getter = Some(2);
    assert!(
        source_first_in_range(
            OBJECT_BASE + 1,
            OBJECT_BASE + 2,
            0x3dcccccd,
            Precision::Double53,
            &mut h
        )
        .is_err()
    );
    assert_ne!(h.c["shapes"]["2"], before);
    assert_eq!(h.read_count.get(), 0);
    assert_eq!(
        h.events.borrow().last().unwrap(),
        &json!({"op":"getter","actor":2})
    );
}
