use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    native::TypedValue,
    objects,
    sources::{Origin, ResolvedFile, sha256},
    tgi::Limits,
};
use serde_json::Value;
fn catalog(text: &str) -> ActorCatalog {
    ActorCatalog::from_files(
        vec![ResolvedFile {
            logical_path: "objects.tgi".into(),
            origin: Origin::V14Loose,
            sha256: sha256(text.as_bytes()),
            bytes: text.as_bytes().to_vec(),
        }],
        Limits::default(),
    )
}
#[test]
fn original_shape_setters_match_all_precision_cases() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-shapes.json"
    ))
    .unwrap();
    let rows = fixture["shape_cases"].as_array().unwrap();
    assert_eq!(rows.len(), 480);
    for row in rows {
        let pc = serde_json::from_value(row["precision"].clone()).unwrap();
        let dimensions: Vec<u32> = serde_json::from_value(row["arguments"].clone()).unwrap();
        let result =
            objects::shape(row["selector"].as_u64().unwrap() as u32, &dimensions, pc).unwrap();
        assert_eq!(
            serde_json::to_value(result.words).unwrap(),
            row["words"],
            "{row}"
        );
    }
    for (selector, dims) in [
        (4, vec![]),
        (3, vec![0]),
        (1, vec![f32::NAN.to_bits()]),
        (2, vec![f32::INFINITY.to_bits()]),
    ] {
        assert!(objects::shape(selector, &dims, Precision::Double53).is_err());
    }
}
#[test]
fn native_plain_string_null_conversion_preserves_source_provenance() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-strings.json"
    ))
    .unwrap();
    for row in fixture["cases"].as_array().unwrap() {
        let raw = row["raw"].as_str().unwrap();
        let source = format!("[Thing]{{\nIDS=sample\nstring field={raw}\n}}\n");
        let cat = catalog(&source);
        let TypedValue::String {
            value,
            source: span,
            ..
        } = cat
            .native_field("sample", "field", Precision::Double53)
            .unwrap()
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(
            serde_json::to_value(value.encode_utf16().collect::<Vec<_>>()).unwrap(),
            row["value_utf16"]
        );
        assert_eq!(
            &source[span.span.start..span.span.end],
            format!("string field={raw}")
        );
    }
}
#[test]
fn soft_null_selects_kind_but_target_null_copies_soft_words() {
    let cat = catalog(
        "[Template Actor]{\nstring category=ACTOR\n[Size]{\nfixed height=2\nfixed scale_min=3\nfixed scale_max=4\nstring blocking_shape=CIRCLE\nfixed blocking_radius=0.2\nstring soft_shape=null\nfixed soft_radius=0.5\nstring target_shape=NULL\n}\n}\n[Thing template=Actor]{\nIDS=unit\n}\n",
    );
    let d = cat
        .native_object_definition("unit", Precision::Double53)
        .unwrap();
    assert_eq!(d.blocking.words, [1, 0.2f32.to_bits(), 0, 0, 0, 0]);
    assert_eq!(d.soft.words, [1, 0.5f32.to_bits(), 0, 0, 0, 0]);
    assert_eq!(d.target, d.soft);
    assert!(!d.has_explicit_target);
    assert_eq!(d.scale_min.bits, 3f32.to_bits()); //Visual scale does not multiply the native shapes.
}
