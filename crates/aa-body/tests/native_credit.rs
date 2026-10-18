use aa_body::credit::{self, Event, Object, SourceFields, State, StatsHandles, World};
use aa_body::{F32, Result};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// Test storage for the 58D950 relation map (the live store is aa-economy's
/// `RelationStatistics`, same arithmetic).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct RelationStat {
    values: BTreeMap<Object, F32>,
    maximum: F32,
    leader: Option<Object>,
}
fn increment_relation(
    stat: &mut RelationStat,
    other: Object,
    amount: F32,
    p: Precision,
) -> Result<()> {
    let n = |f: F32| Binary::from_f32_bits(f.0).unwrap();
    let before = stat.values.get(&other).copied().unwrap_or(F32::ZERO);
    let sum = n(amount).add(&n(before), p)?;
    let stored = F32(sum.store_f32()?);
    stat.values.insert(other, stored);
    if sum.compare(&n(stat.maximum)) == Ordering::Greater {
        stat.maximum = stored;
        stat.leader = Some(other);
    }
    Ok(())
}

struct Host {
    case: Value,
    events: Vec<Value>,
    last_state: State,
    stats: BTreeMap<Object, Vec<F32>>,
    relations: BTreeMap<Object, RelationStat>,
    player_index: usize,
}
impl Host {
    fn new(case: &Value, state: State) -> Self {
        Self {
            case: case.clone(),
            events: vec![],
            last_state: state,
            stats: serde_json::from_value(case["stats"].clone()).unwrap(),
            relations: serde_json::from_value(case["relations"].clone()).unwrap(),
            player_index: 0,
        }
    }
    fn observe(&mut self, event: Value, state: &mut State) {
        self.events.push(json!({"event": event, "state": state}));
        let name = event
            .as_str()
            .unwrap_or_else(|| event.as_object().unwrap().keys().next().unwrap());
        if self.case["mutations"]
            .as_array()
            .unwrap()
            .first()
            .is_some_and(|m| m["on"] == name)
        {
            let mutation = self.case["mutations"].as_array_mut().unwrap().remove(0);
            let mut current = serde_json::to_value(&*state).unwrap();
            for field in ["victim", "settings"] {
                if let Some(updates) = mutation[field].as_object() {
                    for (k, v) in updates {
                        current[field][k] = v.clone();
                    }
                }
            }
            *state = serde_json::from_value(current).unwrap();
            if let Some(updates) = mutation["world"].as_object() {
                for (k, v) in updates {
                    self.case[k] = v.clone();
                }
            }
        }
        self.last_state = state.clone();
    }
    fn player(&self) -> Option<Object> {
        self.case["players"]
            .as_array()
            .unwrap()
            .get(self.player_index)
            .map(|p| Object(p.as_u64().unwrap() as u32))
    }
}
impl World for Host {
    fn resolve(&self, id: u32) -> Result<Option<Object>> {
        Ok(if id >> 16 == 1 {
            self.case["registry"][(id & 65535).to_string()]
                .as_u64()
                .map(|x| Object(x as u32))
        } else {
            None
        })
    }
    fn source_accepts_victim(&mut self, source: Object, state: &mut State) -> Result<bool> {
        self.observe(json!({"SourceAccepts":{"source":source}}), state);
        Ok(self.case["source_accepts"].as_bool().unwrap())
    }
    fn hostile(&mut self, other: Object, state: &mut State) -> Result<bool> {
        self.observe(json!({"Hostile":{"other":other}}), state);
        Ok(self.case["hostile"][other.0.to_string()]
            .as_bool()
            .unwrap_or(false))
    }
    fn outer_actor(&mut self, state: &mut State) -> Result<Option<Object>> {
        self.observe(json!("Outer"), state);
        Ok(self.case["outer"].as_u64().map(|x| Object(x as u32)))
    }
    fn actor_flags(&self, _: Object) -> Result<u32> {
        Ok(self.case["outer_flags"].as_u64().unwrap() as u32)
    }
    fn virtual20(&mut self, argument: bool, state: &mut State) -> Result<bool> {
        self.observe(json!({"Virtual20":{"argument":argument}}), state);
        Ok(self.case["virtual20"][argument.to_string()]
            .as_bool()
            .unwrap())
    }
    fn begin_players(&mut self, state: &mut State) -> Result<Option<Object>> {
        self.observe(json!("BeginPlayers"), state);
        self.player_index = 0;
        Ok(self.player())
    }
    fn next_player(&mut self, state: &mut State) -> Result<Option<Object>> {
        self.observe(json!("NextPlayer"), state);
        self.player_index += 1;
        Ok(self.player())
    }
    fn visible_to(&mut self, player: Object, argument: bool, state: &mut State) -> Result<bool> {
        self.observe(
            json!({"Visible":{"player":player,"argument":argument}}),
            state,
        );
        Ok(self.case["visible"][player.0.to_string()]
            .as_bool()
            .unwrap_or(false))
    }
    fn victim_label(&mut self, state: &mut State) -> Result<String> {
        self.observe(json!("VictimLabel"), state);
        Ok("victim".into())
    }
    fn owner_label(&mut self, owner: Object, state: &mut State) -> Result<String> {
        self.observe(json!({"OwnerLabel":{"owner":owner}}), state);
        Ok("owner".into())
    }
    fn allegiance(&mut self, state: &mut State) -> Result<Option<Object>> {
        self.observe(json!("Allegiance"), state);
        Ok(self.case["allegiance"].as_u64().map(|x| Object(x as u32)))
    }
    fn stats_handles(&self, owner: Object) -> Result<StatsHandles> {
        Ok(serde_json::from_value(self.case["handles"][owner.0.to_string()].clone()).unwrap())
    }
    fn source_fields(&self, _: Object) -> Result<SourceFields> {
        Ok(serde_json::from_value(self.case["source_fields"].clone()).unwrap())
    }
    fn add_statistic(
        &mut self,
        component: Object,
        index: u32,
        amount: F32,
        precision: Precision,
    ) -> Result<()> {
        self.events.push(
            json!({"event":{"Stat":{"component":component,"index":index}},"state":self.last_state}),
        );
        let value = &mut self.stats.get_mut(&component).unwrap()[index as usize];
        aa_body::credit::increment_stat(value, amount, precision)
    }
    fn add_relation(
        &mut self,
        component: Object,
        index: u32,
        other: Option<Object>,
        amount: F32,
        precision: Precision,
    ) -> Result<()> {
        let other = other.unwrap();
        self.events.push(json!({"event":{"Relation":{"component":component,"index":index,"other":other.0}},"state":self.last_state}));
        increment_relation(
            self.relations.get_mut(&component).unwrap(),
            other,
            amount,
            precision,
        )
    }
    fn actor_value(
        &mut self,
        actor: Object,
        state: &mut State,
        precision: Precision,
    ) -> Result<Binary> {
        self.observe(json!({"ActorValue":{"actor":actor}}), state);
        let pair = &self.case["values"][actor.0.to_string()];
        Ok(
            Binary::from_f32_bits(pair[0].as_u64().unwrap() as u32)?.add(
                &Binary::from_f32_bits(pair[1].as_u64().unwrap() as u32)?,
                precision,
            )?,
        )
    }
    fn event(&mut self, event: Event, state: &mut State) -> Result<()> {
        let fail =
            matches!(event, Event::AwardExperience { .. }) && self.case["fail_award"] == true;
        self.observe(serde_json::to_value(event).unwrap(), state);
        if fail {
            return Err(aa_body::Error::Callback("fixture award failure".into()));
        }
        Ok(())
    }
}

#[test]
fn original_kill_credit_order_and_bits() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-credit/credit-native-oracle.json"
    ))
    .unwrap();
    let rows = doc["fixtures"].as_array().unwrap();
    assert_eq!(rows.len(), 186);
    for row in rows {
        let c = &row["input"];
        let mut state: State = serde_json::from_value(c["state"].clone()).unwrap();
        let p = match c["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => unreachable!(),
        };
        let mut world = Host::new(c, state.clone());
        let source = credit::credit_kill(&mut state, &mut world, p).unwrap();
        let result = json!({"source":source,"state":state,"events":world.events,"stats":world.stats,"relations":world.relations,"abi_preserved":true});
        assert_eq!(result, row["output"], "{}", c["name"]);
        assert!(world.case["mutations"].as_array().unwrap().is_empty());
    }
}

#[test]
fn relation_maximum_uses_unstored_sum() {
    let mut stat = RelationStat {
        values: BTreeMap::from([(Object(3), F32(0x4b80_0000))]),
        maximum: F32(0x4b80_0000),
        leader: Some(Object(4)),
    };
    increment_relation(&mut stat, Object(3), F32::ONE, Precision::Extended64).unwrap();
    assert_eq!(stat.values[&Object(3)], F32(0x4b80_0000));
    assert_eq!(stat.maximum, F32(0x4b80_0000));
    assert_eq!(stat.leader, Some(Object(3)));
}

#[test]
fn callback_failure_preserves_credit_and_prior_statistics() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-credit/credit-native-oracle.json"
    ))
    .unwrap();
    let row = doc["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["input"]["name"] == "all_classes_Double53")
        .unwrap();
    let mut case = row["input"].clone();
    case["fail_award"] = json!(true);
    let mut state: State = serde_json::from_value(case["state"].clone()).unwrap();
    let mut world = Host::new(&case, state.clone());
    let error = credit::credit_kill(&mut state, &mut world, Precision::Double53).unwrap_err();
    assert!(error.to_string().contains("fixture award failure"));
    assert_ne!(state.victim.flags & 0x10, 0);
    assert_eq!(world.stats[&Object(101)][5], F32::ONE);
    assert_eq!(world.stats[&Object(102)][4], F32::ONE);
    assert!(world.events.iter().all(|e| e["event"] != "DeathTrigger"));
    let event_count = world.events.len();
    assert_eq!(
        credit::credit_kill(&mut state, &mut world, Precision::Double53).unwrap(),
        None
    );
    assert_eq!(world.events.len(), event_count);
    // Reconstructing a valid world is required after failure; clearing only the
    // credited bit would double-count those successful earlier increments.
}
