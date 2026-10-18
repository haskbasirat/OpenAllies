use aa_actors::{ActorId, IdentityRegistry};
use aa_runtime::names::Table;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
#[derive(Deserialize)]
struct Actor {
    id: u32,
    actor: bool,
    kingdom: bool,
}
#[derive(Deserialize)]
struct Operation {
    kind: String,
    name: String,
    id: u32,
    flags: Option<u32>,
}
#[derive(Deserialize)]
struct Input {
    name: String,
    actors: Vec<Actor>,
    operations: Vec<Operation>,
}
#[derive(Deserialize)]
struct Case {
    input: Input,
    output: Vec<Value>,
}
#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}
#[test]
fn native_name_registration_duplicates_rehash_and_live_class_resolution() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../research/phase7-runtime-names/native-names.json"
    ))
    .unwrap();
    assert_eq!(fixture.cases.len(), 11);
    assert_eq!(
        fixture.cases.iter().map(|c| c.output.len()).sum::<usize>(),
        2569
    );
    for case in fixture.cases {
        let mut table = Table::default();
        let mut registry = IdentityRegistry::new();
        for actor in case.input.actors {
            let id = ActorId::from_raw(actor.id);
            registry.seed_saved_generation(id).unwrap();
            registry.register_saved(id, actor.actor, actor).unwrap();
        }
        let mut names = BTreeSet::new();
        for (op, expected) in case.input.operations.into_iter().zip(case.output) {
            let text = op.name.encode_utf16().collect::<Vec<_>>();
            let mut flags = op.flags.unwrap_or(0x81000000);
            names.insert(op.name.to_ascii_uppercase());
            let result = match op.kind.as_str() {
                "alias" => {
                    table.alias(&text, ActorId::from_raw(op.id)).unwrap();
                    Value::Null
                }
                "publish" => {
                    table
                        .publish(&text, ActorId::from_raw(op.id), &mut flags)
                        .unwrap();
                    Value::Null
                }
                "lookup" => json!(table.lookup(&text).unwrap()),
                "remove" => json!(table.remove_first(&text).unwrap()),
                "actor" | "kingdom" => json!(
                    table
                        .resolve(&text, &registry, |entry| if op.kind == "actor" {
                            entry.value.actor
                        } else {
                            entry.value.kingdom
                        })
                        .unwrap()
                        .map(|entry| entry.id)
                ),
                _ => panic!("unexpected native operation"),
            };
            let mut state = json!({"buckets":table.bucket_count(),"count":table.len()});
            if expected["table"].get("entries").is_some() {
                let mut entries = serde_json::Map::new();
                for name in &names {
                    let rows = table
                        .entries(&name.encode_utf16().collect::<Vec<_>>())
                        .unwrap();
                    if !rows.is_empty() {
                        entries.insert(name.clone(),json!(rows.iter().map(|r|json!({"name":String::from_utf16(&r.name).unwrap(),"id":r.id})).collect::<Vec<_>>()));
                    }
                }
                state["entries"] = Value::Object(entries);
            }
            assert_eq!(
                json!({"result":result,"flags":flags,"table":state}),
                expected,
                "{} {} {}",
                case.input.name,
                op.kind,
                op.name
            );
        }
    }
}
#[test]
fn unknown_locale_is_rejected_after_native_publication_flag_write() {
    let mut table = Table::default();
    let mut flags = 0x81000000;
    assert!(
        table
            .publish(&[0xe9], ActorId::from_raw(1), &mut flags)
            .is_err()
    );
    assert_eq!(flags, 0x01000000);
    assert!(table.is_empty());
    assert!(table.alias(&[65, 0, 66], ActorId::from_raw(2)).is_err());
    assert!(table.lookup(&[0xe9]).is_err());
}
