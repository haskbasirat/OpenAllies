use super::*;
use serde_json::{Value,json};
struct Probe<'a>{c:&'a Value,events:Vec<Value>,values:Vec<u32>,rng:aa_simulation::NativeRng,attempts:usize,children:Vec<u32>,facing:[u32;2]}
fn pair(c:&Value,name:&str)->[u32;2]{std::array::from_fn(|i|c[name][i].as_u64().unwrap()as u32)}
fn bits(f:f32)->u32{f.to_bits()}
impl World for Probe<'_>{
 type Definition=u32;type Layout=usize;type Created=usize;
 fn clear_values(&mut self)->Result<()>{self.values.clear();Ok(())}
 fn select_layout(&mut self,arg:u32)->Result<usize>{let count=self.c["layouts"].as_u64().unwrap()as usize;
  Ok(if arg==u32::MAX {for _ in 0..2 {self.events.push(json!(["seed",self.rng.state]));}if count==1{0}else{self.rng.next_u32()as usize%count}}else{arg as usize})}
 fn row_count(&mut self,_:usize)->Result<usize>{Ok(self.c["rows"].as_u64().unwrap()as usize)}
 fn row(&mut self,l:usize,i:usize)->Result<Row<u32>>{Ok(Row{definition:(i%2)as u32,offset:[bits((i+1)as f32*0.125+l as f32),bits(-0.25-l as f32)],orientation:[bits(0.6),bits(-0.8)]})}
 fn position(&mut self)->Result<[u32;2]>{Ok(pair(self.c,"position"))}
 fn facing(&mut self)->Result<[u32;2]>{Ok(self.facing)}
 fn owner_108(&mut self)->Result<u32>{self.events.push(json!(["owner"]));if self.c["mutate"]==true{self.facing=pair(self.c,"after_facing");}Ok(0x10003)}
 fn current_id(&mut self)->Result<u32>{Ok(0x10007)}
 fn is_actor_data(&mut self,d:&u32)->Result<bool>{Ok(*d==0)}
 fn create(&mut self,r:Request<u32>,actor:bool)->Result<Option<usize>>{
  let mut position=r.position.to_vec();position.extend(r.direction);
  self.events.push(json!(["create",if actor{"actor"}else{"object"},r.definition,position,[u32::MAX,0,r.owner,0,0,u32::MAX,6,0,0,0,0,r.parent,0]]));
  if self.c["fail"].as_i64()==Some(self.attempts as i64){return Err("required_create".into())}
  let i=self.attempts;self.attempts+=1;self.children.push(0);
  Ok((self.c["created"]==true).then_some(i))
 }
 fn append_settlement(&mut self,made:usize)->Result<()>{if self.c["settlement"]==true {self.values.insert(0,0x10010+made as u32);self.children[made]=0x10007;}Ok(())}
}
#[test]
fn whole_city_requests_rng_lists_and_current_facing_match_native(){
 let doc:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-object-ground/native-fresh-city.json")).unwrap();
 let rows=doc["rows"].as_array().unwrap();assert_eq!(rows.len(),288);
 for row in rows {
  let c=&row["input"];let pc=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>panic!()};
  let mut w=Probe{c,events:vec![],values:vec![81,82],rng:aa_simulation::NativeRng::new(c["seed"].as_u64().unwrap()as u32),attempts:0,children:vec![],facing:pair(c,"facing")};
  let result=if c["component"]==true&&c["flag"].as_u64().unwrap()as u8!=0 {Ok(())}else{run(&mut w,c["argument"].as_u64().unwrap()as u32,c["component"]==true,pc)};
  assert_eq!(result.is_err(),!row["output"]["error"].is_null(),"{row}");
  assert_eq!(json!(w.events),row["output"]["events"],"{row}");
  assert_eq!(json!(w.values),row["output"]["values"],"{row}");
  assert_eq!(w.rng.state,u32::try_from(row["output"]["seed"].as_u64().unwrap()).unwrap());
  let links=w.children.iter().map(|v|if c["created"]==true{json!(v)}else{Value::Null}).collect::<Vec<_>>();
  assert_eq!(json!(links),row["output"]["settlement_city"],"{row}");
 }
}
