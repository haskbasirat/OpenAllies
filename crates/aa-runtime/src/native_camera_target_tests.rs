use super::*;
use serde_json::{Value,json};
struct Lab{case:Value,trace:Vec<Value>}
impl Lab{
 fn event(&mut self,name:&str,value:Value)->Result<()>{
  self.trace.push(json!([name,value]));
  if self.case["fail"].as_u64()==Some(self.trace.len()as u64){Err("required dependency cutoff".into())}else{Ok(())}
 }
}
impl Services for Lab{
 fn map_dimensions_74_78(&mut self)->Result<Option<[i32;2]>>{let value=self.case["dimensions"].clone();self.event("dimensions",value.clone())?;serde_json::from_value(value).map_err(|e|e.to_string())}
 fn fallback_dimensions_uvar_a8_ac(&mut self)->Result<[i32;2]>{let value=self.case["fallback"].clone();self.event("fallback",value.clone())?;serde_json::from_value(value).map_err(|e|e.to_string())}
 fn margin_uvar_24(&mut self)->Result<i32>{let value=self.case["margin"].as_i64().unwrap()as i32;self.event("margin",json!(value))?;Ok(value)}
 fn world_editor_275(&mut self)->Result<bool>{let value=self.case["editor"].as_u64().unwrap()!=0;self.event("editor",json!(value))?;Ok(value)}
 fn session_state_e4(&mut self)->Result<u32>{let value=self.case["session"].as_u64().unwrap()as u32;self.event("session",json!(value))?;Ok(value)}
 fn map_height_550210(&mut self,xy:[u32;2])->Result<Binary>{self.event("height",json!(xy))?;b(self.case["height"].as_u64().unwrap()as u32)}
 fn update_camera_6fe050(&mut self,_:&mut State,_:Precision)->Result<()>{self.event("update",Value::Null)}
}
#[test]
fn native_target_bounds_groundtracking_zoom_and_required_prefixes_all_pc(){
 let fixture:Value=serde_json::from_str(include_str!("../../../research/development10/native-camera-target/native-target.json")).unwrap();
 let mut count=0;
 for row in fixture["rows"].as_array().unwrap(){
  let pc=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>panic!()};
  let mut camera=State::before_constructor_reset_7016a0();
  for(i,value)in row["initial"].as_array().unwrap().iter().enumerate(){if let Some(value)=value.as_u64(){camera.put(4*i as u32,value as u32)}}
  //Explicit native-laboratory current interior pointer, not a completed
  //application7016A0 constructor or a public raw-word capability.
  camera.bind_target_rectangle_270_after_native_writer();
  let case=&row["input"];let mut lab=Lab{case:case.clone(),trace:vec![]};
  let result=if case["entry"]=="target"{set_look_at_6ff130(&mut camera,&mut lab,serde_json::from_value(case["target"].clone()).unwrap(),pc)}else{move_position_6fedb0(&mut camera,&mut lab,serde_json::from_value(case["target"].clone()).unwrap(),pc)};
  let words:Vec<Option<u32>>=(0..0x394).step_by(4).map(|offset|if [0x270,0x2a0].contains(&offset){None}else{Some(camera.word(offset).unwrap())}).collect();
  assert_eq!(json!({"words":words,"rectangle_alias":camera.target_rectangle_alias_present(),"point_alias":camera.target_point_alias_present(),"trace":lab.trace,"error":result.err()}),row["output"],"row{count} {case}");
  count+=1;
 }
 assert_eq!(count,123);
}
#[test]
fn unbound_constructor_shape_alias_is_not_replaced_by_a_fake_pointer(){
 let mut camera=State::before_constructor_reset_7016a0();
 assert!(!camera.target_rectangle_alias_present());assert!(!camera.target_point_alias_present());
 assert!(camera.word(0x270).is_err());assert!(camera.word(0x2a0).is_err());
 camera.bind_target_rectangle_270_after_native_writer();
 assert!(camera.target_rectangle_alias_present());assert!(camera.word(0x270).is_err());
}
