use super::*;
use serde_json::{Value, json};
struct Replay<'a> {
    case: &'a Value,
    trace: Vec<Value>,
}
impl SlotFields for Replay<'_> {
    type Definition = u32;
    fn occupied(&mut self, _: i32) -> Result<bool> {
        Ok(self.case["occupied"].as_bool().unwrap())
    }
    fn definition(&mut self, slot: i32) -> Result<u32> {
        Ok(0x7000 + slot as u32)
    }
    fn allegiance(&mut self) -> Result<u32> {
        self.trace.push(json!("allegiance"));
        if self.case["fail"] == "allegiance" {
            return Err("allegiance".into());
        }
        Ok(self.case["owner"].as_u64().unwrap() as u32)
    }
    fn create(&mut self, p: &mut ActorParams<u32, String>, slot: i32) -> Result<bool> {
        assert!(p.name_utf16.is_empty());
        assert!(p.organization_layout.is_none());
        let words = [
            0,
            p.definition,
            p.position_bits[0],
            p.position_bits[1],
            p.position_bits[2],
            p.direction_bits[0],
            p.direction_bits[1],
            0,
            p.variant,
            p.requested_id,
            p.owner,
            0,
            p.word_30,
            p.word_34,
            p.mode,
            p.word_3c,
            p.word_40,
            p.word_44,
            p.word_48,
            p.word_4c,
            u32::from(p.byte_50) | (u32::from(p.byte_51) << 8) | (u32::from(p.byte_52) << 16),
        ];
        self.trace
            .push(json!({"create":{"words":words,"slot":slot,"world":true,"owner":true}}));
        if self.case["fail"] == "create" {
            return Err("create".into());
        }
        Ok(self.case["created"].as_bool().unwrap())
    }
}
#[test]
fn native_slot_spawn_raw_byte_stack_fields_and_ordered_failure_prefix() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/development10/stored-organization-spawn/native-spawn-slot.json"
    ))
    .unwrap();
    let mut checked = 0;
    for row in fixture["rows"].as_array().unwrap() {
        let c = &row["input"];
        let mut world = Replay {
            case: c,
            trace: Vec::new(),
        };
        let result = spawn_slot_fields(
            &mut world,
            c["slot"].as_i64().unwrap() as i32,
            c["flag"].as_u64().unwrap() as u8,
        );
        let (returned, error) = match result {
            Ok(v) => (Some(u32::from(v)), None),
            Err(e) => (None, Some(e)),
        };
        assert_eq!(
            json!({"returned":returned,"trace":world.trace,"error":error}),
            row["output"],
            "{c}"
        );
        checked += 1;
    }
    assert_eq!(checked, 720);
}
struct NotifyReplay<'a> {
    case: &'a Value,
    trace: Vec<Value>,
}
impl NotifyReplay<'_> {
    fn field(&mut self, name: &str) -> bool {
        self.trace.push(json!(name));
        self.case[name].as_bool().unwrap()
    }
    fn stop(&self, name: &str) -> Result<()> {
        if self.case["fail"] == name {
            Err(name.to_owned())
        } else {
            Ok(())
        }
    }
}
impl NotifyFields for NotifyReplay<'_> {
    type Owner = ();
    fn allegiance(&mut self) -> Result<Option<()>> {
        self.trace.push(json!("allegiance"));
        Ok(self.case["owner"].as_bool().unwrap().then_some(()))
    }
    fn hq_present(&mut self) -> Result<bool> {
        Ok(self.field("resource"))
    }
    fn member_is_primary(&mut self) -> Result<bool> {
        Ok(self.field("primary"))
    }
    fn refresh_kingdom(&mut self, _: ()) -> Result<()> {
        self.trace.push(json!("refresh"));
        self.stop("refresh")
    }
    fn statistics_present(&mut self, _: ()) -> Result<bool> {
        Ok(self.field("stats"))
    }
    fn state_1(&mut self) -> Result<bool> {
        self.trace.push(json!("state1"));
        Ok(self.case["state"].as_bool().unwrap())
    }
    fn add_score(&mut self, _: (), index: u32) -> Result<()> {
        self.trace.push(json!(["score", index]));
        self.stop(&format!("score{index}"))
    }
    fn classified_58edd0(&mut self) -> Result<bool> {
        self.trace.push(json!("classify"));
        self.stop("classify")?;
        Ok(self.case["classified"].as_bool().unwrap())
    }
    fn sdk_present(&mut self) -> Result<bool> {
        Ok(self.field("sdk"))
    }
    fn sai_actor_present(&mut self) -> Result<bool> {
        Ok(self.field("sai"))
    }
    fn notify_current_sai_actor(&mut self) -> Result<()> {
        assert!(self.field("sai"));
        self.trace.push(json!("notify"));
        self.stop("notify")
    }
}
#[test]
fn native_member_notification_keeps_current_reads_and_positive_child_prefixes() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/development10/stored-organization-spawn/native-notify.json"
    ))
    .unwrap();
    let mut checked = 0;
    for row in fixture["rows"].as_array().unwrap() {
        let mut replay = NotifyReplay {
            case: &row["input"],
            trace: Vec::new(),
        };
        let error = notify_fields(&mut replay).err();
        assert_eq!(
            json!({"trace":replay.trace,"error":error}),
            row["output"],
            "{}",
            row["input"]
        );
        checked += 1;
    }
    assert_eq!(checked, 168);
}
