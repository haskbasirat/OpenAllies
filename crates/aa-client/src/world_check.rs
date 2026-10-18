//! Asset-free comparison of independent world fragments with original x86 samples.
//! Fixture callbacks below are diagnostic boundaries, never live game services.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const SAMPLES: &[u8] = include_bytes!("../../../research/phase5-client/world-samples.json");
const SAMPLE_SHA: &str = "b1c0c5392ae72a20ec6c95eec6e7a078e2d63125792db6a4fb17bdc84de108af";

pub fn run() -> Result<Value> {
    ensure!(
        format!("{:x}", Sha256::digest(SAMPLES)) == SAMPLE_SHA,
        "Embedded world-fragment sample hash mismatch"
    );
    evaluate(&serde_json::from_slice(SAMPLES)?)
}

type Check = fn(&Value) -> Result<Vec<Value>>;

fn evaluate(bundle: &Value) -> Result<Value> {
    ensure!(
        bundle["schema"] == 1,
        "Unsupported world-fragment sample schema"
    );
    let components: [(&str, Check); 4] = [
        ("actors", actor_samples::check_actors),
        ("navigation", navigation_samples::check_navigation),
        ("orders", order_samples::check_orders),
        ("spatial", spatial_samples::check_spatial),
    ];
    let mut checks = Vec::new();
    let mut coverage = serde_json::Map::new();
    let mut names = std::collections::BTreeSet::new();
    for (area, check) in components {
        let mut rows = check(&bundle["components"][area])?;
        ensure!(!rows.is_empty(), "{area} sample bundle is empty");
        coverage.insert(area.to_owned(), json!(rows.len()));
        for row in &mut rows {
            let name = row["name"].as_str().context("Check name missing")?;
            let name = format!("{area}.{name}");
            ensure!(
                names.insert(name.clone()),
                "Duplicate world check name {name}"
            );
            row["name"] = json!(name);
            ensure!(row["passed"].is_boolean(), "Check pass result missing");
        }
        checks.extend(rows);
    }
    let passed = checks.iter().filter(|row| row["passed"] == true).count();
    Ok(json!({
        "schema": 1,
        "diagnostic": "phase5-world-fragments-check",
        "passed": passed == checks.len(),
        "passed_checks": passed,
        "check_count": checks.len(),
        "coverage": coverage,
        "sample_bundle_sha256": SAMPLE_SHA,
        "original_native_sha256": bundle["original_sha256"],
        "accepted_native_sha256": bundle["accepted_sha256"],
        "source_oracles": bundle["sources"],
        "checks": checks,
        "scope": [
            "Asset-free execution of Rust libraries against 24 selected original-instruction oracle records.",
            "Base Thing construction and identity, prepared navigation state transitions, raw command encoding/decoding, and spatial height/position callbacks are checked separately.",
            "Navigation and spatial callbacks are explicit diagnostic fixtures. No authoritative game world is advanced.",
            "Runtime diagnostics access no original assets, native executable, profile, GPU, GUI, network, or operating-system settings."
        ],
        "limits": [
            "This is a small integration sample, not the full native parity suites or a playable match.",
            "Base Thing construction does not implement the derived Actor factory, ownership, component factories, or publication into a live world.",
            "Navigation receives already prepared paths. No route solver, outer UAI scheduler, formations, collision world, or command-to-Mover shortcut is supplied.",
            "Command decode/encode does not resolve live registries, authorize a request, or execute it.",
            "Spatial height, shape, tile ownership, and presentation leaves are supplied fixture callbacks; no complete terrain or collision service is implied.",
            "Original saves, combat, AI, campaign progression, and multiplayer remain separate unfinished systems."
        ]
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_world_fragments_pass_without_assets_or_graphics() {
        let report = run().unwrap();
        assert_eq!(report["passed"], true);
        assert_eq!(report["check_count"], 24);
        for area in ["actors", "navigation", "orders", "spatial"] {
            assert_eq!(report["coverage"][area], 6);
        }
    }

    #[test]
    fn every_component_reports_a_changed_native_expectation() {
        let original: Value = serde_json::from_slice(SAMPLES).unwrap();
        for area in ["actors", "navigation", "orders", "spatial"] {
            let mut bundle = original.clone();
            let component = &mut bundle["components"][area];
            match area {
                "actors" => component["samples"][0]["rng_after"] = json!(1),
                "navigation" => component["fixtures"][0]["output"]["native_success"] = json!(false),
                "orders" => component["samples"][0]["fixture"]["command"]["word"] = json!(1),
                "spatial" => component["samples"][0]["fixture"]["output"]["height_bits"] = json!(0),
                _ => unreachable!(),
            }
            let report = evaluate(&bundle).unwrap();
            assert_eq!(report["passed"], false, "{area} mismatch went undetected");
            assert_eq!(report["passed_checks"], 23, "{area}");
        }
    }

    #[test]
    fn an_empty_component_cannot_produce_a_passing_report() {
        let mut bundle: Value = serde_json::from_slice(SAMPLES).unwrap();
        bundle["components"]["orders"]["samples"] = json!([]);
        assert!(evaluate(&bundle).is_err());
    }
}

mod actor_samples {
    // Bounded, source-pinned base constructor fixtures; not live map actor creation.
    pub fn check_actors(input: &serde_json::Value) -> anyhow::Result<Vec<serde_json::Value>> {
        use aa_actors::{ActorId, IdentityRegistry, creation::*};
        use aa_numeric::Precision;
        use aa_simulation::NativeRng;
        use serde_json::{Value, json};
        let mut rows = Vec::new();
        for sample in input["samples"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing actor samples"))?
        {
            let c = &sample["input"];
            let u = |value: &Value| -> anyhow::Result<u32> {
                Ok(u32::try_from(
                    value
                        .as_u64()
                        .ok_or_else(|| anyhow::anyhow!("invalid actor word"))?,
                )?)
            };
            let pc: Precision = serde_json::from_value(c["precision"].clone())?;
            let mut record = CreationRecord {
                definition: ThingDefinition {
                    key: (),
                    flags: u(&c["definition_flags"])?,
                },
                position_bits: serde_json::from_value(c["position_bits"].clone())?,
                direction_bits: serde_json::from_value(c["direction_bits"].clone())?,
                name_utf16: serde_json::from_value(c["name_utf16"].clone())?,
                saved_id: ActorId::from_raw(u(&c["saved_id"])?),
            };
            let mut rng = NativeRng::new(u(&c["rng_state"])?);
            let table =
                DirectionTable::original(serde_json::from_value(c["startup_precision"].clone())?);
            let thing = construct(&mut record, u(&c["world_begin_bits"])?, &mut rng, table, pc)?;
            let mut constructor = serde_json::to_value(&thing)?;
            constructor.as_object_mut().unwrap().remove("definition");
            constructor["id"] = json!(0);
            let mut registry = IdentityRegistry::new();
            let id = initialize_identity(&mut registry, record.saved_id, false, thing)?;
            let actual = json!({"constructor": constructor, "record_direction_after": record.direction_bits,
            "rng_after": rng.state, "registered_id": id.raw(), "active": registry.len()});
            let expected = json!({"constructor": sample["constructor"], "record_direction_after": sample["record_direction_after"],
            "rng_after": sample["rng_after"], "registered_id": sample["registered_id"], "active": sample["registry"]["active"]});
            rows.push(
                json!({"name": format!("base_actor_creation_{}", sample["source_case_index"]),
            "passed": actual == expected, "actual": actual, "expected": expected}),
            );
        }
        Ok(rows)
    }
}

mod navigation_samples {
    //! Pinned native navigation diagnostics. World callbacks below are fixture-only.
    //! This module does not implement routes, actor-world integration, or gameplay.
    use aa_navigation::*;
    use aa_numeric::Precision;
    use serde::Deserialize;
    use serde_json::{Value, json};

    #[derive(Clone, Deserialize)]
    struct Input {
        name: String,
        operation: String,
        precision: Precision,
        state: MoveState,
        actor: ActorState,
        source: OwnedPath,
        activation: Activation,
        speed_bits: u32,
        group_speed_minimum_bits: u32,
        catchup_speed_factor_bits: u32,
        random_u32: u32,
        special_state_active: bool,
        can_activate: bool,
        activate_result: bool,
        organization_ready: bool,
        single_node_placement: bool,
        actor_is_moving: bool,
        leader_is_moving: bool,
        formation_ready: bool,
        actor_or_leader_handle_valid: bool,
        begin_result: bool,
        #[serde(default)]
        mutations: std::collections::BTreeMap<String, Value>,
    }
    struct World {
        input: Input,
        events: Vec<Value>,
    }
    impl World {
        fn event(&mut self, name: &str, bits: &[u32]) {
            self.events.push(json!({"name":name,"bits":bits}));
        }
        fn mutate(&self, name: &str, state: &mut MoveState, actor: &mut ActorState) {
            let Some(m) = self.input.mutations.get(name) else {
                return;
            };
            if let Some(fields) = m.get("state") {
                let mut value = serde_json::to_value(&*state).unwrap();
                for (k, v) in fields.as_object().unwrap() {
                    value[k] = v.clone();
                }
                *state = serde_json::from_value(value).unwrap();
            }
            if let Some(fields) = m.get("actor") {
                let mut value = serde_json::to_value(&*actor).unwrap();
                for (k, v) in fields.as_object().unwrap() {
                    value[k] = v.clone();
                }
                *actor = serde_json::from_value(value).unwrap();
            }
            if let Some(kind) = m.get("first_kind") {
                state.path.nodes[0].words[4] = kind.as_u64().unwrap() as u32;
            }
        }
    }
    impl NavigationWorld for World {
        fn special_state_active(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool {
            self.event("SpecialStateActive", &[]);
            self.mutate("SpecialStateActive", state, actor);
            self.input.special_state_active
        }
        fn can_activate(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool {
            self.event("CanActivate", &[]);
            self.mutate("CanActivate", state, actor);
            self.input.can_activate
        }
        fn activate(&mut self, mode: u32, state: &mut MoveState, actor: &mut ActorState) -> bool {
            self.event("Activate", &[mode]);
            self.mutate("Activate", state, actor);
            self.input.activate_result
        }
        fn node_reached(&mut self, state: &mut MoveState, actor: &mut ActorState) {
            self.event("NodeReached", &[]);
            self.mutate("NodeReached", state, actor);
        }
        fn single_node_placement(
            &mut self,
            xy: [u32; 2],
            state: &mut MoveState,
            actor: &mut ActorState,
        ) -> bool {
            self.event("SingleNodePlacement", &xy);
            self.mutate("SingleNodePlacement", state, actor);
            self.input.single_node_placement
        }
        fn actor_is_moving(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool {
            self.event("ActorIsMoving", &[]);
            self.mutate("ActorIsMoving", state, actor);
            self.input.actor_is_moving
        }
        fn formation_ready(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool {
            self.event("FormationReady", &[]);
            self.mutate("FormationReady", state, actor);
            self.input.formation_ready
        }
        fn actor_or_leader_handle_valid(
            &mut self,
            state: &mut MoveState,
            actor: &mut ActorState,
        ) -> bool {
            self.event("ActorOrLeaderHandleValid", &[]);
            self.mutate("ActorOrLeaderHandleValid", state, actor);
            self.input.actor_or_leader_handle_valid
        }
        fn organization_ready(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool {
            self.event("OrganizationReady", &[]);
            self.mutate("OrganizationReady", state, actor);
            self.input.organization_ready
        }
        fn leader_is_moving(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool {
            self.event("LeaderIsMoving", &[]);
            self.mutate("LeaderIsMoving", state, actor);
            self.input.leader_is_moving
        }
        fn stop_action(&mut self, state: &mut MoveState, actor: &mut ActorState) {
            self.event("StopAction", &[]);
            self.mutate("StopAction", state, actor);
        }
        fn begin_mover(
            &mut self,
            r: MoverRequest,
            state: &mut MoveState,
            actor: &mut ActorState,
        ) -> bool {
            self.event(
                "BeginMover",
                &[
                    r.direction_bits[0],
                    r.direction_bits[1],
                    r.destination_bits[0],
                    r.destination_bits[1],
                    r.requested_speed_bits,
                    r.speed_factor_bits,
                ],
            );
            self.mutate("BeginMover", state, actor);
            self.input.begin_result
        }
        fn next_world_random_u32(&mut self, state: &mut MoveState, actor: &mut ActorState) -> u32 {
            self.event("RandomDraw", &[]);
            self.mutate("RandomDraw", state, actor);
            self.input.random_u32
        }
        fn schedule(&mut self, delay: u32, state: &mut MoveState, actor: &mut ActorState) {
            self.event("Schedule", &[delay, 0]);
            self.mutate("Schedule", state, actor);
        }
    }

    /// Run explicitly pinned original-executable samples through Rust leaf functions.
    /// `document` is the bundled oracle selection, not an untrusted user payload.
    pub fn check_navigation(document: &Value) -> anyhow::Result<Vec<Value>> {
        let fixtures = document["fixtures"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("navigation bundle requires fixtures"))?;
        let mut rows = Vec::with_capacity(fixtures.len());
        for row in fixtures {
            let c: Input = serde_json::from_value(row["input"].clone())?;
            let mut state = c.state.clone();
            let mut actor = c.actor.clone();
            let mut world = World {
                input: c.clone(),
                events: vec![],
            };
            let result = match c.operation.as_str() {
                "activate" => activate_move(&mut state, &mut actor, c.activation, &mut world),
                "move_event" | "formation_event" => move_path_event(
                    &mut state,
                    &mut actor,
                    &c.source,
                    c.speed_bits,
                    c.operation == "formation_event",
                    &mut world,
                )?,
                "action" => action_go(
                    &mut state,
                    &mut actor,
                    c.speed_bits,
                    ActionContext {
                        precision: c.precision,
                        group_speed_minimum_bits: c.group_speed_minimum_bits,
                        catchup_speed_factor_bits: c.catchup_speed_factor_bits,
                    },
                    &mut world,
                )?,
                operation => anyhow::bail!("unsupported pinned navigation sample {operation}"),
            };
            let actual = json!({
                "state": state, "actor": actor, "events": world.events,
                "native_success": result, "source_after": c.source,
            });
            let expected = json!({
                "state": row["output"]["state"],
                "actor": row["output"]["actor"],
                "events": row["output"]["events"],
                "native_success": row["output"]["native_success"],
                "source_after": row["output"]["source_after"],
            });
            rows.push(json!({"name": c.name, "passed": actual == expected,
            "actual": actual, "expected": expected}));
        }
        Ok(rows)
    }
}

mod order_samples {
    //! Fixture-only native command checks. Registry contents are supplied samples;
    //! successful decode/encode does not authorize or execute a gameplay command.
    use aa_orders::{Cursor, Position, commands::*};
    use anyhow::{Context, Result};
    use serde_json::{Value, json};

    pub fn check_orders(document: &Value) -> Result<Vec<Value>> {
        let samples = document["samples"]
            .as_array()
            .context("orders samples missing")?;
        let mut rows = Vec::new();
        for sample in samples {
            let input = &sample["fixture"];
            let raw: Vec<u8> = serde_json::from_value(input["raw"].clone())?;
            let start: Position = serde_json::from_value(input["start"].clone())?;
            let end: Position = serde_json::from_value(input["end"].clone())?;
            let counts: RegistrySizes = serde_json::from_value(input["counts"].clone())?;
            let expected_command: CommandWire = serde_json::from_value(input["command"].clone())?;
            let mut cursor = Cursor::with_bounds(&raw, start, end.absolute()?)?;
            let decoded = CommandWire::decode(
                &mut cursor,
                counts,
                DecodeBudget {
                    max_parameter_entries: 64,
                },
            )?;
            let mut writer = WireWriter::new(start)?;
            decoded.encode(&mut writer, counts)?;
            let actual = json!({"command": decoded, "read_end": cursor.position(),
            "write_end": writer.position(), "bytes": writer.bytes()});
            let expected = json!({"command": expected_command, "read_end": end,
            "write_end": end, "bytes": raw});
            rows.push(
                json!({"name": sample["name"], "source_index": sample["source_index"],
            "passed": actual == expected, "actual": actual, "expected": expected}),
            );
        }
        Ok(rows)
    }
}

mod spatial_samples {
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
}
