use aa_numeric::Precision;
use aa_rules::{
    actors::{
        ActorCatalog,
        native_load::{NativeLoadReport, load_files},
    },
    native::layout_data::{LayoutError, NativeLayoutData, NativeThrow, PropertyRef},
    native_member_property_data::{Source, load_list},
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use serde_json::{Value, json};

fn evidence(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-member-property-data")
        .join(name)
}
struct Replay<'a> {
    input: &'a Value,
    events: Vec<Value>,
}
impl Source for Replay<'_> {
    fn child_count(&mut self) -> Result<i32, LayoutError> {
        let count = self.input["count"].as_i64().unwrap() as i32;
        self.events.push(json!({"op":"count","value":count}));
        Ok(count)
    }
    fn property_ids(&mut self, index: i32) -> Result<String, LayoutError> {
        self.events.push(json!({"op":"ids","index":index}));
        self.input["ids"][index as usize]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| {
                LayoutError::Native(NativeThrow::MissingField {
                    field: "property_IDS".into(),
                })
            })
    }
    fn find_property(&mut self, ids: &str) -> Result<Option<PropertyRef>, LayoutError> {
        self.events.push(json!({"op":"lookup","ids":ids}));
        let result = self.input["lookups"][ids]
            .as_u64()
            .map(|n| PropertyRef::from_handle(n as u32).unwrap());
        // The fixture supplies successful allocation as an explicit leaf.
        if result.is_some() {
            self.events.push(json!({"op":"allocate","bytes":8}));
        }
        Ok(result)
    }
}
#[test]
fn native_list_source_order_duplicates_and_throw_prefixes_match() {
    let doc: Value = serde_json::from_slice(
        &std::fs::read(evidence("native-member-property-data.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(doc["cases"].as_array().unwrap().len(), 72);
    for row in doc["cases"].as_array().unwrap() {
        let mut source = Replay {
            input: &row["input"],
            events: vec![],
        };
        let mut list: Vec<_> = row["input"]["initial"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| PropertyRef::from_handle(n.as_u64().unwrap() as u32).unwrap())
            .collect();
        let outcome = match load_list(&mut source, &mut list) {
            Ok(()) => "ok",
            Err(LayoutError::Native(NativeThrow::MissingField { .. })) => "missing_field",
            Err(LayoutError::Native(NativeThrow::Data { .. })) => "unknown_property",
            other => panic!("unexpected {other:?}"),
        };
        let handles: Vec<_> = list.iter().map(|p| p.handle().unwrap()).collect();
        assert_eq!(
            json!({"outcome":outcome,"head_first":handles,"tail_value":handles.last(),"events":source.events}),
            row["output"],
            "{row}"
        );
    }
}
fn build_catalog(text: &str) -> ActorCatalog {
    let templates = concat!(
        "[template Property]\n{ string IDS\n string name\n flag is_technology = false\n}\n",
        "[template SupportReference]\n{ string property_IDS\n}\n",
        "[template ElementComponent]\n{ [list SupportProperty Template=SupportReference]\n}\n",
    );
    let file = |path: &str, text: &str| {
        let bytes = text.as_bytes().to_vec();
        ResolvedFile {
            logical_path: path.into(),
            origin: Origin::V14Loose,
            sha256: sha256(&bytes),
            bytes,
        }
    };
    load_files(
        vec![
            file("member-templates.tgi", templates),
            file("member-source.tgi", text),
        ],
        Limits::default(),
        &mut NativeLoadReport::default(),
    )
}
fn property(id: &str) -> String {
    format!("[Property Template=Property]\n{{ IDS = {id}\n name = {id}\n}}\n")
}
fn actor(id: &str, kind: &str, ids: &[&str]) -> String {
    let mut text = format!(
        "[Thing]\n{{ string IDS = {id}\n string category = {kind}\n [ElementComponent Template=ElementComponent]\n {{\n"
    );
    for id in ids {
        text.push_str(&format!(
            " [SupportProperty]\n {{ property_IDS = {id}\n }}\n"
        ));
    }
    text.push_str(" }\n}\n");
    text
}
#[test]
fn canonical_owners_source_visibility_and_shared_property_identity_are_preserved() {
    let source = format!(
        "{}{}{}{}{}{}{}",
        property("a"),
        property("b"),
        "[Translate]\n{ string node = Property\n string ids_old = alias\n string ids_new = a\n}\n",
        actor("owner", "ACTOR", &["a", "alias", "b"]),
        actor("target", "RANDOM_ACTOR", &[]),
        "[Translate]\n{ string node = Thing\n string ids_old = owner\n string ids_new = target\n}\n",
        "[Translate]\n{ string node = Property\n string ids_old = a\n string ids_new = b\n}\n"
    );
    let catalog = build_catalog(&source);
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let layout = NativeLayoutData::load(&catalog, pc).unwrap();
        let data = catalog
            .native_member_property_data(&layout, "OWNER")
            .unwrap();
        assert_eq!(data.id, "owner");
        assert!(data.element_present);
        assert!(!data.sources.is_empty());
        assert_eq!(
            data.properties,
            vec![
                PropertyRef::Registered(1),
                PropertyRef::Registered(0),
                PropertyRef::Registered(0)
            ]
        );
        assert!(
            catalog
                .native_member_property_data(&layout, "target")
                .unwrap()
                .properties
                .is_empty()
        );
        assert!(
            catalog
                .native_member_property_data(&layout, "alias")
                .is_err()
        );
    }
    // A Property registered later is invisible at the owner load position.
    let catalog = build_catalog(&(actor("early", "ACTOR", &["late"]) + &property("late")));
    let layout = NativeLayoutData::load(&catalog, Precision::Double53).unwrap();
    assert!(matches!(
        catalog.native_member_property_data(&layout, "early"),
        Err(LayoutError::Native(NativeThrow::Data { .. }))
    ));
}
#[test]
fn non_actor_and_unordered_sources_do_not_receive_empty_certificates() {
    let catalog = build_catalog(&actor("object", "OBJECT", &[]));
    let layout = NativeLayoutData::load(&catalog, Precision::Double53).unwrap();
    assert!(
        catalog
            .native_member_property_data(&layout, "object")
            .is_err()
    );
    let unordered = ActorCatalog::from_files(vec![], Limits::default());
    assert!(
        unordered
            .native_member_property_data(&layout, "object")
            .is_err()
    );
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn actual_actor_support_properties_use_owned_source_registry() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let catalog = ActorCatalog::load_v14(&mut files).unwrap();
    let mut precisions = vec![];
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let layout = NativeLayoutData::load(&catalog, pc).unwrap();
        let mut rows = vec![];
        for owner in &catalog.actors {
            // Canonical source category must be read without translating the owner.
            match catalog.native_member_property_data(&layout, &owner.id) {
                Ok(data) => rows.push(data),
                Err(LayoutError::Unsupported(message))
                    if message == "45EC60 requires ActorData or derived RandomActorData" =>
                {
                    ()
                }
                other => panic!("{}: {other:?}", owner.id),
            }
        }
        assert!(rows.len() >= 1084);
        assert!(rows.iter().all(|row| !row.sources.is_empty()));
        let nonempty = rows.iter().filter(|r| !r.properties.is_empty()).count();
        println!(
            "{pc:?}: {} canonical ActorData, {nonempty} nonempty2D8 lists",
            rows.len()
        );
        precisions.push(json!({"precision":format!("{pc:?}"),"definitions":rows.len(),"nonempty":nonempty,"data":rows}));
    }
    let doc = json!({"sources":catalog.sources,"precisions":precisions});
    if let Some(path) = std::env::var_os("AA_MEMBER_PROPERTY_DATA_CAPTURE") {
        std::fs::write(path, serde_json::to_vec_pretty(&doc).unwrap()).unwrap();
        return;
    }
    let expected: Value =
        serde_json::from_slice(&std::fs::read(evidence("actual-source-data.json")).unwrap())
            .unwrap();
    assert_eq!(doc, expected);
}
