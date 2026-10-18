use aa_economy::{
    EconomyError, F32Bits,
    kingdom_registry::{
        RecruitAi, RecruitItemTable,
        item_fields::{BaseItem, CountedItem, Item, ItemInput},
    },
    tick::RecruitSingle,
};
use serde_json::{Value, json};
struct Input<'a> {
    case: &'a Value,
    events: Vec<&'static str>,
}
impl Input<'_> {
    fn event(&mut self, name: &'static str) -> Result<(), EconomyError> {
        if self.case["stop"].as_u64() == Some(self.events.len() as u64) {
            return Err(EconomyError::ResourceIndex);
        }
        self.events.push(name);
        Ok(())
    }
}
impl ItemInput for Input<'_> {
    fn definition(&mut self) -> Result<u32, EconomyError> {
        self.event("definition")?;
        Ok(1)
    }
    fn enabled8(&mut self) -> Result<u8, EconomyError> {
        self.event("enabled")?;
        Ok(self.case["enabled"].as_u64().unwrap() as u8)
    }
    fn count_c(&mut self) -> Result<u32, EconomyError> {
        self.event("count")?;
        Ok(self.case["count"].as_u64().unwrap() as u32)
    }
}
fn fields(item: &Item) -> Value {
    match item {
        Item::Base(v) => json!({"definition":v.definition(),"enabled":v.enabled8(),"count":null}),
        Item::Counted(v) => {
            json!({"definition":v.definition(),"enabled":v.enabled8(),"count":v.count_c()})
        }
        _ => panic!("nonSingle probe"),
    }
}
#[test]
fn base_counted_native_constructors_load_prefixes_and_queries_all_pc() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-recruit-eligibility/native-item-fields.json"
    ))
    .unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let case = &row["input"];
        let expected = &row["output"];
        let mut input = Input {
            case,
            events: vec![],
        };
        let mut table = RecruitItemTable::default();
        let (retained, loaded) = if case["kind"] == "base" {
            let mut item = BaseItem::from_init_constructor(1);
            assert_eq!(
                json!({"definition":item.definition(),"enabled":item.enabled8(),"count":null}),
                expected["constructor"]
            );
            let loaded = item.load_base(&mut input);
            (Item::Base(item), loaded)
        } else {
            let mut item = CountedItem::from_load_constructor();
            assert_eq!(
                json!({"definition":item.definition(),"enabled":item.enabled8(),"count":item.count_c()}),
                expected["constructor"]
            );
            let loaded = item.load_4e79f0(&mut input);
            (Item::Counted(item), loaded)
        };
        assert_eq!(
            loaded.is_err(),
            expected["stopped"].as_bool().unwrap(),
            "{row}"
        );
        let actual = if loaded.is_ok() {
            match retained {
                Item::Base(v) => table.add_base(v),
                Item::Counted(v) => table.add_counted(v),
                _ => unreachable!(),
            }
            let result = table
                .query_item(
                    1,
                    case["wanted"].as_i64().unwrap() as i32,
                    case["restore"].as_bool().unwrap(),
                )
                .unwrap();
            assert_eq!(
                u32::from(result),
                expected["result"].as_u64().unwrap() as u32,
                "{row}"
            );
            fields(&table.items()[0])
        } else {
            //The failing Load keeps the SAME constructed allocation outside
            //the hash; it must not publish a partially loaded definition.
            assert_eq!(table.entries().len(), 0);
            fields(&retained)
        };
        for name in ["definition", "enabled", "count"] {
            assert_eq!(actual[name], expected[name], "{name}: {row}");
        }
        assert_eq!(json!(input.events), expected["events"], "{row}");
    }
}
struct Ai {
    fail: bool,
}
impl RecruitAi for Ai {
    fn ai_enabled(&mut self) -> Result<bool, EconomyError> {
        Ok(true)
    }
    fn ai_listeners(&mut self) -> Result<Vec<u32>, EconomyError> {
        Ok(vec![1])
    }
    fn notify_ai(&mut self, _: u32, _: &RecruitSingle, _: bool) -> Result<(), EconomyError> {
        if self.fail {
            Err(EconomyError::ResourceIndex)
        } else {
            Ok(())
        }
    }
}
fn single(definition: u32) -> RecruitSingle {
    RecruitSingle {
        definition,
        available: true,
        actor: 0,
        organization: 0,
        progress: F32Bits::ZERO,
        regeneration_rate: F32Bits::ZERO,
    }
}
#[test]
fn same_single_links_and_native_add_remove_failure_prefixes() {
    let mut table = RecruitItemTable::default();
    let mut ai = Ai { fail: true };
    assert!(table.add_single(single(1), &mut ai).is_err());
    assert_eq!(table.changes, 0);
    assert_eq!(table.entries().collect::<Vec<_>>(), vec![(1, true)]);
    assert!(table.query_item(1, 1, false).unwrap());
    assert!(!table.singles[0].available);
    ai.fail = false;
    table.add_single(single(2), &mut ai).unwrap();
    table.add_plain(7);
    let changes = table.changes;
    ai.fail = true;
    assert!(table.remove_single(0, true, &mut ai).is_err());
    assert_eq!(table.changes, changes);
    assert_eq!(
        table.entries().collect::<Vec<_>>(),
        vec![(2, true), (7, false)]
    );
    assert_eq!(table.singles.len(), 2);
    assert!(table.query_item(2, 1, true).unwrap());
    //Separate successful owner demonstrates the native vector-index rewrite.
    let mut table = RecruitItemTable::default();
    ai.fail = false;
    table.add_single(single(1), &mut ai).unwrap();
    table.add_single(single(2), &mut ai).unwrap();
    table.add_plain(7);
    table.remove_single(0, true, &mut ai).unwrap();
    assert_eq!(table.find_single(2, true), Some(0));
    assert!(table.query_item(2, 1, true).unwrap());
    assert!(table.query_item(7, i32::MAX, true).unwrap());
    table.clear();
    assert_eq!(table.entries().len(), 0);
    assert!(table.singles.is_empty());
}
