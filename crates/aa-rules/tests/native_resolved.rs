use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    native::TypedValue,
    sources::{Origin, ResolvedFile, sha256},
    tgi::Limits,
};
use serde_json::Value;
fn catalog(text: &str) -> ActorCatalog {
    ActorCatalog::from_files(
        vec![ResolvedFile {
            logical_path: "resolved.tgi".into(),
            origin: Origin::V14Loose,
            sha256: sha256(text.as_bytes()),
            bytes: text.as_bytes().to_vec(),
        }],
        Limits::default(),
    )
}
#[test]
fn inherited_list_order_matches_executed_native_copy() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-lists.json"
    ))
    .unwrap();
    for row in fixture["list_cases"].as_array().unwrap() {
        let n: Vec<usize> = serde_json::from_value(row["counts"].clone()).unwrap();
        let mut source = String::from(
            "[Template Item]{\nstring ids\nfloat weight=1\n}\n[Template Base]{\n[View]{\n[list Aspect template=Item]\n",
        );
        for i in 0..n[0] {
            source += &format!("[Aspect]\nids={}\n", 10 + i);
        }
        source += "}\n}\n[Template Derived inherit=Base]{\n[View]{\n";
        for i in 0..n[1] {
            source += &format!("[Aspect]\nids={}\n", 20 + i);
        }
        source += "}\n}\n[Thing template=Derived]{\nIDS=unit\n[View]{\n";
        for i in 0..n[2] {
            source += &format!("[Aspect]\nids={}\n", 30 + i);
        }
        source += "}\n}\n";
        let cat = catalog(&source);
        let nodes = cat
            .resolved_actor("unit")
            .unwrap()
            .child("View")
            .unwrap()
            .unwrap()
            .children("Aspect")
            .unwrap();
        let tags: Vec<u32> = nodes
            .iter()
            .map(|node| {
                let TypedValue::String { value, .. } =
                    node.field("ids", Precision::Double53).unwrap().unwrap()
                else {
                    panic!()
                };
                let TypedValue::Scalar(weight) =
                    node.field("weight", Precision::Double53).unwrap().unwrap()
                else {
                    panic!()
                };
                assert_eq!(weight.bits, 1f32.to_bits());
                value.parse().unwrap()
            })
            .collect();
        assert_eq!(
            serde_json::to_value(tags).unwrap(),
            row["ordered_source_tags"]
        );
    }
}
#[test]
fn descendant_child_replacement_matches_native_ancestry_guard() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-lists.json"
    ))
    .unwrap();
    for row in fixture["ancestry_cases"].as_array().unwrap() {
        let n = row["depth"].as_u64().unwrap() as usize;
        let index = row["required_template_index"].as_u64().map(|n| n as usize);
        let name = |i: usize| {
            if Some(i) == index {
                "Required".to_owned()
            } else {
                format!("N{i}")
            }
        };
        let mut source = String::new();
        if index.is_none() {
            source += "[Template Required]{\nfixed value=1\n}\n";
        }
        for i in 0..n {
            source += &format!(
                "[Template {}{}]{{\nfixed value=2\n}}\n",
                name(i),
                if i + 1 < n {
                    format!(" inherit={}", name(i + 1))
                } else {
                    String::new()
                }
            );
        }
        source += "[Template Parent]{\n[Child template=Required]\n}\n[Thing template=Parent]{\nIDS=unit\n";
        source += &format!(
            "[Child{}]\n}}\n",
            if n > 0 {
                format!(" template={}", name(0))
            } else {
                String::new()
            }
        );
        let cat = catalog(&source);
        let actual = cat.native_node_sources("unit", &["Child"]).is_ok();
        assert_eq!(actual, row["accepted"].as_bool().unwrap(), "{row} {source}");
        assert_eq!(
            cat.resolved_actor("unit").unwrap().child("Child").is_ok(),
            actual
        );
    }
}

#[test]
fn creation_aspect_order_and_stored_weight_total_match_native() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-creation-definitions.json"
    ))
    .unwrap();
    for row in fixture["cases"].as_array().unwrap() {
        let pc: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let enabled = row["enabled"].as_bool().unwrap();
        let mut source = String::from(
            "[Template Aspect]{\nstring IDS\nfloat weight=1\n}\n[Template Object]{\nstring category=OBJECT\n[Size]{\nfixed height=2\nfixed scale_min=1\nfixed scale_max=1\nstring blocking_shape=POINT\nstring soft_shape=null\nstring target_shape=null\n}\n[View]{\nflag aspects_as_variations=false\n[AspectNormal template=Aspect]\n[list Aspect template=Aspect]\n}\n}\n[Thing template=Object]{\nIDS=sample\n[View]{\n",
        );
        source += &format!(
            "aspects_as_variations={enabled}\n[AspectNormal]{{\nIDS=normal\nweight=0.25\n}}\n"
        );
        for (i, weight) in row["weights"].as_array().unwrap().iter().enumerate() {
            source += &format!(
                "[Aspect]{{\nIDS=aspect{i}\nweight={}\n}}\n",
                f32::from_bits(weight.as_u64().unwrap() as u32)
            );
        }
        source += "}\n}\n";
        let cat = catalog(&source);
        let got = cat.native_creation_definition("sample", pc).unwrap();
        assert_eq!(got.variations_enabled, enabled);
        assert_eq!(got.aspects.len() as u64, row["stored_count"]);
        assert_eq!(
            got.total_variation_weight_bits as u64, row["stored_total_bits"],
            "{row}"
        );
        let tags: Vec<i32> = got
            .aspects
            .iter()
            .map(|a| {
                let TypedValue::String { value, .. } = &a.id else {
                    panic!()
                };
                if value == "normal" {
                    -1
                } else {
                    value.strip_prefix("aspect").unwrap().parse().unwrap()
                }
            })
            .collect();
        assert_eq!(serde_json::to_value(tags).unwrap(), row["loaded_tags"]);
    }
}

#[test]
fn integers_and_points_match_original_field_assignment() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-values.json"
    ))
    .unwrap();
    for row in fixture["cases"].as_array().unwrap() {
        let kind = row["kind"].as_str().unwrap();
        let ty = if kind == "integer" { "int" } else { kind };
        let pc = row
            .get("precision")
            .map(|v| serde_json::from_value(v.clone()).unwrap())
            .unwrap_or(Precision::Double53);
        let source = format!(
            "[Thing]{{\nIDS=sample\n{ty} value={}\n}}\n",
            row["raw"].as_str().unwrap()
        );
        let cat = catalog(&source);
        let value = cat
            .resolved_actor("sample")
            .unwrap()
            .field("value", pc)
            .unwrap()
            .unwrap();
        let words = match value {
            TypedValue::Integer { value, .. } => vec![value as u32],
            TypedValue::Point { bits, .. } => bits,
            _ => panic!(),
        };
        assert_eq!(serde_json::to_value(words).unwrap(), row["words"], "{row}");
    }
    for bad in ["1+2", "2147483648", "3.5"] {
        assert!(
            catalog(&format!("[Thing]{{\nIDS=sample\nint value={bad}\n}}\n"))
                .resolved_actor("sample")
                .unwrap()
                .field("value", Precision::Double53)
                .is_err()
        );
    }
}

#[test]
fn root_registry_lookup_preserves_inheritance_and_rejects_duplicate_ids() {
    let source = "[template Registry]{\nstring IDS\nint count=4\n}\n[AbilityType template=Registry]{\nIDS=attack\n}\n";
    let cat = catalog(source);
    assert_eq!(cat.resolved_definitions("AbilityType").unwrap().len(), 1);
    assert_eq!(
        cat.resolved_definition("AbilityType", "ATTACK", Precision::Double53)
            .unwrap()
            .field("count", Precision::Double53)
            .unwrap()
            .unwrap()
            .integer()
            .unwrap(),
        4
    );
    assert!(
        catalog(&(source.to_owned() + "[AbilityType template=Registry]{\nIDS=attack\n}\n"))
            .resolved_definition("AbilityType", "attack", Precision::Double53)
            .is_err()
    );
}
