use super::*;
#[test]
fn final_native_store_changes_ad_and_preserves_ac_and_neighbors(){
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/view-init-completion/native-completion.json")).unwrap();
    for row in fixture["rows"].as_array().unwrap(){
        let before=row["before"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u8).collect::<Vec<_>>();
        let mut actual=before.clone();let ad=AtomicU8::new(actual[1]);completed_ad_43ff36(&ad);actual[1]=ad.load(Ordering::Acquire);
        let expected=row["after"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u8).collect::<Vec<_>>();
        assert_eq!(actual,expected);
    }
}
