use super::*;
use serde_json::{Value, json};

fn projection(value: &State) -> Value {
    let mut selected = Vec::new();
    let mut node = value.selected.head();
    while let Some(id) = node {
        let n = value.selected.node(id).unwrap();
        selected.push(n.actor.raw());
        node = n.next;
    }
    json!({"kingdom":value.kingdom.map_or(0,ActorId::raw),"word_0c":0xa5a5a5a5u32,
      "mask_words":value.mask_words_10_1c,"group_null":value.current_group.is_null(),
      "look_at":value.look_at_28_2c,"selection":selected,"tail_null":value.selected.tail().is_none(),
      "wildcard":value.wildcard().map(|g|json!({"key":g.key(),"title":g.title(),"text":g.text(),
       "members_count":g.members().len(),"capacity":g.capacity_hint(),"initial_slot":g.initial_null_slot().map_or(0,ActorId::raw),
       "flag_18":u8::from(g.flag_18()),"word_1c":g.word_1c(),"flag_20":u8::from(g.flag_20())}))})
}
#[test]
fn native_player_view_constructor_and_destructor_rows() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-player-view-state/native-player-view.json"
    ))
    .unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let c = &row["case"];
        let kingdom = c["kingdom"].as_u64().unwrap() as u32;
        let caption: Vec<u16> = c["caption"].as_str().unwrap().encode_utf16().collect();
        let mut calls = 0;
        let mut value =
            State::construct((kingdom != 0).then(|| ActorId::from_raw(kingdom)), |key| {
                calls += 1;
                assert_eq!(key, WILDCARD_KEY.encode_utf16().collect::<Vec<_>>());
                Ok(caption.clone())
            })
            .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(value.word_0c(), None);
        assert_eq!(projection(&value), row["before"]);
        for actor in c["selected"].as_array().unwrap().iter().rev() {
            value
                .selected
                .prepend(ActorId::from_raw(actor.as_u64().unwrap() as u32))
                .unwrap();
        }
        match c["operation"].as_str().unwrap() {
            "construct" => {}
            "destroy" => value.destroy().unwrap(),
            "replace" => {
                value.destroy().unwrap();
                let kingdom = c["new_kingdom"].as_u64().unwrap() as u32;
                value =
                    State::construct((kingdom != 0).then(|| ActorId::from_raw(kingdom)), |_| {
                        Ok(caption)
                    })
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let mut expected = row["after"].clone();
        expected.as_object_mut().unwrap().remove("raw28");
        assert_eq!(projection(&value), expected, "{c}");
        //Every selected-node free follows wildcard destruction/buffer release.
        let events = row["trace"].as_array().unwrap();
        let mut wildcard_freed = false;
        for event in events {
            if event["free"] == "wildcard" {
                wildcard_freed = true;
            }
            if event["free"]
                .as_str()
                .is_some_and(|v| v.starts_with("selection:"))
            {
                assert!(wildcard_freed);
                assert_eq!(event["wildcard"], false);
            }
        }
    }
}
#[test]
fn failed_group_retirement_keeps_same_selection_and_unavailable_text_never_completes() {
    let mut value = State::construct(None, |_| Ok(vec![65])).unwrap();
    let first = value
        .selected
        .prepend(ActorId::from_raw(0x1000010))
        .unwrap();
    let second = value
        .selected
        .prepend(ActorId::from_raw(0x1000011))
        .unwrap();
    //Explicit future-positive-member boundary probe, not a native constructor
    //claim: no public group-member producer currently exists.
    value
        .wildcard
        .as_mut()
        .unwrap()
        .members
        .push(ActorId::from_raw(0x1000020));
    assert!(value.destroy().unwrap_err().contains("4CE850"));
    assert!(value.wildcard().is_some());
    assert_eq!(value.selected.head(), Some(second));
    assert_eq!(value.selected.node(second).unwrap().next, Some(first));
    assert!(!value.wildcard().unwrap().flag_18());
    assert_eq!(
        State::construct(None, |_| Err("actual TextManager unavailable".into())).unwrap_err(),
        "actual TextManager unavailable"
    );
}
