//! Replays research/phase8-combat/native-capture-attempt-fixtures.json:
//! original 4D5230 executed under Unicorn with recorded world seams.
use aa_body::{F32, damage::DamageRecord};
use aa_combat::{
    Result,
    capture::{self, AttemptWorld},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

struct Host {
    actors: Value,
    sequences: BTreeMap<u32, Vec<u32>>,
    sai: bool,
    classification: i32,
    events: Vec<Value>,
}
impl Host {
    fn view(&self, actor: u32) -> &Value {
        &self.actors[actor.to_string()]
    }
    fn flag(&self, actor: u32, key: &str) -> bool {
        self.view(actor)[key] == true
    }
    fn tag(owner: Option<u32>) -> u32 {
        owner.unwrap_or(0)
    }
}
impl AttemptWorld for Host {
    fn delegates_capture(&mut self, actor: u32) -> Result<bool> {
        self.events.push(json!({"Delegates": actor}));
        Ok(self.flag(actor, "delegates"))
    }
    fn outer(&mut self, actor: u32) -> Result<Option<u32>> {
        self.events.push(json!({"Outer": actor}));
        let o = self.view(actor)["outer"].as_u64().unwrap_or(0) as u32;
        Ok((o != 0).then_some(o))
    }
    fn allegiance(&mut self, actor: u32) -> Result<Option<u32>> {
        let owner = match self.sequences.get_mut(&actor) {
            Some(seq) if !seq.is_empty() => seq.remove(0),
            _ => self.view(actor)["allegiance"].as_u64().unwrap_or(0) as u32,
        };
        self.events.push(json!({"Allegiance": [actor, owner]}));
        Ok((owner != 0).then_some(owner))
    }
    fn is_allegiance(&mut self, actor: u32) -> Result<bool> {
        self.events.push(json!({"IsAllegiance": actor}));
        Ok(self.flag(actor, "is_allegiance"))
    }
    fn capture_notification(&mut self, captured: u32, source: u32) -> Result<()> {
        self.events
            .push(json!({"CaptureNotification": [captured, source]}));
        Ok(())
    }
    fn announce_capture(
        &mut self,
        captured: u32,
        source: u32,
        old: Option<u32>,
        kingdom: Option<u32>,
    ) -> Result<()> {
        let label = match kingdom {
            Some(k) => {
                self.events.push(json!({"KingdomName": k}));
                format!("name:kingdom:{k}")
            }
            None => {
                self.events.push(json!({"Name": source}));
                format!("name:{source}")
            }
        };
        self.events.push(json!({"Name": captured}));
        self.events.push(
            json!({"Message": [16, Host::tag(old), captured, format!("name:{captured}"), label]}),
        );
        Ok(())
    }
    fn has_wall(&self, actor: u32) -> Result<bool> {
        Ok(self.flag(actor, "wall"))
    }
    fn has_component88(&self, actor: u32) -> Result<bool> {
        Ok(self.flag(actor, "component88"))
    }
    fn has_component_d4(&self, actor: u32) -> Result<bool> {
        Ok(self.flag(actor, "d4"))
    }
    fn transfer_wall(&mut self, _: u32, new: Option<u32>) -> Result<()> {
        self.events.push(json!({"TransferWall": [Host::tag(new)]}));
        Ok(())
    }
    fn transfer_component88(&mut self, _: u32, new: Option<u32>) -> Result<()> {
        self.events
            .push(json!({"TransferComponent88": [Host::tag(new)]}));
        Ok(())
    }
    fn set_allegiance(&mut self, actor: u32, new: Option<u32>) -> Result<()> {
        self.events
            .push(json!({"SetAllegiance": [actor, Host::tag(new)]}));
        Ok(())
    }
    fn announce_owner_change(&mut self, captured: u32, new: Option<u32>) -> Result<()> {
        self.events.push(json!({"Name": captured}));
        self.events
            .push(json!({"Message": [15, Host::tag(new), captured, format!("name:{captured}")]}));
        Ok(())
    }
    fn strategic_ai_present(&self) -> Result<bool> {
        Ok(self.sai)
    }
    fn strategic_ai(&mut self, owner: Option<u32>) -> Result<Option<u32>> {
        self.events.push(json!({"StrategicAi": Host::tag(owner)}));
        Ok(owner.filter(|o| self.flag(*o, "sai")))
    }
    fn strategic_ai_lost(&mut self, ai: u32, captured: u32) -> Result<()> {
        self.events
            .push(json!({"SaiLost": [format!("sai:{ai}"), captured]}));
        Ok(())
    }
    fn strategic_ai_gained(&mut self, ai: u32, captured: u32) -> Result<()> {
        self.events
            .push(json!({"SaiGained": [format!("sai:{ai}"), captured]}));
        Ok(())
    }
    fn statistics(&self, owner: u32) -> Result<Option<u32>> {
        Ok(self.flag(owner, "statistics").then_some(owner))
    }
    fn add_statistic(&mut self, stats: u32, index: u32, amount: F32) -> Result<()> {
        self.events
            .push(json!({"AddStatistic": [format!("stats:{stats}"), index, amount.0]}));
        Ok(())
    }
    fn capture_statistics(&mut self, stats: u32, captured: u32) -> Result<()> {
        self.events
            .push(json!({"CaptureStatistics": [format!("stats:{stats}"), captured]}));
        Ok(())
    }
    fn classification(&mut self, captured: u32) -> Result<i32> {
        self.events.push(json!({"Classification": captured}));
        Ok(self.classification)
    }
    fn capture_reward(&mut self, source: u32, captured: u32, record: &DamageRecord) -> Result<()> {
        self.events
            .push(json!({"CaptureReward": [source, captured, record.reason]}));
        Ok(())
    }
}

#[test]
fn original_capture_attempt_matches_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-capture-attempt-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 17);
    for row in cases {
        let i = &row["input"];
        let mut sequences = BTreeMap::new();
        for (tag, view) in i["actors"].as_object().unwrap() {
            if let Some(seq) = view["allegiance_sequence"].as_array() {
                sequences.insert(
                    tag.parse().unwrap(),
                    seq.iter().map(|v| v.as_u64().unwrap() as u32).collect(),
                );
            }
        }
        let mut host = Host {
            actors: i["actors"].clone(),
            sequences,
            sai: i["sai"] == true,
            classification: i["classification"].as_i64().unwrap() as i32,
            events: vec![],
        };
        // The native record's +18 word (reason slot) carries the fixture
        // record kind tested by 4CEBE0; +14 is the event kind.
        let record = DamageRecord {
            source_id: 0,
            allegiance_id: 0,
            damage_type: 0,
            amount: F32::ZERO,
            time: F32::ZERO,
            event_kind: i["event_kind"].as_u64().unwrap() as u32,
            reason: i["record_kind"].as_u64().unwrap() as u32,
        };
        let source = i["source"].as_u64().unwrap() as u32;
        let r = capture::attempt(1, (source != 0).then_some(source), &record, &mut host).unwrap();
        assert_eq!(json!(r as u8), row["output"]["result"], "{}", i["name"]);
        assert_eq!(json!(host.events), row["output"]["events"], "{}", i["name"]);
    }
}
