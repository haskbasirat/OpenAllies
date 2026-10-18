use aa_mind::{Arena,Error,Mind,MindId,Result,State,StateId,element_move::Payload,node_reached::{self,World,Actor}};
use aa_navigation::result_plan;
use serde_json::{Value,json};
struct Fixture {c:Value,events:Vec<Value>,fail_at:Option<usize>,stopped:Option<Value>}
impl Fixture {
 fn ev(&mut self,name:&str,bits:&[u32],p:&mut Payload)->Result<()> {
  self.events.push(json!({"name":name,"bits":bits}));
  let change=self.c["changes"][name].clone();
  if change["clear_path"]==true {p.plan.movement.path.clear_nodes();}
  if let Some(v)=change.get("position"){self.c["actor"]["position_bits"]=v.clone();}
  if let Some(v)=change["speed"].as_u64(){p.plan.movement.requested_speed_bits=v as u32;}
  if let Some(v)=change["target"].as_u64(){p.plan.movement.target_id=v as u32;}
  if let Some(v)=change.get("kind54"){self.c["kind54"]=v.clone();}
  if let Some(v)=change.get("collision"){p.collision=serde_json::from_value(v.clone()).unwrap();}
  if self.fail_at==Some(self.events.len()-1){self.stopped=Some(json!(p));return Err(Error::World("injected callback failure".into()));}
  Ok(())
 }
 fn n(&self,key:&str)->u32{self.c[key].as_u64().unwrap() as u32}
}
struct Query {position:[u32;2],facing:[u32;2],ignore:u32,flags:u32,mask:u32}
impl World for Fixture {
 type Query=Query;
 fn actor(&self,id:u32)->Result<Actor>{assert_eq!(id,1);Ok(Actor {position:serde_json::from_value(self.c["actor"]["position_bits"].clone()).unwrap(),leader_mind:(self.c["actor"]["has_leader"]==true).then_some(MindId(2))})}
 fn world_step_bits(&self)->Result<u32>{Ok(self.n("step"))}
 fn kind54(&self,_:MindId)->Result<u32>{Ok(self.n("kind54"))}
 fn leader_node_reached(&mut self,m:MindId,actor:u32,p:&mut Payload,_:&mut Arena)->Result<()>{assert_eq!(m,MindId(2));self.ev("LeaderNode",&[actor],p)}
 fn waypoint_offset(&mut self,_:StateId,p:&mut Payload,_:&mut Arena)->Result<()>{self.ev("Offset",&[],p)?;p.plan.movement.waypoint_offset_bits=self.n("offset_result");Ok(())}
 fn begin_query(&mut self,actor:u32,p:&mut Payload,_:&mut Arena)->Result<Query>{self.ev("BeginQuery",&[actor],p)?;Ok(Query {position:[0;2],facing:[0;2],ignore:0,flags:self.n("query_flags"),mask:self.n("query_mask")})}
 fn query_position(&mut self,q:&mut Query,xy:[u32;2],p:&mut Payload,_:&mut Arena)->Result<()>{self.ev("QueryPosition",&xy,p)?;q.position=xy;Ok(())}
 fn query_facing(&mut self,q:&mut Query,xy:[u32;2],p:&mut Payload,_:&mut Arena)->Result<()>{self.ev("QueryFacing",&xy,p)?;q.facing=xy;Ok(())}
 fn query_ignore(&mut self,q:&mut Query,id:u32,_:&mut Payload,_:&mut Arena)->Result<()>{q.ignore=if id==0x10007 && self.c["valid_target"]==true {2}else{0};Ok(())}
 fn query_kind2_flags(&mut self,q:&mut Query)->Result<()>{q.flags|=0x400;q.mask|=0x400;Ok(())}
 fn first_hit(&mut self,q:&mut Query,p:&mut Payload,_:&mut Arena)->Result<[u32;4]>{self.ev("FirstHit",&[q.position[0],q.position[1],q.facing[0],q.facing[1],q.ignore,q.flags,q.mask],p)?;Ok(serde_json::from_value(self.c["hit"].clone()).unwrap())}
 fn collision_response(&mut self,_:MindId,p:&mut Payload,_:&mut Arena)->Result<bool>{self.ev("CollisionResponse",&[],p)?;Ok(self.c["response"]==true)}
 fn default_stance(&mut self,actor:u32,p:&mut Payload,_:&mut Arena)->Result<()>{self.ev("DefaultStance",&[actor],p)}
 fn schedule_zero(&mut self,_:MindId,p:&mut Payload,_:&mut Arena)->Result<()>{self.ev("Schedule",&[0,0],p)}
}
fn setup(c:&Value)->(Payload,Arena,Fixture){
 let p=Payload {plan:result_plan::State {movement:serde_json::from_value(c["state"].clone()).unwrap(),attempts:0,previous_destination_bits:[0;2]},collision:serde_json::from_value(c["collision"].clone()).unwrap(),collision_vector:serde_json::from_value(c["collision_vector"].clone()).unwrap(),detached_time_bits:0,word78:0};
 let mut arena=Arena::default();arena.minds.insert(MindId(1),Mind::base(1));arena.states.insert(StateId(1),State {parent:MindId(1),discard_on_suspend:false,status:2});
 (p,arena,Fixture{c:c.clone(),events:vec![],fail_at:None,stopped:None})
}
#[test]
fn original_node_reached_control_math_and_native_free_order(){
 let doc:Value=serde_json::from_str(include_str!("../../../research/phase7-node-reached/native-node.json")).unwrap();
 for row in doc["cases"].as_array().unwrap(){
  let c=&row["input"];let(mut p,mut a,mut w)=setup(c);let pc=serde_json::from_value(c["precision"].clone()).unwrap();
  let success=node_reached::apply(StateId(1),&mut p,&mut a,pc,&mut w).unwrap();
  let mut expected=row["output"].clone();let freed=expected.as_object_mut().unwrap().remove("freed").unwrap();
  let actual=json!({"events":w.events,"path":p.plan.movement.path,"offset":p.plan.movement.waypoint_offset_bits,"collision":p.collision,"collision_vector":p.collision_vector,"position":w.c["actor"]["position_bits"],"speed":p.plan.movement.requested_speed_bits,"kind54":w.c["kind54"],"target":p.plan.movement.target_id,"success":success});
  assert_eq!(actual,expected,"{}",c["name"]);
  let removed=w.events.iter().filter(|e|e["name"]=="Offset").count();
  let expected_freed=c["state"]["path"]["nodes"].as_array().unwrap().iter().take(removed).map(|n|n["words"].clone()).collect::<Vec<_>>();
  assert_eq!(freed,json!(expected_freed),"native52CDB0 frees in head order");
  for index in 0..w.events.len(){
   let(mut p,mut a,mut bad)=setup(c);bad.fail_at=Some(index);
   assert!(node_reached::apply(StateId(1),&mut p,&mut a,pc,&mut bad).is_err());
   assert_eq!(bad.events,w.events[..=index]);assert_eq!(json!(p),bad.stopped.unwrap(),"{} callback{index}: effects stop on error",c["name"]);
  }
 }
}
