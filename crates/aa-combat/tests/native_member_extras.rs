//! 5705A0 / 5784C0 / 57DCF0 against
//! research/phase8-combat/native-member-extras-fixtures.json.
use aa_body::F32;
use aa_combat::{
    Result,
    organization::{
        AttachWorld, OwnerProperty, PropagationWorld, attach_membership, propagate_properties,
    },
    performer::{Performer, RankWorld, gate_by_rank},
};
use serde_json::{Value, json};

fn doc() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-member-extras-fixtures.json"
    ))
    .unwrap()
}

struct Attach {
    observer: bool,
    events: Vec<Value>,
    fields: Option<(u32, i32)>,
}
impl AttachWorld for Attach {
    fn set_membership(&mut self, _: u32, owner: u32, slot: i32) -> Result<()> {
        self.fields = Some((owner, slot));
        Ok(())
    }
    fn notify_member(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"notify": "member"}));
        Ok(())
    }
    fn refresh_member(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"refresh": "member"}));
        Ok(())
    }
    fn observer_present(&self) -> Result<bool> {
        Ok(self.observer)
    }
    fn notify_observer(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"observer": "member"}));
        Ok(())
    }
}

#[test]
fn membership_attach_matches_native() {
    for case in doc()["cases"].as_array().unwrap() {
        let mut world = Attach {
            observer: case["observer"].as_bool().unwrap(),
            events: vec![],
            fields: None,
        };
        let owner = case["owner"].as_u64().unwrap() as u32;
        let slot = case["slot"].as_i64().unwrap() as i32;
        attach_membership(1, owner, slot, &mut world).unwrap();
        assert_eq!(Value::Array(world.events), case["events"], "{case}");
        assert_eq!(case["fields"], json!([owner, slot as u32, 1]), "{case}");
        assert_eq!(world.fields, Some((owner, slot)));
    }
}

struct Propagate<'a> {
    case: &'a Value,
    events: Vec<Value>,
}
impl PropagationWorld for Propagate<'_> {
    fn owner_properties(&self, _: u32) -> Result<Option<Vec<OwnerProperty>>> {
        if !self.case["properties"].as_bool().unwrap() {
            return Ok(None);
        }
        Ok(Some(
            self.case["specs"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(i, s)| OwnerProperty {
                    property: 100 + i as u32,
                    kind: s["kind"].as_u64().unwrap() as u32,
                    disabled: s["disabled"].as_bool().unwrap(),
                    pass_down_members: s["pass_down"].as_bool().unwrap(),
                })
                .collect(),
        ))
    }
    fn has_properties(&self, _: u32) -> Result<bool> {
        Ok(self.case["member"] != "no_props")
    }
    fn has_component_a4(&self, _: u32) -> Result<bool> {
        Ok(self.case["member"] == "a4")
    }
    fn apply_property(&mut self, member: u32, property: u32, kind: u32, source: u32) -> Result<()> {
        assert_eq!((member, source), (2, 1));
        self.events.push(
            json!({"apply": "member", "property": format!("p{}", property - 100),
            "kind": kind, "source": "owner"}),
        );
        Ok(())
    }
}

#[test]
fn property_propagation_matches_native() {
    for case in doc()["propagation"].as_array().unwrap() {
        let mut world = Propagate {
            case,
            events: vec![],
        };
        let member = (case["member"] != "none").then_some(2);
        propagate_properties(1, member, &mut world).unwrap();
        assert_eq!(Value::Array(world.events), case["events"], "{case}");
    }
}

struct Rank {
    specs: Vec<Value>,
    enabled: Vec<bool>,
    events: Vec<Value>,
}
impl RankWorld for Rank {
    fn ability_local_definition(&self, ability: u32) -> Result<Option<u32>> {
        Ok(self.specs[ability as usize]["local"]
            .as_bool()
            .unwrap()
            .then_some(ability))
    }
    fn rank_limits(&self, local: u32) -> Result<[i32; 2]> {
        let s = &self.specs[local as usize];
        Ok([
            s["min"].as_i64().unwrap() as i32,
            s["max"].as_i64().unwrap() as i32,
        ])
    }
    fn ability_enabled(&self, ability: u32) -> Result<bool> {
        Ok(self.enabled[ability as usize])
    }
    fn set_ability_enabled(&mut self, ability: u32, enabled: bool) -> Result<()> {
        self.enabled[ability as usize] = enabled;
        Ok(())
    }
    fn notify(&mut self, owner: u32) -> Result<()> {
        assert_eq!(owner, 9);
        self.events.push(json!({"notify": "owner"}));
        Ok(())
    }
}

#[test]
fn rank_gating_matches_native() {
    for case in doc()["rank"].as_array().unwrap() {
        let specs = case["specs"].as_array().unwrap().clone();
        let state = Performer {
            owner: 9,
            stage: 4,
            abilities: (0..specs.len() as u32).collect(),
            mana: F32::ZERO,
            target_id: 0,
            target_position: [F32::ZERO; 2],
            active_ability: None,
            active_index: -1,
            started_at: F32::ZERO,
            animation_length: F32::ZERO,
            event_kind: 0,
            animation: None,
            turret_index: -1,
        };
        let mut world = Rank {
            enabled: specs
                .iter()
                .map(|s| s["enabled"].as_bool().unwrap())
                .collect(),
            specs,
            events: vec![],
        };
        let rank = case["rank_object"]
            .as_bool()
            .unwrap()
            .then(|| case["rank"].as_i64().unwrap() as i32);
        gate_by_rank(&state, rank, &mut world).unwrap();
        assert_eq!(Value::Array(world.events), case["events"], "{case}");
        let after: Vec<Value> = world.enabled.iter().map(|&e| json!(u32::from(e))).collect();
        assert_eq!(Value::Array(after), case["after"], "{case}");
    }
}
