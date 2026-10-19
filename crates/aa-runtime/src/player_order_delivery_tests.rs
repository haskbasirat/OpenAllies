//! native-selected-delivery-4ed360.json: original 4ED360 with a NULL group.
//! (1) The trace of an explicit command is identical for all 39 ActorCommand
//! types (the reason deliver_selected_with_bank has no type bound).
//! (2) Every row follows the per-actor sequence deliver_selected_with_bank
//! implements: B4, [guess (default command) -> NULL skips], permit
//! (vt+150 = 4CF430 with the current definition), [live +164 group ->
//! 4D1900/4DFF30, which the Rust binding refuses], 4D9B60, [4D9A90(0)].
use std::collections::BTreeMap;

fn fixture() -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-orders-input/recruit/native-selected-delivery-4ed360.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn expected(input: &serde_json::Value) -> Vec<serde_json::Value> {
    use serde_json::json;
    let flags = input["flags"].as_u64().unwrap();
    let guessing = flags & 0x40 == 0;
    let actors = input["actors"].as_array().unwrap();
    let mut out = vec![json!(["sync", "_command", true])];
    let mut current = (input["definition"].as_u64(), flags);
    for (i, a) in actors.iter().enumerate() {
        out.push(json!(["b4", i]));
        if a["b4"].as_u64().unwrap() & 0xff == 0 {
            continue;
        }
        if guessing {
            let multiple = actors.len() > 1;
            out.push(json!(["guess", i, u32::from(multiple)]));
            let Some(g) = a["guess"].as_u64() else { continue };
            current = (Some(g), current.1 | 0x40);
        }
        let fields = json!({"definition": current.0, "flags": current.1});
        out.push(json!(["permit", i, fields]));
        if a["permit"].as_u64().unwrap() & 0xff != 0 {
            if a["link"].as_bool().unwrap() && a["link_generation"] == 7 && a["link_live"].as_bool().unwrap() {
                out.push(json!(["link", i]));
                out.push(json!(["prepare"]));
            }
            out.push(json!(["deliver", i, fields]));
        }
        if guessing {
            current = (None, current.1 & !0x40);
        }
    }
    out
}

#[test]
fn player_order_selected_delivery_4ed360_null_group_is_type_invariant() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 11520);
    let mut groups: BTreeMap<(String, String, u64), Vec<&serde_json::Value>> = BTreeMap::new();
    for row in rows {
        assert_eq!(row["output"]["events"].as_array().unwrap(), &expected(&row["input"]), "{row}");
        let key = (row["image"].as_str().unwrap().to_owned(), row["precision"].as_str().unwrap().to_owned(),
            row["input"]["group"].as_u64().unwrap());
        groups.entry(key).or_default().push(row);
    }
    let mut explicit = 0;
    for rows in groups.values() {
        if rows[0]["input"]["definition"].is_null() {
            continue;
        }
        assert_eq!(rows.len(), 39);
        let kinds: Vec<u64> = rows.iter().map(|r| r["input"]["definitions"][0][0].as_u64().unwrap()).collect();
        assert_eq!(kinds, (0..39).collect::<Vec<_>>());
        for r in rows {
            assert_eq!(r["output"], rows[0]["output"], "type {} differs", r["input"]["definitions"][0][0]);
        }
        explicit += 1;
    }
    assert_eq!(explicit, 2 * 3 * 48);
}
