use super::*;
#[test]
fn actual_native_sequence_constructor_retains_one_resource_and_task_all_pc(){
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-object-animation/native-sequence.json")).unwrap();
    let application=Arc::new(());
    let priority=crate::resource_task::DefaultPriority::from_fresh_application(&application);
    for row in fixture["rows"].as_array().unwrap(){
        assert_eq!(priority.current(),row["output"]["global_priority"]);
        for case in row["output"]["constructors"].as_array().unwrap(){
            let name=case["name"].as_str().unwrap();
            //Explicit immutable constructor input; no complete depot/decoder claim.
            let source=crate::resource_objects::SourceFile::from_catalog(&aa_rules::sources::CatalogEntry{
                logical_path:name.into(),size:0,origin:aa_rules::sources::Origin::OriginalArchive{member_index:0}});
            let entry=Entry::sequence(&application,name.encode_utf16().collect(),source,&priority).unwrap();
            assert_eq!(entry.handle.kind(),Kind::Sequence);
            assert_eq!(entry.handle.logical_ids(),name.encode_utf16().collect::<Vec<_>>());
            assert_eq!(entry.requests4,case["base"]["0x4"]);
            assert_eq!(entry.flags8,case["base"]["0x8"]);
            assert_eq!(entry.memory0c,case["base"]["0xc"]);
            assert_eq!(entry.memory10,case["base"]["0x10"]);
            assert_eq!(entry.references14,case["base"]["0x14"]);
            assert!(entry.locations.is_empty()&&entry.texture.is_none()&&entry.model.is_none());
            let derived=entry.sequence.as_ref().unwrap();
            assert!(derived.source3c.is_none()&&derived.source40.is_none());
            assert_eq!(derived.word44,case["derived"]["0x44"]);
            assert_eq!(derived.word48,case["derived"]["0x48"]);
            assert_eq!(derived.value4c,case["derived"]["0x4c"]);
            assert!(entry.task.resource().same_identity(&entry.handle));
            assert_eq!(entry.task.references4().unwrap(),case["task"]["0x4"]);
            assert_eq!(entry.task.priority8().unwrap(),case["task"]["0x8"]);
            assert_eq!(entry.task.sequence_c().unwrap(),case["task"]["0xc"]);
            assert!(!entry.task.queued10().unwrap());
            assert_eq!(entry.task.state14().unwrap(),case["task"]["0x14"]);
            assert!(entry.task.thread18().unwrap().is_none());
        }
    }
}
