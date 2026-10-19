use super::*;
use serde_json::Value;

fn fixture() -> Value {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/mapload-20261018/aspect-texture-rename/native-aspect-texture-rename.json"
    ))
    .unwrap();
    assert_eq!(
        doc["original_sha256"],
        "5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26"
    );
    assert_eq!(
        doc["v14_sha256"],
        "3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1"
    );
    assert_eq!(doc["precisions"].as_array().unwrap().len(), 3);
    assert_eq!(doc["images"].as_array().unwrap().len(), 2);
    doc
}
///NULL and an empty C string reach469D10 identically (SafeStringLen 0).
fn wide(v: &Value) -> Vec<u16> {
    v.as_str().map_or(vec![], |s| s.encode_utf16().collect())
}
fn units(v: &Value) -> Vec<u16> {
    serde_json::from_value(v.clone()).unwrap()
}
fn text(v: &Value) -> &str {
    v.as_str().unwrap()
}

#[test]
fn native_469d10_replace_rows_match() {
    let doc = fixture();
    let rows = doc["replace"].as_array().unwrap();
    assert_eq!(rows.len(), 35);
    let mut refused = 0;
    for row in rows {
        let (input, native) = (&row["input"], &row["native"]);
        let mut value = units(&input["source"]);
        let before = value.clone();
        let result = replace_469d10(&mut value, &wide(&input["old"]), &wide(&input["new"]));
        match text(&native["outcome"]) {
            "ok" => {
                assert_eq!(result.unwrap(), native["count"], "{input}");
                assert_eq!(value, units(&native["units"]), "{input}");
                assert_eq!(native["terminator"], 0);
            }
            "uninitialized_published_units" => {
                //Native publishes count-pass length over storage no store of
                //this Replace wrote (a later NUL segment shifted past the
                //original end is never visited).
                assert!(result.is_err(), "{input}");
                assert_eq!(value, before);
                refused += 1;
            }
            other => panic!("unexpected native outcome {other}"),
        }
    }
    assert_eq!(refused, 2);
}

#[test]
fn replace_469d10_c_string_arguments_end_at_first_nul() {
    //Node strings are passed as data pointers: their first NUL ends them.
    let mut v: Vec<u16> = "abcb".encode_utf16().collect();
    assert_eq!(
        replace_469d10(&mut v, &[98, 0, 99], &[120, 0, 121]).unwrap(),
        2
    );
    assert_eq!(String::from_utf16(&v).unwrap(), "axcx");
    let mut v: Vec<u16> = "abc".encode_utf16().collect();
    assert_eq!(replace_469d10(&mut v, &[0, 98], &[120]).unwrap(), 0);
    assert_eq!(String::from_utf16(&v).unwrap(), "abc");
}

#[test]
fn native_46a8a0_list_order_and_46a180_names_match() {
    let doc = fixture();
    let cases = doc["loader"].as_array().unwrap();
    assert_eq!(cases.len(), 7);
    for case in cases {
        let (input, native) = (&case["input"], &case["native"]);
        let fields = &input["fields"];
        let children = input["renames"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| TextureRename {
                from: wide(&r[0]),
                to: wide(&r[1]),
            })
            .collect();
        let renames = TextureRenames::from_source_order(children);
        //Aspect+3C head-first; +40 is the first-created (source-first) node.
        let list: Vec<Value> = renames
            .traversal()
            .iter()
            .map(|r| {
                serde_json::json!([
                    String::from_utf16(&r.from).unwrap(),
                    String::from_utf16(&r.to).unwrap()
                ])
            })
            .collect();
        assert_eq!(Value::Array(list), native["rename_list"]);
        assert_eq!(native["tail_is_last_node"], true);
        assert_eq!(native["search_44_is_object_b4"], true);
        let (folder, prefix) = (text(&input["folder"]), text(&input["resource_prefix"]));
        assert_eq!(native["resource_prefix_0"], prefix);
        assert_eq!(native["aspect_prefix_4"], fields["aspect_prefix"]);
        //Same seam-transported source chain the existing model producer uses.
        assert_eq!(
            crate::data_pre_request::path::model_path(
                folder,
                prefix,
                text(&fields["aspect_prefix"]),
                text(&fields["model"])
            )
            .unwrap(),
            text(&native["model_20"])
        );
        //TextureRename children are read before skin, skin before glow.
        let trace: Vec<String> = native["trace"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e[0] == "string")
            .map(|e| e[1].as_str().unwrap().to_owned())
            .collect();
        let skin = trace.iter().position(|k| k == "skin").unwrap();
        assert!(
            trace
                .iter()
                .rposition(|k| k == "substring_to")
                .map_or(true, |i| i < skin)
        );
        assert_eq!(trace[skin + 1], "glow");
        let joined = format!("{prefix}{}", text(&fields["aspect_prefix"]));
        let mut requests = native["textures"].as_array().unwrap().iter();
        for (key, slot) in [("skin", "skin_30"), ("glow", "glow_34")] {
            match renames
                .texture_name_46a180(text(&fields[key]), Some(&joined), folder)
                .unwrap()
            {
                None => assert_eq!(native[slot], 0, "{key} {input}"),
                Some(name) => {
                    let request = requests.next().unwrap();
                    assert_eq!(name, text(&request["name"]), "{key} {input}");
                    assert_eq!(request["search_owner_is_object_b4"], true);
                    assert_eq!(native[slot], request["result"]);
                }
            }
        }
        for direct in native["direct_46a180_flag0"].as_array().unwrap() {
            let requests = direct["textures"].as_array().unwrap();
            match renames
                .texture_name_46a180(text(&direct["source"]), None, folder)
                .unwrap()
            {
                None => {
                    assert_eq!(direct["result"], 0);
                    assert!(requests.is_empty() && direct["trace"].as_array().unwrap().is_empty());
                }
                Some(name) => {
                    assert_eq!(requests.len(), 1);
                    assert_eq!(name, text(&requests[0]["name"]), "{direct}");
                    assert_eq!(direct["result"], requests[0]["result"]);
                }
            }
        }
    }
}

#[test]
fn actual_gbr_brigade_engineer_source_input_matches_native_chain() {
    //Buildings/GBR/gbr_brigade_engineer.tgi AspectNormal: skin %sBase.dds
    //(AABuildingPackable), glow NULL, one TextureRename DeathArcticBase->Base.
    let doc = fixture();
    let case = &doc["loader"][0];
    assert_eq!(case["input"]["definition"], "gbr_brigade_engineer");
    let renames = TextureRenames::from_source_order(vec![TextureRename {
        from: "DeathArcticBase".encode_utf16().collect(),
        to: "Base".encode_utf16().collect(),
    }]);
    let prefix = "GBRBrigadeEngineer/GBRBrigadeEngineer";
    let skin = renames
        .texture_name_46a180("%sBase.dds", Some(prefix), "Buildings/GBR")
        .unwrap()
        .unwrap();
    assert_eq!(
        skin,
        "Buildings/GBR/GBRBrigadeEngineer/GBRBrigadeEngineerBase.dds"
    );
    assert_eq!(skin, case["native"]["textures"][0]["name"]);
    assert!(
        renames
            .texture_name_46a180("", Some(prefix), "Buildings/GBR")
            .unwrap()
            .is_none()
    );
    //The actual NIF texture names (46AE20 flag0) contain no DeathArcticBase.
    for name in ["GBRBrigadeEngineerBase.dds", "GBRBrigadeEngineerGround.dds"] {
        assert_eq!(
            renames
                .texture_name_46a180(name, None, "Buildings/GBR")
                .unwrap()
                .unwrap(),
            format!("Buildings/GBR/{name}")
        );
    }
}
