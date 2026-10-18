//! The SAME twelve Ni shared-default references initialized at8C3ED0.
//! This ports only guard+property prefix through8C3F20, not later Ni RTTI/
//! shader initialization. Material allocation stays in one retained Factory.
use crate::native_render_properties::{Factory,Globals,Kind,Reference};
use serde_json::Value;
type Result<T>=std::result::Result<T,String>;
const REFERENCE:&[u8]=include_bytes!("../../../research/development10/native-render-defaults/hardware-reference.json");
const REFERENCE_SHA:&str="960780239b2e564be6c03e377268c0cc53baa799fcff7a2c59168d3e36e2e123";
const CAPTURES:[&[u8];2]=[
 include_bytes!("../../../research/development10/native-render-defaults/capture-01/capture.json"),
 include_bytes!("../../../research/development10/native-render-defaults/capture-02/capture.json")];
const SLOT_GLOBALS:[u32;12]=[0xb43f3c,0xb442f8,0xb44304,0xb43f48,0xb44334,0xb44310,0xb4431c,0xb442ec,0xb43dfc,0xb43f1c,0xb43e08,0xb44328];
///Process global values observed from creation through the reached default
///constructors. Future proved native writers must mutate this SAME owner;
///the observation is deliberately not a perpetual immutability assertion.
pub struct ProcessGlobals{fog:[u32;3]}
impl Globals for ProcessGlobals{
 fn word(&mut self,address:u32)->Result<u32>{
  let index=[0xb43e68,0xb43e6c,0xb43e70].iter().position(|&a|a==address)
   .ok_or("Ni default constructor requested a different global owner")?;
  Ok(self.fog[index])
 }
}
///One retained application/process allocation, no Clone/Default/rawslots
///constructor. A later camera uses the same Factory and shared references.
pub struct SharedSet{
 started:bool,completed:bool,slots:Vec<Option<Reference>>,
 partial:Option<Reference>,factory:Factory,globals:ProcessGlobals,
}
impl SharedSet{
 pub fn from_fresh_native_process()->Result<Self>{
  let fog=verified_initial_fog()?;
  Ok(Self{started:false,completed:false,slots:(0..12).map(|_|None).collect(),
   partial:None,factory:Factory::from_native_process_image(),globals:ProcessGlobals{fog}})
 }
 ///8C3ED0 writes its once guard BEFORE the first allocation. Refused source
 ///leaves the reached same allocation and the prior global slots intact.
 pub fn initialize_properties_prefix_8c3ed0(&mut self)->Result<()>{
  if self.started{return Ok(())}self.started=true;
  for slot in 0..SLOT_GLOBALS.len(){
   match self.factory.construct(Kind::from_slot(slot)?,&mut self.globals){
    Ok(property)=>self.slots[slot]=Some(property),
    Err(failure)=>{self.partial=Some(failure.property);return Err(failure.error)}
   }
  }
  self.completed=true;Ok(())
 }
 ///895550 first makes12 NULL slots, then895380 retains the actual current
 ///global in slot order8..34. Every clone below aliases its SAME property.
 pub fn new_property_set_895550(&self)->Result<Vec<Option<Reference>>>{
  if !self.completed{return Err("Ni shared defaults have not completed their actual constructor prefix".into())}
  let mut set=Vec::new();set.try_reserve(12).map_err(|_|"895550 property-set allocation failed")?;
  for property in &self.slots{set.push(property.clone())}Ok(set)
 }
 pub fn factory_and_globals(&mut self)->(&mut Factory,&mut ProcessGlobals){(&mut self.factory,&mut self.globals)}
 pub fn material_next_id(&self)->u32{self.factory.material_next_id()}
 pub fn failed_property(&self)->Option<&Reference>{self.partial.as_ref()}
}
fn parse(bytes:&[u8])->Result<Value>{serde_json::from_slice(bytes).map_err(|e|e.to_string())}
fn verified_initial_fog()->Result<[u32;3]>{
 if aa_rules::sources::sha256(REFERENCE)!=REFERENCE_SHA{return Err("Ni default reference digest differs".into())}
 let reference=parse(REFERENCE)?;
 if reference["schema"]!=1||reference["native_exe_sha256"]!="3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1"
  ||reference["fog_addresses"]!=serde_json::json!([0xb43e68u32,0xb43e6c,0xb43e70]){
  return Err("Ni default reference target/coverage differs".into())
 }
 for(index,bytes)in CAPTURES.iter().enumerate(){
  if reference["captures"][index]["sha256"]!=aa_rules::sources::sha256(bytes){return Err("Ni default raw capture differs".into())}
  let capture=parse(bytes)?;
  if capture["original_error"]!=Value::Null||capture["completed_twelve_properties"]!=true
   ||capture["original_archives_preserved"]!=true||capture["cleanup"]["errors"]!=serde_json::json!([])
   ||capture["cleanup"]["wait_result"]!=0||capture["folder_redirection_observed"]!=true{
   return Err("Ni default hardware observation/cleanup is incomplete".into())
  }
  let rows=capture["global_captures"].as_array().ok_or("Ni default observation rows absent")?;
  let before=rows.iter().find(|r|r["label"]=="defaults_init_entry").ok_or("Ni default entry absent")?;
  let after=rows.iter().find(|r|r["label"]=="defaults_properties_completed").ok_or("Ni default prefix return absent")?;
  if before["seed_words"]!=serde_json::json!([0,0,0,0,1])||after["seed_words"]!=serde_json::json!([0,0,0,1,2]){
   return Err("Ni shared-default current process seed/counter prefix differs".into())
  }
  let probes=capture["isolated_post_cutoff_watch_calibration_probes"].as_array().ok_or("Ni default watch probes absent")?;
  if probes.len()!=4||probes.iter().enumerate().any(|(i,p)|p["index"]!=i||p["observed_bits"]!=p["value"]){
   return Err("Ni default calibrated watch coverage differs".into())
  }
 }
 serde_json::from_value(reference["fog_words"].clone()).map_err(|e|e.to_string())
}
#[cfg(test)]#[path="native_render_defaults_tests.rs"]mod tests;
