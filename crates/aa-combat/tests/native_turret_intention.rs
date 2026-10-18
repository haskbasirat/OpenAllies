//! 0x57DD90 / 0x57DEA0 against
//! research/phase8-combat/native-turret-intention-fixtures.json.
use aa_body::F32;
use aa_combat::{
    Result,
    constructors::{OwnedPerformer, Turret},
    performer::{Context, Performer, TrackWorld, clear_turret_intentions, track_target},
};
use serde_json::{Value, json};

const OWNER: u32 = 0x5000;
const TARGET: u32 = 0x6000;

struct Adapter {
    case: Value,
    events: Vec<Value>,
}
fn context_words(c: &Context) -> Value {
    let mut words = vec![
        c.event_kind,
        c.phase,
        c.source.map_or(0, |_| 1),
        c.allegiance.unwrap_or(0),
    ];
    words.extend(c.source_position.iter().map(|f| f.0));
    words.push(c.target.map_or(0, |_| 1));
    words.extend(c.target_position.iter().map(|f| f.0));
    words.push(c.word2c);
    json!(words)
}
impl TrackWorld for Adapter {
    fn ability_turret(&self, ability: u32) -> Result<i32> {
        Ok(self.case["turret_slots"][ability as usize]
            .as_i64()
            .unwrap() as i32)
    }
    fn turret_tracks_target(&self, owner: u32, index: i32) -> Result<bool> {
        assert_eq!(owner, OWNER);
        Ok(self.case["tracks"][index as usize].as_bool().unwrap())
    }
    fn actor_position(&self, actor: u32) -> Result<[F32; 3]> {
        assert_eq!(actor, TARGET);
        let p = &self.case["target_position"];
        Ok([0, 1, 2].map(|k| F32(p[k].as_u64().unwrap() as u32)))
    }
    fn can_use(&mut self, ability: u32, context: &mut Context, check_range: bool) -> Result<u32> {
        self.events.push(
            json!({"can_use": ability, "check_range": u32::from(check_range),
            "context": context_words(context)}),
        );
        // The native hook scribbles the context; the port must reinitialise.
        context.phase = 0xa5a5_a5a5;
        context.target_position = [F32(0xa5a5_a5a5); 3];
        Ok(self.case["results"][ability as usize].as_u64().unwrap() as u32)
    }
    fn activate_updates(&mut self, performer: &mut OwnedPerformer) -> Result<()> {
        assert!(performer.turret_update_pending);
        self.events.push(json!({"activate": 8, "arg": 1}));
        Ok(())
    }
}

#[test]
fn turret_intention_matches_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-turret-intention-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 60);
    for case in cases {
        let count = case["turret_slots"].as_array().unwrap().len() as u32;
        let turrets = case["initial"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| Turret {
                current_angle: F32::ZERO,
                target_angle: F32::ZERO,
                time: F32::ZERO,
                pending: t[3].as_u64().unwrap() != 0,
                target: [0, 1, 2].map(|k| F32(t[k].as_u64().unwrap() as u32)),
            })
            .collect();
        let mut performer = OwnedPerformer {
            state: Performer {
                owner: OWNER,
                stage: 4,
                abilities: (0..count).collect(),
                mana: F32::ZERO,
                target_id: 0,
                target_position: [F32::ZERO; 2],
                active_ability: case["active"].as_u64().map(|a| a as u32),
                active_index: -1,
                started_at: F32::ZERO,
                animation_length: F32::ZERO,
                event_kind: 0,
                animation: None,
                turret_index: -1,
            },
            maximum_mana: F32::ZERO,
            turrets,
            turret_update_pending: case["latch"].as_bool().unwrap(),
            word60: 0,
        };
        let mut world = Adapter {
            case: case.clone(),
            events: vec![],
        };
        let target = case["target"].as_bool().unwrap().then_some(TARGET);
        let point = [0, 1].map(|k| F32(case["point"][k].as_u64().unwrap() as u32));
        let kind = case["event_kind"].as_u64().unwrap() as u32;
        track_target(&mut performer, kind, target, point, &mut world).unwrap();
        // Handles are compared by presence (the oracle's pointers are arbitrary).
        let mut native = case["events"].clone();
        for event in native.as_array_mut().unwrap() {
            if let Some(words) = event.get_mut("context") {
                words[2] = json!(u32::from(words[2] != json!(0)));
                words[7] = json!(u32::from(words[7] != json!(0)));
            }
        }
        assert_eq!(Value::Array(world.events), native, "{case}");
        let after: Vec<Value> = performer
            .turrets
            .iter()
            .map(|t| {
                json!([
                    t.target[0].0,
                    t.target[1].0,
                    t.target[2].0,
                    u32::from(t.pending)
                ])
            })
            .collect();
        assert_eq!(Value::Array(after), case["after"], "{case}");
        assert_eq!(
            u64::from(performer.turret_update_pending),
            case["latch_after"].as_u64().unwrap()
        );
        clear_turret_intentions(&mut performer, 4).unwrap();
        let cleared: Vec<Value> = performer
            .turrets
            .iter()
            .map(|t| json!(u32::from(t.pending)))
            .collect();
        assert_eq!(Value::Array(cleared), case["cleared"], "{case}");
    }
}
