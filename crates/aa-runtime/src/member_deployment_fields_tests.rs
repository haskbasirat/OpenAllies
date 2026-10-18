use super::*;
use serde_json::{Value, json};
struct Fixture<'a> {
    case: &'a Value,
    events: Vec<Value>,
    definition: u32,
    changed: bool,
    params_address: usize,
}
fn number(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn f(v: f32) -> u32 {
    v.to_bits()
}
fn snapshot(p: &ActorParams<u32, String>) -> Value {
    json!({"position":p.position_bits,"direction":p.direction_bits,"byte52":p.byte_52})
}
impl Fixture<'_> {
    fn event(&mut self, label: &str, args: Vec<Value>) -> Result<()> {
        let mut event = vec![json!(label)];
        event.extend(args);
        self.events.push(json!(event));
        if self.case["fail"] == label {
            Err(label.into())
        } else {
            Ok(())
        }
    }
    fn change(&mut self) {
        self.definition = 2;
        self.changed = true;
    }
}
impl World<u32, String> for Fixture<'_> {
    type Actor = u32;
    type Definition = u32;
    fn definition(&mut self, actor: u32) -> Result<u32> {
        self.event("definition", vec![json!(actor), json!(self.definition)])?;
        Ok(self.definition)
    }
    fn enabled_3ec(&mut self, d: &u32) -> Result<bool> {
        self.event("enabled", vec![json!(d)])?;
        Ok(self.case["enabled"].as_bool().unwrap())
    }
    fn source_word(&mut self, d: &u32, offset: u32) -> Result<u32> {
        self.event("source", vec![json!(d), json!(offset)])?;
        if *d == 2 && offset == 0x3dc {
            return Ok(f(-5.0));
        }
        if *d == 2 && offset == 0x3e8 {
            return Ok(f(19.0));
        }
        let (name, index) = match offset {
            0x3dc => ("creation", 0),
            0x3e0 => ("creation", 1),
            0x3e4 => ("deployment", 0),
            0x3e8 => ("deployment", 1),
            _ => panic!("unknown actual source field"),
        };
        Ok(number(&self.case[name][index]))
    }
    fn actor_word(&mut self, actor: u32, offset: u32) -> Result<u32> {
        self.event("actor", vec![json!(actor), json!(offset)])?;
        if self.changed && offset == 0x20 {
            return Ok(f(-77.0));
        }
        if self.changed && offset == 0x24 {
            return Ok(f(66.0));
        }
        let (name, index) = match offset {
            0x20 => ("position", 0),
            0x24 => ("position", 1),
            0x2c => ("facing", 0),
            0x30 => ("facing", 1),
            _ => panic!("unknown actual actor field"),
        };
        Ok(number(&self.case[name][index]))
    }
    fn contains_46ec70(&mut self, _: u32, point: [u32; 2]) -> Result<bool> {
        self.event("contains", vec![json!(point)])?;
        if self.case["action"] == "shape_changes_source_pose" {
            self.change();
        }
        Ok(self.case["inside"].as_bool().unwrap())
    }
    fn warn_inside_404ed0(&mut self, actor: u32) -> Result<()> {
        self.event("warn", vec![json!(actor)])
    }
    fn create_4f9c10(&mut self, params: &mut ActorParams<u32, String>) -> Result<Option<u32>> {
        assert_eq!(self.params_address, std::ptr::from_mut(params) as usize);
        assert_eq!((params.definition, params.word_40), (91, 0x1234));
        self.event("create", vec![snapshot(params)])?;
        if self.case["action"] == "create_changes_source_pose_params" {
            self.change();
            params.position_bits[0] = f(99.0);
            params.direction_bits[0] = f(0.25);
            params.byte_52 = 7;
        }
        Ok(self.case["created"].as_bool().unwrap().then_some(2))
    }
    fn mind_deploy_198(&mut self, member: u32, point: [u32; 2], structure: u32) -> Result<bool> {
        self.event("mind", vec![json!(member), json!(point), json!(structure)])?;
        Ok(self.case["mind"].as_bool().unwrap())
    }
    fn log_failed_404f60(&mut self, member: u32) -> Result<()> {
        self.event("log", vec![json!(member)])
    }
    fn destroy_4f6760(&mut self, member: u32, flag: u8) -> Result<()> {
        self.event("destroy", vec![json!(member), json!(flag)])
    }
}
#[test]
fn original_v14_deployment_transforms_same_params_and_child_failure_prefix_match() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-member-deployment/native-member-deployment.json"
    ))
    .unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let pc = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => panic!(),
        };
        let mut params = ActorParams::new(91);
        params.position_bits = [f(33.0), f(44.0), f(55.0)];
        params.byte_52 = 0x5a;
        params.word_40 = 0x1234;
        let address = std::ptr::from_mut(&mut params) as usize;
        let mut world = Fixture {
            case: &row["input"],
            events: vec![],
            definition: 1,
            changed: false,
            params_address: address,
        };
        let (result, error) = match deploy(&mut world, &mut params, 1, pc) {
            Ok(r) => (r, None),
            Err(e) => (None, Some(e)),
        };
        assert_eq!(
            json!({"result":result,"error":error,"events":world.events,"params":snapshot(&params)}),
            row["output"],
            "{}",
            row["input"]
        );
    }
}

#[test]
fn original_v14_actual_shape_contains_local_deployment_point() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-member-deployment/native-member-deployment.json"
    ))
    .unwrap();
    for row in fixture["shape_rows"].as_array().unwrap() {
        let pc = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => panic!(),
        };
        let words = row["input"]["shape"]
            .as_array()
            .unwrap()
            .iter()
            .map(number)
            .collect::<Vec<_>>();
        let shape = aa_spatial::collision::Shape {
            kind: words[0],
            radius_bits: words[1],
            bounds: [words[2], words[3], words[4], words[5]],
        };
        let point = [
            number(&row["input"]["point"][0]),
            number(&row["input"]["point"][1]),
        ];
        assert_eq!(
            contains_46ec70(&shape, point, pc).unwrap(),
            row["output"].as_bool().unwrap(),
            "{}",
            row["input"]
        );
    }
}
