//! Read-only fixture host, not a heightfield, collision solver or live game world.
//! Every required callback is supplied by the selected original-execution case.
use aa_numeric::{Binary, Precision};
use aa_spatial::*;
use anyhow::Context;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Deserialize)]
struct ActorPatch {
    flags: Option<u32>,
    position: Option<[u32; 3]>,
    facing: Option<[u32; 2]>,
}
#[derive(Clone, Deserialize)]
struct Mutation {
    event_index: usize,
    actor: Option<ActorPatch>,
    phase: Option<u32>,
    presentation_target: Option<u32>,
}
#[derive(Clone, Deserialize)]
struct Input {
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
    mutations: Vec<Mutation>,
}
struct Host {
    input: Input,
    events: Vec<Value>,
    shape_index: usize,
}
impl Host {
    fn event(&mut self, actor: &mut Actor, kind: &str, extra: Value) {
        let mut row = json!({"kind": kind, "actor": actor});
        if let (Some(row), Some(extra)) = (row.as_object_mut(), extra.as_object()) {
            row.extend(extra.clone());
        }
        self.events.push(row);
        for mutation in &self.input.mutations {
            if mutation.event_index != self.events.len() - 1 {
                continue;
            }
            if let Some(patch) = &mutation.actor {
                if let Some(value) = patch.flags {
                    actor.flags = value;
                }
                if let Some(value) = patch.position {
                    actor.position = value;
                }
                if let Some(value) = patch.facing {
                    actor.facing = value;
                }
            }
            if let Some(value) = mutation.phase {
                self.input.phase = value;
            }
            if let Some(value) = mutation.presentation_target {
                self.input.presentation_target = value;
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
            json!({"layer": layer, "method": method, "position": position}),
        );
        let pair = match layer {
            HeightLayer::Base => self.input.height_base,
            HeightLayer::Overlay => self.input.height_overlay,
        };
        Ok(Binary::from_f32_bits(pair[0])?.add(&Binary::from_f32_bits(pair[1])?, pc)?)
    }
    fn shape(&mut self, actor: &mut Actor, kind: ShapeKind) -> Result<u32> {
        let result = *self
            .input
            .shape_returns
            .get(self.shape_index)
            .ok_or_else(|| Error::World("fixture shape response exhausted".into()))?;
        self.shape_index += 1;
        self.event(
            actor,
            "shape",
            json!({"shape_kind": kind, "result": result}),
        );
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

pub fn check_spatial(document: &Value) -> anyhow::Result<Vec<Value>> {
    let samples = document["samples"]
        .as_array()
        .context("spatial samples missing")?;
    let mut rows = Vec::new();
    for sample in samples {
        let input: Input = serde_json::from_value(sample["fixture"]["input"].clone())?;
        let mut actor = input.actor.clone();
        let mut candidate = input.candidate;
        let mut host = Host {
            input: input.clone(),
            events: Vec::new(),
            shape_index: 0,
        };
        let height = match input.operation.as_str() {
            "query" => Some(
                query_actor_height(
                    &mut actor,
                    input.query_flags,
                    &input.grid,
                    input.precision,
                    &mut host,
                )?
                .store_f32()?,
            ),
            "move" => {
                move_ground(
                    &mut actor,
                    [candidate[0], candidate[1]],
                    &input.grid,
                    input.precision,
                    &mut host,
                )?;
                None
            }
            "commit" => {
                commit_position(&mut actor, candidate, &mut host)?;
                None
            }
            "snap" => {
                let mut xy = [candidate[0], candidate[1]];
                snap_half_cell(&mut xy, input.grid.cell_size, input.precision)?;
                candidate[..2].copy_from_slice(&xy);
                None
            }
            other => anyhow::bail!("unknown spatial fixture operation {other}"),
        };
        let actual = json!({"actor":actor,"phase":host.input.phase,
            "presentation_target":host.input.presentation_target,"events":host.events,
            "height_bits":height,"candidate":candidate});
        let mut expected = sample["fixture"]["output"].clone();
        // Native x86 ABI preservation was checked by the oracle. This Rust
        // diagnostic compares observable state/events, not an x86 ABI claim.
        expected
            .as_object_mut()
            .context("spatial expected output missing")?
            .remove("abi_preserved");
        rows.push(
            json!({"name":sample["name"],"source_index":sample["source_index"],
            "passed":actual==expected,"actual":actual,"expected":expected}),
        );
    }
    Ok(rows)
}
