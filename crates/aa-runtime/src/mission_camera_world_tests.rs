use super::*;
#[test]
fn original_camera_stores_log_arguments_and_missing_capital_no_write_match_all_pcs() {
 let f:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-mission-start/native-mission-camera.json")).unwrap();
 assert_eq!(f["cases"].as_array().unwrap().len(),54);
 for r in f["cases"].as_array().unwrap() {
  let input=&r["input"];let pair=|key:&str|[input[key][0].as_u64().unwrap()as u32,input[key][1].as_u64().unwrap()as u32];
  let choice=match input["kind"].as_u64().unwrap(){0=>Choice::Observer(pair("extents")),1=>Choice::MissingCapital,2=>Choice::Capital(pair("xy")),_=>unreachable!()};
  let pc=match r["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>unreachable!()};
  let mut camera=load_tail_world::CameraState{look_at:Some([0xaa55aa55,0x55aa55aa]),messages:vec![]};let mut logs=Vec::new();
  apply(&mut camera,&mut logs,choice,pc).unwrap();
  let expected=[r["output"]["look_at"][0].as_u64().unwrap()as u32,r["output"]["look_at"][1].as_u64().unwrap()as u32];
  assert_eq!(camera.look_at,Some(expected),"{r}");assert!(camera.messages.is_empty());
  let native=&r["output"]["logs"][0];let (line,args)=match logs.as_slice(){
   [Log::Observer{double_arguments,..}]=>(0x128,Some(*double_arguments)),
   [Log::Capital{double_arguments,..}]=>(0x132,Some(*double_arguments)),
   [Log::MissingCapital]=>(0x136,None),_=>unreachable!()};
  assert_eq!(native["line"].as_u64().unwrap(),line);
  if let Some(args)=args {assert_eq!(native["double_arguments"],serde_json::json!(args),"{r}")}else{assert!(native.get("double_arguments").is_none())}
 }
}
