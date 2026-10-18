use aa_actors::{actor_creation::ActorParams, organization_spawn::{self, World}, world_creation::{Error,Result}};
use serde_json::{Value,json};

struct Adapter<'a>{case:&'a Value,calls:Vec<Value>,fail_at:Option<usize>,allegiance_read:bool}
impl Adapter<'_>{
    fn event(&mut self,event:Value)->Result<()>{
        self.calls.push(event);
        if self.fail_at==Some(self.calls.len()-1){Err(Error::World("required current child".into()))}else{Ok(())}
    }
    fn number(&self,key:&str)->u32{self.case[key].as_u64().unwrap()as u32}
}
impl World for Adapter<'_>{
    type Object=u32;type Definition=u32;type Layout=();type RecruitItem=u32;
    fn occupied(&mut self,organization:u32,slot:i32)->Result<bool>{assert_eq!(organization,7);assert_eq!(slot,self.number("slot")as i32);Ok(self.case["occupied"].as_bool().unwrap())}
    fn slot_definition(&mut self,_:u32,slot:i32)->Result<u32>{assert!(!self.case["occupied"].as_bool().unwrap());Ok(0x7000+slot as u32)}
    fn allegiance(&mut self,_:u32)->Result<u32>{self.event(json!(["allegiance"]))?;self.allegiance_read=true;Ok(self.number("allegiance"))}
    fn owner_gtid(&mut self,_:u32)->Result<u32>{assert!(self.allegiance_read);Ok(if self.case["owner_null"].as_bool().unwrap(){0}else{self.number("updated_gtid")})}
    fn create(&mut self,p:&mut ActorParams<u32,()>)->Result<Option<u32>>{
        assert!(p.name_utf16.is_empty());assert!(p.organization_layout.is_none());
        let words=[0xa25b28,p.definition,p.position_bits[0],p.position_bits[1],p.position_bits[2],
            p.direction_bits[0],p.direction_bits[1],0,p.variant,p.requested_id,p.owner,0,p.word_30,p.word_34,
            p.mode,p.word_3c,p.word_40,p.word_44,p.word_48,p.word_4c,u32::from(p.byte_50)|(u32::from(p.byte_51)<<8)|(u32::from(p.byte_52)<<16)];
        self.event(json!(["create",words]))?;Ok(self.case["result"].as_bool().unwrap().then_some(0x1234))
    }
    fn recruit_item(&mut self,_:u32,_:i32)->Result<Option<u32>>{assert!(self.case["result"].as_bool().unwrap());Ok((self.number("property")!=0).then(||self.number("property")))}
    fn pair_recruit_item(&mut self,member:u32,item:u32)->Result<()>{self.event(json!(["pair_recruit_item",member,item]))}
    fn add_member(&mut self,_:u32,slot:i32,member:u32)->Result<()>{self.event(json!(["add_member",slot,member]))}
    fn notify_member(&mut self,_:u32,member:u32)->Result<()>{self.event(json!(["notify_member",member]))}
}
fn replay(c:&Value,fail_at:Option<usize>)->(Result<Option<u32>>,Vec<Value>){
    let input=&c["input"];let mut world=Adapter{case:input,calls:vec![],fail_at,allegiance_read:false};
    let position=std::array::from_fn(|i|input["position"][i].as_u64().unwrap()as u32);
    let direction=std::array::from_fn(|i|input["direction"][i].as_u64().unwrap()as u32);
    let result=organization_spawn::create_member(&mut world,7,input["slot"].as_i64().unwrap()as i32,position,direction,input["byte50"].as_u64().unwrap()as u8,input["byte51"].as_u64().unwrap()as u8);
    (result,world.calls)
}
fn fixture()->Value{serde_json::from_str(include_str!("../../../research/development10/organization-spawn/native-create-member.json")).unwrap()}
#[test]
fn actual_both_images_all_precisions_descriptor_and_children(){
    let doc=fixture();let rows=doc["rows"].as_array().unwrap();assert_eq!(rows.len(),504);
    for c in rows{let(result,calls)=replay(c,None);assert_eq!(result.unwrap().unwrap_or(0),c["returned"].as_u64().unwrap()as u32,"{c}");assert_eq!(json!(calls),c["calls"],"{c}");
        assert_eq!(c["name_refcount"].as_u64().unwrap(),if c["input"]["occupied"].as_bool().unwrap(){1000}else{999});}
}
#[test]
fn required_children_preserve_every_completed_native_prefix(){
    for c in fixture()["rows"].as_array().unwrap(){let expected=c["calls"].as_array().unwrap();
        for fail_at in 0..expected.len(){let(result,calls)=replay(c,Some(fail_at));assert!(result.is_err());assert_eq!(calls,expected[..=fail_at],"{c}, fail={fail_at}");}}
}
