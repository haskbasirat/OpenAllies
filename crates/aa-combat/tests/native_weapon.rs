// Explicit Body world seams copied from the reviewed native Body test adapter.
// Gameplay arithmetic remains in aa-body; this file supplies fixture callbacks only.
use aa_body::{Body, Environment, Event, F32, HealthChange, Host, damage::*};
use aa_numeric::Precision;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Deserialize)]
struct View {
    wall_link: bool,
    organization: bool,
    organization_leader: bool,
    mind: bool,
    component88: bool,
}
#[derive(Clone, Deserialize)]
struct Input {
    precision: Precision,
    body: Body,
    environment: Environment,
    request: DamageRequest,
    view: View,
    source_present: bool,
    source_position: [F32; 3],
    wall_present: bool,
    segments_present: bool,
    transfer_factor: F32,
    wall_index: i32,
    neighbors: [bool; 2],
    hostile: bool,
    modifier_present: bool,
    factors: Vec<(u32, F32)>,
    outer_present: bool,
    ancestor_present: bool,
    ancestor_component: bool,
    capture_eligible: bool,
    capture_success: bool,
    capture_factor: F32,
    normalization_factor: F32,
    mutations: Vec<Value>,
}
struct World {
    input: Input,
    events: Vec<Value>,
}
impl World {
    fn event(
        &mut self,
        value: Value,
        body: &mut Body,
        env: &mut Environment,
        record: Option<&mut DamageRecord>,
    ) {
        let mut name = value
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| value.as_object().unwrap().keys().next().unwrap().clone());
        if name == "Action" {
            let action = &value["Action"]["kind"];
            name = action
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| action.as_object().unwrap().keys().next().unwrap().clone());
        }
        self.events.push(json!({"event":value,"body":body}));
        if self
            .input
            .mutations
            .first()
            .is_some_and(|m| m["on"] == name)
        {
            let mutation = self.input.mutations.remove(0);
            if let Some(patch) = mutation.get("body") {
                let mut value = serde_json::to_value(&*body).unwrap();
                value
                    .as_object_mut()
                    .unwrap()
                    .extend(patch.as_object().unwrap().clone());
                *body = serde_json::from_value(value).unwrap();
            }
            if let Some(patch) = mutation.get("environment") {
                let mut value = serde_json::to_value(&*env).unwrap();
                value
                    .as_object_mut()
                    .unwrap()
                    .extend(patch.as_object().unwrap().clone());
                *env = serde_json::from_value(value).unwrap();
            }
            if let Some(patch) = mutation.get("view") {
                for (key, value) in patch.as_object().unwrap() {
                    let value = value.as_bool().unwrap();
                    match key.as_str() {
                        "wall_link" => self.input.view.wall_link = value,
                        "organization" => self.input.view.organization = value,
                        "organization_leader" => self.input.view.organization_leader = value,
                        "mind" => self.input.view.mind = value,
                        "component88" => self.input.view.component88 = value,
                        _ => panic!("unknown fixture view"),
                    }
                }
            }
            if let Some(patch) = mutation.get("record") {
                let record = record.expect("record mutations require a native record callback");
                let mut value = serde_json::to_value(*record).unwrap();
                value
                    .as_object_mut()
                    .unwrap()
                    .extend(patch.as_object().unwrap().clone());
                *record = serde_json::from_value(value).unwrap();
            }
            if let Some(patch) = mutation.get("settings") {
                for (key, value) in patch.as_object().unwrap() {
                    match key.as_str() {
                        "neighbors" => {
                            self.input.neighbors = serde_json::from_value(value.clone()).unwrap()
                        }
                        "hostile" => self.input.hostile = value.as_bool().unwrap(),
                        "capture_eligible" => {
                            self.input.capture_eligible = value.as_bool().unwrap()
                        }
                        "capture_success" => self.input.capture_success = value.as_bool().unwrap(),
                        _ => panic!("unknown fixture setting"),
                    }
                }
            }
        }
    }
}
impl Host for World {
    fn event(
        &mut self,
        event: Event,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<(), String> {
        self.event(serde_json::to_value(event).unwrap(), body, env, None);
        Ok(())
    }
    fn linked_health(
        &mut self,
        change: HealthChange,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<(), String> {
        assert_eq!(change.source_id, None);
        self.event(json!({"LinkedHealth":{"requested":change.requested,"token":change.token,"notify":change.notify,"source":0}}),body,env,None);
        Ok(())
    }
    fn health_source_position(&mut self, _: u32) -> Result<Option<[F32; 3]>, String> {
        Ok(self
            .input
            .source_present
            .then_some(self.input.source_position))
    }
    fn health_normalization_maximum(
        &mut self,
        _: &Body,
        env: &Environment,
        pc: Precision,
    ) -> Result<aa_numeric::Binary, String> {
        let value = aa_numeric::Binary::from_f32_bits(env.derived_normalization_maximum.0)
            .map_err(|e| e.to_string())?;
        if self.input.modifier_present {
            value
                .mul(
                    &aa_numeric::Binary::from_f32_bits(self.input.normalization_factor.0)
                        .map_err(|e| e.to_string())?,
                    pc,
                )
                .map_err(|e| e.to_string())
        } else {
            Ok(value)
        }
    }
}
impl aa_body::damage::World for World {
    fn propagate_damage(
        &mut self,
        target: PropagationTarget,
        request: DamageRequest,
        body: &mut Body,
        env: &mut Environment,
        _: Precision,
    ) -> Result<(), String> {
        self.event(
            json!({"PropagateDamage":{"target":target,"record":request.record,
            "apply_type_modifier":request.apply_type_modifier,"weapon":request.weapon_id.is_some(),
            "propagate_to_neighbors":request.propagate_to_neighbors}}),
            body,
            env,
            None,
        );
        Ok(())
    }
    fn actor_view(&self, _: u32) -> Result<ActorView, String> {
        let v = &self.input.view;
        Ok(ActorView {
            wall_link: v.wall_link.then_some(1),
            organization: v.organization.then_some(1),
            organization_leader: (v.organization && v.organization_leader).then_some(1),
            mind: v.mind.then_some(1),
            component88: v.component88.then_some(1),
        })
    }
    fn wall(&self, _: u32) -> Result<Option<Wall>, String> {
        Ok(self.input.wall_present.then_some(Wall {
            actor: 1,
            segments: self.input.segments_present.then_some(1),
            transfer_factor: self.input.transfer_factor,
        }))
    }
    fn wall_index(&self, _: u32, _: u32) -> Result<i32, String> {
        Ok(self.input.wall_index)
    }
    fn wall_neighbor(&self, _: u32, _: i32, forward: bool) -> Result<Option<u32>, String> {
        let i = usize::from(!forward);
        Ok(self.input.neighbors[i].then_some(i as u32 + 1))
    }
    fn hostile(&mut self, _: u32, body: &mut Body, env: &mut Environment) -> Result<bool, String> {
        self.event(json!("Hostile"), body, env, None);
        Ok(self.input.hostile)
    }
    fn resolve_source(&self, _: u32) -> Result<Option<u32>, String> {
        Ok(self.input.source_present.then_some(1))
    }
    fn modifier(&self, _: u32) -> Result<Option<u32>, String> {
        Ok(self.input.modifier_present.then_some(1))
    }
    fn damage_factor(&self, _: u32, kind: u32) -> Result<F32, String> {
        Ok(first_damage_factor(&self.input.factors, kind))
    }
    fn outer_actor(
        &mut self,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<Option<OuterActor>, String> {
        self.event(json!("OuterActor"), body, env, None);
        Ok(self.input.outer_present.then_some(OuterActor {
            actor: 1,
            component98: 1,
        }))
    }
    fn ancestor_component88(&self, _: u32) -> Result<Option<u32>, String> {
        Ok((self.input.ancestor_present && self.input.ancestor_component).then_some(2))
    }
    fn capture_eligible(&mut self, body: &mut Body, env: &mut Environment) -> Result<bool, String> {
        self.event(json!("CaptureEligible"), body, env, None);
        Ok(self.input.capture_eligible)
    }
    fn attempt_capture(
        &mut self,
        _: Option<u32>,
        _: &mut DamageRecord,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<bool, String> {
        self.event(json!("AttemptCapture"), body, env, None);
        Ok(self.input.capture_success)
    }
    fn capture_health_factor(&self) -> Result<F32, String> {
        Ok(self.input.capture_factor)
    }
    fn action(
        &mut self,
        action: Action,
        record: &mut DamageRecord,
        body: &mut Body,
        env: &mut Environment,
    ) -> Result<(), String> {
        self.event(
            json!({"Action":{"kind":action,"record":record}}),
            body,
            env,
            Some(record),
        );
        Ok(())
    }
}

#[derive(Clone, Deserialize)]
struct HitInput {
    name: String,
    body_input: Input,
    hit: aa_combat::Hit,
    target_has_body: bool,
    target_eligible: bool,
    source_offensive_component: bool,
    source_modifier: bool,
    source_organization: bool,
    source_leader: bool,
    leader_factor: F32,
    offensive_primary: F32,
    offensive_secondary: F32,
    minimum: F32,
}

struct HitWorld {
    case: HitInput,
    body: Body,
    environment: Environment,
    body_world: World,
    requests: Vec<DamageRequest>,
}
impl aa_combat::World for HitWorld {
    fn resolve_target(
        &mut self,
        actor: Option<u32>,
    ) -> aa_combat::Result<Option<aa_combat::Target>> {
        Ok(actor
            .filter(|_| self.case.target_eligible && self.case.target_has_body)
            .map(|actor| aa_combat::Target { actor, body: 1 }))
    }
    fn actor_id(&self, actor: u32) -> aa_combat::Result<u32> {
        match actor {
            1 => Ok(self.case.body_input.request.record.source_id),
            2 => Ok(self.case.body_input.request.record.allegiance_id),
            _ => Err(aa_combat::Error::Invalid("unknown fixture actor")),
        }
    }
    fn modifier(&self, _: u32) -> aa_combat::Result<Option<u32>> {
        Ok(self.case.body_input.modifier_present.then_some(1))
    }
    fn damage_type_factor(&self, _: u32, kind: u32) -> aa_combat::Result<aa_numeric::Binary> {
        Ok(aa_numeric::Binary::from_f32_bits(
            first_damage_factor(&self.case.body_input.factors, kind).0,
        )?)
    }
    fn offensive_component(&self, _: u32) -> aa_combat::Result<Option<u32>> {
        Ok(self.case.source_offensive_component.then_some(1))
    }
    fn offensive_factor(&self, _: u32) -> aa_combat::Result<aa_numeric::Binary> {
        let factor = if self.case.source_organization && self.case.source_leader {
            self.case.leader_factor
        } else if self.case.source_modifier {
            aa_combat::offensive_modifier(
                self.case.offensive_primary,
                self.case.offensive_secondary,
            )?
        } else {
            F32::ONE
        };
        Ok(aa_numeric::Binary::from_f32_bits(factor.0)?)
    }
    fn defense(&self, _: u32) -> aa_combat::Result<F32> {
        Ok(self.body.defense)
    }
    fn minimum_weapon_damage(&self) -> aa_combat::Result<F32> {
        Ok(self.case.minimum)
    }
    fn deliver(
        &mut self,
        _: aa_combat::Target,
        request: DamageRequest,
        precision: Precision,
    ) -> aa_combat::Result<()> {
        self.requests.push(request);
        self.body_world.input.request = request;
        deal_damage(
            &mut self.body,
            request,
            &mut self.environment,
            &mut self.body_world,
            precision,
        )
        .map_err(|e| aa_combat::Error::World(e.to_string()))?;
        Ok(())
    }
}

#[test]
fn whole_original_weapon_through_body_chain_matches() {
    let document: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-weapon/native-weapon-fixtures.json"
    ))
    .unwrap();
    let cases = document["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 486);
    for case in cases {
        let input: HitInput = serde_json::from_value(case["input"].clone()).unwrap();
        let hit = input.hit;
        let pc = input.body_input.precision;
        let mut world = HitWorld {
            body: input.body_input.body.clone(),
            environment: input.body_input.environment.clone(),
            body_world: World {
                input: input.body_input.clone(),
                events: vec![],
            },
            requests: vec![],
            case: input,
        };
        let dispatched = aa_combat::hit(hit, &mut world, pc).unwrap();
        let actual = json!({"dispatched":dispatched,"requests":world.requests,"body":world.body,"environment":world.environment,"events":world.body_world.events});
        assert_eq!(actual, case["output"], "{} {pc:?}", world.case.name);
    }
}

#[derive(Clone, Deserialize)]
struct PeriodicInput {
    name: String,
    body_input: Input,
    state: aa_combat::periodic::Properties,
    expired: Vec<u32>,
    body_present: bool,
    outer_present: bool,
    outer_flags: u32,
    ancestor_present: bool,
    ancestor_flags: u32,
    recompute_healing: Option<Vec<F32>>,
    recompute_damage: Option<Vec<aa_combat::periodic::PeriodicDamage>>,
}
struct PeriodicWorld {
    case: PeriodicInput,
    body: Body,
    environment: Environment,
    body_world: World,
    calls: Vec<Value>,
    expiry_events: Vec<Value>,
    fail_after_effect_removal: bool,
    shrink_healing: bool,
}
impl PeriodicWorld {
    fn effect_event(&mut self, name: &str, effect: &aa_combat::periodic::Effect) {
        self.expiry_events
            .push(json!({"name":name,"handle":effect.handle}));
    }
}
impl aa_combat::periodic::World for PeriodicWorld {
    fn expired(&mut self, effect: &aa_combat::periodic::Effect) -> aa_combat::Result<bool> {
        self.effect_event("expired", effect);
        Ok(self.case.expired.contains(&effect.handle))
    }
    fn resolve_source(&self, id: u32) -> aa_combat::Result<Option<u32>> {
        Ok(match id {
            0x10017 => Some(1),
            0x10018 => Some(2),
            _ => None,
        })
    }
    fn actor_effect_removed(
        &mut self,
        _: &mut aa_combat::periodic::Properties,
        effect: &aa_combat::periodic::Effect,
    ) -> aa_combat::Result<()> {
        self.effect_event("actor_effect_removed", effect);
        Ok(())
    }
    fn effect_removed(
        &mut self,
        _: &mut aa_combat::periodic::Properties,
        effect: &aa_combat::periodic::Effect,
    ) -> aa_combat::Result<()> {
        self.effect_event("effect_removed", effect);
        if self.fail_after_effect_removal {
            return Err(aa_combat::Error::World("injected removal failure".into()));
        }
        Ok(())
    }
    fn recompute_modifier(
        &mut self,
        state: &mut aa_combat::periodic::Properties,
        effect: &aa_combat::periodic::Effect,
    ) -> aa_combat::Result<()> {
        self.effect_event("recompute_modifier", effect);
        if let Some(healing) = &self.case.recompute_healing {
            state.healing = healing.clone();
        }
        if let Some(damage) = &self.case.recompute_damage {
            state.damage = damage.clone();
        }
        Ok(())
    }
    fn release_effect(&mut self, effect: &aa_combat::periodic::Effect) -> aa_combat::Result<()> {
        self.effect_event("release_effect", effect);
        Ok(())
    }
    fn body(&self, _: u32) -> aa_combat::Result<Option<u32>> {
        Ok(self.case.body_present.then_some(1))
    }
    fn outer_actor(&mut self, _: u32) -> aa_combat::Result<Option<u32>> {
        self.expiry_events.push(json!({"name":"outer_actor"}));
        Ok(self.case.outer_present.then_some(4))
    }
    fn ancestor(&self, _: u32) -> aa_combat::Result<Option<u32>> {
        Ok(self.case.ancestor_present.then_some(5))
    }
    fn actor_flags(&self, actor: u32) -> aa_combat::Result<u32> {
        Ok(match actor {
            4 => self.case.outer_flags,
            5 => self.case.ancestor_flags,
            _ => self.environment.actor_flags,
        })
    }
    fn heal(
        &mut self,
        state: &mut aa_combat::periodic::Properties,
        _: u32,
        amount: F32,
        pc: Precision,
    ) -> aa_combat::Result<()> {
        self.calls.push(json!({"kind":"heal","amount":amount}));
        self.body
            .heal_plain(
                aa_body::HealRequest {
                    source_id: 0,
                    amount,
                    token: 1,
                    converts_to_damage: false,
                },
                &mut self.environment,
                &mut self.body_world,
                pc,
            )
            .map_err(|error| aa_combat::Error::World(error.to_string()))?;
        if self.shrink_healing {
            state.healing.truncate(1);
        }
        Ok(())
    }
    fn now(&self) -> aa_combat::Result<F32> {
        Ok(self.environment.world_time)
    }
    fn damage(
        &mut self,
        _: &mut aa_combat::periodic::Properties,
        _: u32,
        request: DamageRequest,
        pc: Precision,
    ) -> aa_combat::Result<()> {
        self.calls.push(json!({"kind":"damage","request":request}));
        self.body_world.input.request = request;
        self.body_world.input.source_present = request.record.source_id != 0;
        deal_damage(
            &mut self.body,
            request,
            &mut self.environment,
            &mut self.body_world,
            pc,
        )
        .map_err(|error| aa_combat::Error::World(error.to_string()))?;
        Ok(())
    }
}

#[test]
fn original_periodic_expiry_heal_and_damage_chain_matches() {
    let document: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-weapon/native-periodic-fixtures.json"
    ))
    .unwrap();
    let cases = document["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 90);
    for case in cases {
        let input: PeriodicInput = serde_json::from_value(case["input"].clone()).unwrap();
        let mut state = input.state.clone();
        let pc = input.body_input.precision;
        let mut world = PeriodicWorld {
            body: input.body_input.body.clone(),
            environment: input.body_input.environment.clone(),
            body_world: World {
                input: input.body_input.clone(),
                events: vec![],
            },
            calls: vec![],
            expiry_events: vec![],
            fail_after_effect_removal: false,
            shrink_healing: false,
            case: input,
        };
        aa_combat::periodic::tick(&mut state, &mut world, pc).unwrap();
        let actual = json!({"state":state,"calls":world.calls,"expiry_events":world.expiry_events,
            "body":world.body,"environment":world.environment,"events":world.body_world.events});
        assert_eq!(actual, case["output"], "{} {pc:?}", world.case.name);
    }
}

fn periodic_fixture(name: &str) -> (aa_combat::periodic::Properties, PeriodicWorld, Precision) {
    let document: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-weapon/native-periodic-fixtures.json"
    ))
    .unwrap();
    let row = document["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            row["input"]["name"] == name && row["input"]["body_input"]["precision"] == "Double53"
        })
        .unwrap();
    let input: PeriodicInput = serde_json::from_value(row["input"].clone()).unwrap();
    let state = input.state.clone();
    let pc = input.body_input.precision;
    let world = PeriodicWorld {
        body: input.body_input.body.clone(),
        environment: input.body_input.environment.clone(),
        body_world: World {
            input: input.body_input.clone(),
            events: vec![],
        },
        calls: vec![],
        expiry_events: vec![],
        fail_after_effect_removal: false,
        shrink_healing: false,
        case: input,
    };
    (state, world, pc)
}

#[test]
fn removal_error_stops_before_stat_rebuild_and_body_calls() {
    let (mut state, mut world, pc) = periodic_fixture("expired_single");
    world.fail_after_effect_removal = true;
    let old_body = world.body.clone();
    assert!(aa_combat::periodic::tick(&mut state, &mut world, pc).is_err());
    assert!(state.effects.is_empty());
    assert_eq!(state.modifier_flags, 0);
    assert_eq!(world.body, old_body);
    assert!(world.calls.is_empty());
    assert_eq!(
        world.expiry_events.last().unwrap()["name"],
        "effect_removed"
    );
}

#[test]
fn shrinking_a_sampled_heal_list_errors_after_prior_real_effects() {
    let (mut state, mut world, pc) = periodic_fixture("multiple_entries");
    world.shrink_healing = true;
    assert!(aa_combat::periodic::tick(&mut state, &mut world, pc).is_err());
    assert_eq!(world.body.health, F32(80f32.to_bits()));
    assert_eq!(world.calls.len(), 1);
    assert_eq!(world.calls[0]["kind"], "heal");
    assert_eq!(state.healing.len(), 1);
}
