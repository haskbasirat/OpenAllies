use aa_numeric::Precision;
use aa_spatial::{
    Error, Result,
    collision::Shape,
    creation_placement::{self as cp, Arguments, Context, World},
    geometry::Placement,
};
use serde_json::{Value, json};

fn n(v: &Value, k: &str) -> u32 {
    v[k].as_u64().unwrap() as u32
}
fn b(v: &Value, k: &str) -> bool {
    v[k].as_bool().unwrap()
}
fn pair(v: &Value) -> [u32; 2] {
    [v[0].as_u64().unwrap() as u32, v[1].as_u64().unwrap() as u32]
}
fn shape(v: &Value) -> Shape {
    Shape {
        kind: v[0].as_u64().unwrap() as u32,
        radius_bits: v[1].as_u64().unwrap() as u32,
        bounds: std::array::from_fn(|i| v[i + 2].as_u64().unwrap() as u32),
    }
}
struct Probe {
    input: Value,
    events: Vec<Value>,
    context: Value,
}
impl Probe {
    fn event(&mut self, name: &str, args: Vec<Value>) -> Result<()> {
        let mut e = vec![json!(name)];
        e.extend(args);
        self.events.push(json!(e));
        if self.input["failure"] == name {
            Err(Error::World(name.into()))
        } else {
            Ok(())
        }
    }
}
impl World for Probe {
    type Actor = u32;
    type Definition = u32;
    type Zone = u32;
    type Tiles = usize;
    fn descriptor_definition(&mut self) -> Result<u32> {
        Ok(1)
    }
    fn descriptor_word(&mut self, o: u32) -> Result<u32> {
        Ok(match o {
            8 => pair(&self.input["position"])[0],
            0x28 => n(&self.input, "owner"),
            0x38 => n(&self.input, "mode38"),
            0x3c => n(&self.input, "explicit"),
            _ => {
                return Ok(match o {
                    0xc => pair(&self.input["position"])[1],
                    0x14 => pair(&self.input["facing"])[0],
                    0x18 => pair(&self.input["facing"])[1],
                    _ => panic!("descriptor {o:x}"),
                });
            }
        })
    }
    fn descriptor_byte(&mut self, o: u32) -> Result<u8> {
        assert_eq!(o, 0x52);
        Ok(n(&self.input, "byte52") as u8)
    }
    fn definition_virtual(&mut self, d: u32, slot: u32) -> Result<bool> {
        self.event("virtual", vec![json!(d), json!(slot)])?;
        let x = if d == 1 {
            n(
                &self.input,
                match slot {
                    0x30 => "object_data",
                    0x38 => "actor_data",
                    0x2c => "v2c",
                    0x3c => "v3c",
                    _ => panic!(),
                },
            )
        } else if d == 2 {
            n(&self.input, "other_v2c")
        } else {
            n(&self.input, "zone_v3c")
        };
        Ok(if slot == 0x2c || slot == 0x3c {
            x as u8 != 0
        } else {
            x != 0
        })
    }
    fn definition_word(&mut self, d: u32, o: u32) -> Result<u32> {
        assert_eq!(d, 1);
        Ok(n(
            &self.input,
            match o {
                0x1c => "flags",
                0x174 => "mask174",
                0x430 => "word430",
                0x444 => "word444",
                _ => panic!("word {o:x}"),
            },
        ))
    }
    fn definition_byte(&mut self, d: u32, o: u32) -> Result<u8> {
        assert_eq!(d, 1);
        Ok(n(
            &self.input,
            match o {
                0x1a8 => "byte1a8",
                0x290 => "byte290",
                _ => panic!(),
            },
        ) as u8)
    }
    fn definition_shape(&mut self, _: u32, blocking: bool, _: Precision) -> Result<Shape> {
        if !blocking {
            self.event("virtual_shape", vec![])?;
        }
        // Only the initial selection is logged by the native coordinator hook.
        if !self.events.iter().any(|v| v[0] == "shape") {
            self.event("shape", vec![json!(blocking)])?;
        }
        Ok(shape(
            &self.input[if blocking { "shape" } else { "virtual_shape" }],
        ))
    }
    fn definition_reference(&mut self, _: u32, o: u32) -> Result<Option<u32>> {
        assert_eq!(o, 0x4e0);
        Ok(b(&self.input, "required").then_some(2))
    }
    fn actor_definition(&mut self, a: u32) -> Result<u32> {
        Ok(if a == 3 { 3 } else { 2 })
    }
    fn actor_word(&mut self, _: u32, o: u32) -> Result<u32> {
        assert_eq!(o, 8);
        Ok(n(&self.input, "other_flags"))
    }
    fn map_settings_mode(&mut self, _: bool) -> Result<Option<u32>> {
        Ok(self.input["settings"].as_u64().map(|v| v as u32))
    }
    fn map_cell_size(&mut self) -> Result<u32> {
        Ok(4f32.to_bits())
    }
    fn outside_553370(&mut self, p: &Placement, pc: Precision) -> Result<bool> {
        aa_spatial::query::outside(p, pair(&self.input["extents"]), pc)
    }
    fn required_zone_4f1660(
        &mut self,
        pos: [u32; 2],
        d: u32,
        o: u32,
        a: Option<u32>,
        a7: u32,
        a8: u32,
        _: Precision,
    ) -> Result<u32> {
        self.event(
            "required",
            vec![
                json!(pos),
                json!(d),
                json!(o),
                json!(a.unwrap_or(0)),
                json!(a7),
                json!(a8),
            ],
        )?;
        Ok(n(&self.input, "validation"))
    }
    fn begin_tiles(&mut self, p: &Placement, _: Precision) -> Result<usize> {
        self.event("begin", vec![json!(p.position)])?;
        Ok(0)
    }
    fn tile_zone_54ef80(&mut self, t: &usize) -> Result<Option<u32>> {
        self.event("tile_zone", vec![json!(t)])?;
        Ok(self.input["zones"][*t].as_bool().unwrap().then_some(3))
    }
    fn advance_tile(&mut self, t: &mut usize) -> Result<bool> {
        self.event("advance", vec![])?;
        *t += 1;
        Ok(*t < self.input["zones"].as_array().unwrap().len())
    }
    fn tiles_valid(&mut self, t: &usize) -> Result<bool> {
        Ok(*t < self.input["zones"].as_array().unwrap().len())
    }
    fn zone_actor(&mut self, _: u32) -> Result<u32> {
        Ok(3)
    }
    fn zone_overlap_4f0ae0(&mut self, a: u32, _: &Placement, _: Precision) -> Result<bool> {
        self.event("zone_overlap", vec![json!(a)])?;
        Ok(n(&self.input, "overlap") as u8 != 0)
    }
    fn actor_category_id(&mut self, _: u32) -> Result<u32> {
        Ok(n(&self.input, "zone_category"))
    }
    fn structure_at_4f2220(&mut self, pos: [u32; 2]) -> Result<Option<u32>> {
        self.event("structure", vec![json!(pos)])?;
        Ok(b(&self.input, "structure").then_some(3))
    }
    fn collision_4f0a70(&mut self, c: &mut Context<u32, u32>, _: u32, _: Precision) -> Result<u32> {
        self.context = json!({"definition":c.definition,"source_actor":c.source_actor.unwrap_or(0),"ignore_actor":c.ignore_actor.unwrap_or(0),
            "ignore_definition":c.ignore_definition.unwrap_or(0),"owner":c.hostility_context,"custom_filter":c.custom_filter,"ignore_all":c.ignore_all,"ignore_moving":c.ignore_moving,
            "required":c.required_actor_flags,"mask":c.mask,"shape":[c.placement.shape.kind,c.placement.shape.radius_bits,
                c.placement.shape.bounds[0],c.placement.shape.bounds[1],c.placement.shape.bounds[2],c.placement.shape.bounds[3]],
            "facing":c.placement.facing,"position":c.placement.position});
        self.event("collision", vec![self.context.clone()])?;
        Ok(n(&self.input, "collision"))
    }
    fn footprint_4f0b70(&mut self) -> Result<u32> {
        self.event("footprint", vec![])?;
        Ok(n(&self.input, "footprint"))
    }
}

#[test]
fn original_v14_creation_placement_coordinator() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-placement/native-creation-placement.json"
    ))
    .unwrap();
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 363);
    for row in fixture["cases"].as_array().unwrap() {
        let c = &row["input"];
        let p = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let mut w = Probe {
            input: c.clone(),
            events: vec![],
            context: Value::Null,
        };
        let result = cp::check(
            Arguments {
                source: b(c, "source").then_some(1),
                other: b(c, "other").then_some(2),
                argument_5: n(c, "arg5"),
                argument_6: n(c, "arg6"),
                argument_7: n(c, "arg7"),
                argument_8: match n(c, "last") {
                    0 => None,
                    a => Some(a),
                },
            },
            p,
            &mut w,
        );
        let (result, outcome) = match result {
            Ok(v) => (json!(v), "ok".to_string()),
            Err(Error::World(v)) => (Value::Null, v),
            Err(e) => panic!("{e}"),
        };
        assert_eq!(
            json!({"result":result,"outcome":outcome,"events":w.events,"context":w.context}),
            row["output"],
            "{} {}",
            row["precision"],
            c["label"]
        );
    }
}

#[test]
fn original_v14_mode_zero_exact_shape_tile_filter() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-placement/native-placement-tiles.json"
    ))
    .unwrap();
    assert_eq!(f["cases"].as_array().unwrap().len(), 432);
    for row in f["cases"].as_array().unwrap() {
        let c = &row["input"];
        let p = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let grid = aa_spatial::collision::Grid {
            dimensions: [8, 8],
            coordinate_shift: 2,
            cell_size_bits: 4f32.to_bits(),
        };
        let placed = Placement {
            shape: shape(&c["shape"]),
            facing: pair(&c["facing"]),
            position: pair(&c["position"]),
        };
        let cells = if n(c, "mode") == 0 {
            cp::candidate_tiles(
                &grid,
                [32f32.to_bits(); 2],
                &shape(&c["cell_shape"]),
                &placed,
                p,
            )
            .unwrap()
        } else {
            cp::occupancy_tiles(
                &grid,
                [32f32.to_bits(); 2],
                &shape(&c["global_point_shape"]),
                &placed,
                p,
            )
            .unwrap()
        };
        assert_eq!(json!(cells), row["output"]["cells"], "{row}");
    }
}
