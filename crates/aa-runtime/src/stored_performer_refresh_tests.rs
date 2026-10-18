use super::*;
use aa_actors::actor_creation::{Error,Result as ActorResult};
use value::Fields;
struct Probe{value:u32,events:std::cell::RefCell<Vec<String>>,property_error:bool,table_error:bool}
impl Probe{fn log(&self,s:impl Into<String>){self.events.borrow_mut().push(s.into());}}
impl Fields for Probe{
 fn property_count(&self)->ActorResult<usize>{self.log("property_count");Ok(1)}
 fn property_bias(&self,_:usize)->ActorResult<F32>{self.log("property_bias");if self.property_error{Err(actor_error("required Property source"))}else{Ok(F32(1f32.to_bits()))}}
 fn set_value(&mut self,bits:u32)->ActorResult<()>{self.log(format!("store:{bits:08x}"));self.value=bits;Ok(())}
 fn value(&self)->ActorResult<u32>{Ok(self.value)}
 fn body(&self)->ActorResult<(F32,F32)>{self.log("body");Ok((F32(2f32.to_bits()),F32(3f32.to_bits())))}
 fn speed(&self)->ActorResult<F32>{self.log("speed");Ok(F32(4f32.to_bits()))}
 fn ability_count(&self,defender:bool)->ActorResult<usize>{self.log(if defender{"defender_count"}else{"primary_count"});Ok(if defender{1}else{3})}
 fn ability_at(&self,defender:bool,index:usize)->ActorResult<u32>{Ok(if defender{4}else{index as u32+1})}
 fn enabled(&self,handle:u32)->ActorResult<bool>{self.log(format!("enabled:{handle}"));Ok(handle!=2)}
 fn evaluate(&mut self,handle:u32,unit:[F32;4])->ActorResult<F32>{
  self.log(format!("evaluate:{handle}"));assert_eq!(unit.map(|v|v.0),[2f32.to_bits(),3f32.to_bits(),4f32.to_bits(),1f32.to_bits()]);
  if handle==3&&self.table_error{return Err(Error::Unsupported("required current CRT"))}
  Ok(F32((if handle==4{0.5}else{handle as f32}).to_bits()))
 }
}
#[test]
fn stored_performer_field_kernel_preserves_property_and_second_ability_error_prefix(){
 for pc in [Precision::Single24,Precision::Double53,Precision::Extended64]{
  let mut p=Probe{value:99f32.to_bits(),events:Default::default(),property_error:true,table_error:false};
  assert!(value::refresh_fields(&mut p,pc).unwrap_err().to_string().contains("required Property source"));
  assert_eq!(p.value,99f32.to_bits());assert_eq!(*p.events.borrow(),["property_count","property_bias"]);
  p.property_error=false;p.table_error=true;p.events.borrow_mut().clear();
  assert!(value::refresh_fields(&mut p,pc).unwrap_err().to_string().contains("required current CRT"));
  assert_eq!(p.value,1f32.to_bits());
  assert_eq!(*p.events.borrow(),["property_count","property_bias","store:00000000","body","speed","primary_count","enabled:1","evaluate:1","store:3f800000","enabled:2","enabled:3","evaluate:3"]);
 }
}
#[test]
fn stored_performer_field_kernel_uses_primary_then_defender_same_maximum(){
 for pc in [Precision::Single24,Precision::Double53,Precision::Extended64]{
  let mut p=Probe{value:99f32.to_bits(),events:Default::default(),property_error:false,table_error:false};
  value::refresh_fields(&mut p,pc).unwrap();assert_eq!(p.value,3f32.to_bits());
  let events=p.events.into_inner();
  assert!(!events.iter().any(|s|s=="evaluate:2"));
  assert!(events.iter().position(|s|s=="evaluate:3").unwrap()<events.iter().position(|s|s=="defender_count").unwrap());
  assert_eq!(events.last().map(String::as_str),Some("store:40400000"),"native stores even the unchanged primary max after a smaller Defender value");
 }
}
#[test]
fn stored_performer_field_kernel_replays_native_disabled_table_rows(){
 struct Native<'a>{row:&'a serde_json::Value,value:u32}
 impl Fields for Native<'_>{
  fn property_count(&self)->ActorResult<usize>{Ok(self.row["property_biases"].as_array().unwrap().len())}
  fn property_bias(&self,index:usize)->ActorResult<F32>{Ok(F32(self.row["property_biases"][index].as_u64().unwrap()as u32))}
  fn set_value(&mut self,bits:u32)->ActorResult<()>{self.value=bits;Ok(())}
  fn value(&self)->ActorResult<u32>{Ok(self.value)}
  fn body(&self)->ActorResult<(F32,F32)>{Ok((F32::ZERO,F32::ZERO))}
  fn speed(&self)->ActorResult<F32>{Ok(F32::ZERO)}
  fn ability_count(&self,defender:bool)->ActorResult<usize>{Ok(self.row[if defender{"defender_count"}else{"primary_count"}].as_u64().unwrap_or(0)as usize)}
  fn ability_at(&self,_:bool,index:usize)->ActorResult<u32>{Ok(index as u32)}
  fn enabled(&self,_:u32)->ActorResult<bool>{Ok(false)}
  fn evaluate(&mut self,_:u32,_:[F32;4])->ActorResult<F32>{panic!("native disabled tables never reach a positive Ability/CV service")}
 }
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-stored-performer-refresh/native-stored-performer-wrapper.json")).unwrap();
 let rows=fixture["cases"].as_array().unwrap();assert_eq!(rows.len(),81);
 for row in rows{
  let pc=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>unreachable!()};
  let mut fields=Native{row,value:99f32.to_bits()};value::refresh_fields(&mut fields,pc).unwrap();
  assert_eq!(fields.value as u64,row["output"]["value"].as_u64().unwrap());
 }
}
