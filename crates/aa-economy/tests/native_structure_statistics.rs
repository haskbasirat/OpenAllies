//! 0x58EF60 / 0x584B70 against
//! research/phase8-combat/native-structure-statistics-fixtures.json.
use aa_economy::{
    EconomyError, F32Bits, X87Precision,
    statistics::Statistics,
    structure_statistics::{StructureStatisticsWorld, set_category, structure_count_lost},
};
use serde_json::{Value, json};

const ACTOR: u32 = 0x77;

struct Adapter<'a> {
    case: &'a Value,
    events: Vec<Value>,
}
impl StructureStatisticsWorld for Adapter<'_> {
    fn statistics_enabled(&self) -> Result<bool, EconomyError> {
        Ok(self.case["enabled"].as_bool().unwrap())
    }
    fn statistic_category(&self, actor: u32) -> Result<u32, EconomyError> {
        assert_eq!(actor, ACTOR);
        Ok(self.case["current"].as_u64().unwrap() as u32)
    }
    fn leave_category(&mut self, actor: u32) -> Result<(), EconomyError> {
        self.events.push(json!({"leave": actor == ACTOR}));
        Ok(())
    }
    fn join_category(
        &mut self,
        actor: u32,
        category: u32,
        amount: F32Bits,
    ) -> Result<(), EconomyError> {
        self.events
            .push(json!({"join": actor == ACTOR, "category": category, "amount": amount.0}));
        Ok(())
    }
}

#[test]
fn structure_statistics_match_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-structure-statistics-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 600);
    for fixture in cases {
        let precision = match fixture["precision"].as_str().unwrap() {
            "Single24" => X87Precision::Single24,
            "Double53" => X87Precision::Double53,
            _ => X87Precision::Extended64,
        };
        let case = &fixture["case"];
        let mut statistics = Statistics::new(10);
        statistics.scores[0x33] = F32Bits(case["score33"].as_u64().unwrap() as u32);
        statistics.scores[0x18] = F32Bits(case["score18"].as_u64().unwrap() as u32);
        let mut world = Adapter { case, events: vec![] };
        if case["function"] == "set_category" {
            set_category(
                &mut statistics,
                ACTOR,
                case["category"].as_u64().unwrap() as u32,
                F32Bits(case["amount"].as_u64().unwrap() as u32),
                &mut world,
                precision,
            )
            .unwrap();
        } else {
            let linked =
                case["kingdom"].as_bool().unwrap() && case["statistics"].as_bool().unwrap();
            structure_count_lost(
                case["component_count"].as_i64().unwrap() as i32,
                case["count"].as_i64().unwrap() as i32,
                ACTOR,
                linked.then_some(&mut statistics),
                &mut world,
                precision,
            )
            .unwrap();
        }
        assert_eq!(Value::Array(world.events), fixture["events"], "{fixture}");
        assert_eq!(u64::from(statistics.scores[0x33].0), fixture["score33"].as_u64().unwrap(), "{fixture}");
        assert_eq!(u64::from(statistics.scores[0x18].0), fixture["score18"].as_u64().unwrap(), "{fixture}");
    }
}
