use aa_rules::{actors::{ActorCatalog, native_load::{load_files,NativeLoadReport}},
    native_builder_data::{Inputs,NativeBuilderData},
    sources::{Origin,ResolvedFile,SourceLimits,V14Sources,sha256},tgi::Limits};
use serde_json::{Value,json};
fn evidence(name:&str)->std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-builder-data").join(name)
}
fn projection(arena:&NativeBuilderData)->Value {
    let lists:std::collections::BTreeMap<_,_>=arena.definitions.iter().map(|(k,v)|(k,v.ids.clone())).collect();
    let heads:std::collections::BTreeMap<_,_>=arena.definitions.iter().map(|(k,v)|(k,v.head)).collect();
    let tails:std::collections::BTreeMap<_,_>=arena.definitions.iter().map(|(k,v)|(k,v.tail)).collect();
    json!({"lists":lists,"heads":heads,"tails":tails,"nodes":arena.nodes,
        "order":arena.resolution_order,"failure":arena.failure})
}
fn file(text:&str)->ResolvedFile {
    let bound=text.replace("[BuilderComponent]","[BuilderComponent Template=BuilderComponent]");
    let bytes=format!("[template BuilderComponent]\n{{ [list BuildActor]\n string actor_ids\n}}\n{bound}").into_bytes();ResolvedFile{logical_path:"builder-fixture.tgi".into(),
        origin:Origin::V14Loose,sha256:sha256(&bytes),bytes}
}
#[test]
fn native_deferred_builder_lists_nodes_tails_and_failure_prefix_match() {
    let doc:Value=serde_json::from_slice(&std::fs::read(evidence("native-builder-data.json")).unwrap()).unwrap();
    assert_eq!(doc["cases"].as_array().unwrap().len(),252);
    for row in doc["cases"].as_array().unwrap() {
        let inputs:Inputs=serde_json::from_value(row["input"].clone()).unwrap();
        let arena=inputs.resolve_partial().unwrap();
        assert_eq!(projection(&arena),row["output"],"{row}");
        assert_eq!(inputs.complete().is_ok(),row["output"]["failure"].is_null());
        for definition in arena.definitions.values() {
            let mut current=definition.head;let mut ids=vec![];
            while let Some(handle)=current {let node=arena.node(handle).unwrap();ids.push(node.target.clone());current=node.next;}
            assert_eq!(ids,definition.ids);
        }
    }
}
#[test]
fn canonical_builder_owners_and_node_values_do_not_translate_again() {
    let actors=load_files(vec![file(concat!(
        "[Thing]\n{ string IDS = old\n string category = ACTOR\n [BuilderComponent]\n { [BuildActor]\n { string actor_ids = alias\n }\n [BuildActor]\n { string actor_ids = A\n }\n }\n}\n",
        "[Thing]\n{ string IDS = A\n string category = ACTOR\n [BodyComponent]\n{}\n}\n",
        "[Thing]\n{ string IDS = B\n string category = ACTOR\n [BodyComponent]\n{}\n}\n",
        "[Thing]\n{ string IDS = plain\n string category = ACTOR\n}\n",
        "[Translate]\n{ string node = Thing\n string ids_old = old\n string ids_new = B\n}\n",
        "[Translate]\n{ string node = Thing\n string ids_old = alias\n string ids_new = A\n}\n",
        "[Translate]\n{ string node = Thing\n string ids_old = A\n string ids_new = B\n}\n"
    ))],Limits::default(),&mut NativeLoadReport::default());
    let arena=actors.native_builder_data().unwrap();
    let definition=arena.definition("OLD").unwrap();
    assert!(definition.builder_present);
    assert_eq!(definition.ids,vec!["A","B"],"authored alias maps to A once; authored A maps to B");
    assert!(arena.definition("A").unwrap().body_present);
    assert!(!arena.definition("A").unwrap().builder_present);
    assert!(arena.definition("alias").is_err(),"canonical getter cannot resolve an alias");
    assert!(arena.definition("plain").unwrap().ids.is_empty());
    assert!(arena.node(aa_rules::native_builder_data::NodeHandle(usize::MAX)).is_err());
}
#[test]
fn wrong_class_and_bodyless_targets_are_errors_never_empty_builder_certificates() {
    for (target,kind) in [("object","UnknownActor"),("bodyless","MissingBody"),("missing","UnknownActor"),("NULL","UnknownActor")] {
        let source=format!("[Thing]\n{{ string IDS = owner\n string category = ACTOR\n [BuilderComponent]\n {{ [BuildActor]\n {{ string actor_ids = {target}\n }}\n }}\n}}\n[Thing]\n{{ string IDS = object\n string category = OBJECT\n}}\n[Thing]\n{{ string IDS = bodyless\n string category = ACTOR\n}}\n");
        let actors=load_files(vec![file(&source)],Limits::default(),&mut NativeLoadReport::default());
        let partial=actors.native_builder_inputs().unwrap().resolve_partial().unwrap();
        assert_eq!(json!(partial.failure.as_ref().unwrap().kind),kind);
        assert!(partial.require_complete().is_err());
        assert!(actors.native_builder_data().is_err());
        assert_eq!(partial.nodes.len(),0);
        assert!(partial.definition("owner").unwrap().head.is_none());
        assert!(partial.definition("owner").unwrap().tail.is_none());
    }
    let unordered=ActorCatalog::from_files(vec![],Limits::default());
    assert!(unordered.native_builder_data().unwrap_err().to_string().contains("native registration order"));
}
#[test]
fn random_actor_data_inherits_builder_loading_and_is_a_valid_body_target() {
    let source=concat!(
        "[Thing]\n{ string IDS = random_owner\n string category = RANDOM_ACTOR\n [Flags]\n{ bool ethereal = false\n}\n [BuilderComponent]\n { [BuildActor]\n { string actor_ids = random_target\n }\n }\n}\n",
        "[Thing]\n{ string IDS = random_target\n string category = RANDOM_ACTOR\n [Flags]\n{ bool ethereal = false\n}\n [BodyComponent]\n{}\n}\n"
    );
    let actors=load_files(vec![file(source)],Limits::default(),&mut NativeLoadReport::default());
    let arena=actors.native_builder_data().unwrap();
    assert_eq!(arena.definition("random_owner").unwrap().ids,vec!["random_target"]);
    assert!(arena.definition("random_target").unwrap().body_present);
}
#[test]
#[ignore="requires AA_GAME_ROOT original/v14 assets"]
fn actual_registered_builder_sources_match_native_and_keep_provenance() {
    let root=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut source=V14Sources::open(root.join("Data.rwd"),root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits{max_member_bytes:64*1024*1024}).unwrap();
    let actors=ActorCatalog::load_v14(&mut source).unwrap();
    let inputs=actors.native_builder_inputs().unwrap();
    assert!(inputs.actors.len()>=1028,"all standard ActorData plus derived RandomActorData");
    let arena=inputs.complete().unwrap();
    assert!(inputs.actors.iter().all(|a|!a.sources.is_empty()));
    let nonempty=arena.definitions.values().filter(|v|!v.ids.is_empty()).count();
    assert!(nonempty>0);
    if let Some(path)=std::env::var_os("AA_BUILDER_DATA_CAPTURE") {
        std::fs::write(path,serde_json::to_vec_pretty(&json!({"input":inputs,"output":projection(&arena),
            "source_diagnostics":actors.diagnostics,"sources":actors.sources})).unwrap()).unwrap();
        return;
    }
    let fixture:Value=serde_json::from_slice(&std::fs::read(evidence("native-real-builder-data.json")).unwrap()).unwrap();
    assert_eq!(fixture["cases"].as_array().unwrap().len(),6);
    for row in fixture["cases"].as_array().unwrap() {
        assert_eq!(json!(inputs),row["input"]);
        assert_eq!(projection(&arena),row["output"]);
    }
    println!("{} canonical ActorData, {nonempty} Builder lists, {} resolved references",inputs.actors.len(),arena.resolution_order.len());
}
