//! Native state24's typed flag suffix must retain the same geometry/index.
use aa_spatial::index::{Member,SpatialIndex};
use serde_json::{json,Value};
#[test]
fn falling_marker_changes_only_native_bit12_on_same_member(){
 let fixture:Value=serde_json::from_str(include_str!("../../../research/phase7-spatial/native-index.json")).unwrap();
 for case in fixture["cases"].as_array().unwrap().iter().take(3){
  let mut index=SpatialIndex::new(serde_json::from_value(case["grid"].clone()).unwrap(),serde_json::from_value(case["flags"].clone()).unwrap(),serde_json::from_value(case["precision"].clone()).unwrap()).unwrap();
  let member:Member=serde_json::from_value(case["members"][0].clone()).unwrap();let key=member.key;
  index.register(member).unwrap();index.set_event_capture(true);let _=index.take_events();
  let before=serde_json::to_value(index.member(key).unwrap()).unwrap();let flags=index.member(key).unwrap().actor.flags;
  index.mark_falling(key).unwrap();let mut expected=before.clone();expected["actor"]["flags"]=json!(flags|0x1000);
  assert_eq!(serde_json::to_value(index.member(key).unwrap()).unwrap(),expected);assert!(index.take_events().is_empty());
  index.mark_falling(key).unwrap();assert_eq!(serde_json::to_value(index.member(key).unwrap()).unwrap(),expected);
  assert!(index.mark_falling(u32::MAX).is_err());assert_eq!(serde_json::to_value(index.member(key).unwrap()).unwrap(),expected);
 }
}
