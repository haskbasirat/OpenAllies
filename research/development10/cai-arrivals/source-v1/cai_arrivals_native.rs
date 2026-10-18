//!Exact current arrival owner and524B80 tail against paired native fixtures.
use aa_mind::{
    Arena, MindId,
    cai_arrivals::List,
    cai_member_added::{self, MemberAddedWorld},
};
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/development10/cai-arrivals/native-arrivals.json"
    ))
    .unwrap()
}
fn words(v: &Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
        .collect()
}
struct W {
    list: List,
    mode: u32,
    gtid: u32,
    mutation: Option<String>,
    fail: Option<String>,
    trace: Vec<Value>,
    writes: Vec<Value>,
}
impl MemberAddedWorld for W {
    fn resolve(&mut self, _: u32) -> aa_mind::Result<Option<u32>> {
        Ok(None)
    }
    fn mind_word(&mut self, _: MindId, offset: u32, _: &Arena) -> aa_mind::Result<u32> {
        Ok(match offset {
            0x24 => 0x89abcdef,
            0x28 => 0x10001,
            _ => panic!("offset"),
        })
    }
    fn member_mind_3c(&mut self, _: u32, v: u32, _: &mut Arena) -> aa_mind::Result<()> {
        self.trace.push(json!(["3c", v]));
        Ok(())
    }
    fn member_mind_40(&mut self, _: u32, v: Option<u32>, _: &mut Arena) -> aa_mind::Result<()> {
        self.trace.push(json!(["40", v.unwrap_or(0)]));
        Ok(())
    }
    fn helper_522890(&mut self, _: MindId, _: u32, _: u32, _: &mut Arena) -> aa_mind::Result<()> {
        self.trace.push(json!(["522890"]));
        if self.fail.as_deref() == Some("helper") {
            return Err(aa_mind::Error::World("helper".into()));
        }
        match self.mutation.as_deref() {
            Some("front") => self.list.push_front(0x76543210),
            Some("mode3") => self.mode = 3,
            Some("gtid") => self.gtid ^= 0xffffffff,
            _ => {}
        }
        Ok(())
    }
    fn game_mode(&mut self) -> aa_mind::Result<u32> {
        Ok(self.mode)
    }
    fn actor_gtid(&mut self, _: u32) -> aa_mind::Result<u32> {
        Ok(self.gtid)
    }
    fn arrival_push_front(&mut self, _: MindId, g: u32, _: &mut Arena) -> aa_mind::Result<()> {
        self.trace.push(json!(["allocate", 8]));
        if self.fail.as_deref() == Some("allocate") {
            return Err(aa_mind::Error::World("allocate".into()));
        }
        let empty = self.list.is_empty();
        self.list.push_front(g);
        self.writes.push(json!(["head", 1]));
        if empty {
            self.writes.push(json!(["tail", 1]))
        }
        Ok(())
    }
}
#[test]
fn native_push_tail_current_mutation_and_error_prefixes() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 444);
    for row in rows {
        let c = &row["input"];
        let mut w = W {
            list: List::from_saved(words(&c["initial"])),
            mode: c["mode"].as_u64().unwrap() as u32,
            gtid: c["gtid"].as_u64().unwrap() as u32,
            mutation: c["mutation"].as_str().map(str::to_owned),
            fail: c["fail"].as_str().map(str::to_owned),
            trace: vec![],
            writes: vec![],
        };
        let err =
            cai_member_added::member_added(&mut w, MindId(1), 1689, &mut Arena::default()).err();
        let out = json!({"values":w.list,"head":w.list.front(),"tail":w.list.back(),"trace":w.trace,"writes":w.writes,"error":err.map(|_|w.fail.unwrap())});
        assert_eq!(out, row["output"], "{c}");
    }
}
struct Drain {
    list: List,
    current: std::collections::BTreeMap<u32, (u32, bool)>,
    trace: Vec<Value>,
    freed: Vec<u32>,
    fail: Option<u32>,
}
impl aa_mind::cai_arrivals::DrainWorld for Drain {
    fn front(&mut self, _: MindId, _: &Arena) -> aa_mind::Result<Option<u32>> {
        Ok(self.list.front())
    }
    fn resolve(&mut self, g: u32) -> aa_mind::Result<Option<u32>> {
        self.trace.push(json!(["resolve", g]));
        Ok(self
            .current
            .get(&(g & 0xffff))
            .filter(|(v, _)| v >> 16 == g >> 16)
            .map(|_| g))
    }
    fn pop_front(&mut self, _: MindId, _: &Arena) -> aa_mind::Result<()> {
        let g = self.list.pop_front().unwrap();
        self.trace
            .push(json!(["head", u8::from(!self.list.is_empty())]));
        if self.list.is_empty() {
            self.trace.push(json!(["tail", 0]))
        }
        self.trace.push(json!(["free", g]));
        self.freed.push(g);
        Ok(())
    }
    fn member_organization(&mut self, g: u32) -> aa_mind::Result<u32> {
        Ok(u32::from(self.current[&(g & 0xffff)].1))
    }
    fn organization(&mut self, _: MindId, _: &Arena) -> aa_mind::Result<u32> {
        Ok(1)
    }
    fn member_arrived(&mut self, _: MindId, g: u32, _: &mut Arena) -> aa_mind::Result<()> {
        self.trace.push(json!(["arrived", g]));
        if self.fail == Some(g) {
            return Err(aa_mind::Error::World("arrived".into()));
        }
        Ok(())
    }
}
#[test]
fn native_sleep_peek_resolve_then_unlink_tail_before_callback() {
    let doc = fixture();
    let rows = doc["drain_rows"].as_array().unwrap();
    assert_eq!(rows.len(), 30);
    for row in rows {
        let c = &row["input"];
        let input = words(&c["values"]);
        let mut current = std::collections::BTreeMap::new();
        for (i, &g) in input.iter().enumerate() {
            if g & 0xffff != 0 && i % 3 != 1 {
                current.insert(g & 0xffff, (g, i % 3 == 0));
            }
        }
        let mut w = Drain {
            list: List::from_saved(input),
            current,
            trace: vec![],
            freed: vec![],
            fail: c["fail_after"].as_u64().map(|v| v as u32),
        };
        let _ = aa_mind::cai_arrivals::drain(MindId(1), &mut Arena::default(), &mut w);
        assert_eq!(
            json!({"values":w.list,"trace":w.trace,"freed":w.freed}),
            row["output"],
            "{c}"
        );
    }
}
#[test]
fn actual_regiment_constructor_transfers_arrival_owner_once() {
    struct NoCalls;
    impl aa_mind::construct::World for NoCalls {
        fn random_word(&mut self) -> aa_mind::Result<u32> {
            panic!("RNG")
        }
        fn performer_ability_count(&self, _: u32) -> aa_mind::Result<usize> {
            panic!("ability")
        }
        fn set_ability_jitter(&mut self, _: u32, _: usize, _: u32) -> aa_mind::Result<()> {
            panic!("jitter")
        }
    }
    for pc in [
        aa_numeric::Precision::Single24,
        aa_numeric::Precision::Double53,
        aa_numeric::Precision::Extended64,
    ] {
        let made = aa_mind::construct::construct(
            aa_mind::construct::Kind::Regiment,
            1688,
            &mut NoCalls,
            pc,
        )
        .unwrap();
        let bindings = made
            .states
            .iter()
            .enumerate()
            .map(|(i, s)| (s.offset, aa_mind::StateId(i as u32 + 1)))
            .collect();
        let mut arena = Arena::default();
        let mut installed = made.install(MindId(1), &bindings, &mut arena).unwrap();
        let mut owned = aa_mind::regiment_load::take_constructed_owned(&mut installed).unwrap();
        assert!(owned.tracked_words.is_empty());
        assert!(!installed.words.contains_key(&0x2bc));
        assert!(!installed.words.contains_key(&0x2c0));
        assert_eq!(arena.mind(MindId(1)).unwrap().actor, 1688);
        owned.tracked_words.push_front(7);
        owned.tracked_words.push_front(0);
        owned.tracked_words.push_front(7);
        assert!(aa_mind::regiment_load::take_constructed_owned(&mut installed).is_err());
        assert_eq!(
            owned.tracked_words.iter().copied().collect::<Vec<_>>(),
            [7, 0, 7]
        );
        while owned.tracked_words.pop_front().is_some() {}
        assert!(owned.tracked_words.front().is_none() && owned.tracked_words.back().is_none());
    }
}
