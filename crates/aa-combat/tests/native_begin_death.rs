//! 0x4D7960 against research/phase8-combat/native-begin-death-fixtures.json.
use aa_combat::{
    Result,
    begin_death::{World, begin_death},
};
use serde_json::{Value, json};

struct Adapter {
    input: Value,
    flags8: u32,
    events: Vec<Value>,
}
impl Adapter {
    fn has(&self, offset: u32) -> bool {
        self.input["components"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c.as_u64() == Some(u64::from(offset)))
    }
    fn b(&self, name: &str) -> bool {
        self.input[name].as_bool().unwrap()
    }
}
impl World for Adapter {
    fn flags8(&self, _: u32) -> Result<u32> {
        Ok(self.flags8)
    }
    fn set_flags8(&mut self, _: u32, flags: u32) -> Result<()> {
        self.flags8 = flags;
        Ok(())
    }
    fn flags100(&self, _: u32) -> Result<u32> {
        Ok(self.input["flags100"].as_u64().unwrap() as u32)
    }
    fn credit_kill(&mut self, _: u32) -> Result<Option<u32>> {
        self.events.push(json!({"credit": true}));
        let credited = self.input["credited"].as_u64().unwrap() as u32;
        Ok((credited != 0).then_some(credited))
    }
    fn zone_entries(&self, _: u32) -> Result<Vec<Option<u32>>> {
        let mut index = 0;
        Ok(self.input["zones"]
            .as_array()
            .unwrap()
            .iter()
            .map(|z| {
                z.as_u64().map(|word| {
                    let entry = (index << 16) | word as u32;
                    index += 1;
                    entry
                })
            })
            .collect())
    }
    fn zone_word(&self, entry: u32) -> Result<u16> {
        Ok(entry as u16)
    }
    fn zone_release(&mut self, entry: u32) -> Result<()> {
        self.events.push(json!({"zone_release": entry >> 16}));
        Ok(())
    }
    fn component(&self, _: u32, offset: u32) -> Result<Option<u32>> {
        Ok(self.has(offset).then_some(offset))
    }
    fn component_80_word10(&self, _: u32) -> Result<bool> {
        Ok(self.b("c80_word10"))
    }
    fn component_c0_byte38(&self, _: u32) -> Result<bool> {
        Ok(self.b("c0_byte38"))
    }
    fn wall_owner(&self, _: u32) -> Result<Option<u32>> {
        Ok(self.b("wall_owner").then_some(1))
    }
    fn wall_owner_check(&mut self, _: u32) -> Result<bool> {
        self.events.push(json!({"wall_check": true}));
        Ok(self.b("wall_check"))
    }
    fn virtual_100(&mut self, _: u32) -> Result<Option<bool>> {
        Ok(self.input["virtual_100"].as_u64().map(|v| v & 1 != 0))
    }
    fn virtual_20(&mut self, _: u32) -> Result<bool> {
        Ok(self.b("virtual_20"))
    }
    fn announce(&mut self, _: u32, message: u32, with_name: bool) -> Result<()> {
        if with_name {
            self.events.push(json!({"name": true}));
        }
        self.events.push(json!({"v108": true}));
        self.events
            .push(json!({"announce": message, "manager": true}));
        Ok(())
    }
    fn set_state(&mut self, _: u32, state: u32, enabled: bool) -> Result<()> {
        self.events
            .push(json!({"state": state, "enabled": u32::from(enabled)}));
        Ok(())
    }
    fn refresh_under_siege(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"siege": true}));
        Ok(())
    }
    fn component_death(&mut self, component: u32, credited: Option<u32>) -> Result<()> {
        self.events
            .push(json!({"component_death": component, "credited": credited.unwrap_or(0)}));
        Ok(())
    }
    fn linked_component_88(&self, _: u32) -> Result<Option<u32>> {
        Ok(self.b("linked88").then_some(0x88))
    }
    fn component_88_death(&mut self, _: u32, _: u32) -> Result<()> {
        self.events.push(json!({"component88_death": true}));
        Ok(())
    }
    fn is_wall_member(&mut self, _: u32) -> Result<bool> {
        Ok(self.b("wall_member"))
    }
    fn wall_member_died(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"wall_5859c0": true}));
        self.events.push(json!({"wall_584d50": 0}));
        Ok(())
    }
    fn stop(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"stop": 0}));
        Ok(())
    }
    fn notify(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"notify": true}));
        Ok(())
    }
    fn refresh_37e0(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"refresh_37e0": true}));
        Ok(())
    }
    fn definition_1b2(&self, _: u32) -> Result<bool> {
        Ok(self.b("def_1b2"))
    }
    fn virtual_50(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"virtual_50": 0}));
        Ok(())
    }
    fn start_dying(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"start_dying": true}));
        Ok(())
    }
    fn request_removal(&mut self, _: u32) -> Result<bool> {
        self.events.push(json!({"removal": true}));
        Ok(self.b("removed"))
    }
}

#[test]
fn begin_death_matches_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-begin-death-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 160);
    let (mut announced, mut removed) = (0, 0);
    for case in cases {
        let input = case["input"].clone();
        let mut world = Adapter {
            flags8: input["flags8"].as_u64().unwrap() as u32,
            input,
            events: vec![],
        };
        // The native kill credit sets +8 bit 4 inside 4D57C0 (aa-body);
        // the fixture's seam did not, so the flags stay as given.
        begin_death(1, &mut world).unwrap();
        assert_eq!(Value::Array(world.events.clone()), case["events"], "{case}");
        assert_eq!(
            u64::from(world.flags8),
            case["flags8_after"].as_u64().unwrap(),
            "{case}"
        );
        announced += usize::from(world.events.iter().any(|e| e.get("announce").is_some()));
        removed += usize::from(world.events.iter().any(|e| e.get("removal").is_some()));
    }
    assert!(announced > 10 && removed > 10, "{announced} {removed}");
}
