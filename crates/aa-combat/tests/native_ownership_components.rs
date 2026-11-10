//! 0x589050 / 0x579F10 / 0x584F10 / 0x585FF0 against
//! research/phase8-combat/native-ownership-components-fixtures.json.
use aa_combat::{
    Result,
    ownership_components::{
        ComponentKingdomWorld, WallList, linked_change_kingdom, organization_change_kingdom,
        seen_by, wall_change_kingdom,
    },
};
use serde_json::{Value, json};

const COMPONENT: u32 = 0xC0;
const LINKED: u32 = 0x10;
const LIVE_HANDLE: u32 = 0x1234_0001;

/// Actors are labelled by name; lists keep (name, shrink) entries.
struct Adapter {
    names: Vec<String>,
    structure: Option<u32>,
    lists: [Vec<Option<(u32, u32)>>; 2],
    counts: [i32; 2],
    members: Vec<u32>,
    events: Vec<Value>,
}
impl Adapter {
    fn new() -> Self {
        Adapter {
            names: vec![],
            structure: None,
            lists: [vec![], vec![]],
            counts: [0, 0],
            members: vec![],
            events: vec![],
        }
    }
    fn actor(&mut self, name: &str) -> u32 {
        self.names.push(name.to_string());
        0x100 + self.names.len() as u32 - 1
    }
    fn slot(list: WallList) -> usize {
        match list {
            WallList::Structures => 0,
            WallList::Segments => 1,
        }
    }
}
impl ComponentKingdomWorld for Adapter {
    fn resolve(&self, handle: u32) -> Result<Option<u32>> {
        Ok((handle == LIVE_HANDLE).then_some(LINKED))
    }
    fn change_kingdom(&mut self, actor: u32, new: Option<u32>) -> Result<()> {
        let name = if actor == LINKED {
            "linked".to_string()
        } else {
            self.names[(actor - 0x100) as usize].clone()
        };
        self.events.push(json!({"change": name, "kingdom": new.unwrap_or(0)}));
        for list in 0..2 {
            if let Some((_, by)) = self.lists[list].iter().flatten().find(|(a, _)| *a == actor) {
                self.counts[list] -= *by as i32;
            }
        }
        Ok(())
    }
    fn organization_members(&mut self, component: u32) -> Result<Vec<u32>> {
        assert_eq!(component, COMPONENT);
        Ok(self.members.clone())
    }
    fn wall_structure(&self, component: u32) -> Result<Option<u32>> {
        assert_eq!(component, COMPONENT);
        Ok(self.structure)
    }
    fn wall_count(&self, component: u32, list: WallList) -> Result<i32> {
        assert_eq!(component, COMPONENT);
        Ok(self.counts[Self::slot(list)])
    }
    fn wall_entry(&self, component: u32, list: WallList, index: i32) -> Result<Option<u32>> {
        assert_eq!(component, COMPONENT);
        Ok(self.lists[Self::slot(list)][index as usize].map(|(a, _)| a))
    }
}

fn kingdom(case: &Value) -> Option<u32> {
    Some(case["kingdom"].as_u64().unwrap() as u32).filter(|&k| k != 0)
}

#[test]
fn component_callbacks_match_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-ownership-components-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 190);
    for case in cases {
        let mut world = Adapter::new();
        match case["function"].as_str().unwrap() {
            "linked" => {
                let handle = match case["kind"].as_str().unwrap() {
                    "live" => LIVE_HANDLE,
                    "zero" => 0,
                    _ => LIVE_HANDLE + 0x10000,
                };
                linked_change_kingdom(handle, kingdom(case), &mut world).unwrap();
            }
            "organization" => {
                let count = case["count"].as_u64().unwrap() as usize;
                let ids: Vec<u32> = (0..count).map(|k| world.actor(&format!("m{k}"))).collect();
                world.members = case["order"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|i| ids[i.as_u64().unwrap() as usize])
                    .collect();
                organization_change_kingdom(COMPONENT, kingdom(case), &mut world).unwrap();
            }
            "wall" => {
                if case["structure"].as_bool().unwrap() {
                    world.structure = Some(world.actor("structure"));
                }
                for (slot, name) in ["structures", "segments"].iter().enumerate() {
                    let entries = case["lists"][name].as_array().unwrap();
                    let mut list = vec![];
                    for entry in entries {
                        list.push(match entry {
                            Value::Null => None,
                            Value::String(s) => Some((world.actor(s), 0)),
                            other => Some((
                                world.actor(other["id"].as_str().unwrap()),
                                other["shrink"].as_u64().unwrap() as u32,
                            )),
                        });
                    }
                    world.counts[slot] = list.len() as i32;
                    world.lists[slot] = list;
                }
                wall_change_kingdom(COMPONENT, kingdom(case), &mut world).unwrap();
            }
            "seen" => {
                let mut seen: Vec<u32> = case["seen"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|k| k.as_u64().unwrap() as u32)
                    .collect();
                seen_by(&mut seen, case["kingdom"].as_u64().unwrap() as u32);
                assert_eq!(json!(seen), case["after"], "{case}");
                assert!(case["tail_is_last"].as_bool().unwrap());
                continue;
            }
            other => panic!("{other}"),
        }
        assert_eq!(Value::Array(world.events), case["events"], "{case}");
    }
}
