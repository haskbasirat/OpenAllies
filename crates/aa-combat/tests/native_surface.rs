use aa_body::F32;
use aa_combat::{
    Result,
    surface::{self, World},
};
use aa_numeric::{Binary, Precision};
use aa_spatial::collision::Shape;
use serde_json::{Value, json};
struct Host {
    input: Value,
    position: [F32; 3],
    facing: [F32; 2],
    ratio: F32,
    events: Vec<&'static str>,
}
impl World for Host {
    fn actor_radius(&mut self, _: u32) -> Result<Binary> {
        self.events.push("radius");
        if self.input["mutations"] == true {
            self.facing = [F32(0.3f32.to_bits()), F32((-0.7f32).to_bits())];
        }
        Ok(Binary::from_f64_bits(
            self.input["radius"].as_u64().unwrap(),
        )?)
    }
    fn actor_facing(&self, _: u32) -> Result<[F32; 2]> {
        Ok(self.facing)
    }
    fn target_shape(&mut self, _: u32) -> Result<Shape> {
        self.events.push("shape");
        if self.input["mutations"] == true {
            self.position = [
                F32(11.7f32.to_bits()),
                F32((-8.3f32).to_bits()),
                F32(4.19f32.to_bits()),
            ];
            self.facing = [F32((-0.8f32).to_bits()), F32(0.6f32.to_bits())];
            self.ratio = F32(0.37f32.to_bits());
        }
        Ok(serde_json::from_value(self.input["shape"].clone()).unwrap())
    }
    fn actor_position(&self, _: u32) -> Result<[F32; 3]> {
        Ok(self.position)
    }
    fn default_point(&self) -> Result<[F32; 2]> {
        Ok(serde_json::from_value(self.input["default_point"].clone()).unwrap())
    }
    fn target_height_ratio(&self) -> Result<F32> {
        Ok(self.ratio)
    }
    fn actor_height(&mut self, _: u32) -> Result<Binary> {
        self.events.push("height");
        if self.input["mutations"] == true {
            self.position[2] = F32(999f32.to_bits());
            self.ratio = F32(777f32.to_bits());
        }
        if self.input["fail_height"] == true {
            return Err(aa_combat::Error::World("height failed".into()));
        }
        Ok(Binary::from_f64_bits(
            self.input["height"].as_u64().unwrap(),
        )?)
    }
}
#[test]
fn original_shape_projection_and_mutable_actor_hit_surface_match() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-surface/native-surface-fixtures.json"
    ))
    .unwrap();
    for row in f["cases"].as_array().unwrap() {
        let i = &row["input"];
        let mut host = Host {
            input: i.clone(),
            position: serde_json::from_value(i["position"].clone()).unwrap(),
            facing: serde_json::from_value(i["facing"].clone()).unwrap(),
            ratio: serde_json::from_value(i["ratio"].clone()).unwrap(),
            events: vec![],
        };
        let p: Precision = serde_json::from_value(i["precision"].clone()).unwrap();
        let direction: [F32; 3] = serde_json::from_value(i["direction"].clone()).unwrap();
        let result = if i["operation"] == "shape" {
            json!(
                surface::shape_projection(
                    &serde_json::from_value(i["shape"].clone()).unwrap(),
                    host.facing,
                    [direction[0], direction[1]],
                    host.default_point().unwrap(),
                    p
                )
                .unwrap()
            )
        } else {
            json!(surface::hit_position(1, direction, &mut host, p).unwrap())
        };
        let actual = json!({"result":result,"events":host.events,"position":host.position,"facing":host.facing,"ratio":host.ratio});
        assert_eq!(actual, row["output"], "{}", i["name"]);
    }
}

#[test]
fn hit_surface_propagates_getter_failure_after_visible_mutations() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-surface/native-surface-fixtures.json"
    ))
    .unwrap();
    let mut input = f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["input"]["mutations"] == true)
        .unwrap()["input"]
        .clone();
    input["fail_height"] = json!(true);
    let mut h = Host {
        position: serde_json::from_value(input["position"].clone()).unwrap(),
        facing: serde_json::from_value(input["facing"].clone()).unwrap(),
        ratio: serde_json::from_value(input["ratio"].clone()).unwrap(),
        input,
        events: vec![],
    };
    assert!(surface::hit_position(1, [F32::ONE; 3], &mut h, Precision::Double53).is_err());
    assert_eq!(h.position[2], F32(999f32.to_bits()));
    assert_eq!(h.events, ["radius", "shape", "height"]);
}
