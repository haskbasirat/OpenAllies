use super::*;
use serde_json::{Value, json};
struct Fixture<'a> {
    case: &'a Value,
    events: Vec<Value>,
    attached: Vec<u32>,
    definitions: [u32; 4],
    current_org: usize,
    layout: usize,
    acted: bool,
}
impl Fixture<'_> {
    fn event(&mut self, label: &str, args: &[u32]) -> Result<()> {
        let mut e = vec![json!(label)];
        e.extend(args.iter().map(|v| json!(v)));
        self.events.push(Value::Array(e));
        if self.case["fail"] == label {
            Err(label.into())
        } else {
            Ok(())
        }
    }
}
impl HeroWorld for Fixture<'_> {
    type Actor = u32;
    type Organization = u32;
    type Definition = u32;
    type RecruitTable = u32;
    fn capture_organization(&mut self, a: u32) -> Result<u32> {
        self.event("capture", &[a])?;
        Ok(0)
    }
    fn slot_count(&mut self, o: u32) -> Result<i32> {
        self.event("count", &[o])?;
        let n = self.case["groups"].as_array().unwrap().len();
        Ok(if self.layout == 0 { n } else { n.min(2) } as i32)
    }
    fn group_type(&mut self, o: u32, s: i32) -> Result<i32> {
        self.event("group", &[o, s as u32])?;
        Ok(self.case["groups"][s as usize].as_i64().unwrap() as i32)
    }
    fn current_member(&mut self, a: u32, s: i32) -> Result<Option<u32>> {
        self.event("member", &[a, s as u32])?;
        let rows = self.case["members"].as_array().unwrap();
        let index = if self.current_org == 0 {
            s as usize
        } else {
            rows.len() - 1 - s as usize
        };
        let v = rows[index].as_u64().unwrap() as u32;
        Ok((v != 0).then_some(v))
    }
    fn member_definition(&mut self, m: u32) -> Result<u32> {
        self.event("definition", &[m])?;
        Ok(self.definitions[m as usize - 1])
    }
    fn first_owner_present_108(&mut self, m: u32) -> Result<bool> {
        self.event("owner1", &[m])?;
        if self.case["action"] == "definition_after_owner" {
            self.definitions[m as usize - 1] = 99;
        }
        Ok(self.case["owners1"][m as usize - 1] == 1)
    }
    fn second_owner_recruit_table_108(&mut self, m: u32) -> Result<Option<u32>> {
        self.event("owner2", &[m])?;
        Ok((self.case["table2"][m as usize - 1] == 1).then_some(1))
    }
    fn recruit_contains_4e2a60(&mut self, t: u32, d: u32, f: u8) -> Result<bool> {
        self.event("recruit", &[t, d, f as u32])?;
        Ok(self.case["recruit"].as_bool().unwrap())
    }
    fn attach_hero_57b830(&mut self, a: u32, d: u32) -> Result<bool> {
        self.event("attach", &[a, d])?;
        let value = self.case["attach"].as_bool().unwrap();
        if value {
            self.attached.push(d);
        }
        if !self.acted {
            if self.case["action"] == "switch_org" {
                self.current_org = 1;
            }
            if self.case["action"] == "shorten_layout" {
                self.layout = 1;
            }
            self.acted = true;
        }
        Ok(value)
    }
    fn notify_hero_attached(&mut self, a: u32, d: u32) -> Result<()> {
        self.event("notify", &[a, d])
    }
}
#[test]
fn original_v14_hero_live_reads_lazy_children_and_error_prefixes_match() {
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-fresh-organization-completion/native-hero-postpass.json")).unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 411);
    for row in fixture["rows"].as_array().unwrap() {
        let c = &row["input"];
        let mut w = Fixture {
            case: c,
            events: vec![],
            attached: vec![],
            definitions: [11, 12, 13, 14],
            current_org: 0,
            layout: 0,
            acted: false,
        };
        let result = hero_postpass(&mut w, c["created"].as_bool().unwrap().then_some(1));
        let (result, error) = match result {
            Ok(r) => (r, None),
            Err(e) => (None, Some(e)),
        };
        assert_eq!(
            json!({"result":result,"error":error,"events":w.events,"attached":w.attached,"member_definitions":w.definitions}),
            row["output"],
            "{}",
            row["precision"]
        );
    }
}

#[test]
fn ordinary_extracted_tail_reads_same_parameter_field_after_each_spawn() {
    use aa_mapworld::fresh_creation::EntryWorld;
    struct Members {
        field: u8,
        spawned: Vec<(i32, u8)>,
    }
    impl EntryWorld for Members {
        type Object = u32;
        type Layout = u32;
        type Error = String;
        fn create(&mut self, _: [u32; 3]) -> Result<Option<u32>> {
            Err("unreached create".into())
        }
        fn is_gactor(&mut self, _: &u32) -> Result<bool> {
            Err("unreached assertion".into())
        }
        fn assert_latch(&self, _: u32) -> bool {
            panic!("unreached assertion")
        }
        fn set_assert_latch(&mut self, _: u32, _: bool) {
            panic!("unreached assertion")
        }
        fn assertion_dialog(&mut self, _: u32) -> Result<bool> {
            Err("unreached assertion".into())
        }
        fn definition_is_actor_data(&mut self) -> Result<bool> {
            Err("unreached definition".into())
        }
        fn definition_flags_174(&mut self) -> Result<u32> {
            Err("unreached definition".into())
        }
        fn layout(&mut self) -> Result<u32> {
            Err("unreached preplacement".into())
        }
        fn layout_slot_count(&mut self, _: &u32) -> Result<i32> {
            Err("unreached preplacement".into())
        }
        fn first_slot(&mut self, _: &u32) -> Result<u32> {
            Err("unreached preplacement".into())
        }
        fn slot_placement(
            &mut self,
            _: &u32,
            _: u32,
            _: &mut [u32; 2],
            _: [u32; 2],
        ) -> Result<bool> {
            Err("unreached preplacement".into())
        }
        fn organization(&mut self, _: &u32) -> Result<bool> {
            Ok(true)
        }
        fn leader_slot(&mut self, _: &u32) -> Result<i32> {
            Ok(0)
        }
        fn organization_slot_count(&mut self, _: &u32) -> Result<i32> {
            Ok(3)
        }
        fn slot_defined(&mut self, _: &u32, _: i32) -> Result<bool> {
            Ok(true)
        }
        fn spawn_member(&mut self, _: &u32, slot: i32, flag: u8) -> Result<()> {
            self.spawned.push((slot, flag));
            self.field = if slot == 0 { 7 } else { 9 };
            Ok(())
        }
        fn member_count(&mut self, _: &u32) -> Result<u32> {
            Ok(self.spawned.len() as u32)
        }
        fn log_no_members(&mut self, _: &u32) -> Result<()> {
            Err("no empty completion expected".into())
        }
        fn destroy(&mut self, _: u32) -> Result<()> {
            Err("no destruction expected".into())
        }
    }
    let mut w = Members {
        field: 3,
        spawned: vec![],
    };
    let mut reads = 0;
    assert_eq!(
        finish_organization_members(&mut w, 1, |w| {
            reads += 1;
            Ok(w.field)
        })
        .unwrap(),
        Some(1)
    );
    assert_eq!(reads, 2);
    assert_eq!(w.spawned, vec![(0, 0), (1, 7), (2, 9)]);
    let mut w = Members {
        field: 3,
        spawned: vec![],
    };
    let mut reads = 0;
    let error = finish_organization_members(&mut w, 1, |w| {
        reads += 1;
        if reads == 2 {
            Err("required current Params+51".into())
        } else {
            Ok(w.field)
        }
    })
    .unwrap_err();
    assert!(error.to_string().contains("Params+51"));
    assert_eq!(w.spawned, vec![(0, 0), (1, 7)]);
}
