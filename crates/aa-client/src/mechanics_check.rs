//! Asset-independent integration diagnostic, not a simulation/gameplay driver.
//! Expected outputs are exact selected original-x86 oracle records. Host seams
//! below deliberately reproduce fixture callbacks, never game-world services.
use aa_actors::{components::*, registry::*};
use aa_body::{Body, Environment, Event as BodyEvent, F32, HealRequest, Host};
use aa_movement::{
    ActorMotion, Event as MotionEvent, GroundTurning, GroundWorld, MoverState, StepContext,
};
use aa_numeric::Precision;
use aa_orders::{native::*, *};
use aa_simulation::{NativeRng, native_unit_float};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

// The historical bundle missed native CRT initializer9E65F0 (order323).
// The successor changes only registry expectations/provenance, from the
// separately executed original/v14 registry audit; other samples are retained.
const SAMPLES: &[u8] = include_bytes!("../../../research/phase4-orders/mechanics-samples-registry358.json");
const SAMPLE_SHA: &str = "d18efc196ef2c26a307863e6cee4d6f50e06f9980044ec3e290f807160d7f8d0";
fn decode<T: DeserializeOwned>(v: &Value) -> Result<T> {
    Ok(serde_json::from_value(v.clone())?)
}
fn number(v: &Value) -> Result<u32> {
    u32::try_from(v.as_u64().context("Expected unsigned32 field")?)
        .context("Value exceeds unsigned32")
}
fn rows(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().context("Expected sample array")
}

#[derive(Default)]
struct Checks {
    rows: Vec<Value>,
}
impl Checks {
    fn equal(&mut self, name: impl Into<String>, actual: Value, expected: &Value) {
        self.rows.push(json!({"name":name.into(),"passed":actual==*expected,"actual":actual,"expected":expected}));
    }
}

pub fn run() -> Result<Value> {
    ensure!(
        format!("{:x}", Sha256::digest(SAMPLES)) == SAMPLE_SHA,
        "Embedded mechanics sample hash mismatch"
    );
    evaluate(&serde_json::from_slice(SAMPLES)?)
}
fn evaluate(bundle: &Value) -> Result<Value> {
    ensure!(bundle["schema"] == 1, "Unsupported mechanics sample schema");
    let mut checks = Checks::default();
    for r in rows(&bundle["rng"]["constructors"])? {
        let seed = number(&r["seed"])?;
        let state = if r["address"] == "0x85d870" {
            NativeRng::default()
        } else {
            NativeRng::from_native_seed(seed)
        };
        checks.equal(
            format!("rng.constructor.{}.{seed}", r["address"]),
            json!(state.state),
            &r["state"],
        );
    }
    for r in rows(&bundle["rng"]["bounded"])? {
        let mut state = NativeRng::new(number(&r["state_before"])?);
        let bound = number(&r["bound"])?;
        let value = state.next_bounded(bound)?;
        checks.equal(
            format!("rng.bounded.{bound}"),
            json!([value, state.state]),
            &json!([r["value"], r["state_after"]]),
        );
    }
    for r in rows(&bundle["rng"]["converted"])? {
        let precision: Precision = decode(&r["precision"])?;
        let value = number(&r["value"])?;
        checks.equal(
            format!("rng.float.{precision:?}.{value}"),
            json!(native_unit_float(value, precision)?.0),
            &r["result_bits"],
        );
    }
    checks.equal(
        "orders.registry_count",
        json!(DESCRIPTORS.len()),
        &bundle["order_registry"]["count"],
    );
    for r in rows(&bundle["order_registry"]["samples"])? {
        let id = number(&r["id"])? as usize;
        let d = DESCRIPTORS
            .get(id)
            .context("Descriptor sample out of range")?;
        checks.equal(
            format!("orders.descriptor.{id}"),
            json!({"id":d.id,"name":d.name,"handler":format!("0x{:x}",d.handler_va)}),
            &json!({"id":r["id"],"name":r["name"],"handler":r["handler"]}),
        );
    }
    for r in rows(&bundle["orders"])? {
        check_orders(r, &mut checks)?;
    }
    for r in rows(&bundle["actors"])? {
        check_registry(r, &mut checks)?;
    }
    for r in rows(&bundle["components"])? {
        check_components(r, &mut checks)?;
    }
    for r in rows(&bundle["body"])? {
        check_body(r, &mut checks)?;
    }
    for r in rows(&bundle["movement"])? {
        check_movement(r, &mut checks)?;
    }
    let passed = checks.rows.iter().filter(|c| c["passed"] == true).count();
    Ok(
        json!({"schema":1,"diagnostic":"phase4-mechanics-check","passed":passed==checks.rows.len(),"passed_checks":passed,"check_count":checks.rows.len(),"sample_bundle_sha256":SAMPLE_SHA,"source_oracles":bundle["source_oracles"],"original_native_sha256":bundle["original_sha256"],"accepted_native_sha256":bundle["accepted_native_sha256"],"checks":checks.rows,
      "scope":["Asset-independent compiled-library comparison with selected original-instruction oracle outputs.","RNG, descriptor/move decoding, identity transitions, component callback order, Body healing and straight/blocked Mover fragments are checked separately.","Orders, Body and movement use explicitly diagnostic callbacks; no authoritative world is advanced."],
      "limits":["This is a small integration sample, not the full native parity suites or a playable battle.","No pathfinding, actor factory, AI/combat, request authorization, command cadence or network/save protocol is implemented by this diagnostic.","Move command delivery is not connected directly to Mover stepping; the native path/action boundary remains missing.","No gradual turning, native FPU lifecycle or seed/draw scheduling claim. Precision is explicit in each sample.","No original assets, native executable, profile, GPU, GUI or operating-system settings are accessed or modified."]}),
    )
}

struct DiagnosticOrders {
    allowed: bool,
    actor_exists: bool,
    actor_allows: bool,
    events: Vec<Value>,
}
impl MoveContext for DiagnosticOrders {
    type Actor = ();
    fn can_process(&mut self) -> bool {
        self.events.push(json!({"kind":"can_process"}));
        self.allowed
    }
    fn resolve_actor(&mut self, s: &[u16]) -> Option<()> {
        self.events
            .push(json!({"kind":"resolve_actor","selector":s}));
        self.actor_exists.then_some(())
    }
    fn allows_command(&mut self, _: &(), t: u32) -> bool {
        self.events.push(json!({"kind":"allows_command","type":t}));
        self.actor_allows
    }
    fn deliver(&mut self, _: &(), t: u32, c: MoveCommand) {
        self.events.push(json!({"kind":"deliver","type":t,"destination_bits":c.destination_bits,"auxiliary_bits":c.auxiliary_bits,"word_28":c.word_28}));
    }
}
fn check_orders(r: &Value, checks: &mut Checks) -> Result<()> {
    let packet = Packet::new(decode(&r["packet"])?)?;
    let mut context = DiagnosticOrders {
        allowed: decode(&r["can_process"])?,
        actor_exists: decode(&r["actor_exists"])?,
        actor_allows: decode(&r["actor_allows"])?,
        events: vec![],
    };
    let mut decoded = vec![];
    let result = dispatch(
        Some(&packet),
        None,
        &registry()?,
        false,
        &mut Gate {
            session: false,
            admin: false,
        },
        |id, c, _| {
            let KnownPayload::MoveDecree(order) =
                decode_payload(id, c).map_err(|e| e.to_string())?
            else {
                return Err("Diagnostic must never apply a request as a decree".into());
            };
            decoded.push(json!({"selector":order.selector_utf16,"words":order.words}));
            process_move(&order, &mut context);
            Ok(())
        },
    )?;
    let expected:Vec<_>=rows(&r["events"])?.iter().map(|e| if e["kind"]=="deliver" {
        json!({"kind":"deliver","type":5,"destination_bits":[e["fields"]["0x18"],e["fields"]["0x1c"]],"auxiliary_bits":[e["fields"]["0x20"],e["fields"]["0x24"]],"word_28":e["fields"]["0x28"]})
    } else {e.clone()}).collect();
    checks.equal(
        format!("orders.{}", r["name"]),
        json!({"decoded":decoded,"events":context.events,"executed":result.executed}),
        &json!({"decoded":r["orders"],"events":expected,"executed":rows(&r["orders"])?.len()}),
    );
    Ok(())
}

fn registry_snapshot(r: &IdentityRegistry<u32>) -> Value {
    json!({"active":r.len(),"next_slot":r.next_slot(),"highest_slot":r.highest_slot(),
      "generations":(0..SLOT_COUNT).filter_map(|i|{let g=r.generation_at(i as u16);(g!=0).then_some([i as u32,u32::from(g)])}).collect::<Vec<_>>(),
      "entries":r.iter().map(|e|json!({"slot":e.id.slot(),"id":e.id.raw(),"key":e.value,"is_actor":e.is_actor})).collect::<Vec<_>>(),
      "occupied":r.occupied_words().iter().enumerate().filter_map(|(i,&v)|(v!=0).then_some([i as u32,v])).collect::<Vec<_>>()})
}
fn by_key(r: &IdentityRegistry<u32>, key: u32) -> Result<ActorId> {
    r.iter()
        .find(|e| e.value == key)
        .map(|e| e.id)
        .context("Diagnostic object missing")
}
fn check_registry(case: &Value, checks: &mut Checks) -> Result<()> {
    ensure!(
        case["initial"].as_object().is_some_and(|v| v.is_empty()),
        "Diagnostic requires native-reset registry input"
    );
    let mut registry = IdentityRegistry::new();
    for (i, step) in rows(&case["steps"])?.iter().enumerate() {
        let op = &step["operation"];
        let mut result = Value::Null;
        match op["op"].as_str().context("Missing registry operation")? {
            "allocate" => {
                result = json!(
                    registry
                        .allocate(decode(&op["is_actor"])?, number(&op["key"])?)?
                        .raw()
                )
            }
            "seed" => registry.seed_saved_generation(ActorId::from_raw(number(&op["id"])?))?,
            "register" => registry.register_saved(
                ActorId::from_raw(number(&op["id"])?),
                decode(&op["is_actor"])?,
                number(&op["key"])?,
            )?,
            "lookup" => {
                result = json!(
                    registry
                        .get_actor(ActorId::from_raw(number(&op["id"])?))
                        .map(|e| e.value)
                )
            }
            "remove" => {
                let id = by_key(&registry, number(&op["key"])?)?;
                registry.remove(id)?;
            }
            "swap" => {
                let a = by_key(&registry, number(&op["a"])?)?;
                let b = by_key(&registry, number(&op["b"])?)?;
                registry.swap_identities(a, b)?;
            }
            "enumerate" => {
                result = json!(
                    registry
                        .iter()
                        .map(|e| json!({"id":e.id.raw(),"key":e.value}))
                        .collect::<Vec<_>>()
                )
            }
            other => bail!("Unsupported diagnostic registry operation {other}"),
        }
        checks.equal(
            format!("actors.{}.{i}", case["name"]),
            json!({"result":result,"state":registry_snapshot(&registry),"error":null}),
            &json!({"result":step["result"],"state":step["state"],"error":step["error"]}),
        );
    }
    Ok(())
}
#[derive(Default)]
struct DiagnosticComponents {
    events: Vec<Value>,
}
impl LifecycleHooks<(), u32> for DiagnosticComponents {
    fn init_create(&mut self, s: ComponentSlot, _: &u32, _: &mut Components<()>) -> bool {
        self.events
            .push(json!({"unexpected":"init_create","slot":s.index()}));
        false
    }
    fn init_existing(&mut self, s: ComponentSlot, _: &mut Components<()>) -> bool {
        self.events
            .push(json!({"kind":"init_existing","slot":s.index()}));
        true
    }
    fn post_existing(&mut self, s: ComponentSlot, _: &mut Components<()>) {
        self.events
            .push(json!({"kind":"post_existing","slot":s.index()}));
    }
    fn post_create(&mut self, s: ComponentSlot, _: &u32, _: u32, _: &mut Components<()>) {
        self.events
            .push(json!({"unexpected":"post_create","slot":s.index()}));
    }
}
fn check_components(r: &Value, checks: &mut Checks) -> Result<()> {
    ensure!(
        r["input"]["mode"] == "existing",
        "Wrong lifecycle diagnostic"
    );
    let mut c = Components::new(StateBits { low: 0, high: 0 });
    for slot in rows(&r["input"]["present"])? {
        c.insert(
            ComponentSlot::from_index(number(slot)? as usize)
                .context("Invalid native component slot")?,
            (),
        );
    }
    let mut h = DiagnosticComponents::default();
    let result = c.init_existing(&mut h).is_ok();
    checks.equal("actors.components.existing_mind_last",json!({"result":result,"events":h.events,"present":c.iter().map(|(s,_)|s.index()).collect::<Vec<_>>() }),&json!({"result":r["result"],"events":r["events"],"present":r["present"]}));
    Ok(())
}

struct DiagnosticBody {
    events: Vec<Value>,
    mutations: Vec<Value>,
}
impl Host for DiagnosticBody {
    fn event(
        &mut self,
        event: BodyEvent,
        body: &mut Body,
        env: &mut Environment,
    ) -> std::result::Result<(), String> {
        self.events.push(json!({"event":event,"body":body}));
        let value = serde_json::to_value(&event).map_err(|e| e.to_string())?;
        let name = value
            .as_str()
            .map(str::to_owned)
            .or_else(|| value.as_object().and_then(|v| v.keys().next().cloned()))
            .ok_or("Invalid diagnostic event")?;
        if self.mutations.first().is_some_and(|m| m["on"] == name) {
            let mutation = self.mutations.remove(0);
            if let Some(changes) = mutation["body"].as_object() {
                let mut state = json!(&*body);
                for (k, v) in changes {
                    state[k] = v.clone();
                }
                *body = serde_json::from_value(state).map_err(|e| e.to_string())?;
            }
            if let Some(changes) = mutation["environment"].as_object() {
                let mut state = json!(&*env);
                for (k, v) in changes {
                    state[k] = v.clone();
                }
                *env = serde_json::from_value(state).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}
fn check_body(r: &Value, checks: &mut Checks) -> Result<()> {
    let mut body: Body = decode(&r["state"])?;
    let mut env: Environment = decode(&r["environment"])?;
    let mut host = DiagnosticBody {
        events: vec![],
        mutations: decode(&r["mutations"])?,
    };
    body.heal_plain(
        HealRequest {
            source_id: number(&r["source_id"])?,
            amount: F32(number(&r["amount"])?),
            token: number(&r["token"])?,
            converts_to_damage: false,
        },
        &mut env,
        &mut host,
        decode(&r["precision"])?,
    )?;
    checks.equal(
        "body.heal_callback_and_clock",
        json!({"state":body,"events":host.events,"mutations_remaining":host.mutations.len()}),
        &json!({"state":r["result"],"events":r["events"],"mutations_remaining":0}),
    );
    Ok(())
}

#[derive(Clone, Deserialize)]
struct MotionInput {
    name: String,
    mover: MoverState,
    actor: ActorMotion,
    context: StepContext,
    activation: bool,
    collision: bool,
    target_valid: bool,
    target_position: [f32; 2],
    shape_in_range: bool,
    notify_result: bool,
}
struct DiagnosticWorld {
    input: MotionInput,
    events: Vec<Value>,
}
impl DiagnosticWorld {
    fn record(&mut self, name: &str, bits: &[u32]) {
        self.events.push(json!({"name":name,"bits":bits}));
    }
}
impl GroundWorld for DiagnosticWorld {
    type Collision = ();
    fn activate(&mut self, _: &mut MoverState, _: &mut ActorMotion) -> aa_movement::Result<bool> {
        self.record("Activate", &[]);
        Ok(self.input.activation)
    }
    fn target_position(&mut self, _: u32) -> aa_movement::Result<Option<[f32; 2]>> {
        Ok(self
            .input
            .target_valid
            .then_some(self.input.target_position))
    }
    fn shapes_in_range(&mut self, _: &ActorMotion, _: u32, _: f32) -> aa_movement::Result<bool> {
        Ok(self.input.shape_in_range)
    }
    fn collision_at(&mut self, _: &ActorMotion, xy: [f32; 2]) -> aa_movement::Result<Option<()>> {
        self.record("CollisionQuery", &xy.map(f32::to_bits));
        Ok(self.input.collision.then_some(()))
    }
    fn set_facing(
        &mut self,
        _: &mut MoverState,
        a: &mut ActorMotion,
        xy: [f32; 2],
    ) -> aa_movement::Result<()> {
        self.record("SetFacing", &xy.map(f32::to_bits));
        if a.flags & (1 << 17) == 0 {
            a.facing = [xy[0], xy[1], 0.0];
        }
        Ok(())
    }
    fn move_on_ground(
        &mut self,
        _: &mut MoverState,
        a: &mut ActorMotion,
        xy: [f32; 2],
    ) -> aa_movement::Result<()> {
        self.record("MoveOnGround", &xy.map(f32::to_bits));
        let height = (f64::from(a.position[0]) * 0.125 + f64::from(a.position[1]) * 0.25) as f32;
        a.position = [xy[0], xy[1], height];
        Ok(())
    }
    fn notify(
        &mut self,
        e: MotionEvent,
        _: &mut MoverState,
        a: &mut ActorMotion,
    ) -> aa_movement::Result<bool> {
        self.record(&format!("{e:?}"), &[]);
        if e == MotionEvent::CharacterStop {
            a.character_stopped = true;
        }
        if e == MotionEvent::CharacterResume {
            a.character_stopped = false;
        }
        Ok(self.input.notify_result)
    }
    fn blocked(
        &mut self,
        _: &(),
        xy: [f32; 2],
        _: &mut MoverState,
        _: &mut ActorMotion,
    ) -> aa_movement::Result<()> {
        self.record("Blocked", &xy.map(f32::to_bits));
        Ok(())
    }
}
struct NoDiagnosticTurning;
impl GroundTurning for NoDiagnosticTurning {
    fn signed_angle(&mut self, _: [f32; 2], _: [f32; 2], _: Precision) -> aa_movement::Result<f32> {
        Err(aa_movement::Error::Turning(
            "Diagnostic excludes gradual turning".into(),
        ))
    }
    fn rotate(
        &mut self,
        _: [f32; 2],
        _: f32,
        _: bool,
        _: Precision,
    ) -> aa_movement::Result<[f32; 2]> {
        Err(aa_movement::Error::Turning(
            "Diagnostic excludes gradual turning".into(),
        ))
    }
}
fn motion_bits(m: &MoverState) -> Value {
    json!({"destination":m.destination.map(f32::to_bits),"last_step_speed":[m.last_step_speed.to_bits()],"speed":[m.speed.to_bits()],"requested_speed":[m.requested_speed.to_bits()],"acceleration":[m.acceleration.to_bits()],"turn_rate":[m.turn_rate.to_bits()],"desired_direction":m.desired_direction.map(f32::to_bits),"target_range":[m.target_range.map(f32::to_bits)],"speed_factor":[m.speed_factor.to_bits()],"target_id":m.target_id,"ignore_terrain_speed_limit":m.ignore_terrain_speed_limit,"instant_turn":m.instant_turn,"turning":m.turning,"falling":m.falling})
}
fn check_movement(r: &Value, checks: &mut Checks) -> Result<()> {
    let input: MotionInput = decode(&r["input"])?;
    let mut m = input.mover.clone();
    let mut a = input.actor.clone();
    let mut world = DiagnosticWorld {
        input: input.clone(),
        events: vec![],
    };
    let result = aa_movement::ground_step(
        &mut m,
        &mut a,
        input.context,
        &mut world,
        &mut NoDiagnosticTurning,
    )?;
    let actual = json!({"mover_bits":motion_bits(&m),"position_bits":a.position.map(f32::to_bits),"facing_bits":a.facing.map(f32::to_bits),"character_stopped":a.character_stopped,"native_success":result.native_success,"events":world.events});
    let mut expected = r["output"].clone();
    expected
        .as_object_mut()
        .context("Expected native motion output")?
        .remove("abi_preserved"); // This Rust diagnostic does not measure native ABI.
    checks.equal(format!("movement.{}", input.name), actual, &expected);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_original_samples_pass_without_asset_or_gpu_access() {
        let r = run().unwrap();
        assert_eq!(r["passed"], true);
        assert!(r["check_count"].as_u64().unwrap() >= 50);
    }
    #[test]
    fn changed_native_expectation_is_reported_as_failure() {
        let mut b: Value = serde_json::from_slice(SAMPLES).unwrap();
        b["rng"]["converted"][0]["result_bits"] = json!(1);
        let r = evaluate(&b).unwrap();
        assert_eq!(r["passed"], false);
        assert_eq!(
            r["passed_checks"].as_u64().unwrap() + 1,
            r["check_count"].as_u64().unwrap()
        );
    }
}
