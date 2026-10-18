use super::*;
#[test]
fn same_default_slots_guard_material_factory_and_native_fields(){
 let fixture:Value=serde_json::from_str(include_str!("../../../research/development10/native-render-defaults/native-defaults.json")).unwrap();
 let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),24);
 for row in rows{
  let mut shared=SharedSet::from_fresh_native_process().unwrap();
  shared.globals.fog=serde_json::from_value(row["input"]["fog"].clone()).unwrap();
  assert!(shared.new_property_set_895550().is_err());
  for _ in 0..row["input"]["repeat"].as_u64().unwrap(){shared.initialize_properties_prefix_8c3ed0().unwrap()}
  assert!(shared.started&&shared.completed&&shared.partial.is_none());
  assert_eq!(shared.material_next_id(),row["output"]["material_next"].as_u64().unwrap()as u32);
  let mut sets=Vec::new();
  for _ in 0..row["input"]["sets"].as_u64().unwrap(){sets.push(shared.new_property_set_895550().unwrap())}
  for(slot,expected)in row["output"]["properties"].as_array().unwrap().iter().enumerate(){
   let property=shared.slots[slot].as_ref().unwrap();assert_eq!(property.kind().unwrap().slot(),slot);
   for set in &sets{assert!(property.same_allocation(set[slot].as_ref().unwrap()))}
   for(offset,value)in expected["fields"].as_object().unwrap(){
    assert_eq!(property.word(offset.parse().unwrap()).unwrap(),value.as_u64().unwrap()as u32,"slot{slot}+{offset} row{row}")
   }
   for(offset,value)in expected["bytes"].as_object().unwrap(){
    assert_eq!(property.byte(offset.parse().unwrap()).unwrap(),value.as_u64().unwrap()as u8,"slot{slot}+{offset} row{row}")
   }
  }
 }
}
#[test]
fn actual_two_process_observations_bind_current_initial_globals(){
 assert_eq!(verified_initial_fog().unwrap(),[0,0,0]);
 let mut shared=SharedSet::from_fresh_native_process().unwrap();
 shared.initialize_properties_prefix_8c3ed0().unwrap();assert_eq!(shared.material_next_id(),2);
 let first=shared.new_property_set_895550().unwrap();let second=shared.new_property_set_895550().unwrap();
 assert!(first.iter().zip(&second).all(|(a,b)|a.as_ref().unwrap().same_allocation(b.as_ref().unwrap())));
}
