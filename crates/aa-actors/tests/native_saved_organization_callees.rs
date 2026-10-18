//! Replays research/phase8-saved-components/native-organization-callees.json:
//! original 4CED10, 4CEBB0 (57A330, 4364C0, 57E010) and 561760 under Unicorn.
use aa_actors::{
    ActorId,
    organization_callees::{self as oc, ActorFacts, BodyUpdatable, RankWorld, Result},
};
use serde_json::{Value, json};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../research/phase8-saved-components/native-organization-callees.json"
);

struct World<'a> {
    actors: &'a serde_json::Map<String, Value>,
    names: Vec<String>,
    calls: Vec<Value>,
}
impl World<'_> {
    fn id(&self, name: &str) -> ActorId {
        ActorId::from_raw(0x10001 + self.names.iter().position(|n| n == name).unwrap() as u32)
    }
    fn name(&self, id: ActorId) -> &str {
        &self.names[(id.raw() - 0x10001) as usize]
    }
    fn actor(&self, id: ActorId) -> &Value {
        &self.actors[self.name(id)]
    }
}
impl ActorFacts for World<'_> {
    fn experience_rank(&self, actor: ActorId) -> Result<Option<u32>> {
        Ok(self.actor(actor).get("rank").and_then(Value::as_u64).map(|v| v as u32))
    }
    fn element_organization(&self, actor: ActorId) -> Result<Option<ActorId>> {
        Ok(self.actor(actor).get("element_org").and_then(Value::as_str).map(|n| self.id(n)))
    }
    fn has_performer(&self, actor: ActorId) -> Result<bool> {
        Ok(self.actor(actor).get("performer").and_then(Value::as_bool).unwrap_or(false))
    }
    fn organization_members(&self, actor: ActorId) -> Result<Option<Vec<ActorId>>> {
        Ok(self.actor(actor).get("members").map(|m| {
            m.as_array().unwrap().iter().filter_map(Value::as_str).map(|n| self.id(n)).collect()
        }))
    }
}
impl RankWorld for World<'_> {
    fn performer_rank_changed(&mut self, actor: ActorId, rank: u32) -> Result<()> {
        let this = format!("{}.Performer", self.name(actor));
        self.calls.push(json!({"seam": "57DCF0", "this": this, "rank": rank}));
        Ok(())
    }
}
impl BodyUpdatable for World<'_> {
    fn body_has_scheduled(&mut self, member: ActorId, event: u8, data: u32) -> Result<bool> {
        let r = self.actor(member)["body"].as_bool().unwrap();
        let this = format!("{}.Body", self.name(member));
        self.calls.push(json!({"seam": "50B220", "this": this, "event": event, "data": data, "result": r}));
        Ok(r)
    }
    fn body_unschedule(&mut self, member: ActorId, event: u8, data: u32) -> Result<()> {
        let this = format!("{}.Body", self.name(member));
        self.calls.push(json!({"seam": "50B1C0", "this": this, "event": event, "data": data}));
        Ok(())
    }
}

#[test]
fn organization_callees_match_original() {
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    for (i, case) in cases.iter().enumerate() {
        let input = &case["input"];
        let actors = input["actors"].as_object().unwrap();
        let mut w = World { actors, names: actors.keys().cloned().collect(), calls: Vec::new() };
        let actor = w.id(input["actor"].as_str().unwrap());
        match input["function"].as_str().unwrap() {
            "4CED10" => {
                let r = oc::experience_source(actor, &w).unwrap();
                assert_eq!(u64::from(r), case["native"]["result"].as_u64().unwrap(), "case {i}");
            }
            "4CEBB0" => {
                let old = input["old"].as_u64().unwrap() as u32;
                let new = input["new"].as_u64().unwrap() as u32;
                oc::rank_changed(actor, old, new, &mut w).unwrap();
            }
            "561760" => {
                let owner = (input["owner"].as_u64().unwrap() != 0).then(|| w.id("own"));
                oc::body_set_organization(actor, owner, &mut w).unwrap();
            }
            f => panic!("{f}"),
        }
        assert_eq!(w.calls, case["native"]["calls"].as_array().unwrap().clone(), "case {i}");
    }
    assert_eq!(cases.len(), 25);
}
