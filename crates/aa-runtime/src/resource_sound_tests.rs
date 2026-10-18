use super::*;
#[test]
fn original_v14_sound_constructor_and_common_task_all_pc() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-audio-pre-request/native-audio.json"
    ))
    .unwrap();
    let application = Arc::new(());
    let priority = crate::resource_task::DefaultPriority::from_fresh_application(&application);
    for row in fixture["rows"].as_array().unwrap() {
        for case in row["native"]["sound_constructors"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let source = SourceFile::from_catalog(&aa_rules::sources::CatalogEntry {
                logical_path: name.into(),
                size: 0,
                origin: aa_rules::sources::Origin::OriginalArchive { member_index: 0 },
            });
            let entry = Entry::sound(
                &application,
                name.encode_utf16().collect(),
                source,
                &priority,
            )
            .unwrap();
            assert_eq!(entry.handle.kind(), Kind::Sound);
            assert_eq!(
                entry.handle.logical_ids(),
                name.encode_utf16().collect::<Vec<_>>()
            );
            assert_eq!(entry.requests4, case["base"]["0x4"]);
            assert_eq!(entry.flags8, case["base"]["0x8"]);
            assert_eq!(entry.references14, case["base"]["0x14"]);
            let sound = entry.sound.as_ref().unwrap();
            assert!(sound.raw3c.is_none());
            assert_eq!(sound.standard_rate40, case["derived"]["0x40"]);
            assert_eq!(sound.streaming41, case["derived"]["0x41"]);
            assert_eq!(sound.byte42, case["derived"]["0x42"]);
            assert!(entry.texture.is_none() && entry.model.is_none() && entry.sequence.is_none());
            assert!(entry.task.resource().same_identity(&entry.handle));
            assert_eq!(entry.task.references4().unwrap(), case["task"]["0x4"]);
            assert_eq!(entry.task.priority8().unwrap(), case["task"]["0x8"]);
            assert_eq!(entry.task.sequence_c().unwrap(), case["task"]["0xc"]);
            assert!(!entry.task.queued10().unwrap());
            assert_eq!(entry.task.state14().unwrap(), case["task"]["0x14"]);
            assert!(entry.task.thread18().unwrap().is_none());
        }
        for ext in row["native"]["extensions"].as_array().unwrap() {
            assert_eq!(
                crate::resource_objects::class_for_name(
                    &format!("x.{}", ext["extension"].as_str().unwrap())
                        .encode_utf16()
                        .collect::<Vec<_>>()
                )
                .unwrap(),
                Some(Kind::Sound)
            );
            assert_eq!(ext["descriptor"], "0xb3d76c");
        }
    }
}
