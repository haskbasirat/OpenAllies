use super::*;
#[test]fn partial_constructor_does_not_invent_directory_or_nicamera_ready(){let state=State::before_constructor_reset_7016a0();assert_eq!(state.word(0x21c).unwrap(),0xbf800000);assert_eq!(state.byte(0x33c).unwrap(),0);for offset in[0x204,0x164,0x174,0x138,0x114]{assert!(state.word(offset).is_err())}}

use serde_json::{Value,json};
struct Lab{case:Value,modes:usize,devs:usize,trace:Vec<Value>,pose:Option<Vec<u32>>,run_pose:bool}
impl Lab{
 fn event(&mut self,name:&str,value:Value)->Result<()>{self.trace.push(json!([name,value]));if self.case["fail"].as_u64()==Some(self.trace.len()as u64){Err("required dependency cutoff".into())}else{Ok(())}}
 fn word_for(&self,offset:u32)->u32{
  if offset==0x5c{return self.case["fov"].as_u64().map(|v|v as u32).unwrap_or(15.0f32.to_bits())}
  if let Some(value)=self.case["avars"][offset.to_string()].as_u64(){return value as u32}
  let value=match offset{0x1c=>1.0,0x20=>512.0,0x24=>0.1,0x28=>1.5,0x2c=>1.0,0x30=>0.5,0x34|0x38=>1.2,0x3c=>0.7,0x40=>2.1,0x44=>1.0,0x48=>-45.0,0x4c=>0.0,0x50=>40.0,0x54=>10.0,0x58=>100.0,_=>panic!("unbound AVar")};(value as f32).to_bits()
 }
}
impl ResetServices for Lab{
 fn metagame_40cee0(&mut self)->Result<bool>{let value=self.case["modes"][self.modes].as_bool().unwrap();self.modes+=1;self.event("mode",json!(value))?;Ok(value)}
 fn developer_camera_6fe9c0(&mut self,_:&State)->Result<bool>{let value=self.case["devs"][self.devs].as_bool().unwrap();self.devs+=1;self.event("developer",json!(value))?;Ok(value)}
 fn avar(&mut self,offset:u32)->Result<u32>{let word=self.word_for(offset);self.event("avar",json!([offset,word]))?;Ok(word)}
 fn map_dimensions_74_78(&mut self)->Result<Option<[i32;2]>>{let value=self.case["dimensions"].clone();self.event("dimensions",value.clone())?;if value.is_null(){Ok(None)}else{Ok(Some(serde_json::from_value(value).unwrap()))}}
 fn fallback_dimensions_uvar_a8_ac(&mut self)->Result<[i32;2]>{let value=self.case["fallback"].clone();self.event("fallback",value.clone())?;Ok(serde_json::from_value(value).unwrap())}
 fn visual_height_43d050(&mut self,xy:[u32;2],flags:u32)->Result<Binary>{self.event("visual",json!([xy,flags]))?;b(self.case["height"].as_u64().unwrap()as u32)}
 fn seed_word(&mut self,address:u32)->Result<u32>{let word=self.case["seeds"][((address-0xb43cf8)/4)as usize].as_u64().unwrap()as u32;self.event("seed",json!([address,word]))?;Ok(word)}
 fn dirty_planes_873d90(&mut self,_:&mut State)->Result<()>{self.event("dirty",Value::Null)}
 fn set_pose_6ff220(&mut self,camera:&mut State,center:[u32;3],yaw:u32,pitch:u32,zoom:u32,pc:Precision)->Result<()>{let value=vec![center[0],center[1],center[2],yaw,pitch,zoom];self.pose=Some(value.clone());self.event("pose",json!(value))?;if self.run_pose{pose::set_pose_6ff220(camera,&pose::Globals::from_static_constructor_9e3520(),self,center,yaw,pitch,zoom,pc)}else{Ok(())}}
}
#[test]fn native_camera_reset_projection_all_pc_ordered_dependency_prefixes(){
 let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-native-camera/native-camera-reset.json")).unwrap();let mut count=0;
 for row in fixture["rows"].as_array().unwrap(){let pc=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>panic!()};let case=&row["input"];
  let mut camera=State::before_constructor_reset_7016a0();for offset in(0..0x394).step_by(4){camera.put(offset,0xa5a5a5a5)}for(offset,value)in[(0x21c,(-1.0f32).to_bits()),(0x220,1.0f32.to_bits()),(0x224,1.0f32.to_bits()),(0x228,(-1.0f32).to_bits()),(0x230,0.75f32.to_bits())]{camera.put(offset,value)}camera.put_byte(0x33c,case["tracking"].as_u64().unwrap()as u8);camera.put(0x174,0);camera.put(0x218,case["fov"].as_u64().map(|v|v as u32).unwrap_or(15.0f32.to_bits()));
  let mut lab=Lab{case:case.clone(),modes:0,devs:0,trace:Vec::new(),pose:None,run_pose:false};let result=reset_700e30(&mut camera,&mut lab,pc);let fields=row["output"]["fields"].as_object().unwrap();for(offset,expected)in fields{let offset=u32::from_str_radix(offset.trim_start_matches("0x"),16).unwrap();assert_eq!(camera.word(offset).unwrap(),expected.as_u64().unwrap()as u32,"row{count} Camera+{offset:X}")}
  assert_eq!(json!(lab.trace),row["output"]["trace"],"row{count}");assert_eq!(json!(lab.pose),row["output"]["pose"],"row{count}");assert_eq!(result.err().map(Value::String).unwrap_or(Value::Null),row["output"]["error"],"row{count}");count+=1;
 }assert_eq!(count,318);
}
impl pose::Services for Lab{
 fn metagame_40cee0(&mut self)->Result<bool>{ResetServices::metagame_40cee0(self)}
 fn developer_camera_6fe9c0(&mut self,camera:&State)->Result<bool>{ResetServices::developer_camera_6fe9c0(self,camera)}
 fn avar(&mut self,offset:u32)->Result<u32>{ResetServices::avar(self,offset)}
 fn free_yaw_uvar_b4(&mut self)->Result<bool>{Ok(false)}
 fn world_editor_275(&mut self)->Result<bool>{Ok(false)}
 fn session_state_e4(&mut self)->Result<u32>{Ok(2)}
 fn map_height_550210(&mut self,xy:[u32;2])->Result<Binary>{self.event("map_height",json!(xy))?;b(12.5f32.to_bits())}
 fn update_camera_6fe050(&mut self,_:&mut State,_:Precision)->Result<()>{Ok(())}
}
#[test]fn native_reset_pose_rotations_zoom_match_original_before_required_ni_update(){
 let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-native-camera/native-camera-pose.json")).unwrap();let mut count=0;
 for row in fixture["rows"].as_array().unwrap(){let pc=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>panic!()};let case=&row["input"];
  let mut camera=State::before_constructor_reset_7016a0();for offset in(0..0x394).step_by(4){camera.put(offset,0xa5a5a5a5)}for(offset,value)in[(0x21c,(-1.0f32).to_bits()),(0x220,1.0f32.to_bits()),(0x224,1.0f32.to_bits()),(0x228,(-1.0f32).to_bits()),(0x230,0.75f32.to_bits())]{camera.put(offset,value)}camera.put_byte(0x33c,case["tracking"].as_u64().unwrap()as u8);camera.put(0x174,0);camera.put(0x218,case["fov"].as_u64().map(|v|v as u32).unwrap_or(15.0f32.to_bits()));camera.put(0x68,0x3f800000);
  let mut lab=Lab{case:case.clone(),modes:0,devs:0,trace:Vec::new(),pose:None,run_pose:true};let result=reset_700e30(&mut camera,&mut lab,pc);assert!(result.is_ok(),"row{count}: {result:?}");
  for offset in (0x38..=0x68).step_by(4).chain((0x208..=0x210).step_by(4)).chain((0x22c..=0x254).step_by(4)).chain([0x2b8,0x2bc,0x2c0]){assert_eq!(camera.word(offset).unwrap(),row["output"]["full_words"][(offset/4)as usize].as_u64().unwrap()as u32,"row{count} Camera+{offset:X}")}
  assert_eq!(json!(lab.trace),row["output"]["trace"],"row{count}");assert_eq!(json!(pose::Globals::from_static_constructor_9e3520().matrix_b40054()),row["output"]["matrix_b40054"]);assert_eq!(row["output"]["root_164_writes"],json!([]));count+=1;
 }assert_eq!(count,180);
}
