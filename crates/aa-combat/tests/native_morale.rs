//! Replays research/phase8-combat/native-morale-fixtures.json: original
//! Organization morale/attack-efficiency routines executed under Unicorn.
use aa_body::{
    F32, credit::Object, damage::DamageRecord, experience::Request, experience_amounts,
    properties::ModifierState,
};
use aa_combat::{
    Error, Result,
    morale::{
        self, Actor, CaptureWorld, Mind, MoraleEffect, Organization, Settings, StateDescriptor,
        World,
    },
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};

fn f(v: &Value) -> F32 {
    F32(v.as_u64().unwrap() as u32)
}

fn organization(input: &Value) -> Organization {
    let o = &input["organization"];
    Organization {
        owner: 1,
        morale: f(&o["morale"]),
        maximum: f(&o["maximum"]),
        breakpoint: f(&o["breakpoint"]),
        living: o["living"].as_i64().unwrap() as i32,
        engaged_latch: o["engaged_latch"] == true,
        engage_latch: o["engage_latch"] == true,
        attack_efficiency: f(&o["attack_efficiency"]),
        attack_efficiency_desired: f(&o["attack_efficiency_desired"]),
        byte8d: false,
        last_damage: DamageRecord {
            source_id: 0,
            allegiance_id: 0,
            damage_type: 0,
            amount: F32::ZERO,
            time: F32::ZERO,
            event_kind: 0,
            reason: 10,
        },
        leader: None,
        primary: None,
        regroup_started: F32::ZERO,
        entrench_base: F32::ZERO,
        flare_pending: false,
        flare_word: 0,
        marker_pending: false,
        watched_target: 0,
        current_combat_value: F32::ZERO,
        maximum_combat_value: F32::ZERO,
        field_d8: F32::ZERO,
    }
}

fn snapshot(o: &Organization) -> Value {
    json!({
        "morale": o.morale.0, "maximum": o.maximum.0, "living": o.living as u32,
        "attack_efficiency": o.attack_efficiency.0,
        "attack_efficiency_desired": o.attack_efficiency_desired.0,
        "breakpoint": o.breakpoint.0, "engaged_latch": o.engaged_latch,
        "engage_latch": o.engage_latch,
    })
}

fn settings(input: &Value) -> Settings {
    let s = &input["settings"];
    Settings {
        engagement_loss: f(&s["engagement_loss"]),
        engagement_loss_factor_max: f(&s["engagement_loss_factor_max"]),
        retreat_penalty: f(&s["retreat_penalty"]),
        slayer_base: f(&s["slayer_base"]),
        slayer_min: f(&s["slayer_min"]),
        slayer_max: f(&s["slayer_max"]),
        breaker_base: f(&s["breaker_base"]),
        breaker_min: f(&s["breaker_min"]),
        breaker_max: f(&s["breaker_max"]),
        replenish: f(&s["replenish"]),
        supply_bonus: f(&s["supply_bonus"]),
        attack_efficiency_climbing: f(&s["attack_efficiency_climbing"]),
        attack_efficiency_falling: f(&s["attack_efficiency_falling"]),
        experience: experience_amounts::Settings {
            minimum: f(&s["experience_minimum"]),
            maximum: f(&s["experience_maximum"]),
            kill: f(&s["experience_kill"]),
            capture: f(&s["experience_capture"]),
            rout: f(&s["experience_rout"]),
            engagement: f(&s["experience_engage"]),
            default_value: f(&s["experience_default"]),
        },
    }
}

struct Host {
    input: Value,
    mind: Option<Mind>,
    declines: bool,
    mutations: Vec<Value>,
    values: Value,
    events: Vec<Value>,
    initial: Value,
    fail_on: Option<&'static str>,
}

impl Host {
    fn new(input: &Value) -> Self {
        let mind = (!input["mind"].is_null()).then(|| Mind {
            word28: input["mind"]["word28"].as_u64().unwrap() as u32,
            word2c: input["mind"]["word2c"].as_u64().unwrap() as u32,
        });
        Self {
            input: input.clone(),
            mind,
            declines: input["mind"]["declines"] == true,
            mutations: input["mutations"].as_array().unwrap().clone(),
            values: input["actor_values"].clone(),
            events: vec![],
            initial: snapshot(&organization(input)),
            fail_on: None,
        }
    }
    fn event(&mut self, org: Option<&mut Organization>, name: &str, value: Value) -> Result<()> {
        let snap = org.as_ref().map_or(self.initial.clone(), |o| snapshot(o));
        self.events
            .push(json!({"event": {name: value}, "organization": snap}));
        if self.fail_on == Some(name) {
            return Err(Error::World(format!("{name} failed")));
        }
        if self.mutations.first().is_some_and(|m| m["on"] == name) {
            let m = self.mutations.remove(0);
            if let (Some(o), Some(fields)) = (org, m["organization"].as_object()) {
                for (k, v) in fields {
                    match k.as_str() {
                        "morale" => o.morale = f(v),
                        "maximum" => o.maximum = f(v),
                        "breakpoint" => o.breakpoint = f(v),
                        other => panic!("unsupported mutation {other}"),
                    }
                }
            }
            if !m["mind"].is_null() {
                let mind = self.mind.as_mut().unwrap();
                mind.word28 = m["mind"]["word28"].as_u64().unwrap() as u32;
                mind.word2c = m["mind"]["word2c"].as_u64().unwrap() as u32;
            }
        }
        Ok(())
    }
    fn view(&self, tag: u32) -> Value {
        self.input["others"]
            .get(tag.to_string())
            .cloned()
            .unwrap_or(json!({}))
    }
    fn value(&mut self, org: Option<&mut Organization>, actor: u32) -> Result<Binary> {
        let bits = self.values[actor.to_string()]
            .as_array_mut()
            .unwrap()
            .remove(0)
            .as_u64()
            .unwrap();
        self.event(org, "ActorValue", json!([actor, bits]))?;
        Ok(Binary::from_f64_bits(bits).unwrap())
    }
    fn actor_view(&self, actor: u32) -> Actor {
        if actor == 1 {
            let a = &self.input["actor"];
            return Actor {
                flags8: a["flags8"].as_u64().unwrap() as u32,
                flags100: a["flags100"].as_u64().unwrap() as u32,
                flags104: a["flags104"].as_u64().unwrap() as u32,
                engaged: a["engaged"] == true,
                has_organization: true,
                membership: None,
            };
        }
        let v = self.view(actor);
        Actor {
            flags8: 0,
            flags100: 0,
            flags104: 0,
            engaged: false,
            has_organization: v.get("organization_maximum").is_some(),
            membership: match v.get("membership_organization") {
                None | Some(Value::Null) => None,
                Some(t) => {
                    let t = t.as_u64().unwrap() as u32;
                    Some((t != 0).then_some(t))
                }
            },
        }
    }
}

impl World for Host {
    fn session_mode(&self) -> Result<u32> {
        Ok(self.input["game_mode"].as_u64().unwrap() as u32)
    }
    fn actor(&self, actor: u32) -> Result<Actor> {
        Ok(self.actor_view(actor))
    }
    fn mind(&self, actor: u32) -> Result<Option<Mind>> {
        assert_eq!(actor, 1);
        Ok(self.mind)
    }
    fn mind_declines(&mut self, o: &mut Organization) -> Result<bool> {
        let r = self.declines;
        self.event(Some(o), "MindDeclines", json!(r))?;
        Ok(r)
    }
    fn mind_exhausted(&mut self, o: &mut Organization) -> Result<()> {
        self.event(Some(o), "MindExhausted", Value::Null)
    }
    fn mind_wavering(&mut self, o: &mut Organization) -> Result<()> {
        self.event(Some(o), "MindWavering", Value::Null)
    }
    fn set_state(&mut self, o: &mut Organization, state: u8, enabled: bool) -> Result<()> {
        self.event(Some(o), "SetState", json!([state, enabled]))
    }
    fn modifier(&mut self, o: &mut Organization) -> Result<Option<ModifierState>> {
        self.event(Some(o), "Modifier", json!(1))?;
        let m = &self.input["modifier"];
        if m.is_null() {
            return Ok(None);
        }
        let mut state = ModifierState::new(0);
        state.flags = m["flags"].as_u64().unwrap() as u32;
        state.scalars[0x44 / 4 - 1] = f(&m["multiply"]);
        state.scalars[0x48 / 4 - 1] = f(&m["divide"]);
        Ok(Some(state))
    }
    fn state_group(&mut self, o: &mut Organization, group: u32) -> Result<Option<StateDescriptor>> {
        self.event(Some(o), "StateGroup", json!(group))?;
        let g = &self.input["state_group"];
        Ok((!g.is_null()).then(|| StateDescriptor {
            byte1c: g["byte1c"] != 0,
            byte1d: g["byte1d"] != 0,
            byte1e: g["byte1e"] != 0,
        }))
    }
    fn actor_value(&mut self, o: &mut Organization, actor: u32, _: Precision) -> Result<Binary> {
        self.value(Some(o), actor)
    }
    fn definition4f4(&self, actor: u32) -> Result<F32> {
        if actor == 1 {
            return Ok(f(&self.input["actor"]["definition4f4"]));
        }
        Ok(F32(self
            .view(actor)
            .get("definition4f4")
            .map_or(0, |v| v.as_u64().unwrap() as u32)))
    }
    fn organization_maximum(&self, actor: u32) -> Result<F32> {
        self.view(actor)
            .get("organization_maximum")
            .map(f)
            .ok_or(Error::Invalid("no organization"))
    }
    fn member_slots(&self, _: &Organization) -> Result<i32> {
        Ok(self.input["organization"]["members_total"]
            .as_i64()
            .unwrap() as i32)
    }
    fn engage_target(&mut self, o: &mut Organization) -> Result<Option<u32>> {
        let t = self.input["args"]["engage_target"].as_u64().unwrap() as u32;
        self.event(Some(o), "EngageTarget", json!(t))?;
        Ok((t != 0).then_some(t))
    }
    fn target_eligible(&mut self, actor: u32) -> Result<bool> {
        let r = self.input["args"]["target_eligible"] == true;
        self.event(None, "TargetEligible", json!([actor, r]))?;
        Ok(r)
    }
    fn award(&mut self, o: &mut Organization, actor: u32, r: Request) -> Result<()> {
        self.event(
            Some(o),
            "Award",
            json!([actor, r.amount.0, r.argument1 as u8, r.argument2 as u8]),
        )
    }
}

impl experience_amounts::World for Host {
    fn actor_value(&mut self, actor: Object, _: Precision) -> aa_body::Result<Binary> {
        self.value(None, actor.0)
            .map_err(|e| aa_body::Error::Callback(e.to_string()))
    }
    fn definition4f4(&self, actor: Object) -> aa_body::Result<F32> {
        Ok(World::definition4f4(self, actor.0).unwrap())
    }
}

impl CaptureWorld for Host {
    fn actor(&self, actor: u32) -> Result<Actor> {
        Ok(self.actor_view(actor))
    }
    fn has_mind(&self, actor: u32) -> Result<bool> {
        assert_eq!(actor, 1);
        Ok(self.mind.is_some())
    }
    fn award(&mut self, actor: u32, r: Request) -> Result<()> {
        self.event(
            None,
            "Award",
            json!([actor, r.amount.0, r.argument1 as u8, r.argument2 as u8]),
        )
    }
    fn notify_capture(&mut self, _: u32, captured: u32) -> Result<()> {
        self.event(None, "CaptureNotify", json!(captured))
    }
}

fn run(input: &Value, host: &mut Host) -> Result<(Value, Organization)> {
    let p: Precision = serde_json::from_value(input["precision"].clone()).unwrap();
    let s = settings(input);
    let a = &input["args"];
    let mut o = organization(input);
    let mut result = Value::Null;
    match input["operation"].as_str().unwrap() {
        "state" => result = json!(morale::state(&mut o, host)?),
        "gain" => morale::gain(&mut o, f(&a["amount"]), host, p)?,
        "loss" => morale::lose(
            &mut o,
            f(&a["amount"]),
            a["kind"].as_u64().unwrap() as u32,
            host,
            p,
        )?,
        "modify" => morale::modify(
            &mut o,
            f(&a["amount"]),
            a["kind"].as_u64().unwrap() as u32,
            host,
            p,
        )?,
        "slayer" => morale::slayer(&mut o, 2, 3, &s, host, p)?,
        "slayer_dispatch" => {
            if let Some(actor) = morale::slayer_organization(2, 3, host)? {
                assert_eq!(actor, 1);
                morale::slayer(&mut o, 2, 3, &s, host, p)?;
            }
        }
        "breaker" => morale::breaker(&mut o, 2, &s, host, p)?,
        "retreat" => morale::retreat_penalty(&mut o, &s, host, p)?,
        "tick" => morale::tick(&mut o, f(&a["delta"]), &s, host, p)?,
        "attack_efficiency" => morale::attack_efficiency(&mut o, f(&a["delta"]), &s, p)?,
        "effect" => {
            let target = a["target"].as_u64().unwrap() as u32;
            let selected = morale::effect_organization((target != 0).then_some(target), host)?;
            result = json!(selected.is_some() as u8);
            if let Some(actor) = selected {
                assert_eq!(actor, 1);
                let effect = MoraleEffect {
                    kind: a["effect_kind"].as_u64().unwrap() as u32,
                    amount: f(&a["effect_amount"]),
                };
                morale::apply_effect(&mut o, effect, f(&a["scale"]), host, p)?;
            }
        }
        "engage_experience" => morale::engagement_experience(&mut o, f(&a["delta"]), &s, host, p)?,
        "capture_reward" => morale::capture_reward(
            1,
            3,
            a["record_kind"].as_u64().unwrap() as u32,
            &s.experience,
            host,
            p,
        )?,
        other => panic!("{other}"),
    }
    Ok((result, o))
}

fn fixtures() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-morale-fixtures.json"
    ))
    .unwrap()
}

#[test]
fn original_morale_routines_match_every_fixture() {
    let doc = fixtures();
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 700);
    for row in cases {
        let input = &row["input"];
        let mut host = Host::new(input);
        let (result, o) = run(input, &mut host)
            .unwrap_or_else(|e| panic!("{} {}: {e}", input["name"], input["precision"]));
        let out = &row["output"];
        assert_eq!(
            result, out["result"],
            "{} {}",
            input["name"], input["precision"]
        );
        assert_eq!(
            snapshot(&o),
            out["organization"],
            "{} {}",
            input["name"],
            input["precision"]
        );
        assert_eq!(
            json!(host.events),
            out["events"],
            "{} {}",
            input["name"],
            input["precision"]
        );
        assert!(host.mutations.is_empty());
    }
}

#[test]
fn missing_mind_at_native_null_dereference_is_an_error_after_prior_writes() {
    // 579660 with morale ending inside (0, breakpoint] and no Mind: the native
    // state query dereferences a null Mind. Rust reports it, keeping the write.
    let doc = fixtures();
    let mut input = doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["input"]["operation"] == "loss" && !r["input"]["mind"].is_null())
        .unwrap()["input"]
        .clone();
    input["mind"] = Value::Null;
    input["organization"]["morale"] = json!(30f32.to_bits());
    input["args"]["amount"] = json!(15f32.to_bits());
    input["modifier"] = Value::Null;
    input["game_mode"] = json!(0);
    let mut host = Host::new(&input);
    let p = Precision::Extended64;
    let mut o = organization(&input);
    let error = morale::lose(&mut o, F32(15f32.to_bits()), 0, &mut host, p).unwrap_err();
    assert!(matches!(error, Error::Invalid(_)));
    assert_eq!(o.morale, F32(15f32.to_bits()));
}

#[test]
fn failed_state_transition_keeps_the_committed_morale() {
    let doc = fixtures();
    let input = doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["input"]["name"]
                .as_str()
                .unwrap()
                .starts_with("gain_50_10_100")
        })
        .unwrap()["input"]
        .clone();
    let mut host = Host::new(&input);
    host.fail_on = Some("SetState");
    let mut o = organization(&input);
    assert!(
        morale::gain(
            &mut o,
            F32(10f32.to_bits()),
            &mut host,
            Precision::Extended64
        )
        .is_err()
    );
    assert_eq!(o.morale, F32(60f32.to_bits()));
}
