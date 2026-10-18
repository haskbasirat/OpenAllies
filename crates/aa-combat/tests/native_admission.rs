use aa_body::{F32, properties::ModifierState};
use aa_combat::{Error, Result, ability::Ability, admission::*, performer::Context};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
struct Host {
    input: Value,
    events: Vec<Value>,
    precision: Precision,
}
impl Host {
    fn flag(&self, k: &str) -> bool {
        self.input[k].as_bool().unwrap()
    }
    fn bits(&self, k: &str) -> F32 {
        F32(self.input[k].as_u64().unwrap() as u32)
    }
    fn view(&self, id: u32) -> &Value {
        &self.input["actors"][id.to_string()]
    }
    fn effect(&self, id: u32) -> &Value {
        &self.input["effects"][(id - 100) as usize]
    }
    fn modifier_state(&self) -> ModifierState {
        let mut m = ModifierState::new(0);
        m.scalars[0x64 / 4 - 1] = self.bits("recharge_factor");
        m.scalars[0x28 / 4 - 1] = self.bits("projectile_delta");
        m.scalars[0x2c / 4 - 1] = self.bits("bombard_delta");
        m
    }
}
fn body_error(e: aa_body::Error) -> Error {
    Error::World(e.to_string())
}
impl World for Host {
    fn definition(&self, _: u32) -> Result<Definition> {
        Ok(serde_json::from_value(self.input["definition"].clone()).unwrap())
    }
    fn local_definition(&self, _: u32) -> Result<LocalDefinition> {
        Ok(serde_json::from_value(self.input["local"].clone()).unwrap())
    }
    fn actor(&self, id: u32) -> Result<Actor> {
        Ok(serde_json::from_value(self.view(id).clone()).unwrap())
    }
    fn is_actor(&mut self, id: u32) -> Result<bool> {
        Ok(self.view(id)["is_actor"].as_bool().unwrap())
    }
    fn is_kingdom(&mut self, id: u32) -> Result<bool> {
        Ok(self.view(id)["is_kingdom"].as_bool().unwrap())
    }
    fn allegiance(&mut self, id: u32) -> Result<Option<u32>> {
        Ok(self.view(id)["allegiance"].as_u64().map(|v| v as u32))
    }
    fn friendly(&mut self, _: u32, _: Option<u32>) -> Result<bool> {
        Ok(self.flag("friendly"))
    }
    fn allied(&mut self, _: u32, _: Option<u32>) -> Result<bool> {
        Ok(self.flag("allied"))
    }
    fn class_flags(&mut self, id: u32) -> Result<u32> {
        Ok(self.view(id)["class_flags"].as_u64().unwrap() as u32)
    }
    fn injured(&mut self, _: u32) -> Result<bool> {
        Ok(self.flag("injured"))
    }
    fn organization_leader(&mut self, id: u32) -> Result<Option<u32>> {
        Ok(self.view(id)["leader"].as_u64().map(|v| v as u32))
    }
    fn summoner_accepts(&mut self, _: u32, _: u32) -> Result<bool> {
        Ok(self.flag("summoner_accepts"))
    }
    fn modifier(&mut self, id: u32) -> Result<Option<u32>> {
        Ok(self.actor(id)?.has_properties.then_some(id))
    }
    fn recharge_modified(&mut self, _: u32, b: F32) -> Result<Binary> {
        self.modifier_state()
            .recharge(b, self.precision)
            .map_err(body_error)
    }
    fn projectile_range_modified(&mut self, _: u32, b: F32) -> Result<Binary> {
        let v = self
            .modifier_state()
            .projectile_range(b, self.bits("minimum_range_setting"), self.precision)
            .map_err(body_error)?;
        Ok(Binary::from_f32_bits(v.0)?)
    }
    fn bombard_range_modified(&mut self, _: u32, b: F32) -> Result<Binary> {
        let v = self
            .modifier_state()
            .bombard_range(b, self.bits("minimum_range_setting"), self.precision)
            .map_err(body_error)?;
        Ok(Binary::from_f32_bits(v.0)?)
    }
    fn effect_kind(&self, e: u32) -> Result<u32> {
        Ok(self.effect(e)["kind"].as_u64().unwrap() as u32)
    }
    fn effect_property(&mut self, e: u32) -> Result<Option<u32>> {
        Ok(self.effect(e)["property"].as_bool().unwrap().then_some(e))
    }
    fn property_immunity_class(&self, p: u32) -> Result<u32> {
        Ok(self.effect(p)["immunity_class"].as_u64().unwrap() as u32)
    }
    fn immune_to(&mut self, _: u32, c: u32) -> Result<bool> {
        Ok(self.input["immune_classes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_u64() == Some(0) || v.as_u64() == Some(u64::from(c))))
    }
    fn effects_for(&mut self, _: u32, _: Option<u32>) -> Result<Vec<u32>> {
        Ok(self.definition(10)?.effects)
    }
    fn has_property(&mut self, _: u32, _: Option<u32>) -> Result<bool> {
        Ok(self.flag("already_has_property"))
    }
    fn mind_allows_full_health_heal(&mut self, _: u32) -> Result<bool> {
        Ok(self.flag("heal_override"))
    }
    fn range_test(&mut self, _: &Context, r: F32, inverse: bool) -> Result<bool> {
        self.events
            .push(json!({"name":"range","range":r,"inverse":inverse}));
        Ok(self.flag(if inverse { "min_pass" } else { "max_pass" }))
    }
    fn turret_covers(&mut self, _: u32, _: u32, _: [F32; 2]) -> Result<bool> {
        self.events.push(json!({"name":"turret"}));
        Ok(self.flag("turret_pass"))
    }
    fn can_pay(&mut self, _: u32, _: &Ability) -> Result<bool> {
        self.events.push(json!({"name":"payment"}));
        Ok(self.flag("payment"))
    }
    fn resolve_id(&mut self, id: u32) -> Result<Option<u32>> {
        Ok(self.input["live_ids"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_u64() == Some(u64::from(id)))
            .then_some(2))
    }
    fn owned_actors(&mut self, _: u32) -> Result<Vec<u32>> {
        Ok(serde_json::from_value(self.input["owned"].clone()).unwrap())
    }
    fn summon_filter(&self, e: u32) -> Result<Filter> {
        Ok(serde_json::from_value(self.effect(e)["summon_filter"].clone()).unwrap())
    }
    fn free_recharge_enabled(&self) -> Result<bool> {
        Ok(self.flag("free_recharge"))
    }
    fn now(&self) -> Result<F32> {
        Ok(self.bits("now"))
    }
}
#[test]
fn native_ability_admission_target_masks_range_gates_and_recharge_match() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-ability/native-admission-fixtures.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 186);
    for row in rows {
        let input = &row["input"];
        let mut ability: Ability = serde_json::from_value(input["ability"].clone()).unwrap();
        let mut context: Context = serde_json::from_value(input["context"].clone()).unwrap();
        let precision: Precision = serde_json::from_value(input["precision"].clone()).unwrap();
        let mut host = Host {
            input: input.clone(),
            events: vec![],
            precision,
        };
        let code = can_use(
            &mut ability,
            &mut context,
            input["check_range"].as_bool().unwrap(),
            &mut host,
            precision,
        )
        .unwrap_or_else(|e| panic!("{} {e}", input["name"]));
        assert_eq!(
            json!({"code":code,"context":context,"summoned":ability.summoned_ids,"events":host.events}),
            row["output"],
            "{}",
            input["name"]
        );
    }
}

#[test]
fn native_fuzzy_range_preserves_modified_base_and_extended_residuals() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-ability-motion/native-range-fixtures.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 144);
    for row in rows {
        let input = &row["input"];
        let ability: Ability = serde_json::from_value(input["ability"].clone()).unwrap();
        let precision: Precision = serde_json::from_value(input["precision"].clone()).unwrap();
        let mut host = Host {
            input: input.clone(),
            events: vec![],
            precision,
        };
        let modifier = (input["range_modifier"] == true).then_some(1);
        let mut value = effective_range(&ability, modifier, &mut host, precision).unwrap();
        let mut stores = vec![];
        for _ in 0..3 {
            let b = value.store_f32().unwrap();
            stores.push(b);
            value = value
                .sub(&Binary::from_f32_bits(b).unwrap(), precision)
                .unwrap();
        }
        assert_eq!(json!(stores), row["output"], "{}", input["name"]);
        assert!(host.events.is_empty());
    }
}

#[test]
fn native_performer_maximum_range_uses_enabled_entries_and_binary32_maximum() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-moving-target/native-maximum-fixtures.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 108);
    for row in rows {
        let c = &row["input"];
        let precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut host = Host {
            input: c.clone(),
            events: vec![],
            precision,
        };
        let abilities: Vec<Ability> = serde_json::from_value(c["abilities"].clone()).unwrap();
        assert_eq!(
            u64::from(
                maximum_range(&abilities, 1, &mut host, precision)
                    .unwrap()
                    .store_f32()
                    .unwrap()
            ),
            row["output"].as_u64().unwrap(),
            "{}",
            c["name"]
        );
    }
}
