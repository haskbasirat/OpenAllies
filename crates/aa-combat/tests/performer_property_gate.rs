//! Native57F1E0 prefix fixture: downstream bodies remain required stops.
use aa_combat::{performer_property_gate::{self as gate,World},Error,Result};
use serde_json::{Value,json};
struct Owner{input:Value,enabled:Vec<bool>}
fn required(name:&str)->Error{Error::World(name.into())}
impl World for Owner{
 type Requirement=u32;
 fn ability_count(&self)->Result<usize>{Ok(self.enabled.len())}
 fn ability_at(&self,index:usize)->Result<u32>{Ok(index as u32)}
 fn required_properties(&self,id:u32)->Result<Option<Vec<u32>>>{Ok(self.input["abilities"][id as usize]["required"].as_array().map(|v|v.iter().map(|v|v.as_u64().unwrap()as u32).collect()))}
 fn has_property(&self,required:&u32)->Result<bool>{
  let Some(properties)=self.input["properties"].as_array()else{return Err(required_error())};
  Ok(properties.iter().any(|p|p["definition"].as_u64().unwrap()as u32==*required))
 }
 fn enabled(&self,id:u32)->Result<bool>{Ok(self.enabled[id as usize])}
 fn set_enabled(&mut self,id:u32,value:bool)->Result<()>{self.enabled[id as usize]=value;Ok(())}
 fn refresh_actions(&mut self)->Result<()>{Err(required("4D1350 actions"))}
 fn refresh_value(&mut self)->Result<()>{Err(required("57E430 value"))}
}
fn required_error()->Error{required("581280 NULL Properties read")}
#[test]
fn every_native_property_gate_prefix_matches(){
 let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-performer-property-refresh/native-performer-property.json")).unwrap();
 let rows=fixture["cases"].as_array().unwrap();assert_eq!(rows.len(),306);
 for row in rows{
  let input=row["input"].clone();let enabled=input["abilities"].as_array().unwrap().iter().map(|v|v["enabled"].as_bool().unwrap()).collect();
  let mut owner=Owner{input,enabled};let error=gate::refresh(&mut owner).unwrap_err();
  assert!(error.to_string().contains(row["output"]["required"].as_str().unwrap()),"{row}: {error}");
  assert_eq!(json!(owner.enabled.iter().map(|b|u8::from(*b)).collect::<Vec<_>>()),row["output"]["enabled"],"{row}");
 }
}
