//! Replays research/phase8-economy/native_structure_state.py: original
//! structure enable 0x565400, disable 0x564DC0 and command recompute 0x565140
//! (0x565090, 0x585F00, 0x4D14C0/0x4D0820, 0x517FA0, 0x584E10, 0x584DE0).
use aa_production::Result;
use aa_production::construction::Construction;
use aa_production::structure_state::*;
use serde_json::{Value, json};

struct World {
    op: Value,
    group_mask: u64,
    states: u64,
    mask: u64,
    events: Vec<Value>,
}
impl World {
    fn flag(&self, name: &str) -> bool {
        self.op[name].as_bool().unwrap()
    }
    fn settlement(&self) -> &Value {
        &self.op["settlement"]
    }
}
impl Construction for World {
    fn construction_kingdom(&mut self) -> Result<Option<u32>> {
        unreachable!()
    }
    fn construction_cost(&mut self, _: u32) -> Result<Vec<u32>> {
        unreachable!()
    }
    fn construction_can_afford(&mut self, _: u32, _: &[u32]) -> Result<bool> {
        unreachable!()
    }
    fn construction_debit(&mut self, _: u32, _: &[u32]) -> Result<()> {
        unreachable!()
    }
    fn construction_credit(&mut self, _: u32, _: &[u32]) -> Result<()> {
        unreachable!()
    }
    fn notify_construct_unaffordable(&mut self, _: u32, _: &[u32], _: u32) -> Result<()> {
        unreachable!()
    }
    fn notify_upgrade_unaffordable(&mut self, _: u32, _: &[u32], _: Option<u32>) -> Result<()> {
        unreachable!()
    }
    fn has_structure(&mut self) -> Result<bool> {
        unreachable!()
    }
    fn state_active(&mut self, state: u32) -> Result<bool> {
        self.events
            .push(json!({"kind": "state_active", "state": state}));
        Ok(state == STATE_OUT_OF_SUPPLY && self.flag("out_of_supply"))
    }
    fn actor_flags(&mut self) -> Result<u64> {
        Ok(self.states)
    }
    fn definition_flags(&mut self, _: u32) -> Result<u32> {
        unreachable!()
    }
    fn kingdom_allows(&mut self, _: u32, _: u32) -> Result<bool> {
        unreachable!()
    }
    fn upgrade_list(&mut self) -> Result<Vec<u32>> {
        Ok(if self.flag("upgrades") {
            vec![1]
        } else {
            vec![]
        })
    }
    fn construction_begun(&mut self, _: Option<u32>, _: u32) -> Result<()> {
        unreachable!()
    }
    fn apply_upgrade_modifiers(&mut self, _: u32) -> Result<()> {
        unreachable!()
    }
    fn construction_actor_state(&mut self, state: u32, enabled: bool) -> Result<()> {
        // Test double mirrored by the oracle's 0x4D64F0 seam.
        self.events
            .push(json!({"kind": "actor_state", "state": state, "enabled": enabled}));
        if enabled {
            self.states |= 1 << state;
        } else {
            self.states &= !(1 << state);
        }
        Ok(())
    }
    fn destroyed(&mut self) -> Result<bool> {
        unreachable!()
    }
    fn destroy_actor(&mut self) -> Result<()> {
        unreachable!()
    }
}
impl StructureState for World {
    fn state_group_mask(&mut self, group: u32) -> Result<u64> {
        assert_eq!(group, MODE_GROUP);
        Ok(self.group_mask)
    }
    fn command_mask(&mut self) -> Result<u64> {
        Ok(self.mask)
    }
    fn set_command_mask(&mut self, mask: u64) -> Result<()> {
        self.mask = mask;
        Ok(())
    }
    fn settlement_center_disabled(&mut self) -> Result<bool> {
        self.events.push(json!({"kind": "settlement_center"}));
        Ok(self.flag("center") && self.flag("center_disabled"))
    }
    fn actor_virtual_100(&mut self) -> Result<bool> {
        self.events.push(json!({"kind": "virtual_100"}));
        Ok(self.flag("virtual_100"))
    }
    fn in_settlement(&mut self) -> Result<bool> {
        self.events.push(json!({"kind": "in_settlement"}));
        Ok(self.flag("in_settlement"))
    }
    fn settlement_upgrade_facts(&mut self) -> Result<SettlementUpgradeFacts> {
        let s = self.settlement();
        let constructing: Vec<bool> = s["structures"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c.as_bool().unwrap())
            .collect();
        Ok(SettlementUpgradeFacts {
            count: s["count"].as_i64().unwrap() as i32,
            limit: s["limit"].as_i64().unwrap() as i32,
            constructing,
        })
    }
    fn settlement_members(&mut self) -> Result<SettlementMembers> {
        let s = self.settlement();
        Ok(SettlementMembers {
            settlement_actor: 1000,
            center: s["center"].as_bool().unwrap().then_some(1001),
            structures: (0..s["structures"].as_array().unwrap().len() as u32)
                .map(|i| 2000 + i)
                .collect(),
            members: s["members"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(i, p)| p.as_bool().unwrap().then_some(3000 + i as u32))
                .collect(),
        })
    }
    fn has_component_90(&mut self) -> Result<bool> {
        Ok(self.flag("c90"))
    }
    fn can_disable(&mut self) -> Result<bool> {
        Ok(self.flag("can_disable"))
    }
    fn can_sell(&mut self) -> Result<bool> {
        Ok(self.flag("can_sell"))
    }
    fn token_word(&mut self) -> Result<Option<u32>> {
        self.events.push(json!({"kind": "token"}));
        Ok(self.op["token"].as_u64().map(|t| t as u32))
    }
    fn allow_sell_under_siege(&mut self) -> Result<bool> {
        Ok(self.flag("allow_sell_under_siege"))
    }
    fn refresh_actor_organization(&mut self, actor: u32) -> Result<()> {
        let name = match actor {
            1 => "actor".to_owned(),
            1000 => "settlement".to_owned(),
            1001 => "center".to_owned(),
            2000..=2999 => format!("s{}", actor - 2000),
            _ => format!("m{}", actor - 3000),
        };
        self.events.push(json!({"kind": "refresh", "actor": name}));
        Ok(())
    }
    fn clear_queue(&mut self, structure: u32) -> Result<()> {
        self.events
            .push(json!({"kind": "clear_queue", "structure": format!("s{}", structure - 2000)}));
        Ok(())
    }
    fn this_actor(&mut self) -> Result<u32> {
        Ok(1)
    }
}

#[test]
fn structure_state_matches_original_x86() {
    let document: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-economy/native-structure-state-fixtures.json"
    ))
    .unwrap();
    let group_mask = document["group_mask"].as_u64().unwrap();
    let mut executions = 0;
    for fixture in document["fixtures"].as_array().unwrap() {
        let op = &fixture["op"];
        let mut world = World {
            op: op.clone(),
            group_mask,
            states: op["states"].as_u64().unwrap(),
            mask: op["mask"].as_u64().unwrap(),
            events: vec![],
        };
        let label = format!("{} {}", fixture["name"], fixture["precision"]);
        match op["kind"].as_str().unwrap() {
            "enable" => enable_structure(&mut world),
            "disable" => disable_structure(&mut world),
            _ => recompute_commands(&mut world),
        }
        .expect(&label);
        let actual = json!({"states": world.states, "mask": world.mask, "events": world.events});
        assert_eq!(actual, fixture["expected"], "{label}");
        executions += 1;
    }
    assert_eq!(executions, document["executions"].as_u64().unwrap());
}

#[test]
fn state_group_is_the_lowest_set_state() {
    assert_eq!(state_in_group(0, u64::MAX), None);
    assert_eq!(
        state_in_group((1 << 0x1E) | (1 << 0x1B), 1 << 0x1E),
        Some(0x1E)
    );
    assert_eq!(
        state_in_group((1 << 0x1E) | (1 << 0x1B), u64::MAX),
        Some(0x1B)
    );
}
