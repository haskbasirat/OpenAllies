use super::*;
use serde_json::{Value, json};
struct Ground {
    input: Value,
    pc: Precision,
    events: Vec<Value>,
    stop: Option<&'static str>,
}
impl Ground {
    fn word(&self, name: &str) -> u32 {
        self.input[name].as_u64().unwrap() as u32
    }
    fn event(&mut self, event: Value) -> Result<()> {
        let stop = self.stop == event["op"].as_str();
        self.events.push(event);
        if stop {
            Err(Error::World("required child".into()))
        } else {
            Ok(())
        }
    }
}
impl ObjectEffects for Ground {
    fn precision(&self) -> Precision {
        self.pc
    }
    fn flags(&self) -> Result<u32> {
        Ok(self.word("flags"))
    }
    fn blocking_radius(&self) -> Result<u32> {
        Ok(self.input["shape"][1].as_u64().unwrap() as u32)
    }
    fn cell_size(&self) -> Result<u32> {
        Ok(self.word("cell"))
    }
    fn position(&self) -> Result<[u32; 2]> {
        Ok(std::array::from_fn(|i| {
            self.input["position"][i].as_u64().unwrap() as u32
        }))
    }
    fn blocking_shape(&self) -> Result<Shape> {
        let v = &self.input["shape"];
        Ok(Shape {
            kind: v[0].as_u64().unwrap() as u32,
            radius_bits: v[1].as_u64().unwrap() as u32,
            bounds: std::array::from_fn(|i| v[i + 2].as_u64().unwrap() as u32),
        })
    }
    fn facing(&self) -> Result<[u32; 2]> {
        Ok(std::array::from_fn(|i| {
            self.input["facing"][i].as_u64().unwrap() as u32
        }))
    }
    fn actor_category(&self) -> Result<bool> {
        Ok(self.input["actor"].as_bool().unwrap())
    }
    fn wall_present(&self) -> Result<bool> {
        Ok(self.input["wall"].as_bool().unwrap())
    }
    fn session_mode(&self) -> Result<i32> {
        Ok(self.word("session") as i32)
    }
    fn world_word_48(&self) -> Result<u32> {
        Ok(self.word("world48"))
    }
    fn height_margin_bits(&self) -> Result<u32> {
        Ok(self.word("slope"))
    }
    fn ground_height(&mut self, position: [u32; 2], flags: u32) -> Result<Binary> {
        let mode = self.word("mode");
        let reference = self.word("reference");
        let valid = self.input["reference_valid"].as_bool().unwrap();
        let reference_height = self.word("reference_height");
        let height = self.input["height64"].as_u64().unwrap();
        actor_post_create_height(
            mode,
            || Ok(reference),
            |r| {
                assert_eq!(r, 7);
                Ok(if valid {
                    Some(Binary::from_f32_bits(reference_height)?)
                } else {
                    None
                })
            },
            || {
                self.event(json!({"op":"ground_height","position":position,"flags":flags}))?;
                Ok(Binary::from_f64_bits(height)?)
            },
        )
    }
    fn edit_height(&mut self, edit: HeightEdit) -> Result<()> {
        self.event(json!({"op":"edit_height","edit":edit}))
    }
    fn clear_doodads(&mut self, placement: Placement) -> Result<()> {
        self.event(json!({"op":"clear_doodads","placement":placement}))
    }
    fn ground_variation(&mut self) -> Result<Option<u32>> {
        self.event(json!({"op":"ground_variation"}))?;
        Ok((self.word("variation") != 0).then_some(self.word("variation")))
    }
    fn footprint_padding_bits(&self) -> Result<u32> {
        Ok(self.word("padding"))
    }
    fn paint_terrain(&mut self, paint: TerrainPaint) -> Result<()> {
        self.event(json!({"op":"paint_terrain","paint":paint}))
    }
}
#[test]
fn native_nonnull_gacd_ground_orchestration_all_precisions_and_required_prefixes() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-object-ground/native-ground-nonnull.json"
    ))
    .unwrap();
    assert_eq!(f["rows"].as_array().unwrap().len(), 192);
    for row in f["rows"].as_array().unwrap() {
        let pc = match row["pc"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let mut world = Ground {
            input: row["input"].clone(),
            pc,
            events: vec![],
            stop: None,
        };
        object_effects_on_owners(&mut world).unwrap();
        assert_eq!(json!(world.events), row["native"]["events"], "{row}");
        for stop in ["edit_height", "clear_doodads", "paint_terrain"] {
            let Some(position) = world.events.iter().position(|v| v["op"] == stop) else {
                continue;
            };
            let mut prefix = Ground {
                input: row["input"].clone(),
                pc,
                events: vec![],
                stop: Some(stop),
            };
            assert!(object_effects_on_owners(&mut prefix).is_err());
            assert_eq!(prefix.events, world.events[..=position]);
        }
    }
}
#[test]
fn non_mode6_never_reads_reference_and_zero_never_resolves() {
    for mode in [0, 1, 2, 3, 4, 5, u32::MAX] {
        assert_eq!(
            actor_post_create_height(
                mode,
                || panic!("unreached+4C"),
                |_| panic!("unreachedGTID"),
                || Ok(Binary::from_i32(3))
            )
            .unwrap()
            .store_f32()
            .unwrap(),
            3f32.to_bits()
        );
    }
    actor_post_create_height(
        6,
        || Ok(0),
        |_| panic!("null reference"),
        || Ok(Binary::from_i32(3)),
    )
    .unwrap();
}
