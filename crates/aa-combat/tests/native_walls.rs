//! Wall / denizen / membership seams against
//! research/phase8-combat/native-wall-fixtures.json.
use aa_combat::{
    Result,
    organization::{DetachWorld, detach_membership},
    walls::{
        IntruderWorld, TransferWorld, World, intruder_near_wall, propagate_under_siege,
        transfer_component_88, transfer_wall, wall_complete,
    },
};
use serde_json::{Value, json};

fn doc() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-wall-fixtures.json"
    ))
    .unwrap()
}

#[test]
fn wall_complete_matches_native() {
    for case in doc()["cases"].as_array().unwrap() {
        let flags: Vec<Option<u32>> = case["flags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f.as_u64().map(|v| v as u32))
            .collect();
        let count = case["count"].as_i64().unwrap() as i32;
        assert_eq!(
            u64::from(wall_complete(count, &flags)),
            case["result"].as_u64().unwrap(),
            "{case}"
        );
    }
}

// Actor ids: 1 wall actor, 2 owner, 10+i segment i; wall component 100.
struct Siege {
    case: Value,
    events: Vec<Value>,
}
fn name(actor: u32) -> Value {
    match actor {
        1 => json!("wall_actor"),
        2 => json!("owner"),
        n => json!(format!("segment{}", n - 10)),
    }
}
impl World for Siege {
    fn wall_actor(&self, _: u32) -> Result<u32> {
        Ok(1)
    }
    fn wall_owner(&self, _: u32) -> Result<Option<u32>> {
        Ok(Some(2))
    }
    fn wall_segments(&self, _: u32) -> Result<Vec<Option<u32>>> {
        Ok(self.case["segments"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, s)| match s.as_str() {
                Some("seg") => Some(10 + i as u32),
                Some("owner") => Some(2),
                _ => None,
            })
            .collect())
    }
    fn flags100(&self, actor: u32) -> Result<u32> {
        Ok(match actor {
            1 => self.case["actor_flags"].as_u64().unwrap() as u32,
            2 => self.case["owner_flags"].as_u64().unwrap() as u32,
            _ => 0,
        })
    }
    fn set_state(&mut self, actor: u32, state: u32, enabled: bool) -> Result<()> {
        self.events
            .push(json!({"state": name(actor), "value": state, "enabled": u32::from(enabled)}));
        Ok(())
    }
    fn announce_under_siege(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"announce": 7}));
        Ok(())
    }
    fn notify(&mut self, actor: u32) -> Result<()> {
        self.events.push(json!({"notify": name(actor), "args": []}));
        Ok(())
    }
    fn required_segments(&self, _: u32) -> Result<i32> {
        Ok(self.case["required"].as_i64().unwrap() as i32)
    }
    fn set_flag13(&mut self, actor: u32, value: bool) -> Result<()> {
        self.events
            .push(json!({"flag13": name(actor), "bit": 13, "value": u32::from(value)}));
        Ok(())
    }
    fn refresh_segments(&mut self, _: u32) -> Result<()> {
        self.events
            .push(json!({"refresh_segments": "wall", "args": []}));
        Ok(())
    }
    fn owner_component_88(&self, _: u32) -> Result<Option<u32>> {
        Ok(Some(100))
    }
}

#[test]
fn under_siege_propagation_matches_native() {
    for case in doc()["sieges"].as_array().unwrap() {
        let mut world = Siege {
            case: case.clone(),
            events: vec![],
        };
        propagate_under_siege(100, &mut world).unwrap();
        assert_eq!(Value::Array(world.events), case["events"], "{case}");
    }
}

// Transfer: actor id = 1000 + 16*link + index.
struct Transfer {
    siege: Siege,
    links: Vec<Vec<String>>,
    refcounts: Vec<u64>,
}
impl World for Transfer {
    fn wall_actor(&self, w: u32) -> Result<u32> {
        self.siege.wall_actor(w)
    }
    fn wall_owner(&self, w: u32) -> Result<Option<u32>> {
        self.siege.wall_owner(w)
    }
    fn wall_segments(&self, w: u32) -> Result<Vec<Option<u32>>> {
        self.siege.wall_segments(w)
    }
    fn flags100(&self, a: u32) -> Result<u32> {
        self.siege.flags100(a)
    }
    fn set_state(&mut self, a: u32, s: u32, e: bool) -> Result<()> {
        self.siege.set_state(a, s, e)
    }
    fn announce_under_siege(&mut self, a: u32) -> Result<()> {
        self.siege.announce_under_siege(a)
    }
    fn notify(&mut self, a: u32) -> Result<()> {
        self.siege.notify(a)
    }
    fn required_segments(&self, w: u32) -> Result<i32> {
        self.siege.required_segments(w)
    }
    fn set_flag13(&mut self, a: u32, v: bool) -> Result<()> {
        self.siege.set_flag13(a, v)
    }
    fn refresh_segments(&mut self, w: u32) -> Result<()> {
        self.siege.refresh_segments(w)
    }
    fn owner_component_88(&self, w: u32) -> Result<Option<u32>> {
        self.siege.owner_component_88(w)
    }
}
fn actor_name(id: u32) -> String {
    format!("l{}a{}", (id - 1000) / 16, (id - 1000) % 16)
}
impl TransferWorld for Transfer {
    fn denizen_links(&self, _: u32) -> Result<Vec<Vec<Option<u32>>>> {
        Ok(self
            .links
            .iter()
            .enumerate()
            .map(|(li, link)| {
                link.iter()
                    .enumerate()
                    .map(|(ai, kind)| (kind != "dead").then_some(1000 + 16 * li as u32 + ai as u32))
                    .collect()
            })
            .collect())
    }
    fn has_component_70(&self, actor: u32) -> Result<bool> {
        let (li, ai) = (
            ((actor - 1000) / 16) as usize,
            ((actor - 1000) % 16) as usize,
        );
        Ok(self.links[li][ai] != "no70")
    }
    fn component_70_transfer(&mut self, actor: u32, _: u32) -> Result<bool> {
        let n = actor_name(actor);
        self.siege.events.push(json!({"c70_40": n, "arg": 0}));
        self.siege
            .events
            .push(json!({"c70_18c": n, "owner": "denizen_owner"}));
        let (li, ai) = (
            ((actor - 1000) / 16) as usize,
            ((actor - 1000) % 16) as usize,
        );
        Ok(self.links[li][ai] == "accept")
    }
    fn begin_death(&mut self, actor: u32) -> Result<()> {
        self.siege
            .events
            .push(json!({"begin_death": actor_name(actor), "args": []}));
        Ok(())
    }
    fn denizen_actor(&self, _: u32) -> Result<u32> {
        Ok(3)
    }
    fn release_link(&mut self, _: u32, index: usize) -> Result<()> {
        if self.refcounts[index] == 1 {
            self.siege.events.push(json!({"link_delete": index}));
        }
        Ok(())
    }
    fn clear_denizen_links(&mut self, _: u32) -> Result<()> {
        for _ in &self.links {
            self.siege.events.push(json!({"link_node_dtor": true}));
        }
        Ok(())
    }
}

#[test]
fn denizen_transfer_matches_native() {
    let doc = doc();
    let cases = doc["transfers"].as_array().unwrap();
    assert_eq!(cases.len(), 30);
    for case in cases {
        let mut world = Transfer {
            siege: Siege {
                case: json!({}),
                events: vec![],
            },
            links: serde_json::from_value(case["links"].clone()).unwrap(),
            refcounts: serde_json::from_value(case["refcounts"].clone()).unwrap(),
        };
        if case["entry"] == "wall" {
            transfer_wall(100, &mut world).unwrap();
        } else {
            transfer_component_88(100, &mut world).unwrap();
        }
        assert_eq!(Value::Array(world.siege.events), case["events"], "{case}");
        assert!(
            case["list_cleared"].as_bool().unwrap() && case["chain_cleared"].as_bool().unwrap()
        );
    }
}

/// Intruder list entries keep their original index (actor id) across the
/// `0x569300` front drops.
struct Intruders {
    list: Vec<(u32, String)>,
    within: Vec<String>,
    events: Vec<Value>,
}
impl IntruderWorld for Intruders {
    fn intruders(&self, _: u32) -> Result<Vec<Option<(u32, bool)>>> {
        Ok(self
            .list
            .iter()
            .map(|(id, kind)| match kind.as_str() {
                "dead" => None,
                other => Some((*id, other == "dying")),
            })
            .collect())
    }
    fn drop_front_intruder(&mut self, _: u32) -> Result<()> {
        self.list.remove(0);
        Ok(())
    }
    fn within_wall(&mut self, _: u32, actor: u32) -> Result<bool> {
        let n = format!("i{actor}");
        self.events.push(json!({"within_wall": n}));
        Ok(self.within.contains(&n))
    }
}

#[test]
fn intruder_predicate_matches_native() {
    for case in doc()["intruders"].as_array().unwrap() {
        let list: Vec<String> = serde_json::from_value(case["list"].clone()).unwrap();
        let mut world = Intruders {
            list: list
                .into_iter()
                .enumerate()
                .map(|(k, s)| (k as u32, s))
                .collect(),
            within: serde_json::from_value(case["within"].clone()).unwrap(),
            events: vec![],
        };
        let got = intruder_near_wall(100, 1, &mut world).unwrap();
        assert_eq!(u64::from(got), case["result"].as_u64().unwrap(), "{case}");
        assert_eq!(Value::Array(world.events), case["events"], "{case}");
        assert_eq!(
            !world.list.is_empty(),
            case["remaining"].as_bool().unwrap(),
            "{case}"
        );
    }
}

struct Detach {
    case: Value,
    events: Vec<Value>,
}
impl DetachWorld for Detach {
    fn has_component_b0(&self, _: u32) -> Result<bool> {
        Ok(self.case["has_b0"].as_bool().unwrap())
    }
    fn detach_component_b0(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"detach_b0": "b0", "args": []}));
        Ok(())
    }
    fn clear_membership(&mut self, _: u32) -> Result<()> {
        Ok(())
    }
    fn refresh_member(&mut self, _: u32) -> Result<()> {
        self.events
            .push(json!({"refresh_member": "member", "args": []}));
        Ok(())
    }
    fn observer_present(&self) -> Result<bool> {
        Ok(self.case["observer"].as_bool().unwrap())
    }
    fn member_flags8(&self, _: u32) -> Result<u32> {
        Ok(self.case["flags8"].as_u64().unwrap() as u32)
    }
    fn notify_observer(&mut self, _: u32) -> Result<()> {
        self.events.push(json!({"observer": "member"}));
        Ok(())
    }
}

#[test]
fn membership_detach_matches_native() {
    for case in doc()["detaches"].as_array().unwrap() {
        let mut world = Detach {
            case: case.clone(),
            events: vec![],
        };
        detach_membership(1, &mut world).unwrap();
        assert_eq!(Value::Array(world.events), case["events"], "{case}");
        assert_eq!(case["cleared"], json!([0, 0xffff_ffffu32, 0]));
    }
}

#[test]
fn within_wall_geometry_matches_native() {
    use aa_body::F32;
    use aa_combat::walls::{intruder_radius, intruder_within_wall, point_within_wall};
    use aa_numeric::Precision;
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-wall-geometry-fixtures.json"
    ))
    .unwrap();
    let f = |v: &Value| F32(v.as_u64().unwrap() as u32);
    let pair = |v: &Value| [f(&v[0]), f(&v[1])];
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 400);
    for case in cases {
        let polygon: Vec<[F32; 2]> = case["polygon"]
            .as_array()
            .unwrap()
            .iter()
            .map(pair)
            .collect();
        let pc = Precision::Double53;
        let radius = intruder_radius(
            f(&case["tile"]),
            case["organization"]
                .as_bool()
                .unwrap()
                .then(|| f(&case["organization_radius"])),
            f(&case["radius"]),
            pc,
        )
        .unwrap();
        let within = intruder_within_wall(
            pair(&case["wall"]),
            pair(&case["actor"]),
            radius,
            &polygon,
            pc,
        )
        .unwrap();
        assert_eq!(
            u64::from(within),
            case["within"].as_u64().unwrap(),
            "{case}"
        );
        let direct =
            point_within_wall(pair(&case["point"]), pair(&case["wall"]), &polygon, pc).unwrap();
        assert_eq!(
            u64::from(direct),
            case["point_within"].as_u64().unwrap(),
            "{case}"
        );
    }
}
