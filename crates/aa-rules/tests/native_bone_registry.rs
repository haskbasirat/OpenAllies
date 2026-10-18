//!486650/4867B0/486870 actual mode0 Bone registration fixture.
use aa_rules::{actors::native_load::{load_files,NativeLoadReport},sources::{Origin,ResolvedFile,sha256},tgi::Limits};
#[test]
fn bone_native_directory_keeps_append_order_and_duplicate_prefix(){
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/view-object-init/native-bone-registry.json")).unwrap();
    for row in fixture["rows"].as_array().unwrap(){
        let text=row["input"].as_array().unwrap().iter().map(|input|format!("[Bone]\n{{\nIDS = {}\nbone_name = {}\n}}\n",input["ids"].as_str().unwrap(),input["bone_name"].as_str().unwrap())).collect::<String>();
        let file=ResolvedFile{logical_path:"explicit-native-bone-input.tgi".into(),origin:Origin::V14Loose,sha256:sha256(text.as_bytes()),bytes:text.into_bytes()};
        let mut report=NativeLoadReport::default();let catalog=load_files(vec![file],Limits::default(),&mut report);
        let registry=catalog.native_data_registry("Bone").unwrap();assert_eq!(registry.mode,Some(0));
        let actual=registry.entries.iter().map(|entry|entry.ids.as_str()).collect::<Vec<_>>();
        let expected=row["output"]["entries"].as_array().unwrap().iter().map(|entry|String::from_utf16(&serde_json::from_value::<Vec<u16>>(entry["ids"].clone()).unwrap()).unwrap()).collect::<Vec<_>>();
        assert_eq!(actual,expected);
        assert_eq!(!catalog.diagnostics.is_empty(),!row["output"]["error"].is_null());
    }
}
