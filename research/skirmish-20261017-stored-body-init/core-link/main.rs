#[path="E:/Games/Axis and Alies/rust-rebuild/crates/aa-runtime/src/body_init_fields.rs"] mod kernel;
use aa_actors::actor_creation::{Error,Result};
use kernel::Fields;
#[test]
fn exact_current_shared_kernel_matches_original_v14_finite_prefix_fixture() {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("E:/Games/Axis and Alies/rust-rebuild/research/skirmish-20261016-load-component-init/native-body-prefix.json")).unwrap();
 struct Raw{state:u32,health:u32,maximum:u32,construction:Option<bool>,writes:Vec<(u32,u32)>}
 impl Fields for Raw {
  fn state_low(&self)->Result<u32>{Ok(self.state)}
  fn health(&self)->Result<u32>{Ok(self.health)}
  fn maximum(&self)->Result<u32>{Ok(self.maximum)}
  fn construction_target(&self)->Result<Option<u32>>{Ok((self.state&(1<<29)!=0&&self.construction==Some(true)).then_some(1))}
  fn update_maximum(&mut self,_:u32)->Result<()>{Err(Error::World("561D40".into()))}
  fn store_health(&mut self,b:u32)->Result<()>{self.writes.push((self.health,b));self.health=b;Ok(())}
  fn set_health(&mut self,_:u32)->Result<()>{Err(Error::World("561E50".into()))}
 }
 let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),216);
 for row in rows {
  let c=&row["input"];let mut host=Raw{state:c["state_low"].as_u64().unwrap()as u32,health:c["health_bits"].as_u64().unwrap()as u32,maximum:c["maximum_bits"].as_u64().unwrap()as u32,construction:c["construction"].as_bool(),writes:vec![]};
  let result=kernel::initialize(&mut host);
  let out=match result{Ok(true)=>"complete",Ok(false)=>panic!(),Err(e)if e.to_string().contains("561D40")=>"required_561D40",Err(e)if e.to_string().contains("561E50")=>"required_561E50",Err(e)=>panic!("{e}")};
  assert_eq!(out,row["native"]["outcome"]);
  assert_eq!(host.health,row["native"]["health_bits"].as_u64().unwrap()as u32);
  assert_eq!(host.maximum,row["native"]["maximum_bits"].as_u64().unwrap()as u32);
  let expected:Vec<_>=row["native"]["writes"].as_array().unwrap().iter().map(|v|(v["before"].as_u64().unwrap()as u32,v["after"].as_u64().unwrap()as u32)).collect();assert_eq!(host.writes,expected);
 }
}
