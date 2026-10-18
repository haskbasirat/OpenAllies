use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    native::{CATEGORIES, ScalarType, TypedValue},
    sources::{Origin, ResolvedFile, sha256},
    tgi::Limits,
};
use serde_json::Value;
fn catalog(text: &str) -> ActorCatalog {
    ActorCatalog::from_files(
        vec![ResolvedFile {
            logical_path: "typed.tgi".into(),
            origin: Origin::V14Loose,
            sha256: sha256(text.as_bytes()),
            bytes: text.as_bytes().to_vec(),
        }],
        Limits::default(),
    )
}
#[test]
fn typed_inheritance_retains_value_and_declaration_provenance_without_coercion() {
    let src = "[Template Body]{\nfixed health\nfixed defense=2\n}\n[Template Base]{\nstring category=UNIT\n[optional BodyComponent Template=Body]\n}\n[Thing Template=Base]{\nIDS=soldier\n[BodyComponent]\nhealth=12.5%\n}\n[Thing Template=Base]{\nIDS=empty\n}\n";
    let c = catalog(src);
    let field = c
        .native_field("soldier", "BodyComponent.health", Precision::Double53)
        .unwrap()
        .unwrap();
    let TypedValue::Scalar(v) = field else {
        panic!()
    };
    assert_eq!(v.bits, 0.125f32.to_bits());
    assert_eq!(v.field_type, ScalarType::Fixed);
    assert_eq!(&src[v.source.span.start..v.source.span.end], "health=12.5%");
    assert_eq!(
        &src[v.declaration_source.span.start..v.declaration_source.span.end],
        "fixed health"
    );
    assert!(
        c.native_field("empty", "BodyComponent.health", Precision::Double53)
            .unwrap()
            .is_none()
    );
    assert_eq!(c.native_class("soldier").unwrap(), 3);
    let bad =
        catalog("[Thing]{\nIDS=bad\nstring category=ACTOR\n[BodyComponent]\nfloat health=100\n}\n");
    assert!(
        bad.native_field("bad", "BodyComponent.health", Precision::Double53)
            .unwrap()
            .unwrap()
            .fixed_bits()
            .is_err()
    );
}
#[test]
fn unresolved_required_values_and_performer_lists_remain_explicit() {
    let c = catalog(
        "[Template Base]{\nstring category=ACTOR\n[BodyComponent]\nfixed health\n[optional MoverComponent]\nfixed speed_max=2\n[PerformerComponent]\n}\n[Thing Template=Base]{\nIDS=actor\n}\n",
    );
    assert!(
        c.native_field("actor", "BodyComponent.health", Precision::Double53)
            .is_err()
    );
    assert!(c.native_component_mask("actor", None).is_err());
    assert_eq!(c.native_component_mask("actor", Some(0)).unwrap().bits, 1);
    assert_eq!(c.native_component_mask("actor", Some(2)).unwrap().bits, 33);
    assert!(
        c.native_node_sources("actor", &["MoverComponent"])
            .unwrap()
            .is_empty()
    );
}
#[test]
fn all_original_category_descriptor_constructors_match_builtin_table() {
    let d: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-actors/categories-native.json"
    ))
    .unwrap();
    assert_eq!(d["categories"].as_array().unwrap().len(), CATEGORIES.len());
    for row in d["categories"].as_array().unwrap() {
        let cat = CATEGORIES
            .iter()
            .find(|c| c.name == row["name"].as_str().unwrap())
            .unwrap();
        assert_eq!(serde_json::to_value(cat).unwrap(), *row);
        let c = catalog(&format!(
            "[Thing]{{\nIDS=example\nstring category={}\n}}\n",
            cat.name
        ));
        assert_eq!(c.native_category("example").unwrap(), *cat);
    }
}

#[test]
fn runtime_class_does_not_substitute_for_definition_factory_kind() {
    for category in ["RANDOM_ACTOR", "RANDOM_OBJECT", "VFXMODEL"] {
        let c = catalog(&format!(
            "[Thing]{{\nIDS=example\nstring category={category}\n}}\n"
        ));
        assert!(
            c.native_final_flags("example", Precision::Double53)
                .unwrap_err()
                .to_string()
                .contains("definition kind")
        );
        assert!(c.native_component_mask("example", Some(0)).is_err());
    }
}
