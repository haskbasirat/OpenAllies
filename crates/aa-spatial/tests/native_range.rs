use aa_numeric::Precision;
use aa_spatial::{
    collision::Shape,
    geometry::Placement,
    range::{self, ActorRangeWorld, ActorTransform},
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
    wrappers: Vec<Wrapper>,
}
#[derive(Clone, Deserialize)]
struct Case {
    operation: String,
    precision: Precision,
    a: Placement,
    b: Placement,
    range_bits: u32,
    inverted: bool,
    result: bool,
}
#[derive(Clone, Deserialize)]
struct Mutation {
    on_actor: u32,
    actor: u32,
    position: Option<[u32; 2]>,
    radius_bits: Option<u32>,
}
#[derive(Deserialize)]
struct Wrapper {
    operation: String,
    precision: Precision,
    a: Placement,
    b: Placement,
    range_bits: u32,
    inverted: bool,
    result: bool,
    mutations: Vec<Mutation>,
    events: Vec<serde_json::Value>,
}
fn fixture() -> Fixture {
    serde_json::from_str(include_str!(
        "../../../research/phase7-range/native-range.json"
    ))
    .unwrap()
}
#[test]
fn native_range_helpers_match_all_shapes_edges_and_precisions() {
    for (index, c) in fixture().cases.into_iter().enumerate() {
        let actual = match c.operation.as_str() {
            "shapes" => range::shapes_in_range(&c.a, &c.b, c.range_bits, c.inverted, c.precision),
            "shape_point" => range::shape_point_in_range(
                &c.a,
                c.b.position,
                c.range_bits,
                c.inverted,
                c.precision,
            ),
            "point_delta" => {
                range::point_delta_in_range(c.b.position, c.range_bits, c.inverted, c.precision)
            }
            _ => panic!("Unexpected native operation"),
        }
        .unwrap();
        assert_eq!(
            actual, c.result,
            "native range vector {index}: {}",
            c.operation
        );
    }
}
struct World {
    actors: [Placement; 2],
    mutations: Vec<Mutation>,
    events: Vec<serde_json::Value>,
}
impl ActorRangeWorld for World {
    type ShapeHandle = u32;
    fn snapshot_transform(&mut self, id: u32) -> aa_spatial::Result<ActorTransform> {
        let actor = &self.actors[id as usize - 1];
        let snapshot = ActorTransform {
            position: actor.position,
            facing: actor.facing,
        };
        self.events.push(serde_json::json!({"kind":"snapshot","actor":id,"position":snapshot.position,"facing":snapshot.facing}));
        Ok(snapshot)
    }
    fn range_shape(&mut self, id: u32) -> aa_spatial::Result<u32> {
        self.events
            .push(serde_json::json!({"kind":"getter","actor":id}));
        for mutation in &self.mutations {
            if mutation.on_actor == id {
                let actor = &mut self.actors[mutation.actor as usize - 1];
                if let Some(position) = mutation.position {
                    actor.position = position;
                }
                if let Some(radius) = mutation.radius_bits {
                    actor.shape.radius_bits = radius;
                }
            }
        }
        Ok(id)
    }
    fn read_shape(&self, handle: &u32) -> aa_spatial::Result<Shape> {
        Ok(self.actors[*handle as usize - 1].shape.clone())
    }
}
#[test]
fn native_actor_wrappers_snapshot_before_getters_and_preserve_shape_aliases() {
    for c in fixture().wrappers {
        let mut world = World {
            actors: [c.a, c.b.clone()],
            mutations: c.mutations,
            events: Vec::new(),
        };
        let actual = match c.operation.as_str() {
            "actors" => {
                range::actors_in_range(1, 2, c.range_bits, c.inverted, c.precision, &mut world)
            }
            "actor_point" => range::actor_point_in_range(
                1,
                c.b.position,
                c.range_bits,
                c.inverted,
                c.precision,
                &mut world,
            ),
            _ => panic!("Unknown wrapper"),
        }
        .unwrap();
        assert_eq!(actual, c.result);
        assert_eq!(world.events, c.events);
    }
}
#[test]
fn invalid_range_inputs_are_errors_not_invented_distance_results() {
    let c = fixture().cases.remove(0);
    assert!(range::shapes_in_range(&c.a, &c.b, f32::NAN.to_bits(), false, c.precision).is_err());
    let mut bad = c.a;
    bad.shape.kind = 4;
    assert!(range::shapes_in_range(&bad, &c.b, 0, false, c.precision).is_err());
}
