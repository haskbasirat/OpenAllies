use aa_numeric::Precision;
use aa_spatial::{
    Error, Result,
    geometry::Placement,
    required_placement::{self as rp, Arguments, World},
};
use serde_json::{Value, json};

struct Replay {
    input: Value,
    events: Vec<Value>,
}
impl Replay {
    fn event(&mut self, name: &str, args: &[Value]) -> Result<()> {
        let mut event = vec![json!(name)];
        event.extend_from_slice(args);
        self.events.push(json!(event));
        if self.input["failure"] == name {
            return Err(Error::World(name.into()));
        }
        let mutation = self.input["mutation"].clone();
        if mutation["event"] == name
            && (mutation["args"].is_null() || mutation["args"] == json!(args))
        {
            let actor = mutation["actor"].as_u64().unwrap().to_string();
            for field in ["flags", "reference"] {
                if !mutation[field].is_null() {
                    self.input["actors"][&actor][field] = mutation[field].clone();
                }
            }
        }
        Ok(())
    }
    fn n(&self, name: &str) -> u32 {
        self.input[name].as_u64().unwrap() as u32
    }
    fn actor(&self, a: u32, name: &str) -> u32 {
        self.input["actors"][a.to_string()][name].as_u64().unwrap() as u32
    }
    fn definition_actor(&self, d: u32) -> u32 {
        (1..=4)
            .find(|a| self.actor(*a, "definition") == d)
            .unwrap_or(0)
    }
}
impl World for Replay {
    type Definition = u32;
    type Actor = u32;
    type Zone = u32;
    type Node = (usize, usize);
    type Tiles = usize;
    fn manager_radius_74(&mut self) -> Result<u32> {
        Ok(self.n("radius"))
    }
    fn outside_553370(&mut self, p: &Placement, pc: Precision) -> Result<bool> {
        let extents = serde_json::from_value(self.input["extents"].clone()).unwrap();
        aa_spatial::query::outside(p, extents, pc)
    }
    fn definition_is_actor(&mut self, d: u32) -> Result<bool> {
        self.event("definition_actor", &[json!(d)])?;
        Ok(self.n("definition_actor") != 0)
    }
    fn definition_word(&mut self, d: u32, o: u32) -> Result<u32> {
        Ok(match o {
            0x174 => {
                if d == 1 {
                    self.n("mask")
                } else {
                    0
                }
            }
            0x430 => {
                let a = self.definition_actor(d);
                if a == 0 { 0 } else { self.actor(a, "word430") }
            }
            _ => panic!("{o:x}"),
        })
    }
    fn definition_byte(&mut self, d: u32, o: u32) -> Result<u8> {
        assert_eq!(o, 0x1a8);
        Ok(self.actor(self.definition_actor(d), "byte1a8") as u8)
    }
    fn definition_reference(&mut self, d: u32, o: u32) -> Result<Option<u32>> {
        Ok(match o {
            0x464 => Some(2),
            0x4e0 => {
                if d == if self.input["settlement"].as_bool().unwrap() {
                    2
                } else {
                    1
                } {
                    let n = self.n("required");
                    (n != 0).then_some(n)
                } else {
                    None
                }
            }
            _ => panic!("{o:x}"),
        })
    }
    fn actor_virtual(&mut self, a: u32, o: u32) -> Result<bool> {
        self.event("virtual", &[json!(a), json!(o)])?;
        Ok(self.actor(
            a,
            match o {
                0xb0 => "b0",
                0xb4 => "b4",
                _ => panic!("{o:x}"),
            },
        ) as u8
            != 0)
    }
    fn actor_link(&mut self, a: u32, o: u32) -> Result<Option<u32>> {
        self.event("virtual", &[json!(a), json!(o)])?;
        let n = self.actor(
            a,
            match o {
                0x15c => "parent15c",
                0x100 => "parent100",
                _ => panic!("{o:x}"),
            },
        );
        Ok((n != 0).then_some(n))
    }
    fn actor_reference_50(&mut self, a: u32) -> Result<u32> {
        Ok(self.actor(a, "reference"))
    }
    fn resolve_reference(&mut self, r: u32) -> Result<Option<u32>> {
        let e = &self.input["registry"][r.to_string()];
        if e.is_null() || e["generation_delta"] != 0 {
            return Ok(None);
        }
        let a = e["actor"].as_u64().unwrap() as u32;
        Ok((a != 0).then_some(a))
    }
    fn actor_definition(&mut self, a: u32) -> Result<u32> {
        Ok(self.actor(a, "definition"))
    }
    fn actor_flags(&mut self, a: u32) -> Result<u32> {
        Ok(self.actor(a, "flags"))
    }
    fn begin_tiles(&mut self, _: &Placement, _: Precision) -> Result<usize> {
        self.event("begin", &[])?;
        Ok(0)
    }
    fn tiles_valid(&mut self, t: &usize) -> Result<bool> {
        Ok(*t < self.input["tiles"].as_array().unwrap().len())
    }
    fn advance_tile(&mut self, t: &mut usize) -> Result<()> {
        self.event("advance", &[])?;
        *t += 1;
        Ok(())
    }
    fn game_mode_e4(&mut self) -> Result<u32> {
        Ok(self.n("game_mode"))
    }
    fn current_thread_is_simulation(&mut self) -> Result<bool> {
        self.event("thread", &[])?;
        Ok(self.input["simulation_thread"].as_bool().unwrap())
    }
    fn tile_zone_54ef80(
        &mut self,
        t: &usize,
        a: Option<u32>,
        d: Option<u32>,
    ) -> Result<Option<u32>> {
        self.event(
            "zone",
            &[json!(t), json!(a.unwrap_or(0)), json!(d.unwrap_or(0))],
        )?;
        Ok(self.input["tiles"][t]["zone"]
            .as_bool()
            .unwrap()
            .then_some(3))
    }
    fn zone_actor(&mut self, z: u32) -> Result<u32> {
        Ok(z)
    }
    fn zone_overlap_4f0ae0(&mut self, a: u32, _: &Placement, _: Precision) -> Result<bool> {
        self.event("zone_overlap", &[json!(a)])?;
        Ok(self.n("zone_overlap") as u8 != 0)
    }
    fn tile_terrain_flags_4c(&mut self, t: &usize) -> Result<u8> {
        self.event("terrain", &[json!(t)])?;
        Ok(self.input["tiles"][t]["land"].as_u64().unwrap() as u8)
    }
    fn first_node(&mut self, t: &usize) -> Result<Option<Self::Node>> {
        Ok((!self.input["tiles"][t]["nodes"]
            .as_array()
            .unwrap()
            .is_empty())
        .then_some((*t, 0)))
    }
    fn node_actor(&mut self, n: Self::Node) -> Result<u32> {
        Ok(self.input["tiles"][n.0]["nodes"][n.1].as_u64().unwrap() as u32)
    }
    fn next_node(&mut self, n: Self::Node) -> Result<Option<Self::Node>> {
        Ok(
            (n.1 + 1 < self.input["tiles"][n.0]["nodes"].as_array().unwrap().len())
                .then_some((n.0, n.1 + 1)),
        )
    }
    fn actor_overlap(&mut self, a: u32, _: &Placement, _: Precision) -> Result<bool> {
        self.event("virtual", &[json!(a), json!(0xc)])?;
        self.event("overlap", &[json!(a)])?;
        Ok(self.actor(a, "overlap") as u8 != 0)
    }
    fn actor_mobile_4d1120(&mut self, a: u32) -> Result<bool> {
        self.event("mobile", &[json!(a)])?;
        Ok(self.actor(a, "mobile") as u8 != 0)
    }
    fn actor_enemy_140(&mut self, a: u32, _: u32) -> Result<bool> {
        self.event("virtual", &[json!(a), json!(0x140)])?;
        Ok(self.actor(a, "enemy") as u8 != 0)
    }
    fn global_mode_b28158(&mut self) -> Result<u32> {
        Ok(self.n("global_mode"))
    }
}
#[test]
fn untouched_required_placement_rows_and_service_prefixes_match() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-required-placement/native-required-placement.json");
    let doc: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let rows = doc["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 177);
    for row in rows {
        let input = &row["input"];
        let p = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => unreachable!(),
        };
        let mut w = Replay {
            input: input.clone(),
            events: vec![],
        };
        let other = w.n("other");
        let args = Arguments {
            position: serde_json::from_value(input["position"].clone()).unwrap(),
            definition: input["definition"].as_bool().unwrap().then_some(1),
            owner: w.n("owner"),
            other: (other != 0).then_some(other),
            argument_7: w.n("argument7"),
            argument_8: w.n("argument8"),
        };
        let (result, outcome) = match rp::check(args, p, &mut w) {
            Ok(v) => (Some(v), "ok".to_string()),
            Err(Error::World(e)) => (None, e),
            Err(e) => panic!("{e}"),
        };
        assert_eq!(
            json!({"result":result,"outcome":outcome,"events":w.events}),
            row["output"],
            "{} {}",
            row["precision"],
            input["label"]
        );
    }
}
