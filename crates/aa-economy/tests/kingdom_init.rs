use aa_economy::{
    EconomyError, X87Precision,
    kingdom_init::{self, Flow, PlainRecruit, RecruitLink, Result, World},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Deserialize)]
struct Definition {
    id: u32,
    mask: u32,
    links: [bool; 3],
    allowed: bool,
    #[serde(default)]
    null_links: bool,
}
#[derive(Clone, Deserialize, serde::Serialize)]
struct Team {
    members: Vec<u32>,
    count: u32,
}
#[derive(Clone, Deserialize)]
struct Mutation {
    event: String,
    resource_count: Option<i32>,
    world_version: Option<u32>,
    word184: Option<u32>,
    word190: Option<u32>,
    sai_active: Option<bool>,
    majors: Option<Vec<Option<u32>>>,
    next_node: Option<(u32, Option<u32>)>,
}
#[derive(Clone, Deserialize)]
struct Case {
    name: String,
    actor_success: bool,
    argument: u32,
    world_version: u32,
    word184: u32,
    resource_count: i32,
    flows: [Vec<u32>; 3],
    stock: Vec<u32>,
    team: Option<Team>,
    word190: u32,
    sai_active: bool,
    categories: [Vec<u32>; 2],
    definitions: Vec<Definition>,
    majors: Vec<Option<u32>>,
    register_majors: Vec<Option<u32>>,
    related: BTreeMap<String, bool>,
    mutations: Vec<Mutation>,
    fail_at: Option<String>,
}
struct Host {
    c: Case,
    next: BTreeMap<u32, Option<u32>>,
    events: Vec<Value>,
    recruits: Vec<PlainRecruit>,
}
impl Host {
    fn new(c: Case) -> Self {
        let mut next = c
            .definitions
            .iter()
            .map(|d| (d.id, None))
            .collect::<BTreeMap<_, _>>();
        for category in &c.categories {
            for pair in category.windows(2) {
                next.insert(pair[0], Some(pair[1]));
            }
        }
        Self {
            c,
            next,
            events: vec![],
            recruits: vec![],
        }
    }
    fn event(&mut self, key: String, args: Value) -> Result<()> {
        let mut row = args.as_object().unwrap().clone();
        row.insert("event".into(), json!(key));
        self.events.push(Value::Object(row));
        if self.c.fail_at.as_ref() == Some(&key) {
            return Err(EconomyError::World(format!("callback:{key}")));
        }
        for m in self.c.mutations.iter().filter(|m| m.event == key) {
            if let Some(v) = m.resource_count {
                self.c.resource_count = v;
            }
            if let Some(v) = m.world_version {
                self.c.world_version = v;
            }
            if let Some(v) = m.word184 {
                self.c.word184 = v;
            }
            if let Some(v) = m.word190 {
                self.c.word190 = v;
            }
            if let Some(v) = m.sai_active {
                self.c.sai_active = v;
            }
            if let Some(v) = &m.majors {
                self.c.majors = v.clone();
            }
            if let Some((a, b)) = m.next_node {
                self.next.insert(a, b);
            }
        }
        Ok(())
    }
    fn definition(&self, id: u32) -> &Definition {
        self.c.definitions.iter().find(|d| d.id == id).unwrap()
    }
    fn snapshot(&self, result: Result<bool>) -> Value {
        let status = match result {
            Ok(v) => json!({"returned":v,"error":null}),
            Err(EconomyError::World(message)) => {
                json!({"returned":null,"error":if message.contains("null current node"){"null_definition_cursor"}else{&message}})
            }
            Err(error) => panic!("unexpected error {error}"),
        };
        json!({"events":self.events,"status":status,"flows":self.c.flows,"stock":self.c.stock,"team":self.c.team,"recruits":self.recruits,"word184":self.c.word184,"resource_count":self.c.resource_count,"majors":self.c.majors})
    }
}
impl World for Host {
    fn actor_init(&mut self, k: u32, argument: u32, _pc: X87Precision) -> Result<bool> {
        assert_eq!(k, 1000);
        self.event("actor_init".into(), json!({"argument":argument}))?;
        Ok(self.c.actor_success)
    }
    fn initialized_diagnostic(&mut self, _k: u32) -> Result<()> {
        self.event("diagnostic".into(), json!({}))
    }
    fn world_version(&mut self) -> Result<u32> {
        Ok(self.c.world_version)
    }
    fn schedule_word_184(&mut self, _k: u32) -> Result<u32> {
        Ok(self.c.word184)
    }
    fn legacy_schedule(&mut self, _k: u32) -> Result<()> {
        self.event("legacy".into(), json!({}))?;
        self.c.word184 = 0x777;
        Ok(())
    }
    fn resource_count(&mut self) -> Result<i32> {
        Ok(self.c.resource_count)
    }
    fn clear_flow_cell(&mut self, _k: u32, flow: Flow, index: usize) -> Result<()> {
        self.event(format!("flow:{}:{index}", flow.offset()), json!({}))?;
        let slot = match flow {
            Flow::Provided1a0 => 0,
            Flow::Gross1ac => 1,
            Flow::Consumed1b8 => 2,
        };
        let row = &mut self.c.flows[slot];
        if row.len() <= index {
            row.resize(index + 1, 0xa5a5_a5a5);
        }
        row[index] = 0;
        Ok(())
    }
    fn team(&mut self, _k: u32) -> Result<Option<u32>> {
        Ok(self.c.team.as_ref().map(|_| 1))
    }
    fn prepend_team_member(&mut self, _t: u32, k: u32) -> Result<()> {
        self.event("team_prepend".into(), json!({}))?;
        self.c.team.as_mut().unwrap().members.insert(0, k);
        Ok(())
    }
    fn team_member_count(&mut self, _t: u32) -> Result<u32> {
        Ok(self.c.team.as_ref().unwrap().count)
    }
    fn set_team_member_count(&mut self, _t: u32, value: u32) -> Result<()> {
        self.event("team_count".into(), json!({"value":value}))?;
        self.c.team.as_mut().unwrap().count = value;
        Ok(())
    }
    fn definition_category_head(&mut self, category: u32) -> Result<Option<u32>> {
        Ok(self.c.categories[match category {
            19 => 0,
            42 => 1,
            _ => panic!("category"),
        }]
        .first()
        .copied())
    }
    fn definition_at_node(&mut self, node: u32) -> Result<u32> {
        Ok(node)
    }
    fn next_definition_node(&mut self, node: u32) -> Result<Option<u32>> {
        Ok(self.next[&node])
    }
    fn definition_component_mask(&mut self, d: u32) -> Result<u32> {
        Ok(self.definition(d).mask)
    }
    fn recruit_link_nonempty(&mut self, d: u32, link: RecruitLink) -> Result<bool> {
        let row = self.definition(d);
        if row.null_links && link != RecruitLink::Property17c {
            return Err(EconomyError::World("null_recruit_backreference".into()));
        }
        Ok(row.links[match link {
            RecruitLink::Primary => 0,
            RecruitLink::Secondary => 1,
            RecruitLink::Property17c => 2,
        }])
    }
    fn recruit_allowed(&mut self, _k: u32, d: u32) -> Result<bool> {
        self.event(format!("allowed:{d}"), json!({}))?;
        Ok(self.definition(d).allowed)
    }
    fn append_plain_recruit(&mut self, _k: u32, item: PlainRecruit) -> Result<()> {
        self.event(
            format!("recruit:{}", item.definition),
            json!({"available":item.available}),
        )?;
        self.recruits.push(item);
        Ok(())
    }
    fn kingdom_word_190(&mut self, _k: u32) -> Result<u32> {
        Ok(self.c.word190)
    }
    fn sai_active(&mut self) -> Result<bool> {
        Ok(self.c.sai_active)
    }
    fn register_sai(&mut self, _k: u32) -> Result<()> {
        self.event("sai".into(), json!({}))
    }
    fn register_world(&mut self, _k: u32) -> Result<()> {
        self.event("register".into(), json!({}))?;
        self.c.majors = self.c.register_majors.clone();
        Ok(())
    }
    fn major_count(&mut self) -> Result<i32> {
        Ok(self.c.majors.len() as i32)
    }
    fn major_at(&mut self, index: usize) -> Result<Option<u32>> {
        Ok(self.c.majors[index])
    }
    fn related(&mut self, other: u32, _k: u32) -> Result<bool> {
        self.event(format!("related:{other}"), json!({}))?;
        Ok(*self.c.related.get(&other.to_string()).unwrap_or(&false))
    }
    fn refresh_relation_228(&mut self, other: u32) -> Result<()> {
        self.event(format!("refresh228:{other}"), json!({}))
    }
    fn refresh_relation_22c(&mut self, other: u32) -> Result<()> {
        self.event(format!("refresh22c:{other}"), json!({}))
    }
}

#[test]
fn full_init_coordinator_matches_original_v14_mutations_and_failure_prefixes() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-kingdom-init/native-kingdom-init.json"
    ))
    .unwrap();
    let rows = doc["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 99);
    for row in rows {
        let c: Case = serde_json::from_value(row["case"].clone()).unwrap();
        let label = c.name.clone();
        let arg = c.argument;
        let pc: X87Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let mut host = Host::new(c);
        let result = kingdom_init::initialize(1000, arg, pc, &mut host);
        assert_eq!(host.snapshot(result), row["expected"], "{label} {pc:?}");
    }
}
