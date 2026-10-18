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
    execution: Value,
    actions: Vec<Value>,
    query: usize,
    flight: Vec<Value>,
    projectile_config: Value,
    projectile_active: bool,
}
impl aa_combat::World for HitWorld {
    fn resolve_target(
        &mut self,
        actor: Option<u32>,
    ) -> aa_combat::Result<Option<aa_combat::Target>> {
        Ok(actor
            .filter(|a| *a == 2 && self.case.target_eligible && self.case.target_has_body)
            .map(|actor| aa_combat::Target { actor, body: 1 }))
    }
    fn actor_id(&self, actor: u32) -> aa_combat::Result<u32> {
        match actor {
            1 => Ok(self.case.body_input.request.record.source_id),
            2 => Ok(777),
            3 => Ok(self.case.body_input.request.record.allegiance_id),
            7 => Ok(0x10077),
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

use aa_combat::{ability::Ability, admission as a, execution as e, performer::Context};
use aa_numeric::Binary;
impl HitWorld {
    fn flag(&self, k: &str) -> bool {
        self.execution[k].as_bool().unwrap()
    }
    fn event_action(&mut self, name: &str, mut values: Value) {
        values
            .as_object_mut()
            .unwrap()
            .insert("name".into(), json!(name));
        self.actions.push(values);
    }
    fn actor_value(&self, id: u32) -> &Value {
        &self.execution["actors"][id.to_string()]
    }
}
fn unexpected<T>(name: &str) -> aa_combat::Result<T> {
    Err(aa_combat::Error::World(format!(
        "unexpected fixture service: {name}"
    )))
}
impl a::World for HitWorld {
    fn definition(&self, _: u32) -> aa_combat::Result<a::Definition> {
        let d: e::Definition =
            serde_json::from_value(self.execution["definition"].clone()).unwrap();
        Ok(a::Definition {
            minimum_range: F32::ZERO,
            recharge_time: F32::ZERO,
            range: F32::ZERO,
            targets: a::Filter {
                required: 0,
                forbidden: 0,
            },
            area: d.area,
            projectile: d.projectile.is_some(),
            mana_cost: F32::ZERO,
            volley_area: d.volley_area,
            flags: d.flags,
            effects: (0..self.execution["effects"].as_array().unwrap().len())
                .map(|i| 100 + i as u32)
                .collect(),
        })
    }
    fn local_definition(&self, _: u32) -> aa_combat::Result<a::LocalDefinition> {
        unexpected("local_definition")
    }
    fn actor(&self, id: u32) -> aa_combat::Result<a::Actor> {
        let v = self.actor_value(id);
        Ok(a::Actor {
            flags8: v["flags8"].as_u64().unwrap() as u32,
            flags100: 0,
            flags104: 0,
            position: serde_json::from_value(v["position"].clone()).unwrap(),
            has_regiment: false,
            has_organization: false,
            has_properties: if id == 2 {
                self.case.body_input.modifier_present
            } else {
                self.case.source_modifier
            },
            has_summoner: false,
            has_mind: false,
            mana: None,
            body: (id == 2).then_some([self.body.health, self.body.maximum]),
            kingdom_kind: 0,
        })
    }
    fn is_actor(&mut self, _: u32) -> aa_combat::Result<bool> {
        Ok(self.case.target_eligible)
    }
    fn is_kingdom(&mut self, id: u32) -> aa_combat::Result<bool> {
        Ok(id == 1 && self.flag("source_kingdom"))
    }
    fn allegiance(&mut self, _: u32) -> aa_combat::Result<Option<u32>> {
        Ok(Some(3))
    }
    fn friendly(&mut self, _: u32, _: Option<u32>) -> aa_combat::Result<bool> {
        unexpected("friendly")
    }
    fn allied(&mut self, _: u32, _: Option<u32>) -> aa_combat::Result<bool> {
        unexpected("allied")
    }
    fn class_flags(&mut self, _: u32) -> aa_combat::Result<u32> {
        unexpected("class_flags")
    }
    fn injured(&mut self, _: u32) -> aa_combat::Result<bool> {
        unexpected("injured")
    }
    fn organization_leader(&mut self, _: u32) -> aa_combat::Result<Option<u32>> {
        unexpected("organization_leader")
    }
    fn summoner_accepts(&mut self, _: u32, _: u32) -> aa_combat::Result<bool> {
        unexpected("summoner_accepts")
    }
    fn modifier(&mut self, _: u32) -> aa_combat::Result<Option<u32>> {
        unexpected("admission modifier")
    }
    fn recharge_modified(&mut self, _: u32, _: F32) -> aa_combat::Result<Binary> {
        unexpected("recharge_modified")
    }
    fn projectile_range_modified(&mut self, _: u32, _: F32) -> aa_combat::Result<Binary> {
        unexpected("projectile_range_modified")
    }
    fn bombard_range_modified(&mut self, _: u32, _: F32) -> aa_combat::Result<Binary> {
        unexpected("bombard_range_modified")
    }
    fn effect_kind(&self, effect: u32) -> aa_combat::Result<u32> {
        Ok(
            self.execution["effects"][(effect - 100) as usize]["damage"]["kind"]
                .as_u64()
                .unwrap_or(99) as u32,
        )
    }
    fn effect_property(&mut self, _: u32) -> aa_combat::Result<Option<u32>> {
        unexpected("effect_property")
    }
    fn property_immunity_class(&self, _: u32) -> aa_combat::Result<u32> {
        unexpected("property_immunity_class")
    }
    fn immune_to(&mut self, _: u32, _: u32) -> aa_combat::Result<bool> {
        unexpected("immune_to")
    }
    fn effects_for(&mut self, _: u32, _: Option<u32>) -> aa_combat::Result<Vec<u32>> {
        Ok((0..self.execution["effects"].as_array().unwrap().len())
            .map(|i| 100 + i as u32)
            .collect())
    }
    fn has_property(&mut self, _: u32, _: Option<u32>) -> aa_combat::Result<bool> {
        unexpected("has_property")
    }
    fn mind_allows_full_health_heal(&mut self, _: u32) -> aa_combat::Result<bool> {
        unexpected("mind_allows_full_health_heal")
    }
    fn range_test(&mut self, _: &Context, _: F32, _: bool) -> aa_combat::Result<bool> {
        unexpected("range_test")
    }
    fn turret_covers(&mut self, _: u32, _: u32, _: [F32; 2]) -> aa_combat::Result<bool> {
        unexpected("turret_covers")
    }
    fn can_pay(&mut self, _: u32, _: &Ability) -> aa_combat::Result<bool> {
        unexpected("can_pay")
    }
    fn resolve_id(&mut self, id: u32) -> aa_combat::Result<Option<u32>> {
        Ok(match id {
            0x10017 if self.projectile_config["source_live"].as_bool().unwrap() => Some(1),
            777 if self.projectile_config["target_live"].as_bool().unwrap() => Some(2),
            _ => None,
        })
    }
    fn owned_actors(&mut self, _: u32) -> aa_combat::Result<Vec<u32>> {
        unexpected("owned_actors")
    }
    fn summon_filter(&self, _: u32) -> aa_combat::Result<a::Filter> {
        unexpected("summon_filter")
    }
    fn free_recharge_enabled(&self) -> aa_combat::Result<bool> {
        unexpected("free_recharge_enabled")
    }
    fn now(&self) -> aa_combat::Result<F32> {
        Ok(self.environment.world_time)
    }
}
impl e::World for HitWorld {
    fn execution_definition(&self, _: u32) -> aa_combat::Result<e::Definition> {
        Ok(serde_json::from_value(self.execution["definition"].clone()).unwrap())
    }
    fn effect(&self, id: u32) -> aa_combat::Result<e::Effect> {
        Ok(serde_json::from_value(self.execution["effects"][(id - 100) as usize].clone()).unwrap())
    }
    fn position_on_primary_island(&mut self, _: [F32; 2]) -> aa_combat::Result<bool> {
        Ok(self.flag("primary_allowed"))
    }
    fn clamp_map_position(&mut self, p: &mut [F32; 2]) -> aa_combat::Result<()> {
        *p = serde_json::from_value(self.execution["clamped"].clone()).unwrap();
        self.flight.push(json!({"name":"clamp","position":p}));
        Ok(())
    }
    fn ability_fx(
        &mut self,
        _: &mut Ability,
        c: &mut Context,
        event: u32,
    ) -> aa_combat::Result<()> {
        self.event_action("fx", json!({"event":event,"context":c}));
        Ok(())
    }
    fn pay_cost(&mut self, _: u32, _: &mut Ability) -> aa_combat::Result<()> {
        self.event_action("payment", json!({}));
        Ok(())
    }
    fn create_projectile(
        &mut self,
        _: &mut Ability,
        c: &mut Context,
        _: u32,
        _: Precision,
    ) -> aa_combat::Result<Option<u32>> {
        self.event_action("projectile", json!({"context":c}));
        Ok(self.flag("projectile_success").then_some(123))
    }
    fn begin_area_query(&mut self, c: [F32; 2], r: F32) -> aa_combat::Result<u32> {
        self.query = 0;
        self.event_action("begin_area", json!({"radius":r,"center":c}));
        Ok(1)
    }
    fn next_area_target(&mut self, _: u32) -> aa_combat::Result<Option<u32>> {
        let t = self.execution["area_targets"]
            .as_array()
            .unwrap()
            .get(self.query)
            .and_then(Value::as_u64)
            .map(|v| v as u32);
        self.query += 1;
        self.event_action("next_area", json!({"target":t}));
        Ok(t)
    }
    fn end_area_query(&mut self, _: u32) -> aa_combat::Result<()> {
        self.event_action("end_area", json!({}));
        Ok(())
    }
    fn area_impulse(&mut self, p: [F32; 3], a: F32, r: F32) -> aa_combat::Result<()> {
        self.event_action("area_impulse", json!({"position":p,"amount":a,"radius":r}));
        Ok(())
    }
    fn ability_is_structure(&self, _: u32) -> aa_combat::Result<bool> {
        Ok(self.flag("structure"))
    }
    fn actor_definition_flags168(&self, id: u32) -> aa_combat::Result<u32> {
        Ok(self.actor_value(id)["definition_flags168"]
            .as_u64()
            .unwrap() as u32)
    }
    fn actor_radius(&mut self, _: u32) -> aa_combat::Result<Binary> {
        Ok(Binary::from_f32_bits(
            self.execution["radius"].as_u64().unwrap() as u32,
        )?)
    }
    fn apply_impulse(&mut self, a: u32, v: [F32; 3]) -> aa_combat::Result<()> {
        self.event_action("impulse", json!({"actor":a,"vector":v}));
        Ok(())
    }
    fn other_effect(
        &mut self,
        effect: u32,
        _: &mut Ability,
        c: &mut Context,
        s: F32,
        _: Precision,
    ) -> aa_combat::Result<()> {
        self.event_action("other", json!({"effect":effect,"context":c,"scale":s}));
        Ok(())
    }
}

use aa_combat::projectile::{self, Projectile};
impl projectile::World for HitWorld {
    fn activate_projectile(&mut self, _: &mut Projectile) -> aa_combat::Result<bool> {
        let result = self.projectile_config["activate"].as_bool().unwrap();
        self.projectile_active = result;
        self.flight.push(json!({"name":"activate","result":result}));
        Ok(result)
    }
    fn schedule_projectile(&mut self, _: &mut Projectile, e: i32) -> aa_combat::Result<()> {
        self.flight.push(json!({"name":"schedule","event":e}));
        Ok(())
    }
    fn projectile_active(&self, _: &Projectile) -> aa_combat::Result<bool> {
        Ok(self.projectile_active)
    }
    fn deactivate_projectile(&mut self, _: &mut Projectile) -> aa_combat::Result<()> {
        self.projectile_active = false;
        self.flight.push(json!({"name":"deactivate"}));
        Ok(())
    }
    fn projectile_definition(&self, _: u32) -> aa_combat::Result<projectile::Definition> {
        Ok(projectile::Definition {
            kind: self.projectile_config["kind"].as_u64().unwrap() as u32,
            life: F32(self.projectile_config["life"].as_u64().unwrap() as u32),
        })
    }
    fn position_in_map(&mut self, p: [F32; 2]) -> aa_combat::Result<bool> {
        self.flight.push(json!({"name":"inside","point":p}));
        Ok(self.projectile_config["inside"].as_bool().unwrap())
    }
    fn target_height_ratio(&self, arcing: bool) -> aa_combat::Result<F32> {
        Ok(F32(self.projectile_config["ratios"][usize::from(arcing)]
            .as_u64()
            .unwrap() as u32))
    }
    fn actor_height(&mut self, id: u32) -> aa_combat::Result<Binary> {
        Ok(Binary::from_f32_bits(
            self.projectile_config["motion"][id.to_string()]["height"]
                .as_u64()
                .unwrap() as u32,
        )?)
    }
    fn actor_speed(&mut self, id: u32) -> aa_combat::Result<Binary> {
        Ok(Binary::from_f32_bits(
            self.projectile_config["motion"][id.to_string()]["speed"]
                .as_u64()
                .unwrap() as u32,
        )?)
    }
    fn actor_facing(&self, id: u32) -> aa_combat::Result<[F32; 3]> {
        Ok(serde_json::from_value(
            self.projectile_config["motion"][id.to_string()]["facing"].clone(),
        )
        .unwrap())
    }
    fn target_hit_position(&mut self, id: u32, d: [F32; 3]) -> aa_combat::Result<[F32; 3]> {
        self.flight
            .push(json!({"name":"surface","actor":id,"direction":d}));
        Ok(serde_json::from_value(self.projectile_config["surface"].clone()).unwrap())
    }
    fn move_projectile(&mut self, _: u32, p: [F32; 3]) -> aa_combat::Result<()> {
        self.execution["actors"]["7"]["position"] = json!(p);
        self.flight.push(json!({"name":"move","position":p}));
        Ok(())
    }
    fn projectile_ability_handle(&self, _: u32) -> aa_combat::Result<u32> {
        Ok(1)
    }
}
#[test]
fn native_projectile_activation_flight_impact_and_real_body_chain_match() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-ability/native-projectile-fixtures.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 60);
    for row in rows {
        let input = &row["input"];
        let e = &input["execution"];
        let hit: HitInput = serde_json::from_value(e["weapon_case"].clone()).unwrap();
        let pc = hit.body_input.precision;
        let mut world = HitWorld {
            body: hit.body_input.body.clone(),
            environment: hit.body_input.environment.clone(),
            body_world: World {
                input: hit.body_input.clone(),
                events: vec![],
            },
            requests: vec![],
            case: hit,
            execution: e.clone(),
            actions: vec![],
            query: 0,
            flight: vec![],
            projectile_config: input.clone(),
            projectile_active: input["initial_active"].as_bool().unwrap(),
        };
        world.execution["actors"]["7"] =
            json!({"flags8":0,"definition_flags168":0,"position":input["owner_position"]});
        let mut state = Projectile::new(7, F32(input["speed"].as_u64().unwrap() as u32));
        let ability: Ability = serde_json::from_value(e["ability"].clone()).unwrap();
        let context: Context = serde_json::from_value(e["context"].clone()).unwrap();
        for (i, op) in input["operations"].as_array().unwrap().iter().enumerate() {
            match op["kind"].as_str().unwrap() {
                "start" => {
                    let source = input["copy_ability"].as_bool().unwrap().then_some(&ability);
                    projectile::start(&mut state, &context, source, 42, &mut world, pc)
                        .unwrap_or_else(|e| panic!("{} start {e}", input["name"]));
                }
                "tick" => projectile::advance(
                    &mut state,
                    F32(op["delta"].as_u64().unwrap() as u32),
                    &mut world,
                    pc,
                )
                .unwrap_or_else(|e| panic!("{} tick{i} {e}", input["name"])),
                _ => panic!("unknown operation"),
            }
            let actual = json!({"projectile":state,"position":world.execution["actors"]["7"]["position"],"active":world.projectile_active,"flight":world.flight,"actions":world.actions,"requests":world.requests,"body":world.body,"environment":world.environment,"events":world.body_world.events});
            assert_eq!(
                actual, row["steps"][i],
                "{} {i} {}",
                world.case.name, input["precision"]
            );
        }
    }
}
