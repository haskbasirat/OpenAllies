use super::*;
fn fixture()->serde_json::Value {serde_json::from_str(include_str!("../../../research/skirmish-20261017-mission-start/native-mission-finalize.json")).unwrap()}
#[test]
fn original_retained_compare_float_temporary_and_ftol_clamp_match_all_pcs() {
 let f=fixture();assert_eq!(f["ratios"].as_array().unwrap().len(),48);
 for row in f["ratios"].as_array().unwrap() {
  let pc=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>unreachable!()};
  let input=&row["input"];let maximum=input[0].as_i64().unwrap()as i32;let available=input[1].as_i64().unwrap()as i32;let current=input[2].as_i64().unwrap()as i32;
  let actual=ratio(current,maximum,available,pc).unwrap();let expected=row["output"]["new_ratio"].as_u64().map(|v|v as u32 as i32);
  assert_eq!(actual,expected,"{row}");
 }
 for pc in [Precision::Single24,Precision::Double53,Precision::Extended64] {
  assert_eq!(ratio(0,3,7,pc).unwrap(),None);assert_eq!(ratio(-1,3,7,pc).unwrap(),None);assert_eq!(ratio(50,3,0,pc).unwrap(),None);
 }
}
#[test]
fn original_zero_duration_sun_copies_raw_colors_and_preserves_queue_dimmer_time() {
 let f=fixture();assert_eq!(f["suns"].as_array().unwrap().len(),12);
 for row in f["suns"].as_array().unwrap() {
  let before:Vec<u32>=row["output"]["before"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect();
  let after:Vec<u32>=row["output"]["after"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect();
  let colors:[[u32;3];3]=std::array::from_fn(|i|std::array::from_fn(|j|row["colors"][i][j].as_u64().unwrap()as u32));
  let mut sun=crate::sun_state::State::new([0;3]);
  sun.scheduled_head=Some(u64::from(before[1]));sun.direction.copy_from_slice(&before[2..5]);
  sun.target_colors=std::array::from_fn(|i|std::array::from_fn(|j|before[5+3*i+j]));
  sun.current_colors=std::array::from_fn(|i|std::array::from_fn(|j|before[14+3*i+j]));
  sun.color_duration=before[23];sun.current_dimmer=before[24];sun.target_dimmer=before[25];sun.dimmer_duration=before[26];sun.start_time=before[27];
  sun.byte_70=before[28]as u8;sun.word_74=before[29];sun.word_78=before[30];sun.byte_7c=before[31]as u8;sun.word_80=before[32];sun.word_84=before[33];
  instant_colors(&mut sun,colors);
  assert_eq!(sun.scheduled_head,Some(u64::from(after[1])));assert_eq!(sun.direction,after[2..5]);
  assert_eq!(sun.target_colors.iter().flatten().copied().collect::<Vec<_>>(),after[5..14]);assert_eq!(sun.current_colors.iter().flatten().copied().collect::<Vec<_>>(),after[14..23]);
  assert_eq!([sun.color_duration,sun.current_dimmer,sun.target_dimmer,sun.dimmer_duration,sun.start_time],after[23..28]);
  assert_eq!([sun.byte_70 as u32,sun.word_74,sun.word_78,sun.byte_7c as u32,sun.word_80,sun.word_84],[after[28]&255,after[29],after[30],after[31]&255,after[32],after[33]]);
 }
 assert_eq!(f["source_stores"].as_array().unwrap().len(),12);
 for row in f["source_stores"].as_array().unwrap(){assert_eq!(row["output"]["fields"],serde_json::json!(["diffuse","ambient","specular"]));assert_eq!(row["output"]["colors"],row["colors"]);}
}
