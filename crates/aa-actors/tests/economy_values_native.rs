use aa_actors::{
    ActorId,
    economy_values::{
        self, ActorSnapshot, Definition, GateWorld, Modifier, Result, VectorKind, World,
    },
};
use aa_body::F32;
use serde_json::{Value, json};
struct Host {
    input: Value,
}
impl GateWorld for Host {
    type Definition = u32;
    fn actor(&self, actor: ActorId) -> Result<ActorSnapshot<u32>> {
        Ok(serde_json::from_value(self.input["actors"][actor.raw().to_string()].clone()).unwrap())
    }
    fn definition(&self, id: &u32) -> Result<Definition<u32>> {
        Ok(serde_json::from_value(self.input["definitions"][id.to_string()].clone()).unwrap())
    }
}
impl World for Host {
    fn resource_count(&self) -> Result<usize> {
        Ok(self.input["resource_count"].as_u64().unwrap() as usize)
    }
    fn actor_modifier(&self, actor: ActorId) -> Result<Option<u32>> {
        Ok(self.input["actor_modifiers"][actor.raw().to_string()]
            .as_u64()
            .map(|v| v as u32))
    }
    fn modifier(&self, id: u32) -> Result<Modifier> {
        Ok(serde_json::from_value(self.input["modifiers"][id.to_string()].clone()).unwrap())
    }
    fn transport_vector(&self, id: u32, kind: VectorKind) -> Result<Vec<F32>> {
        Ok(serde_json::from_value(
            self.input["transports"][id.to_string()][format!("{kind:?}")].clone(),
        )
        .unwrap())
    }
    fn layout_slot_count(&self, id: u32) -> Result<usize> {
        Ok(self.input["organizations"][id.to_string()]["slots"]
            .as_array()
            .unwrap()
            .len())
    }
    fn layout_slot_definition(&self, id: u32, index: usize) -> Result<Option<u32>> {
        Ok(self.input["organizations"][id.to_string()]["slots"][index]
            .as_u64()
            .map(|v| v as u32))
    }
    fn member_count(&self, id: u32) -> Result<usize> {
        Ok(self.input["organizations"][id.to_string()]["members"]
            .as_array()
            .unwrap()
            .len())
    }
    fn member(&self, id: u32, index: usize) -> Result<Option<ActorId>> {
        Ok(
            self.input["organizations"][id.to_string()]["members"][index]
                .as_u64()
                .map(|v| ActorId::from_raw(v as u32)),
        )
    }
    fn store_member_production(&mut self, id: u32, values: &[F32]) -> Result<()> {
        self.input["caches"][id.to_string()] = json!(values);
        Ok(())
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase7-economy-values/native-values.json"
    ))
    .unwrap()
}
#[test]
fn actual_native_economy_gates_modifier_getters_vectors_and_member_caches() {
    let fixture = fixture();
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 516);
    for case in fixture["cases"].as_array().unwrap() {
        let input = &case["input"];
        let mut host = Host {
            input: input.clone(),
        };
        let actor = input["actor"].as_u64().map(|v| ActorId::from_raw(v as u32));
        let definition = input["definition"].as_u64().unwrap() as u32;
        let p = serde_json::from_value(input["precision"].clone()).unwrap();
        let mut out: Vec<F32> = serde_json::from_value(input["initial_output"].clone()).unwrap();
        let predicate = match input["operation"].as_str().unwrap() {
            "eligible" => Some(economy_values::eligible(actor.unwrap(), &host).unwrap()),
            "packed" => Some(economy_values::packed_supply(actor.unwrap(), &host).unwrap()),
            "supplied" => Some(economy_values::supplied(
                &host.actor(actor.unwrap()).unwrap(),
            )),
            "recruitable" => {
                Some(economy_values::recruitable(&host.definition(&definition).unwrap()).unwrap())
            }
            "Cost" => {
                economy_values::cost_or_upkeep(&mut out, actor, VectorKind::Cost, &host, p)
                    .unwrap();
                None
            }
            "Upkeep" => {
                economy_values::cost_or_upkeep(&mut out, actor, VectorKind::Upkeep, &host, p)
                    .unwrap();
                None
            }
            "Production" => {
                economy_values::production(
                    &mut out,
                    actor,
                    input["cache_members"].as_bool().unwrap(),
                    &mut host,
                    p,
                )
                .unwrap();
                None
            }
            op if op.starts_with("definition_") => {
                let kind = serde_json::from_value(json!(&op[11..])).unwrap();
                let context = input["context"].as_u64().map(|v| v as u32);
                economy_values::definition_values(&mut out, &definition, kind, context, &host, p)
                    .unwrap();
                None
            }
            _ => unreachable!(),
        };
        assert_eq!(
            json!({"predicate":predicate,"values":out,"caches":host.input["caches"]}),
            case["output"],
            "{}",
            input["name"]
        );
    }
}
#[test]
fn invalid_organization_cycles_and_missing_layout_data_fail_explicitly() {
    let fixture = fixture();
    let mut host = Host {
        input: fixture["cases"][0]["input"].clone(),
    };
    host.input["actors"]["1"]["organization"] = json!(1);
    host.input["organizations"]["1"]["members"] = json!([1]);
    let mut output = vec![];
    let error = economy_values::production(
        &mut output,
        Some(ActorId::from_raw(1)),
        true,
        &mut host,
        aa_numeric::Precision::Extended64,
    )
    .unwrap_err();
    assert!(matches!(error, economy_values::Error::Cycle(_)));
    assert!(!output.is_empty());
    let mut definition = host.definition(&1).unwrap();
    definition.class178 = 2;
    definition.flags174 = 1 << 7;
    definition.layout_kind18 = None;
    assert!(economy_values::recruitable(&definition).is_err());
    assert!(aa_body::economy_modifiers::production_delta(&[], 0, 0).is_err());
}
