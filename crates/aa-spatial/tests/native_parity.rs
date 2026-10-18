use aa_numeric::{Binary, Precision};
use aa_spatial::*;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Deserialize)]
struct Input {
    name: String,
    operation: String,
    precision: Precision,
    actor: Actor,
    candidate: [u32; 3],
    grid: TerrainGrid,
    query_flags: u32,
    phase: u32,
    presentation_target: u32,
    height_base: [u32; 2],
    height_overlay: [u32; 2],
    shape_returns: Vec<u32>,
    mutations: Vec<Value>,
}
#[derive(Deserialize)]
struct Fixture {
    input: Input,
    output: Value,
}
#[derive(Deserialize)]
struct Document {
    passed: bool,
    case_count: usize,
    fixtures: Vec<Fixture>,
}
fn document() -> Document {
    serde_json::from_str(include_str!(
        "../../../research/phase5-spatial/native-spatial-fixtures.json"
    ))
    .unwrap()
}
struct Host {
    input: Input,
    events: Vec<Value>,
    shape_index: usize,
}
impl Host {
    fn event(&mut self, actor: &mut Actor, kind: &str, extra: Value) {
        let mut row = json!({"kind":kind,"actor":actor});
        row.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        self.events.push(row);
        for m in &self.input.mutations {
            if m["event_index"].as_u64().unwrap() as usize == self.events.len() - 1 {
                if let Some(fields) = m.get("actor") {
                    let mut a = serde_json::to_value(&*actor).unwrap();
                    a.as_object_mut()
                        .unwrap()
                        .extend(fields.as_object().unwrap().clone());
                    *actor = serde_json::from_value(a).unwrap();
                }
                if let Some(v) = m.get("phase") {
                    self.input.phase = v.as_u64().unwrap() as u32;
                }
                if let Some(v) = m.get("presentation_target") {
                    self.input.presentation_target = v.as_u64().unwrap() as u32;
                }
            }
        }
    }
}
impl World for Host {
    fn sample_height(
        &mut self,
        actor: &mut Actor,
        layer: HeightLayer,
        method: HeightMethod,
        position: [u32; 2],
        pc: Precision,
    ) -> Result<Binary> {
        self.event(
            actor,
            "height",
            json!({"layer":layer,"method":method,"position":position}),
        );
        let pair = match layer {
            HeightLayer::Base => self.input.height_base,
            HeightLayer::Overlay => self.input.height_overlay,
        };
        Ok(Binary::from_f32_bits(pair[0])?.add(&Binary::from_f32_bits(pair[1])?, pc)?)
    }
    fn shape(&mut self, actor: &mut Actor, kind: ShapeKind) -> Result<u32> {
        let result = self.input.shape_returns[self.shape_index];
        self.shape_index += 1;
        self.event(actor, "shape", json!({"shape_kind":kind,"result":result}));
        Ok(result)
    }
    fn update_tiles(
        &mut self,
        actor: &mut Actor,
        old_b: ShapePlacement,
        new_b: ShapePlacement,
        old_a: ShapePlacement,
        new_a: ShapePlacement,
    ) -> Result<()> {
        self.event(
            actor,
            "tiles",
            json!({"old_b":old_b,"new_b":new_b,"old_a":old_a,"new_a":new_a}),
        );
        Ok(())
    }
    fn position_changed(&mut self, actor: &mut Actor) -> Result<()> {
        self.event(actor, "position", json!({}));
        Ok(())
    }
    fn world_phase(&self) -> u32 {
        self.input.phase
    }
    fn presentation_available(&self) -> bool {
        self.input.presentation_target != 0
    }
    fn presentation_changed(&mut self, actor: &mut Actor) -> Result<()> {
        self.event(
            actor,
            "presentation",
            json!({"target":self.input.presentation_target}),
        );
        Ok(())
    }
}

#[test]
fn original_x86_query_snap_and_commit_fixtures() {
    let d = document();
    assert!(d.passed);
    assert_eq!(d.case_count, 504);
    for f in d.fixtures {
        let i = f.input;
        let mut a = i.actor.clone();
        let mut candidate = i.candidate;
        let mut host = Host {
            input: i.clone(),
            events: Vec::new(),
            shape_index: 0,
        };
        let height = match i.operation.as_str() {
            "query" => Some(
                query_actor_height(&mut a, i.query_flags, &i.grid, i.precision, &mut host)
                    .unwrap()
                    .store_f32()
                    .unwrap(),
            ),
            "move" => {
                move_ground(
                    &mut a,
                    [candidate[0], candidate[1]],
                    &i.grid,
                    i.precision,
                    &mut host,
                )
                .unwrap();
                None
            }
            "commit" => {
                commit_position(&mut a, candidate, &mut host).unwrap();
                None
            }
            "snap" => {
                let mut xy = [candidate[0], candidate[1]];
                snap_half_cell(&mut xy, i.grid.cell_size, i.precision).unwrap();
                candidate[..2].copy_from_slice(&xy);
                None
            }
            other => panic!("unknown operation {other}"),
        };
        let actual = json!({"actor":a,"phase":host.input.phase,"presentation_target":host.input.presentation_target,
            "events":host.events,"height_bits":height,"candidate":candidate,"abi_preserved":true});
        assert_eq!(actual, f.output, "{} {:?}", i.name, i.precision);
    }
}

#[test]
fn undefined_height_and_invalid_grids_reject_explicitly() {
    let i = document().fixtures.remove(0).input;
    let mut host = Host {
        input: i.clone(),
        events: Vec::new(),
        shape_index: 0,
    };
    let mut actor = i.actor.clone();
    actor.flags = 1 << 12;
    let before = actor.clone();
    assert!(matches!(
        move_ground(&mut actor, [0, 0], &i.grid, i.precision, &mut host),
        Err(Error::UninitializedNativeHeight)
    ));
    assert_eq!(actor, before);
    assert!(host.events.is_empty());
    let mut bad = i.grid.clone();
    bad.width = 0;
    assert!(matches!(bad.validate(), Err(Error::InvalidGrid)));
    bad = i.grid.clone();
    bad.cells.pop();
    assert!(matches!(bad.validate(), Err(Error::InvalidGrid)));
    bad = i.grid.clone();
    bad.width = i32::MAX;
    bad.height = 2;
    assert!(matches!(bad.validate(), Err(Error::InvalidGrid)));
}

#[test]
fn invalid_snap_domain_and_nonfinite_commit_reject_without_silent_geometry() {
    for size in [0, 0x8000_0000, (-1.0f32).to_bits(), 1] {
        let mut p = [1.0f32.to_bits(), 2.0f32.to_bits()];
        assert!(snap_half_cell(&mut p, size, Precision::Double53).is_err());
    }
    let mut p = [f32::INFINITY.to_bits(), 0];
    assert!(snap_half_cell(&mut p, 1.0f32.to_bits(), Precision::Double53).is_err());
    let i = document().fixtures.remove(0).input;
    let mut a = i.actor.clone();
    let before = a.clone();
    let mut host = Host {
        input: i,
        events: Vec::new(),
        shape_index: 0,
    };
    assert!(commit_position(&mut a, [f32::NAN.to_bits(), 0, 0], &mut host).is_err());
    assert_eq!(a, before);
    assert!(host.events.is_empty());
}

struct FailingWorld {
    fail_at: &'static str,
    events: Vec<&'static str>,
}
impl FailingWorld {
    fn boundary(&mut self, kind: &'static str) -> Result<()> {
        self.events.push(kind);
        if self.fail_at == kind {
            Err(Error::World(format!("injected {kind} boundary failure")))
        } else {
            Ok(())
        }
    }
}
impl World for FailingWorld {
    fn sample_height(
        &mut self,
        _: &mut Actor,
        _: HeightLayer,
        _: HeightMethod,
        _: [u32; 2],
        _: Precision,
    ) -> Result<Binary> {
        Err(Error::World("unexpected height sampler in commit".into()))
    }
    fn shape(&mut self, _: &mut Actor, _: ShapeKind) -> Result<u32> {
        self.boundary("shape")?;
        Ok(1)
    }
    fn update_tiles(
        &mut self,
        _: &mut Actor,
        _: ShapePlacement,
        _: ShapePlacement,
        _: ShapePlacement,
        _: ShapePlacement,
    ) -> Result<()> {
        self.boundary("tiles")
    }
    fn position_changed(&mut self, _: &mut Actor) -> Result<()> {
        self.boundary("position")
    }
    fn world_phase(&self) -> u32 {
        1
    }
    fn presentation_available(&self) -> bool {
        true
    }
    fn presentation_changed(&mut self, _: &mut Actor) -> Result<()> {
        self.boundary("presentation")
    }
}

#[test]
fn world_failures_stop_later_callbacks_without_claiming_rollback() {
    let old_position = [0; 3];
    let new_position = [1.0f32.to_bits(), 2.0f32.to_bits(), 3.0f32.to_bits()];
    for fail_at in ["tiles", "position", "presentation"] {
        let mut actor = Actor {
            flags: 1 << 26,
            position: old_position,
            facing: [0; 2],
        };
        let mut world = FailingWorld {
            fail_at,
            events: Vec::new(),
        };
        assert!(matches!(
            commit_position(&mut actor, new_position, &mut world),
            Err(Error::World(_))
        ));
        let mut expected = vec!["shape", "shape", "shape", "shape", "tiles"];
        if fail_at != "tiles" {
            expected.push("position");
        }
        if fail_at == "presentation" {
            expected.push("presentation");
        }
        assert_eq!(world.events, expected);
        assert_eq!(
            actor.position,
            if fail_at == "tiles" {
                old_position
            } else {
                new_position
            }
        );
    }
}
