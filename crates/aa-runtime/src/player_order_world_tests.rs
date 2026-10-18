use super::*;
use crate::player_selection::{self as selection, View, World as SelectionWorld};
use serde_json::{Value, json};
fn u(c: &Value, k: &str, d: u32) -> u32 {
    c[k].as_u64().map(|x| x as u32).unwrap_or(d)
}
fn b(c: &Value, k: &str) -> bool {
    c[k].as_bool().unwrap_or(false)
}
struct Permission<'a> {
    c: &'a Value,
    calls: Vec<&'static str>,
}
impl Permission<'_> {
    fn actor(&self, id: ActorId) -> &Value {
        if id.raw() == 2 {
            &self.c["parent"]
        } else {
            self.c
        }
    }
}
impl PermissionWorld for Permission<'_> {
    fn flags(&mut self, id: ActorId) -> Result<u32> {
        Ok(u(self.actor(id), "flags", 0))
    }
    fn parent(&mut self, id: ActorId) -> Result<Option<ActorId>> {
        let c = self.actor(id);
        Ok((b(c, "element") && b(c, "element_parent")).then_some(ActorId::from_raw(2)))
    }
    fn present(&mut self, id: ActorId, slot: ComponentSlot) -> Result<bool> {
        Ok(b(
            self.actor(id),
            match slot {
                ComponentSlot::Mover => "mover",
                ComponentSlot::MobileDeployment => "mobile",
                ComponentSlot::Recruit => "recruit",
                ComponentSlot::Denizen => "denizen",
                _ => panic!("unexpected native eligibility field"),
            },
        ))
    }
    fn gate(&mut self) -> Result<bool> {
        self.calls.push("gate");
        Ok(b(self.c, "gate"))
    }
    fn phase(&mut self) -> Result<u32> {
        self.calls.push("phase");
        Ok(u(self.c, "phase", 0))
    }
    fn owner(&mut self, _: ActorId) -> Result<Option<ActorId>> {
        Ok(match self.c["owner"].as_str().unwrap_or("same") {
            "same" => Some(ActorId::from_raw(100)),
            "other" => Some(ActorId::from_raw(101)),
            "null" => None,
            _ => panic!(),
        })
    }
    fn actions(&mut self, id: ActorId) -> Result<u64> {
        Ok(self.actor(id)["action_bits"].as_u64().unwrap_or(0))
    }
}
struct Selected {
    list: selection::List,
    masks: [u32; 4],
    point: [u32; 2],
    null_group: bool,
}
impl View for Selected {
    fn player_id(&self) -> u32 {
        99
    }
    fn list(&self) -> &selection::List {
        &self.list
    }
    fn list_mut(&mut self) -> &mut selection::List {
        &mut self.list
    }
    fn kingdom(&self) -> Option<ActorId> {
        None
    }
    fn mask_words_mut(&mut self) -> &mut [u32; 4] {
        &mut self.masks
    }
    fn look_at_mut(&mut self) -> &mut [u32; 2] {
        &mut self.point
    }
    fn group_is_null(&self) -> std::result::Result<bool, String> {
        Ok(self.null_group)
    }
    fn clear_group(&mut self) {
        self.null_group = true;
    }
}
struct Storage<'a> {
    c: &'a Value,
    bytes: [u8; 2],
}
impl SelectionWorld for Storage<'_> {
    fn application_state(&mut self) -> std::result::Result<u32, String> {
        Ok(u(self.c, "application", 2))
    }
    fn is_actor(&mut self, _: ActorId) -> std::result::Result<bool, String> {
        Ok(true)
    }
    fn element_parent(&mut self, _: ActorId) -> std::result::Result<Option<ActorId>, String> {
        Ok((b(self.c, "element") && b(self.c, "element_parent")).then_some(ActorId::from_raw(2)))
    }
    fn change_byte_4c(&mut self, id: ActorId, inc: bool) -> std::result::Result<(), String> {
        let v = &mut self.bytes[id.raw() as usize - 1];
        *v = if inc {
            v.wrapping_add(1)
        } else {
            v.wrapping_sub(1)
        };
        Ok(())
    }
    fn update_group(&mut self, v: &mut impl View) -> std::result::Result<(), String> {
        let list = v.list();
        let single = match (list.head(), list.tail()) {
            (None, None) => true,
            (Some(h), Some(t)) => list.node(h)?.actor == list.node(t)?.actor,
            _ => panic!(),
        };
        if single {
            v.clear_group();
            Ok(())
        } else {
            Err("positive_group_store".into())
        }
    }
    fn rebuild_masks(&mut self, v: &mut impl View) -> std::result::Result<(), String> {
        // Explicit native-memory-host leaf: admitted singleton class3/4 +40
        // dispatches actual887B50 false. Concrete binding is checked separately.
        v.mask_words_mut().fill(0);
        Ok(())
    }
}
#[test]
fn original_permission_selection_and_constructor_rows() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-player-orders/native-player-orders.json"
    ))
    .unwrap();
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1206);
    for row in rows {
        let c = &row["input"];
        let result = match row["kind"].as_str().unwrap() {
            "permission" => {
                let mut w = Permission { c, calls: vec![] };
                let mut index = Some(u(c, "kind", 5));
                if u(c, "command_flags", 0x40) & 0x40 == 0 {
                    w.calls.push("default");
                    if !c["default"].as_bool().unwrap_or(true) {
                        index = None;
                    }
                }
                let allowed = permission_native(
                    &mut w,
                    ActorId::from_raw(1),
                    Some(ActorId::from_raw(100)),
                    index,
                    u(c, "selected", 1) as u8,
                )
                .unwrap();
                json!({"allowed":allowed,"calls":w.calls})
            }
            "selection" => {
                let mut v = Selected {
                    list: selection::List::from_vworld_constructor(),
                    masks: [u32::MAX; 4],
                    point: [0; 2],
                    null_group: true,
                };
                for id in c["initial"]
                    .as_array()
                    .map(Vec::as_slice)
                    .unwrap_or(&[])
                    .iter()
                    .rev()
                {
                    v.list
                        .prepend(ActorId::from_raw(id.as_u64().unwrap() as u32))
                        .unwrap();
                }
                let mut w = Storage {
                    c,
                    bytes: [u(c, "byte4c", 0) as u8, 0],
                };
                let result = match c["operation"].as_str().unwrap() {
                    "select" => selection::select(
                        &mut w,
                        &mut v,
                        c["candidate"]
                            .as_bool()
                            .unwrap_or(true)
                            .then_some(ActorId::from_raw(1)),
                    )
                    .map(|_| ()),
                    "deselect" => selection::deselect(
                        &mut w,
                        &mut v,
                        if c["candidate"].as_bool().unwrap_or(true) {
                            ActorId::from_raw(1)
                        } else {
                            ActorId::NULL
                        },
                    )
                    .map(|_| ()),
                    "clear" => selection::clear(&mut w, &mut v),
                    _ => panic!(),
                };
                let result = result.err().unwrap_or_else(|| "complete".into());
                let mut ids = vec![];
                let mut next = v.list.head();
                while let Some(node) = next {
                    let n = v.list.node(node).unwrap();
                    ids.push(n.actor.raw());
                    next = n.next;
                }
                json!({"result":result,"selected":ids,"byte4c":w.bytes,"mask_words":v.masks,"group_null":v.null_group})
            }
            "constructor" => {
                let input = if u(c, "kind", 0) == 5 {
                    Input::Move {
                        destination: [
                            c["position"][0].as_u64().unwrap() as u32,
                            c["position"][1].as_u64().unwrap() as u32,
                        ],
                        auxiliary: [
                            c["auxiliary"][0].as_u64().unwrap() as u32,
                            c["auxiliary"][1].as_u64().unwrap() as u32,
                        ],
                        speed: u(c, "speed", 0),
                        trailing: 0,
                    }
                } else {
                    Input::Attack {
                        target: if b(c, "target") {
                            ActorId::from_raw(1)
                        } else {
                            ActorId::NULL
                        },
                        trailing: 0,
                    }
                };
                let d = aa_rules::native::actor_command::predefined_actor_command(
                    if u(c, "kind", 0) == 5 { "move" } else { "kill" },
                )
                .unwrap();
                let p = command_fields(d, 1, input);
                json!({"kind":p.definition.unwrap().mind_type,"flags":p.flags,"word10":p.word10,"object":p.object_definition.unwrap_or(0),"position":p.position_bits,"auxiliary":p.auxiliary_bits,"speed":p.scalar_bits,"parameter":p.parameter_definition.unwrap_or(0),"boolean":p.boolean,"parameters":0,"extra":p.extra_definition.unwrap_or(0),"trailing":p.trailing})
            }
            _ => panic!(),
        };
        assert_eq!(
            result, row["output"],
            "{} {} {} {c}",
            row["image"], row["precision"], row["kind"]
        );
    }
}
#[test]
fn stable_selection_cursor_survives_previous_node_retirement() {
    let mut list = selection::List::from_vworld_constructor();
    let tail = list.prepend(ActorId::from_raw(1)).unwrap();
    let head = list.prepend(ActorId::from_raw(2)).unwrap();
    let captured = list.node(head).unwrap().next;
    list.remove(head).unwrap();
    assert_eq!(captured, Some(tail));
    assert_eq!(list.node(tail).unwrap().actor, ActorId::from_raw(1));
    assert!(list.node(head).is_err());
    let new = list.prepend(ActorId::from_raw(3)).unwrap();
    assert_ne!(
        new, head,
        "native pointer identities are not compacted/reused"
    );
}
