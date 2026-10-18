use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    native::TypedValue,
    sources::{Origin, ResolvedFile, sha256},
    tgi::Limits,
};
#[test]
fn original_nation_badge_count_and_order_are_not_technology_indices() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-nation-badges.json"
    ))
    .unwrap();
    for row in data["cases"].as_array().unwrap() {
        let mut src = String::from(
            "[Template Nation]{\nstring IDS\n[list Badge]{string badge_texture}\n}\n[Nation template=Nation]{\nIDS=test\n",
        );
        for tag in row["source_tags"].as_array().unwrap() {
            src.push_str(&format!("[Badge]{{badge_texture=badge_{tag}}}\n"));
        }
        src.push_str("}\n");
        let cat = ActorCatalog::from_files(
            vec![ResolvedFile {
                logical_path: "nation.tgi".into(),
                origin: Origin::V14Loose,
                sha256: sha256(src.as_bytes()),
                bytes: src.as_bytes().to_vec(),
            }],
            Limits::default(),
        );
        let out = cat
            .native_nation_badges("TEST", Precision::Double53)
            .unwrap();
        let stored: Vec<_> = row["stored"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| format!("badge_{v}"))
            .collect();
        let actual: Vec<_> = out
            .textures
            .iter()
            .map(|v| match v {
                TypedValue::String { value, .. } => value.clone(),
                _ => panic!(),
            })
            .collect();
        assert_eq!(out.nation_id, "test");
        assert_eq!(out.count().unwrap() as usize, stored.len());
        assert_eq!(actual, stored);
    }
}
