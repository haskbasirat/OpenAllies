use super::*;
#[test]fn initial_visual_global_is_pe_zero_and_not_created_by_simulation_world(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/native-render-defaults/visual-process-initial.json")).unwrap();
 let rows=fixture["words"].as_array().unwrap();
 assert_eq!(rows.iter().filter(|r|r["address"]==0xb36518u32&&r["initial_loader_word"]==0).count(),2);
 let mut counter=crate::world_revision::Counter::fresh_application();let world=counter.construct();
 let global=Global::from_fresh_process_image();assert_eq!(global.dimensions_for_world(&world,None).unwrap(),None);
}
#[test]fn native_ceil_extent_conversion_retains_order_and_signed_domain(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/visual-map/native-dimensions.json")).unwrap();
 for row in fixture["rows"].as_array().unwrap(){
  let extents=serde_json::from_value(row["extents"].clone()).unwrap();
  assert_eq!(serde_json::json!(dimensions_7241ce(extents).unwrap()),row["dimensions"]);
 }
 assert!(dimensions_7241ce([f32::NAN.to_bits(),0]).is_err());
 assert!(dimensions_7241ce([0,2147483648.0f32.to_bits()]).is_err());
}
