//! Original 570020 oracle: value getters are explicit fixtures; resource vector
//! operations and both Kingdoms' coupled arithmetic execute in original bytes.
use aa_actors::{ActorId, ground_states::EconomyState};
use aa_body::F32;
use aa_economy::{Catalog, Economy, Limits, State, X87Precision};
use aa_numeric::Precision;
use aa_runtime::economy_refresh::{self, Error, Result, World};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Deserialize)]
struct Input {
    name: String,
    precision: Precision,
    catalog: Catalog,
    kingdoms: Vec<State>,
    limits: Limits,
    resources: [Vec<F32>; 4],
    flags: [u8; 2],
    block_reason: String,
    parent: bool,
    owners: Vec<Option<usize>>,
    provides: Vec<bool>,
    upkeep: Vec<F32>,
    provided: Vec<F32>,
    cost: Vec<F32>,
    mutations: Vec<Mutation>,
}
#[derive(Clone, Deserialize)]
struct Mutation {
    on: String,
    #[serde(default)]
    call: usize,
    flags: Option<[u8; 2]>,
    #[serde(default)]
    resources: BTreeMap<usize, Vec<F32>>,
    parent: Option<bool>,
}
#[derive(Deserialize)]
struct Case {
    input: Input,
    output: Value,
}
#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
    equality_cases: Vec<Equality>,
}
#[derive(Deserialize)]
struct Equality {
    precision: Precision,
    a: Vec<F32>,
    b: Vec<F32>,
    equal: bool,
}
fn fixture() -> Fixture {
    serde_json::from_str(include_str!(
        "../../../research/phase7-economy-refresh/native-refresh-fixtures.json"
    ))
    .unwrap()
}
fn view(c: &EconomyState) -> Value {
    json!({"flags":[u8::from(c.flag_10),u8::from(c.flag_11)],
        "resources":c.resources.iter().map(|v| &v.values).collect::<Vec<_>>()})
}
struct Adapter {
    input: Input,
    kingdoms: Vec<Economy>,
    parent: bool,
    events: Vec<Value>,
    calls: BTreeMap<String, usize>,
    fail: Option<&'static str>,
}
impl Adapter {
    fn new(input: Input) -> (EconomyState, Self) {
        let mut component =
            EconomyState::new(ActorId::from_raw(1), input.catalog.resources.len()).unwrap();
        for (target, values) in component.resources.iter_mut().zip(&input.resources) {
            target.values.clone_from(values);
        }
        component.flag_10 = input.flags[0] != 0;
        component.flag_11 = input.flags[1] != 0;
        let precision = match input.precision {
            Precision::Single24 => X87Precision::Single24,
            Precision::Double53 => X87Precision::Double53,
            Precision::Extended64 => X87Precision::Extended64,
        };
        let kingdoms = input
            .kingdoms
            .iter()
            .map(|state| {
                Economy::new(
                    input.catalog.clone(),
                    state.clone(),
                    input.limits,
                    precision,
                )
                .unwrap()
            })
            .collect();
        let parent = input.parent || input.block_reason == "linked";
        (
            component,
            Self {
                input,
                kingdoms,
                parent,
                events: Vec::new(),
                calls: BTreeMap::new(),
                fail: None,
            },
        )
    }
    fn index(&self, name: &str) -> usize {
        self.calls.get(name).copied().unwrap_or_default()
    }
    fn event(
        &mut self,
        name: &str,
        c: Option<&mut EconomyState>,
        value: Option<Value>,
    ) -> Result<()> {
        let mut event = json!({"name":name});
        if let Some(c) = c.as_ref() {
            event["component"] = view(c);
        }
        if let Some(value) = value {
            event["value"] = value;
        }
        self.events.push(event);
        let call = self.calls.entry(name.to_owned()).or_default();
        let index = *call;
        *call += 1;
        if let Some(c) = c {
            for mutation in &self.input.mutations {
                if mutation.on != name || mutation.call != index {
                    continue;
                }
                if let Some(flags) = mutation.flags {
                    c.flag_10 = flags[0] != 0;
                    c.flag_11 = flags[1] != 0;
                }
                for (index, values) in &mutation.resources {
                    c.resources[*index].values.clone_from(values);
                }
                if let Some(parent) = mutation.parent {
                    self.parent = parent;
                }
            }
        }
        if self.fail == Some(name) {
            Err(Error::World(format!("injected {name} failure")))
        } else {
            Ok(())
        }
    }
    fn output(&self, c: &EconomyState) -> Value {
        json!({"component":view(c),"kingdoms":self.kingdoms.iter().map(Economy::state).collect::<Vec<_>>(),
            "events":self.events,"abi_preserved":true})
    }
}
impl World for Adapter {
    type Kingdom = usize;
    fn resource_count(&self) -> usize {
        self.input.catalog.resources.len()
    }
    fn owner(&mut self, actor: ActorId, c: &mut EconomyState) -> Result<Option<usize>> {
        assert_eq!(actor, ActorId::from_raw(1));
        let value = self.input.owners[self.index("Owner").min(self.input.owners.len() - 1)];
        self.event("Owner", Some(c), value.map(|v| json!(v)))?;
        Ok(value)
    }
    fn upkeep_blocked(&mut self, _: ActorId, c: &mut EconomyState) -> Result<bool> {
        self.event("UpkeepBlocked", Some(c), None)?;
        Ok(self.parent
            || self.input.block_reason == "destructed"
            || self.input.block_reason == "nonpaying_organization")
    }
    fn upkeep_values(
        &mut self,
        _: ActorId,
        c: &mut EconomyState,
        pc: Precision,
    ) -> Result<Vec<F32>> {
        assert_eq!(pc, self.input.precision);
        self.event("UpkeepValues", Some(c), None)?;
        Ok(self.input.upkeep.clone())
    }
    fn provides(&mut self, _: ActorId, c: &mut EconomyState) -> Result<bool> {
        let value = self.input.provides[self.index("Provides").min(self.input.provides.len() - 1)];
        self.event("Provides", Some(c), Some(json!(value)))?;
        Ok(value)
    }
    fn provided_values(
        &mut self,
        _: ActorId,
        c: &mut EconomyState,
        write_member_cache: bool,
        pc: Precision,
    ) -> Result<Vec<F32>> {
        assert_eq!(pc, self.input.precision);
        assert!(write_member_cache);
        self.event("ProvidedValues", Some(c), Some(json!(write_member_cache)))?;
        Ok(self.input.provided.clone())
    }
    fn cost_values(&mut self, _: ActorId, c: &mut EconomyState, pc: Precision) -> Result<Vec<F32>> {
        assert_eq!(pc, self.input.precision);
        self.event("CostValues", Some(c), None)?;
        Ok(self.input.cost.clone())
    }
    fn kingdom_economy(&mut self, kingdom: usize) -> Result<&mut Economy> {
        self.event("Kingdom", None, Some(json!(kingdom)))?;
        Ok(&mut self.kingdoms[kingdom])
    }
    fn parent_organization(&mut self, _: ActorId, c: &mut EconomyState) -> Result<Option<ActorId>> {
        self.event("Parent", Some(c), None)?;
        Ok(self.parent.then_some(ActorId::from_raw(2)))
    }
    fn refresh_parent(
        &mut self,
        actor: ActorId,
        _current: &mut EconomyState,
        pc: Precision,
    ) -> Result<()> {
        assert_eq!(actor, ActorId::from_raw(2));
        assert_eq!(pc, self.input.precision);
        self.event("RefreshParent", None, Some(json!(2)))
    }
}

#[test]
fn native_refresh_with_real_kingdom_arithmetic() {
    let fixture = fixture();
    assert_eq!(fixture.cases.len(), 159);
    for case in fixture.cases {
        let name = case.input.name.clone();
        let pc = case.input.precision;
        let (mut component, mut world) = Adapter::new(case.input);
        economy_refresh::refresh(&mut component, &mut world, pc).unwrap();
        assert_eq!(world.output(&component), case.output, "{name}");
    }
}

#[test]
fn native_vector_numeric_equality_including_nonfinite_values() {
    let cases = fixture().equality_cases;
    assert_eq!(cases.len(), 450);
    for case in cases {
        assert_eq!(
            economy_refresh::equal(&case.a, &case.b),
            case.equal,
            "{:?} {:?} {:?}",
            case.precision,
            case.a,
            case.b
        );
    }
}

#[test]
fn callback_failure_retains_completed_resource_updates() {
    let case = fixture().cases.remove(0);
    let pc = case.input.precision;
    let (mut component, mut world) = Adapter::new(case.input);
    world.fail = Some("CostValues");
    let original_cost = component.resources[2].values.clone();
    let result = economy_refresh::refresh(&mut component, &mut world, pc);
    assert!(matches!(result, Err(Error::World(_))));
    assert_eq!(component.resources[2].values, original_cost);
    assert_eq!(component.flag_11, case.output["component"]["flags"][1] == 1);
    assert_eq!(component.resources[1].values, world.input.upkeep);
    assert_eq!(
        world.output(&component)["kingdoms"],
        case.output["kingdoms"]
    );
    assert_eq!(world.events.last().unwrap()["name"], "CostValues");
    assert!(!world.events.iter().any(|e| e["name"] == "Parent"));
}

#[test]
fn nonfinite_arithmetic_is_an_explicit_partial_update_error() {
    let mut input = fixture().cases.remove(0).input;
    input.flags = [0, 1];
    input.upkeep[0] = F32(0x7fc00000);
    let pc = input.precision;
    let (mut component, mut world) = Adapter::new(input);
    let initial = world.kingdoms[0].state().clone();
    let result = economy_refresh::refresh(&mut component, &mut world, pc);
    assert!(matches!(
        result,
        Err(Error::Economy(aa_economy::EconomyError::NonFiniteInput))
    ));
    // Old contribution was removed first. Native NaN arithmetic itself is not
    // claimed: the checked implementation stops after assigning the new data.
    assert!(!component.flag_11);
    assert_eq!(component.resources[1].values[0], F32(0x7fc00000));
    assert_ne!(world.kingdoms[0].state(), &initial);
    assert!(!world.events.iter().any(|e| e["name"] == "Provides"));
}
