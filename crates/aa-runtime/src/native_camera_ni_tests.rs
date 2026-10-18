use super::*;
use serde_json::Value;
fn pc(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        _ => panic!("{name}"),
    }
}
fn lab(words: &[Value]) -> State {
    let mut camera = State::before_constructor_reset_7016a0();
    //Exactly the child input slice. These local/frustum words are unchanged
    //by6FE050; no expected child output is used as an initial value.
    for offset in [0x24, 0xb0]
        .into_iter()
        .chain((0x38..=0x68).step_by(4))
        .chain((0x138..=0x14c).step_by(4))
    {
        camera.put(offset, words[offset as usize / 4].as_u64().unwrap() as u32);
    }
    camera.put_byte(0x150, words[0x150 / 4].as_u64().unwrap() as u8);
    camera
}
fn outputs() -> Vec<u32> {
    [0x28, 0x2c, 0x30]
        .into_iter()
        .chain((0x6c..=0x9c).step_by(4))
        .chain((0xc8..=0x104).step_by(4))
        .chain([0x10c, 0x110])
        .chain((0x114..=0x134).step_by(4))
        .chain((0x19c..=0x1f8).step_by(4))
        .chain((0x2d4..=0x300).step_by(4))
        .collect()
}
#[test]
fn full_native_pose_fixture_replays_ni_child_output_slice() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-native-camera/native-camera-pose.json"
    ))
    .unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 180);
    for row in rows {
        let words = row["output"]["full_words"].as_array().unwrap();
        let mut camera = lab(words);
        update_6fe050(
            &mut camera,
            &mut RequiredChildren,
            pc(row["precision"].as_str().unwrap()),
        )
        .unwrap();
        for offset in outputs() {
            assert_eq!(
                camera.word(offset).unwrap(),
                words[offset as usize / 4].as_u64().unwrap() as u32,
                "{} {} case{} offset{offset:X}",
                row["image"],
                row["precision"],
                row["case"]
            );
        }
        assert!(
            camera.word(0x108).is_err(),
            "unwritten field must stay unknown"
        );
    }
}
#[test]
fn positive_parent_and_attachment_require_actual_owners_at_native_prefix() {
    let mut camera = State::before_constructor_reset_7016a0();
    camera.put(0x24, 1);
    let error = update_6fe050(&mut camera, &mut RequiredChildren, Precision::Double53).unwrap_err();
    assert!(error.contains("45CAB0"));
    assert!(camera.word(0x6c).is_err());
    camera.put(0x24, 0);
    camera.put(0xb0, 1);
    for i in 0..13 {
        camera.put(0x38 + 4 * i, 0x100 + i);
    }
    let error = update_6fe050(&mut camera, &mut RequiredChildren, Precision::Double53).unwrap_err();
    assert!(error.contains("virtual38"));
    for i in 0..13 {
        assert_eq!(camera.word(0x6c + 4 * i).unwrap(), 0x100 + i);
    }
    assert!(
        camera.word(0x114).is_err(),
        "directions follow attached callback"
    );
}

#[test]
fn direct_original_v14_all_pc_arbitrary_perspective_orthographic_and_required_gates() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-native-camera/native-camera-ni.json"
    ))
    .unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 150);
    for row in rows {
        let input = &row["input"];
        let mut camera = State::before_constructor_reset_7016a0();
        let gate = input["gate"].as_str();
        camera.put(0x24, if gate == Some("parent") { 1 } else { 0 });
        camera.put(0xb0, if gate == Some("attached") { 1 } else { 0 });
        for (i, word) in input["local"].as_array().unwrap().iter().enumerate() {
            camera.put(0x38 + 4 * i as u32, word.as_u64().unwrap() as u32);
        }
        for (i, word) in input["frustum"].as_array().unwrap().iter().enumerate() {
            camera.put(0x138 + 4 * i as u32, word.as_u64().unwrap() as u32);
        }
        camera.put_byte(0x150, input["ortho"].as_u64().unwrap() as u8);
        let result = update_6fe050(
            &mut camera,
            &mut RequiredChildren,
            pc(row["precision"].as_str().unwrap()),
        );
        let output = &row["output"];
        match output["error"].as_str() {
            None => assert!(result.is_ok(), "{row} {result:?}"),
            Some("parent") => assert!(result.unwrap_err().contains("45CAB0")),
            Some("attached") => assert!(result.unwrap_err().contains("virtual38")),
            Some(other) => panic!("{other}"),
        }
        for (i, offset) in fixture["output_offsets"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            let offset = offset.as_u64().unwrap() as u32;
            let stored = output["writes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|w| w["offset"].as_u64() == Some(offset as u64));
            if stored {
                assert_eq!(
                    camera.word(offset).unwrap(),
                    output["words"][i].as_u64().unwrap() as u32,
                    "case{} {} offset{offset:X}",
                    row["case"],
                    row["precision"]
                );
            } else {
                assert!(
                    camera.word(offset).is_err(),
                    "unwritten case{} offset{offset:X}",
                    row["case"]
                );
            }
        }
    }
}
