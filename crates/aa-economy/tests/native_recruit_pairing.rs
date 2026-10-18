//! Replays research/phase8-economy/native_recruit_pairing.py: the original
//! recruit-single pairing 0x4E1200.
use aa_economy::kingdom_registry::{RecruitItemTable, RecruitPairing};
use aa_economy::tick::RecruitSingle;
use aa_economy::{EconomyError, F32Bits};
use serde_json::{Value, json};

const MEMBER: u32 = 7;
const ELEMENT: u32 = 8;

struct Member<'a> {
    case: &'a Value,
    linked: Option<usize>,
}
impl RecruitPairing for Member<'_> {
    fn member_gtid(&mut self, member: u32) -> Result<u32, EconomyError> {
        assert_eq!(member, MEMBER);
        Ok(self.case["member_gtid"].as_u64().unwrap() as u32)
    }
    fn member_element(&mut self, member: u32) -> Result<Option<u32>, EconomyError> {
        assert_eq!(member, MEMBER);
        Ok(self.case["element"].as_bool().unwrap().then_some(ELEMENT))
    }
    fn link_element(&mut self, element: u32, item: usize) -> Result<(), EconomyError> {
        assert_eq!(element, ELEMENT);
        self.linked = Some(item);
        Ok(())
    }
}

#[test]
fn recruit_pairing_matches_original_x86() {
    let document: Value = serde_json::from_str(include_str!(
        "fixtures/native-recruit-pairing-fixtures.json"
    ))
    .unwrap();
    let mut executions = 0;
    for fixture in document["fixtures"].as_array().unwrap() {
        let case = &fixture["case"];
        let word = |k: &str| case[k].as_u64().unwrap() as u32;
        let mut table = RecruitItemTable::default();
        table.singles = vec![
            RecruitSingle {
                definition: 1,
                available: false,
                actor: 0,
                organization: 0,
                progress: F32Bits::ZERO,
                regeneration_rate: F32Bits::ZERO,
            },
            RecruitSingle {
                definition: 2,
                available: true,
                actor: word("item_actor"),
                organization: word("item_organization"),
                progress: F32Bits::ZERO,
                regeneration_rate: F32Bits::ZERO,
            },
        ];
        let mut member = Member { case, linked: None };
        table.pair_single(1, MEMBER, &mut member).unwrap();
        let expected = &fixture["expected"];
        let single = &table.singles[1];
        assert_eq!(json!(single.actor), expected["item_actor"]);
        assert_eq!(json!(single.organization), expected["item_organization"]);
        match expected["element_item_is_item"].as_bool() {
            Some(true) => assert_eq!(member.linked, Some(1)),
            Some(false) => panic!("native did not link the element"),
            None => assert_eq!(member.linked, None),
        }
        assert_eq!(table.singles[0].actor, 0);
        executions += 1;
    }
    assert_eq!(executions, document["executions"].as_u64().unwrap());
}
