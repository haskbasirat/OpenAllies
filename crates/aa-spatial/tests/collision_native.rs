use aa_numeric::{Binary, Precision};
use aa_spatial::collision::{self, Actor, Descriptor, Grid, Node, Result, Shape, Tile, World};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Deserialize)]
struct ActorInput {
    definition: u32,
    flags: u32,
    position: [u32; 2],
    facing: [u32; 2],
    blocking_radius_bits: u32,
    shape: Shape,
}
#[derive(Clone, Deserialize)]
struct Definition {
    flags: u32,
    flag_1a8: bool,
}
#[derive(Clone, Deserialize)]
struct Input {
    name: String,
    operation: String,
    grid: Grid,
    tile: Tile,
    actors: Vec<ActorInput>,
    definitions: Vec<Definition>,
    nodes: Vec<Node>,
    descriptor: Option<Descriptor>,
    precision: Precision,
    custom_result: bool,
    source_result: bool,
    category_result: bool,
    allegiance_results: Vec<Option<u32>>,
    hostile_result: bool,
    moving_result: bool,
    outer_results: Vec<Option<u32>>,
    #[serde(default)]
    mutation: Value,
}
struct Host {
    input: Input,
    events: Vec<Value>,
    outer_count: usize,
    allegiance_count: usize,
}
impl Host {
    fn mutate(&mut self, event: &str, actor: u32, d: Option<&mut Descriptor>) {
        let change = &self.input.mutation;
        if change["event"].as_str() != Some(event) {
            return;
        }
        if let Some(d) = d
            && let Some(update) = change["descriptor"].as_object()
        {
            let mut value = serde_json::to_value(&*d).unwrap();
            for (key, v) in update {
                value[key] = v.clone();
            }
            *d = serde_json::from_value(value).unwrap();
        }
        if !change["position"].is_null() {
            self.input.actors[actor as usize - 1].position =
                serde_json::from_value(change["position"].clone()).unwrap();
        }
        if change.as_object().is_some_and(|o| o.contains_key("next")) {
            self.input.nodes[0].next = serde_json::from_value(change["next"].clone()).unwrap();
        }
    }
}
impl World for Host {
    fn tile(&self, _: u32) -> Result<Tile> {
        Ok(self.input.tile.clone())
    }
    fn node(&self, id: u32) -> Result<Node> {
        Ok(self.input.nodes[id as usize - 1].clone())
    }
    fn actor(&self, id: u32) -> Result<Actor> {
        let a = &self.input.actors[id as usize - 1];
        Ok(Actor {
            definition: a.definition,
            flags: a.flags,
            position: a.position,
            facing: a.facing,
        })
    }
    fn definition_flags(&self, id: u32) -> Result<u32> {
        Ok(self.input.definitions[id as usize - 1].flags)
    }
    fn definition_flag_1a8(&self, id: u32) -> Result<bool> {
        Ok(self.input.definitions[id as usize - 1].flag_1a8)
    }
    fn blocking_radius(&mut self, a: u32, d: Option<&mut Descriptor>) -> Result<Binary> {
        self.events.push(json!(["radius", a]));
        let radius = self.input.actors[a as usize - 1].blocking_radius_bits;
        self.mutate("radius", a, d);
        Ok(Binary::from_f32_bits(radius)?)
    }
    fn custom_filter(&mut self, a: u32, d: &mut Descriptor) -> Result<bool> {
        self.events.push(json!(["custom", a]));
        self.mutate("custom", a, Some(d));
        Ok(self.input.custom_result)
    }
    fn source_excludes(&mut self, s: u32, a: u32, d: &mut Descriptor) -> Result<bool> {
        self.events.push(json!(["source", s, a]));
        self.mutate("source", a, Some(d));
        Ok(self.input.source_result)
    }
    fn category_b4(&mut self, a: u32, d: &mut Descriptor) -> Result<bool> {
        self.events.push(json!(["category", a]));
        self.mutate("category", a, Some(d));
        Ok(self.input.category_result)
    }
    fn allegiance(&mut self, a: u32, d: &mut Descriptor) -> Result<Option<u32>> {
        self.events.push(json!(["allegiance", a]));
        let v = self.input.allegiance_results[self
            .allegiance_count
            .min(self.input.allegiance_results.len() - 1)];
        self.allegiance_count += 1;
        self.mutate("allegiance", a, Some(d));
        Ok(v)
    }
    fn hostility_dispatch(&self, _: u32) -> Result<u32> {
        Ok(1)
    }
    fn hostile(
        &mut self,
        _: u32,
        c: Option<u32>,
        other: Option<u32>,
        d: &mut Descriptor,
    ) -> Result<bool> {
        self.events.push(json!(["hostile", c, other]));
        self.mutate("hostile", 1, Some(d));
        Ok(self.input.hostile_result)
    }
    fn moving_exclusion(&mut self, a: u32, d: &mut Descriptor) -> Result<bool> {
        self.events.push(json!(["moving", a]));
        self.mutate("moving", a, Some(d));
        Ok(self.input.moving_result)
    }
    fn resolve_reference(&self, r: u32) -> Result<Option<u32>> {
        Ok((r != 0).then_some(r))
    }
    fn outer_actor(&mut self, a: u32, d: &mut Descriptor) -> Result<Option<u32>> {
        self.events.push(json!(["outer", a]));
        let v = self.input.outer_results[self.outer_count.min(self.input.outer_results.len() - 1)];
        self.outer_count += 1;
        self.mutate("outer", a, Some(d));
        Ok(v)
    }
    fn blocking_shape(&mut self, a: u32, d: Option<&mut Descriptor>) -> Result<Shape> {
        self.events.push(json!(["shape", a]));
        self.mutate("shape", a, d);
        Ok(self.input.actors[a as usize - 1].shape.clone())
    }
}

#[test]
fn native_collision_callbacks_shapes_and_mutations() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-collision/native-collision-fixtures.json"
    ))
    .unwrap();
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 399);
    for row in fixture["cases"].as_array().unwrap() {
        let input: Input = serde_json::from_value(row["input"].clone()).unwrap();
        let mut descriptor = input.descriptor.clone();
        let grid = input.grid.clone();
        let precision = input.precision;
        let label = format!("{} {:?}", input.name, precision);
        let operation = input.operation.clone();
        let mut host = Host {
            input,
            events: vec![],
            outer_count: 0,
            allegiance_count: 0,
        };
        let result = match operation.as_str() {
            "find" => json!(
                collision::find_blocker(1, &grid, descriptor.as_mut(), precision, &mut host)
                    .unwrap()
            ),
            "blocked" => json!(
                collision::blocked(1, &grid, descriptor.as_mut().unwrap(), precision, &mut host)
                    .unwrap()
            ),
            "excluded" => {
                json!(collision::excluded(1, descriptor.as_mut().unwrap(), &mut host).unwrap())
            }
            _ => unreachable!(),
        };
        assert_eq!(
            json!({"result":result,"events":host.events,"descriptor":descriptor}),
            row["output"],
            "{label}"
        );
    }
}

#[test]
fn malformed_geometry_is_reported_instead_of_assuming_clear() {
    let actor = Actor {
        definition: 1,
        flags: 0,
        position: [0; 2],
        facing: [0, (-1.0_f32).to_bits()],
    };
    let probe = collision::Probe {
        position: [0; 2],
        radius_bits: 1.0_f32.to_bits(),
    };
    let invalid = Shape {
        kind: 4,
        radius_bits: 0,
        bounds: [0; 4],
    };
    assert!(matches!(
        collision::overlaps(&invalid, &actor, &probe, Precision::Extended64),
        Err(collision::Error::Shape(4))
    ));
    let grid = Grid {
        dimensions: [8, 8],
        coordinate_shift: 1,
        cell_size_bits: 0,
    };
    assert!(matches!(
        collision::probe(&grid, [1, 1], Precision::Extended64),
        Err(collision::Error::Grid)
    ));
}
